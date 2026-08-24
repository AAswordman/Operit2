#![allow(non_snake_case)]

use std::path::PathBuf;

use operit_host_api::FileSystemHost;
use operit_host_api::HostManager::HostManager;
use operit_model::SttConfig::{SttConfig, SttProviderType, SttRecognitionResult};
use operit_providers::stt::SpeechToTextServiceFactory::SpeechToTextServiceFactory;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::RuntimeStorageLayout::RUNTIME_CLEAN_ON_EXIT_DIR_PATH;
use uuid::Uuid;

use crate::data::preferences::SttConfigManager::SttConfigManager;
use crate::services::LocalProviderService::LocalProviderService;

const LOCAL_STT_PCM_FORMAT: u16 = 1;
const LOCAL_STT_CHANNEL_COUNT: u16 = 1;
const LOCAL_STT_BITS_PER_SAMPLE: u16 = 16;

#[derive(Clone, Debug)]
struct LocalPcmWaveInspection {
    sampleRate: u32,
    channelCount: u16,
    bitsPerSample: u16,
    dataBytes: usize,
    durationMs: u64,
    peakSample: u32,
    rmsSample: f64,
}

#[derive(Clone)]
/// Transcribes audio through the configured local or remote STT provider.
pub struct SttRecognitionService {
    configManager: SttConfigManager,
    context: HostManager,
    temporaryAudioDirectory: PathBuf,
}

impl SttRecognitionService {
    /// Creates an STT recognition service from application host context.
    pub fn getInstance(context: &HostManager) -> Self {
        Self::newWithContext(RuntimeStorePaths::default(), context.clone())
    }

    /// Creates an STT recognition service with explicit runtime paths and host context.
    pub fn newWithContext(paths: RuntimeStorePaths, context: HostManager) -> Self {
        let temporaryAudioDirectory = paths.runtime_storage_path(RUNTIME_CLEAN_ON_EXIT_DIR_PATH);
        Self {
            configManager: SttConfigManager::new(paths),
            context,
            temporaryAudioDirectory,
        }
    }

    /// Transcribes one in-memory audio payload with the selected STT configuration.
    pub fn transcribeCurrent(
        &self,
        audioBytes: Vec<u8>,
        fileName: String,
        contentType: String,
        language: Option<String>,
    ) -> Result<SttRecognitionResult, String> {
        let config = self.configManager.getCurrentSttConfig()?;
        self.transcribeWithConfig(config, audioBytes, fileName, contentType, language)
    }

    /// Transcribes one in-memory audio payload with an explicit STT configuration id.
    pub fn transcribeWithConfigId(
        &self,
        configId: String,
        audioBytes: Vec<u8>,
        fileName: String,
        contentType: String,
        language: Option<String>,
    ) -> Result<SttRecognitionResult, String> {
        let config = self.configManager.getSttConfig(&configId)?;
        self.transcribeWithConfig(config, audioBytes, fileName, contentType, language)
    }

    /// Routes one STT request to its configured provider implementation.
    fn transcribeWithConfig(
        &self,
        config: SttConfig,
        audioBytes: Vec<u8>,
        fileName: String,
        contentType: String,
        language: Option<String>,
    ) -> Result<SttRecognitionResult, String> {
        if audioBytes.is_empty() {
            return Err("STT audio payload is empty".to_string());
        }
        let providerType = SttProviderType::normalize(&config.providerType);
        if providerType == SttProviderType::LOCAL_MODEL {
            let response = self.transcribeLocal(config.model, audioBytes, contentType, language)?;
            return Ok(SttRecognitionResult {
                text: response.text,
            });
        }
        SpeechToTextServiceFactory::createService(&config)?.transcribe(
            &config,
            &audioBytes,
            &fileName,
            &contentType,
            language.as_deref(),
        )
    }

    /// Runs local STT from one runtime-owned temporary WAV file and removes it afterward.
    fn transcribeLocal(
        &self,
        modelBinding: String,
        audioBytes: Vec<u8>,
        contentType: String,
        language: Option<String>,
    ) -> Result<operit_local_models::LocalInference::LocalSttResponse, String> {
        if contentType.trim() != "audio/wav" {
            return Err("LOCAL_MODEL STT requires audio/wav input".to_string());
        }
        let model = LocalProviderService::parseModelBinding(modelBinding)?;
        let localProvider = LocalProviderService::getInstance(&self.context)?;
        self.transcribeLocalWithTemporaryInput(model, localProvider, audioBytes, language)
    }

    /// Runs local STT with a host-owned temporary audio file.
    fn transcribeLocalWithTemporaryInput(
        &self,
        model: operit_local_models::LocalInference::LocalModelSelection,
        localProvider: LocalProviderService,
        audioBytes: Vec<u8>,
        language: Option<String>,
    ) -> Result<operit_local_models::LocalInference::LocalSttResponse, String> {
        let audioInspection = inspectLocalPcmWave(&audioBytes)?;
        if audioInspection.peakSample == 0 {
            return Err(format!(
                "LOCAL_MODEL STT received silent WAV input ({})",
                audioInspection.describe()
            ));
        }
        let inputDescription = audioInspection.describe();
        let fileSystemHost = self
            .context
            .fileSystemHost
            .as_ref()
            .ok_or_else(|| "FileSystemHost is required for LOCAL_MODEL STT".to_string())?;
        fileSystemHost
            .makeDirectory(&self.temporaryAudioDirectory.to_string_lossy(), true)
            .map_err(errorString)?;
        let audioPath = self
            .temporaryAudioDirectory
            .join(format!("local-stt-{}.wav", Uuid::new_v4()));
        let audioPath = audioPath.to_string_lossy().to_string();
        fileSystemHost
            .writeFileBytes(&audioPath, &audioBytes)
            .map_err(errorString)?;
        let recognition = localProvider.transcribeAudio(model, audioPath.clone(), language);
        let cleanup = fileSystemHost
            .deleteFile(&audioPath, false)
            .map_err(errorString);
        match (recognition, cleanup) {
            (Ok(response), Ok(())) => Ok(response),
            (Err(error), Ok(())) => Err(format!("{error}; input={inputDescription}")),
            (Ok(_), Err(error)) => Err(format!(
                "failed to remove local STT input: {error}; input={inputDescription}"
            )),
            (Err(recognitionError), Err(cleanupError)) => Err(format!(
                "{recognitionError}; failed to remove local STT input: {cleanupError}; input={inputDescription}"
            )),
        }
    }
}

impl LocalPcmWaveInspection {
    /// Formats one human-readable summary of the inspected WAV payload.
    fn describe(&self) -> String {
        format!(
            "wav={}ms, {}Hz, {}ch, {}bit, data={} bytes, peak={} ({:.1} dBFS), rms={:.1} dBFS",
            self.durationMs,
            self.sampleRate,
            self.channelCount,
            self.bitsPerSample,
            self.dataBytes,
            self.peakSample,
            sampleDbfs(self.peakSample as f64),
            sampleDbfs(self.rmsSample)
        )
    }
}

/// Inspects one local STT WAV payload and verifies the exact PCM contract.
fn inspectLocalPcmWave(audioBytes: &[u8]) -> Result<LocalPcmWaveInspection, String> {
    if audioBytes.len() < 12 {
        return Err(format!(
            "LOCAL_MODEL STT WAV is too small: {} bytes",
            audioBytes.len()
        ));
    }
    if &audioBytes[0..4] != b"RIFF" {
        return Err("LOCAL_MODEL STT WAV must start with RIFF".to_string());
    }
    if &audioBytes[8..12] != b"WAVE" {
        return Err("LOCAL_MODEL STT WAV must use WAVE format".to_string());
    }

    let mut cursor = 12usize;
    let mut fmtChunk = None;
    let mut dataChunk = None;
    while cursor + 8 <= audioBytes.len() {
        let chunkId = &audioBytes[cursor..cursor + 4];
        let chunkSize = readLeU32(audioBytes, cursor + 4)? as usize;
        let chunkDataStart = cursor + 8;
        let chunkDataEnd = chunkDataStart
            .checked_add(chunkSize)
            .ok_or_else(|| "LOCAL_MODEL STT WAV chunk size overflow".to_string())?;
        if chunkDataEnd > audioBytes.len() {
            return Err(format!(
                "LOCAL_MODEL STT WAV chunk exceeds file size: chunk={} bytes, file={} bytes",
                chunkSize,
                audioBytes.len()
            ));
        }
        if chunkId == b"fmt " {
            fmtChunk = Some(&audioBytes[chunkDataStart..chunkDataEnd]);
        }
        if chunkId == b"data" {
            dataChunk = Some(&audioBytes[chunkDataStart..chunkDataEnd]);
        }
        cursor = chunkDataEnd + (chunkSize % 2);
    }

    let fmtChunk =
        fmtChunk.ok_or_else(|| "LOCAL_MODEL STT WAV is missing fmt chunk".to_string())?;
    if fmtChunk.len() < 16 {
        return Err(format!(
            "LOCAL_MODEL STT WAV fmt chunk is too small: {} bytes",
            fmtChunk.len()
        ));
    }
    let audioFormat = readLeU16(fmtChunk, 0)?;
    let channelCount = readLeU16(fmtChunk, 2)?;
    let sampleRate = readLeU32(fmtChunk, 4)?;
    let byteRate = readLeU32(fmtChunk, 8)?;
    let bitsPerSample = readLeU16(fmtChunk, 14)?;
    if audioFormat != LOCAL_STT_PCM_FORMAT {
        return Err(format!(
            "LOCAL_MODEL STT WAV must be PCM format {}, got {}",
            LOCAL_STT_PCM_FORMAT, audioFormat
        ));
    }
    if channelCount != LOCAL_STT_CHANNEL_COUNT {
        return Err(format!(
            "LOCAL_MODEL STT WAV must be mono, got {} channels",
            channelCount
        ));
    }
    if bitsPerSample != LOCAL_STT_BITS_PER_SAMPLE {
        return Err(format!(
            "LOCAL_MODEL STT WAV must be {}-bit PCM, got {}",
            LOCAL_STT_BITS_PER_SAMPLE, bitsPerSample
        ));
    }
    if sampleRate == 0 || byteRate == 0 {
        return Err(format!(
            "LOCAL_MODEL STT WAV has invalid rate metadata: sampleRate={}, byteRate={}",
            sampleRate, byteRate
        ));
    }

    let dataChunk =
        dataChunk.ok_or_else(|| "LOCAL_MODEL STT WAV is missing data chunk".to_string())?;
    if dataChunk.is_empty() {
        return Err("LOCAL_MODEL STT WAV data chunk is empty".to_string());
    }
    if dataChunk.len() % 2 != 0 {
        return Err(format!(
            "LOCAL_MODEL STT WAV PCM16 data has an incomplete sample: {} bytes",
            dataChunk.len()
        ));
    }

    let mut peakSample = 0u32;
    let mut sumSquares = 0f64;
    let mut sampleCount = 0usize;
    for sampleBytes in dataChunk.chunks_exact(2) {
        let sample = i16::from_le_bytes([sampleBytes[0], sampleBytes[1]]);
        let magnitude = (sample as i32).abs() as u32;
        peakSample = peakSample.max(magnitude);
        let sampleValue = sample as f64;
        sumSquares += sampleValue * sampleValue;
        sampleCount += 1;
    }
    let rmsSample = (sumSquares / sampleCount as f64).sqrt();
    let durationMs = dataChunk.len() as u64 * 1000 / byteRate as u64;
    Ok(LocalPcmWaveInspection {
        sampleRate,
        channelCount,
        bitsPerSample,
        dataBytes: dataChunk.len(),
        durationMs,
        peakSample,
        rmsSample,
    })
}

/// Reads one little-endian u16 from a byte slice.
fn readLeU16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let end = offset + 2;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| "LOCAL_MODEL STT WAV numeric field is truncated".to_string())?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

/// Reads one little-endian u32 from a byte slice.
fn readLeU32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let end = offset + 4;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| "LOCAL_MODEL STT WAV numeric field is truncated".to_string())?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Converts one PCM amplitude into dBFS.
fn sampleDbfs(sample: f64) -> f64 {
    if sample <= 0.0 {
        return f64::NEG_INFINITY;
    }
    20.0 * (sample / 32768.0).log10()
}

/// Converts one displayable I/O error into a string.
fn errorString(error: impl std::fmt::Display) -> String {
    error.to_string()
}
