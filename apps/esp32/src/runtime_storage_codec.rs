//! Bounded NVS snapshot encoding. Shared runtime record bytes stay unchanged.
//! Only compact ONV3 records in compressed 2-KiB chunks are supported,
//! with commit-last recovery and a 7 KB logical bound.
use super::runtime_storage_pack as pack;
use operit_host_api::{HostError, HostResult};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    sync::Arc,
};

pub const STORAGE_BUDGET: usize = 7000;
pub type Files = BTreeMap<String, Arc<Vec<u8>>>;

pub fn copy_bytes(bytes: &[u8]) -> HostResult<Vec<u8>> {
    if bytes.len() > STORAGE_BUDGET {
        return Err(HostError::new("ESP32 node state exceeds NVS budget"));
    }
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| allocation_error("record copy", bytes.len()))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

fn write_packed_files(mut writer: impl Write, files: &Files) -> io::Result<()> {
    writer.write_all(b"ONV3")?;
    let count =
        u16::try_from(files.len()).map_err(|_| io::Error::other("Too many runtime records"))?;
    writer.write_all(&count.to_le_bytes())?;
    let mut previous = "";
    for (path, bytes) in files {
        let mut prefix = previous
            .as_bytes()
            .iter()
            .zip(path.as_bytes())
            .take_while(|(a, b)| a == b)
            .count();
        while !path.is_char_boundary(prefix) {
            prefix -= 1;
        }
        let suffix = &path.as_bytes()[prefix..];
        let prefix =
            u16::try_from(prefix).map_err(|_| io::Error::other("Runtime path too long"))?;
        let suffix_len =
            u16::try_from(suffix.len()).map_err(|_| io::Error::other("Runtime path too long"))?;
        let length =
            u16::try_from(bytes.len()).map_err(|_| io::Error::other("Runtime record too long"))?;
        writer.write_all(&prefix.to_le_bytes())?;
        writer.write_all(&suffix_len.to_le_bytes())?;
        writer.write_all(&length.to_le_bytes())?;
        writer.write_all(suffix)?;
        writer.write_all(bytes)?;
        previous = path;
    }
    Ok(())
}
pub fn decode_packed_files(bytes: &[u8]) -> HostResult<Files> {
    fn invalid() -> HostError {
        HostError::new("Invalid packed runtime snapshot")
    }
    if bytes.len() < 6 || bytes.len() > STORAGE_BUDGET {
        return Err(invalid());
    }
    if &bytes[..4] != b"ONV3" {
        return Err(invalid());
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let mut files = Files::new();
    let mut previous = String::new();
    let mut at = 6;
    for _ in 0..count {
        let header_len = 6;
        let header = bytes.get(at..at + header_len).ok_or_else(invalid)?;
        at += header_len;
        let field = |offset| u16::from_le_bytes([header[offset], header[offset + 1]]) as usize;
        let (prefix, path_len, length) = (field(0), field(2), field(4));
        let suffix = std::str::from_utf8(bytes.get(at..at + path_len).ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        at += path_len;
        let shared = previous.get(..prefix).ok_or_else(invalid)?;
        let mut path = String::new();
        path.try_reserve_exact(shared.len() + suffix.len())
            .map_err(|_| allocation_error("snapshot path", shared.len() + suffix.len()))?;
        path.push_str(shared);
        path.push_str(suffix);
        let record = bytes.get(at..at + length).ok_or_else(invalid)?;
        at += length;
        if path.is_empty() || files.contains_key(&path) {
            return Err(invalid());
        }
        previous = path.clone();
        files.insert(path, Arc::new(copy_bytes(record)?));
    }
    if at != bytes.len() {
        return Err(invalid());
    }
    Ok(files)
}
/// One transaction/recovery implementation for hardware and fault-injection tests.
/// Logical byte bounds do not imply a physical NVS capacity guarantee.
pub trait SnapshotBackend {
    fn remove_chunk(&mut self, bank: u8, index: usize) -> HostResult<()>;
    fn write_chunk(&mut self, bank: u8, index: usize, bytes: &[u8]) -> HostResult<()>;
    fn publish(&mut self, marker: u32) -> HostResult<()>;
    fn check_capacity(&mut self, entries: usize) -> HostResult<()>;
    fn cleanup_warning(&mut self, _error: HostError) {}
}
fn clear_bank(backend: &mut impl SnapshotBackend, bank: u8) -> HostResult<()> {
    for index in 0..SNAPSHOT_CHUNKS {
        backend.remove_chunk(bank, index)?;
    }
    Ok(())
}
/// Only the commit marker selects authority. A failed/unpublished bank is
/// disposable, including after reset; never reclaim the selected bank.
pub fn recover_snapshot(backend: &mut impl SnapshotBackend, active: Option<u8>) -> HostResult<()> {
    if let Some(bank) = active {
        clear_bank(backend, bank ^ 1)?;
    } else {
        clear_bank(backend, 0)?;
        clear_bank(backend, 1)?;
    }
    Ok(())
}
pub fn commit_snapshot_transaction(
    files: &Files,
    active: Option<u8>,
    backend: &mut impl SnapshotBackend,
) -> HostResult<u8> {
    // Preflight the exact compressed chunk layout, without allocating the
    // whole database or changing the expanded/logical 7-KiB limit.
    let required = snapshot_required_entries(files)?;
    let bank = active.map_or(0, |bank| bank ^ 1);
    clear_bank(backend, bank)?;
    backend.check_capacity(required)?;
    let result = {
        let shared = std::cell::RefCell::new(&mut *backend);
        commit_packed_snapshot(
            files,
            bank,
            |index, bytes| {
                shared
                    .borrow_mut()
                    .write_chunk(bank, index, bytes)
                    .map_err(|e| io::Error::other(e.message))
            },
            |marker| shared.borrow_mut().publish(marker),
        )
    };
    if let Err(error) = result {
        // Partial writes must not consume the next attempt's available pages.
        if let Err(cleanup) = clear_bank(backend, bank) {
            return Err(HostError::new(format!(
                "{}; inactive snapshot cleanup: {}",
                error.message, cleanup.message
            )));
        }
        return Err(error);
    }
    // Publication is already durable; cleanup failure must never turn an
    // acknowledged credential/decision into a falsely reported failed write.
    if let Some(old) = active {
        if let Err(error) = clear_bank(backend, old) {
            backend.cleanup_warning(error);
        }
    }
    Ok(bank)
}
// The persisted marker must describe the one supported chunk format.
const SNAPSHOT_FORMAT: u32 = (1 << 31) | (1 << 30);
pub const SNAPSHOT_CHUNK_SIZE: usize = 2048;
const SNAPSHOT_CHUNKS: usize = STORAGE_BUDGET.div_ceil(SNAPSHOT_CHUNK_SIZE);
pub fn snapshot_length(marker: u32) -> HostResult<usize> {
    if marker & SNAPSHOT_FORMAT != SNAPSHOT_FORMAT {
        return Err(HostError::new("Unsupported runtime NVS snapshot format"));
    }
    let length = ((marker & !SNAPSHOT_FORMAT) >> 1) as usize;
    if !(6..=STORAGE_BUDGET).contains(&length) {
        return Err(HostError::new("Invalid runtime NVS snapshot length"));
    }
    Ok(length)
}
pub fn snapshot_required_entries(files: &Files) -> HostResult<usize> {
    let mut entries = 2; // marker update + conservative page overhead
    commit_packed_snapshot(
        files,
        0,
        |_, bytes| {
            entries += bytes.len().div_ceil(32) + 3; // BLOB_DATA, BLOB_IDX, split overhead
            Ok(())
        },
        |_| Ok(()),
    )?;
    Ok(entries)
}
/// Decode one bounded chunk; validate its expanded size before copying it.
pub fn decode_snapshot_chunk(stored: &[u8], destination: &mut [u8]) -> HostResult<()> {
    if destination.is_empty()
        || destination.len() > SNAPSHOT_CHUNK_SIZE
        || stored.len() > SNAPSHOT_CHUNK_SIZE + 3
        || pack::expanded_size(stored)? != destination.len()
    {
        return Err(HostError::new(
            "Invalid runtime snapshot expanded chunk length",
        ));
    }
    let raw = pack::unpack(stored)?;
    destination.copy_from_slice(&raw);
    Ok(())
}

/// Streaming envelope writer, with publication strictly after all chunks.
pub fn commit_packed_snapshot(
    files: &Files,
    bank: u8,
    mut sink: impl FnMut(usize, &[u8]) -> io::Result<()>,
    publish: impl FnOnce(u32) -> HostResult<()>,
) -> HostResult<()> {
    if bank > 1 {
        return Err(HostError::new("Invalid runtime NVS snapshot bank"));
    }
    let mut size = BudgetCounter(0);
    write_packed_files(&mut size, files).map_err(|e| HostError::new(e.to_string()))?;
    let mut writer = ChunkWriter {
        sink: |index, bytes: &[u8]| {
            let packed = pack::pack(bytes).map_err(|e| io::Error::other(e.message))?;
            sink(index, &packed)
        },
        buffer: [0; SNAPSHOT_CHUNK_SIZE],
        used: 0,
        index: 0,
    };
    write_packed_files(&mut writer, files).map_err(|e| HostError::new(e.to_string()))?;
    writer.flush().map_err(|e| HostError::new(e.to_string()))?;
    publish(SNAPSHOT_FORMAT | ((size.0 as u32) << 1) | bank as u32)
}

struct BudgetCounter(usize);
impl Write for BudgetCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > STORAGE_BUDGET.saturating_sub(self.0) {
            return Err(io::Error::other("ESP32 node state exceeds NVS budget"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn allocation_error(operation: &str, length: usize) -> HostError {
    #[cfg(target_os = "espidf")]
    let details = unsafe {
        format!(
            "; heap_free={}, largest_8bit={}",
            esp_idf_svc::sys::esp_get_free_heap_size(),
            esp_idf_svc::sys::heap_caps_get_largest_free_block(esp_idf_svc::sys::MALLOC_CAP_8BIT)
        )
    };
    #[cfg(not(target_os = "espidf"))]
    let details = "";
    HostError::new(format!(
        "runtime storage {operation} allocation failed ({length} bytes){details}"
    ))
}

struct ChunkWriter<F, const SIZE: usize> {
    sink: F,
    buffer: [u8; SIZE],
    used: usize,
    index: usize,
}
impl<F: FnMut(usize, &[u8]) -> io::Result<()>, const SIZE: usize> Write for ChunkWriter<F, SIZE> {
    fn write(&mut self, mut bytes: &[u8]) -> io::Result<usize> {
        let length = bytes.len();
        while !bytes.is_empty() {
            let copied = bytes.len().min(SIZE - self.used);
            self.buffer[self.used..self.used + copied].copy_from_slice(&bytes[..copied]);
            self.used += copied;
            bytes = &bytes[copied..];
            if self.used == SIZE {
                self.flush()?;
            }
        }
        Ok(length)
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.used != 0 {
            (self.sink)(self.index, &self.buffer[..self.used])?;
            self.index += 1;
            self.used = 0;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn packed_snapshot_round_trip_and_commit_failure_are_bounded() {
        let files: Files = [("runtime/test".into(), Arc::new(vec![1, 2, 3]))].into();
        let mut snapshot = Vec::new();
        let mut marker = 0;
        commit_packed_snapshot(
            &files,
            1,
            |_, chunk| {
                snapshot.extend_from_slice(&pack::unpack(chunk).unwrap());
                Ok(())
            },
            |value| {
                marker = value;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(marker, SNAPSHOT_FORMAT | ((snapshot.len() as u32) << 1) | 1);
        assert_eq!(snapshot_length(marker).unwrap(), snapshot.len());
        assert_eq!(decode_packed_files(&snapshot).unwrap(), files);
        for end in 0..snapshot.len() {
            assert!(decode_packed_files(&snapshot[..end]).is_err());
        }
        let mut extra = snapshot.clone();
        extra.push(0);
        assert!(decode_packed_files(&extra).is_err());
        let mut published = false;
        assert!(commit_packed_snapshot(
            &files,
            0,
            |_, _| Err(io::Error::other("power loss")),
            |_| {
                published = true;
                Ok(())
            }
        )
        .is_err());
        assert!(!published);
        let huge: Files = [("runtime/test".into(), Arc::new(vec![0; STORAGE_BUDGET]))].into();
        assert!(commit_packed_snapshot(
            &huge,
            0,
            |_, _| panic!("must validate before writing"),
            |_| panic!("must not publish")
        )
        .is_err());
    }
    #[test]
    fn unsupported_formats_and_invalid_lengths_are_rejected() {
        for flags in [0, 1 << 30, 1 << 31] {
            assert!(snapshot_length(flags | (6 << 1)).is_err());
        }
        for length in [0, 5, STORAGE_BUDGET + 1] {
            assert!(snapshot_length(SNAPSHOT_FORMAT | ((length as u32) << 1)).is_err());
        }
        assert!(decode_packed_files(b"ONV2\0\0").is_err());
        assert!(decode_packed_files(b"{\"x\":[]}").is_err());
        assert!(copy_bytes(&[0; STORAGE_BUDGET + 1]).is_err());
    }

    #[test]
    fn full_budget_streams_current_chunks_and_publishes_last() {
        let files: Files = [("x".into(), Arc::new(vec![42; STORAGE_BUDGET - 13]))].into();
        let chunks = std::cell::RefCell::new(Vec::new());
        commit_packed_snapshot(
            &files,
            1,
            |index, bytes| {
                let mut chunks = chunks.borrow_mut();
                assert_eq!(index, chunks.len());
                let raw = pack::unpack(bytes).unwrap();
                assert!(raw.len() <= SNAPSHOT_CHUNK_SIZE);
                chunks.push(raw);
                Ok(())
            },
            |marker| {
                assert_eq!(marker & 1, 1);
                assert_eq!(snapshot_length(marker).unwrap(), STORAGE_BUDGET);
                assert_eq!(chunks.borrow().len(), SNAPSHOT_CHUNKS);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            decode_packed_files(&chunks.into_inner().concat()).unwrap(),
            files
        );
    }

    #[test]
    fn compact_paths_preserve_utf8() {
        let files: Files = [
            (
                "runtime/space/device_profiles/你好.preferences.json".into(),
                Arc::new(vec![1, 2]),
            ),
            (
                "runtime/space/device_profiles/你们.preferences.json".into(),
                Arc::new(vec![3, 4]),
            ),
            (
                "runtime/space/members/local.preferences.json".into(),
                Arc::new(vec![5, 6]),
            ),
        ]
        .into();
        let mut new = Vec::new();
        commit_packed_snapshot(
            &files,
            0,
            |_, bytes| {
                new.extend_from_slice(&pack::unpack(bytes).unwrap());
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(&new[..4], b"ONV3");
        assert_eq!(decode_packed_files(&new).unwrap(), files);
        let mut invalid = new.clone();
        invalid[6..8].copy_from_slice(&1u16.to_le_bytes());
        assert!(
            decode_packed_files(&invalid).is_err(),
            "first prefix cannot reference an absent path"
        );
    }

    // Six 4-KiB pages, one reserved for GC; preserve the 213 non-runtime
    // live entries measured on the failing device (Wi-Fi, calibration, settings).
    #[derive(Clone)]
    pub(crate) struct PhysicalNvs {
        has_marker: bool,
        chunks: BTreeMap<(u8, usize), Vec<u8>>,
        marker: u32,
        fail_chunk: Option<usize>,
        fail_publish: bool,
        capacity: usize,
        removals: usize,
    }
    impl Default for PhysicalNvs {
        fn default() -> Self {
            Self {
                has_marker: false,
                chunks: BTreeMap::new(),
                marker: 0,
                fail_chunk: None,
                fail_publish: false,
                capacity: 5 * 126,
                removals: 0,
            }
        }
    }
    impl PhysicalNvs {
        pub(crate) fn active_bank(&self) -> Option<u8> {
            self.has_marker.then_some((self.marker & 1) as u8)
        }

        fn used(&self) -> usize {
            213 + self
                .chunks
                .values()
                .map(|b| b.len().div_ceil(32) + 2)
                .sum::<usize>()
        }
        pub(crate) fn selected(&self) -> Vec<u8> {
            let bank = (self.marker & 1) as u8;
            let length = snapshot_length(self.marker).unwrap();
            let mut raw = Vec::new();
            raw.resize(length, 0);
            for (index, destination) in raw.chunks_mut(SNAPSHOT_CHUNK_SIZE).enumerate() {
                decode_snapshot_chunk(&self.chunks[&(bank, index)], destination).unwrap();
            }
            raw
        }
    }
    impl SnapshotBackend for PhysicalNvs {
        fn remove_chunk(&mut self, bank: u8, index: usize) -> HostResult<()> {
            self.removals += 1;
            self.chunks.remove(&(bank, index));
            Ok(())
        }
        fn write_chunk(&mut self, bank: u8, index: usize, bytes: &[u8]) -> HostResult<()> {
            if self.has_marker {
                assert_ne!(
                    bank,
                    (self.marker & 1) as u8,
                    "never erase/write the selected bank"
                );
            }
            if self.fail_chunk == Some(index) {
                return Err(HostError::new("injected NVS chunk failure"));
            }
            if self.used() + bytes.len().div_ceil(32) + 2 > self.capacity {
                return Err(HostError::new("ESP_ERR_NVS_NOT_ENOUGH_SPACE"));
            }
            self.chunks.insert((bank, index), bytes.to_vec());
            Ok(())
        }
        fn publish(&mut self, marker: u32) -> HostResult<()> {
            if self.fail_publish {
                return Err(HostError::new("injected marker failure"));
            }
            self.marker = marker;
            self.has_marker = true;
            Ok(())
        }
        fn check_capacity(&mut self, required: usize) -> HostResult<()> {
            if self.used() + required > self.capacity {
                return Err(HostError::new(format!(
                    "physical NVS capacity exhausted: used={}, required={required}, capacity={}",
                    self.used(),
                    self.capacity
                )));
            }
            Ok(())
        }
    }
    fn physical_fixture() -> (Files, PhysicalNvs) {
        let files: Files =
            (0..20)
                .map(|i| {
                    (
            format!("runtime/space/device_profiles/core-shared-device-{i:03}.preferences.json"),
            Arc::new(vec![i as u8; 240]),
        )
                })
                .collect();
        let mut nvs = PhysicalNvs::default();
        commit_snapshot_transaction(&files, None, &mut nvs).unwrap();
        // Simulate an interrupted write to every chunk in the inactive bank.
        for index in 0..SNAPSHOT_CHUNKS {
            nvs.chunks.insert((1, index), vec![9; SNAPSHOT_CHUNK_SIZE]);
        }
        assert!(nvs.used() <= nvs.capacity);
        nvs.capacity = nvs.used();
        nvs.removals = 0;
        (files, nvs)
    }
    #[test]
    fn reboot_reclaims_partial_bank_without_erasing_authority_or_other_namespaces() {
        let (files, mut nvs) = physical_fixture();
        let old = nvs.selected();
        assert!(nvs
            .check_capacity(snapshot_required_entries(&files).unwrap())
            .is_err());
        recover_snapshot(&mut nvs, Some(0)).unwrap();
        assert_eq!(nvs.selected(), old);
        assert!(nvs.chunks.keys().all(|(bank, _)| *bank == 0));
        let bank = commit_snapshot_transaction(&files, Some(0), &mut nvs).unwrap();
        assert_eq!(bank, 1);
        assert_eq!(decode_packed_files(&nvs.selected()).unwrap(), files);
        assert!(nvs.chunks.keys().all(|(bank, _)| *bank == 1));
    }
    #[test]
    fn every_chunk_and_marker_failure_rolls_back_and_retry_does_not_leak_pages() {
        let (files, initial) = physical_fixture();
        let count = initial.chunks.keys().filter(|(bank, _)| *bank == 0).count();
        for failure in 0..=count {
            let (_, mut nvs) = physical_fixture();
            recover_snapshot(&mut nvs, Some(0)).unwrap();
            let old = nvs.selected();
            let old_marker = nvs.marker;
            let used = nvs.used();
            nvs.fail_chunk = (failure < count).then_some(failure);
            nvs.fail_publish = failure == count;
            assert!(commit_snapshot_transaction(&files, Some(0), &mut nvs).is_err());
            assert_eq!(nvs.marker, old_marker);
            assert_eq!(nvs.selected(), old);
            assert_eq!(nvs.used(), used);
            nvs.fail_chunk = None;
            nvs.fail_publish = false;
            let bank = commit_snapshot_transaction(&files, Some(0), &mut nvs).unwrap();
            assert_eq!(decode_packed_files(&nvs.selected()).unwrap(), files);
            for _ in 0..30 {
                let active = Some((nvs.marker & 1) as u8);
                commit_snapshot_transaction(&files, active, &mut nvs).unwrap();
                assert!(nvs.used() < nvs.capacity);
                assert_eq!(decode_packed_files(&nvs.selected()).unwrap(), files);
            }
            assert_eq!(bank, 1);
        }
    }
    #[test]
    fn repeated_updates_deletions_and_reboots_keep_only_the_current_snapshot() {
        let mut files = Files::new();
        let mut nvs = PhysicalNvs::default();
        let identity = Arc::new(pack::pack(b"stable-identity").unwrap());
        files.insert(
            "runtime/peer/local_identity.preferences.json".into(),
            identity.clone(),
        );
        for generation in 0..256 {
            // Vary both content and geometry: old long snapshots must not leave tail chunks.
            let transient = format!("runtime/peer/session-{generation}.preferences.json");
            files.insert(
                transient.clone(),
                Arc::new(pack::pack(&vec![generation as u8; 2048]).unwrap()),
            );
            let active = nvs.active_bank();
            commit_snapshot_transaction(&files, active, &mut nvs).unwrap();
            files.remove(&transient);
            let active = nvs.active_bank();
            commit_snapshot_transaction(&files, active, &mut nvs).unwrap();
            let active = nvs.active_bank();
            recover_snapshot(&mut nvs, active).unwrap();
            assert_eq!(decode_packed_files(&nvs.selected()).unwrap(), files);
            assert_eq!(files.len(), 1);
            assert!(nvs.chunks.keys().all(|(bank, _)| Some(*bank) == active));
            assert_eq!(
                nvs.chunks.len(),
                snapshot_length(nvs.marker)
                    .unwrap()
                    .div_ceil(SNAPSHOT_CHUNK_SIZE)
            );
            assert!(
                nvs.used() < 230,
                "NVS live entries grew at generation {generation}"
            );
        }
        assert_eq!(
            files["runtime/peer/local_identity.preferences.json"],
            identity
        );
    }

    #[test]
    fn compressed_chunks_reject_corruption() {
        let raw = vec![42; SNAPSHOT_CHUNK_SIZE];
        let packed = pack::pack(&raw).unwrap();
        assert!(packed.len() < raw.len());
        let mut restored = vec![0; raw.len()];
        decode_snapshot_chunk(&packed, &mut restored).unwrap();
        assert_eq!(restored, raw);
        assert!(decode_snapshot_chunk(&raw, &mut restored).is_err());
        for end in 0..packed.len() {
            assert!(decode_snapshot_chunk(&packed[..end], &mut restored).is_err());
        }
        assert!(decode_snapshot_chunk(&packed, &mut restored[..100]).is_err());
    }
    #[test]
    fn oversized_state_is_rejected_before_cleanup_and_capacity_failure_keeps_old_bank() {
        let (_, mut nvs) = physical_fixture();
        let old = nvs.selected();
        let old_marker = nvs.marker;
        let huge: Files = [(
            "runtime/too_large".into(),
            Arc::new(vec![0; STORAGE_BUDGET]),
        )]
        .into();
        assert!(commit_snapshot_transaction(&huge, Some(0), &mut nvs).is_err());
        assert_eq!(nvs.removals, 0);
        assert_eq!(nvs.marker, old_marker);
        assert_eq!(nvs.selected(), old);
        nvs.capacity = nvs.used()
            - nvs
                .chunks
                .iter()
                .filter(|((bank, _), _)| *bank == 1)
                .map(|(_, bytes)| bytes.len().div_ceil(32) + 2)
                .sum::<usize>();
        let files: Files = [(
            "runtime/large".into(),
            Arc::new({
                let mut seed = 0x12345678u32;
                (0..STORAGE_BUDGET - 64)
                    .map(|_| {
                        seed ^= seed << 13;
                        seed ^= seed >> 17;
                        seed ^= seed << 5;
                        seed as u8
                    })
                    .collect()
            }),
        )]
        .into();
        assert!(commit_snapshot_transaction(&files, Some(0), &mut nvs).is_err());
        assert_eq!(nvs.marker, old_marker);
        assert_eq!(nvs.selected(), old);
        assert!(nvs.chunks.keys().all(|(bank, _)| *bank == 0));
    }
}
