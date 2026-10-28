//! A desktop persistence carrier for the exact firmware packed storage implementation.
use crate::{
    runtime_storage_codec as codec,
    runtime_storage_host::{PackedRuntimeStorageHost, RuntimeStorageBackend},
};
use operit_host_api::{HostError, HostResult};
use crate::storage_pressure::StorageControl;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

const SLOT_BYTES: usize = codec::SNAPSHOT_CHUNK_SIZE + 3 + 4;
const BANK_BYTES: usize = SLOT_BYTES * codec::STORAGE_BUDGET.div_ceil(codec::SNAPSHOT_CHUNK_SIZE);
pub const CARRIER_STACK_BYTES: usize = 128 * 1024;
pub type DeviceStorage = PackedRuntimeStorageHost<FileSnapshotBackend>;

enum Operation {
    Read(u8, usize),
    Write(u8, usize, Vec<u8>),
    Remove(u8, usize),
    Publish(u32),
    Capacity(usize),
}
pub struct FileSnapshotBackend {
    carrier: FileCarrier,
    marker: Option<u32>,
    control: StorageControl,
}

struct FileCarrier {
    file: File,
    markerPath: PathBuf,
    temporaryMarkerPath: PathBuf,
    control: StorageControl,
}

impl FileSnapshotBackend {
    /// Opens only desktop I/O outside the guarded device tasks; all record work remains shared.
    pub fn open(stateDirectory: PathBuf) -> HostResult<Self> {
        let runtime = stateDirectory.join("runtime");
        fs::create_dir_all(&runtime)?;
        for entry in fs::read_dir(&runtime)? {
            let name = entry?.file_name();
            if name != "snapshot.data" && name != "snapshot.marker" && name != "snapshot.pending" {
                return Err(HostError::new(
                    "Unsupported simulator storage layout; use a fresh simulator state directory",
                ));
            }
        }
        let marker = match fs::read(runtime.join("snapshot.marker")) {
            Ok(bytes) => {
                Some(u32::from_le_bytes(bytes.try_into().map_err(|_| {
                    HostError::new("Invalid simulator snapshot marker")
                })?))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(runtime.join("snapshot.data"))?;
        if file.metadata()?.len() == 0 && marker.is_none() {
            file.set_len((2 * BANK_BYTES) as u64)?;
        }
        if file.metadata()?.len() != (2 * BANK_BYTES) as u64 {
            return Err(HostError::new("Invalid simulator snapshot carrier length"));
        }
        let control = StorageControl::default();
        let mut carrier = FileCarrier {
            file,
            control: control.clone(),
            markerPath: runtime.join("snapshot.marker"),
            temporaryMarkerPath: runtime.join("snapshot.pending"),
        };
        carrier.refresh_entries(marker.map(codec::snapshot_length).transpose()?)?;
        Ok(Self { carrier, marker, control })
    }

    pub fn control(&self) -> StorageControl { self.control.clone() }

    /// Completes one bounded carrier operation while keeping OS path handling off device stacks.
    fn transact(&mut self, operation: Operation) -> HostResult<Vec<u8>> {
        operit_host_native_scheduler::withStackBudget(CARRIER_STACK_BYTES, || {
            self.carrier.perform(operation)
        })?
    }
}

impl FileCarrier {
    /// Seeks to a fixed slot within the two-bank snapshot file.
    fn slot(&mut self, bank: u8, index: usize) -> HostResult<()> {
        if bank > 1 || index >= codec::STORAGE_BUDGET.div_ceil(codec::SNAPSHOT_CHUNK_SIZE) {
            return Err(HostError::new("Invalid simulator snapshot slot"));
        }
        self.file.seek(SeekFrom::Start(
            (bank as usize * BANK_BYTES + index * SLOT_BYTES) as u64,
        ))?;
        Ok(())
    }

    /// Account for both banks and the published marker before reserving staging space.
    fn refresh_entries(&mut self, logical_bytes: Option<usize>) -> HostResult<()> {
        let mut used = usize::from(self.markerPath.exists());
        for bank in 0..2 {
            for index in 0..codec::STORAGE_BUDGET.div_ceil(codec::SNAPSHOT_CHUNK_SIZE) {
                self.slot(bank, index)?;
                let mut header = [0; 4];
                self.file.read_exact(&mut header)?;
                let length = u32::from_le_bytes(header) as usize;
                if length > codec::SNAPSHOT_CHUNK_SIZE + 3 {
                    return Err(HostError::new("Invalid simulator NVS chunk length"));
                }
                if length != 0 { used += length.div_ceil(32) + 3; }
            }
        }
        self.control.update(used, logical_bytes);
        Ok(())
    }

    /// Performs raw disk I/O only; compression, validation and publication order stay on device tasks.
    fn perform(&mut self, operation: Operation) -> HostResult<Vec<u8>> {
        match operation {
            Operation::Read(bank, index) => {
                self.slot(bank, index)?;
                let mut length = [0u8; 4];
                self.file.read_exact(&mut length)?;
                let length = u32::from_le_bytes(length) as usize;
                if length == 0 || length > codec::SNAPSHOT_CHUNK_SIZE + 3 {
                    return Err(HostError::new("Invalid simulator snapshot chunk length"));
                }
                let mut bytes = vec![0; length];
                self.file.read_exact(&mut bytes)?;
                return Ok(bytes);
            }
            Operation::Write(bank, index, bytes) => {
                self.control.check_capacity(bytes.len().div_ceil(32) + 3)?;
                self.control.before_mutation()?;
                self.slot(bank, index)?;
                self.file.write_all(&(bytes.len() as u32).to_le_bytes())?;
                self.file.write_all(&bytes)?;
            }
            Operation::Remove(bank, index) => {
                self.slot(bank, index)?;
                self.file.write_all(&0u32.to_le_bytes())?;
            }
            Operation::Capacity(needed) => {
                self.refresh_entries(None)?;
                self.control.check_capacity(needed)?;
                return Ok(Vec::new());
            }
            Operation::Publish(marker) => {
                self.control.before_mutation()?;
                self.file.sync_all()?;
                let mut publication = File::create(&self.temporaryMarkerPath)?;
                publication.write_all(&marker.to_le_bytes())?;
                publication.sync_all()?;
                drop(publication);
                operit_host_native_storage::publishStagedFile(
                    &self.temporaryMarkerPath,
                    &self.markerPath,
                    true,
                )?;
            }
        }
        let logical_bytes = if self.markerPath.exists() {
            let raw = fs::read(&self.markerPath)?;
            let marker = u32::from_le_bytes(raw.try_into().map_err(|_| HostError::new("Invalid snapshot marker"))?);
            Some(codec::snapshot_length(marker)?)
        } else { Some(0) };
        self.refresh_entries(logical_bytes)?;
        Ok(Vec::new())
    }
}

impl RuntimeStorageBackend for FileSnapshotBackend {
    /// Returns the durable marker loaded by desktop bootstrap.
    fn marker(&mut self) -> HostResult<Option<u32>> {
        Ok(self.marker)
    }
    /// Reads one hardware-format compressed chunk through the bounded carrier.
    fn read_chunk(&mut self, bank: u8, index: usize, destination: &mut [u8]) -> HostResult<usize> {
        let bytes = self.transact(Operation::Read(bank, index))?;
        if bytes.len() > destination.len() {
            return Err(HostError::new(
                "Simulator snapshot chunk exceeds destination",
            ));
        }
        destination[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }
}

impl codec::SnapshotBackend for FileSnapshotBackend {
    /// Clears only an unpublished or retired chunk slot.
    fn remove_chunk(&mut self, bank: u8, index: usize) -> HostResult<()> {
        self.transact(Operation::Remove(bank, index)).map(|_| ())
    }
    /// Sends at most one bounded compressed chunk to the inactive bank.
    fn write_chunk(&mut self, bank: u8, index: usize, bytes: &[u8]) -> HostResult<()> {
        if bytes.len() > codec::SNAPSHOT_CHUNK_SIZE + 3 {
            return Err(HostError::new("Simulator snapshot chunk exceeds budget"));
        }
        self.transact(Operation::Write(bank, index, bytes.to_vec()))
            .map(|_| ())
    }
    /// Flushes inactive chunks before atomically publishing their selection marker.
    fn publish(&mut self, marker: u32) -> HostResult<()> {
        self.transact(Operation::Publish(marker))?;
        self.marker = Some(marker);
        Ok(())
    }
    /// Reserve estimated NVS entries for both banks, external namespaces and GC.
    fn check_capacity(&mut self, entries: usize) -> HostResult<()> {
        self.transact(Operation::Capacity(entries)).map(|_| ())
    }
    /// Reports cleanup failures after the selected snapshot has already committed.
    fn cleanup_warning(&mut self, error: HostError) {
        eprintln!("simulator snapshot cleanup: {}", error.message);
    }
}

/// Resolves the simulator's configured persistent device directory.
pub fn stateDirectory() -> PathBuf {
    std::env::var_os("OPERIT_SIM_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("operit2-esp32-simulator"))
}


#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::RuntimeStorageHost;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "operit-nvs-carrier-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn open(&self) -> (ArcStorage, StorageControl) {
            let backend = FileSnapshotBackend::open(self.0.clone()).unwrap();
            let control = backend.control();
            (DeviceStorage::open(backend).unwrap(), control)
        }
    }
    type ArcStorage = std::sync::Arc<DeviceStorage>;
    impl Drop for Directory {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }
    fn noise(size: usize) -> Vec<u8> {
        let mut seed = 0x13456789u32;
        (0..size).map(|_| {
            seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5;
            seed as u8
        }).collect()
    }
    #[test]
    fn physical_pressure_rejects_small_write_without_changing_authority_then_recovers() {
        let directory = Directory::new();
        let (storage, control) = directory.open();
        storage.writeBytes("runtime/identity", b"keep identity and pairing").unwrap();
        let marker = fs::read(directory.0.join("runtime/snapshot.marker")).unwrap();
        let before = control.snapshot();
        control.configure(json!({"otherNamespaceEntries": crate::storage_pressure::CAPACITY})).unwrap();
        let error = storage.writeBytes("runtime/member", b"new member").unwrap_err();
        assert!(error.message.contains("NVS capacity exhausted"), "{}", error.message);
        assert_eq!(fs::read(directory.0.join("runtime/snapshot.marker")).unwrap(), marker);
        assert_eq!(control.snapshot()["runtimeEntries"], before["runtimeEntries"]);
        assert!(storage.readBytes("runtime/member").is_err());
        control.configure(json!({"otherNamespaceEntries": 213})).unwrap();
        storage.writeBytes("runtime/member", b"new member").unwrap();
        drop(storage);
        let (storage, _) = directory.open();
        assert_eq!(storage.readBytes("runtime/identity").unwrap(), b"keep identity and pairing");
        assert_eq!(storage.readBytes("runtime/member").unwrap(), b"new member");
    }
    #[test]
    fn every_chunk_and_publication_failure_preserves_bank_across_restart_and_retry() {
        // 4500 incompressible bytes require three chunk writes followed by publication.
        for after in 0..=3 {
            let directory = Directory::new();
            let (storage, control) = directory.open();
            storage.writeBytes("runtime/identity", b"original").unwrap();
            let marker = fs::read(directory.0.join("runtime/snapshot.marker")).unwrap();
            let entries = control.snapshot()["runtimeEntries"].clone();
            control.configure(json!({"failAfterMutations": after})).unwrap();
            let bytes = noise(4500);
            let error = storage.writeBytes("runtime/audio-fixture", &bytes).unwrap_err();
            assert!(error.message.contains("Injected NVS write failure"), "{after}: {}", error.message);
            assert_eq!(control.snapshot()["runtimeEntries"], entries, "leaked staged entries at {after}");
            assert_eq!(fs::read(directory.0.join("runtime/snapshot.marker")).unwrap(), marker);
            assert!(storage.readBytes("runtime/audio-fixture").is_err());
            assert_eq!(storage.readBytes("runtime/identity").unwrap(), b"original");
            drop(storage);
            let (storage, _) = directory.open();
            assert!(storage.readBytes("runtime/audio-fixture").is_err());
            storage.writeBytes("runtime/audio-fixture", &bytes).unwrap();
            drop(storage);
            let (storage, control) = directory.open();
            assert_eq!(storage.readBytes("runtime/audio-fixture").unwrap(), bytes);
            assert_eq!(storage.readBytes("runtime/identity").unwrap(), b"original");
            assert!(control.snapshot()["logicalBytes"].as_u64().unwrap() < 7000);
        }
    }
    #[test]
    fn logical_snapshot_limit_rejects_before_modifying_the_carrier() {
        let directory = Directory::new();
        let (storage, _) = directory.open();
        storage.writeBytes("runtime/identity", b"original").unwrap();
        let marker = fs::read(directory.0.join("runtime/snapshot.marker")).unwrap();
        let data = fs::read(directory.0.join("runtime/snapshot.data")).unwrap();
        let error = storage.writeBytes("runtime/oversize", &noise(7100)).unwrap_err();
        assert!(error.message.contains("exceeds NVS budget"), "{}", error.message);
        assert_eq!(fs::read(directory.0.join("runtime/snapshot.marker")).unwrap(), marker);
        assert_eq!(fs::read(directory.0.join("runtime/snapshot.data")).unwrap(), data);
        assert_eq!(storage.readBytes("runtime/identity").unwrap(), b"original");
    }
}
