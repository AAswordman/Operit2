//! Bounded instruction batches turn tiny delta operations into coalesced Host I/O.
use super::*;
use std::io::BufReader;

const MAX_BASE_READS: usize = 16_384;
const MAX_READ_GAP: u64 = 64 * 1024;

struct BaseRead {
    offset: u64,
    target: usize,
    length: usize,
    residual: bool,
}

struct PatchOutput {
    writer: PendingWrite,
    hash: Sha256,
    prefix: Vec<u8>,
    bytes: Vec<u8>,
    length: usize,
    ranges: Vec<BaseRead>,
    readCalls: u64,
    readBytes: u64,
    writeCalls: u64,
}

impl PatchOutput {
    /// Allocates a fixed output window and a bounded list of deferred source ranges.
    fn new(context: &HostManager, destination: &str) -> Result<Self, String> {
        let writes = context
            .runtimeStorageWriteHost
            .as_ref()
            .ok_or("Update writer unavailable")?;
        Ok(Self {
            writer: PendingWrite(Some(
                writes
                    .createWriteSession(destination)
                    .map_err(|e| e.to_string())?,
            )),
            hash: Sha256::new(),
            prefix: Vec::with_capacity(8),
            bytes: vec![0; BUFFER],
            length: 0,
            ranges: Vec::with_capacity(MAX_BASE_READS),
            readCalls: 0,
            readBytes: 0,
            writeCalls: 0,
        })
    }

    /// Sorts source requests, combines nearby ranges, and emits one verified output window.
    fn flush(
        &mut self,
        base: &dyn FileSource,
        control: &HttpDownloadControl,
    ) -> Result<(), String> {
        if self.length == 0 {
            return Ok(());
        }
        ensureDownloadActive(control)?;
        self.ranges.sort_unstable_by_key(|range| range.offset);
        let mut first = 0;
        while first < self.ranges.len() {
            ensureDownloadActive(control)?;
            let start = self.ranges[first].offset;
            let mut end = start + self.ranges[first].length as u64;
            let mut last = first + 1;
            while last < self.ranges.len() {
                let range = &self.ranges[last];
                let nextEnd = end.max(range.offset + range.length as u64);
                if range.offset > end + MAX_READ_GAP || nextEnd - start > BUFFER as u64 {
                    break;
                }
                end = nextEnd;
                last += 1;
            }
            let count = (end - start) as usize;
            let bytes = base
                .readBytesRange(start, count)
                .map_err(|e| e.to_string())?;
            if bytes.len() != count {
                return Err("Truncated base package".into());
            }
            self.readCalls += 1;
            self.readBytes += count as u64;
            for range in &self.ranges[first..last] {
                let offset = (range.offset - start) as usize;
                let source = &bytes[offset..offset + range.length];
                let target = &mut self.bytes[range.target..range.target + range.length];
                if range.residual {
                    for (target, source) in target.iter_mut().zip(source) {
                        *target = source.wrapping_add(*target);
                    }
                } else {
                    target.copy_from_slice(source);
                }
            }
            first = last;
        }
        ensureDownloadActive(control)?;
        let bytes = &self.bytes[..self.length];
        self.prefix
            .extend_from_slice(&bytes[..bytes.len().min(8 - self.prefix.len())]);
        self.hash.update(bytes);
        self.writer
            .0
            .as_mut()
            .expect("pending update")
            .writeChunk(bytes)
            .map_err(|e| e.to_string())?;
        self.writeCalls += 1;
        self.length = 0;
        self.ranges.clear();
        Ok(())
    }
}

/// Applies authenticated COPY, ADD and residual instructions with bounded, coalesced I/O.
pub(super) fn applyInstructions(
    base: &dyn FileSource,
    decoder: &mut impl Read,
    baseSize: u64,
    targetSize: u64,
    targetHash: &str,
    residuals: bool,
    assetName: Option<&str>,
    context: &HostManager,
    destination: &str,
    control: &HttpDownloadControl,
    progress: &dyn Fn(u64, u64),
) -> Result<(), String> {
    let started = operit_host_api::TimeUtils::currentTimeMillisU128();
    let mut decoder = BufReader::with_capacity(BUFFER, decoder);
    let mut output = PatchOutput::new(context, destination)?;
    let mut written = 0u64;
    let mut instructions = 0u64;
    progress(0, targetSize);
    loop {
        ensureDownloadActive(control)?;
        let mut opcode = [0];
        decoder.read_exact(&mut opcode).map_err(|e| e.to_string())?;
        if opcode[0] == 0 {
            break;
        }
        let (offset, length) = match opcode[0] {
            1 => (integer(&mut decoder)?, integer(&mut decoder)?),
            3 if residuals => (integer(&mut decoder)?, integer(&mut decoder)?),
            2 => (0, integer(&mut decoder)?),
            _ => return Err("Invalid patch instruction".into()),
        };
        // Every instruction must advance the bounded output; no separate instruction cap is needed.
        if length == 0
            || length > targetSize.saturating_sub(written)
            || (opcode[0] != 2 && (offset > baseSize || length > baseSize - offset))
        {
            return Err("Patch instruction exceeds package bounds".into());
        }
        instructions += 1;
        let mut done = 0;
        while done < length {
            ensureDownloadActive(control)?;
            let count = (length - done).min((BUFFER - output.length) as u64) as usize;
            if opcode[0] != 2 {
                output.ranges.push(BaseRead {
                    offset: offset + done,
                    target: output.length,
                    length: count,
                    residual: opcode[0] == 3,
                });
            }
            if opcode[0] != 1 {
                decoder
                    .read_exact(&mut output.bytes[output.length..output.length + count])
                    .map_err(|e| e.to_string())?;
            }
            output.length += count;
            done += count as u64;
            if output.length == BUFFER || output.ranges.len() == MAX_BASE_READS {
                output.flush(base, control)?;
                progress(written + done, targetSize);
            }
        }
        written += length;
    }
    output.flush(base, control)?;
    let mut trailing = [0];
    if written != targetSize
        || decoder.read(&mut trailing).map_err(|e| e.to_string())? != 0
        || format!("{:x}", output.hash.finalize()) != targetHash
    {
        return Err("Reconstructed package SHA-256 or length mismatch".into());
    }
    if let Some(assetName) = assetName {
        validateDownloadedUpdatePackage(&output.prefix, assetName)?;
    }
    ensureDownloadActive(control)?;
    output
        .writer
        .0
        .take()
        .expect("pending update")
        .commit()
        .map_err(|e| e.to_string())?;
    progress(targetSize, targetSize);
    crate::AppLogger::AppLogger::i(
        "ApplicationUpdate",
        &format!(
            "Patch merge: outputBytes={written}, instructions={instructions}, baseReads={}, baseReadBytes={}, writes={}, elapsedMs={}",
            output.readCalls, output.readBytes, output.writeCalls,
            operit_host_api::TimeUtils::currentTimeMillisU128().saturating_sub(started),
        ),
    );
    Ok(())
}
