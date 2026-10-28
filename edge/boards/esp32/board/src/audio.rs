//! Board audio I/O and hardware signal conversion; no container or speech protocol parsing.
//! INMP441: BCLK27, WS22, DIN35, L/R=GND. Speaker: onboard DAC GPIO26.
#[derive(Default)]
pub struct MicrophonePcm {
    previous: i32,
    filtered: i32,
}
impl MicrophonePcm {
    /// INMP441's signed 24 bits occupy the high bits of each left 32-bit slot.
    /// Remove DC, apply 4x gain, and saturate before emitting little-endian PCM16.
    pub fn sample(&mut self, left: i32) -> i16 {
        self.sample16((left >> 16) as i16)
    }
    /// The native producer retains exactly the high 16 bits used above.
    pub fn sample16(&mut self, left: i16) -> i16 {
        let input = left as i32;
        let filtered = input - self.previous + self.filtered * 127 / 128;
        self.previous = input;
        self.filtered = filtered;
        (filtered * 4).clamp(i16::MIN as i32, i16::MAX as i32) as i16
    }
}
/// Quarter-volume signed PCM16 to unsigned 8-bit DAC, with silence at midscale.
pub fn dac_sample(sample: i16) -> u8 {
    ((sample as i32 >> 10) + 128) as u8
}

#[cfg(target_os = "espidf")]
pub mod native {
    use std::ffi::c_void;
    unsafe extern "C" {
        fn operit_audio_mic_open(handle: *mut *mut c_void) -> i32;
        fn operit_audio_mic_read(
            handle: *mut c_void,
            buffer: *mut c_void,
            length: usize,
            read: *mut usize,
        ) -> i32;
        fn operit_audio_mic_close(handle: *mut c_void);
        fn operit_audio_speaker_open(rate: u32, handle: *mut *mut c_void) -> i32;
        fn operit_audio_speaker_write(
            handle: *mut c_void,
            buffer: *const u8,
            length: usize,
            written: *mut usize,
        ) -> i32;
        fn operit_audio_speaker_close(handle: *mut c_void);
    }
    fn check(code: i32) -> Result<(), String> {
        if code == 0 {
            Ok(())
        } else if code == -20000 {
            Err("Microphone PCM queue overflow".into())
        } else if code <= -10000 {
            Err(format!(
                "Microphone RX overrun (max read gap {} ms)",
                -10000 - code
            ))
        } else {
            Err(format!("Audio driver error {code}"))
        }
    }
    #[repr(C)]
    pub struct CaptureBlock {
        pub nonzero: u32,
        pub peak: u32,
        pub rightPeak: u32,
        pub samples: [i16; 256],
    }
    const _: () = assert!(std::mem::size_of::<CaptureBlock>() == 524);
    pub struct Microphone(*mut c_void);
    // Each handle is exclusively owned by one scheduler task; no concurrent calls.
    unsafe impl Send for Microphone {}
    impl Microphone {
        pub fn open() -> Result<Self, String> {
            let mut handle = std::ptr::null_mut();
            check(unsafe { operit_audio_mic_open(&mut handle) })?;
            Ok(Self(handle))
        }
        pub fn read(&mut self) -> Result<CaptureBlock, String> {
            let mut block = CaptureBlock {
                nonzero: 0,
                peak: 0,
                rightPeak: 0,
                samples: [0; 256],
            };
            let mut read = 0;
            check(unsafe {
                operit_audio_mic_read(
                    self.0,
                    (&mut block as *mut CaptureBlock).cast(),
                    std::mem::size_of::<CaptureBlock>(),
                    &mut read,
                )
            })?;
            if read != std::mem::size_of::<CaptureBlock>() {
                return Err("Invalid microphone PCM block".into());
            }
            Ok(block)
        }
    }

    impl Drop for Microphone {
        fn drop(&mut self) {
            unsafe { operit_audio_mic_close(self.0) }
        }
    }
    pub struct Speaker(*mut c_void);
    unsafe impl Send for Speaker {}
    impl Speaker {
        pub fn open(rate: u32) -> Result<Self, String> {
            let mut handle = std::ptr::null_mut();
            check(unsafe { operit_audio_speaker_open(rate, &mut handle) })?;
            Ok(Self(handle))
        }
        pub fn write(&mut self, mut bytes: &[u8]) -> Result<(), String> {
            while !bytes.is_empty() {
                let mut written = 0;
                check(unsafe {
                    operit_audio_speaker_write(self.0, bytes.as_ptr(), bytes.len(), &mut written)
                })?;
                if written == 0 || written > bytes.len() {
                    return Err("Speaker made no progress".into());
                }
                bytes = &bytes[written..];
            }
            Ok(())
        }
        /// Convert mono PCM16 into this board's unsigned DAC format at bounded gain.
        pub fn write_pcm16(&mut self, samples: &[i16]) -> Result<(), String> {
            let mut output = [128u8; 256];
            for block in samples.chunks(output.len()) {
                for (target, &sample) in output.iter_mut().zip(block) {
                    *target = super::dac_sample(sample);
                }
                self.write(&output[..block.len()])?;
            }
            Ok(())
        }
        pub fn finish(&mut self) -> Result<(), String> {
            // Drain queued samples with midscale silence before releasing DMA.
            for _ in 0..8 {
                self.write(&[128; 256])?;
            }
            Ok(())
        }
    }
    impl Drop for Speaker {
        fn drop(&mut self) {
            unsafe { operit_audio_speaker_close(self.0) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_capture_preserves_pcm_across_arbitrary_low_bits() {
        let mut raw_filter = MicrophonePcm::default();
        let mut compact_filter = MicrophonePcm::default();
        let mut seed = 0x92f3ab14u32;
        for _ in 0..16000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let raw = seed as i32;
            assert_eq!(
                raw_filter.sample(raw),
                compact_filter.sample16((raw >> 16) as i16)
            );
        }
    }
    #[test]
    fn microphone_sign_dc_removal_and_saturation() {
        assert!(MicrophonePcm::default().sample(i32::MAX) > 0);
        assert!(MicrophonePcm::default().sample(i32::MIN) < 0);
        let mut filter = MicrophonePcm::default();
        for _ in 0..4096 {
            filter.sample(1 << 24);
        }
        assert!(filter.sample(1 << 24).abs() < 8);
        assert_eq!(dac_sample(0), 128);
        assert_eq!(dac_sample(i16::MIN), 96);
        assert_eq!(dac_sample(i16::MAX), 159);
    }
}
