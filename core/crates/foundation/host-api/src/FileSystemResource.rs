//! Opaque filesystem resources transported through the existing host filesystem API.
//! Platform adapters interpret their own backend; they must never be passed to std::fs.
use crate::{HostError, HostResult};
use serde::{Deserialize, Serialize};

pub const RESOURCE_PREFIX: &str = "operit-resource:";

/// A file reference whose native location is resolved by the consuming platform provider.
pub trait FileSource: Send + Sync {
    /// Resolves a native file location, rejecting document resources without native access.
    fn nativePath(&self) -> HostResult<String>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSystemResource {
    pub backend: String,
    pub root: String,
    pub path: String,
}

impl FileSystemResource {
    pub fn validate(&self) -> HostResult<()> {
        if self.backend.is_empty() || !self.backend.bytes().all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c)) {
            return Err(HostError::new("Invalid filesystem backend"));
        }
        if self.root.is_empty() || self.root.contains('\0') || self.path.starts_with('/') || self.path.contains('\\') || self.path.contains('\0')
            || self.path.split('/').any(|part| part == "." || part == ".." || (!self.path.is_empty() && part.is_empty())) {
            return Err(HostError::new("Invalid filesystem resource root or relative path"));
        }
        Ok(())
    }

    pub fn encode(&self) -> HostResult<String> {
        self.validate()?;
        serde_json::to_string(self).map(|json| format!("{RESOURCE_PREFIX}{json}"))
            .map_err(|e| HostError::new(e.to_string()))
    }

    pub fn parse(value: &str) -> HostResult<Option<Self>> {
        let Some(json) = value.strip_prefix(RESOURCE_PREFIX) else { return Ok(None) };
        let resource: Self = serde_json::from_str(json).map_err(|e| HostError::new(format!("Invalid filesystem resource: {e}")))?;
        resource.validate()?;
        Ok(Some(resource))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaqueRootsAndUnicodePathsRoundTrip() {
        let resource = FileSystemResource { backend: "android_documents".into(), root: "content://provider/tree/opaque%2Fid".into(), path: "项目/a b.txt".into() };
        assert_eq!(FileSystemResource::parse(&resource.encode().unwrap()).unwrap(), Some(resource));
        assert_eq!(FileSystemResource::parse("/tmp/a").unwrap(), None);
    }
    #[test]
    fn traversalAndMalformedResourcesAreRejected() {
        for path in ["../a", "a/../b", "./a", "/tmp", "a\\b", "a\0b"] {
            assert!(FileSystemResource { backend: "test".into(), root: "opaque".into(), path: path.into() }.encode().is_err());
        }
        assert!(FileSystemResource::parse("operit-resource:broken").is_err());
    }
}
