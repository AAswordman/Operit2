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
use operit_host_api::{HostError, HostResult};
use std::sync::{Arc, Mutex};

const STORAGE_NAMESPACE: &str = "operit_link";
const SNAPSHOT_KEY: &str = "snapshot_v2";
use crate::runtime_storage_codec as codec;

use crate::runtime_storage_host::{PackedRuntimeStorageHost, RuntimeStorageBackend};

pub type Esp32RuntimeStorageHost = PackedRuntimeStorageHost<NvsSnapshotBackend>;

impl Esp32RuntimeStorageHost {
    /// Opens the hardware snapshot carrier with shared packed-record semantics.
    pub fn new(partition: EspDefaultNvsPartition) -> HostResult<Arc<Self>> {
        let nvs = EspNvs::new(partition, STORAGE_NAMESPACE, true)
            .map_err(|error| HostError::new(format!("runtime NVS: {error}")))?;
        Self::open(NvsSnapshotBackend(nvs))
    }
}

impl RuntimeStorageBackend for NvsSnapshotBackend {
    /// Rejects obsolete storage before the shared recovery code can reclaim chunks.
    fn marker(&mut self) -> HostResult<Option<u32>> {
        let marker = self.0.get_u32(SNAPSHOT_KEY)
            .map_err(|error| HostError::new(format!("runtime NVS snapshot: {error}")))?;
        if marker.is_none() && (self.0.get_u32("snapshot_v1")
            .map_err(|error| HostError::new(format!("runtime NVS format check: {error}")))?.is_some()
            || self.0.blob_len("files").map_err(|error| HostError::new(format!("runtime NVS format check: {error}")))?.is_some()) {
            return Err(HostError::new("Unsupported runtime NVS storage format; data left unchanged"));
        }
        Ok(marker)
    }

    /// Reads one NVS blob without constructing an additional chunk buffer.
    fn read_chunk(&mut self, bank: u8, index: usize, destination: &mut [u8]) -> HostResult<usize> {
        self.0.get_blob(&snapshotChunkKey(bank, index), destination)
            .map_err(|error| HostError::new(format!("runtime NVS snapshot read: {error}")))?
            .map(|bytes| bytes.len()).ok_or_else(|| HostError::new("Runtime NVS snapshot chunk missing"))
    }
}
/// All scalar publication uses IDF's atomic NVS update (no erase-first).
/// EspNvs::set_blob erases first, so it is used only on an unselected bank.
pub struct NvsSnapshotBackend(EspNvs<NvsDefault>);
impl codec::SnapshotBackend for NvsSnapshotBackend {
    /// Reports cleanup failure after an already committed snapshot.
    fn cleanup_warning(&mut self, error: HostError) {
        log::warn!("runtime NVS post-commit cleanup: {}", error.message);
    }
    /// Removes one unselected snapshot chunk.
    fn remove_chunk(&mut self, bank: u8, index: usize) -> HostResult<()> {
        self.0
            .remove(&snapshotChunkKey(bank, index))
            .map(|_| ())
            .map_err(|e| HostError::new(format!("runtime NVS chunk cleanup: {e}")))
    }
    /// Writes one staged chunk into the unselected NVS bank.
    fn write_chunk(&mut self, bank: u8, index: usize, bytes: &[u8]) -> HostResult<()> {
        self.0
            .set_blob(&snapshotChunkKey(bank, index), bytes)
            .map_err(|e| HostError::new(format!("runtime NVS chunk write: {e}")))
    }
    /// Atomically publishes the completed bank marker through NVS.
    fn publish(&mut self, marker: u32) -> HostResult<()> {
        self.0
            .set_u32(SNAPSHOT_KEY, marker)
            .map_err(|e| HostError::new(format!("runtime NVS snapshot commit: {e}")))
    }
    /// Rejects snapshots that exceed the physical NVS entry budget.
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

/// Names one chunk within the shared two-bank snapshot format.
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
