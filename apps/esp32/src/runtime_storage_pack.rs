//! Small bounded LZSS records for the ESP32 virtual storage cache.
//! Records and range-read history are bounded; no whole-database decompression.
use operit_host_api::{HostError, HostResult};
use std::io::{self, Write};

pub const MAX_RECORD_SIZE: usize = 16 * 1024;
const WINDOW: usize = 4096;

fn invalid() -> HostError {
    HostError::new("Invalid packed runtime storage record")
}
fn allocation(length: usize) -> HostError {
    HostError::new(format!(
        "Packed runtime storage allocation failed ({length} bytes)"
    ))
}

fn match_at(input: &[u8], at: usize) -> (usize, usize) {
    let mut best = (0, 0);
    for previous in (at.saturating_sub(WINDOW)..at).rev() {
        if input[previous] != input[at] {
            continue;
        }
        let mut length = 1;
        while length < 18
            && at + length < input.len()
            && input[previous + length] == input[at + length]
        {
            length += 1;
        }
        if length >= 3 && length > best.1 {
            best = (at - previous, length);
        }
        if length == 18 {
            break;
        }
    }
    best
}
fn compress(input: &[u8], mut output: impl Write) -> io::Result<()> {
    let mut at = 0;
    while at < input.len() {
        let mut flags = 0u8;
        let mut group = [0u8; 16];
        let mut used = 0;
        for bit in 0..8 {
            if at == input.len() {
                break;
            }
            let (distance, length) = match_at(input, at);
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
pub fn pack(input: &[u8]) -> HostResult<Vec<u8>> {
    if input.len() > MAX_RECORD_SIZE {
        return Err(HostError::new(
            "ESP32 runtime record exceeds expanded size budget",
        ));
    }
    let mut count = Counter(0);
    compress(input, &mut count).map_err(|e| HostError::new(e.to_string()))?;
    let compressed = count.0 < input.len();
    let size = 3 + if compressed { count.0 } else { input.len() };
    let mut output = Vec::new();
    output
        .try_reserve_exact(size)
        .map_err(|_| allocation(size))?;
    output.push(u8::from(compressed));
    output.extend_from_slice(&(input.len() as u16).to_le_bytes());
    if compressed {
        compress(input, &mut output).map_err(|e| HostError::new(e.to_string()))?;
    } else {
        output.extend_from_slice(input);
    }
    Ok(output)
}
pub fn expanded_size(input: &[u8]) -> HostResult<usize> {
    if input.len() < 3 || input[0] > 1 {
        return Err(invalid());
    }
    let length = u16::from_le_bytes([input[1], input[2]]) as usize;
    if length > MAX_RECORD_SIZE {
        return Err(invalid());
    }
    Ok(length)
}
pub fn unpack(input: &[u8]) -> HostResult<Vec<u8>> {
    let length = expanded_size(input)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| allocation(length))?;
    if input[0] == 0 {
        if input.len() != length + 3 {
            return Err(invalid());
        }
        output.extend_from_slice(&input[3..]);
        return Ok(output);
    }
    let mut at = 3;
    while output.len() < length {
        let flags = *input.get(at).ok_or_else(invalid)?;
        at += 1;
        for bit in 0..8 {
            if output.len() == length {
                break;
            }
            if flags & (1 << bit) == 0 {
                output.push(*input.get(at).ok_or_else(invalid)?);
                at += 1;
            } else {
                let bytes = input.get(at..at + 2).ok_or_else(invalid)?;
                at += 2;
                let code = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
                let distance = (code >> 4) + 1;
                let count = (code & 15) + 3;
                if distance > output.len() || count > length - output.len() {
                    return Err(invalid());
                }
                for _ in 0..count {
                    output.push(output[output.len() - distance]);
                }
            }
        }
    }
    if at != input.len() {
        return Err(invalid());
    }
    Ok(output)
}
/// Decodes a requested slice using at most one fixed LZSS window plus the
/// returned bytes, rather than allocating the whole expanded record for a page.
/// Validate the entire encoding, including corruption after the requested slice.
pub fn unpack_range(input: &[u8], offset: u64, requested: usize) -> HostResult<Vec<u8>> {
    let length = expanded_size(input)?;
    let start = usize::try_from(offset).unwrap_or(length).min(length);
    let count = requested.min(length - start);
    if start == 0 && count == length { return unpack(input); }
    let mut output = Vec::new();
    output.try_reserve_exact(count).map_err(|_| allocation(count))?;
    if input[0] == 0 {
        if input.len() != length + 3 { return Err(invalid()); }
        output.extend_from_slice(&input[3 + start..3 + start + count]);
        return Ok(output);
    }
    let window_size = WINDOW.min(length);
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
            if distance > produced || token_count > length - produced { return Err(invalid()); }
            for _ in 0..token_count {
                let byte = if distance == 0 { literal } else { history[(produced - distance) % window_size] };
                history[produced % window_size] = byte;
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
    fn rejects_corruption_truncation_and_expansion_bombs() {
        assert!(pack(&vec![0; MAX_RECORD_SIZE + 1]).is_err());
        for bytes in [
            vec![],
            vec![2, 0, 0],
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
