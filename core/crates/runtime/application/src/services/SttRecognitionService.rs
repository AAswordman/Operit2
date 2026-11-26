#![allow(non_snake_case)]

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};


use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    HostManager::defaultHostRuntimeTaskSchedulerHost, HostRuntimeTaskSchedulerHost,
};
use operit_model::SttConfig::{SttConfig, SttProviderType, SttRecognitionResult};
use operit_providers::stt::SpeechToTextServiceFactory::SpeechToTextServiceFactory;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::RuntimeStorageLayout::RUNTIME_CLEAN_ON_EXIT_DIR_PATH;
use operit_util::stream::Stream::{CollectFuture, Stream};
use tokio::sync::mpsc;
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
        let filesystem = self.context.vfs()?;
        filesystem
            .makeDirectory(&self.temporaryAudioDirectory.to_string_lossy(), true)
            .map_err(errorString)?;
        let audioPath = self
            .temporaryAudioDirectory
            .join(format!("local-stt-{}.wav", Uuid::new_v4()));
        let audioPath = audioPath.to_string_lossy().to_string();
        filesystem
            .writeFileBytes(&audioPath, &audioBytes)
            .map_err(errorString)?;
        let recognition = localProvider.transcribeAudio(model, audioPath.clone(), language);
        let cleanup = filesystem
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

    /// Opens one live STT stream session bound to [configId], or to the selected
    /// configuration when it is `None`. Pushed audio must be 16 kHz mono PCM16.
    pub fn startStreamSession(
        &self,
        configId: Option<String>,
        language: Option<String>,
    ) -> Result<String, String> {
        let config = match configId {
            Some(configId) => self.configManager.getSttConfig(&configId)?,
            None => self.configManager.getCurrentSttConfig()?,
        };
        let incremental =
            SttProviderType::normalize(&config.providerType) == SttProviderType::LOCAL_MODEL;
        let (events, eventReceiver) = mpsc::unbounded_channel::<String>();
        let sessionId = Uuid::new_v4().to_string();
        let session = Arc::new(SttStreamSession {
            service: self.clone(),
            config,
            language,
            incremental,
            events,
            eventReceiver: Mutex::new(Some(eventReceiver)),
            state: Mutex::new(SttStreamState::default()),
            idle: Condvar::new(),
        });
        lockSttStreamSessions()?.insert(sessionId.clone(), session);
        Ok(sessionId)
    }

    /// Streams the transcript recognized so far in one live STT stream session.
    pub fn streamPartialText(&self, sessionId: String) -> SttPartialTextStream {
        SttPartialTextStream {
            receiver: SttStreamSession::resolve(&sessionId).ok().and_then(|session| {
                session
                    .eventReceiver
                    .lock()
                    .ok()
                    .and_then(|mut receiver| receiver.take())
            }),
        }
    }

    /// Appends 16 kHz mono PCM16 samples to one live STT stream session.
    pub fn pushStreamAudio(
        &self,
        sessionId: String,
        audio: Vec<u8>,
    ) -> Result<(), String> {
        SttStreamSession::resolve(&sessionId)?.push(audio)
    }

    /// Recognizes the remaining audio, returns the complete transcript and
    /// releases one live STT stream session.
    pub fn finishStreamSession(&self, sessionId: String) -> Result<String, String> {
        let session = SttStreamSession::resolve(&sessionId)?;
        let transcript = session.finish();
        lockSttStreamSessions()?.remove(&sessionId);
        transcript
    }

    /// Releases one live STT stream session without recognizing its audio.
    pub fn cancelStreamSession(&self, sessionId: String) -> Result<(), String> {
        let session = SttStreamSession::resolve(&sessionId)?;
        session.cancel();
        lockSttStreamSessions()?.remove(&sessionId);
        Ok(())
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

/// Sample rate of the PCM16 audio accepted by one stream session.
const STT_STREAM_SAMPLE_RATE: u32 = 16_000;

/// Bytes of one 20 ms analysis frame of mono PCM16.
const STT_STREAM_FRAME_BYTES: usize = STT_STREAM_SAMPLE_RATE as usize / 50 * 2;

/// Mean squared sample above which one frame counts as audible, about -39 dBFS.
const STT_STREAM_AUDIBLE_MEAN_SQUARE: i64 = 360 * 360;

/// Consecutive audible frames that open a phrase, 160 ms.
const STT_STREAM_SPEECH_FRAMES: u32 = 8;

/// Frames kept ahead of a phrase so its first syllables are not clipped, 240 ms.
const STT_STREAM_PRE_ROLL_FRAMES: usize = 12;

/// Frames held before a phrase opens: the pre-roll plus the speech opening it.
const STT_STREAM_HISTORY_FRAMES: usize =
    STT_STREAM_PRE_ROLL_FRAMES + STT_STREAM_SPEECH_FRAMES as usize;

/// Consecutive silent frames that close a phrase, 500 ms.
const STT_STREAM_PHRASE_SILENCE_FRAMES: usize = 25;

/// Silent frames kept behind a closed phrase for recognition context, 200 ms.
const STT_STREAM_PHRASE_TAIL_FRAMES: usize = 10;

/// Frames after which uninterrupted speech is cut at its quietest frame, 12 s.
const STT_STREAM_MAX_PHRASE_FRAMES: usize = 600;

/// Trailing frames searched for that cut, 1.5 s.
const STT_STREAM_CUT_SEARCH_FRAMES: usize = 75;

/// Streams the transcript recognized so far in one STT stream session.
pub struct SttPartialTextStream {
    receiver: Option<mpsc::UnboundedReceiver<String>>,
}

impl Stream for SttPartialTextStream {
    type Item = String;

    /// Delivers each updated transcript in recognition order until the session ends.
    fn collect<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(Self::Item),
    ) -> CollectFuture<'a> {
        Box::pin(async move {
            let Some(receiver) = self.receiver.as_mut() else {
                return;
            };
            while let Some(text) = receiver.recv().await {
                collector(text);
            }
        })
    }
}

/// Audio and recognition progress shared by the feeder, the worker and the finisher.
#[derive(Default)]
struct SttStreamState {
    segmenter: SttPhraseSegmenter,
    audio: Vec<u8>,
    phrases: VecDeque<Vec<u8>>,
    recognizing: bool,
    transcript: String,
    failure: Option<String>,
    finished: bool,
}

/// Owns one live microphone stream until it is finished or cancelled.
struct SttStreamSession {
    service: SttRecognitionService,
    config: SttConfig,
    language: Option<String>,
    /// Whether phrases are recognized while the stream is open. Remote providers
    /// bill per request, so their audio is recognized once when the stream ends.
    incremental: bool,
    events: mpsc::UnboundedSender<String>,
    eventReceiver: Mutex<Option<mpsc::UnboundedReceiver<String>>>,
    state: Mutex<SttStreamState>,
    idle: Condvar,
}

impl SttStreamSession {
    /// Resolves one live session by id.
    fn resolve(sessionId: &str) -> Result<Arc<Self>, String> {
        lockSttStreamSessions()?
            .get(sessionId)
            .cloned()
            .ok_or_else(|| "STT stream session does not exist or has finished".to_string())
    }

    /// Locks the session state.
    fn lockState(&self) -> Result<MutexGuard<'_, SttStreamState>, String> {
        self.state
            .lock()
            .map_err(|_| "STT stream session mutex poisoned".to_string())
    }

    /// Buffers pushed audio and queues every phrase that ended at a pause.
    fn push(self: &Arc<Self>, audio: Vec<u8>) -> Result<(), String> {
        if audio.is_empty() {
            return Ok(());
        }
        let phrases = {
            let mut state = self.lockState()?;
            if let Some(failure) = &state.failure {
                return Err(failure.clone());
            }
            if state.finished {
                return Err("STT stream session has already finished".to_string());
            }
            if self.incremental {
                state.segmenter.push(&audio)
            } else {
                state.audio.extend_from_slice(&audio);
                Vec::new()
            }
        };
        for phrase in phrases {
            self.enqueuePhrase(phrase)?;
        }
        Ok(())
    }

    /// Queues one closed phrase and keeps a single worker draining in order.
    fn enqueuePhrase(self: &Arc<Self>, phrase: Vec<u8>) -> Result<(), String> {
        {
            let mut state = self.lockState()?;
            state.phrases.push_back(phrase);
            if state.recognizing {
                return Ok(());
            }
            state.recognizing = true;
        }
        let session = Arc::clone(self);
        let scheduled = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeTask(
            "operit-stt-stream-phrase",
            Box::new(move || session.recognizeQueuedPhrases()),
        );
        if let Err(error) = scheduled {
            self.lockState()?.recognizing = false;
            return Err(errorString(error));
        }
        Ok(())
    }

    /// Recognizes queued phrases in arrival order until the queue is empty.
    fn recognizeQueuedPhrases(self: &Arc<Self>) {
        loop {
            let phrase = {
                let mut state = match self.lockState() {
                    Ok(state) => state,
                    Err(_) => break,
                };
                match state.phrases.pop_front() {
                    Some(phrase) => phrase,
                    None => {
                        state.recognizing = false;
                        self.idle.notify_all();
                        break;
                    }
                }
            };
            let skipped = match self.lockState() {
                Ok(state) => state.failure.is_some(),
                Err(_) => true,
            };
            let outcome = if skipped {
                Ok(())
            } else {
                self.recognizePhrase(&phrase)
            };
            if let Err(error) = outcome {
                if let Ok(mut state) = self.lockState() {
                    state.failure.get_or_insert(error);
                    state.phrases.clear();
                }
            }
        }
    }

    /// Recognizes one phrase, extends the transcript and publishes it.
    fn recognizePhrase(&self, phrase: &[u8]) -> Result<(), String> {
        let fileName = format!("operit-stt-stream-{}.wav", Uuid::new_v4());
        let response = self.service.transcribeWithConfig(
            self.config.clone(),
            buildStreamWave(phrase),
            fileName,
            "audio/wav".to_string(),
            self.language.clone(),
        )?;
        let transcript = {
            let mut state = self.lockState()?;
            appendPhraseText(&mut state.transcript, &response.text);
            state.transcript.clone()
        };
        // A dropped receiver means nobody is watching; finish still returns the transcript.
        let _ = self.events.send(transcript);
        Ok(())
    }

    /// Recognizes the phrase still in progress, waits for queued phrases and
    /// returns the complete transcript.
    fn finish(self: &Arc<Self>) -> Result<String, String> {
        {
            let mut state = self.lockState()?;
            if state.finished {
                return Err("STT stream session has already finished".to_string());
            }
            state.finished = true;
            let remainder = if self.incremental {
                state.segmenter.finish()
            } else {
                std::mem::take(&mut state.audio)
            };
            if !remainder.is_empty() {
                state.phrases.push_back(remainder);
            }
        }
        let mut state = self.lockState()?;
        while state.recognizing {
            state = self
                .idle
                .wait(state)
                .map_err(|_| "STT stream session mutex poisoned".to_string())?;
        }
        drop(state);
        self.recognizeQueuedPhrases();
        let state = self.lockState()?;
        if let Some(failure) = &state.failure {
            return Err(failure.clone());
        }
        Ok(state.transcript.clone())
    }

    /// Drops queued audio and stops recognizing without returning a transcript.
    fn cancel(&self) {
        if let Ok(mut state) = self.lockState() {
            state.finished = true;
            state.phrases.clear();
            state
                .failure
                .get_or_insert_with(|| "STT stream session was cancelled".to_string());
        }
    }
}

/// Stores live sessions because the proxy rebuilds service objects per request.
fn sttStreamSessionRegistry() -> &'static Mutex<BTreeMap<String, Arc<SttStreamSession>>> {
    static REGISTRY: OnceLock<Mutex<BTreeMap<String, Arc<SttStreamSession>>>> =
        OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Locks the stream session registry.
fn lockSttStreamSessions(
) -> Result<MutexGuard<'static, BTreeMap<String, Arc<SttStreamSession>>>, String> {
    sttStreamSessionRegistry()
        .lock()
        .map_err(|_| "STT stream session registry mutex poisoned".to_string())
}

/// Splits one PCM16 stream into phrases at pauses so that each phrase can be
/// recognized while the speaker is still talking.
#[derive(Default)]
struct SttPhraseSegmenter {
    leftover: Vec<u8>,
    history: VecDeque<(i64, Vec<u8>)>,
    phrase: Vec<u8>,
    energies: VecDeque<i64>,
    loud: u32,
    quiet: usize,
    open: bool,
}

impl SttPhraseSegmenter {
    /// Feeds PCM16 samples and returns every phrase that ended at a pause.
    fn push(&mut self, audio: &[u8]) -> Vec<Vec<u8>> {
        let mut phrases = Vec::new();
        let mut rest = audio;
        if !self.leftover.is_empty() {
            let take = (STT_STREAM_FRAME_BYTES - self.leftover.len()).min(rest.len());
            self.leftover.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.leftover.len() < STT_STREAM_FRAME_BYTES {
                return phrases;
            }
            let frame = std::mem::take(&mut self.leftover);
            if let Some(phrase) = self.pushFrame(frame) {
                phrases.push(phrase);
            }
        }
        let mut cursor = 0;
        while cursor + STT_STREAM_FRAME_BYTES <= rest.len() {
            let frame = rest[cursor..cursor + STT_STREAM_FRAME_BYTES].to_vec();
            if let Some(phrase) = self.pushFrame(frame) {
                phrases.push(phrase);
            }
            cursor += STT_STREAM_FRAME_BYTES;
        }
        self.leftover.extend_from_slice(&rest[cursor..]);
        phrases
    }

    /// Tracks one 20 ms frame and returns a phrase as soon as it ends.
    fn pushFrame(&mut self, frame: Vec<u8>) -> Option<Vec<u8>> {
        let meanSquare = frameMeanSquare(&frame);
        let audible = meanSquare > STT_STREAM_AUDIBLE_MEAN_SQUARE;
        if !self.open {
            self.loud = if audible { self.loud + 1 } else { 0 };
            self.history.push_back((meanSquare, frame));
            while self.history.len() > STT_STREAM_HISTORY_FRAMES {
                self.history.pop_front();
            }
            if self.loud < STT_STREAM_SPEECH_FRAMES {
                return None;
            }
            self.open = true;
            self.loud = 0;
            self.quiet = 0;
            for (energy, bytes) in std::mem::take(&mut self.history) {
                self.energies.push_back(energy);
                self.phrase.extend_from_slice(&bytes);
            }
            return None;
        }
        self.phrase.extend_from_slice(&frame);
        self.energies.push_back(meanSquare);
        if audible {
            self.quiet = 0;
        } else {
            self.quiet += 1;
        }
        if self.quiet >= STT_STREAM_PHRASE_SILENCE_FRAMES {
            let keepFrames =
                self.energies.len() - STT_STREAM_PHRASE_SILENCE_FRAMES + STT_STREAM_PHRASE_TAIL_FRAMES;
            let phrase = self.phrase[..keepFrames * STT_STREAM_FRAME_BYTES].to_vec();
            self.reset();
            return Some(phrase);
        }
        if self.energies.len() >= STT_STREAM_MAX_PHRASE_FRAMES {
            let searchStart = self.energies.len() - STT_STREAM_CUT_SEARCH_FRAMES;
            let cutFrame = (searchStart..self.energies.len())
                .min_by_key(|index| self.energies[*index])
                .unwrap_or(searchStart);
            let cutBytes = (cutFrame + 1) * STT_STREAM_FRAME_BYTES;
            let phrase = self.phrase[..cutBytes].to_vec();
            self.phrase.drain(..cutBytes);
            self.energies.drain(..=cutFrame);
            self.quiet = 0;
            return Some(phrase);
        }
        None
    }

    /// Forgets the open phrase so that the next speech starts a fresh one.
    fn reset(&mut self) {
        self.history.clear();
        self.phrase.clear();
        self.energies.clear();
        self.loud = 0;
        self.quiet = 0;
        self.open = false;
    }

    /// Returns the phrase still being spoken when the stream ends.
    fn finish(&mut self) -> Vec<u8> {
        let phrase = std::mem::take(&mut self.phrase);
        self.reset();
        phrase
    }
}

/// Returns the mean squared sample of one PCM16 frame.
fn frameMeanSquare(frame: &[u8]) -> i64 {
    let mut sum = 0i64;
    for sample in frame.chunks_exact(2) {
        let value = i16::from_le_bytes([sample[0], sample[1]]) as i64;
        sum += value * value;
    }
    sum / (frame.len() / 2).max(1) as i64
}

/// Builds one 16 kHz mono PCM16 WAV file around a stream phrase.
fn buildStreamWave(pcm: &[u8]) -> Vec<u8> {
    const HEADER_BYTES: usize = 44;
    const BLOCK_ALIGN: u16 = 2;
    const BYTE_RATE: u32 = STT_STREAM_SAMPLE_RATE * BLOCK_ALIGN as u32;
    let mut wave = Vec::with_capacity(HEADER_BYTES + pcm.len());
    wave.extend_from_slice(b"RIFF");
    wave.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    wave.extend_from_slice(b"WAVEfmt ");
    wave.extend_from_slice(&16u32.to_le_bytes());
    wave.extend_from_slice(&1u16.to_le_bytes());
    wave.extend_from_slice(&1u16.to_le_bytes());
    wave.extend_from_slice(&STT_STREAM_SAMPLE_RATE.to_le_bytes());
    wave.extend_from_slice(&BYTE_RATE.to_le_bytes());
    wave.extend_from_slice(&BLOCK_ALIGN.to_le_bytes());
    wave.extend_from_slice(&16u16.to_le_bytes());
    wave.extend_from_slice(b"data");
    wave.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    wave.extend_from_slice(pcm);
    wave
}

/// Appends one recognized phrase, spacing only between Latin words so that
/// Chinese phrases join as continuous text.
fn appendPhraseText(transcript: &mut String, phrase: &str) {
    let phrase = phrase.trim();
    if phrase.is_empty() {
        return;
    }
    if let (Some(last), Some(first)) = (transcript.chars().last(), phrase.chars().next()) {
        if last.is_ascii_alphanumeric() && first.is_ascii_alphanumeric() {
            transcript.push(' ');
        }
    }
    transcript.push_str(phrase);
}

#[cfg(test)]
mod stream_session_tests {
    use super::*;

    /// Builds frames of one constant amplitude, 20 ms each.
    fn pcm(frames: usize, amplitude: i16) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(frames * STT_STREAM_FRAME_BYTES);
        for _ in 0..frames * (STT_STREAM_FRAME_BYTES / 2) {
            bytes.extend_from_slice(&amplitude.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn segmenter_closes_a_phrase_after_a_pause() {
        let mut segmenter = SttPhraseSegmenter::default();
        assert!(segmenter.push(&pcm(100, 0)).is_empty());
        assert!(segmenter.push(&pcm(40, 1200)).is_empty());
        let phrases = segmenter.push(&pcm(25, 0));
        assert_eq!(phrases.len(), 1);
        assert_eq!(phrases[0].len(), (12 + 40 + 10) * STT_STREAM_FRAME_BYTES);
        assert!(segmenter.push(&pcm(20, 0)).is_empty());
        assert!(segmenter.push(&pcm(30, 1200)).is_empty());
        assert_eq!(
            segmenter.finish().len(),
            (12 + 30) * STT_STREAM_FRAME_BYTES
        );
        assert!(segmenter.finish().is_empty());
    }

    #[test]
    fn segmenter_joins_frames_split_across_pushes() {
        let mut segmenter = SttPhraseSegmenter::default();
        let mut audio = pcm(3, 0);
        audio.extend(pcm(20, 1200));
        audio.extend(pcm(25, 0));
        let split = STT_STREAM_FRAME_BYTES * 10 + 7;
        assert!(segmenter.push(&audio[..split]).is_empty());
        let phrases = segmenter.push(&audio[split..]);
        assert_eq!(phrases.len(), 1);
        assert_eq!(
            phrases[0].len(),
            (3 + 20 + 10) * STT_STREAM_FRAME_BYTES
        );
    }

    #[test]
    fn segmenter_cuts_long_speech_at_its_quietest_frame() {
        let mut segmenter = SttPhraseSegmenter::default();
        let mut leading = pcm(12, 0);
        leading.extend(pcm(
            STT_STREAM_MAX_PHRASE_FRAMES - 12 - 1,
            1200,
        ));
        assert!(segmenter.push(&leading).is_empty());
        let mut trailing = pcm(1, 400);
        trailing.extend(pcm(12, 1200));
        let phrases = segmenter.push(&trailing);
        assert_eq!(phrases.len(), 1);
        assert_eq!(
            phrases[0].len(),
            STT_STREAM_MAX_PHRASE_FRAMES * STT_STREAM_FRAME_BYTES
        );
        assert_eq!(
            segmenter.finish().len(),
            12 * STT_STREAM_FRAME_BYTES
        );
    }

    #[test]
    fn phrases_join_without_breaking_chinese_or_gluing_latin_words() {
        let mut transcript = String::new();
        appendPhraseText(&mut transcript, "  remind me to ");
        appendPhraseText(&mut transcript, "water the ferns");
        assert_eq!(transcript, "remind me to water the ferns");
        let mut chinese = String::new();
        appendPhraseText(&mut chinese, "帮我打开");
        appendPhraseText(&mut chinese, "浏览器");
        appendPhraseText(&mut chinese, "，然后搜索");
        assert_eq!(chinese, "帮我打开浏览器，然后搜索");
    }

    #[test]
    fn stream_wave_declares_16khz_mono_pcm16() {
        let wave = buildStreamWave(&pcm(2, 1200));
        assert_eq!(&wave[0..4], b"RIFF");
        assert_eq!(&wave[8..12], b"WAVE");
        assert_eq!(
            u16::from_le_bytes(wave[22..24].try_into().unwrap()),
            1
        );
        assert_eq!(
            u32::from_le_bytes(wave[24..28].try_into().unwrap()),
            16_000
        );
        assert_eq!(
            u32::from_le_bytes(wave[40..44].try_into().unwrap()),
            2 * STT_STREAM_FRAME_BYTES as u32
        );
    }
}
