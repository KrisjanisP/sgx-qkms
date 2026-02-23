//! CLI tool driving the API client
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use log::{debug, info};
// models may be unused if all inputs are primitive types
#[allow(unused_imports)]
use qkd014_server_gen::{
    models, ApiNoContext, Client, ContextWrapperExt,
    GetKeyResponse,
    GetKeySimpleResponse,
    GetKeyWithIdsResponse,
    GetKeyWithIdsSimpleResponse,
    GetStatusResponse,
};
use simple_logger::SimpleLogger;
use swagger::{AuthData, ContextBuilder, EmptyContext, Push, XSpanIdString};

type ClientContext = swagger::make_context_ty!(
    ContextBuilder,
    EmptyContext,
    Option<AuthData>,
    XSpanIdString
);

#[derive(Parser, Debug)]
#[clap(
    name = "ETSI QKD 014 REST-based Key Delivery API",
    version = "1.1.1",
    about = "CLI access to ETSI QKD 014 REST-based Key Delivery API"
)]
struct Cli {
    #[clap(subcommand)]
    operation: Operation,

    /// Address or hostname of the server hosting this API, including optional port
    #[clap(short = 'a', long, default_value = "http://localhost")]
    server_address: String,

    /// Path to the client private key if using client-side TLS authentication
    #[cfg(all(feature = "client-tls", not(any(target_os = "macos", target_os = "windows", target_os = "ios"))))]
    #[clap(long, requires_all(&["client_certificate", "server_certificate"]))]
    client_key: Option<String>,

    /// Path to the client's public certificate associated with the private key
    #[cfg(all(feature = "client-tls", not(any(target_os = "macos", target_os = "windows", target_os = "ios"))))]
    #[clap(long, requires_all(&["client_key", "server_certificate"]))]
    client_certificate: Option<String>,

    /// Path to CA certificate used to authenticate the server
    #[cfg(all(feature = "client-tls", not(any(target_os = "macos", target_os = "windows", target_os = "ios"))))]
    #[clap(long)]
    server_certificate: Option<String>,

    /// If set, write output to file instead of stdout
    #[clap(short, long)]
    output_file: Option<String>,

    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Parser, Debug)]
enum Operation {
    /// Get keys
    GetKey {
        /// URL-encoded SAE ID of slave SAE
        slave_sae_id: String,
        #[clap(value_parser = parse_json::<models::KeyRequest>)]
        key_request: Option<models::KeyRequest>,
    },
    /// Get keys (simple GET form)
    GetKeySimple {
        /// URL-encoded SAE ID of slave SAE
        slave_sae_id: String,
        /// Number of keys requested (default 1)
        number: Option<i32>,
        /// Size of each key in bits (default is key_size from Status). Some KMEs require a multiple of 8 and may return 400 with message \"size shall be a multiple of 8\". 
        size: Option<i32>,
    },
    /// Get keys with key IDs
    GetKeyWithIds {
        /// URL-encoded SAE ID of master SAE
        master_sae_id: String,
        #[clap(value_parser = parse_json::<models::KeyIds>)]
        key_ids: models::KeyIds,
    },
    /// Get keys with key ID (simple GET form)
    GetKeyWithIdsSimple {
        /// URL-encoded SAE ID of master SAE
        master_sae_id: String,
        /// ID of the key (UUID)
        #[clap(value_parser = parse_json::<uuid::Uuid>)]
        key_id: uuid::Uuid,
    },
    /// Get status of keys available
    GetStatus {
        /// URL-encoded SAE ID of slave SAE
        slave_sae_id: String,
    },
}

// On Linux/Unix with OpenSSL (client-tls feature), support certificate pinning and mutual TLS
#[cfg(all(feature = "client-tls", not(any(target_os = "macos", target_os = "windows", target_os = "ios"))))]
fn create_client(args: &Cli, context: ClientContext) -> Result<Box<dyn ApiNoContext<ClientContext>>> {
    if args.client_certificate.is_some() {
        debug!("Using mutual TLS");
        let client = Client::try_new_https_mutual(
            &args.server_address,
            args.server_certificate.clone().unwrap(),
            args.client_key.clone().unwrap(),
            args.client_certificate.clone().unwrap(),
        )
        .context("Failed to create HTTPS client")?;
        Ok(Box::new(client.with_context(context)))
    } else if args.server_certificate.is_some() {
        debug!("Using TLS with pinned server certificate");
        let client =
            Client::try_new_https_pinned(&args.server_address, args.server_certificate.clone().unwrap())
                .context("Failed to create HTTPS client")?;
        Ok(Box::new(client.with_context(context)))
    } else {
        debug!("Using client without certificates");
        let client =
            Client::try_new(&args.server_address).context("Failed to create HTTP(S) client")?;
        Ok(Box::new(client.with_context(context)))
    }
}

// On macOS/Windows/iOS or without client-tls feature, use simple client (no cert pinning/mutual TLS)
#[cfg(any(
    not(feature = "client-tls"),
    all(feature = "client-tls", any(target_os = "macos", target_os = "windows", target_os = "ios"))
))]
fn create_client(args: &Cli, context: ClientContext) -> Result<Box<dyn ApiNoContext<ClientContext>>> {
    // Client::try_new() automatically detects the URL scheme (http:// or https://)
    // and creates the appropriate client.
    // Note: Certificate pinning and mutual TLS are only available on Linux/Unix with OpenSSL
    let client =
        Client::try_new(&args.server_address).context("Failed to create HTTP(S) client")?;
    Ok(Box::new(client.with_context(context)))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    if let Some(log_level) = args.verbosity.log_level() {
        SimpleLogger::new().with_level(log_level.to_level_filter()).init()?;
    }

    debug!("Arguments: {:?}", &args);

    let auth_data: Option<AuthData> = None;

    #[allow(trivial_casts)]
    let context = swagger::make_context!(
        ContextBuilder,
        EmptyContext,
        auth_data,
        XSpanIdString::default()
    );

    let client = create_client(&args, context)?;

    let result = match args.operation {
        Operation::GetKey {
            slave_sae_id,
            key_request,
        } => {
            info!("Performing a GetKey request on {:?}", (
                &slave_sae_id
            ));

            let result = client.get_key(
                slave_sae_id,
                key_request,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetKeyResponse::KeysRetrievedSuccessfully
                (body)
                => "KeysRetrievedSuccessfully\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetKeyResponse::BadRequestFormat
                (body)
                => "BadRequestFormat\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetKeyResponse::Unauthorized
                => "Unauthorized\n".to_string()
                    ,
                GetKeyResponse::ErrorOnServerSide
                (body)
                => "ErrorOnServerSide\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetKeySimple {
            slave_sae_id,
            number,
            size,
        } => {
            info!("Performing a GetKeySimple request on {:?}", (
                &slave_sae_id
            ));

            let result = client.get_key_simple(
                slave_sae_id,
                number,
                size,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetKeySimpleResponse::KeysRetrievedSuccessfully
                (body)
                => "KeysRetrievedSuccessfully\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetKeySimpleResponse::BadRequestFormat
                (body)
                => "BadRequestFormat\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetKeySimpleResponse::Unauthorized
                => "Unauthorized\n".to_string()
                    ,
                GetKeySimpleResponse::ErrorOnServerSide
                (body)
                => "ErrorOnServerSide\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetKeyWithIds {
            master_sae_id,
            key_ids,
        } => {
            info!("Performing a GetKeyWithIds request on {:?}", (
                &master_sae_id
            ));

            let result = client.get_key_with_ids(
                master_sae_id,
                key_ids,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetKeyWithIdsResponse::KeysRetrievedSuccessfully
                (body)
                => "KeysRetrievedSuccessfully\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetKeyWithIdsResponse::BadRequestFormat
                (body)
                => "BadRequestFormat\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetKeyWithIdsResponse::Unauthorized
                => "Unauthorized\n".to_string()
                    ,
                GetKeyWithIdsResponse::ErrorOnServerSide
                (body)
                => "ErrorOnServerSide\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetKeyWithIdsSimple {
            master_sae_id,
            key_id,
        } => {
            info!("Performing a GetKeyWithIdsSimple request on {:?}", (
                &master_sae_id
            ));

            let result = client.get_key_with_ids_simple(
                master_sae_id,
                key_id,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetKeyWithIdsSimpleResponse::KeysRetrievedSuccessfully
                (body)
                => "KeysRetrievedSuccessfully\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetKeyWithIdsSimpleResponse::BadRequestFormat
                (body)
                => "BadRequestFormat\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetKeyWithIdsSimpleResponse::Unauthorized
                => "Unauthorized\n".to_string()
                    ,
                GetKeyWithIdsSimpleResponse::ErrorOnServerSide
                (body)
                => "ErrorOnServerSide\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetStatus {
            slave_sae_id,
        } => {
            info!("Performing a GetStatus request on {:?}", (
                &slave_sae_id
            ));

            let result = client.get_status(
                slave_sae_id,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetStatusResponse::StatusRetrievedSuccessfully
                (body)
                => "StatusRetrievedSuccessfully\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetStatusResponse::BadRequestFormat
                (body)
                => "BadRequestFormat\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetStatusResponse::Unauthorized
                => "Unauthorized\n".to_string()
                    ,
                GetStatusResponse::ErrorOnServerSide
                (body)
                => "ErrorOnServerSide\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
    };

    if let Some(output_file) = args.output_file {
        std::fs::write(output_file, result)?
    } else {
        println!("{}", result);
    }
    Ok(())
}

// May be unused if all inputs are primitive types
#[allow(dead_code)]
fn parse_json<T: serde::de::DeserializeOwned>(json_string: &str) -> Result<T> {
    serde_json::from_str(json_string).map_err(|err| anyhow!("Error parsing input: {}", err))
}
