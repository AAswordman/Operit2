use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{HostError, HostResult};

/// One device-local terminal launch configuration registered by the user.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TerminalProfile {
    pub id: String,
    pub name: String,
    pub terminal: String,
    pub terminalType: String,
    pub options: BTreeMap<String, String>,
}

/// Editable launch fields supported by one concrete host implementation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalProfileSupport {
    pub terminal: String,
    pub terminalType: String,
    pub fields: Vec<String>,
}

/// Terminal profiles and the explicitly selected default on this Core device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalProfileSettings {
    pub revision: u64,
    pub defaultProfileId: String,
    pub profiles: Vec<TerminalProfile>,
}

impl TerminalProfileSettings {
    /// Decodes one durable profile registry without discarding malformed data.
    pub fn fromBytes(bytes: &[u8]) -> HostResult<Self> {
        serde_json::from_slice(bytes)
            .map_err(|error| HostError::new(format!("Invalid terminal profile settings: {error}")))
    }

    /// Encodes one complete profile registry for atomic persistence.
    pub fn toBytes(&self) -> HostResult<Vec<u8>> {
        serde_json::to_vec(self).map_err(|error| HostError::new(error.to_string()))
    }

    /// Rejects ambiguous identities and malformed launch values before persistence.
    pub fn validate(&self) -> HostResult<()> {
        let mut ids = BTreeSet::new();
        for profile in &self.profiles {
            if profile.id.is_empty()
                || !profile
                    .id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
                || !ids.insert(&profile.id)
            {
                return Err(HostError::new(
                    "Terminal profile IDs must be unique ASCII identifiers",
                ));
            }
            if profile.name.trim().is_empty() || profile.name.contains('\0') {
                return Err(HostError::new(
                    "Terminal profile name must not be blank or contain NUL",
                ));
            }
            for value in profile.options.values() {
                if value.trim().is_empty() || value.contains('\0') {
                    return Err(HostError::new(
                        "Terminal profile options must not be blank or contain NUL",
                    ));
                }
            }
        }
        if !ids.contains(&self.defaultProfileId) {
            return Err(HostError::new(
                "Default terminal profile must reference an existing profile",
            ));
        }
        Ok(())
    }
}

/// Parses an argument array without interpreting shell quoting or shell commands.
pub fn profileArguments(profile: &TerminalProfile) -> HostResult<Vec<String>> {
    match profile.options.get("arguments") {
        None => Ok(Vec::new()),
        Some(value) => {
            let arguments: Vec<String> = serde_json::from_str(value).map_err(|error| {
                HostError::new(format!(
                    "Terminal arguments must be a JSON string array: {error}"
                ))
            })?;
            if arguments.iter().any(|value| value.contains('\0')) {
                return Err(HostError::new("Terminal arguments must not contain NUL"));
            }
            Ok(arguments)
        }
    }
}

/// A downloadable root filesystem advertised by the active terminal host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalDistribution {
    pub id: String,
    pub name: String,
    pub architecture: String,
    pub url: String,
    pub sha256: String,
    pub downloadBytes: u64,
    pub notice: String,
}

/// Reports streaming download and installation phases without retaining archive bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalInstallProgress {
    pub state: String,
    pub downloadedBytes: u64,
    pub totalBytes: u64,
}

/// Publishes installation progress independently of the requesting UI connection.
pub type TerminalInstallProgressCallback =
    std::sync::Arc<dyn Fn(TerminalInstallProgress) + Send + Sync + 'static>;
