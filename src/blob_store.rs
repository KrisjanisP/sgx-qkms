use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use rustls::{ServerConfig, ServerConnection, StreamOwned};
use rustls_mbedcrypto_provider::mbedtls_crypto_provider;
use sha2::{Digest, Sha256};

use crate::http_protocol::{self, BinaryHttpResponse, HttpMethod, ParsedRequest};
use crate::{load_certs, load_private_key};

struct BlobEntry {
    data: Vec<u8>,
    checksum: String,
    #[cfg(target_env = "sgx")]
    seal_data: SealData,
}

type BlobStore = Arc<Mutex<HashMap<String, BlobEntry>>>;

pub fn run(
    addr: &str,
    server_cert_path: &str,
    server_key_path: &str,
) -> Result<(), Box<dyn Error>> {
    crate::print_cert_info("Blob-store server cert", server_cert_path);

    let store: BlobStore = Arc::new(Mutex::new(HashMap::new()));

    let server_cert_chain = load_certs(server_cert_path);
    let server_key = load_private_key(server_key_path);

    let server_config = ServerConfig::builder_with_provider(Arc::new(mbedtls_crypto_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(server_cert_chain, server_key)?;

    let server_config = Arc::new(server_config);
    let listener = TcpListener::bind(addr)?;
    println!("blob-store: listening on {addr}");

    loop {
        let (tcp_stream, peer_addr) = match listener.accept() {
            Ok(conn) => conn,
            Err(e) => return Err(e.into()),
        };
        let server_config = server_config.clone();
        let store = store.clone();

        std::thread::spawn(move || {
            if let Err(e) = handle_connection(tcp_stream, server_config, &store) {
                eprintln!("blob-store: failed for {peer_addr}: {e}");
            }
        });
    }
}

fn handle_connection(
    mut tcp_stream: TcpStream,
    server_config: Arc<ServerConfig>,
    store: &BlobStore,
) -> Result<(), Box<dyn Error>> {
    let mut server_conn = ServerConnection::new(server_config)?;
    while server_conn.is_handshaking() {
        server_conn.complete_io(&mut tcp_stream)?;
    }

    let mut tls_stream = StreamOwned::new(server_conn, tcp_stream);
    let raw = http_protocol::read_http_request(&mut tls_stream)?;
    let parsed = http_protocol::parse_http_request(&raw)?;
    let response = route_request(&parsed, store);
    let response_bytes = http_protocol::serialize_http_response(&response.to_http_response()?)?;
    tls_stream.write_all(&response_bytes)?;
    tls_stream.flush()?;
    Ok(())
}

fn route_request(request: &ParsedRequest, store: &BlobStore) -> BinaryHttpResponse {
    match (&request.method, request.path.as_str()) {
        (HttpMethod::Put, path) if path.starts_with("/blob/") => {
            let name = &path["/blob/".len()..];
            if name.is_empty() || name.contains('/') {
                return json_error(400, "blob name must be non-empty and contain no slashes");
            }
            handle_put(name, request, store)
        }
        (HttpMethod::Get, "/blobs") => handle_list(store),
        (HttpMethod::Get, path) if path.starts_with("/blob/") => {
            let rest = &path["/blob/".len()..];
            if let Some(name) = rest.strip_suffix("/checksum") {
                handle_get_checksum(name, store)
            } else {
                handle_get(rest, store)
            }
        }
        (HttpMethod::Delete, path) if path.starts_with("/blob/") => {
            let name = &path["/blob/".len()..];
            handle_delete(name, store)
        }
        _ => json_error(404, "not found"),
    }
}

fn handle_put(name: &str, request: &ParsedRequest, store: &BlobStore) -> BinaryHttpResponse {
    let data = match &request.raw_body {
        Some(b) if !b.is_empty() => b.clone(),
        _ => return json_error(400, "missing request body"),
    };

    let checksum = sha256_hex(&data);

    let entry = seal_blob(data, &checksum);

    store.lock().unwrap().insert(name.to_string(), entry);
    println!("blob-store: stored '{}' (checksum {})", name, &checksum[..16]);

    let body = serde_json::json!({ "checksum": checksum });
    BinaryHttpResponse::json(200, body.to_string())
}

fn handle_get(name: &str, store: &BlobStore) -> BinaryHttpResponse {
    let blobs = store.lock().unwrap();
    match blobs.get(name) {
        Some(entry) => {
            let plaintext = unseal_blob(entry);
            BinaryHttpResponse::octet_stream(plaintext)
        }
        None => json_error(404, "blob not found"),
    }
}

fn handle_get_checksum(name: &str, store: &BlobStore) -> BinaryHttpResponse {
    let blobs = store.lock().unwrap();
    match blobs.get(name) {
        Some(entry) => {
            let body = serde_json::json!({ "checksum": entry.checksum });
            BinaryHttpResponse::json(200, body.to_string())
        }
        None => json_error(404, "blob not found"),
    }
}

fn handle_delete(name: &str, store: &BlobStore) -> BinaryHttpResponse {
    let mut blobs = store.lock().unwrap();
    if blobs.remove(name).is_some() {
        println!("blob-store: deleted '{name}'");
        BinaryHttpResponse::json(200, r#"{"status":"deleted"}"#.to_string())
    } else {
        json_error(404, "blob not found")
    }
}

fn handle_list(store: &BlobStore) -> BinaryHttpResponse {
    let blobs = store.lock().unwrap();
    let names: Vec<&str> = blobs.keys().map(String::as_str).collect();
    let body = serde_json::json!({ "blobs": names });
    BinaryHttpResponse::json(200, body.to_string())
}

fn json_error(status: u16, message: &str) -> BinaryHttpResponse {
    let body = serde_json::json!({ "message": message });
    BinaryHttpResponse::json(status, body.to_string())
}

fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// SGX sealing: encrypt blobs at rest using EGETKEY-derived AES-256-GCM keys.
// Outside SGX the blobs are stored as plaintext.
// ---------------------------------------------------------------------------

#[cfg(target_env = "sgx")]
mod sealing {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes128Gcm, Nonce};
    use sgx_isa::{Attributes, ErrorCode, Keyname, Keypolicy, Keyrequest, Miscselect, Report};

    pub struct SealData {
        pub rand: [u8; 16],
        pub isvsvn: u16,
        pub cpusvn: [u8; 16],
        pub attributes: Attributes,
        pub miscselect: Miscselect,
        pub nonce: [u8; 12],
    }

    const BLOB_SEAL_LABEL: [u8; 16] = *b"sgxqkms-blobst\x00\x00";

    fn egetkey(label: [u8; 16], seal_data: &SealData) -> Result<[u8; 16], ErrorCode> {
        let mut keyid = [0u8; 32];
        let (label_dst, rand_dst) = keyid.split_at_mut(16);
        label_dst.copy_from_slice(&label);
        rand_dst.copy_from_slice(&seal_data.rand);

        Keyrequest {
            keyname: Keyname::Seal as _,
            keypolicy: Keypolicy::MRENCLAVE,
            isvsvn: seal_data.isvsvn,
            cpusvn: seal_data.cpusvn,
            attributemask: [!0; 2],
            keyid,
            miscmask: !0,
            ..Default::default()
        }
        .egetkey()
    }

    pub fn seal(plaintext: &[u8]) -> (Vec<u8>, SealData) {
        let report = Report::for_self();
        let mut rand = [0u8; 16];
        mbedtls::rng::Random::random(&mut mbedtls::rng::Rdrand, &mut rand)
            .expect("rdrand failed");
        let mut nonce_bytes = [0u8; 12];
        mbedtls::rng::Random::random(&mut mbedtls::rng::Rdrand, &mut nonce_bytes)
            .expect("rdrand failed");

        let seal_data = SealData {
            rand,
            isvsvn: report.isvsvn,
            cpusvn: report.cpusvn,
            attributes: report.attributes,
            miscselect: report.miscselect,
            nonce: nonce_bytes,
        };

        let key = egetkey(BLOB_SEAL_LABEL, &seal_data).expect("EGETKEY failed during seal");
        let cipher = Aes128Gcm::new_from_slice(&key).unwrap();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext).expect("AES-GCM seal failed");

        (ciphertext, seal_data)
    }

    pub fn unseal(ciphertext: &[u8], seal_data: &SealData) -> Vec<u8> {
        let report = Report::for_self();
        if report.attributes != seal_data.attributes
            || report.miscselect != seal_data.miscselect
        {
            panic!("enclave attributes changed; cannot unseal blob");
        }

        let key = egetkey(BLOB_SEAL_LABEL, seal_data).expect("EGETKEY failed during unseal");
        let cipher = Aes128Gcm::new_from_slice(&key).unwrap();
        let nonce = Nonce::from_slice(&seal_data.nonce);
        cipher
            .decrypt(nonce, ciphertext)
            .expect("AES-GCM unseal failed (data corrupted or wrong enclave)")
    }
}

#[cfg(target_env = "sgx")]
use sealing::SealData;

#[cfg(target_env = "sgx")]
fn seal_blob(data: Vec<u8>, _checksum: &str) -> BlobEntry {
    let (ciphertext, seal_data) = sealing::seal(&data);
    BlobEntry {
        data: ciphertext,
        checksum: _checksum.to_string(),
        seal_data,
    }
}

#[cfg(target_env = "sgx")]
fn unseal_blob(entry: &BlobEntry) -> Vec<u8> {
    sealing::unseal(&entry.data, &entry.seal_data)
}

#[cfg(not(target_env = "sgx"))]
fn seal_blob(data: Vec<u8>, checksum: &str) -> BlobEntry {
    BlobEntry {
        data,
        checksum: checksum.to_string(),
    }
}

#[cfg(not(target_env = "sgx"))]
fn unseal_blob(entry: &BlobEntry) -> Vec<u8> {
    entry.data.clone()
}
