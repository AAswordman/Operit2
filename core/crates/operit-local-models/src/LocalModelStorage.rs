use operit_util::RuntimeStorageLayout::{
    RUNTIME_LOCAL_ENGINES_DIR_PATH, RUNTIME_LOCAL_MODELS_DIR_PATH,
};
use thiserror::Error;

use crate::LocalEngineManifest::LocalPlatformTarget;
use crate::LocalModelManifest::{LocalEngineKind, LocalModelKind};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum LocalModelStorageError {
    #[error("{field} is empty")]
    EmptySegment { field: &'static str },
    #[error("{field} has invalid characters: {value}")]
    InvalidSegment { field: &'static str, value: String },
}

/// Builds the runtime storage path for one local engine platform target.
pub fn buildLocalEngineStoragePath(
    engineId: &str,
    version: &str,
    target: &LocalPlatformTarget,
) -> Result<String, LocalModelStorageError> {
    let engineId = validatedStorageSegment("engineId", engineId)?;
    let version = validatedStorageSegment("version", version)?;
    Ok(format!(
        "{}/{}/{}/{}",
        RUNTIME_LOCAL_ENGINES_DIR_PATH,
        engineId,
        version,
        target.storageSegment()
    ))
}

/// Builds the runtime storage path for one local model version.
pub fn buildLocalModelStoragePath(
    kind: &LocalModelKind,
    engine: &LocalEngineKind,
    modelId: &str,
    version: &str,
) -> Result<String, LocalModelStorageError> {
    let modelId = validatedStorageSegment("modelId", modelId)?;
    let version = validatedStorageSegment("version", version)?;
    Ok(format!(
        "{}/{}/{}/{}/{}",
        RUNTIME_LOCAL_MODELS_DIR_PATH,
        kind.storageSegment(),
        engine.storageSegment(),
        modelId,
        version
    ))
}

/// Validates one runtime storage path segment and returns its trimmed value.
pub fn validatedStorageSegment(
    field: &'static str,
    value: &str,
) -> Result<String, LocalModelStorageError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(LocalModelStorageError::EmptySegment { field });
    }
    if trimmed == "." || trimmed == ".." {
        return Err(LocalModelStorageError::InvalidSegment {
            field,
            value: trimmed.to_string(),
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(LocalModelStorageError::InvalidSegment {
            field,
            value: trimmed.to_string(),
        });
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalModelManifest::{LocalEngineKind, LocalModelKind};

    /// Verifies the stable storage path for one local model.
    #[test]
    fn buildStoragePathUsesLocalModelLayout() {
        let path = buildLocalModelStoragePath(
            &LocalModelKind::SpeechToText,
            &LocalEngineKind::SherpaNcnn,
            "model-a",
            "v1.0",
        )
        .unwrap();

        assert_eq!(path, "runtime/models/local/stt/sherpa_ncnn/model-a/v1.0");
    }

    /// Verifies storage segment validation for directory traversal markers.
    #[test]
    fn validatedStorageSegmentRejectsTraversalMarkers() {
        assert!(validatedStorageSegment("modelId", ".").is_err());
        assert!(validatedStorageSegment("modelId", "..").is_err());
    }
}
