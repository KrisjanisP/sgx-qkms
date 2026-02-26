use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

use crate::api_models::{ErrorResponse, KeyContainer, KeyItem, Status};
use crate::http_protocol::{HttpMethod, HttpResponse, ParsedRequest};
use crate::key_store::KeyStore;

pub fn route_request(
    request: &ParsedRequest,
    client_identity: &str,
    store: &Arc<KeyStore>,
) -> HttpResponse {
    let endpoint = classify_endpoint(&request.path);

    match (&request.method, endpoint) {
        (HttpMethod::Get, Endpoint::Status { slave_sae_id }) => {
            handle_get_status(slave_sae_id, client_identity, store)
        }
        (HttpMethod::Get | HttpMethod::Post, Endpoint::EncKeys { slave_sae_id }) => {
            handle_enc_keys(store, &request.query_params, slave_sae_id, client_identity)
        }
        (HttpMethod::Get | HttpMethod::Post, Endpoint::DecKeys { master_sae_id }) => {
            handle_dec_keys(store, &request.query_params, master_sae_id, client_identity)
        }
        (HttpMethod::Other, Endpoint::Status { .. })
        | (HttpMethod::Other, Endpoint::EncKeys { .. })
        | (HttpMethod::Other, Endpoint::DecKeys { .. })
        | (_, Endpoint::Status { .. }) => method_not_allowed(),
        (_, Endpoint::Unknown) => handle_not_found(),
    }
}

fn handle_get_status(
    slave_sae_id: &str,
    client_identity: &str,
    store: &Arc<KeyStore>,
) -> HttpResponse {
    let status = Status {
        source_kme_id: "placeholder-source-kme".to_string(),
        target_kme_id: "placeholder-target-kme".to_string(),
        master_sae_id: client_identity.to_string(),
        slave_sae_id: slave_sae_id.to_string(),
        key_size: 256,
        stored_key_count: store.available_count() as i32,
        max_key_count: 100,
        max_key_per_request: 10,
        max_key_size: 512,
        min_key_size: 128,
        max_sae_id_count: 0,
        status_extension: None,
    };
    json_response(200, &status)
}

fn handle_enc_keys(
    store: &Arc<KeyStore>,
    query_params: &HashMap<String, String>,
    _slave_sae_id: &str,
    _client_identity: &str,
) -> HttpResponse {
    let count: usize = query_params
        .get("number")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let reserved = store.reserve_keys(count);
    if reserved.is_empty() {
        return error_response(400, "no keys available for reservation");
    }

    let keys = reserved
        .into_iter()
        .map(|(id, val)| KeyItem {
            key_id: id,
            key_id_extension: None,
            key: val,
            key_extension: None,
        })
        .collect();

    let container = KeyContainer {
        keys,
        key_container_extension: None,
    };
    json_response(200, &container)
}

fn handle_dec_keys(
    store: &Arc<KeyStore>,
    query_params: &HashMap<String, String>,
    _master_sae_id: &str,
    _client_identity: &str,
) -> HttpResponse {
    let key_id = match query_params.get("key_ID") {
        Some(id) => id,
        None => return error_response(400, "missing key_ID parameter"),
    };

    match store.retrieve_key(key_id) {
        Some((id, val)) => {
            let container = KeyContainer {
                keys: vec![KeyItem {
                    key_id: id,
                    key_id_extension: None,
                    key: val,
                    key_extension: None,
                }],
                key_container_extension: None,
            };
            json_response(200, &container)
        }
        None => error_response(400, "key not found"),
    }
}

fn handle_not_found() -> HttpResponse {
    error_response(404, "Not Found")
}

fn method_not_allowed() -> HttpResponse {
    error_response(405, "Method Not Allowed")
}

fn error_response(status: u16, message: &str) -> HttpResponse {
    let payload = ErrorResponse {
        message: message.to_string(),
        details: None,
    };
    json_response(status, &payload)
}

fn json_response<T: Serialize>(status: u16, payload: &T) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json",
        body: serde_json::to_string(payload).unwrap_or_else(|_| {
            "{\"message\":\"serialization error\",\"details\":null}".to_string()
        }),
    }
}

enum Endpoint<'a> {
    Status { slave_sae_id: &'a str },
    EncKeys { slave_sae_id: &'a str },
    DecKeys { master_sae_id: &'a str },
    Unknown,
}

fn classify_endpoint(path: &str) -> Endpoint<'_> {
    if let Some(slave_sae_id) = extract_path_param(path, "/api/v1/keys/", "/status") {
        return Endpoint::Status { slave_sae_id };
    }

    if let Some(slave_sae_id) = extract_path_param(path, "/api/v1/keys/", "/enc_keys") {
        return Endpoint::EncKeys { slave_sae_id };
    }

    if let Some(master_sae_id) = extract_path_param(path, "/api/v1/keys/", "/dec_keys") {
        return Endpoint::DecKeys { master_sae_id };
    }

    Endpoint::Unknown
}

fn extract_path_param<'a>(path: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    if !path.starts_with(prefix) || !path.ends_with(suffix) {
        return None;
    }

    let start = prefix.len();
    let end = path.len().saturating_sub(suffix.len());
    if end <= start {
        return None;
    }
    Some(&path[start..end])
}
