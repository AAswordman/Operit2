use js_sys::Uint8Array;
use operit_host_api::{ArchiveStagingHost, HostResult};
use wasm_bindgen::prelude::*;

use crate::common::{bytes_to_js, call_archive_staging, js_i64};

/// Stores staged archive bytes in the Web runtime worker's OPFS container.
#[derive(Clone, Debug, Default)]
pub struct WebArchiveStagingHost;

unsafe impl Send for WebArchiveStagingHost {}
unsafe impl Sync for WebArchiveStagingHost {}

impl WebArchiveStagingHost {
    /// Creates the Web worker archive staging host.
    pub fn new() -> Self {
        Self
    }
}

impl ArchiveStagingHost for WebArchiveStagingHost {
    /// Creates one empty archive in worker-owned OPFS staging storage.
    fn createArchive(&self, archiveId: &str, expectedByteLength: u64) -> HostResult<()> {
        let expectedByteLength = i64::try_from(expectedByteLength).map_err(|_| {
            operit_host_api::HostError::new("archive staging byte length does not fit i64")
        })?;
        call_archive_staging(
            "createArchive",
            &[
                JsValue::from_str(archiveId),
                JsValue::from_f64(expectedByteLength as f64),
            ],
        )?;
        Ok(())
    }

    /// Appends one ordered chunk to an in-progress worker-owned archive.
    fn appendArchive(&self, archiveId: &str, chunk: &[u8]) -> HostResult<()> {
        call_archive_staging(
            "appendArchive",
            &[JsValue::from_str(archiveId), bytes_to_js(chunk)],
        )?;
        Ok(())
    }

    /// Finalizes one worker-owned archive and returns its persisted length.
    fn sealArchive(&self, archiveId: &str) -> HostResult<u64> {
        let value = call_archive_staging("sealArchive", &[JsValue::from_str(archiveId)])?;
        u64::try_from(js_i64(value, "archiveStaging.sealArchive")?).map_err(|_| {
            operit_host_api::HostError::new("archive staging length must not be negative")
        })
    }

    /// Reads one bounded range from a sealed worker-owned archive.
    fn readArchive(&self, archiveId: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let offset = i64::try_from(offset).map_err(|_| {
            operit_host_api::HostError::new("archive staging offset does not fit i64")
        })?;
        let length = i64::try_from(length).map_err(|_| {
            operit_host_api::HostError::new("archive staging read length does not fit i64")
        })?;
        let value = call_archive_staging(
            "readArchive",
            &[
                JsValue::from_str(archiveId),
                JsValue::from_f64(offset as f64),
                JsValue::from_f64(length as f64),
            ],
        )?;
        Ok(Uint8Array::new(&value).to_vec())
    }

    /// Removes one worker-owned archive and its staging metadata.
    fn removeArchive(&self, archiveId: &str) -> HostResult<()> {
        call_archive_staging("removeArchive", &[JsValue::from_str(archiveId)])?;
        Ok(())
    }
}
