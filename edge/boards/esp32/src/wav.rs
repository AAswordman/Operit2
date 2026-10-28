//! Streaming PCM16 WAV decoding for the device application.
//! Emits mono PCM samples without depending on board drivers or Link packets.
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Riff,
    Header,
    Format,
    Skip,
    Data,
    Padding,
}
/// Consumes WAV headers/metadata and samples across arbitrary transport boundaries.
/// Only a 16-byte header and one stereo frame are retained; audio is never accumulated.
pub struct WavDecoder {
    phase: Phase,
    header: [u8; 16],
    filled: usize,
    remaining: u32,
    pad: bool,
    format: Option<(u32, u16)>,
    frame: [u8; 4],
    frame_len: usize,
    samples: u64,
}
impl Default for WavDecoder {
    fn default() -> Self {
        Self {
            phase: Phase::Riff,
            header: [0; 16],
            filled: 0,
            remaining: 0,
            pad: false,
            format: None,
            frame: [0; 4],
            frame_len: 0,
            samples: 0,
        }
    }
}
impl WavDecoder {
    pub fn push(
        &mut self,
        mut bytes: &[u8],
        mut write: impl FnMut(u32, &[i16]) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut output = [0i16; 128];
        let mut used = 0;
        while !bytes.is_empty() {
            match self.phase {
                Phase::Riff | Phase::Header | Phase::Format => {
                    let target = match self.phase {
                        Phase::Riff => 12,
                        Phase::Header => 8,
                        _ => 16,
                    };
                    let count = (target - self.filled).min(bytes.len());
                    self.header[self.filled..self.filled + count].copy_from_slice(&bytes[..count]);
                    self.filled += count;
                    bytes = &bytes[count..];
                    if self.phase == Phase::Format {
                        self.remaining -= count as u32;
                    }
                    if self.filled != target {
                        continue;
                    }
                    self.filled = 0;
                    match self.phase {
                        Phase::Riff => {
                            if &self.header[..4] != b"RIFF" || &self.header[8..12] != b"WAVE" {
                                return Err("Expected PCM16 WAV audio".into());
                            }
                            self.phase = Phase::Header;
                        }
                        Phase::Header => {
                            self.remaining =
                                u32::from_le_bytes(self.header[4..8].try_into().unwrap());
                            self.pad = self.remaining & 1 != 0;
                            self.phase = match &self.header[..4] {
                                b"fmt " => {
                                    if self.remaining < 16 || self.format.is_some() {
                                        return Err("Invalid WAV format chunk".into());
                                    }
                                    Phase::Format
                                }
                                b"data" => {
                                    if self.format.is_none() {
                                        return Err("WAV format must precede samples".into());
                                    }
                                    Phase::Data
                                }
                                _ => Phase::Skip,
                            };
                        }
                        Phase::Format => {
                            let channels =
                                u16::from_le_bytes(self.header[2..4].try_into().unwrap());
                            let rate = u32::from_le_bytes(self.header[4..8].try_into().unwrap());
                            let align = u16::from_le_bytes(self.header[12..14].try_into().unwrap());
                            let bits = u16::from_le_bytes(self.header[14..16].try_into().unwrap());
                            if self.header[..2] != [1, 0]
                                || !matches!(channels, 1 | 2)
                                || bits != 16
                                || align != channels * 2
                                || !(8000..=48000).contains(&rate)
                            {
                                return Err(
                                    "Supported WAV format is 8-48 kHz mono/stereo PCM16".into()
                                );
                            }
                            self.format = Some((rate, channels));
                            self.phase = Phase::Skip;
                        }
                        _ => unreachable!(),
                    }
                }
                Phase::Skip => {
                    let count = bytes.len().min(self.remaining as usize);
                    bytes = &bytes[count..];
                    self.remaining -= count as u32;
                }
                Phase::Padding => {
                    bytes = &bytes[1..];
                    self.phase = Phase::Header;
                }
                Phase::Data => {
                    let (rate, channels) = self.format.unwrap();
                    let count = bytes.len().min(self.remaining as usize);
                    for &byte in &bytes[..count] {
                        self.frame[self.frame_len] = byte;
                        self.frame_len += 1;
                        if self.frame_len == channels as usize * 2 {
                            let left = i16::from_le_bytes([self.frame[0], self.frame[1]]) as i32;
                            let sample = if channels == 2 {
                                (left + i16::from_le_bytes([self.frame[2], self.frame[3]]) as i32)
                                    / 2
                            } else {
                                left
                            };
                            output[used] = sample as i16;
                            used += 1;
                            self.samples += 1;
                            self.frame_len = 0;
                            if used == output.len() {
                                write(rate, &output)?;
                                used = 0;
                            }
                        }
                    }
                    bytes = &bytes[count..];
                    // ffmpeg may use unknown data length for a non-seekable WAV stream.
                    if self.remaining != u32::MAX {
                        self.remaining -= count as u32;
                    }
                }
            }
            if matches!(self.phase, Phase::Skip | Phase::Data) && self.remaining == 0 {
                if self.frame_len != 0 {
                    return Err("WAV ends within a sample frame".into());
                }
                self.phase = if self.pad {
                    Phase::Padding
                } else {
                    Phase::Header
                };
            }
        }
        if used != 0 {
            write(self.format.unwrap().0, &output[..used])?;
        }
        Ok(())
    }
    pub fn finish(&self) -> Result<(), String> {
        if self.samples == 0
            || self.frame_len != 0
            || self.filled != 0
            || !(self.phase == Phase::Header
                || self.phase == Phase::Data && self.remaining == u32::MAX)
        {
            return Err("Truncated or empty WAV stream".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wav(channels: u16, samples: &[i16]) -> Vec<u8> {
        let mut bytes = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0".to_vec();
        bytes.extend(channels.to_le_bytes());
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend((32000 * channels as u32).to_le_bytes());
        bytes.extend((channels * 2).to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((samples.len() as u32 * 2).to_le_bytes());
        for s in samples {
            bytes.extend(s.to_le_bytes());
        }
        let len = bytes.len() as u32 - 8;
        bytes[4..8].copy_from_slice(&len.to_le_bytes());
        bytes
    }
    #[test]
    fn wav_every_split_preserves_samples_and_stereo_downmix() {
        let bytes = wav(2, &[-32768, 32767, 0, 0, 32767, 32767, -32768, -32768]);
        for split in 0..=bytes.len() {
            let mut decoder = WavDecoder::default();
            let mut result = Vec::new();
            for part in [&bytes[..split], &bytes[split..]] {
                decoder
                    .push(part, |rate, b| {
                        assert_eq!(rate, 16000);
                        result.extend_from_slice(b);
                        Ok(())
                    })
                    .unwrap();
            }
            decoder.finish().unwrap();
            assert_eq!(result, [0, 0, 32767, -32768]);
        }
    }
    #[test]
    fn one_byte_chunks_skip_odd_metadata_without_audio_buffering() {
        let mut bytes = wav(1, &[0, 32767, -32768]);
        bytes.splice(12..12, b"JUNK\x03\0\0\0abc\0".iter().copied());
        let mut decoder = WavDecoder::default();
        let mut output = Vec::new();
        for b in bytes {
            decoder
                .push(&[b], |_, b| {
                    output.extend_from_slice(b);
                    Ok(())
                })
                .unwrap();
        }
        decoder.finish().unwrap();
        assert_eq!(output, [0, 32767, -32768]);
    }
    #[test]
    fn rejects_compressed_truncated_or_unsupported_audio() {
        let mut bytes = wav(1, &[0, 1]);
        bytes[20] = 3;
        assert!(WavDecoder::default().push(&bytes, |_, _| Ok(())).is_err());
        let bytes = wav(1, &[0, 1]);
        let mut d = WavDecoder::default();
        d.push(&bytes[..bytes.len() - 1], |_, _| Ok(())).unwrap();
        assert!(d.finish().is_err());
        assert!(WavDecoder::default()
            .push(b"ID3 not wave", |_, _| Ok(()))
            .is_err());
    }
    #[test]
    fn long_audio_stream_keeps_fixed_buffers_and_supports_unknown_wav_length() {
        for unknown in [false, true] {
            let samples = 16000 * 65u32;
            let mut header = wav(1, &[0]);
            header.truncate(44);
            header[40..44]
                .copy_from_slice(&(if unknown { u32::MAX } else { samples * 2 }).to_le_bytes());
            let mut decoder = WavDecoder::default();
            let mut received = 0;
            decoder
                .push(&header, |_, _| panic!("header contains no samples"))
                .unwrap();
            let block = [0u8; 1024];
            let mut remaining = samples as usize * 2;
            while remaining > 0 {
                let n = remaining.min(block.len());
                decoder
                    .push(&block[..n], |rate, bytes| {
                        assert_eq!(rate, 16000);
                        assert!(bytes.len() <= 128);
                        assert!(bytes.iter().all(|&b| b == 0));
                        received += bytes.len();
                        Ok(())
                    })
                    .unwrap();
                remaining -= n;
            }
            decoder.finish().unwrap();
            assert_eq!(received, samples as usize);
            assert!(std::mem::size_of_val(&decoder) < 128);
        }
    }
}
