use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollRequest {
    pub csr_pem: String,
    pub node_id: String,
    pub nonce: String,
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollStatus {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
}
