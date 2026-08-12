use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Arc;

const ARCHIVE_SOURCE_READ_CHUNK_BYTES: usize = 64 * 1024;

/// Reads bounded byte ranges from one immutable archive object.
pub(crate) trait ArchiveSource: Send + Sync {
    /// Returns the persisted byte length of this immutable archive.
    fn len(&self) -> Result<u64, String>;

    /// Reads bytes beginning at one absolute archive offset.
    fn readAt(&self, offset: u64, length: usize) -> Result<Vec<u8>, String>;
}

/// Adapts one range-readable archive source to synchronous ZIP reader contracts.
pub(crate) struct ArchiveSourceReader {
    source: Arc<dyn ArchiveSource>,
    position: u64,
}

impl ArchiveSourceReader {
    /// Creates a ZIP-compatible reader over the supplied immutable archive source.
    pub(crate) fn new(source: Arc<dyn ArchiveSource>) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    /// Converts one archive-source error into an I/O error for ZIP consumers.
    fn sourceError(error: String) -> io::Error {
        io::Error::new(io::ErrorKind::Other, error)
    }
}

impl Read for ArchiveSourceReader {
    /// Reads the next requested range and advances the archive position.
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let requestedLength = buffer.len().min(ARCHIVE_SOURCE_READ_CHUNK_BYTES);
        let bytes = self
            .source
            .readAt(self.position, requestedLength)
            .map_err(Self::sourceError)?;
        buffer[..bytes.len()].copy_from_slice(&bytes);
        self.position = self
            .position
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "archive position overflow")
            })?;
        Ok(bytes.len())
    }
}

impl Seek for ArchiveSourceReader {
    /// Repositions the ZIP reader within the immutable archive source.
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let sourceLength = self.source.len().map_err(Self::sourceError)?;
        let next = match position {
            SeekFrom::Start(offset) => i128::from(offset),
            SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
            SeekFrom::End(offset) => i128::from(sourceLength) + i128::from(offset),
        };
        if next < 0 || next > i128::from(sourceLength) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "archive seek is outside the source",
            ));
        }
        self.position = u64::try_from(next).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "archive seek does not fit u64")
        })?;
        Ok(self.position)
    }
}
