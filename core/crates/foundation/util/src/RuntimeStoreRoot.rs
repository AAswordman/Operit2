use std::path::PathBuf;

/// Returns the canonical VFS namespace for runtime files.
pub fn default_runtime_dir() -> PathBuf { PathBuf::from("/operitapp/data") }

/// Returns the canonical VFS namespace for workspace collections.
pub fn default_workspace_dir() -> PathBuf { PathBuf::from("/operitapp/workspaces") }
