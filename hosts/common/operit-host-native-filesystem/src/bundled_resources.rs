//! Immutable application resources rooted in the installed distribution.

use std::path::PathBuf;
use std::io::Read;
use operit_host_contract::{HostResult, BundledResourceHost::{BundledResourceHost, validateResourceKey}};

pub struct DirectoryBundledResourceHost {
    root: PathBuf,
}

impl DirectoryBundledResourceHost {
    /// Binds the exact resource root selected by the platform application layout.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Resolves the resource layout relative to the installed application executable.
    pub fn besideExecutable(relative: &str) -> HostResult<Self> {
        let executable = std::env::current_exe()?;
        let directory = executable.parent().ok_or_else(|| operit_host_contract::HostError::new("Application executable has no parent directory"))?;
        Ok(Self::new(directory.join(relative)))
    }
}

impl BundledResourceHost for DirectoryBundledResourceHost {
    /// Reads one immutable application resource outside the mutable storage VFS.
    fn read(&self, key: &str) -> HostResult<Vec<u8>> {
        validateResourceKey(key)?;
        Ok(std::fs::read(self.root.join(key))?)
    }
}

/// Reads resources from a packaged application owned by an independent native process.
pub struct ArchiveBundledResourceHost {
    archive: PathBuf,
    prefix: &'static str,
}

impl ArchiveBundledResourceHost {
    /// Binds one application archive and its exact resource namespace.
    pub fn new(archive: PathBuf, prefix: &'static str) -> Self {
        Self { archive, prefix }
    }
}

impl BundledResourceHost for ArchiveBundledResourceHost {
    /// Opens only the requested archive entry without extracting application resources.
    fn read(&self, key: &str) -> HostResult<Vec<u8>> {
        validateResourceKey(key)?;
        let file = std::fs::File::open(&self.archive)?;
        let mut archive = zip::ZipArchive::new(file).map_err(|error| operit_host_contract::HostError::new(error.to_string()))?;
        let mut entry = archive.by_name(&format!("{}{key}", self.prefix))
            .map_err(|error| operit_host_contract::HostError::new(error.to_string()))?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Exercises both installed layouts with real bytes, missing entries and traversal attempts.
    #[test]
    fn bundled_resources_read_the_selected_distribution_only() {
        let root = std::env::temp_dir().join(format!("operit-bundled-resources-{}", std::process::id()));
        std::fs::create_dir_all(root.join("assets/plugins")).unwrap();
        let key = "assets/plugins/demo.toolpkg";
        std::fs::write(root.join(key), b"directory package").unwrap();
        let directory = DirectoryBundledResourceHost::new(root.clone());
        assert_eq!(directory.read(key).unwrap(), b"directory package");
        assert!(directory.read("missing.toolpkg").is_err());
        assert!(directory.read("../outside").is_err());

        let archive_path = root.join("application.apk");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive_path).unwrap());
        zip.start_file(format!("assets/flutter_assets/{key}"), zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"archive package").unwrap();
        zip.finish().unwrap();
        let archive = ArchiveBundledResourceHost::new(archive_path, "assets/flutter_assets/");
        assert_eq!(archive.read(key).unwrap(), b"archive package");
        assert!(archive.read("missing.toolpkg").is_err());
        assert!(archive.read("../outside").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
