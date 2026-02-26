use std::error::Error;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use mbedtls::hash::Type as MdType;
use mbedtls::pk::Pk;
use mbedtls::rng::Rdrand;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use rustls_mbedcrypto_provider::mbedtls_crypto_provider;
use sha2::{Digest, Sha256};

use crate::enrollment_models::{EnrollResponse, EnrollStatus};
use crate::http_protocol;
use crate::load_ca_cert;

pub fn run(
    node_id: &str,
    ra_host: &str,
    ra_port: u16,
    out_cert: &str,
    out_key: &str,
    blob_store: &str,
) -> Result<(), Box<dyn Error>> {
    print_embedded_ca_info();
    println!("enroll: generating ECDSA P-256 key pair...");
    let mut rng = Rdrand;
    let mut key = Pk::generate_ec(&mut rng, mbedtls::pk::EcGroupId::SecP256R1)?;
    let key_pem = key.write_private_pem_string()?;

    let subject = format!("CN={node_id}");
    let csr_pem = mbedtls::x509::csr::Builder::new()
        .subject(&subject)?
        .key(&mut key)
        .signature_hash(MdType::Sha256)
        .write_pem_string(&mut rng)?;
    println!("enroll: CSR generated (subject {subject})");

    let spki_der = extract_spki_from_csr(&csr_pem)?;
    let mut nonce = [0u8; 32];
    mbedtls::rng::Random::random(&mut rng, &mut nonce)?;

    let mut hasher = Sha256::new();
    hasher.update(&spki_der);
    hasher.update(node_id.as_bytes());
    hasher.update(&nonce);
    let binding = hasher.finalize();
    println!("enroll: binding = {}", hex_encode(binding.as_slice()));

    let quote_bytes = get_sgx_quote(&binding);
    let quote_b64 = base64::engine::general_purpose::STANDARD.encode(&quote_bytes);
    let nonce_hex = hex_encode(&nonce);

    let enroll_req = crate::enrollment_models::EnrollRequest {
        csr_pem: csr_pem.clone(),
        node_id: node_id.to_string(),
        nonce: nonce_hex,
        quote: quote_b64,
    };
    let body = serde_json::to_string(&enroll_req)?;

    let addr = format!("{ra_host}:{ra_port}");
    let server_name: ServerName<'_> = ServerName::try_from(ra_host.to_string())?;

    println!("enroll: submitting CSR to {addr}...");
    let resp_body = tls_post(&addr, &server_name, "/enroll", &body)?;
    let resp: EnrollResponse = serde_json::from_str(&resp_body)?;
    println!("enroll: enrollment request submitted, id={}, status={}", resp.id, resp.status);

    println!("enroll: polling for approval...");
    let cert_pem = loop {
        std::thread::sleep(Duration::from_secs(3));

        let status_path = format!("/enroll/{}", resp.id);
        match tls_get(&addr, &server_name, &status_path) {
            Ok(status_body) => {
                let status: EnrollStatus = serde_json::from_str(&status_body)?;
                match status.status.as_str() {
                    "PENDING" => {
                        print!(".");
                        std::io::stdout().flush()?;
                    }
                    "APPROVED" => {
                        println!("\nenroll: APPROVED");
                        break status.certificate.ok_or("approved but no certificate returned")?;
                    }
                    "REJECTED" => {
                        return Err("enrollment REJECTED by administrator".into());
                    }
                    other => {
                        return Err(format!("unexpected status: {other}").into());
                    }
                }
            }
            Err(e) => {
                eprintln!("\nenroll: poll error: {e}, retrying...");
            }
        }
    };

    let bs_host = blob_store.split(':').next().unwrap_or(blob_store);
    let bs_server_name: ServerName<'_> = ServerName::try_from(bs_host.to_string())?;
    tls_put(blob_store, &bs_server_name, &format!("/blob/{out_cert}"), cert_pem.as_bytes())?;
    println!("enroll: certificate stored to blob-store as '{out_cert}'");
    tls_put(blob_store, &bs_server_name, &format!("/blob/{out_key}"), key_pem.as_bytes())?;
    println!("enroll: private key stored to blob-store as '{out_key}'");

    Ok(())
}

#[cfg(target_env = "sgx")]
fn get_sgx_quote(binding: &[u8]) -> Vec<u8> {
    use sgx_isa::{Report, Targetinfo};

    let mut reportdata = [0u8; 64];
    reportdata[..binding.len().min(64)].copy_from_slice(&binding[..binding.len().min(64)]);

    let targetinfo = Targetinfo::from(Report::for_self());
    let report = Report::for_target(&targetinfo, &reportdata);

    // In a full implementation, this report would be sent to a quoting enclave
    // to obtain an ECDSA quote. For now, return the raw report bytes.
    AsRef::<[u8]>::as_ref(&report).to_vec()
}

#[cfg(not(target_env = "sgx"))]
fn get_sgx_quote(_binding: &[u8]) -> Vec<u8> {
    eprintln!("enroll: WARNING: not running in SGX, using empty quote (testing mode)");
    Vec::new()
}

fn make_tls_config() -> Result<Arc<ClientConfig>, Box<dyn Error>> {
    let ca_store = load_ca_cert();
    let config = ClientConfig::builder_with_provider(Arc::new(mbedtls_crypto_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(ca_store)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

fn tls_post(addr: &str, server_name: &ServerName<'_>, path: &str, body: &str) -> Result<String, Box<dyn Error>> {
    let request = http::Request::builder()
        .method(http::Method::POST)
        .uri(path)
        .header(http::header::HOST, addr)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body.as_bytes().to_vec())?;
    let response = send_http_request(addr, server_name, &request)?;
    parse_text_response(response)
}

fn tls_put(addr: &str, server_name: &ServerName<'_>, path: &str, body: &[u8]) -> Result<String, Box<dyn Error>> {
    let request = http::Request::builder()
        .method(http::Method::PUT)
        .uri(path)
        .header(http::header::HOST, addr)
        .header(http::header::CONTENT_TYPE, "application/octet-stream")
        .body(body.to_vec())?;
    let response = send_http_request(addr, server_name, &request)?;
    parse_text_response(response)
}

fn tls_get(addr: &str, server_name: &ServerName<'_>, path: &str) -> Result<String, Box<dyn Error>> {
    let request = http::Request::builder()
        .method(http::Method::GET)
        .uri(path)
        .header(http::header::HOST, addr)
        .body(Vec::new())?;
    let response = send_http_request(addr, server_name, &request)?;
    parse_text_response(response)
}

fn send_http_request(
    addr: &str,
    server_name: &ServerName<'_>,
    request: &http::Request<Vec<u8>>,
) -> Result<http::Response<Vec<u8>>, Box<dyn Error>> {
    let config = make_tls_config()?;
    let mut tcp = TcpStream::connect(addr)?;
    let mut conn = ClientConnection::new(config, server_name.to_owned())?;
    while conn.is_handshaking() {
        conn.complete_io(&mut tcp)?;
    }
    let mut tls = StreamOwned::new(conn, tcp);

    let raw_request = http_protocol::serialize_http_request(request)?;
    tls.write_all(&raw_request)?;
    tls.flush()?;

    let raw_response = http_protocol::read_http_response(&mut tls)?;
    http_protocol::parse_http_response_message(&raw_response)
}

fn parse_text_response(response: http::Response<Vec<u8>>) -> Result<String, Box<dyn Error>> {
    let status = response.status();
    let body = String::from_utf8_lossy(response.body()).into_owned();
    if !status.is_success() {
        return Err(format!("HTTP {status}: {body}").into());
    }
    Ok(body)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn extract_spki_from_csr(csr_pem: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use x509_parser::pem::parse_x509_pem;
    use x509_parser::prelude::FromDer;

    let (_, pem) = parse_x509_pem(csr_pem.as_bytes())?;
    let (_, csr) =
        x509_parser::certification_request::X509CertificationRequest::from_der(&pem.contents)?;
    Ok(csr.certification_request_info.subject_pki.raw.to_vec())
}

fn print_embedded_ca_info() {
    use x509_parser::prelude::parse_x509_certificate;

    let pem = crate::EmbeddedAssets::get("ca.crt").expect("ca.crt not found in embedded assets");
    let certs = rustls_pemfile::certs(&mut &*pem.data)
        .collect::<Result<Vec<_>, _>>()
        .expect("failed to parse embedded CA PEM");

    for cert_der in &certs {
        let (_, cert) = parse_x509_certificate(cert_der.as_ref())
            .expect("failed to parse embedded CA certificate");
        println!(
            "enroll: trusted CA: subject={}, not_after={}",
            cert.subject(),
            cert.validity().not_after
        );
    }
}
