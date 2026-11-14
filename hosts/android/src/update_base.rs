//! Installed APK capability; native package paths never leave the Android Host.
use operit_host_api::{FileInfo, FileSource, HostError, HostResult};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub(super) struct InstalledApk(pub PathBuf);

impl FileSource for InstalledApk {
    /// Gives the retained package capability a stable virtual identity.
    fn vfsPath(&self) -> &str {
        "/operitapp/application/installed.apk"
    }
    /// Reads package bytes when a consumer explicitly requests the whole file.
    fn readBytes(&self) -> HostResult<Vec<u8>> {
        Ok(std::fs::read(&self.0)?)
    }
    /// Reads only the range requested by hashing or delta reconstruction.
    fn readBytesRange(&self, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let mut file = std::fs::File::open(&self.0)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::with_capacity(length);
        file.take(length as u64).read_to_end(&mut bytes)?;
        Ok(bytes)
    }
    /// Reports the retained APK size without exposing Android's install path.
    fn fileInfo(&self) -> HostResult<FileInfo> {
        let metadata = std::fs::metadata(&self.0)?;
        if !metadata.is_file() {
            return Err(HostError::new("Installed APK is not a file"));
        }
        Ok(FileInfo {
            path: self.vfsPath().into(),
            exists: true,
            fileType: "file".into(),
            size: i64::try_from(metadata.len()).map_err(|e| HostError::new(e.to_string()))?,
            permissions: String::new(),
            owner: String::new(),
            group: String::new(),
            lastModified: String::new(),
            rawStatOutput: String::new(),
        })
    }
    /// Restricts this capability to reads rather than general installer interoperation.
    fn nativePath(&self) -> HostResult<String> {
        Err(HostError::new("Installed APK is a read-only update base"))
    }
}
