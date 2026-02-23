#![allow(unused_qualifications)]
#[cfg(not(feature = "validate"))]
use validator::Validate;

use crate::models;
#[cfg(any(feature = "client", feature = "server"))]
use crate::header;
#[cfg(feature = "validate")]
use serde_valid::Validate;

#[derive(Debug, Clone, PartialEq, Validate, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Error {
    #[serde(rename = "message")]

    pub message: String,

    #[serde(rename = "details")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub details: Option<Vec<std::collections::HashMap<String, serde_json::Value>>>,

}


impl Error {
    #[allow(clippy::new_without_default)]
    pub fn new(message: String, ) -> Error {
        Error {
            message,
            details: None,
        }
    }
}

/// Converts the Error value to the Query Parameters representation (style=form, explode=false)
/// specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde serializer
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("message".to_string()),
            Some(self.message.to_string()),
            // Skipping non-primitive type details in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Error value
/// as specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Error {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub message: Vec<String>,
            pub details: Vec<Vec<std::collections::HashMap<String, serde_json::Value>>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing Error".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "message" => intermediate_rep.message.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "details" => return std::result::Result::Err("Parsing a container in this style is not supported in Error".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing Error".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Error {
            message: intermediate_rep.message.into_iter().next().ok_or_else(|| "message missing in Error".to_string())?,
            details: intermediate_rep.details.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Error> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Error>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Error>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for Error - value: {hdr_value} is invalid {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Error> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <Error as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{value}' into Error - {err}"))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {hdr_value:?} to string: {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Vec<Error>>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_values: header::IntoHeaderValue<Vec<Error>>) -> std::result::Result<Self, Self::Error> {
        let hdr_values : Vec<String> = hdr_values.0.into_iter().map(|hdr_value| {
            hdr_value.to_string()
        }).collect();

        match hyper::header::HeaderValue::from_str(&hdr_values.join(", ")) {
           std::result::Result::Ok(hdr_value) => std::result::Result::Ok(hdr_value),
           std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to convert {hdr_values:?} into a header - {e}",))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Vec<Error>> {
    type Error = String;

    fn try_from(hdr_values: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_values.to_str() {
            std::result::Result::Ok(hdr_values) => {
                let hdr_values : std::vec::Vec<Error> = hdr_values
                .split(',')
                .filter_map(|hdr_value| match hdr_value.trim() {
                    "" => std::option::Option::None,
                    hdr_value => std::option::Option::Some({
                        match <Error as std::str::FromStr>::from_str(hdr_value) {
                            std::result::Result::Ok(value) => std::result::Result::Ok(value),
                            std::result::Result::Err(err) => std::result::Result::Err(
                                format!("Unable to convert header value '{hdr_value}' into Error - {err}"))
                        }
                    })
                }).collect::<std::result::Result<std::vec::Vec<_>, String>>()?;

                std::result::Result::Ok(header::IntoHeaderValue(hdr_values))
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to parse header: {hdr_values:?} as a string - {e}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Validate, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct KeyContainer {
    #[serde(rename = "keys")]

    #[cfg_attr(feature = "validate", validate)]
    pub keys: Vec<models::KeyItem>,

    /// Optional, for future use
    #[serde(rename = "key_container_extension")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub key_container_extension: Option<std::collections::HashMap<String, serde_json::Value>>,

}


impl KeyContainer {
    #[allow(clippy::new_without_default)]
    pub fn new(keys: Vec<models::KeyItem>, ) -> KeyContainer {
        KeyContainer {
            keys,
            key_container_extension: None,
        }
    }
}

/// Converts the KeyContainer value to the Query Parameters representation (style=form, explode=false)
/// specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde serializer
impl std::fmt::Display for KeyContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping non-primitive type keys in query parameter serialization
            // Skipping map key_container_extension in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a KeyContainer value
/// as specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde deserializer
impl std::str::FromStr for KeyContainer {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub keys: Vec<Vec<models::KeyItem>>,
            pub key_container_extension: Vec<std::collections::HashMap<String, serde_json::Value>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing KeyContainer".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "keys" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyContainer".to_string()),
                    "key_container_extension" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyContainer".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing KeyContainer".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(KeyContainer {
            keys: intermediate_rep.keys.into_iter().next().ok_or_else(|| "keys missing in KeyContainer".to_string())?,
            key_container_extension: intermediate_rep.key_container_extension.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<KeyContainer> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<KeyContainer>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<KeyContainer>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for KeyContainer - value: {hdr_value} is invalid {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<KeyContainer> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <KeyContainer as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{value}' into KeyContainer - {err}"))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {hdr_value:?} to string: {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Vec<KeyContainer>>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_values: header::IntoHeaderValue<Vec<KeyContainer>>) -> std::result::Result<Self, Self::Error> {
        let hdr_values : Vec<String> = hdr_values.0.into_iter().map(|hdr_value| {
            hdr_value.to_string()
        }).collect();

        match hyper::header::HeaderValue::from_str(&hdr_values.join(", ")) {
           std::result::Result::Ok(hdr_value) => std::result::Result::Ok(hdr_value),
           std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to convert {hdr_values:?} into a header - {e}",))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Vec<KeyContainer>> {
    type Error = String;

    fn try_from(hdr_values: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_values.to_str() {
            std::result::Result::Ok(hdr_values) => {
                let hdr_values : std::vec::Vec<KeyContainer> = hdr_values
                .split(',')
                .filter_map(|hdr_value| match hdr_value.trim() {
                    "" => std::option::Option::None,
                    hdr_value => std::option::Option::Some({
                        match <KeyContainer as std::str::FromStr>::from_str(hdr_value) {
                            std::result::Result::Ok(value) => std::result::Result::Ok(value),
                            std::result::Result::Err(err) => std::result::Result::Err(
                                format!("Unable to convert header value '{hdr_value}' into KeyContainer - {err}"))
                        }
                    })
                }).collect::<std::result::Result<std::vec::Vec<_>, String>>()?;

                std::result::Result::Ok(header::IntoHeaderValue(hdr_values))
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to parse header: {hdr_values:?} as a string - {e}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Validate, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct KeyIdItem {
    /// ID of the key (UUID)
    #[serde(rename = "key_ID")]

    pub key_id: uuid::Uuid,

    /// Optional, for future use
    #[serde(rename = "key_ID_extension")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub key_id_extension: Option<std::collections::HashMap<String, serde_json::Value>>,

}


impl KeyIdItem {
    #[allow(clippy::new_without_default)]
    pub fn new(key_id: uuid::Uuid, ) -> KeyIdItem {
        KeyIdItem {
            key_id,
            key_id_extension: None,
        }
    }
}

/// Converts the KeyIdItem value to the Query Parameters representation (style=form, explode=false)
/// specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde serializer
impl std::fmt::Display for KeyIdItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping non-primitive type key_ID in query parameter serialization
            // Skipping map key_ID_extension in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a KeyIdItem value
/// as specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde deserializer
impl std::str::FromStr for KeyIdItem {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub key_id: Vec<uuid::Uuid>,
            pub key_id_extension: Vec<std::collections::HashMap<String, serde_json::Value>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing KeyIdItem".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "key_ID" => intermediate_rep.key_id.push(<uuid::Uuid as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "key_ID_extension" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyIdItem".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing KeyIdItem".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(KeyIdItem {
            key_id: intermediate_rep.key_id.into_iter().next().ok_or_else(|| "key_ID missing in KeyIdItem".to_string())?,
            key_id_extension: intermediate_rep.key_id_extension.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<KeyIdItem> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<KeyIdItem>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<KeyIdItem>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for KeyIdItem - value: {hdr_value} is invalid {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<KeyIdItem> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <KeyIdItem as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{value}' into KeyIdItem - {err}"))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {hdr_value:?} to string: {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Vec<KeyIdItem>>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_values: header::IntoHeaderValue<Vec<KeyIdItem>>) -> std::result::Result<Self, Self::Error> {
        let hdr_values : Vec<String> = hdr_values.0.into_iter().map(|hdr_value| {
            hdr_value.to_string()
        }).collect();

        match hyper::header::HeaderValue::from_str(&hdr_values.join(", ")) {
           std::result::Result::Ok(hdr_value) => std::result::Result::Ok(hdr_value),
           std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to convert {hdr_values:?} into a header - {e}",))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Vec<KeyIdItem>> {
    type Error = String;

    fn try_from(hdr_values: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_values.to_str() {
            std::result::Result::Ok(hdr_values) => {
                let hdr_values : std::vec::Vec<KeyIdItem> = hdr_values
                .split(',')
                .filter_map(|hdr_value| match hdr_value.trim() {
                    "" => std::option::Option::None,
                    hdr_value => std::option::Option::Some({
                        match <KeyIdItem as std::str::FromStr>::from_str(hdr_value) {
                            std::result::Result::Ok(value) => std::result::Result::Ok(value),
                            std::result::Result::Err(err) => std::result::Result::Err(
                                format!("Unable to convert header value '{hdr_value}' into KeyIdItem - {err}"))
                        }
                    })
                }).collect::<std::result::Result<std::vec::Vec<_>, String>>()?;

                std::result::Result::Ok(header::IntoHeaderValue(hdr_values))
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to parse header: {hdr_values:?} as a string - {e}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Validate, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct KeyIds {
    #[serde(rename = "key_IDs")]

    #[cfg_attr(feature = "validate", validate)]
    pub key_ids: Vec<models::KeyIdItem>,

    /// Optional, for future use
    #[serde(rename = "key_IDs_extension")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub key_ids_extension: Option<std::collections::HashMap<String, serde_json::Value>>,

}


impl KeyIds {
    #[allow(clippy::new_without_default)]
    pub fn new(key_ids: Vec<models::KeyIdItem>, ) -> KeyIds {
        KeyIds {
            key_ids,
            key_ids_extension: None,
        }
    }
}

/// Converts the KeyIds value to the Query Parameters representation (style=form, explode=false)
/// specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde serializer
impl std::fmt::Display for KeyIds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping non-primitive type key_IDs in query parameter serialization
            // Skipping map key_IDs_extension in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a KeyIds value
/// as specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde deserializer
impl std::str::FromStr for KeyIds {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub key_ids: Vec<Vec<models::KeyIdItem>>,
            pub key_ids_extension: Vec<std::collections::HashMap<String, serde_json::Value>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing KeyIds".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "key_IDs" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyIds".to_string()),
                    "key_IDs_extension" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyIds".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing KeyIds".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(KeyIds {
            key_ids: intermediate_rep.key_ids.into_iter().next().ok_or_else(|| "key_IDs missing in KeyIds".to_string())?,
            key_ids_extension: intermediate_rep.key_ids_extension.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<KeyIds> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<KeyIds>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<KeyIds>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for KeyIds - value: {hdr_value} is invalid {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<KeyIds> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <KeyIds as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{value}' into KeyIds - {err}"))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {hdr_value:?} to string: {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Vec<KeyIds>>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_values: header::IntoHeaderValue<Vec<KeyIds>>) -> std::result::Result<Self, Self::Error> {
        let hdr_values : Vec<String> = hdr_values.0.into_iter().map(|hdr_value| {
            hdr_value.to_string()
        }).collect();

        match hyper::header::HeaderValue::from_str(&hdr_values.join(", ")) {
           std::result::Result::Ok(hdr_value) => std::result::Result::Ok(hdr_value),
           std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to convert {hdr_values:?} into a header - {e}",))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Vec<KeyIds>> {
    type Error = String;

    fn try_from(hdr_values: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_values.to_str() {
            std::result::Result::Ok(hdr_values) => {
                let hdr_values : std::vec::Vec<KeyIds> = hdr_values
                .split(',')
                .filter_map(|hdr_value| match hdr_value.trim() {
                    "" => std::option::Option::None,
                    hdr_value => std::option::Option::Some({
                        match <KeyIds as std::str::FromStr>::from_str(hdr_value) {
                            std::result::Result::Ok(value) => std::result::Result::Ok(value),
                            std::result::Result::Err(err) => std::result::Result::Err(
                                format!("Unable to convert header value '{hdr_value}' into KeyIds - {err}"))
                        }
                    })
                }).collect::<std::result::Result<std::vec::Vec<_>, String>>()?;

                std::result::Result::Ok(header::IntoHeaderValue(hdr_values))
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to parse header: {hdr_values:?} as a string - {e}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Validate, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct KeyItem {
    /// ID of the key (UUID)
    #[serde(rename = "key_ID")]

    pub key_id: uuid::Uuid,

    /// Optional, for future use
    #[serde(rename = "key_ID_extension")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub key_id_extension: Option<std::collections::HashMap<String, serde_json::Value>>,

    /// Key data encoded by base64
    #[serde(rename = "key")]

    pub key: swagger::ByteArray,

    /// Optional, for future use
    #[serde(rename = "key_extension")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub key_extension: Option<std::collections::HashMap<String, serde_json::Value>>,

}


impl KeyItem {
    #[allow(clippy::new_without_default)]
    pub fn new(key_id: uuid::Uuid, key: swagger::ByteArray, ) -> KeyItem {
        KeyItem {
            key_id,
            key_id_extension: None,
            key,
            key_extension: None,
        }
    }
}

/// Converts the KeyItem value to the Query Parameters representation (style=form, explode=false)
/// specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde serializer
impl std::fmt::Display for KeyItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping non-primitive type key_ID in query parameter serialization
            // Skipping map key_ID_extension in query parameter serialization
            // Skipping byte array key in query parameter serialization
            // Skipping map key_extension in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a KeyItem value
/// as specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde deserializer
impl std::str::FromStr for KeyItem {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub key_id: Vec<uuid::Uuid>,
            pub key_id_extension: Vec<std::collections::HashMap<String, serde_json::Value>>,
            pub key: Vec<swagger::ByteArray>,
            pub key_extension: Vec<std::collections::HashMap<String, serde_json::Value>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing KeyItem".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "key_ID" => intermediate_rep.key_id.push(<uuid::Uuid as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "key_ID_extension" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyItem".to_string()),
                    "key" => return std::result::Result::Err("Parsing binary data in this style is not supported in KeyItem".to_string()),
                    "key_extension" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyItem".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing KeyItem".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(KeyItem {
            key_id: intermediate_rep.key_id.into_iter().next().ok_or_else(|| "key_ID missing in KeyItem".to_string())?,
            key_id_extension: intermediate_rep.key_id_extension.into_iter().next(),
            key: intermediate_rep.key.into_iter().next().ok_or_else(|| "key missing in KeyItem".to_string())?,
            key_extension: intermediate_rep.key_extension.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<KeyItem> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<KeyItem>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<KeyItem>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for KeyItem - value: {hdr_value} is invalid {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<KeyItem> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <KeyItem as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{value}' into KeyItem - {err}"))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {hdr_value:?} to string: {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Vec<KeyItem>>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_values: header::IntoHeaderValue<Vec<KeyItem>>) -> std::result::Result<Self, Self::Error> {
        let hdr_values : Vec<String> = hdr_values.0.into_iter().map(|hdr_value| {
            hdr_value.to_string()
        }).collect();

        match hyper::header::HeaderValue::from_str(&hdr_values.join(", ")) {
           std::result::Result::Ok(hdr_value) => std::result::Result::Ok(hdr_value),
           std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to convert {hdr_values:?} into a header - {e}",))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Vec<KeyItem>> {
    type Error = String;

    fn try_from(hdr_values: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_values.to_str() {
            std::result::Result::Ok(hdr_values) => {
                let hdr_values : std::vec::Vec<KeyItem> = hdr_values
                .split(',')
                .filter_map(|hdr_value| match hdr_value.trim() {
                    "" => std::option::Option::None,
                    hdr_value => std::option::Option::Some({
                        match <KeyItem as std::str::FromStr>::from_str(hdr_value) {
                            std::result::Result::Ok(value) => std::result::Result::Ok(value),
                            std::result::Result::Err(err) => std::result::Result::Err(
                                format!("Unable to convert header value '{hdr_value}' into KeyItem - {err}"))
                        }
                    })
                }).collect::<std::result::Result<std::vec::Vec<_>, String>>()?;

                std::result::Result::Ok(header::IntoHeaderValue(hdr_values))
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to parse header: {hdr_values:?} as a string - {e}")),
        }
    }
}

/// All fields are optional; the JSON body may be empty. GET is only permitted for empty request or only number/size (as query params). 
#[derive(Debug, Clone, PartialEq, Validate, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct KeyRequest {
    /// Number of keys requested (default 1)
    #[serde(rename = "number")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub number: Option<i32>,

    /// Size of each key in bits (default key_size from Status)
    #[serde(rename = "size")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub size: Option<i32>,

    /// Optional list of additional slave SAE IDs for key multicast
    #[serde(rename = "additional_slave_SAE_IDs")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub additional_slave_sae_ids: Option<Vec<String>>,

    /// Extensions that the KME must support or return 400
    #[serde(rename = "extension_mandatory")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub extension_mandatory: Option<Vec<std::collections::HashMap<String, serde_json::Value>>>,

    /// Extensions that the KME may ignore
    #[serde(rename = "extension_optional")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub extension_optional: Option<Vec<std::collections::HashMap<String, serde_json::Value>>>,

}


impl KeyRequest {
    #[allow(clippy::new_without_default)]
    pub fn new() -> KeyRequest {
        KeyRequest {
            number: None,
            size: None,
            additional_slave_sae_ids: None,
            extension_mandatory: None,
            extension_optional: None,
        }
    }
}

/// Converts the KeyRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde serializer
impl std::fmt::Display for KeyRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.number.as_ref().map(|number| {
                [
                    "number".to_string(),
                    number.to_string(),
                ].join(",")
            }),
            self.size.as_ref().map(|size| {
                [
                    "size".to_string(),
                    size.to_string(),
                ].join(",")
            }),
            self.additional_slave_sae_ids.as_ref().map(|additional_slave_sae_ids| {
                [
                    "additional_slave_SAE_IDs".to_string(),
                    additional_slave_sae_ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","),
                ].join(",")
            }),
            // Skipping non-primitive type extension_mandatory in query parameter serialization
            // Skipping non-primitive type extension_optional in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a KeyRequest value
/// as specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde deserializer
impl std::str::FromStr for KeyRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub number: Vec<i32>,
            pub size: Vec<i32>,
            pub additional_slave_sae_ids: Vec<Vec<String>>,
            pub extension_mandatory: Vec<Vec<std::collections::HashMap<String, serde_json::Value>>>,
            pub extension_optional: Vec<Vec<std::collections::HashMap<String, serde_json::Value>>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing KeyRequest".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "number" => intermediate_rep.number.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "size" => intermediate_rep.size.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "additional_slave_SAE_IDs" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyRequest".to_string()),
                    "extension_mandatory" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyRequest".to_string()),
                    "extension_optional" => return std::result::Result::Err("Parsing a container in this style is not supported in KeyRequest".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing KeyRequest".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(KeyRequest {
            number: intermediate_rep.number.into_iter().next(),
            size: intermediate_rep.size.into_iter().next(),
            additional_slave_sae_ids: intermediate_rep.additional_slave_sae_ids.into_iter().next(),
            extension_mandatory: intermediate_rep.extension_mandatory.into_iter().next(),
            extension_optional: intermediate_rep.extension_optional.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<KeyRequest> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<KeyRequest>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<KeyRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for KeyRequest - value: {hdr_value} is invalid {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<KeyRequest> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <KeyRequest as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{value}' into KeyRequest - {err}"))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {hdr_value:?} to string: {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Vec<KeyRequest>>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_values: header::IntoHeaderValue<Vec<KeyRequest>>) -> std::result::Result<Self, Self::Error> {
        let hdr_values : Vec<String> = hdr_values.0.into_iter().map(|hdr_value| {
            hdr_value.to_string()
        }).collect();

        match hyper::header::HeaderValue::from_str(&hdr_values.join(", ")) {
           std::result::Result::Ok(hdr_value) => std::result::Result::Ok(hdr_value),
           std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to convert {hdr_values:?} into a header - {e}",))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Vec<KeyRequest>> {
    type Error = String;

    fn try_from(hdr_values: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_values.to_str() {
            std::result::Result::Ok(hdr_values) => {
                let hdr_values : std::vec::Vec<KeyRequest> = hdr_values
                .split(',')
                .filter_map(|hdr_value| match hdr_value.trim() {
                    "" => std::option::Option::None,
                    hdr_value => std::option::Option::Some({
                        match <KeyRequest as std::str::FromStr>::from_str(hdr_value) {
                            std::result::Result::Ok(value) => std::result::Result::Ok(value),
                            std::result::Result::Err(err) => std::result::Result::Err(
                                format!("Unable to convert header value '{hdr_value}' into KeyRequest - {err}"))
                        }
                    })
                }).collect::<std::result::Result<std::vec::Vec<_>, String>>()?;

                std::result::Result::Ok(header::IntoHeaderValue(hdr_values))
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to parse header: {hdr_values:?} as a string - {e}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Validate, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Status {
    #[serde(rename = "source_KME_ID")]

    pub source_kme_id: String,

    #[serde(rename = "target_KME_ID")]

    pub target_kme_id: String,

    #[serde(rename = "master_SAE_ID")]

    pub master_sae_id: String,

    #[serde(rename = "slave_SAE_ID")]

    pub slave_sae_id: String,

    #[serde(rename = "key_size")]

    pub key_size: i32,

    #[serde(rename = "stored_key_count")]

    pub stored_key_count: i32,

    #[serde(rename = "max_key_count")]

    pub max_key_count: i32,

    #[serde(rename = "max_key_per_request")]

    pub max_key_per_request: i32,

    #[serde(rename = "max_key_size")]

    pub max_key_size: i32,

    #[serde(rename = "min_key_size")]

    pub min_key_size: i32,

    /// \"0\" when the KME does not support key multicast
    #[serde(rename = "max_SAE_ID_count")]

    pub max_sae_id_count: i32,

    /// Optional, for future use
    #[serde(rename = "status_extension")]

    #[serde(skip_serializing_if="Option::is_none")]
    pub status_extension: Option<std::collections::HashMap<String, serde_json::Value>>,

}


impl Status {
    #[allow(clippy::new_without_default)]
    pub fn new(source_kme_id: String, target_kme_id: String, master_sae_id: String, slave_sae_id: String, key_size: i32, stored_key_count: i32, max_key_count: i32, max_key_per_request: i32, max_key_size: i32, min_key_size: i32, max_sae_id_count: i32, ) -> Status {
        Status {
            source_kme_id,
            target_kme_id,
            master_sae_id,
            slave_sae_id,
            key_size,
            stored_key_count,
            max_key_count,
            max_key_per_request,
            max_key_size,
            min_key_size,
            max_sae_id_count,
            status_extension: None,
        }
    }
}

/// Converts the Status value to the Query Parameters representation (style=form, explode=false)
/// specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde serializer
impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("source_KME_ID".to_string()),
            Some(self.source_kme_id.to_string()),
            Some("target_KME_ID".to_string()),
            Some(self.target_kme_id.to_string()),
            Some("master_SAE_ID".to_string()),
            Some(self.master_sae_id.to_string()),
            Some("slave_SAE_ID".to_string()),
            Some(self.slave_sae_id.to_string()),
            Some("key_size".to_string()),
            Some(self.key_size.to_string()),
            Some("stored_key_count".to_string()),
            Some(self.stored_key_count.to_string()),
            Some("max_key_count".to_string()),
            Some(self.max_key_count.to_string()),
            Some("max_key_per_request".to_string()),
            Some(self.max_key_per_request.to_string()),
            Some("max_key_size".to_string()),
            Some(self.max_key_size.to_string()),
            Some("min_key_size".to_string()),
            Some(self.min_key_size.to_string()),
            Some("max_SAE_ID_count".to_string()),
            Some(self.max_sae_id_count.to_string()),
            // Skipping map status_extension in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Status value
/// as specified in <https://swagger.io/docs/specification/serialization/>
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub source_kme_id: Vec<String>,
            pub target_kme_id: Vec<String>,
            pub master_sae_id: Vec<String>,
            pub slave_sae_id: Vec<String>,
            pub key_size: Vec<i32>,
            pub stored_key_count: Vec<i32>,
            pub max_key_count: Vec<i32>,
            pub max_key_per_request: Vec<i32>,
            pub max_key_size: Vec<i32>,
            pub min_key_size: Vec<i32>,
            pub max_sae_id_count: Vec<i32>,
            pub status_extension: Vec<std::collections::HashMap<String, serde_json::Value>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing Status".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "source_KME_ID" => intermediate_rep.source_kme_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "target_KME_ID" => intermediate_rep.target_kme_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "master_SAE_ID" => intermediate_rep.master_sae_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "slave_SAE_ID" => intermediate_rep.slave_sae_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "key_size" => intermediate_rep.key_size.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "stored_key_count" => intermediate_rep.stored_key_count.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "max_key_count" => intermediate_rep.max_key_count.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "max_key_per_request" => intermediate_rep.max_key_per_request.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "max_key_size" => intermediate_rep.max_key_size.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "min_key_size" => intermediate_rep.min_key_size.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "max_SAE_ID_count" => intermediate_rep.max_sae_id_count.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "status_extension" => return std::result::Result::Err("Parsing a container in this style is not supported in Status".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing Status".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Status {
            source_kme_id: intermediate_rep.source_kme_id.into_iter().next().ok_or_else(|| "source_KME_ID missing in Status".to_string())?,
            target_kme_id: intermediate_rep.target_kme_id.into_iter().next().ok_or_else(|| "target_KME_ID missing in Status".to_string())?,
            master_sae_id: intermediate_rep.master_sae_id.into_iter().next().ok_or_else(|| "master_SAE_ID missing in Status".to_string())?,
            slave_sae_id: intermediate_rep.slave_sae_id.into_iter().next().ok_or_else(|| "slave_SAE_ID missing in Status".to_string())?,
            key_size: intermediate_rep.key_size.into_iter().next().ok_or_else(|| "key_size missing in Status".to_string())?,
            stored_key_count: intermediate_rep.stored_key_count.into_iter().next().ok_or_else(|| "stored_key_count missing in Status".to_string())?,
            max_key_count: intermediate_rep.max_key_count.into_iter().next().ok_or_else(|| "max_key_count missing in Status".to_string())?,
            max_key_per_request: intermediate_rep.max_key_per_request.into_iter().next().ok_or_else(|| "max_key_per_request missing in Status".to_string())?,
            max_key_size: intermediate_rep.max_key_size.into_iter().next().ok_or_else(|| "max_key_size missing in Status".to_string())?,
            min_key_size: intermediate_rep.min_key_size.into_iter().next().ok_or_else(|| "min_key_size missing in Status".to_string())?,
            max_sae_id_count: intermediate_rep.max_sae_id_count.into_iter().next().ok_or_else(|| "max_SAE_ID_count missing in Status".to_string())?,
            status_extension: intermediate_rep.status_extension.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Status> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Status>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Status>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for Status - value: {hdr_value} is invalid {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Status> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <Status as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{value}' into Status - {err}"))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {hdr_value:?} to string: {e}"))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Vec<Status>>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_values: header::IntoHeaderValue<Vec<Status>>) -> std::result::Result<Self, Self::Error> {
        let hdr_values : Vec<String> = hdr_values.0.into_iter().map(|hdr_value| {
            hdr_value.to_string()
        }).collect();

        match hyper::header::HeaderValue::from_str(&hdr_values.join(", ")) {
           std::result::Result::Ok(hdr_value) => std::result::Result::Ok(hdr_value),
           std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to convert {hdr_values:?} into a header - {e}",))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Vec<Status>> {
    type Error = String;

    fn try_from(hdr_values: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_values.to_str() {
            std::result::Result::Ok(hdr_values) => {
                let hdr_values : std::vec::Vec<Status> = hdr_values
                .split(',')
                .filter_map(|hdr_value| match hdr_value.trim() {
                    "" => std::option::Option::None,
                    hdr_value => std::option::Option::Some({
                        match <Status as std::str::FromStr>::from_str(hdr_value) {
                            std::result::Result::Ok(value) => std::result::Result::Ok(value),
                            std::result::Result::Err(err) => std::result::Result::Err(
                                format!("Unable to convert header value '{hdr_value}' into Status - {err}"))
                        }
                    })
                }).collect::<std::result::Result<std::vec::Vec<_>, String>>()?;

                std::result::Result::Ok(header::IntoHeaderValue(hdr_values))
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!("Unable to parse header: {hdr_values:?} as a string - {e}")),
        }
    }
}
