package com.k2fsa.sherpa.onnx

import android.content.res.AssetManager

data class FeatureConfig(
    var sampleRate: Int = 16000,
    var featureDim: Int = 80,
    var dither: Float = 0.0f,
)

data class QnnConfig(
    var backendLib: String = "",
    var contextBinary: String = "",
    var systemLib: String = "",
)

data class HomophoneReplacerConfig(
    var dictDir: String = "",
    var lexicon: String = "",
    var ruleFsts: String = "",
)

data class EndpointRule(
    var mustContainNonSilence: Boolean,
    var minTrailingSilence: Float,
    var minUtteranceLength: Float,
)

data class EndpointConfig(
    var rule1: EndpointRule = EndpointRule(false, 2.4f, 0.0f),
    var rule2: EndpointRule = EndpointRule(true, 1.4f, 0.0f),
    var rule3: EndpointRule = EndpointRule(false, 0.0f, 20.0f),
)

data class OnlineTransducerModelConfig(
    var encoder: String = "",
    var decoder: String = "",
    var joiner: String = "",
    var qnnConfig: QnnConfig = QnnConfig(),
)

data class OnlineParaformerModelConfig(
    var encoder: String = "",
    var decoder: String = "",
)

data class OnlineZipformer2CtcModelConfig(var model: String = "")

data class OnlineNeMoCtcModelConfig(var model: String = "")

data class OnlineToneCtcModelConfig(var model: String = "")

data class OnlineModelConfig(
    var transducer: OnlineTransducerModelConfig = OnlineTransducerModelConfig(),
    var paraformer: OnlineParaformerModelConfig = OnlineParaformerModelConfig(),
    var zipformer2Ctc: OnlineZipformer2CtcModelConfig = OnlineZipformer2CtcModelConfig(),
    var neMoCtc: OnlineNeMoCtcModelConfig = OnlineNeMoCtcModelConfig(),
    var toneCtc: OnlineToneCtcModelConfig = OnlineToneCtcModelConfig(),
    var tokens: String = "",
    var numThreads: Int = 1,
    var debug: Boolean = false,
    var provider: String = "cpu",
    var modelType: String = "",
    var modelingUnit: String = "",
    var bpeVocab: String = "",
)

data class OnlineLMConfig(
    var model: String = "",
    var scale: Float = 0.5f,
)

data class OnlineCtcFstDecoderConfig(
    var graph: String = "",
    var maxActive: Int = 3000,
)

data class OnlineRecognizerConfig(
    var featConfig: FeatureConfig = FeatureConfig(),
    var modelConfig: OnlineModelConfig = OnlineModelConfig(),
    var lmConfig: OnlineLMConfig = OnlineLMConfig(),
    var ctcFstDecoderConfig: OnlineCtcFstDecoderConfig = OnlineCtcFstDecoderConfig(),
    var hr: HomophoneReplacerConfig = HomophoneReplacerConfig(),
    var endpointConfig: EndpointConfig = EndpointConfig(),
    var enableEndpoint: Boolean = true,
    var decodingMethod: String = "greedy_search",
    var maxActivePaths: Int = 4,
    var hotwordsFile: String = "",
    var hotwordsScore: Float = 1.5f,
    var ruleFsts: String = "",
    var ruleFars: String = "",
    var blankPenalty: Float = 0.0f,
)

data class OnlineRecognizerResult(
    val text: String,
    val tokens: Array<String>,
    val timestamps: FloatArray,
    val ysProbs: FloatArray,
)

class OnlineStream(var ptr: Long = 0) {
    /** Accepts waveform samples for streaming recognition. */
    fun acceptWaveform(samples: FloatArray, sampleRate: Int) =
        acceptWaveform(ptr, samples, sampleRate)

    /** Marks the input waveform as complete. */
    fun inputFinished() = inputFinished(ptr)

    /** Sets one stream option. */
    fun setOption(key: String, value: String) = setOption(ptr, key, value)

    /** Reads one stream option. */
    fun getOption(key: String): String = getOption(ptr, key)

    /** Releases native stream resources. */
    fun release() {
        if (ptr != 0L) {
            delete(ptr)
            ptr = 0
        }
    }

    /** Accepts waveform samples in the native stream. */
    private external fun acceptWaveform(ptr: Long, samples: FloatArray, sampleRate: Int)

    /** Marks native stream input as complete. */
    private external fun inputFinished(ptr: Long)

    /** Sets one native stream option. */
    private external fun setOption(ptr: Long, key: String, value: String)

    /** Reads one native stream option. */
    private external fun getOption(ptr: Long, key: String): String

    /** Deletes one native stream. */
    private external fun delete(ptr: Long)
}

class OnlineRecognizer(
    assetManager: AssetManager? = null,
    val config: OnlineRecognizerConfig,
) {
    private var ptr: Long = if (assetManager == null) {
        newFromFile(config)
    } else {
        newFromAsset(assetManager, config)
    }

    /** Creates one streaming recognizer input. */
    fun createStream(hotwords: String = ""): OnlineStream = OnlineStream(createStream(ptr, hotwords))

    /** Resets one recognizer stream. */
    fun reset(stream: OnlineStream) = reset(ptr, stream.ptr)

    /** Decodes pending frames from one stream. */
    fun decode(stream: OnlineStream) = decode(ptr, stream.ptr)

    /** Returns whether one stream has reached an endpoint. */
    fun isEndpoint(stream: OnlineStream): Boolean = isEndpoint(ptr, stream.ptr)

    /** Returns whether one stream has frames ready for decoding. */
    fun isReady(stream: OnlineStream): Boolean = isReady(ptr, stream.ptr)

    /** Returns the current recognition result. */
    fun getResult(stream: OnlineStream): OnlineRecognizerResult = getResult(ptr, stream.ptr)

    /** Releases native recognizer resources. */
    fun release() {
        if (ptr != 0L) {
            delete(ptr)
            ptr = 0
        }
    }

    /** Deletes one native recognizer. */
    private external fun delete(ptr: Long)

    /** Creates one recognizer from Android assets. */
    private external fun newFromAsset(
        assetManager: AssetManager,
        config: OnlineRecognizerConfig,
    ): Long

    /** Creates one recognizer from filesystem model files. */
    private external fun newFromFile(config: OnlineRecognizerConfig): Long

    /** Creates one native recognizer stream. */
    private external fun createStream(ptr: Long, hotwords: String): Long

    /** Resets one native recognizer stream. */
    private external fun reset(ptr: Long, streamPtr: Long)

    /** Decodes one native recognizer stream. */
    private external fun decode(ptr: Long, streamPtr: Long)

    /** Reads endpoint state from one native recognizer stream. */
    private external fun isEndpoint(ptr: Long, streamPtr: Long): Boolean

    /** Reads readiness state from one native recognizer stream. */
    private external fun isReady(ptr: Long, streamPtr: Long): Boolean

    /** Reads one native recognizer result. */
    private external fun getResult(ptr: Long, streamPtr: Long): OnlineRecognizerResult
}

data class WaveData(
    val samples: FloatArray,
    val sampleRate: Int,
)

class WaveReader {
    companion object {
        /** Reads one mono WAV file from Android assets. */
        @JvmStatic
        external fun readWaveFromAsset(assetManager: AssetManager, filename: String): WaveData

        /** Reads one mono WAV file from the filesystem. */
        @JvmStatic
        external fun readWaveFromFile(filename: String): WaveData
    }
}

data class OfflineTtsVitsModelConfig(
    var model: String = "",
    var lexicon: String = "",
    var tokens: String = "",
    var dataDir: String = "",
    var dictDir: String = "",
    var noiseScale: Float = 0.667f,
    var noiseScaleW: Float = 0.8f,
    var lengthScale: Float = 1.0f,
)

data class OfflineTtsMatchaModelConfig(
    var acousticModel: String = "",
    var vocoder: String = "",
    var lexicon: String = "",
    var tokens: String = "",
    var dataDir: String = "",
    var dictDir: String = "",
    var noiseScale: Float = 1.0f,
    var lengthScale: Float = 1.0f,
)

data class OfflineTtsKokoroModelConfig(
    var model: String = "",
    var voices: String = "",
    var tokens: String = "",
    var dataDir: String = "",
    var lexicon: String = "",
    var lang: String = "",
    var dictDir: String = "",
    var lengthScale: Float = 1.0f,
)

data class OfflineTtsZipVoiceModelConfig(
    var tokens: String = "",
    var encoder: String = "",
    var decoder: String = "",
    var vocoder: String = "",
    var dataDir: String = "",
    var lexicon: String = "",
    var featScale: Float = 0.1f,
    var tShift: Float = 0.5f,
    var targetRms: Float = 0.1f,
    var guidanceScale: Float = 1.0f,
)

data class OfflineTtsKittenModelConfig(
    var model: String = "",
    var voices: String = "",
    var tokens: String = "",
    var dataDir: String = "",
    var lengthScale: Float = 1.0f,
)

data class OfflineTtsPocketModelConfig(
    var lmFlow: String = "",
    var lmMain: String = "",
    var encoder: String = "",
    var decoder: String = "",
    var textConditioner: String = "",
    var vocabJson: String = "",
    var tokenScoresJson: String = "",
    var voiceEmbeddingCacheCapacity: Int = 50,
)

data class OfflineTtsSupertonicModelConfig(
    var durationPredictor: String = "",
    var textEncoder: String = "",
    var vectorEstimator: String = "",
    var vocoder: String = "",
    var ttsJson: String = "",
    var unicodeIndexer: String = "",
    var voiceStyle: String = "",
)

data class OfflineTtsModelConfig(
    var vits: OfflineTtsVitsModelConfig = OfflineTtsVitsModelConfig(),
    var matcha: OfflineTtsMatchaModelConfig = OfflineTtsMatchaModelConfig(),
    var kokoro: OfflineTtsKokoroModelConfig = OfflineTtsKokoroModelConfig(),
    var zipvoice: OfflineTtsZipVoiceModelConfig = OfflineTtsZipVoiceModelConfig(),
    var kitten: OfflineTtsKittenModelConfig = OfflineTtsKittenModelConfig(),
    var pocket: OfflineTtsPocketModelConfig = OfflineTtsPocketModelConfig(),
    var supertonic: OfflineTtsSupertonicModelConfig = OfflineTtsSupertonicModelConfig(),
    var numThreads: Int = 1,
    var debug: Boolean = false,
    var provider: String = "cpu",
)

data class OfflineTtsConfig(
    var model: OfflineTtsModelConfig = OfflineTtsModelConfig(),
    var ruleFsts: String = "",
    var ruleFars: String = "",
    var maxNumSentences: Int = 1,
    var silenceScale: Float = 0.2f,
)

class GeneratedAudio(
    val samples: FloatArray,
    val sampleRate: Int,
) {
    /** Writes generated audio through the Sherpa ONNX native WAV writer. */
    fun save(filename: String): Boolean = saveImpl(filename, samples, sampleRate)

    /** Writes normalized samples to a WAV file. */
    private external fun saveImpl(filename: String, samples: FloatArray, sampleRate: Int): Boolean
}

data class GenerationConfig(
    var silenceScale: Float = 0.2f,
    var speed: Float = 1.0f,
    var sid: Int = 0,
    var referenceAudio: FloatArray? = null,
    var referenceSampleRate: Int = 0,
    var referenceText: String? = null,
    var numSteps: Int = 5,
    var extra: Map<String, String>? = null,
)

class OfflineTts(
    assetManager: AssetManager? = null,
    var config: OfflineTtsConfig,
) {
    private var ptr: Long = if (assetManager == null) {
        newFromFile(config)
    } else {
        newFromAsset(assetManager, config)
    }

    /** Returns the generated audio sample rate. */
    fun sampleRate(): Int = getSampleRate(ptr)

    /** Returns the number of model speakers. */
    fun numSpeakers(): Int = getNumSpeakers(ptr)

    /** Generates audio for one text and speaker. */
    fun generate(text: String, sid: Int = 0, speed: Float = 1.0f): GeneratedAudio =
        generateImpl(ptr, text, sid, speed)

    /** Generates audio while streaming sample blocks to a callback. */
    fun generateWithCallback(
        text: String,
        sid: Int = 0,
        speed: Float = 1.0f,
        callback: (samples: FloatArray) -> Int,
    ): GeneratedAudio = generateWithCallbackImpl(ptr, text, sid, speed, callback)

    /** Generates audio with an explicit generation configuration. */
    fun generateWithConfig(text: String, config: GenerationConfig): GeneratedAudio =
        generateWithConfigImpl(ptr, text, config, null)

    /** Generates configured audio while streaming sample blocks to a callback. */
    fun generateWithConfigAndCallback(
        text: String,
        config: GenerationConfig,
        callback: (samples: FloatArray) -> Int,
    ): GeneratedAudio = generateWithConfigImpl(ptr, text, config, callback)

    /** Allocates native TTS resources after a prior release. */
    fun allocate(assetManager: AssetManager? = null) {
        if (ptr == 0L) {
            ptr = if (assetManager == null) {
                newFromFile(config)
            } else {
                newFromAsset(assetManager, config)
            }
        }
    }

    /** Releases native TTS resources. */
    fun release() {
        if (ptr != 0L) {
            delete(ptr)
            ptr = 0
        }
    }

    /** Creates one TTS engine from Android assets. */
    private external fun newFromAsset(assetManager: AssetManager, config: OfflineTtsConfig): Long

    /** Creates one TTS engine from filesystem model files. */
    private external fun newFromFile(config: OfflineTtsConfig): Long

    /** Deletes one native TTS engine. */
    private external fun delete(ptr: Long)

    /** Reads the native TTS sample rate. */
    private external fun getSampleRate(ptr: Long): Int

    /** Reads the native TTS speaker count. */
    private external fun getNumSpeakers(ptr: Long): Int

    /** Generates one native audio result. */
    private external fun generateImpl(ptr: Long, text: String, sid: Int, speed: Float): GeneratedAudio

    /** Generates one native audio result with a callback. */
    private external fun generateWithCallbackImpl(
        ptr: Long,
        text: String,
        sid: Int,
        speed: Float,
        callback: (samples: FloatArray) -> Int,
    ): GeneratedAudio

    /** Generates one native configured audio result. */
    private external fun generateWithConfigImpl(
        ptr: Long,
        text: String,
        config: GenerationConfig,
        callback: ((samples: FloatArray) -> Int)?,
    ): GeneratedAudio
}
