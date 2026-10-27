//! A desktop persistence carrier for the exact firmware packed storage implementation.
use crate::{
    runtime_storage_codec as codec,
    runtime_storage_host::{PackedRuntimeStorageHost, RuntimeStorageBackend},
};
use operit_host_api::{HostError, HostResult};
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
}
pub struct FileSnapshotBackend {
    carrier: FileCarrier,
    marker: Option<u32>,
}

struct FileCarrier {
    file: File,
    markerPath: PathBuf,
    temporaryMarkerPath: PathBuf,
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
        let carrier = FileCarrier {
            file,
            markerPath: runtime.join("snapshot.marker"),
            temporaryMarkerPath: runtime.join("snapshot.pending"),
        };
        Ok(Self { carrier, marker })
    }

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
                self.slot(bank, index)?;
                self.file.write_all(&(bytes.len() as u32).to_le_bytes())?;
                self.file.write_all(&bytes)?;
            }
            Operation::Remove(bank, index) => {
                self.slot(bank, index)?;
                self.file.write_all(&0u32.to_le_bytes())?;
            }
            Operation::Publish(marker) => {
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
    /// Applies the shared logical NVS bound; physical flash capacity remains hardware telemetry.
    fn check_capacity(&mut self, _entries: usize) -> HostResult<()> {
        Ok(())
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
