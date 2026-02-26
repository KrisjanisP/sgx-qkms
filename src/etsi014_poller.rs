use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use rustls_mbedcrypto_provider::mbedtls_crypto_provider;

use crate::api_models::KeyContainer;
use crate::http_protocol;
use crate::key_store::{KeyGatherer, KeyStore};
use crate::{load_ca_cert, load_certs, load_private_key};

pub struct Etsi014Poller {
    pub host: String,
    pub port: u16,
    pub server_name: String,
    pub client_cert_path: String,
    pub client_key_path: String,
    pub slave_sae_id: String,
    pub number: usize,
    pub size: usize,
    pub interval: Duration,
    pub reservable: bool,
}

impl KeyGatherer for Etsi014Poller {
    fn run(&self, store: Arc<KeyStore>) {
        let ca_store = load_ca_cert();
        let client_certs = load_certs(&self.client_cert_path);
        let client_key = load_private_key(&self.client_key_path);

        let tls_config = ClientConfig::builder_with_provider(Arc::new(mbedtls_crypto_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .expect("failed to configure TLS versions for poller")
            .with_root_certificates(ca_store)
            .with_client_auth_cert(client_certs, client_key)
            .expect("failed to configure client auth for poller");

        let tls_config = Arc::new(tls_config);
        let addr = format!("{}:{}", self.host, self.port);

        loop {
            match self.poll_once(&addr, &tls_config) {
                Ok(container) => {
                    let mut added = 0;
                    for key in &container.keys {
                        store.add_key(&key.key_id, &key.key, self.reservable);
                        added += 1;
                    }
                    if added > 0 {
                        println!(
                            "poller: added {added} keys (reservable={}, store total available={})",
                            self.reservable,
                            store.available_count()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("poller: failed to fetch keys: {e}");
                }
            }
            std::thread::sleep(self.interval);
        }
    }
}

impl Etsi014Poller {
    fn poll_once(
        &self,
        addr: &str,
        tls_config: &Arc<ClientConfig>,
    ) -> Result<KeyContainer, Box<dyn std::error::Error>> {
        let mut tcp = TcpStream::connect(addr)?;
        let server_name = ServerName::try_from(self.server_name.clone())?;
        let mut conn = ClientConnection::new(tls_config.clone(), server_name)?;
        while conn.is_handshaking() {
            conn.complete_io(&mut tcp)?;
        }
        let mut tls = StreamOwned::new(conn, tcp);

        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri(format!(
                "/api/v1/keys/{}/enc_keys?number={}&size={}",
                self.slave_sae_id, self.number, self.size
            ))
            .header(http::header::HOST, &self.host)
            .body(Vec::new())?;
        let raw_request = http_protocol::serialize_http_request(&request)?;
        tls.write_all(&raw_request)?;
        tls.flush()?;

        let raw_response = http_protocol::read_http_response(&mut tls)?;
        let response = http_protocol::parse_http_response_message(&raw_response)?;
        if !response.status().is_success() {
            let body = String::from_utf8_lossy(response.body());
            return Err(format!("poller HTTP {}: {body}", response.status()).into());
        }

        let container: KeyContainer = serde_json::from_slice(response.body())?;
        Ok(container)
    }
}
