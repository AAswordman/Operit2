use std::sync::Arc;

use operit_host_api::ArchiveStagingHost;
use operit_host_api::HostManager::HostManager;
use operit_util::stream::ReverseStream::ReverseStream;
use operit_util::stream::Stream::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::data::archive::ArchiveSource::ArchiveSource;

const ARCHIVE_TRANSFER_MAX_CHUNK_BYTES: usize = 64 * 1024;

/// Identifies one immutable archive staged by the runtime-owning host.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct StagedArchive {
    pub archiveId: String,
    pub byteLength: i64,
}

/// Owns host-backed archive upload and immutable range-source creation.
#[derive(Clone)]
pub struct ArchiveTransferManager {
    archiveStagingHost: Arc<dyn ArchiveStagingHost>,
}

impl ArchiveTransferManager {
    /// Creates an archive transfer manager using the runtime owner's host capabilities.
    #[allow(non_snake_case)]
    pub fn getInstance(hostManager: &HostManager) -> Result<Self, String> {
        let archiveStagingHost = hostManager.archiveStagingHost.clone().ok_or_else(|| {
            "ArchiveStagingHost is not registered for streamed archive transfers".to_string()
        })?;
        Ok(Self { archiveStagingHost })
    }

    /// Creates one host-owned target for a streamed archive upload of an exact byte length.
    #[allow(non_snake_case)]
    pub fn beginArchiveUpload(&self, expectedByteLength: i64) -> Result<String, String> {
        let expectedByteLength = u64::try_from(expectedByteLength)
            .map_err(|_| "Archive byte length must not be negative".to_string())?;
        let archiveId = Uuid::new_v4().simple().to_string();
        self.archiveStagingHost
            .createArchive(&archiveId, expectedByteLength)
            .map_err(|error| error.message)?;
        Ok(archiveId)
    }

    /// Writes one complete caller-owned input stream into an archive upload.
    #[allow(non_snake_case)]
    pub async fn writeArchiveUpload(
        &self,
        archiveId: String,
        mut bytes: ReverseStream<Vec<u8>>,
    ) -> Result<(), String> {
        let mut writeError = None;
        bytes
            .collect(&mut |chunk| {
                if writeError.is_some() {
                    return;
                }
                if chunk.len() > ARCHIVE_TRANSFER_MAX_CHUNK_BYTES {
                    writeError = Some(format!(
                        "Archive upload chunk exceeds the {ARCHIVE_TRANSFER_MAX_CHUNK_BYTES}-byte limit"
                    ));
                    return;
                }
                if let Err(error) = self.archiveStagingHost.appendArchive(&archiveId, &chunk) {
                    writeError = Some(error.message);
                }
            })
            .await;
        match writeError {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Seals one uploaded archive after verifying its caller-declared byte length.
    #[allow(non_snake_case)]
    pub fn completeArchiveUpload(
        &self,
        archiveId: String,
        expectedByteLength: i64,
    ) -> Result<StagedArchive, String> {
        let byteLength = self.sealArchive(&archiveId, expectedByteLength)?;
        Ok(StagedArchive {
            archiveId,
            byteLength: i64::try_from(byteLength)
                .map_err(|_| "Archive byte length does not fit i64".to_string())?,
        })
    }

    /// Removes one host-owned archive upload regardless of its sealed state.
    #[allow(non_snake_case)]
    pub fn discardArchiveUpload(&self, archiveId: String) -> Result<(), String> {
        self.archiveStagingHost
            .removeArchive(&archiveId)
            .map_err(|error| error.message)
    }

    /// Opens one sealed archive as a portable range-readable source for runtime consumers.
    pub(crate) fn openStagedArchive(
        &self,
        archive: &StagedArchive,
    ) -> Result<Arc<dyn ArchiveSource>, String> {
        let byteLength = self.sealArchive(&archive.archiveId, archive.byteLength)?;
        Ok(Arc::new(StagedArchiveSource {
            archiveStagingHost: self.archiveStagingHost.clone(),
            archiveId: archive.archiveId.clone(),
            byteLength,
        }))
    }

    /// Seals an archive and verifies that its persisted length matches the supplied value.
    fn sealArchive(&self, archiveId: &str, expectedByteLength: i64) -> Result<u64, String> {
        let expectedByteLength = u64::try_from(expectedByteLength)
            .map_err(|_| "Archive byte length must not be negative".to_string())?;
        let actualByteLength = self
            .archiveStagingHost
            .sealArchive(archiveId)
            .map_err(|error| error.message)?;
        if actualByteLength != expectedByteLength {
            return Err(format!(
                "Archive byte length does not match the uploaded content: expected {expectedByteLength}, got {actualByteLength}"
            ));
        }
        Ok(actualByteLength)
    }
}

/// Adapts one sealed host-owned archive to the portable archive-reader contract.
struct StagedArchiveSource {
    archiveStagingHost: Arc<dyn ArchiveStagingHost>,
    archiveId: String,
    byteLength: u64,
}

impl ArchiveSource for StagedArchiveSource {
    /// Returns the verified byte length recorded when this archive was sealed.
    fn len(&self) -> Result<u64, String> {
        Ok(self.byteLength)
    }

    /// Reads one bounded range from the sealed platform-owned archive object.
    fn readAt(&self, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        if offset > self.byteLength {
            return Err("Archive source offset is outside the sealed upload".to_string());
        }
        let remaining = self.byteLength - offset;
        let requested = u64::try_from(length)
            .map_err(|_| "Archive source read length does not fit u64".to_string())?;
        let boundedLength = usize::try_from(remaining.min(requested))
            .map_err(|_| "Archive source read length does not fit this platform".to_string())?;
        let bytes = self
            .archiveStagingHost
            .readArchive(&self.archiveId, offset, boundedLength)
            .map_err(|error| error.message)?;
        if bytes.len() != boundedLength {
            return Err("Archive staging host returned an incomplete sealed read".to_string());
        }
        Ok(bytes)
    }
}
