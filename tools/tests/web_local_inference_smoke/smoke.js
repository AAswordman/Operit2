const resultElement = document.getElementById("result");
const asrBundle = "sherpa-onnx-wasm-simd-1.13.2-vad-asr-zh_en-paraformer_small";
const ttsBundle = "sherpa-onnx-wasm-simd-1.13.2-vits-piper-en_US-libritts_r-medium";
const asrDirectory = "runtime/models/local/smoke/asr";
const ttsDirectory = "runtime/models/local/smoke/tts";

/** Loads one fixture into the isolated runtime storage view. */
async function loadFixture(storagePath, fixturePath) {
  const response = await fetch(fixturePath);
  if (!response.ok) {
    throw new Error(`Fixture request failed: ${response.status} ${fixturePath}`);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  globalThis.__operitLocalInferenceTest.putRuntimeFile(storagePath, bytes);
}

/** Loads every Sherpa file required by the Web smoke test. */
async function loadSherpaBundles() {
  for (const fileName of [
    "sherpa-onnx-asr.js",
    "sherpa-onnx-wasm-main-vad-asr.js",
    "sherpa-onnx-wasm-main-vad-asr.wasm",
    "sherpa-onnx-wasm-main-vad-asr.data",
  ]) {
    await loadFixture(
      `${asrDirectory}/${asrBundle}/${fileName}`,
      `/fixtures/asr/${fileName}`,
    );
  }
  for (const fileName of [
    "sherpa-onnx-tts.js",
    "sherpa-onnx-wasm-main-tts.js",
    "sherpa-onnx-wasm-main-tts.wasm",
    "sherpa-onnx-wasm-main-tts.data",
  ]) {
    await loadFixture(
      `${ttsDirectory}/${ttsBundle}/${fileName}`,
      `/fixtures/tts/${fileName}`,
    );
  }
}

/** Runs real Web TTS and feeds its WAV output into real Web STT. */
async function runSmokeTest() {
  if (globalThis.crossOriginIsolated !== true) {
    throw new Error("Smoke test server did not enable cross-origin isolation");
  }
  await loadSherpaBundles();
  await globalThis.__operitLocalInferenceTest.initialize();

  const outputPath = "runtime/temp/clean_on_exit/web-smoke-tts.wav";
  const synthesis = globalThis.__operitLocalInferenceTest.synthesize({
    engineLibraryDirectory: "embedded:sherpa-onnx@1.13.2",
    modelDirectory: ttsDirectory,
    driverJson: JSON.stringify({
      SherpaOnnxWebTtsBundle: {
        ttsScript: `${ttsBundle}/sherpa-onnx-tts.js`,
        runtimeScript: `${ttsBundle}/sherpa-onnx-wasm-main-tts.js`,
        runtimeWasm: `${ttsBundle}/sherpa-onnx-wasm-main-tts.wasm`,
        runtimeData: `${ttsBundle}/sherpa-onnx-wasm-main-tts.data`,
        speakerCount: 904,
      },
    }),
    text: "This is a local speech recognition test.",
    voice: "0",
    speed: 1,
    outputPath,
    optionsJson: "{}",
  });
  if (synthesis.audioPath !== outputPath || synthesis.outputFormat !== "wav") {
    throw new Error(`Unexpected Web TTS response: ${JSON.stringify(synthesis)}`);
  }
  const wav = globalThis.__operitLocalInferenceTest.readRuntimeFile(outputPath);
  if (wav.length <= 44) {
    throw new Error(`Web TTS produced an invalid WAV length: ${wav.length}`);
  }

  const recognition = globalThis.__operitLocalInferenceTest.transcribe({
    engineLibraryDirectory: "embedded:sherpa-onnx@1.13.2",
    modelDirectory: asrDirectory,
    driverJson: JSON.stringify({
      SherpaOnnxWebAsrBundle: {
        recognizerScript: `${asrBundle}/sherpa-onnx-asr.js`,
        runtimeScript: `${asrBundle}/sherpa-onnx-wasm-main-vad-asr.js`,
        runtimeWasm: `${asrBundle}/sherpa-onnx-wasm-main-vad-asr.wasm`,
        runtimeData: `${asrBundle}/sherpa-onnx-wasm-main-vad-asr.data`,
      },
    }),
    audioPath: outputPath,
    language: "en",
    optionsJson: "{}",
  });
  if (typeof recognition.text !== "string" || recognition.text.trim().length === 0) {
    throw new Error(`Web STT returned empty text: ${JSON.stringify(recognition)}`);
  }
  return { recognition: recognition.text, wavBytes: wav.length };
}

try {
  const result = await runSmokeTest();
  resultElement.textContent = `PASS\n${JSON.stringify(result)}`;
} catch (error) {
  const message = error instanceof Error ? error.stack || error.message : String(error);
  resultElement.textContent = `FAIL\n${message}`;
  throw error;
}
