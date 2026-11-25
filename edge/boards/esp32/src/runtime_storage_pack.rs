//! Small bounded LZSS records for the ESP32 virtual storage cache.
//! Records and range-read history are bounded; no whole-database decompression.
use operit_host_api::{HostError, HostResult};
use std::io::{self, Write};

pub const MAX_RECORD_SIZE: usize = 16 * 1024;
const WINDOW: usize = 4096;

/// Immutable initial LZSS history for format 2. These public field names live
/// in flash; no per-record dictionary is allocated. Never reorder or edit this
/// dictionary without assigning a new format byte. Formats 0 and 1 remain readable.
const DICTIONARY: &[u8] = concat!(
    "\"__operit_preferences_schema_version\":\"record\":\"nodeId\":\"spaceId\":\"pairingIncarnation\":\"updatedAt\":",
    "\"joinedAt\":\"displayName\":\"userName\":\"platform\":\"model\":\"coreVersion\":\"deviceId\":\"deviceInfo\":",
    "\"spaceName\":\"spaceRevision\":\"createdAt\":\"requestId\":\"targetDeviceId\":\"applicantDeviceId\":",
    "\"applicantName\":\"status\":\"expiresAt\":\"canApprove\":\"reviewerDeviceId\":\"reviewerName\":\"reviewerHops\":",
    "\"assignmentVersion\":\"decisionApprove\":\"sourceSpaceId\":\"sourceRevision\":\"targetSpaceId\":\"profile\":",
    "\"source\":\"space\":\"members\":\"deviceProfiles\":\"controlOperations\":\"topology\":\"approvedDecision\":",
    "\"decisionRevision\":\"operation\":\"domain\":\"entityId\":\"entityType\":\"opId\":\"originDeviceId\":\"payload\":",
    "\"command\":\"commandId\":\"issuerNodeId\":\"schemaVersion\":\"semantics\":\"sequence\":\"peers\":\"links\":",
    "\"bootstrap\":\"initialAdminNodeId\":\"admit_space\":\"identities\":\"null\":\"true\":\"false\":[]{}",
).as_bytes();

fn invalid() -> HostError {
    HostError::new("Invalid packed runtime storage record")
}
fn allocation(length: usize) -> HostError {
    HostError::new(format!(
        "Packed runtime storage allocation failed ({length} bytes)"
    ))
}

#[inline(never)]
fn match_at(input: &[u8], at: usize) -> (usize, usize) {
    let mut best = (0, 0);
    let dictionary = DICTIONARY;
    let position = dictionary.len() + at;
    let byte = |index: usize| {
        if index < dictionary.len() { dictionary[index] }
        else { input[index - dictionary.len()] }
    };
    for previous in (position.saturating_sub(WINDOW)..position).rev() {
        if byte(previous) != input[at] { continue; }
        let mut length = 1;
        while length < 18 && at + length < input.len()
            && byte(previous + length) == input[at + length] {
            length += 1;
        }
        if length >= 3 && length > best.1 { best = (position - previous, length); }
        if length == 18 { break; }
    }
    best
}
// A bounded hash chain skips positions that cannot match three bytes. Keep
// every candidate, newest first, so persisted output and tie-breaking stay exact.
struct MatchIndex {
    heads: [u16; 64],
    previous: Vec<u8>,
    next: usize,
}
impl MatchIndex {
    fn new(length: usize) -> HostResult<Self> {
        let size = if length < 512 { 0 } else { WINDOW.min(DICTIONARY.len() + length) };
        let mut previous = Vec::new();
        previous.try_reserve_exact(size).map_err(|_| allocation(size))?;
        previous.resize(size, 0);
        Ok(Self { heads: [u16::MAX; 64], previous, next: 0 })
    }
    fn reset(&mut self) { self.heads.fill(u16::MAX); self.next = 0; }
    fn byte(input: &[u8], index: usize) -> u8 {
        if index < DICTIONARY.len() { DICTIONARY[index] }
        else { input[index - DICTIONARY.len()] }
    }
    fn hash(input: &[u8], index: usize) -> usize {
        (Self::byte(input, index) as usize * 31 * 31
            + Self::byte(input, index + 1) as usize * 31
            + Self::byte(input, index + 2) as usize) & 63
    }
    fn find(&mut self, input: &[u8], at: usize) -> (usize, usize) {
        if self.previous.is_empty() { return match_at(input, at); }
        if input.len() - at < 3 { return (0, 0); }
        let position = DICTIONARY.len() + at;
        while self.next < position {
            let hash = Self::hash(input, self.next);
            let slot = self.next % self.previous.len();
            self.previous[slot] = if self.heads[hash] == u16::MAX { 0 }
                else { (self.next - self.heads[hash] as usize).min(255) as u8 };
            self.heads[hash] = self.next as u16;
            self.next += 1;
        }
        let mut candidate = self.heads[Self::hash(input, position)];
        let mut best = (0, 0);
        while candidate != u16::MAX {
            let previous = candidate as usize;
            if position - previous > WINDOW { break; }
            let mut length = 0;
            while length < 18 && at + length < input.len()
                && Self::byte(input, previous + length) == input[at + length] {
                length += 1;
            }
            if length >= 3 && length > best.1 { best = (position - previous, length); }
            if length == 18 { break; }
            let gap = self.previous[previous % self.previous.len()] as usize;
            candidate = match gap {
                0 => u16::MAX,
                255 => {
                    let hash = Self::hash(input, previous);
                    (position.saturating_sub(WINDOW)..=previous - 255).rev()
                        .find(|index| Self::hash(input, *index) == hash)
                        .map_or(u16::MAX, |index| index as u16)
                }
                _ => (previous - gap) as u16,
            };
        }
        best
    }
}
#[inline(never)]
fn compress(input: &[u8], mut output: impl Write, index: &mut MatchIndex) -> io::Result<()> {
    index.reset();
    let mut at = 0;
    while at < input.len() {
        let mut flags = 0u8;
        let mut group = [0u8; 16];
        let mut used = 0;
        for bit in 0..8 {
            if at == input.len() { break; }
            let (distance, length) = index.find(input, at);
            if length >= 3 {
                flags |= 1 << bit;
                let code = (((distance - 1) << 4) | (length - 3)) as u16;
                group[used..used + 2].copy_from_slice(&code.to_le_bytes());
                used += 2;
                at += length;
            } else {
                group[used] = input[at];
                used += 1;
                at += 1;
            }
        }
        output.write_all(&[flags])?;
        output.write_all(&group[..used])?;
    }
    Ok(())
}
struct Counter(usize);
impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
/// Two passes reserve the compressed size, not the expanded JSON record size.
#[inline(never)]
pub fn pack(input: &[u8]) -> HostResult<Vec<u8>> {
    if input.len() > MAX_RECORD_SIZE {
        return Err(HostError::new(
            "ESP32 runtime record exceeds expanded size budget",
        ));
    }
    let mut index = MatchIndex::new(input.len())?;
    let mut count = Counter(0);
    compress(input, &mut count, &mut index).map_err(|e| HostError::new(e.to_string()))?;
    let compressed = count.0 < input.len();
    let size = 3 + if compressed { count.0 } else { input.len() };
    let mut output = Vec::new();
    output.try_reserve_exact(size).map_err(|_| allocation(size))?;
    output.push(if compressed { 2 } else { 0 });
    output.extend_from_slice(&(input.len() as u16).to_le_bytes());
    if compressed { compress(input, &mut output, &mut index).map_err(|e| HostError::new(e.to_string()))?; }
    else { output.extend_from_slice(input); }
    Ok(output)
}
pub fn expanded_size(input: &[u8]) -> HostResult<usize> {
    if input.len() < 3 || input[0] > 2 {
        return Err(invalid());
    }
    let length = u16::from_le_bytes([input[1], input[2]]) as usize;
    if length > MAX_RECORD_SIZE {
        return Err(invalid());
    }
    Ok(length)
}
pub fn unpack(input: &[u8]) -> HostResult<Vec<u8>> {
    unpack_range(input, 0, usize::MAX)
}
/// Decodes a requested slice using at most one fixed LZSS window plus the
/// returned bytes, rather than allocating the whole expanded record for a page.
/// Validate the entire encoding, including corruption after the requested slice.
#[inline(never)]
pub fn unpack_range(input: &[u8], offset: u64, requested: usize) -> HostResult<Vec<u8>> {
    let length = expanded_size(input)?;
    let start = usize::try_from(offset).unwrap_or(length).min(length);
    let count = requested.min(length - start);
    let whole = start == 0 && count == length;
    let mut output = Vec::new();
    output.try_reserve_exact(count).map_err(|_| allocation(count))?;
    if input[0] == 0 {
        if input.len() != length + 3 { return Err(invalid()); }
        output.extend_from_slice(&input[3 + start..3 + start + count]);
        return Ok(output);
    }
    let dictionary = if input[0] == 2 { DICTIONARY } else { &[] };
    let window_size = if whole { 0 } else { WINDOW.min(length) };
    let mut history = Vec::new();
    history.try_reserve_exact(window_size).map_err(|_| allocation(window_size))?;
    history.resize(window_size, 0);
    let (mut at, mut produced) = (3, 0);
    while produced < length {
        let flags = *input.get(at).ok_or_else(invalid)?;
        at += 1;
        for bit in 0..8 {
            if produced == length { break; }
            let (distance, token_count, literal) = if flags & (1 << bit) == 0 {
                let byte = *input.get(at).ok_or_else(invalid)?;
                at += 1;
                (0, 1, byte)
            } else {
                let bytes = input.get(at..at + 2).ok_or_else(invalid)?;
                at += 2;
                let code = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
                ((code >> 4) + 1, (code & 15) + 3, 0)
            };
            if distance > dictionary.len() + produced || token_count > length - produced { return Err(invalid()); }
            for _ in 0..token_count {
                let byte = if distance == 0 { literal }
                    else if distance > produced { dictionary[dictionary.len() + produced - distance] }
                    else if whole { output[produced - distance] }
                    else { history[(produced - distance) % window_size] };
                if !whole { history[produced % window_size] = byte; }
                if produced >= start && produced - start < count { output.push(byte); }
                produced += 1;
            }
        }
    }
    if at != input.len() { return Err(invalid()); }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexed_matching_preserves_all_legacy_candidates_across_window_wraps() {
        assert!(DICTIONARY.len() + MAX_RECORD_SIZE < u16::MAX as usize);
        let mut random = 123456789u32;
        let mut binary = Vec::new();
        for _ in 0..MAX_RECORD_SIZE {
            random ^= random << 13; random ^= random >> 17; random ^= random << 5;
            binary.push(random as u8);
        }
        for bytes in [binary, vec![b'x'; MAX_RECORD_SIZE], DICTIONARY.repeat(12),
            (0..MAX_RECORD_SIZE).map(|i| (i % 127) as u8).collect()] {
            let mut index = MatchIndex::new(bytes.len()).unwrap();
            for at in 0..bytes.len() {
                assert_eq!(index.find(&bytes, at), match_at(&bytes, at), "offset {at}");
            }
            index.reset();
            for at in (0..bytes.len()).step_by(17) {
                assert_eq!(index.find(&bytes, at), match_at(&bytes, at), "stride offset {at}");
            }
        }
    }
    #[test]
    fn round_trip_empty_repetitive_and_binary_records() {
        let mut random = 123456789u32;
        for length in [0, 1, 2, 3, 7, 8, 17, 18, 31, 4096, MAX_RECORD_SIZE] {
            let bytes: Vec<_> = (0..length)
                .map(|_| {
                    random ^= random << 13;
                    random ^= random >> 17;
                    random ^= random << 5;
                    random as u8
                })
                .collect();
            assert_eq!(unpack(&pack(&bytes).unwrap()).unwrap(), bytes);
            let bytes = vec![b'x'; length];
            assert_eq!(unpack(&pack(&bytes).unwrap()).unwrap(), bytes);
        }
    }
    #[test]
    fn bounded_ranges_match_plain_and_compressed_records_across_window_wraps() {
        let mut random = 123456789u32;
        let binary: Vec<u8> = (0..MAX_RECORD_SIZE).map(|_| {
            random ^= random << 13;
            random ^= random >> 17;
            random ^= random << 5;
            random as u8
        }).collect();
        let repeat: Vec<u8> = (0..MAX_RECORD_SIZE).map(|i| (i % 127) as u8).collect();
        for bytes in [Vec::new(), binary, repeat] {
            let packed = pack(&bytes).unwrap();
            for (offset, count) in [(0, 1), (7, 19), (4090, 32), (8190, 64),
                (16380, usize::MAX), (u64::MAX, usize::MAX), (0, usize::MAX), (12, 0)] {
                let start = usize::try_from(offset).unwrap_or(bytes.len()).min(bytes.len());
                let end = start.saturating_add(count).min(bytes.len());
                assert_eq!(unpack_range(&packed, offset, count).unwrap(), bytes[start..end]);
            }
        }
    }
    #[test]
    fn bounded_ranges_reject_truncated_and_trailing_data_beyond_the_slice() {
        let packed = pack(&vec![b'x'; MAX_RECORD_SIZE]).unwrap();
        for end in 0..packed.len() { assert!(unpack_range(&packed[..end], 0, 1).is_err()); }
        let mut extra = packed;
        extra.push(0);
        assert!(unpack_range(&extra, 0, 1).is_err());
        assert!(unpack_range(&[0, 2, 0, 1], 0, 1).is_err());
    }
    #[test]
    fn dictionary_records_preserve_legacy_formats_and_bounded_reads() {
        // Independent legacy wire examples: raw bytes and an overlapping LZSS run.
        assert_eq!(unpack(&[0, 3, 0, 1, 2, 3]).unwrap(), [1, 2, 3]);
        assert_eq!(unpack(&[1, 18, 0, 2, b'x', 14, 0]).unwrap(), vec![b'x'; 18]);
        let bytes = DICTIONARY.repeat(12);
        let packed = pack(&bytes).unwrap();
        assert_eq!(packed[0], 2);
        assert_eq!(unpack(&packed).unwrap(), bytes);
        for (start, count) in [(0, 31), (800, 100), (4090, 40), (8190, 250), (0, 0)] {
            assert_eq!(unpack_range(&packed, start as u64, count).unwrap(), bytes[start..start + count]);
        }
        for end in 0..packed.len() {
            assert!(unpack_range(&packed[..end], 0, 1).is_err());
        }
        let mut extra = packed;
        extra.push(0);
        assert!(unpack_range(&extra, 0, 1).is_err());
        // A dictionary reference outside its initial history, and an overlong run.
        assert!(unpack(&[2, 3, 0, 1, 255, 255]).is_err());
        assert!(unpack_range(&[2, 3, 0, 1, 255, 255], 0, 1).is_err());
        assert!(unpack(&[2, 2, 0, 1, 0, 0]).is_err());
    }
    #[test]
    fn persisted_dictionary_format_has_a_fixed_wire_example() {
        // Changing dictionary layout must fail this test rather than silently
        // reinterpret the same format number on an already provisioned device.
        let hash = DICTIONARY.iter().fold(0xcbf29ce484222325u64,
            |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3));
        assert_eq!(hash, 0x56d3c2a673a5cff4u64);
        let bytes = br#""nodeId":"#;
        let packed = pack(bytes).unwrap();
        assert_eq!(packed, vec![2, 9, 0, 1, 246, 49]);
        assert_eq!(unpack(&packed).unwrap(), bytes);
    }
    #[test]
    fn rejects_corruption_truncation_and_expansion_bombs() {
        assert!(pack(&vec![0; MAX_RECORD_SIZE + 1]).is_err());
        for bytes in [
            vec![],
            vec![3, 0, 0],
            vec![0, 1, 0],
            vec![1, 255, 255],
            vec![1, 3, 0, 1, 0, 0],
            vec![0, 0, 0, 1],
        ] {
            assert!(unpack(&bytes).is_err());
        }
        let packed = pack(&vec![b'x'; 1024]).unwrap();
        for end in 0..packed.len() {
            assert!(unpack(&packed[..end]).is_err());
        }
    }
}
