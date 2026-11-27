//! Read-only resources shipped with the application, independent of user storage.

use crate::HostResult;

/// Opens an exact application resource key without searching user-controlled paths.
pub trait BundledResourceHost: Send + Sync {
    /// Reads a packaged resource; missing or inaccessible resources are errors.
    fn read(&self, key: &str) -> HostResult<Vec<u8>>;
}

/// Rejects absolute paths and traversal before crossing the platform resource boundary.
pub fn validateResourceKey(key: &str) -> HostResult<()> {
    if key.is_empty() || key.contains(['\\', ':', '\0'])
        || key.split('/').any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(crate::HostError::new("Invalid bundled resource key"));
    }
    Ok(())
}
