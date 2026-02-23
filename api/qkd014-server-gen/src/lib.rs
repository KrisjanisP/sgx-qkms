#![allow(missing_docs, trivial_casts, unused_variables, unused_mut, unused_imports, unused_extern_crates, unused_attributes, non_camel_case_types)]
#![allow(clippy::derive_partial_eq_without_eq, clippy::disallowed_names)]

use async_trait::async_trait;
use futures::Stream;
#[cfg(feature = "mock")]
use mockall::automock;
use std::error::Error;
use std::collections::BTreeSet;
use std::task::{Poll, Context};
use swagger::{ApiError, ContextWrapper, auth::Authorization};
use serde::{Serialize, Deserialize};

#[cfg(any(feature = "client", feature = "server"))]
type ServiceError = Box<dyn Error + Send + Sync + 'static>;

pub const BASE_PATH: &str = "";
pub const API_VERSION: &str = "1.1.1";

mod auth;
pub use auth::{AuthenticationApi, Claims};


#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetKeyResponse {
    /// Keys retrieved successfully
    KeysRetrievedSuccessfully
    (models::KeyContainer)
    ,
    /// Bad request format
    BadRequestFormat
    (models::Error)
    ,
    /// Unauthorized
    Unauthorized
    ,
    /// Error on server side
    ErrorOnServerSide
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetKeySimpleResponse {
    /// Keys retrieved successfully
    KeysRetrievedSuccessfully
    (models::KeyContainer)
    ,
    /// Bad request format
    BadRequestFormat
    (models::Error)
    ,
    /// Unauthorized
    Unauthorized
    ,
    /// Error on server side
    ErrorOnServerSide
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetKeyWithIdsResponse {
    /// Keys retrieved successfully
    KeysRetrievedSuccessfully
    (models::KeyContainer)
    ,
    /// Bad request format
    BadRequestFormat
    (models::Error)
    ,
    /// Unauthorized
    Unauthorized
    ,
    /// Error on server side
    ErrorOnServerSide
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetKeyWithIdsSimpleResponse {
    /// Keys retrieved successfully
    KeysRetrievedSuccessfully
    (models::KeyContainer)
    ,
    /// Bad request format
    BadRequestFormat
    (models::Error)
    ,
    /// Unauthorized
    Unauthorized
    ,
    /// Error on server side
    ErrorOnServerSide
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetStatusResponse {
    /// Status retrieved successfully
    StatusRetrievedSuccessfully
    (models::Status)
    ,
    /// Bad request format
    BadRequestFormat
    (models::Error)
    ,
    /// Unauthorized
    Unauthorized
    ,
    /// Error on server side
    ErrorOnServerSide
    (models::Error)
}

/// API
#[cfg_attr(feature = "mock", automock)]
#[async_trait]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub trait Api<C: Send + Sync> {
    /// Get keys
    async fn get_key(
        &self,
        slave_sae_id: String,
        key_request: Option<models::KeyRequest>,
        context: &C) -> Result<GetKeyResponse, ApiError>;

    /// Get keys (simple GET form)
    async fn get_key_simple(
        &self,
        slave_sae_id: String,
        number: Option<i32>,
        size: Option<i32>,
        context: &C) -> Result<GetKeySimpleResponse, ApiError>;

    /// Get keys with key IDs
    async fn get_key_with_ids(
        &self,
        master_sae_id: String,
        key_ids: models::KeyIds,
        context: &C) -> Result<GetKeyWithIdsResponse, ApiError>;

    /// Get keys with key ID (simple GET form)
    async fn get_key_with_ids_simple(
        &self,
        master_sae_id: String,
        key_id: uuid::Uuid,
        context: &C) -> Result<GetKeyWithIdsSimpleResponse, ApiError>;

    /// Get status of keys available
    async fn get_status(
        &self,
        slave_sae_id: String,
        context: &C) -> Result<GetStatusResponse, ApiError>;

}

/// API where `Context` isn't passed on every API call
#[cfg_attr(feature = "mock", automock)]
#[async_trait]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub trait ApiNoContext<C: Send + Sync> {
    // The std::task::Context struct houses a reference to std::task::Waker with the lifetime <'a>.
    // Adding an anonymous lifetime `'a` to allow mockall to create a mock object with the right lifetimes.
    // This is needed because the compiler is unable to determine the lifetimes on F's trait bound
    // where F is the closure created by mockall. We use higher-rank trait bounds here to get around this.

    fn context(&self) -> &C;

    /// Get keys
    async fn get_key(
        &self,
        slave_sae_id: String,
        key_request: Option<models::KeyRequest>,
        ) -> Result<GetKeyResponse, ApiError>;

    /// Get keys (simple GET form)
    async fn get_key_simple(
        &self,
        slave_sae_id: String,
        number: Option<i32>,
        size: Option<i32>,
        ) -> Result<GetKeySimpleResponse, ApiError>;

    /// Get keys with key IDs
    async fn get_key_with_ids(
        &self,
        master_sae_id: String,
        key_ids: models::KeyIds,
        ) -> Result<GetKeyWithIdsResponse, ApiError>;

    /// Get keys with key ID (simple GET form)
    async fn get_key_with_ids_simple(
        &self,
        master_sae_id: String,
        key_id: uuid::Uuid,
        ) -> Result<GetKeyWithIdsSimpleResponse, ApiError>;

    /// Get status of keys available
    async fn get_status(
        &self,
        slave_sae_id: String,
        ) -> Result<GetStatusResponse, ApiError>;

}

/// Trait to extend an API to make it easy to bind it to a context.
pub trait ContextWrapperExt<C: Send + Sync> where Self: Sized
{
    /// Binds this API to a context.
    fn with_context(self, context: C) -> ContextWrapper<Self, C>;
}

impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ContextWrapperExt<C> for T {
    fn with_context(self: T, context: C) -> ContextWrapper<T, C> {
         ContextWrapper::<T, C>::new(self, context)
    }
}

#[async_trait]
impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ApiNoContext<C> for ContextWrapper<T, C> {
    fn context(&self) -> &C {
        ContextWrapper::context(self)
    }

    /// Get keys
    async fn get_key(
        &self,
        slave_sae_id: String,
        key_request: Option<models::KeyRequest>,
        ) -> Result<GetKeyResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_key(slave_sae_id, key_request, &context).await
    }

    /// Get keys (simple GET form)
    async fn get_key_simple(
        &self,
        slave_sae_id: String,
        number: Option<i32>,
        size: Option<i32>,
        ) -> Result<GetKeySimpleResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_key_simple(slave_sae_id, number, size, &context).await
    }

    /// Get keys with key IDs
    async fn get_key_with_ids(
        &self,
        master_sae_id: String,
        key_ids: models::KeyIds,
        ) -> Result<GetKeyWithIdsResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_key_with_ids(master_sae_id, key_ids, &context).await
    }

    /// Get keys with key ID (simple GET form)
    async fn get_key_with_ids_simple(
        &self,
        master_sae_id: String,
        key_id: uuid::Uuid,
        ) -> Result<GetKeyWithIdsSimpleResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_key_with_ids_simple(master_sae_id, key_id, &context).await
    }

    /// Get status of keys available
    async fn get_status(
        &self,
        slave_sae_id: String,
        ) -> Result<GetStatusResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_status(slave_sae_id, &context).await
    }

}


#[cfg(feature = "client")]
pub mod client;

// Re-export Client as a top-level name
#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub mod server;

// Re-export router() as a top-level name
#[cfg(feature = "server")]
pub use self::server::Service;

#[cfg(feature = "server")]
pub mod context;

pub mod models;

#[cfg(any(feature = "client", feature = "server"))]
pub(crate) mod header;
