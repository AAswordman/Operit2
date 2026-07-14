package app.operit

import com.k2fsa.sherpa.onnx.FeatureConfig
import com.k2fsa.sherpa.onnx.OfflineTts
import com.k2fsa.sherpa.onnx.OfflineTtsConfig
import com.k2fsa.sherpa.onnx.OfflineTtsKittenModelConfig
import com.k2fsa.sherpa.onnx.OfflineTtsMatchaModelConfig
import com.k2fsa.sherpa.onnx.OfflineTtsModelConfig
import com.k2fsa.sherpa.onnx.OfflineTtsVitsModelConfig
import com.k2fsa.sherpa.onnx.OnlineModelConfig
import com.k2fsa.sherpa.onnx.OnlineRecognizer
import com.k2fsa.sherpa.onnx.OnlineRecognizerConfig
import com.k2fsa.sherpa.onnx.OnlineTransducerModelConfig
import com.k2fsa.sherpa.onnx.WaveReader
import java.io.File
import org.json.JSONArray
import org.json.JSONObject

internal object AndroidLocalInference {
    private val loadLock = Any()
    private var loadedEngineDirectory: String? = null

    private data class TaggedDriver(val name: String, val payload: JSONObject)

    /** Transcribes one WAV request with the exact installed Sherpa ONNX model. */
    fun transcribe(requestJson: String): String {
        val request = JSONObject(requestJson)
        validateOptions(request.getString("optionsJson"))
        val engineDirectory = requiredDirectory(request.getString("engineLibraryDirectory"))
        loadEngine(engineDirectory)
        val modelDirectory = requiredDirectory(request.getString("modelDirectory"))
        val driver = requiredDriver(request.getString("driverJson"), "SherpaOnnxStreamingTransducer")
        val audioPath = requiredFile(File(request.getString("audioPath")))
        val wave = WaveReader.readWaveFromFile(audioPath.absolutePath)
        require(wave.sampleRate > 0) { "LOCAL_MODEL STT WAV sample rate must be positive" }
        require(wave.samples.isNotEmpty()) { "LOCAL_MODEL STT WAV contains no samples" }

        val recognizer = OnlineRecognizer(
            config = OnlineRecognizerConfig(
                featConfig = FeatureConfig(sampleRate = wave.sampleRate, featureDim = 80),
                modelConfig = OnlineModelConfig(
                    transducer = OnlineTransducerModelConfig(
                        encoder = modelFile(modelDirectory, driver.getString("encoder")).absolutePath,
                        decoder = modelFile(modelDirectory, driver.getString("decoder")).absolutePath,
                        joiner = modelFile(modelDirectory, driver.getString("joiner")).absolutePath,
                    ),
                    tokens = modelFile(modelDirectory, driver.getString("tokens")).absolutePath,
                    numThreads = runtimeThreadCount(),
                    debug = false,
                    provider = "cpu",
                    modelType = driver.getString("modelType"),
                ),
                enableEndpoint = false,
                decodingMethod = "greedy_search",
            ),
        )
        val stream = recognizer.createStream()
        try {
            stream.acceptWaveform(wave.samples, wave.sampleRate)
            stream.inputFinished()
            while (recognizer.isReady(stream)) {
                recognizer.decode(stream)
            }
            val result = recognizer.getResult(stream)
            val details = JSONObject()
                .put("tokens", jsonArray(result.tokens.asIterable()))
                .put("timestamps", jsonArray(result.timestamps.asIterable()))
                .put("language", request.optString("language", ""))
            return JSONObject()
                .put("text", result.text)
                .put("resultJson", details.toString())
                .toString()
        } finally {
            stream.release()
            recognizer.release()
        }
    }

    /** Synthesizes one WAV request with the exact installed Sherpa ONNX model. */
    fun synthesize(requestJson: String): String {
        val request = JSONObject(requestJson)
        validateOptions(request.getString("optionsJson"))
        val engineDirectory = requiredDirectory(request.getString("engineLibraryDirectory"))
        loadEngine(engineDirectory)
        val modelDirectory = requiredDirectory(request.getString("modelDirectory"))
        val taggedDriver = requiredTaggedDriver(request.getString("driverJson"))
        val driver = taggedDriver.payload
        val speed = request.getDouble("speed")
        require(speed.isFinite() && speed > 0.0) { "LOCAL_MODEL TTS speed must be finite and positive" }
        val speaker = request.getString("voice").toInt()
        val speakerCount = driver.getInt("speakerCount")
        require(speaker in 0 until speakerCount) {
            "LOCAL_MODEL TTS speaker must be between 0 and ${speakerCount - 1}"
        }
        val outputPath = requiredOutputFile(request.getString("outputPath"))
        val tts = OfflineTts(
            config = ttsConfigForDriver(taggedDriver.name, driver, modelDirectory),
        )
        try {
            require(tts.numSpeakers() == speakerCount) {
                "LOCAL_MODEL TTS speaker count does not match the installed model manifest"
            }
            val generated = tts.generate(
                text = request.getString("text"),
                sid = speaker,
                speed = speed.toFloat(),
            )
            require(generated.samples.isNotEmpty()) { "LOCAL_MODEL TTS generated no audio samples" }
            require(generated.sampleRate > 0) { "LOCAL_MODEL TTS generated an invalid sample rate" }
            require(generated.save(outputPath.absolutePath)) {
                "LOCAL_MODEL TTS failed to write ${outputPath.absolutePath}"
            }
            require(outputPath.isFile && outputPath.length() > 44L) {
                "LOCAL_MODEL TTS output WAV is invalid: ${outputPath.absolutePath}"
            }
            return JSONObject()
                .put("audioPath", outputPath.absolutePath)
                .put("outputFormat", "wav")
                .toString()
        } finally {
            tts.release()
        }
    }

    /** Builds one Sherpa ONNX TTS config from an exact driver tag. */
    private fun ttsConfigForDriver(driverName: String, driver: JSONObject, modelDirectory: File): OfflineTtsConfig {
        return when (driverName) {
            "SherpaOnnxVits" => vitsTtsConfig(driver, modelDirectory)
            "SherpaOnnxMatcha" => matchaTtsConfig(driver, modelDirectory)
            "SherpaOnnxKitten" => kittenTtsConfig(driver, modelDirectory)
            else -> throw IllegalArgumentException("LOCAL_MODEL Android TTS driver is unsupported: $driverName")
        }
    }

    /** Builds one VITS TTS config from manifest-declared files. */
    private fun vitsTtsConfig(driver: JSONObject, modelDirectory: File): OfflineTtsConfig {
        return OfflineTtsConfig(
            model = OfflineTtsModelConfig(
                vits = OfflineTtsVitsModelConfig(
                    model = modelFile(modelDirectory, driver.getString("model")).absolutePath,
                    lexicon = modelFile(modelDirectory, driver.getString("lexicon")).absolutePath,
                    tokens = modelFile(modelDirectory, driver.getString("tokens")).absolutePath,
                ),
                numThreads = runtimeThreadCount(),
                debug = false,
                provider = "cpu",
            ),
            ruleFsts = modelFileList(modelDirectory, driver.getJSONArray("ruleFsts")),
            ruleFars = modelFileList(modelDirectory, driver.getJSONArray("ruleFars")),
        )
    }

    /** Builds one Matcha TTS config from manifest-declared files. */
    private fun matchaTtsConfig(driver: JSONObject, modelDirectory: File): OfflineTtsConfig {
        return OfflineTtsConfig(
            model = OfflineTtsModelConfig(
                matcha = OfflineTtsMatchaModelConfig(
                    acousticModel = modelFile(modelDirectory, driver.getString("acousticModel")).absolutePath,
                    vocoder = modelFile(modelDirectory, driver.getString("vocoder")).absolutePath,
                    lexicon = modelFile(modelDirectory, driver.getString("lexicon")).absolutePath,
                    tokens = modelFile(modelDirectory, driver.getString("tokens")).absolutePath,
                ),
                numThreads = runtimeThreadCount(),
                debug = false,
                provider = "cpu",
            ),
            ruleFsts = modelFileList(modelDirectory, driver.getJSONArray("ruleFsts")),
            ruleFars = modelFileList(modelDirectory, driver.getJSONArray("ruleFars")),
        )
    }

    /** Builds one Kitten TTS config from manifest-declared files. */
    private fun kittenTtsConfig(driver: JSONObject, modelDirectory: File): OfflineTtsConfig {
        return OfflineTtsConfig(
            model = OfflineTtsModelConfig(
                kitten = OfflineTtsKittenModelConfig(
                    model = modelFile(modelDirectory, driver.getString("model")).absolutePath,
                    voices = modelFile(modelDirectory, driver.getString("voices")).absolutePath,
                    tokens = modelFile(modelDirectory, driver.getString("tokens")).absolutePath,
                    dataDir = modelDirectory(modelDirectory, driver.getString("dataDir")).absolutePath,
                ),
                numThreads = runtimeThreadCount(),
                debug = false,
                provider = "cpu",
            ),
        )
    }

    /** Loads one installed Sherpa ONNX engine directory into the Android process. */
    private fun loadEngine(directory: File) {
        val canonicalDirectory = directory.canonicalPath
        synchronized(loadLock) {
            when (val loaded = loadedEngineDirectory) {
                null -> {
                    System.load(requiredFile(File(directory, "libonnxruntime.so")).absolutePath)
                    System.load(requiredFile(File(directory, "libsherpa-onnx-c-api.so")).absolutePath)
                    System.load(requiredFile(File(directory, "libsherpa-onnx-jni.so")).absolutePath)
                    loadedEngineDirectory = canonicalDirectory
                }
                canonicalDirectory -> Unit
                else -> throw IllegalStateException(
                    "Sherpa ONNX is already loaded from a different engine directory: $loaded",
                )
            }
        }
    }

    /** Parses and returns one exact externally tagged local model driver. */
    private fun requiredDriver(driverJson: String, driverName: String): JSONObject {
        val root = JSONObject(driverJson)
        require(root.length() == 1 && root.has(driverName)) {
            "LOCAL_MODEL Android driver must be $driverName"
        }
        return root.getJSONObject(driverName)
    }

    /** Parses one externally tagged driver and returns its tag and payload. */
    private fun requiredTaggedDriver(driverJson: String): TaggedDriver {
        val root = JSONObject(driverJson)
        val keys = root.keys()
        require(keys.hasNext()) { "LOCAL_MODEL Android driver tag is missing" }
        val driverName = keys.next()
        require(!keys.hasNext()) { "LOCAL_MODEL Android driver must contain exactly one tag" }
        return TaggedDriver(driverName, root.getJSONObject(driverName))
    }

    /** Validates the structured local inference options object. */
    private fun validateOptions(optionsJson: String) {
        JSONObject(optionsJson)
    }

    /** Returns one verified canonical directory. */
    private fun requiredDirectory(path: String): File {
        val directory = File(path).canonicalFile
        require(directory.isDirectory) { "Required local model directory is missing: $directory" }
        return directory
    }

    /** Returns one verified canonical input file. */
    private fun requiredFile(file: File): File {
        val canonicalFile = file.canonicalFile
        require(canonicalFile.isFile) { "Required local model file is missing: $canonicalFile" }
        return canonicalFile
    }

    /** Resolves one model file while enforcing the installed model directory boundary. */
    private fun modelFile(modelDirectory: File, relativePath: String): File {
        require(relativePath.isNotBlank()) { "Local model file path is empty" }
        val file = File(modelDirectory, relativePath).canonicalFile
        require(file.toPath().startsWith(modelDirectory.canonicalFile.toPath())) {
            "Local model file escapes its installation directory: $relativePath"
        }
        return requiredFile(file)
    }

    /** Resolves one model directory while enforcing the installed model directory boundary. */
    private fun modelDirectory(modelDirectory: File, relativePath: String): File {
        require(relativePath.isNotBlank()) { "Local model directory path is empty" }
        val directory = File(modelDirectory, relativePath).canonicalFile
        require(directory.toPath().startsWith(modelDirectory.canonicalFile.toPath())) {
            "Local model directory escapes its installation directory: $relativePath"
        }
        require(directory.isDirectory) { "Required local model directory is missing: $directory" }
        return directory
    }

    /** Resolves a comma-separated list of declared model files. */
    private fun modelFileList(modelDirectory: File, paths: JSONArray): String {
        val files = ArrayList<String>(paths.length())
        for (index in 0 until paths.length()) {
            files.add(modelFile(modelDirectory, paths.getString(index)).absolutePath)
        }
        return files.joinToString(",")
    }

    /** Returns one canonical writable output file path. */
    private fun requiredOutputFile(path: String): File {
        val file = File(path).canonicalFile
        val parent = file.parentFile
            ?: throw IllegalArgumentException("LOCAL_MODEL TTS output path has no parent")
        require(parent.isDirectory) { "LOCAL_MODEL TTS output directory is missing: $parent" }
        require(!file.exists()) { "LOCAL_MODEL TTS output file already exists: $file" }
        return file
    }

    /** Returns a bounded native inference thread count for this device. */
    private fun runtimeThreadCount(): Int = Runtime.getRuntime().availableProcessors().coerceIn(1, 4)

    /** Encodes values as a JSON array without string coercion. */
    private fun jsonArray(values: Iterable<*>): JSONArray {
        val array = JSONArray()
        for (value in values) {
            array.put(value)
        }
        return array
    }
}
