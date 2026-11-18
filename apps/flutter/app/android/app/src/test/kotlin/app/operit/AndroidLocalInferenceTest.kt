package app.operit

import com.k2fsa.sherpa.onnx.GeneratedAudio
import com.k2fsa.sherpa.onnx.OfflineTts
import java.io.File
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.mockito.Mockito.*

/** Exercises Android TTS speaker and output contracts without loading native model weights. */
class AndroidLocalInferenceTest {
    @get:Rule
    val temporary = TemporaryFolder()

    /** Supplies the seven declared Matcha files and an already loaded mock engine. */
    private fun withMatchaRequest(test: (JSONObject) -> Unit) {
        val engine = temporary.newFolder("engine")
        val model = temporary.newFolder("matcha")
        for (name in listOf("model-steps-3.onnx", "vocos-22khz-univ.onnx", "lexicon.txt",
            "tokens.txt", "phone.fst", "number.fst", "date.fst")) {
            File(model, name).writeText("fixture")
        }
        val driver = JSONObject()
            .put("acousticModel", "model-steps-3.onnx")
            .put("vocoder", "vocos-22khz-univ.onnx")
            .put("lexicon", "lexicon.txt")
            .put("tokens", "tokens.txt")
            .put("ruleFsts", JSONArray(listOf("phone.fst", "number.fst", "date.fst")))
            .put("ruleFars", JSONArray())
            .put("speakerCount", 1)
        val request = JSONObject()
            .put("engineLibraryDirectory", engine.path)
            .put("modelDirectory", model.path)
            .put("driverJson", JSONObject().put("SherpaOnnxMatcha", driver).toString())
            .put("text", "你好")
            .put("voice", "0")
            .put("speed", 1.0)
            .put("optionsJson", "{}")
        val loaded = AndroidLocalInference::class.java.getDeclaredField("loadedEngineDirectory")
        loaded.isAccessible = true
        synchronized(AndroidLocalInference) {
            val previous = loaded.get(null)
            loaded.set(null, engine.canonicalPath)
            try {
                test(request)
            } finally {
                loaded.set(null, previous)
            }
        }
    }

    /** Supplies the AISHELL-3 files with the speaker count embedded in the pinned ONNX model. */
    private fun withAishell3Request(test: (JSONObject) -> Unit) = withMatchaRequest { request ->
        File(request.getString("modelDirectory"), "vits-aishell3.int8.onnx").writeText("fixture")
        val driver = JSONObject()
            .put("model", "vits-aishell3.int8.onnx")
            .put("lexicon", "lexicon.txt")
            .put("tokens", "tokens.txt")
            .put("ruleFsts", JSONArray())
            .put("ruleFars", JSONArray())
            .put("speakerCount", 174)
        request.put("driverJson", JSONObject().put("SherpaOnnxVits", driver).toString())
        request.put("outputPath", File(temporary.root, "aishell3.wav").path)
        test(request)
    }

    /** Accepts the reported speaker and the final AISHELL-3 speaker with exact engine metadata. */
    @Test
    fun aishell3SynthesizesValidSpeakers() = withAishell3Request { request ->
        val generated = mock(GeneratedAudio::class.java)
        `when`(generated.samples).thenReturn(floatArrayOf(0.1f))
        `when`(generated.sampleRate).thenReturn(8000)
        `when`(generated.save(anyString())).thenAnswer {
            File(it.getArgument<String>(0)).writeBytes(ByteArray(48))
            true
        }
        mockConstruction(OfflineTts::class.java) { tts, _ ->
            `when`(tts.numSpeakers()).thenReturn(174)
            `when`(tts.generate(anyString(), anyInt(), anyFloat())).thenReturn(generated)
        }.use { engines ->
            for (speaker in listOf(1, 173)) {
                request.put("voice", speaker.toString())
                request.put("outputPath", File(temporary.root, "speaker-$speaker.wav").path)
                val response = JSONObject(AndroidLocalInference.synthesize(request.toString()))
                assertEquals(request.getString("outputPath"), response.getString("audioPath"))
                verify(engines.constructed().last()).generate("你好", speaker, 1.0f)
                verify(engines.constructed().last()).release()
            }
        }
    }

    /** Rejects out-of-range IDs before native allocation and still rejects inconsistent models. */
    @Test
    fun aishell3RejectsInvalidSpeakersAndMismatchedModels() = withAishell3Request { request ->
        mockConstruction(OfflineTts::class.java) { tts, _ ->
            `when`(tts.numSpeakers()).thenReturn(175)
        }.use { engines ->
            for (speaker in listOf(-1, 174)) {
                request.put("voice", speaker.toString())
                assertThrows(IllegalArgumentException::class.java) {
                    AndroidLocalInference.synthesize(request.toString())
                }
            }
            assertTrue(engines.constructed().isEmpty())
            request.put("voice", "1")
            val error = assertThrows(IllegalArgumentException::class.java) {
                AndroidLocalInference.synthesize(request.toString())
            }
            assertTrue(error.message!!.contains("speaker count does not match"))
            verify(engines.constructed().single(), never()).generate(anyString(), anyInt(), anyFloat())
            verify(engines.constructed().single()).release()
            assertFalse(File(request.getString("outputPath")).exists())
        }
    }

    /** Returns the selected capability path even when canonicalization changes its spelling. */
    @Test
    fun synthesisPreservesRequestedOutputPath() = withMatchaRequest { request ->
        val outputDirectory = temporary.newFolder("output")
        val alias = File(outputDirectory, "alias").apply { mkdir() }
        val generated = mock(GeneratedAudio::class.java)
        `when`(generated.samples).thenReturn(floatArrayOf(0.1f, 0.2f))
        `when`(generated.sampleRate).thenReturn(22050)
        `when`(generated.save(anyString())).thenAnswer {
            File(it.getArgument<String>(0)).writeBytes(ByteArray(48))
            true
        }
        mockConstruction(OfflineTts::class.java) { tts, _ ->
            `when`(tts.numSpeakers()).thenReturn(1)
            `when`(tts.generate(anyString(), anyInt(), anyFloat())).thenReturn(generated)
        }.use { engines ->
            val canonical = File(outputDirectory, "canonical.wav").canonicalPath
            val aliased = "${alias.path}/../aliased.wav"
            assertNotEquals(aliased, File(aliased).canonicalPath)
            for (selected in listOf(canonical, aliased)) {
                request.put("outputPath", selected)
                val response = JSONObject(AndroidLocalInference.synthesize(request.toString()))
                assertEquals(selected, response.getString("audioPath"))
                assertEquals("wav", response.getString("outputFormat"))
                assertEquals(48L, File(selected).length())
                verify(generated).save(File(selected).canonicalPath)
            }
            assertEquals(2, engines.constructed().size)
            engines.constructed().forEach { verify(it).release() }
        }
    }

    /** Rejects a failed native write instead of reporting the selected path as a success. */
    @Test
    fun failedWriteDoesNotReturnAnOutputCapability() = withMatchaRequest { request ->
        request.put("outputPath", File(temporary.root, "failed.wav").path)
        val generated = mock(GeneratedAudio::class.java)
        `when`(generated.samples).thenReturn(floatArrayOf(0.1f))
        `when`(generated.sampleRate).thenReturn(22050)
        `when`(generated.save(anyString())).thenReturn(false)
        mockConstruction(OfflineTts::class.java) { tts, _ ->
            `when`(tts.numSpeakers()).thenReturn(1)
            `when`(tts.generate(anyString(), anyInt(), anyFloat())).thenReturn(generated)
        }.use { engines ->
            val error = assertThrows(IllegalArgumentException::class.java) {
                AndroidLocalInference.synthesize(request.toString())
            }
            assertTrue(error.message!!.startsWith("LOCAL_MODEL TTS failed to write "))
            verify(engines.constructed().single()).release()
        }
    }

    /** Keeps existing-file and missing-parent checks ahead of native TTS allocation. */
    @Test
    fun invalidOutputsAreRejectedBeforeGeneration() = withMatchaRequest { request ->
        val existing = temporary.newFile("existing.wav")
        existing.writeText("preserve")
        mockConstruction(OfflineTts::class.java).use { engines ->
            for (output in listOf(existing, File(temporary.root, "missing/output.wav"))) {
                request.put("outputPath", output.path)
                assertThrows(IllegalArgumentException::class.java) {
                    AndroidLocalInference.synthesize(request.toString())
                }
            }
            assertEquals("preserve", existing.readText())
            assertTrue(engines.constructed().isEmpty())
        }
    }
}
