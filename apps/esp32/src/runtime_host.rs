#![allow(non_snake_case)]

//! ESP32 Host adapters for the shared node runtime.
//! The node runtime owns the record format; this module only supplies durable
//! bytes and the standard `_operit-link` mDNS advertisement.
use esp_idf_svc::mdns::EspMdns;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use operit_host_api::ServiceDiscovery::{
    DiscoveredService, DiscoveryAdvertisement, DiscoveryCallback, DiscoverySubscription,
    ServiceAdvertisement, ServiceDiscoveryHost,
};
use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
use std::sync::{Arc, Mutex};

const STORAGE_NAMESPACE: &str = "operit_link";
const SNAPSHOT_KEY: &str = "snapshot_v2";
use crate::runtime_storage_codec::{self as codec, Files};
use crate::runtime_storage_pack as pack;

struct StorageState {
    nvs: EspNvs<NvsDefault>,
    files: Files,
    snapshot: Option<u8>,
}

/// Keeps the shared runtime record bytes unchanged and maps their virtual paths
/// into NVS. One lock covers read-modify-commit, avoiding lost pairing records.
/// Packed record bytes and compact paths stream into inactive bounded NVS chunks; the marker
/// is committed last. Only the current storage format is supported.
pub struct Esp32RuntimeStorageHost {
    state: Mutex<StorageState>,
}
impl Esp32RuntimeStorageHost {
    pub fn new(partition: EspDefaultNvsPartition) -> HostResult<Arc<Self>> {
        let nvs = EspNvs::new(partition, STORAGE_NAMESPACE, true)
            .map_err(|e| HostError::new(format!("runtime NVS: {e}")))?;
        let marker = nvs
            .get_u32(SNAPSHOT_KEY)
            .map_err(|e| HostError::new(format!("runtime NVS snapshot: {e}")))?;
        let snapshot = marker.map(|value| (value & 1) as u8);
        let files = if let Some(marker) = marker {
            let length = codec::snapshot_length(marker)?;
            let mut buffer = Vec::new();
            buffer
                .try_reserve_exact(length)
                .map_err(|_| codec::allocation_error("snapshot read", length))?;
            buffer.resize(length, 0);
            for (index, chunk) in buffer.chunks_mut(codec::SNAPSHOT_CHUNK_SIZE).enumerate() {
                let mut stored = [0u8; codec::SNAPSHOT_CHUNK_SIZE + 3];
                let read = nvs
                    .get_blob(&snapshotChunkKey((marker & 1) as u8, index), &mut stored)
                    .map_err(|e| HostError::new(format!("runtime NVS snapshot read: {e}")))?
                    .ok_or_else(|| HostError::new("Runtime NVS snapshot chunk missing"))?;
                codec::decode_snapshot_chunk(read, chunk)?;
            }
            codec::decode_packed_files(&buffer)?
        } else {
            // Reject obsolete data, never decode/migrate it or treat it as a new node.
            // This check must precede recovery, which removes unpublished chunks.
            if nvs
                .get_u32("snapshot_v1")
                .map_err(|e| HostError::new(format!("runtime NVS format check: {e}")))?
                .is_some()
                || nvs
                    .blob_len("files")
                    .map_err(|e| HostError::new(format!("runtime NVS format check: {e}")))?
                    .is_some()
            {
                return Err(HostError::new(
                    "Unsupported runtime NVS storage format; data left unchanged",
                ));
            }
            Files::new()
        };
        // Validate authority before reclaiming only the unpublished bank.
        codec::recover_snapshot(&mut NvsSnapshotBackend(&nvs), snapshot)?;
        Ok(Arc::new(Self {
            state: Mutex::new(StorageState {
                nvs,
                files,
                snapshot,
            }),
        }))
    }
    fn update(&self, change: impl FnOnce(&mut Files) -> HostResult<()>) -> HostResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| HostError::new("runtime NVS lock poisoned"))?;
        let mut files = state.files.clone();
        change(&mut files)?;
        if files == state.files {
            return Ok(());
        }
        let bank = codec::commit_snapshot_transaction(
            &files,
            state.snapshot,
            &mut NvsSnapshotBackend(&state.nvs),
        )?;
        state.files = files;
        state.snapshot = Some(bank);
        Ok(())
    }
}
/// All scalar publication uses IDF's atomic NVS update (no erase-first).
/// EspNvs::set_blob erases first, so it is used only on an unselected bank.
struct NvsSnapshotBackend<'a>(&'a EspNvs<NvsDefault>);
impl codec::SnapshotBackend for NvsSnapshotBackend<'_> {
    fn cleanup_warning(&mut self, error: HostError) {
        log::warn!("runtime NVS post-commit cleanup: {}", error.message);
    }
    fn remove_chunk(&mut self, bank: u8, index: usize) -> HostResult<()> {
        self.0
            .remove(&snapshotChunkKey(bank, index))
            .map(|_| ())
            .map_err(|e| HostError::new(format!("runtime NVS chunk cleanup: {e}")))
    }
    fn write_chunk(&mut self, bank: u8, index: usize, bytes: &[u8]) -> HostResult<()> {
        self.0
            .set_blob(&snapshotChunkKey(bank, index), bytes)
            .map_err(|e| HostError::new(format!("runtime NVS chunk write: {e}")))
    }
    fn publish(&mut self, marker: u32) -> HostResult<()> {
        self.0
            .set_u32(SNAPSHOT_KEY, marker)
            .map_err(|e| HostError::new(format!("runtime NVS snapshot commit: {e}")))
    }
    fn check_capacity(&mut self, needed: usize) -> HostResult<()> {
        let mut stats = esp_idf_svc::sys::nvs_stats_t::default();
        let result = unsafe { esp_idf_svc::sys::nvs_get_stats(std::ptr::null(), &mut stats) };
        if result != esp_idf_svc::sys::ESP_OK as i32 {
            return Err(HostError::new(format!(
                "runtime NVS capacity check: {result}"
            )));
        }
        // BLOB_DATA/BLOB_IDX overhead, possible page split, and marker update.
        // available_entries already excludes IDF's reserved GC page.
        if needed > stats.available_entries {
            return Err(HostError::new(format!(
                "ESP32 NVS capacity exhausted: need {needed} entries, available {}",
                stats.available_entries
            )));
        }
        Ok(())
    }
}
impl RuntimeStorageHost for Esp32RuntimeStorageHost {
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("runtime NVS lock poisoned"))?
            .files
            .get(path)
            .ok_or_else(|| HostError::new("runtime storage entry not found"))
            .and_then(|bytes| pack::unpack(bytes))
    }
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.update(|files| {
            files.insert(path.into(), Arc::new(pack::pack(content)?));
            Ok(())
        })
    }
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.update(|files| {
            let previous = files
                .get(path)
                .map(|bytes| pack::unpack(bytes))
                .transpose()?
                .unwrap_or_default();
            if content.len() > pack::MAX_RECORD_SIZE.saturating_sub(previous.len()) {
                return Err(HostError::new("ESP32 node state exceeds NVS budget"));
            }
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(previous.len() + content.len())
                .map_err(|_| codec::allocation_error("append", previous.len() + content.len()))?;
            bytes.extend_from_slice(&previous);
            bytes.extend_from_slice(content);
            files.insert(path.into(), Arc::new(pack::pack(&bytes)?));
            Ok(())
        })
    }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.update(|files| {
            let prefix = format!("{path}/");
            files.retain(|key, _| key != path && (!recursive || !key.starts_with(&prefix)));
            Ok(())
        })
    }
    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self
            .state
            .lock()
            .map_err(|_| HostError::new("runtime NVS lock poisoned"))?
            .files
            .contains_key(path))
    }
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        Ok(self
            .state
            .lock()
            .map_err(|_| HostError::new("runtime NVS lock poisoned"))?
            .files
            .iter()
            .filter(|(path, _)| path.starts_with(prefix))
            .map(|(path, bytes)| RuntimeStorageEntry {
                path: path.clone(),
                isDirectory: false,
                size: pack::expanded_size(bytes).unwrap_or(0) as i64,
            })
            .collect())
    }
}

fn snapshotChunkKey(bank: u8, index: usize) -> String {
    format!("files_{bank}_{index}")
}

struct AdvertisementHandle {
    mdns: Arc<Mutex<EspMdns>>,
}
impl DiscoveryAdvertisement for AdvertisementHandle {}
impl Drop for AdvertisementHandle {
    fn drop(&mut self) {
        if let Ok(mut mdns) = self.mdns.lock() {
            let _ = mdns.remove_service("_operit-link", "_tcp");
        }
    }
}
pub struct Esp32ServiceDiscoveryHost {
    mdns: Arc<Mutex<EspMdns>>,
}
impl Esp32ServiceDiscoveryHost {
    pub fn new() -> HostResult<Arc<Self>> {
        let mdns = EspMdns::take().map_err(|e| HostError::new(format!("mDNS init: {e}")))?;
        Ok(Arc::new(Self {
            mdns: Arc::new(Mutex::new(mdns)),
        }))
    }
    pub fn advertisePort(
        &self,
        node_id: &str,
        display_name: &str,
        port: u16,
    ) -> HostResult<Box<dyn DiscoveryAdvertisement>> {
        let mut mdns = self
            .mdns
            .lock()
            .map_err(|_| HostError::new("mDNS lock poisoned"))?;
        mdns.set_hostname(format!("operit-{}", node_id))
            .map_err(|e| HostError::new(format!("mDNS hostname: {e}")))?;
        mdns.add_service(
            Some(node_id),
            "_operit-link",
            "_tcp",
            port,
            &[
                ("nodeId", node_id),
                ("displayName", display_name),
                ("transports", "tcp"),
            ],
        )
        .map_err(|e| HostError::new(format!("mDNS service: {e}")))?;
        Ok(Box::new(AdvertisementHandle {
            mdns: self.mdns.clone(),
        }))
    }
}
impl ServiceDiscoveryHost for Esp32ServiceDiscoveryHost {
    fn supportsAdvertisement(&self) -> bool {
        true
    }
    fn advertise(
        &self,
        service: ServiceAdvertisement,
    ) -> HostResult<Box<dyn DiscoveryAdvertisement>> {
        let node_id = service
            .properties
            .get("nodeId")
            .unwrap_or(&service.instance);
        let display_name = service
            .properties
            .get("displayName")
            .unwrap_or(&service.instance);
        self.advertisePort(node_id, display_name, service.port)
    }
    fn discover(&self, _serviceType: &str, _timeoutMs: u64) -> HostResult<Vec<DiscoveredService>> {
        Err(HostError::new("ESP32 discovery browser is not implemented"))
    }
    fn subscribe(
        &self,
        _serviceType: &str,
        _callback: DiscoveryCallback,
    ) -> HostResult<Box<dyn DiscoverySubscription>> {
        Err(HostError::new("ESP32 discovery browser is not implemented"))
    }
}
