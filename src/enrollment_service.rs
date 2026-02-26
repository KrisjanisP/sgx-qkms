use std::collections::HashMap;
use std::error::Error;
use std::io::{BufRead, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use rcgen::KeyPair;
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use rustls_mbedcrypto_provider::mbedtls_crypto_provider;
use sha2::{Digest, Sha256};
use x509_parser::prelude::FromDer;

use crate::enrollment_models::{EnrollRequest, EnrollResponse, EnrollStatus};
use crate::http_protocol::{self, HttpMethod, HttpResponse, ParsedRequest};
use crate::{load_certs, load_private_key};

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Status {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone)]
struct PendingEnrollment {
    csr_pem: String,
    _node_id: String,
    _nonce_hex: String,
    _quote_b64: String,
    status: Status,
    certificate_pem: Option<String>,
}

type EnrollStore = Arc<Mutex<HashMap<String, PendingEnrollment>>>;

type ApprovalSender = Option<mpsc::Sender<(String, String)>>;

pub fn run(
    addr: &str,
    server_cert_path: &str,
    server_key_path: &str,
    ca_cert_path: &str,
    ca_key_path: &str,
    interactive: bool,
) -> Result<(), Box<dyn Error>> {
    crate::print_cert_info("CA cert", ca_cert_path);
    crate::print_cert_info("Server cert", server_cert_path);

    let store: EnrollStore = Arc::new(Mutex::new(HashMap::new()));
    let ca_cert_pem = std::fs::read_to_string(ca_cert_path)?;
    let ca_key_pem = std::fs::read_to_string(ca_key_path)?;
    let ca_material = Arc::new(CaMaterial { ca_cert_pem, ca_key_pem });

    let approval_tx: ApprovalSender = if interactive {
        let (tx, rx) = mpsc::channel::<(String, String)>();
        let istore = store.clone();
        let ica = ca_material.clone();
        std::thread::spawn(move || interactive_approval_loop(rx, &istore, &ica));
        println!("enroll-service: interactive mode enabled (approve/reject via stdin)");
        Some(tx)
    } else {
        None
    };

    let server_cert_chain = load_certs(server_cert_path);
    let server_key = load_private_key(server_key_path);

    let server_config = ServerConfig::builder_with_provider(Arc::new(mbedtls_crypto_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(server_cert_chain, server_key)?;

    let server_config = Arc::new(server_config);
    let listener = TcpListener::bind(addr)?;
    println!("enrollment service listening on {addr}");

    loop {
        let (tcp_stream, peer_addr) = match listener.accept() {
            Ok(conn) => conn,
            Err(e) => return Err(e.into()),
        };
        let server_config = server_config.clone();
        let store = store.clone();
        let ca_material = ca_material.clone();
        let approval_tx = approval_tx.clone();

        std::thread::spawn(move || {
            if let Err(e) = handle_connection(tcp_stream, server_config, &store, &ca_material, &approval_tx) {
                eprintln!("enroll-service: failed for {peer_addr}: {e}");
            }
        });
    }
}

struct CaMaterial {
    ca_cert_pem: String,
    ca_key_pem: String,
}

fn interactive_approval_loop(
    rx: mpsc::Receiver<(String, String)>,
    store: &EnrollStore,
    ca: &CaMaterial,
) {
    let stdin = std::io::stdin();
    for (id, node_id) in rx {
        println!();
        println!(">>> Enrollment request: id={id}, node={node_id}");
        print!(">>> Approve? [y/n]: ");
        let _ = std::io::stdout().flush();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            eprintln!("enroll-service: failed to read stdin");
            continue;
        }

        match line.trim() {
            "y" | "Y" | "yes" => {
                let csr_pem = {
                    let enrollments = store.lock().unwrap();
                    match enrollments.get(&id) {
                        Some(e) => e.csr_pem.clone(),
                        None => {
                            eprintln!("enroll-service: enrollment {id} not found");
                            continue;
                        }
                    }
                };

                match sign_csr(&csr_pem, ca) {
                    Ok(cert_pem) => {
                        let mut enrollments = store.lock().unwrap();
                        if let Some(e) = enrollments.get_mut(&id) {
                            e.status = Status::Approved;
                            e.certificate_pem = Some(cert_pem);
                        }
                        println!("enroll-service: approved enrollment {id}");
                    }
                    Err(e) => {
                        eprintln!("enroll-service: signing failed for {id}: {e}");
                    }
                }
            }
            _ => {
                let mut enrollments = store.lock().unwrap();
                if let Some(e) = enrollments.get_mut(&id) {
                    e.status = Status::Rejected;
                }
                println!("enroll-service: rejected enrollment {id}");
            }
        }
    }
}

fn handle_connection(
    mut tcp_stream: TcpStream,
    server_config: Arc<ServerConfig>,
    store: &EnrollStore,
    ca: &CaMaterial,
    approval_tx: &ApprovalSender,
) -> Result<(), Box<dyn Error>> {
    let mut server_conn = ServerConnection::new(server_config)?;
    while server_conn.is_handshaking() {
        server_conn.complete_io(&mut tcp_stream)?;
    }

    let mut tls_stream = StreamOwned::new(server_conn, tcp_stream);
    let raw = http_protocol::read_http_request(&mut tls_stream)?;
    let parsed = http_protocol::parse_http_request(&raw)?;
    let response = route_request(&parsed, store, ca, approval_tx);
    tls_stream.write_all(&response.to_http_bytes())?;
    tls_stream.flush()?;
    Ok(())
}

fn route_request(request: &ParsedRequest, store: &EnrollStore, ca: &CaMaterial, approval_tx: &ApprovalSender) -> HttpResponse {
    match (&request.method, request.path.as_str()) {
        (HttpMethod::Post, "/enroll") => handle_enroll(request, store, approval_tx),
        (HttpMethod::Post, path) if path.ends_with("/approve") => {
            if let Some(id) = path
                .strip_prefix("/enroll/")
                .and_then(|rest| rest.strip_suffix("/approve"))
            {
                handle_approve(id, store, ca)
            } else {
                json_error(404, "not found")
            }
        }
        (HttpMethod::Get, path) if path.starts_with("/enroll/") => {
            let id = &path["/enroll/".len()..];
            handle_status(id, store)
        }
        _ => json_error(404, "not found"),
    }
}

fn handle_enroll(request: &ParsedRequest, store: &EnrollStore, approval_tx: &ApprovalSender) -> HttpResponse {
    let body = match &request.body {
        Some(b) => b,
        None => return json_error(400, "missing request body"),
    };

    let req: EnrollRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => return json_error(400, &format!("invalid JSON: {e}")),
    };

    if let Err(msg) = verify_csr_and_binding(&req) {
        eprintln!("enroll-service: CSR verification failed: {msg}");
        return json_error(400, &msg);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let node_id = req.node_id.clone();

    println!("enroll-service: enrollment request {id} for node '{node_id}'");
    if req.quote.is_empty() {
        println!("enroll-service: WARNING: empty quote (non-SGX testing mode)");
    } else {
        println!("enroll-service: WARNING: quote verification is stubbed (no DCAP)");
        log_quote_info(&req.quote);
    }

    let enrollment = PendingEnrollment {
        csr_pem: req.csr_pem,
        _node_id: req.node_id,
        _nonce_hex: req.nonce,
        _quote_b64: req.quote,
        status: Status::Pending,
        certificate_pem: None,
    };

    store.lock().unwrap().insert(id.clone(), enrollment);

    if let Some(tx) = approval_tx {
        let _ = tx.send((id.clone(), node_id));
    }

    let resp = EnrollResponse {
        id,
        status: "PENDING".to_string(),
    };
    json_response(200, &resp)
}

fn handle_status(id: &str, store: &EnrollStore) -> HttpResponse {
    let enrollments = store.lock().unwrap();
    match enrollments.get(id) {
        Some(e) => {
            let status_str = match e.status {
                Status::Pending => "PENDING",
                Status::Approved => "APPROVED",
                Status::Rejected => "REJECTED",
            };
            let resp = EnrollStatus {
                status: status_str.to_string(),
                certificate: e.certificate_pem.clone(),
            };
            json_response(200, &resp)
        }
        None => json_error(404, "enrollment not found"),
    }
}

fn handle_approve(id: &str, store: &EnrollStore, ca: &CaMaterial) -> HttpResponse {
    let csr_pem = {
        let enrollments = store.lock().unwrap();
        match enrollments.get(id) {
            Some(e) => match e.status {
                Status::Pending => e.csr_pem.clone(),
                Status::Approved => return json_error(400, "already approved"),
                Status::Rejected => return json_error(400, "already rejected"),
            },
            None => return json_error(404, "enrollment not found"),
        }
    };

    let cert_pem = match sign_csr(&csr_pem, ca) {
        Ok(pem) => pem,
        Err(e) => return json_error(500, &format!("certificate signing failed: {e}")),
    };

    {
        let mut enrollments = store.lock().unwrap();
        if let Some(e) = enrollments.get_mut(id) {
            e.status = Status::Approved;
            e.certificate_pem = Some(cert_pem);
        }
    }

    println!("enroll-service: approved enrollment {id}");

    let resp = EnrollStatus {
        status: "APPROVED".to_string(),
        certificate: None,
    };
    json_response(200, &resp)
}

fn verify_csr_and_binding(req: &EnrollRequest) -> Result<(), String> {
    use x509_parser::pem::parse_x509_pem;

    let (_, pem) = parse_x509_pem(req.csr_pem.as_bytes())
        .map_err(|e| format!("failed to parse CSR PEM: {e}"))?;
    let (_, csr) =
        x509_parser::certification_request::X509CertificationRequest::from_der(&pem.contents)
            .map_err(|e| format!("failed to parse CSR DER: {e}"))?;

    csr.verify_signature().map_err(|e| format!("CSR signature invalid: {e}"))?;
    println!("enroll-service: CSR signature verified (proof-of-possession OK)");

    let spki_raw = csr.certification_request_info.subject_pki.raw;

    let nonce_bytes = hex_decode(&req.nonce)
        .map_err(|e| format!("invalid nonce hex: {e}"))?;

    let mut hasher = Sha256::new();
    hasher.update(spki_raw);
    hasher.update(req.node_id.as_bytes());
    hasher.update(&nonce_bytes);
    let binding = hasher.finalize();

    if !req.quote.is_empty() {
        let quote_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &req.quote,
        ).map_err(|e| format!("invalid quote base64: {e}"))?;

        // REPORTDATA is at offset 368 in an SGX quote, 64 bytes long.
        // First 32 bytes should match our binding value.
        const REPORTDATA_OFFSET: usize = 368;
        if quote_bytes.len() >= REPORTDATA_OFFSET + 64 {
            let reportdata = &quote_bytes[REPORTDATA_OFFSET..REPORTDATA_OFFSET + 32];
            if reportdata != binding.as_slice() {
                return Err("REPORTDATA binding mismatch: quote does not match CSR SPKI + node_id + nonce".to_string());
            }
            println!("enroll-service: REPORTDATA binding verified");
        } else {
            println!("enroll-service: WARNING: quote too short to extract REPORTDATA, skipping binding check");
        }
    } else {
        println!("enroll-service: WARNING: empty quote, skipping binding check (non-SGX testing)");
    }

    println!("enroll-service: computed binding = {}", hex_encode(binding.as_slice()));

    Ok(())
}

fn sign_csr(csr_pem: &str, ca: &CaMaterial) -> Result<String, Box<dyn Error>> {
    let ca_key_pair = KeyPair::from_pem(&ca.ca_key_pem)?;
    let ca_issuer = rcgen::Issuer::from_ca_cert_pem(&ca.ca_cert_pem, ca_key_pair)?;

    let csr_params = rcgen::CertificateSigningRequestParams::from_pem(csr_pem)?;
    let issued = csr_params.signed_by(&ca_issuer)?;

    Ok(issued.pem())
}

fn log_quote_info(quote_b64: &str) {
    let Ok(quote) = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        quote_b64,
    ) else {
        println!("enroll-service: WARNING: could not decode quote for logging");
        return;
    };

    if quote.len() < 432 {
        println!("enroll-service: quote is {} bytes (too short for full SGX quote structure)", quote.len());
        return;
    }

    // SGX quote v3 offsets (approximate, for logging only)
    let version = u16::from_le_bytes([quote[0], quote[1]]);
    println!("enroll-service: quote version={version}, size={} bytes", quote.len());
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("odd-length hex string".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| format!("invalid hex at offset {i}: {e}"))
        })
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn json_response<T: serde::Serialize>(status: u16, payload: &T) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json",
        body: serde_json::to_string(payload).unwrap_or_else(|_| {
            r#"{"status":"error"}"#.to_string()
        }),
    }
}

fn json_error(status: u16, message: &str) -> HttpResponse {
    let payload = serde_json::json!({ "message": message });
    json_response(status, &payload)
}
