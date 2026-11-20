//! Byte-exact CAB expansion and restoration; no installation or package code is executed.
use operit_host_api::{ApplicationUpdateTransform, HttpDownloadControl};
use serde::Deserialize;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MAGIC: &[u8; 8] = b"O2MSI1\0\0";
const LIMIT: u64 = 16 * 1024 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    size: u64,
    folders: Vec<Folder>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Folder {
    compression: u16,
    raw_size: u64,
    ranges: Vec<[u64; 2]>,
}

/// Reports malformed package data without panicking or accepting unchecked offsets.
fn require(condition: bool, message: &str) -> io::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, message))
    }
}

/// Notices cancellation between native operations and bounded file copies.
fn active(control: &HttpDownloadControl) -> io::Result<()> {
    require(!control.isCancelled(), "UPDATE_CANCELLED")
}

/// Copies exactly the declared bytes with bounded memory and cancellation.
fn copy(
    source: &mut File,
    target: &mut File,
    mut length: u64,
    control: &HttpDownloadControl,
) -> io::Result<()> {
    let mut buffer = vec![0; 1024 * 1024];
    while length > 0 {
        active(control)?;
        let count = length.min(buffer.len() as u64) as usize;
        source.read_exact(&mut buffer[..count])?;
        target.write_all(&buffer[..count])?;
        length -= count as u64;
    }
    Ok(())
}

/// Decodes a checked little-endian field from a package header.
fn number(bytes: &[u8], offset: usize, count: usize) -> io::Result<u64> {
    let field = bytes
        .get(offset..offset + count)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Truncated MSI field"))?;
    Ok(field
        .iter()
        .enumerate()
        .map(|(i, byte)| u64::from(*byte) << (i * 8))
        .sum())
}

impl Recipe {
    /// Validates output bounds, expansion limits and nonoverlapping physical ranges.
    fn ranges(&self) -> io::Result<Vec<[u64; 2]>> {
        require(
            self.size > 0
                && self.size <= LIMIT
                && !self.folders.is_empty()
                && self.folders.len() <= 4096,
            "Invalid MSI recipe size",
        )?;
        let mut ranges = self
            .folders
            .iter()
            .flat_map(|folder| &folder.ranges)
            .copied()
            .collect::<Vec<_>>();
        ranges.sort_unstable();
        let mut end = 0;
        for [offset, count] in &ranges {
            require(
                *offset >= end
                    && *offset <= self.size
                    && *count > 0
                    && *count <= self.size - offset,
                "Invalid MSI recipe ranges",
            )?;
            end = offset + count;
        }
        let mut total = 0;
        for folder in &self.folders {
            require(
                folder.raw_size > 0
                    && folder.raw_size <= 32768 * 65535
                    && !folder.ranges.is_empty()
                    && (15..=21).contains(&(folder.compression >> 8))
                    && folder.compression & 255 == 3,
                "Invalid MSI folder recipe",
            )?;
            total += folder.raw_size;
        }
        require(total <= LIMIT, "Invalid MSI expanded size")?;
        Ok(ranges)
    }
}

/// Runs built-in cabinet tools invisibly and kills/reaps the child on cancellation.
fn cabinet(
    program: &str,
    args: &[&str],
    work: &Path,
    control: &HttpDownloadControl,
) -> io::Result<()> {
    active(control)?;
    let system = std::env::var_os("SystemRoot").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Windows system directory is missing",
        )
    })?;
    let mut child =
        crate::process::background_command(Path::new(&system).join("System32").join(program))
            .args(args)
            .current_dir(work)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    loop {
        if control.isCancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return active(control);
        }
        if let Some(status) = child.try_wait()? {
            return require(
                status.success(),
                &format!("Windows CAB tool {program} failed: {status}"),
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Synthesizes one safe filename around the original compressed folder bytes.
fn unpack(
    source: &mut File,
    folder: &Folder,
    work: &Path,
    control: &HttpDownloadControl,
) -> io::Result<()> {
    let length = folder.ranges.iter().map(|range| range[1]).sum::<u64>();
    require(
        length + 68 <= u32::MAX as u64,
        "CAB folder exceeds format limit",
    )?;
    let mut header = [0; 68];
    header[..4].copy_from_slice(b"MSCF");
    header[8..12].copy_from_slice(&((length + 68) as u32).to_le_bytes());
    header[16..20].copy_from_slice(&44u32.to_le_bytes());
    header[24..30].copy_from_slice(&[3, 1, 1, 0, 1, 0]);
    header[36..40].copy_from_slice(&68u32.to_le_bytes());
    header[40..42].copy_from_slice(&(folder.raw_size.div_ceil(32768) as u16).to_le_bytes());
    header[42..44].copy_from_slice(&folder.compression.to_le_bytes());
    header[44..48].copy_from_slice(&(folder.raw_size as u32).to_le_bytes());
    header[60..68].copy_from_slice(b"payload\0");
    let mut cab = File::create(work.join("input.cab"))?;
    cab.write_all(&header)?;
    for [offset, count] in &folder.ranges {
        source.seek(SeekFrom::Start(*offset))?;
        copy(source, &mut cab, *count, control)?;
    }
    drop(cab);
    let payload = work.join("payload");
    if payload.exists() {
        std::fs::remove_file(&payload)?;
    }
    cabinet(
        "expand.exe",
        &["-r", "-F:payload", "input.cab", "."],
        work,
        control,
    )?;
    require(
        payload.metadata()?.len() == folder.raw_size,
        "Expanded CAB size mismatch",
    )
}

/// Processes independent folders largest-first with at most four cancellable CAB workers.
fn folders(
    recipe: &Recipe,
    work: &Path,
    control: &HttpDownloadControl,
    operation: impl Fn(usize, &Folder, &Path, &HttpDownloadControl) -> io::Result<()> + Sync,
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> io::Result<()> {
    let mut order = (0..recipe.folders.len()).collect::<Vec<_>>();
    order.sort_unstable_by_key(|&index| std::cmp::Reverse(recipe.folders[index].raw_size));
    let workers = std::thread::available_parallelism()?
        .get()
        .min(4)
        .min(order.len());
    let next = AtomicUsize::new(0);
    let failure = Mutex::new(None);
    let completed = Mutex::new(0u64);
    let total = recipe.folders.iter().map(|folder| folder.raw_size).sum();
    progress(0, total);
    let jobs = HttpDownloadControl::new();
    let cancelled = jobs.clone();
    let _subscription = control.onCancel(Arc::new(move || cancelled.cancel()));
    std::thread::scope(|scope| {
        let mut threads = Vec::with_capacity(workers);
        for _ in 0..workers {
            let thread = std::thread::Builder::new()
                .name("update-cab".into())
                .spawn_scoped(scope, || {
                    while !jobs.isCancelled() {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&index) = order.get(index) else {
                            break;
                        };
                        let result = tempfile::Builder::new()
                            .prefix("folder-")
                            .tempdir_in(work)
                            .and_then(|directory| {
                                operation(index, &recipe.folders[index], directory.path(), &jobs)
                            });
                        if let Err(error) = result {
                            failure
                                .lock()
                                .expect("CAB worker error")
                                .get_or_insert(error);
                            jobs.cancel();
                            break;
                        }
                        let mut completed = completed.lock().expect("CAB completed bytes");
                        *completed += recipe.folders[index].raw_size;
                        progress(*completed, total);
                    }
                });
            match thread {
                Ok(thread) => threads.push(thread),
                Err(error) => {
                    failure
                        .lock()
                        .expect("CAB worker error")
                        .get_or_insert(error);
                    jobs.cancel();
                    break;
                }
            }
        }
        for thread in threads {
            if thread.join().is_err() {
                failure
                    .lock()
                    .expect("CAB worker error")
                    .get_or_insert_with(|| io::Error::other("CAB worker panicked"));
                jobs.cancel();
            }
        }
    });
    if let Some(error) = failure.into_inner().expect("CAB worker error") {
        return Err(error);
    }
    active(control)
}

/// Serializes the compact installer shell followed by all uncompressed folders.
fn decode(
    source: &Path,
    output: &mut File,
    output_path: &Path,
    work: &Path,
    metadata: &[u8],
    control: &HttpDownloadControl,
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> io::Result<()> {
    require(
        metadata.len() <= 4 * 1024 * 1024,
        "MSI recipe exceeds limit",
    )?;
    let recipe: Recipe = serde_json::from_slice(metadata)?;
    let ranges = recipe.ranges()?;
    let mut original = File::open(source)?;
    require(
        original.metadata()?.len() == recipe.size,
        "MSI recipe does not match original size",
    )?;
    output.write_all(MAGIC)?;
    output.write_all(&(metadata.len() as u32).to_le_bytes())?;
    output.write_all(metadata)?;
    let mut offset = 0;
    for [start, count] in ranges {
        original.seek(SeekFrom::Start(offset))?;
        copy(&mut original, output, start - offset, control)?;
        offset = start + count;
    }
    original.seek(SeekFrom::Start(offset))?;
    copy(&mut original, output, recipe.size - offset, control)?;
    let mut offset = output.stream_position()?;
    let offsets = recipe
        .folders
        .iter()
        .map(|folder| {
            let start = offset;
            offset += folder.raw_size;
            start
        })
        .collect::<Vec<_>>();
    output.set_len(offset)?;
    folders(
        &recipe,
        work,
        control,
        |index, folder, work, control| {
            unpack(&mut File::open(source)?, folder, work, control)?;
            let mut output = std::fs::OpenOptions::new().write(true).open(output_path)?;
            output.seek(SeekFrom::Start(offsets[index]))?;
            copy(
                &mut File::open(work.join("payload"))?,
                &mut output,
                folder.raw_size,
                control,
            )
        },
        progress,
    )
}

/// Restores CAB byte streams into their original sectors, preserving signatures and installer metadata.
fn encode(
    source: &Path,
    output: &mut File,
    output_path: &Path,
    work: &Path,
    control: &HttpDownloadControl,
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> io::Result<()> {
    let mut image = File::open(source)?;
    let mut header = [0; 12];
    image.read_exact(&mut header)?;
    require(&header[..8] == MAGIC, "Invalid expanded MSI header")?;
    let length = number(&header, 8, 4)?;
    require(length <= 4 * 1024 * 1024, "MSI recipe exceeds limit")?;
    let mut metadata = vec![0; length as usize];
    image.read_exact(&mut metadata)?;
    let recipe: Recipe = serde_json::from_slice(&metadata)?;
    let ranges = recipe.ranges()?;
    let expected = 12 + length + recipe.size - ranges.iter().map(|range| range[1]).sum::<u64>()
        + recipe.folders.iter().map(|f| f.raw_size).sum::<u64>();
    require(
        image.metadata()?.len() == expected,
        "Expanded MSI length mismatch",
    )?;
    let mut offset = 0;
    for [start, count] in ranges {
        output.seek(SeekFrom::Start(offset))?;
        copy(&mut image, output, start - offset, control)?;
        offset = start + count;
    }
    output.seek(SeekFrom::Start(offset))?;
    copy(&mut image, output, recipe.size - offset, control)?;
    output.set_len(recipe.size)?;
    let mut offset = image.stream_position()?;
    let offsets = recipe
        .folders
        .iter()
        .map(|folder| {
            let start = offset;
            offset += folder.raw_size;
            start
        })
        .collect::<Vec<_>>();
    folders(
        &recipe,
        work,
        control,
        |index, folder, work, control| {
            let mut image = File::open(source)?;
            image.seek(SeekFrom::Start(offsets[index]))?;
            copy(
                &mut image,
                &mut File::create(work.join("payload"))?,
                folder.raw_size,
                control,
            )?;
            let mut output = std::fs::OpenOptions::new().write(true).open(output_path)?;
            let memory = format!("CompressionMemory={}", folder.compression >> 8);
            cabinet(
                "makecab.exe",
                &[
                    "/D",
                    "CompressionType=LZX",
                    "/D",
                    &memory,
                    "/D",
                    "FolderSizeThreshold=1000000000",
                    "/D",
                    "MaxDiskSize=0",
                    "/D",
                    "MaxCabinetSize=0",
                    "payload",
                    "output.cab",
                ],
                work,
                control,
            )?;
            let mut cab = File::open(work.join("output.cab"))?;
            cab.seek(SeekFrom::Start(36))?;
            let mut header = [0; 8];
            cab.read_exact(&mut header)?;
            let start = number(&header, 0, 4)?;
            require(
                number(&header, 6, 2)? == u64::from(folder.compression)
                    && number(&header, 4, 2)? == folder.raw_size.div_ceil(32768)
                    && cab.metadata()?.len().checked_sub(start)
                        == Some(folder.ranges.iter().map(|range| range[1]).sum()),
                "Windows CAB encoder does not reproduce target layout",
            )?;
            cab.seek(SeekFrom::Start(start))?;
            for [offset, count] in &folder.ranges {
                output.seek(SeekFrom::Start(*offset))?;
                copy(&mut cab, &mut output, *count, control)?;
            }
            Ok(())
        },
        progress,
    )
}

/// Publishes only completed transforms; temporary files remain inside the disposable update job.
pub(super) fn transform(
    source: &Path,
    target: &Path,
    transform: ApplicationUpdateTransform,
    control: &HttpDownloadControl,
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> io::Result<()> {
    active(control)?;
    let parent = target.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "Update target has no parent")
    })?;
    std::fs::create_dir_all(parent)?;
    let work = tempfile::Builder::new().prefix("cab-").tempdir_in(parent)?;
    let mut output = tempfile::Builder::new()
        .prefix("cab-")
        .tempfile_in(parent)?;
    let output_path = output.path().to_path_buf();
    match transform {
        ApplicationUpdateTransform::ExpandMsiCabinets { recipe } => decode(
            source,
            output.as_file_mut(),
            &output_path,
            work.path(),
            &recipe,
            control,
            progress,
        )?,
        ApplicationUpdateTransform::RestoreMsiCabinets => encode(
            source,
            output.as_file_mut(),
            &output_path,
            work.path(),
            control,
            progress,
        )?,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Unsupported Windows installer transform",
            ))
        }
    }
    active(control)?;
    output.as_file_mut().flush()?;
    output
        .persist_noclobber(target)
        .map_err(|error| error.error)?;
    Ok(())
}
