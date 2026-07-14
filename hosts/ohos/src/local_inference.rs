use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_float, c_int};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};
use operit_host_api::{
    HostError, HostResult, LocalInferenceHost, LocalSttInferenceHostRequest,
    LocalSttInferenceHostResponse, LocalTtsInferenceHostRequest, LocalTtsInferenceHostResponse,
};
use operit_local_models::LocalModelManifest::LocalModelDriver;

#[derive(Clone, Debug, Default)]
pub struct OhosLocalInferenceHost;

impl OhosLocalInferenceHost {
    /// Creates a stateless OHOS local inference host.
    pub fn new() -> Self {
        Self
    }
}

impl LocalInferenceHost for OhosLocalInferenceHost {
    /// Transcribes one WAV request with the installed OHOS Sherpa ONNX C API.
    fn transcribeLocalSpeech(
        &self,
        request: LocalSttInferenceHostRequest,
    ) -> HostResult<LocalSttInferenceHostResponse> {
        transcribeStreamingTransducer(request)
    }

    /// Synthesizes one WAV request with the installed OHOS Sherpa ONNX C API.
    fn synthesizeLocalSpeech(
        &self,
        request: LocalTtsInferenceHostRequest,
    ) -> HostResult<LocalTtsInferenceHostResponse> {
        synthesizeTts(request)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOfflineTtsVitsModelConfig {
    model: *const c_char,
    lexicon: *const c_char,
    tokens: *const c_char,
    data_dir: *const c_char,
    noise_scale: c_float,
    noise_scale_w: c_float,
    length_scale: c_float,
    dict_dir: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOfflineTtsMatchaModelConfig {
    acoustic_model: *const c_char,
    vocoder: *const c_char,
    lexicon: *const c_char,
    tokens: *const c_char,
    data_dir: *const c_char,
    noise_scale: c_float,
    length_scale: c_float,
    dict_dir: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOfflineTtsKokoroModelConfig {
    model: *const c_char,
    voices: *const c_char,
    tokens: *const c_char,
    data_dir: *const c_char,
    length_scale: c_float,
    dict_dir: *const c_char,
    lexicon: *const c_char,
    lang: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOfflineTtsKittenModelConfig {
    model: *const c_char,
    voices: *const c_char,
    tokens: *const c_char,
    data_dir: *const c_char,
    length_scale: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOfflineTtsZipvoiceModelConfig {
    tokens: *const c_char,
    encoder: *const c_char,
    decoder: *const c_char,
    vocoder: *const c_char,
    data_dir: *const c_char,
    lexicon: *const c_char,
    feat_scale: c_float,
    t_shift: c_float,
    target_rms: c_float,
    guidance_scale: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOfflineTtsPocketModelConfig {
    lm_flow: *const c_char,
    lm_main: *const c_char,
    encoder: *const c_char,
    decoder: *const c_char,
    text_conditioner: *const c_char,
    vocab_json: *const c_char,
    token_scores_json: *const c_char,
    voice_embedding_cache_capacity: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOfflineTtsSupertonicModelConfig {
    duration_predictor: *const c_char,
    text_encoder: *const c_char,
    vector_estimator: *const c_char,
    vocoder: *const c_char,
    tts_json: *const c_char,
    unicode_indexer: *const c_char,
    voice_style: *const c_char,
}

#[repr(C)]
struct SherpaOnnxOfflineTtsModelConfig {
    vits: SherpaOnnxOfflineTtsVitsModelConfig,
    num_threads: c_int,
    debug: c_int,
    provider: *const c_char,
    matcha: SherpaOnnxOfflineTtsMatchaModelConfig,
    kokoro: SherpaOnnxOfflineTtsKokoroModelConfig,
    kitten: SherpaOnnxOfflineTtsKittenModelConfig,
    zipvoice: SherpaOnnxOfflineTtsZipvoiceModelConfig,
    pocket: SherpaOnnxOfflineTtsPocketModelConfig,
    supertonic: SherpaOnnxOfflineTtsSupertonicModelConfig,
}

#[repr(C)]
struct SherpaOnnxOfflineTtsConfig {
    model: SherpaOnnxOfflineTtsModelConfig,
    rule_fsts: *const c_char,
    max_num_sentences: c_int,
    rule_fars: *const c_char,
    silence_scale: c_float,
}

#[repr(C)]
struct SherpaOnnxGeneratedAudio {
    samples: *const c_float,
    n: c_int,
    sample_rate: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOnlineTransducerModelConfig {
    encoder: *const c_char,
    decoder: *const c_char,
    joiner: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOnlineParaformerModelConfig {
    encoder: *const c_char,
    decoder: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOnlineZipformer2CtcModelConfig {
    model: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOnlineNemoCtcModelConfig {
    model: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SherpaOnnxOnlineToneCtcModelConfig {
    model: *const c_char,
}

#[repr(C)]
struct SherpaOnnxOnlineModelConfig {
    transducer: SherpaOnnxOnlineTransducerModelConfig,
    paraformer: SherpaOnnxOnlineParaformerModelConfig,
    zipformer2_ctc: SherpaOnnxOnlineZipformer2CtcModelConfig,
    tokens: *const c_char,
    num_threads: c_int,
    provider: *const c_char,
    debug: c_int,
    model_type: *const c_char,
    modeling_unit: *const c_char,
    bpe_vocab: *const c_char,
    tokens_buf: *const c_char,
    tokens_buf_size: c_int,
    nemo_ctc: SherpaOnnxOnlineNemoCtcModelConfig,
    t_one_ctc: SherpaOnnxOnlineToneCtcModelConfig,
}

#[repr(C)]
struct SherpaOnnxFeatureConfig {
    sample_rate: c_int,
    feature_dim: c_int,
}

#[repr(C)]
struct SherpaOnnxOnlineCtcFstDecoderConfig {
    graph: *const c_char,
    max_active: c_int,
}

#[repr(C)]
struct SherpaOnnxHomophoneReplacerConfig {
    dict_dir: *const c_char,
    lexicon: *const c_char,
    rule_fsts: *const c_char,
}

#[repr(C)]
struct SherpaOnnxOnlineRecognizerConfig {
    feat_config: SherpaOnnxFeatureConfig,
    model_config: SherpaOnnxOnlineModelConfig,
    decoding_method: *const c_char,
    max_active_paths: c_int,
    enable_endpoint: c_int,
    rule1_min_trailing_silence: c_float,
    rule2_min_trailing_silence: c_float,
    rule3_min_utterance_length: c_float,
    hotwords_file: *const c_char,
    hotwords_score: c_float,
    ctc_fst_decoder_config: SherpaOnnxOnlineCtcFstDecoderConfig,
    rule_fsts: *const c_char,
    rule_fars: *const c_char,
    blank_penalty: c_float,
    hotwords_buf: *const c_char,
    hotwords_buf_size: c_int,
    hr: SherpaOnnxHomophoneReplacerConfig,
}

#[repr(C)]
struct SherpaOnnxOnlineRecognizerResult {
    text: *const c_char,
    tokens: *const c_char,
    tokens_arr: *const *const c_char,
    timestamps: *const c_float,
    count: c_int,
    json: *const c_char,
}

#[repr(C)]
struct SherpaOnnxWave {
    samples: *const c_float,
    sample_rate: c_int,
    num_samples: c_int,
}

enum SherpaOnnxOfflineTts {}
enum SherpaOnnxOnlineRecognizer {}
enum SherpaOnnxOnlineStream {}

type CreateOfflineTts =
    unsafe extern "C" fn(*const SherpaOnnxOfflineTtsConfig) -> *const SherpaOnnxOfflineTts;
type DestroyOfflineTts = unsafe extern "C" fn(*const SherpaOnnxOfflineTts);
type OfflineTtsNumSpeakers = unsafe extern "C" fn(*const SherpaOnnxOfflineTts) -> c_int;
type OfflineTtsGenerate = unsafe extern "C" fn(
    *const SherpaOnnxOfflineTts,
    *const c_char,
    c_int,
    c_float,
) -> *const SherpaOnnxGeneratedAudio;
type DestroyGeneratedAudio = unsafe extern "C" fn(*const SherpaOnnxGeneratedAudio);
type WriteWave = unsafe extern "C" fn(*const c_float, c_int, c_int, *const c_char) -> c_int;
type ReadWave = unsafe extern "C" fn(*const c_char) -> *const SherpaOnnxWave;
type FreeWave = unsafe extern "C" fn(*const SherpaOnnxWave);
type CreateOnlineRecognizer = unsafe extern "C" fn(
    *const SherpaOnnxOnlineRecognizerConfig,
) -> *const SherpaOnnxOnlineRecognizer;
type DestroyOnlineRecognizer = unsafe extern "C" fn(*const SherpaOnnxOnlineRecognizer);
type CreateOnlineStream =
    unsafe extern "C" fn(*const SherpaOnnxOnlineRecognizer) -> *const SherpaOnnxOnlineStream;
type DestroyOnlineStream = unsafe extern "C" fn(*const SherpaOnnxOnlineStream);
type OnlineStreamAcceptWaveform =
    unsafe extern "C" fn(*const SherpaOnnxOnlineStream, c_int, *const c_float, c_int);
type OnlineStreamInputFinished = unsafe extern "C" fn(*const SherpaOnnxOnlineStream);
type IsOnlineStreamReady =
    unsafe extern "C" fn(*const SherpaOnnxOnlineRecognizer, *const SherpaOnnxOnlineStream) -> c_int;
type DecodeOnlineStream =
    unsafe extern "C" fn(*const SherpaOnnxOnlineRecognizer, *const SherpaOnnxOnlineStream);
type GetOnlineStreamResult = unsafe extern "C" fn(
    *const SherpaOnnxOnlineRecognizer,
    *const SherpaOnnxOnlineStream,
) -> *const SherpaOnnxOnlineRecognizerResult;
type DestroyOnlineRecognizerResult = unsafe extern "C" fn(*const SherpaOnnxOnlineRecognizerResult);

/// Transcribes one validated streaming transducer request through Sherpa ONNX.
fn transcribeStreamingTransducer(
    request: LocalSttInferenceHostRequest,
) -> HostResult<LocalSttInferenceHostResponse> {
    validateOptions(&request.optionsJson, "STT")?;
    let engineDirectory = requiredDirectory(&request.engineLibraryDirectory)?;
    let modelDirectory = requiredDirectory(&request.modelDirectory)?;
    let audioPath = requiredFile(Path::new(&request.audioPath))?;
    let driver = requiredStreamingTransducerDriver(&request.driverJson)?;
    let LocalModelDriver::SherpaOnnxStreamingTransducer {
        encoder,
        decoder,
        joiner,
        tokens,
        modelType,
    } = driver
    else {
        return Err(HostError::new(
            "OHOS local STT driver must be SherpaOnnxStreamingTransducer",
        ));
    };

    let encoderPath = pathCString(&declaredModelFile(&modelDirectory, &encoder)?)?;
    let decoderPath = pathCString(&declaredModelFile(&modelDirectory, &decoder)?)?;
    let joinerPath = pathCString(&declaredModelFile(&modelDirectory, &joiner)?)?;
    let tokensPath = pathCString(&declaredModelFile(&modelDirectory, &tokens)?)?;
    let audioPathValue = pathCString(&audioPath)?;
    let provider = CString::new("cpu").map_err(cStringError)?;
    let decodingMethod = CString::new("greedy_search").map_err(cStringError)?;
    let modelTypeValue = CString::new(modelType).map_err(cStringError)?;
    let empty = CString::new("").map_err(cStringError)?;
    let threadCount = nativeThreadCount()?;
    let config = SherpaOnnxOnlineRecognizerConfig {
        feat_config: SherpaOnnxFeatureConfig {
            sample_rate: 16_000,
            feature_dim: 80,
        },
        model_config: SherpaOnnxOnlineModelConfig {
            transducer: SherpaOnnxOnlineTransducerModelConfig {
                encoder: encoderPath.as_ptr(),
                decoder: decoderPath.as_ptr(),
                joiner: joinerPath.as_ptr(),
            },
            paraformer: SherpaOnnxOnlineParaformerModelConfig {
                encoder: empty.as_ptr(),
                decoder: empty.as_ptr(),
            },
            zipformer2_ctc: SherpaOnnxOnlineZipformer2CtcModelConfig {
                model: empty.as_ptr(),
            },
            tokens: tokensPath.as_ptr(),
            num_threads: threadCount,
            provider: provider.as_ptr(),
            debug: 0,
            model_type: modelTypeValue.as_ptr(),
            modeling_unit: empty.as_ptr(),
            bpe_vocab: empty.as_ptr(),
            tokens_buf: std::ptr::null(),
            tokens_buf_size: 0,
            nemo_ctc: SherpaOnnxOnlineNemoCtcModelConfig {
                model: empty.as_ptr(),
            },
            t_one_ctc: SherpaOnnxOnlineToneCtcModelConfig {
                model: empty.as_ptr(),
            },
        },
        decoding_method: decodingMethod.as_ptr(),
        max_active_paths: 4,
        enable_endpoint: 0,
        rule1_min_trailing_silence: 2.4,
        rule2_min_trailing_silence: 1.4,
        rule3_min_utterance_length: 20.0,
        hotwords_file: empty.as_ptr(),
        hotwords_score: 1.5,
        ctc_fst_decoder_config: SherpaOnnxOnlineCtcFstDecoderConfig {
            graph: empty.as_ptr(),
            max_active: 3_000,
        },
        rule_fsts: empty.as_ptr(),
        rule_fars: empty.as_ptr(),
        blank_penalty: 0.0,
        hotwords_buf: std::ptr::null(),
        hotwords_buf_size: 0,
        hr: SherpaOnnxHomophoneReplacerConfig {
            dict_dir: empty.as_ptr(),
            lexicon: empty.as_ptr(),
            rule_fsts: empty.as_ptr(),
        },
    };
    let (_onnxRuntimeLibrary, library) = loadSherpaLibraries(&engineDirectory)?;
    unsafe { runStreamingTranscription(&library, &config, audioPathValue.as_ptr()) }
}

/// Synthesizes one validated TTS request through the loaded native engine.
fn synthesizeTts(
    request: LocalTtsInferenceHostRequest,
) -> HostResult<LocalTtsInferenceHostResponse> {
    validateOptions(&request.optionsJson, "TTS")?;
    let text = requiredText(&request.text)?.to_string();
    let speed = requiredSpeed(request.speed)?;
    let speaker = requiredSpeaker(&request.voice)?;
    let engineDirectory = requiredDirectory(&request.engineLibraryDirectory)?;
    let modelDirectory = requiredDirectory(&request.modelDirectory)?;
    let outputPath = requiredOutputPath(&request.outputPath)?;
    let driver = requiredTtsDriver(&request.driverJson)?;
    let provider = CString::new("cpu").map_err(cStringError)?;
    let textValue = CString::new(text).map_err(cStringError)?;
    let outputValue = pathCString(&outputPath)?;
    let empty = CString::new("").map_err(cStringError)?;
    let threadCount = nativeThreadCount()?;
    let (_onnxRuntimeLibrary, library) = loadSherpaLibraries(&engineDirectory)?;

    match driver {
        LocalModelDriver::SherpaOnnxVits {
            model,
            lexicon,
            tokens,
            ruleFsts,
            ruleFars,
            speakerCount,
        } => {
            validateTtsSpeakerId(speaker, speakerCount)?;
            let modelPath = pathCString(&declaredModelFile(&modelDirectory, &model)?)?;
            let lexiconPath = pathCString(&declaredModelFile(&modelDirectory, &lexicon)?)?;
            let tokensPath = pathCString(&declaredModelFile(&modelDirectory, &tokens)?)?;
            let ruleFstsValue = joinedModelFiles(&modelDirectory, &ruleFsts)?;
            let ruleFarsValue = joinedModelFiles(&modelDirectory, &ruleFars)?;
            let config = SherpaOnnxOfflineTtsConfig {
                model: SherpaOnnxOfflineTtsModelConfig {
                    vits: SherpaOnnxOfflineTtsVitsModelConfig {
                        model: modelPath.as_ptr(),
                        lexicon: lexiconPath.as_ptr(),
                        tokens: tokensPath.as_ptr(),
                        data_dir: empty.as_ptr(),
                        noise_scale: 0.667,
                        noise_scale_w: 0.8,
                        length_scale: 1.0,
                        dict_dir: empty.as_ptr(),
                    },
                    num_threads: threadCount,
                    debug: 0,
                    provider: provider.as_ptr(),
                    matcha: unsafe { std::mem::zeroed() },
                    kokoro: unsafe { std::mem::zeroed() },
                    kitten: unsafe { std::mem::zeroed() },
                    zipvoice: unsafe { std::mem::zeroed() },
                    pocket: unsafe { std::mem::zeroed() },
                    supertonic: unsafe { std::mem::zeroed() },
                },
                rule_fsts: ruleFstsValue.as_ptr(),
                max_num_sentences: 1,
                rule_fars: ruleFarsValue.as_ptr(),
                silence_scale: 0.2,
            };
            unsafe {
                runSynthesis(
                    &library,
                    &config,
                    textValue.as_ptr(),
                    speaker,
                    speed,
                    speakerCount,
                    outputValue.as_ptr(),
                )?;
            }
        }
        LocalModelDriver::SherpaOnnxMatcha {
            acousticModel,
            vocoder,
            lexicon,
            tokens,
            ruleFsts,
            ruleFars,
            speakerCount,
        } => {
            validateTtsSpeakerId(speaker, speakerCount)?;
            let acousticModelPath =
                pathCString(&declaredModelFile(&modelDirectory, &acousticModel)?)?;
            let vocoderPath = pathCString(&declaredModelFile(&modelDirectory, &vocoder)?)?;
            let lexiconPath = pathCString(&declaredModelFile(&modelDirectory, &lexicon)?)?;
            let tokensPath = pathCString(&declaredModelFile(&modelDirectory, &tokens)?)?;
            let ruleFstsValue = joinedModelFiles(&modelDirectory, &ruleFsts)?;
            let ruleFarsValue = joinedModelFiles(&modelDirectory, &ruleFars)?;
            let config = SherpaOnnxOfflineTtsConfig {
                model: SherpaOnnxOfflineTtsModelConfig {
                    vits: unsafe { std::mem::zeroed() },
                    num_threads: threadCount,
                    debug: 0,
                    provider: provider.as_ptr(),
                    matcha: SherpaOnnxOfflineTtsMatchaModelConfig {
                        acoustic_model: acousticModelPath.as_ptr(),
                        vocoder: vocoderPath.as_ptr(),
                        lexicon: lexiconPath.as_ptr(),
                        tokens: tokensPath.as_ptr(),
                        data_dir: empty.as_ptr(),
                        noise_scale: 1.0,
                        length_scale: 1.0,
                        dict_dir: empty.as_ptr(),
                    },
                    kokoro: unsafe { std::mem::zeroed() },
                    kitten: unsafe { std::mem::zeroed() },
                    zipvoice: unsafe { std::mem::zeroed() },
                    pocket: unsafe { std::mem::zeroed() },
                    supertonic: unsafe { std::mem::zeroed() },
                },
                rule_fsts: ruleFstsValue.as_ptr(),
                max_num_sentences: 1,
                rule_fars: ruleFarsValue.as_ptr(),
                silence_scale: 0.2,
            };
            unsafe {
                runSynthesis(
                    &library,
                    &config,
                    textValue.as_ptr(),
                    speaker,
                    speed,
                    speakerCount,
                    outputValue.as_ptr(),
                )?;
            }
        }
        LocalModelDriver::SherpaOnnxKitten {
            model,
            voices,
            tokens,
            dataDir,
            speakerCount,
        } => {
            validateTtsSpeakerId(speaker, speakerCount)?;
            let modelPath = pathCString(&declaredModelFile(&modelDirectory, &model)?)?;
            let voicesPath = pathCString(&declaredModelFile(&modelDirectory, &voices)?)?;
            let tokensPath = pathCString(&declaredModelFile(&modelDirectory, &tokens)?)?;
            let dataDirPath = pathCString(&declaredModelDirectory(&modelDirectory, &dataDir)?)?;
            let config = SherpaOnnxOfflineTtsConfig {
                model: SherpaOnnxOfflineTtsModelConfig {
                    vits: unsafe { std::mem::zeroed() },
                    num_threads: threadCount,
                    debug: 0,
                    provider: provider.as_ptr(),
                    matcha: unsafe { std::mem::zeroed() },
                    kokoro: unsafe { std::mem::zeroed() },
                    kitten: SherpaOnnxOfflineTtsKittenModelConfig {
                        model: modelPath.as_ptr(),
                        voices: voicesPath.as_ptr(),
                        tokens: tokensPath.as_ptr(),
                        data_dir: dataDirPath.as_ptr(),
                        length_scale: 1.0,
                    },
                    zipvoice: unsafe { std::mem::zeroed() },
                    pocket: unsafe { std::mem::zeroed() },
                    supertonic: unsafe { std::mem::zeroed() },
                },
                rule_fsts: empty.as_ptr(),
                max_num_sentences: 1,
                rule_fars: empty.as_ptr(),
                silence_scale: 0.2,
            };
            unsafe {
                runSynthesis(
                    &library,
                    &config,
                    textValue.as_ptr(),
                    speaker,
                    speed,
                    speakerCount,
                    outputValue.as_ptr(),
                )?;
            }
        }
        unsupported => {
            return Err(HostError::new(format!(
                "OHOS local TTS driver is unsupported: {unsupported:?}"
            )));
        }
    }
    let metadata = fs::metadata(&outputPath).map_err(ioError)?;
    if metadata.len() <= 44 {
        return Err(HostError::new(format!(
            "OHOS local TTS output WAV is invalid: {}",
            outputPath.display()
        )));
    }
    Ok(LocalTtsInferenceHostResponse {
        audioPath: outputPath.to_string_lossy().to_string(),
        outputFormat: "wav".to_string(),
    })
}

/// Runs native streaming transcription while releasing every Sherpa allocation.
unsafe fn runStreamingTranscription(
    library: &Library,
    config: &SherpaOnnxOnlineRecognizerConfig,
    audioPath: *const c_char,
) -> HostResult<LocalSttInferenceHostResponse> {
    let readWave: Symbol<ReadWave> = loadSymbol(library, b"SherpaOnnxReadWave\0")?;
    let freeWave: Symbol<FreeWave> = loadSymbol(library, b"SherpaOnnxFreeWave\0")?;
    let createRecognizer: Symbol<CreateOnlineRecognizer> =
        loadSymbol(library, b"SherpaOnnxCreateOnlineRecognizer\0")?;
    let destroyRecognizer: Symbol<DestroyOnlineRecognizer> =
        loadSymbol(library, b"SherpaOnnxDestroyOnlineRecognizer\0")?;
    let createStream: Symbol<CreateOnlineStream> =
        loadSymbol(library, b"SherpaOnnxCreateOnlineStream\0")?;
    let destroyStream: Symbol<DestroyOnlineStream> =
        loadSymbol(library, b"SherpaOnnxDestroyOnlineStream\0")?;
    let acceptWaveform: Symbol<OnlineStreamAcceptWaveform> =
        loadSymbol(library, b"SherpaOnnxOnlineStreamAcceptWaveform\0")?;
    let inputFinished: Symbol<OnlineStreamInputFinished> =
        loadSymbol(library, b"SherpaOnnxOnlineStreamInputFinished\0")?;
    let isReady: Symbol<IsOnlineStreamReady> =
        loadSymbol(library, b"SherpaOnnxIsOnlineStreamReady\0")?;
    let decode: Symbol<DecodeOnlineStream> =
        loadSymbol(library, b"SherpaOnnxDecodeOnlineStream\0")?;
    let getResult: Symbol<GetOnlineStreamResult> =
        loadSymbol(library, b"SherpaOnnxGetOnlineStreamResult\0")?;
    let destroyResult: Symbol<DestroyOnlineRecognizerResult> =
        loadSymbol(library, b"SherpaOnnxDestroyOnlineRecognizerResult\0")?;

    let wave = readWave(audioPath);
    if wave.is_null() {
        return Err(HostError::new("OHOS local STT failed to read WAV input"));
    }
    let waveValue = &*wave;
    if waveValue.samples.is_null() || waveValue.sample_rate <= 0 || waveValue.num_samples <= 0 {
        freeWave(wave);
        return Err(HostError::new("OHOS local STT WAV input is invalid"));
    }

    let recognizer = createRecognizer(config);
    if recognizer.is_null() {
        freeWave(wave);
        return Err(HostError::new(
            "OHOS Sherpa ONNX failed to create the STT recognizer",
        ));
    }
    let stream = createStream(recognizer);
    if stream.is_null() {
        destroyRecognizer(recognizer);
        freeWave(wave);
        return Err(HostError::new(
            "OHOS Sherpa ONNX failed to create the STT stream",
        ));
    }
    acceptWaveform(
        stream,
        waveValue.sample_rate,
        waveValue.samples,
        waveValue.num_samples,
    );
    inputFinished(stream);
    while isReady(recognizer, stream) != 0 {
        decode(recognizer, stream);
    }
    let result = getResult(recognizer, stream);
    if result.is_null() {
        destroyStream(stream);
        destroyRecognizer(recognizer);
        freeWave(wave);
        return Err(HostError::new("OHOS Sherpa ONNX returned no STT result"));
    }
    let resultValue = &*result;
    let textResult = cStringFromPointer(resultValue.text, "OHOS local STT text");
    let resultJsonResult =
        cStringFromPointer(resultValue.json, "OHOS local STT result JSON");
    destroyResult(result);
    destroyStream(stream);
    destroyRecognizer(recognizer);
    freeWave(wave);
    let text = textResult?;
    let resultJson = resultJsonResult?;
    Ok(LocalSttInferenceHostResponse { text, resultJson })
}

/// Runs native synthesis while releasing every Sherpa allocation.
unsafe fn runSynthesis(
    library: &Library,
    config: &SherpaOnnxOfflineTtsConfig,
    text: *const c_char,
    speaker: c_int,
    speed: c_float,
    speakerCount: c_int,
    outputPath: *const c_char,
) -> HostResult<()> {
    let create: Symbol<CreateOfflineTts> = loadSymbol(library, b"SherpaOnnxCreateOfflineTts\0")?;
    let destroy: Symbol<DestroyOfflineTts> = loadSymbol(library, b"SherpaOnnxDestroyOfflineTts\0")?;
    let numSpeakers: Symbol<OfflineTtsNumSpeakers> =
        loadSymbol(library, b"SherpaOnnxOfflineTtsNumSpeakers\0")?;
    let generate: Symbol<OfflineTtsGenerate> =
        loadSymbol(library, b"SherpaOnnxOfflineTtsGenerate\0")?;
    let destroyAudio: Symbol<DestroyGeneratedAudio> =
        loadSymbol(library, b"SherpaOnnxDestroyOfflineTtsGeneratedAudio\0")?;
    let writeWave: Symbol<WriteWave> = loadSymbol(library, b"SherpaOnnxWriteWave\0")?;
    let tts = create(config);
    if tts.is_null() {
        return Err(HostError::new(
            "OHOS Sherpa ONNX failed to create the TTS engine",
        ));
    }
    let actualSpeakerCount = numSpeakers(tts);
    if actualSpeakerCount != speakerCount {
        destroy(tts);
        return Err(HostError::new(format!(
            "OHOS local TTS speaker count mismatch: manifest={speakerCount}, engine={actualSpeakerCount}"
        )));
    }
    let audio = generate(tts, text, speaker, speed);
    if audio.is_null() {
        destroy(tts);
        return Err(HostError::new("OHOS Sherpa ONNX generated no audio"));
    }
    let value = &*audio;
    let validAudio = !value.samples.is_null() && value.n > 0 && value.sample_rate > 0;
    let written =
        validAudio && writeWave(value.samples, value.n, value.sample_rate, outputPath) == 1;
    destroyAudio(audio);
    destroy(tts);
    if !validAudio {
        return Err(HostError::new(
            "OHOS Sherpa ONNX returned invalid audio samples",
        ));
    }
    if !written {
        return Err(HostError::new(
            "OHOS Sherpa ONNX failed to write the output WAV",
        ));
    }
    Ok(())
}

/// Loads one required symbol from the Sherpa ONNX library.
unsafe fn loadSymbol<'a, T>(library: &'a Library, name: &[u8]) -> HostResult<Symbol<'a, T>> {
    library.get(name).map_err(|error| {
        HostError::new(format!(
            "OHOS Sherpa ONNX symbol {} is unavailable: {error}",
            String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
        ))
    })
}

/// Loads the installed ONNX Runtime dependency and Sherpa C API in dependency order.
fn loadSherpaLibraries(engineDirectory: &Path) -> HostResult<(Library, Library)> {
    let onnxRuntimePath = requiredFile(&engineDirectory.join("libonnxruntime.so"))?;
    let sherpaPath = requiredFile(&engineDirectory.join("libsherpa-onnx-c-api.so"))?;
    let onnxRuntimeLibrary = unsafe { Library::new(&onnxRuntimePath) }.map_err(|error| {
        HostError::new(format!(
            "failed to load installed OHOS ONNX Runtime library {}: {error}",
            onnxRuntimePath.display()
        ))
    })?;
    let sherpaLibrary = unsafe { Library::new(&sherpaPath) }.map_err(|error| {
        HostError::new(format!(
            "failed to load installed OHOS Sherpa ONNX library {}: {error}",
            sherpaPath.display()
        ))
    })?;
    Ok((onnxRuntimeLibrary, sherpaLibrary))
}

/// Parses one exact local TTS model driver value.
fn requiredTtsDriver(driverJson: &str) -> HostResult<LocalModelDriver> {
    serde_json::from_str(driverJson)
        .map_err(|error| HostError::new(format!("OHOS local TTS driver JSON is invalid: {error}")))
}

/// Parses one exact local STT model driver value.
fn requiredStreamingTransducerDriver(driverJson: &str) -> HostResult<LocalModelDriver> {
    serde_json::from_str(driverJson)
        .map_err(|error| HostError::new(format!("OHOS local STT driver JSON is invalid: {error}")))
}

/// Validates the structured local inference options object.
fn validateOptions(optionsJson: &str, capability: &str) -> HostResult<()> {
    let value: serde_json::Value = serde_json::from_str(optionsJson).map_err(|error| {
        HostError::new(format!(
            "OHOS local {capability} options JSON is invalid: {error}"
        ))
    })?;
    if !value.is_object() {
        return Err(HostError::new(format!(
            "OHOS local {capability} options must be a JSON object"
        )));
    }
    Ok(())
}

/// Returns non-empty synthesis text.
fn requiredText(text: &str) -> HostResult<&str> {
    let value = text.trim();
    if value.is_empty() {
        return Err(HostError::new("OHOS local TTS text is empty"));
    }
    Ok(value)
}

/// Returns one finite positive speech speed.
fn requiredSpeed(speed: f64) -> HostResult<c_float> {
    if !speed.is_finite() || speed <= 0.0 {
        return Err(HostError::new(
            "OHOS local TTS speed must be finite and positive",
        ));
    }
    Ok(speed as c_float)
}

/// Parses one numeric TTS speaker id.
fn requiredSpeaker(voice: &str) -> HostResult<c_int> {
    voice.trim().parse::<c_int>().map_err(|error| {
        HostError::new(format!(
            "OHOS local TTS voice must be a numeric speaker id: {error}"
        ))
    })
}

/// Validates one requested TTS speaker id against manifest metadata.
fn validateTtsSpeakerId(speaker: c_int, speakerCount: c_int) -> HostResult<()> {
    if speaker < 0 || speaker >= speakerCount {
        return Err(HostError::new(format!(
            "OHOS local TTS speaker must be between 0 and {}",
            speakerCount - 1
        )));
    }
    Ok(())
}

/// Returns one canonical existing directory.
fn requiredDirectory(path: &str) -> HostResult<PathBuf> {
    let directory = fs::canonicalize(path).map_err(ioError)?;
    if !directory.is_dir() {
        return Err(HostError::new(format!(
            "required OHOS local model directory is missing: {}",
            directory.display()
        )));
    }
    Ok(directory)
}

/// Returns one canonical existing file.
fn requiredFile(path: &Path) -> HostResult<PathBuf> {
    let file = fs::canonicalize(path).map_err(ioError)?;
    if !file.is_file() {
        return Err(HostError::new(format!(
            "required OHOS local model file is missing: {}",
            file.display()
        )));
    }
    Ok(file)
}

/// Resolves one model file inside the installed model directory.
fn declaredModelFile(modelDirectory: &Path, relativePath: &str) -> HostResult<PathBuf> {
    if relativePath.trim().is_empty() {
        return Err(HostError::new("OHOS local model file path is empty"));
    }
    let file = requiredFile(&modelDirectory.join(relativePath))?;
    if !file.starts_with(modelDirectory) {
        return Err(HostError::new(format!(
            "OHOS local model file escapes its installation directory: {relativePath}"
        )));
    }
    Ok(file)
}

/// Resolves one model directory inside the installed model directory.
fn declaredModelDirectory(modelDirectory: &Path, relativePath: &str) -> HostResult<PathBuf> {
    if relativePath.trim().is_empty() {
        return Err(HostError::new("OHOS local model directory path is empty"));
    }
    let directory = requiredDirectory(&modelDirectory.join(relativePath).to_string_lossy())?;
    if !directory.starts_with(modelDirectory) {
        return Err(HostError::new(format!(
            "OHOS local model directory escapes its installation directory: {relativePath}"
        )));
    }
    Ok(directory)
}

/// Joins declared model paths for one Sherpa rule list.
fn joinedModelFiles(modelDirectory: &Path, paths: &[String]) -> HostResult<CString> {
    let values = paths
        .iter()
        .map(|path| {
            declaredModelFile(modelDirectory, path).map(|file| file.to_string_lossy().to_string())
        })
        .collect::<HostResult<Vec<_>>>()?;
    CString::new(values.join(",")).map_err(cStringError)
}

/// Validates one new writable output file path.
fn requiredOutputPath(path: &str) -> HostResult<PathBuf> {
    let output = PathBuf::from(path);
    if output.exists() {
        return Err(HostError::new(format!(
            "OHOS local TTS output file already exists: {}",
            output.display()
        )));
    }
    let parent = output
        .parent()
        .ok_or_else(|| HostError::new("OHOS local TTS output path has no parent"))?;
    let canonicalParent = requiredDirectory(&parent.to_string_lossy())?;
    let fileName = output
        .file_name()
        .ok_or_else(|| HostError::new("OHOS local TTS output path has no file name"))?;
    Ok(canonicalParent.join(fileName))
}

/// Encodes one filesystem path as a C string.
fn pathCString(path: &Path) -> HostResult<CString> {
    CString::new(path.to_string_lossy().as_bytes()).map_err(cStringError)
}

/// Returns a bounded native inference thread count.
fn nativeThreadCount() -> HostResult<c_int> {
    std::thread::available_parallelism()
        .map(|count| count.get().min(4) as c_int)
        .map_err(|error| {
            HostError::new(format!(
                "OHOS local inference could not determine native parallelism: {error}"
            ))
        })
}

/// Reads an owned UTF-8 string from one non-null C pointer.
unsafe fn cStringFromPointer(pointer: *const c_char, label: &str) -> HostResult<String> {
    if pointer.is_null() {
        return Err(HostError::new(format!("{label} pointer is null")));
    }
    CStr::from_ptr(pointer)
        .to_str()
        .map(str::to_string)
        .map_err(|error| HostError::new(format!("{label} is not UTF-8: {error}")))
}

/// Converts an interior-NUL error into a host error.
fn cStringError(error: std::ffi::NulError) -> HostError {
    HostError::new(format!(
        "OHOS local inference string contains an interior NUL: {error}"
    ))
}

/// Converts one filesystem error into a host error.
fn ioError(error: std::io::Error) -> HostError {
    HostError::new(format!("OHOS local inference filesystem error: {error}"))
}
