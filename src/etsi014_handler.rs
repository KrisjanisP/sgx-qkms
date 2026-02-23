use async_trait::async_trait;
use qkd014_server_gen::models;
use qkd014_server_gen::{
    Api, GetKeyResponse, GetKeySimpleResponse, GetKeyWithIdsResponse, GetKeyWithIdsSimpleResponse,
    GetStatusResponse,
};
use swagger::{ApiError, Has, XSpanIdString};

/// Placeholder ETSI GS QKD 014 handler.
///
/// All endpoints currently return a generated 503 response with a descriptive
/// payload until business logic is implemented.
#[derive(Debug, Clone, Default)]
pub struct Etsi014Handler;

#[derive(Debug, Clone)]
pub struct RequestContext {
    span_id: XSpanIdString,
    pub client_identity: String,
}

impl RequestContext {
    pub fn new(client_identity: String) -> Self {
        Self {
            span_id: XSpanIdString::default(),
            client_identity,
        }
    }
}

impl Has<XSpanIdString> for RequestContext {
    fn get(&self) -> &XSpanIdString {
        &self.span_id
    }

    fn get_mut(&mut self) -> &mut XSpanIdString {
        &mut self.span_id
    }

    fn set(&mut self, value: XSpanIdString) {
        self.span_id = value;
    }
}

#[async_trait]
impl Api<RequestContext> for Etsi014Handler {
    async fn get_key(
        &self,
        _slave_sae_id: String,
        _key_request: Option<models::KeyRequest>,
        _context: &RequestContext,
    ) -> Result<GetKeyResponse, ApiError> {
        Ok(GetKeyResponse::ErrorOnServerSide(
            models::Error::new("GetKey is not implemented yet".to_string()),
        ))
    }

    async fn get_key_simple(
        &self,
        _slave_sae_id: String,
        _number: Option<u32>,
        _size: Option<u32>,
        _context: &RequestContext,
    ) -> Result<GetKeySimpleResponse, ApiError> {
        Ok(GetKeySimpleResponse::ErrorOnServerSide(
            models::Error::new("GetKeySimple is not implemented yet".to_string()),
        ))
    }

    async fn get_key_with_ids(
        &self,
        _master_sae_id: String,
        _key_ids: models::KeyIds,
        _context: &RequestContext,
    ) -> Result<GetKeyWithIdsResponse, ApiError> {
        Ok(GetKeyWithIdsResponse::ErrorOnServerSide(
            models::Error::new("GetKeyWithIds is not implemented yet".to_string()),
        ))
    }

    async fn get_key_with_ids_simple(
        &self,
        _master_sae_id: String,
        _key_id: uuid::Uuid,
        _context: &RequestContext,
    ) -> Result<GetKeyWithIdsSimpleResponse, ApiError> {
        Ok(GetKeyWithIdsSimpleResponse::ErrorOnServerSide(
            models::Error::new("GetKeyWithIdsSimple is not implemented yet".to_string()),
        ))
    }

    async fn get_status(
        &self,
        slave_sae_id: String,
        context: &RequestContext,
    ) -> Result<GetStatusResponse, ApiError> {
        let status = models::Status::new(
            "placeholder-source-kme".to_string(),
            "placeholder-target-kme".to_string(),
            context.client_identity.clone(),
            slave_sae_id,
            256,
            0,
            100,
            10,
            512,
            128,
            0,
        );
        Ok(GetStatusResponse::StatusRetrievedSuccessfully(status))
    }
}

