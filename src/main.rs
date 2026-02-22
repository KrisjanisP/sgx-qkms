use rustls::RootCertStore;
use std::{fs::File, io::BufReader};

#[tokio::main]
async fn main() {
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

    println!(
        "Loaded {added} CA cert(s) from {CA_CERT_PATH} (ignored: {ignored})"
    );
}
