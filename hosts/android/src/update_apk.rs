//! Reversible APK entry transforms using the Android Host's standard zlib encoder.
use operit_host_api::{ApplicationUpdateTransform, HttpDownloadControl};
use serde::Deserialize;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

const MAGIC: &[u8; 8] = b"O2APK1\0\0";
const BUFFER: usize = 1024 * 1024;
const LIMIT: u64 = 16 * 1024 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    offset: u64,
    compressed: u64,
    size: u64,
    level: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    size: u64,
    entries: Vec<Entry>,
}

/// Produces a consistent invalid-data error for malformed authenticated transform recipes.
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Checks cancellation between every bounded read and compression write.
fn active(control: &HttpDownloadControl) -> io::Result<()> {
    if control.isCancelled() {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "UPDATE_CANCELLED",
        ));
    }
    Ok(())
}

/// Parses only bounded, nonoverlapping entry ranges with a fixed reproducible codec.
fn recipe(metadata: &[u8]) -> io::Result<Recipe> {
    if metadata.len() > 4 * BUFFER {
        return Err(invalid("APK recipe exceeds limit"));
    }
    let recipe: Recipe = serde_json::from_slice(metadata).map_err(io::Error::other)?;
    if recipe.size == 0 || recipe.size > LIMIT || recipe.entries.len() > 4096 {
        return Err(invalid("Invalid APK recipe size"));
    }
    let (mut end, mut total) = (0, 0u64);
    for entry in &recipe.entries {
        if entry.offset < end
            || entry.offset > recipe.size
            || entry.compressed == 0
            || entry.compressed > recipe.size - entry.offset
            || entry.size == 0
            || entry.size > LIMIT
            || entry.level != 6
        {
            return Err(invalid("Invalid APK entry recipe"));
        }
        end = entry.offset + entry.compressed;
        total = total
            .checked_add(entry.size)
            .ok_or_else(|| invalid("APK expanded size overflow"))?;
        if total > LIMIT {
            return Err(invalid("APK expanded size exceeds limit"));
        }
    }
    Ok(recipe)
}

/// Copies an exact byte count with bounded buffers and prompt cancellation.
fn copy(
    source: &mut impl Read,
    target: &mut impl Write,
    mut count: u64,
    control: &HttpDownloadControl,
) -> io::Result<()> {
    let mut buffer = vec![0; BUFFER];
    while count > 0 {
        active(control)?;
        let length = count.min(BUFFER as u64) as usize;
        source.read_exact(&mut buffer[..length])?;
        target.write_all(&buffer[..length])?;
        count -= length as u64;
    }
    Ok(())
}

/// Expands selected raw DEFLATE streams and preserves the untouched signed APK shell.
fn decode(
    source: &mut File,
    output: &mut File,
    metadata: &[u8],
    control: &HttpDownloadControl,
) -> io::Result<()> {
    let recipe = recipe(metadata)?;
    if source.metadata()?.len() != recipe.size {
        return Err(invalid("APK recipe does not match original size"));
    }
    output.write_all(MAGIC)?;
    output.write_all(&(metadata.len() as u32).to_le_bytes())?;
    output.write_all(metadata)?;
    let mut offset = 0;
    for entry in &recipe.entries {
        source.seek(SeekFrom::Start(offset))?;
        copy(source, output, entry.offset - offset, control)?;
        offset = entry.offset + entry.compressed;
    }
    source.seek(SeekFrom::Start(offset))?;
    copy(source, output, recipe.size - offset, control)?;
    for entry in &recipe.entries {
        source.seek(SeekFrom::Start(entry.offset))?;
        let mut decoder = flate2::read::DeflateDecoder::new((&mut *source).take(entry.compressed));
        copy(&mut decoder, output, entry.size, control)?;
        let mut trailing = [0];
        if decoder.read(&mut trailing)? != 0 || decoder.total_in() != entry.compressed {
            return Err(invalid("Expanded APK entry does not match recipe"));
        }
    }
    Ok(())
}

struct EntryWriter<'a> {
    file: &'a mut File,
    remaining: u64,
}

impl Write for EntryWriter<'_> {
    /// Prevents the encoder from overwriting APK shell bytes outside the declared entry.
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(invalid("APK compressed entry exceeds recipe"));
        }
        let count = self.file.write(bytes)?;
        self.remaining -= count as u64;
        Ok(count)
    }

    /// Flushes the destination without changing the DEFLATE stream.
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// Recreates each original DEFLATE stream without rewriting APK metadata or signatures.
fn encode(source: &mut File, output: &mut File, control: &HttpDownloadControl) -> io::Result<()> {
    let mut magic = [0; 8];
    source.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(invalid("Invalid expanded APK header"));
    }
    let mut length = [0; 4];
    source.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    if length > 4 * BUFFER {
        return Err(invalid("APK recipe exceeds limit"));
    }
    let mut metadata = vec![0; length];
    source.read_exact(&mut metadata)?;
    let recipe = recipe(&metadata)?;
    let shell = recipe.size - recipe.entries.iter().map(|e| e.compressed).sum::<u64>();
    let expanded = recipe.entries.iter().map(|e| e.size).sum::<u64>();
    if source.metadata()?.len() != 12 + length as u64 + shell + expanded {
        return Err(invalid("Expanded APK length mismatch"));
    }
    let mut offset = 0;
    for entry in &recipe.entries {
        output.seek(SeekFrom::Start(offset))?;
        copy(source, output, entry.offset - offset, control)?;
        offset = entry.offset + entry.compressed;
    }
    output.seek(SeekFrom::Start(offset))?;
    copy(source, output, recipe.size - offset, control)?;
    output.set_len(recipe.size)?;
    for entry in &recipe.entries {
        output.seek(SeekFrom::Start(entry.offset))?;
        let writer = EntryWriter {
            file: output,
            remaining: entry.compressed,
        };
        let mut encoder =
            flate2::write::DeflateEncoder::new(writer, flate2::Compression::new(entry.level));
        copy(source, &mut encoder, entry.size, control)?;
        if encoder.finish()?.remaining != 0 {
            return Err(invalid("APK encoder does not reproduce target layout"));
        }
    }
    Ok(())
}

/// Writes only inside the disposable update job and removes incomplete transforms on all failures.
pub(super) fn transform(
    source: &Path,
    target: &Path,
    transform: ApplicationUpdateTransform,
    control: &HttpDownloadControl,
) -> io::Result<()> {
    active(control)?;
    let mut input = File::open(source)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    let result = match transform {
        ApplicationUpdateTransform::ExpandApkEntries { recipe } => {
            decode(&mut input, &mut output, &recipe, control)
        }
        ApplicationUpdateTransform::RestoreApkEntries => encode(&mut input, &mut output, control),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Unsupported Android installer transform",
        )),
    }
    .and_then(|_| active(control))
    .and_then(|_| output.flush());
    drop(output);
    if result.is_err() {
        std::fs::remove_file(target)?;
    }
    result
}
