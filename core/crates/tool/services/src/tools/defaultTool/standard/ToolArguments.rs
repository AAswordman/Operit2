#![allow(non_snake_case)]

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

use crate::ToolExecutionManager::ToolCall;

/// Rejects undeclared arguments for zero-argument business operations.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EmptyArguments {}

/// Carries one exact ordinary configuration identifier.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigArguments {
    pub id: String,
}

/// Preserves a typed structured request without serializing nested values to text.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequestArguments<T> {
    pub request: T,
}

/// Decodes the declared argument contract once before execution and permission admission.
pub(crate) fn decodeArguments<T: DeserializeOwned>(request: &ToolCall) -> Result<T, String> {
    serde_json::from_value(Value::Object(
        request.parameters.clone().into_iter().collect(),
    ))
    .map_err(|error| error.to_string())
}

/// Requires identifiers to retain their exact nonblank representation.
pub(crate) fn requireExactIdentifier(value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.trim() != value {
        return Err("Identifier must be exact nonblank text".into());
    }
    Ok(())
}

/// Decodes a configuration ID without accepting alternate spellings.
pub(crate) fn decodeConfigArguments(request: &ToolCall) -> Result<ConfigArguments, String> {
    let args: ConfigArguments = decodeArguments(request)?;
    requireExactIdentifier(&args.id)?;
    Ok(args)
}
