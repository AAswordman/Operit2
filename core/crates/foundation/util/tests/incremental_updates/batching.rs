use super::*;
use operit_host_api::{FileInfo, HostResult, RuntimeStorageWriteHost, RuntimeStorageWriteSession};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

const BUFFER: usize = 1024 * 1024;

struct CountedSource<'a> {
    source: &'a dyn FileSource,
    calls: AtomicUsize,
    cancel_on_read: Option<(usize, HttpDownloadControl)>,
}

impl FileSource for CountedSource<'_> {
    /// Preserves the original portable file capability identity.
    fn vfsPath(&self) -> &str {
        self.source.vfsPath()
    }

    /// Rejects accidental whole-package reads in the production merge path.
    fn readBytes(&self) -> HostResult<Vec<u8>> {
        panic!("merge must use bounded reads")
    }

    /// Counts real Host requests and optionally cancels during a deferred batch read.
    fn readBytesRange(&self, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        assert!(length <= BUFFER);
        let bytes = self.source.readBytesRange(offset, length)?;
        let calls = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        if let Some((at, control)) = &self.cancel_on_read {
            if calls == *at {
                control.cancel();
            }
        }
        Ok(bytes)
    }

    /// Uses actual source metadata so size and digest verification stay enabled.
    fn fileInfo(&self) -> HostResult<FileInfo> {
        self.source.fileInfo()
    }

    /// Delegates native interoperation without using it for batch reconstruction.
    fn nativePath(&self) -> HostResult<String> {
        self.source.nativePath()
    }
}

struct CountedWrites {
    host: Arc<dyn RuntimeStorageWriteHost>,
    calls: Arc<AtomicUsize>,
}

struct CountedWriter {
    writer: Box<dyn RuntimeStorageWriteSession>,
    calls: Arc<AtomicUsize>,
}

impl RuntimeStorageWriteHost for CountedWrites {
    /// Wraps the real transactional writer so publication and cleanup remain observable.
    fn createWriteSession(&self, path: &str) -> HostResult<Box<dyn RuntimeStorageWriteSession>> {
        Ok(Box::new(CountedWriter {
            writer: self.host.createWriteSession(path)?,
            calls: self.calls.clone(),
        }))
    }
}

impl RuntimeStorageWriteSession for CountedWriter {
    /// Measures physical write calls and enforces the output memory bound.
    fn writeChunk(&mut self, bytes: &[u8]) -> HostResult<()> {
        assert!(bytes.len() <= BUFFER);
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.writer.writeChunk(bytes)
    }

    /// Publishes through the production atomic commit implementation.
    fn commit(self: Box<Self>) -> HostResult<()> {
        self.writer.commit()
    }

    /// Discards through the production staging-file cleanup implementation.
    fn discard(self: Box<Self>) -> HostResult<()> {
        self.writer.discard()
    }
}

/// Encodes independently authored instructions with fully authenticated package headers.
fn stage(fixture: &mut Fixture, base: &[u8], target: Vec<u8>, instructions: &[u8]) {
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gzip.write_all(b"O2PATCH2\0").unwrap();
    gzip.write_all(&(base.len() as u64).to_le_bytes()).unwrap();
    gzip.write_all(&(target.len() as u64).to_le_bytes())
        .unwrap();
    gzip.write_all(&Sha256::digest(base)).unwrap();
    gzip.write_all(&Sha256::digest(&target)).unwrap();
    gzip.write_all(instructions).unwrap();
    gzip.write_all(&[0]).unwrap();
    let patch = gzip.finish().unwrap();
    fixture.manifest.schema = 2;
    fixture.manifest.size = target.len() as u64;
    fixture.manifest.sha256 = format!("{:x}", Sha256::digest(&target));
    let declaration = &mut fixture.manifest.patches[0];
    declaration.baseSize = base.len() as u64;
    declaration.baseSha256 = format!("{:x}", Sha256::digest(base));
    declaration.size = patch.len() as u64;
    declaration.sha256 = format!("{:x}", Sha256::digest(&patch));
    let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
    storage
        .writeBytes("runtime/temp/application_updates/test/base.apk", base)
        .unwrap();
    storage
        .writeBytes(
            "runtime/temp/application_updates/test/delta.patch.gz",
            &patch,
        )
        .unwrap();
    fixture.target = target;
}

/// Exercises reordered and overlapping reads, literals, wrapping residuals, and split large COPYs.
#[test]
fn tiny_mixed_instructions_use_bounded_coalesced_host_io() {
    let mut fixture = Fixture::new();
    let mut base = (0..3 * BUFFER + 17)
        .map(|i| (i.wrapping_mul(73) ^ (i >> 5)) as u8)
        .collect::<Vec<_>>();
    base[..4].copy_from_slice(b"PK\x03\x04");
    let mut target = base[..4].to_vec();
    let mut instructions = vec![1];
    instructions.extend_from_slice(&0u64.to_le_bytes());
    instructions.extend_from_slice(&4u64.to_le_bytes());
    for i in 0..65_537usize {
        let length = 17 + i % 31;
        let offset = (i * 8191) % (base.len() - length);
        let opcode = [1, 2, 3][i % 3];
        instructions.push(opcode);
        if opcode != 2 {
            instructions.extend_from_slice(&(offset as u64).to_le_bytes());
        }
        instructions.extend_from_slice(&(length as u64).to_le_bytes());
        for byte in &base[offset..offset + length] {
            target.push(match opcode {
                1 => *byte,
                2 => 251,
                _ => byte.wrapping_add(251),
            });
        }
        if opcode != 1 {
            instructions.extend(std::iter::repeat(251).take(length));
        }
    }
    instructions.push(1);
    instructions.extend_from_slice(&17u64.to_le_bytes());
    instructions.extend_from_slice(&((2 * BUFFER + 7) as u64).to_le_bytes());
    target.extend_from_slice(&base[17..17 + 2 * BUFFER + 7]);
    stage(&mut fixture, &base, target, &instructions);
    let writes = Arc::new(AtomicUsize::new(0));
    fixture.context.runtimeStorageWriteHost = Some(Arc::new(CountedWrites {
        host: fixture.context.runtimeStorageWriteHost.clone().unwrap(),
        calls: writes.clone(),
    }));
    let source = CountedSource {
        source: &fixture.base,
        calls: AtomicUsize::new(0),
        cancel_on_read: None,
    };
    applyPatch(
        &source,
        &fixture.patch,
        &fixture.manifest,
        &fixture.manifest.patches[0],
        &fixture.context,
        "runtime/temp/application_updates/test/output.apk",
        &HttpDownloadControl::new(),
    )
    .unwrap();
    assert_eq!(fixture.output(), fixture.target);
    assert!(
        source.calls.load(Ordering::Relaxed) < 100,
        "tiny ranges must share Host reads"
    );
    assert!(
        writes.load(Ordering::Relaxed) < 16,
        "tiny instructions must share output writes"
    );
}

/// Cancellation during a coalesced read must discard all staged output before publication.
#[test]
fn cancellation_during_deferred_read_discards_the_output_batch() {
    let fixture = Fixture::new();
    let control = HttpDownloadControl::new();
    let source = CountedSource {
        source: &fixture.base,
        calls: AtomicUsize::new(0),
        cancel_on_read: Some((2, control.clone())),
    };
    assert_eq!(
        applyPatch(
            &source,
            &fixture.patch,
            &fixture.manifest,
            &fixture.manifest.patches[0],
            &fixture.context,
            "runtime/temp/application_updates/test/output.apk",
            &control
        )
        .unwrap_err(),
        "UPDATE_CANCELLED"
    );
    assert!(!fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .exists("runtime/temp/application_updates/test/output.apk")
        .unwrap());
    let base_path = fixture.base.nativePath().unwrap();
    assert_eq!(
        std::fs::read_dir(std::path::Path::new(&base_path).parent().unwrap())
            .unwrap()
            .count(),
        2
    );
}

/// Late malformed operations and target mismatches must remove already-written staging windows.
#[test]
fn invalid_streams_discard_previous_output_windows() {
    let mut base = vec![73; 2 * BUFFER + 17];
    base[..4].copy_from_slice(b"PK\x03\x04");
    let mut prefix = vec![1];
    prefix.extend_from_slice(&0u64.to_le_bytes());
    prefix.extend_from_slice(&(BUFFER as u64).to_le_bytes());
    let mut cases = Vec::new();
    // COPY length zero, a base read outside the source, a missing residual body, and trailing bytes.
    for (offset, length) in [(0u64, 0u64), (base.len() as u64, 1)] {
        let mut instructions = prefix.clone();
        instructions.push(1);
        instructions.extend_from_slice(&offset.to_le_bytes());
        instructions.extend_from_slice(&length.to_le_bytes());
        cases.push((instructions, base.clone()));
    }
    let mut truncated = prefix;
    truncated.push(3);
    truncated.extend_from_slice(&0u64.to_le_bytes());
    truncated.extend_from_slice(&17u64.to_le_bytes());
    truncated.extend_from_slice(&[1; 3]);
    cases.push((truncated, base.clone()));
    let mut copy = vec![1];
    copy.extend_from_slice(&0u64.to_le_bytes());
    copy.extend_from_slice(&(base.len() as u64).to_le_bytes());
    let mut trailing = copy.clone();
    trailing.extend_from_slice(&[0, 42]);
    cases.push((trailing, base.clone()));
    let mut wrong_target = base.clone();
    *wrong_target.last_mut().unwrap() ^= 1;
    cases.push((copy, wrong_target));
    for (instructions, target) in cases {
        let mut fixture = Fixture::new();
        stage(&mut fixture, &base, target, &instructions);
        let writes = Arc::new(AtomicUsize::new(0));
        fixture.context.runtimeStorageWriteHost = Some(Arc::new(CountedWrites {
            host: fixture.context.runtimeStorageWriteHost.clone().unwrap(),
            calls: writes.clone(),
        }));
        assert!(fixture.apply(&HttpDownloadControl::new()).is_err());
        assert!(writes.load(Ordering::Relaxed) >= 1);
        let path = fixture.base.nativePath().unwrap();
        assert_eq!(
            std::fs::read_dir(std::path::Path::new(&path).parent().unwrap())
                .unwrap()
                .count(),
            2
        );
    }
}

/// Valid generator output can exceed four million COPY/ADD instructions while still advancing safely.
#[test]
fn more_than_four_million_tiny_instructions_remain_valid() {
    let mut fixture = Fixture::new();
    let base = b"PK\x03\x04x";
    let mut target = base[..4].to_vec();
    let mut instruction = vec![1];
    instruction.extend_from_slice(&4u64.to_le_bytes());
    instruction.extend_from_slice(&1u64.to_le_bytes());
    let mut instructions = vec![1];
    instructions.extend_from_slice(&0u64.to_le_bytes());
    instructions.extend_from_slice(&4u64.to_le_bytes());
    for _ in 0..4_000_001 {
        instructions.extend_from_slice(&instruction);
    }
    target.resize(4_000_005, b'x');
    stage(&mut fixture, base, target, &instructions);
    fixture.apply(&HttpDownloadControl::new()).unwrap();
    assert_eq!(fixture.output(), fixture.target);
}
