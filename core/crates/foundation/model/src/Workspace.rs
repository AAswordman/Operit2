use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One mounted folder inside a named workspace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFolder {
    pub name: String,
    pub path: String,
}

/// First-class named workspace that can mount multiple VFS folders.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub folders: Vec<WorkspaceFolder>,
    pub createdAt: i64,
    pub updatedAt: i64,
}

impl Workspace {
    /// Creates a workspace with one mounted folder.
    pub fn fromSingleFolder(
        name: String,
        folderName: String,
        path: String,
        timestamp: i64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            folders: vec![WorkspaceFolder {
                name: folderName,
                path,
            }],
            createdAt: timestamp,
            updatedAt: timestamp,
        }
    }

    /// Returns the first mounted folder, which is the primary working directory.
    pub fn primaryFolder(&self) -> &WorkspaceFolder {
        self.folders
            .first()
            .expect("workspace must contain at least one folder")
    }

    /// Mounts one folder and reports whether the workspace changed.
    ///
    /// Re-selecting an already mounted directory must stay idempotent: a path
    /// that is mounted under another name is ignored, and a known name whose
    /// path moved (for example after the host path is remapped into the VFS)
    /// is rebound in place instead of failing validation with a duplicate
    /// folder name.
    pub fn mountFolder(&mut self, folder: WorkspaceFolder) -> bool {
        if self.folders.iter().any(|existing| existing.path == folder.path) {
            return false;
        }
        if let Some(existing) = self
            .folders
            .iter_mut()
            .find(|existing| existing.name == folder.name)
        {
            existing.path = folder.path;
            return true;
        }
        self.folders.push(folder);
        true
    }

    /// Returns the VFS paths of every mounted folder.
    pub fn folderPaths(&self) -> Vec<String> {
        self.folders
            .iter()
            .map(|folder| folder.path.clone())
            .collect()
    }

    /// Finds a mounted folder by its display name.
    pub fn folderByName(&self, name: &str) -> Option<&WorkspaceFolder> {
        self.folders.iter().find(|folder| folder.name == name)
    }

    /// Derives a single-segment folder name from a VFS path.
    pub fn folderNameFromPath(path: &str) -> Result<String, String> {
        let name = path
            .trim()
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .map(str::trim)
            .filter(|name| !matches!(*name, "" | "." | ".." | "file:") && !name.contains('\\'))
            .ok_or_else(|| format!("cannot derive folder name from path: {path}"))?;
        Ok(name.to_string())
    }

    /// Validates workspace identity, name, and mounted folders.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("workspace id is required".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("workspace name is required".to_string());
        }
        if self.folders.is_empty() {
            return Err("workspace must contain at least one folder".to_string());
        }
        let mut names = HashSet::new();
        let mut paths = HashSet::new();
        for folder in &self.folders {
            let name = folder.name.trim();
            if name.is_empty() {
                return Err("workspace folder name is required".to_string());
            }
            if name != folder.name {
                return Err(format!("invalid workspace folder name: {}", folder.name));
            }
            if name == "." || name == ".." || name.contains('/') || name.contains('\\') {
                return Err(format!("invalid workspace folder name: {name}"));
            }
            if !names.insert(name.to_string()) {
                return Err(format!("duplicate workspace folder name: {name}"));
            }
            let path = folder.path.trim();
            if path.is_empty() {
                return Err("workspace folder path is required".to_string());
            }
            if path != folder.path {
                return Err(format!("invalid workspace folder path: {}", folder.path));
            }
            if !paths.insert(path.to_string()) {
                return Err(format!("duplicate workspace folder path: {path}"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn singleFolderWorkspace(name: &str, path: &str) -> Workspace {
        Workspace::fromSingleFolder(
            name.to_string(),
            name.to_string(),
            path.to_string(),
            1000,
        )
    }

    #[test]
    fn mountFolderAppendsNewFolders() {
        let mut workspace = singleFolderWorkspace("harmoon", "/mnt/linux/home/harmoon");
        let changed = workspace.mountFolder(WorkspaceFolder {
            name: "debug".to_string(),
            path: "/mnt/linux/home/harmoon/projects".to_string(),
        });
        assert!(changed);
        assert_eq!(workspace.folders.len(), 2);
        workspace.validate().expect("workspace must stay valid");
    }

    #[test]
    fn mountFolderIgnoresAlreadyMountedPaths() {
        let mut workspace = singleFolderWorkspace("harmoon", "/mnt/linux/home/harmoon");
        let changed = workspace.mountFolder(WorkspaceFolder {
            name: "home".to_string(),
            path: "/mnt/linux/home/harmoon".to_string(),
        });
        assert!(!changed);
        assert_eq!(workspace.folders.len(), 1);
        assert_eq!(workspace.folders[0].name, "harmoon");
    }

    #[test]
    fn mountFolderRebindsKnownNameToMovedPath() {
        let mut workspace = singleFolderWorkspace("harmoon", "/mnt/linux/home/harmoon");
        let changed = workspace.mountFolder(WorkspaceFolder {
            name: "harmoon".to_string(),
            path: "/home/harmoon".to_string(),
        });
        assert!(changed);
        assert_eq!(workspace.folders.len(), 1);
        assert_eq!(workspace.folders[0].path, "/home/harmoon");
        workspace.validate().expect("rebind must stay valid");
    }

    /// Rejects paths without a real folder name instead of inventing a root folder.
    #[test]
    fn folderNamesRejectInvalidPaths() {
        for path in [
            "", " \t ", "/", "///", ".", "..", "/tmp/.", "/tmp/..", "file:", "file:///",
        ] {
            assert!(Workspace::folderNameFromPath(path).is_err(), "{path:?}");
        }
    }

    /// Derives folder names from real VFS folders while accepting surrounding whitespace and trailing separators.
    #[test]
    fn folderNamesPreserveRealFolders() {
        for (path, expected) in [
            ("/mnt/linux/home/harmoon", "harmoon"),
            (" /app/workspaces/demo/ ", "demo"),
            ("/mnt/android/sdcard/Projects", "Projects"),
        ] {
            assert_eq!(Workspace::folderNameFromPath(path).unwrap(), expected);
        }
    }
}
