use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use rustls_mbedcrypto_provider::mbedtls_crypto_provider;
use sha2::{Digest, Sha256};

use crate::enrollment_models::{EnrollResponse, EnrollStatus};
use crate::load_ca_cert;

pub fn run(
    node_id: &str,
    ra_host: &str,
    ra_port: u16,
    out_cert: &str,
    out_key: &str,
) -> Result<(), Box<dyn Error>> {
    print_embedded_ca_info();
    println!("enroll: generating ECDSA P-256 key pair...");
    let key_pair = KeyPair::generate()?;
    let key_pem = key_pair.serialize_pem();

    let san_uri = format!("urn:qkms:{node_id}");
    let mut params = CertificateParams::new(vec![san_uri.clone()])?;
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, node_id);

    let csr = params.serialize_request(&key_pair)?;
    let csr_pem = csr.pem()?;
    println!("enroll: CSR generated (subject CN={node_id}, SAN {san_uri})");

    let spki_der = rcgen::PublicKeyData::subject_public_key_info(&key_pair);
    let mut nonce = [0u8; 32];
    getrandom::getrandom(&mut nonce).expect("failed to generate random nonce");

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

    std::fs::write(out_cert, &cert_pem)?;
    std::fs::write(out_key, &key_pem)?;
    println!("enroll: certificate saved to {out_cert}");
    println!("enroll: private key saved to {out_key}");

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
    report.as_ref().to_vec()
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
    let config = make_tls_config()?;
    let mut tcp = TcpStream::connect(addr)?;
    let mut conn = ClientConnection::new(config, server_name.to_owned())?;
    while conn.is_handshaking() {
        conn.complete_io(&mut tcp)?;
    }
    let mut tls = StreamOwned::new(conn, tcp);

    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    tls.write_all(request.as_bytes())?;
    tls.flush()?;

    read_response_body(&mut tls)
}

fn tls_get(addr: &str, server_name: &ServerName<'_>, path: &str) -> Result<String, Box<dyn Error>> {
    let config = make_tls_config()?;
    let mut tcp = TcpStream::connect(addr)?;
    let mut conn = ClientConnection::new(config, server_name.to_owned())?;
    while conn.is_handshaking() {
        conn.complete_io(&mut tcp)?;
    }
    let mut tls = StreamOwned::new(conn, tcp);

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
    );
    tls.write_all(request.as_bytes())?;
    tls.flush()?;

    read_response_body(&mut tls)
}

fn read_response_body(stream: &mut impl Read) -> Result<String, Box<dyn Error>> {
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_else(|| text.to_string());
    Ok(body)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
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
