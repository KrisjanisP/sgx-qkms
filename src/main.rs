use rustls::{
    ClientConfig, RootCertStore, ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, ServerName},
    server::WebPkiClientVerifier,
};
use std::{env, fs::File, io::BufReader, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};

fn load_ca_cert() -> RootCertStore {
    const CA_CERT_PATH: &str = "certs/ca/ca.crt";

    let cert_file = File::open(CA_CERT_PATH)
        .unwrap_or_else(|e| panic!("Failed to open CA cert at {CA_CERT_PATH}: {e}"));
    let mut cert_reader = BufReader::new(cert_file);

    let certs = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|e| panic!("Failed to parse PEM certs from {CA_CERT_PATH}: {e}"));

    let mut root_store = RootCertStore::empty();
    let (added, ignored) = root_store.add_parsable_certificates(certs);
    if added == 0 {
        panic!("No CA certificates were loaded from {CA_CERT_PATH}");
    }
    if ignored > 0 {
        panic!("Ignored {ignored} CA certificates from {CA_CERT_PATH}");
    }

    root_store
}

fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
    let cert_file =
        File::open(path).unwrap_or_else(|e| panic!("Failed to open cert file at {path}: {e}"));
    let mut cert_reader = BufReader::new(cert_file);

    rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|e| panic!("Failed to parse certs from {path}: {e}"))
}

fn load_private_key(path: &str) -> PrivateKeyDer<'static> {
    let key_file =
        File::open(path).unwrap_or_else(|e| panic!("Failed to open key file at {path}: {e}"));
    let mut key_reader = BufReader::new(key_file);

    rustls_pemfile::private_key(&mut key_reader)
        .unwrap_or_else(|e| panic!("Failed to parse private key from {path}: {e}"))
        .unwrap_or_else(|| panic!("No private key found in {path}"))
}

async fn run_sample_server() -> Result<(), Box<dyn std::error::Error>> {
    const ADDR: &str = "127.0.0.1:8443";
    const SERVER_CERT_PATH: &str = "certs/sae/server.crt";
    const SERVER_KEY_PATH: &str = "certs/sae/server.key";

    let ca_cert_store = load_ca_cert();
    let server_cert_chain = load_certs(SERVER_CERT_PATH);
    let server_key = load_private_key(SERVER_KEY_PATH);

    let client_verifier = WebPkiClientVerifier::builder(Arc::new(ca_cert_store)).build()?;
    let server_config = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(server_cert_chain, server_key)?;

    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind(ADDR).await?;
    println!("sample mTLS server listening on {ADDR}");

    loop {
        let (tcp_stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => return Err(e.into()),
        };
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            let mut tls_stream = match acceptor.accept(tcp_stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("TLS accept failed for {peer_addr}: {e}");
                    return;
                }
            };

            let mut buf = [0_u8; 1024];
            let n = match tls_stream.read(&mut buf).await {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Read failed for {peer_addr}: {e}");
                    return;
                }
            };

            let name = String::from_utf8_lossy(&buf[..n]).trim().to_string();
            let response = format!("hello, {name}\n");
            if let Err(e) = tls_stream.write_all(response.as_bytes()).await {
                eprintln!("Write failed for {peer_addr}: {e}");
                return;
            }
            if let Err(e) = tls_stream.shutdown().await {
                eprintln!("Shutdown failed for {peer_addr}: {e}");
                return;
            }

            println!("served greeting for '{name}' from {peer_addr}");
        });
    }
}

async fn run_sample_client(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    const ADDR: &str = "127.0.0.1:8443";
    const CLIENT_CERT_PATH: &str = "certs/sae/client.crt";
    const CLIENT_KEY_PATH: &str = "certs/sae/client.key";

    let ca_cert_store = load_ca_cert();
    let client_cert_chain = load_certs(CLIENT_CERT_PATH);
    let client_key = load_private_key(CLIENT_KEY_PATH);

    let client_config = ClientConfig::builder()
        .with_root_certificates(ca_cert_store)
        .with_client_auth_cert(client_cert_chain, client_key)?;

    let connector = TlsConnector::from(Arc::new(client_config));
    let tcp_stream = TcpStream::connect(ADDR).await?;
    let server_name = ServerName::try_from("localhost")?;
    let mut tls_stream = connector.connect(server_name, tcp_stream).await?;

    tls_stream.write_all(format!("{name}\n").as_bytes()).await?;
    tls_stream.shutdown().await?;

    let mut response = Vec::new();
    tls_stream.read_to_end(&mut response).await?;
    println!("{}", String::from_utf8_lossy(&response).trim_end());

    Ok(())
}

#[tokio::main]
async fn main() {
    let mut args = env::args();
    let _program = args.next();
    let mode = args.next();

    match mode.as_deref() {
        Some("server") => {
            if let Err(e) = run_sample_server().await {
                eprintln!("server error: {e}");
                std::process::exit(1);
            }
        }
        Some("client") => {
            let name = args.next().unwrap_or_else(|| "world".to_string());
            if let Err(e) = run_sample_client(&name).await {
                eprintln!("client error: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  sgx-qkms server");
            eprintln!("  sgx-qkms client <name>");
            std::process::exit(1);
        }
    }
}
