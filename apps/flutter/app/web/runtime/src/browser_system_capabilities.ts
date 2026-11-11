/** Validates the browser prerequisites of the runtime's synchronous Host channels. */
export function requireBrowserSharedMemory(): void {
  if (!globalThis.isSecureContext) {
    throw new Error("Operit2 Web requires HTTPS or localhost. Open https://web.operit.app/ directly.");
  }
  if (globalThis.crossOriginIsolated !== true) {
    throw new Error("Operit2 Web requires cross-origin isolation. Open https://web.operit.app/ directly; " +
      "the server must send Cross-Origin-Opener-Policy: same-origin and Cross-Origin-Embedder-Policy: require-corp.");
  }
  if (typeof globalThis.SharedArrayBuffer !== "function") {
    throw new Error("This browser does not expose SharedArrayBuffer, required by Operit2 Web's Host and plugin runtime. " +
      "Open this page in an up-to-date browser with shared memory enabled.");
  }
}

/** Describes the local Tesseract worker boundary used by the browser host. */
export interface BrowserOcrWorker {
  recognize(image: Blob): Promise<{ data: { text: string } }>;
  terminate(): Promise<unknown>;
}

/** Describes the pinned OCR engine staged with the application bundle. */
export interface BrowserOcrEngine {
  createWorker(language: string, mode: number, options: {
    workerPath: string; corePath: string; langPath: string;
    workerBlobURL: boolean; gzip: boolean;
  }): Promise<BrowserOcrWorker>;
}

/** Reads an actual current browser location without substituting cached coordinates. */
export function readBrowserLocation(timeout: number, highAccuracy: boolean, includeAddress: boolean) {
  if (!Number.isFinite(timeout) || timeout <= 0) {
    throw new Error("Location timeout must be positive");
  }
  if (includeAddress) {
    throw new Error("The browser location host does not provide reverse geocoding");
  }
  if (!globalThis.isSecureContext || !navigator.geolocation) {
    throw new Error("Browser geolocation requires a secure context and the Geolocation API");
  }
  return new Promise<object>((resolve, reject) => {
    navigator.geolocation.getCurrentPosition((position) => {
      const { latitude, longitude, accuracy } = position.coords;
      if (!Number.isFinite(latitude) || !Number.isFinite(longitude) ||
          !Number.isFinite(accuracy) || accuracy < 0) {
        reject(new Error("The browser returned an invalid location"));
        return;
      }
      resolve({ latitude, longitude, accuracy, provider: "browser.geolocation",
        timestamp: position.timestamp, rawData: JSON.stringify({ latitude, longitude, accuracy }),
        address: "", city: "", province: "", country: "" });
    }, (error) => reject(new Error(`Browser location error ${error.code}: ${error.message}`)), {
      enableHighAccuracy: highAccuracy, timeout: timeout * 1000, maximumAge: 0,
    });
  });
}

/** Captures the user-authorized display surface and releases every media track. */
export async function captureBrowserScreen(): Promise<Uint8Array> {
  if (!globalThis.isSecureContext || !navigator.mediaDevices?.getDisplayMedia) {
    throw new Error("Browser screen capture requires a secure context and the Screen Capture API");
  }
  const stream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: false });
  const video = document.createElement("video");
  video.muted = true;
  video.playsInline = true;
  try {
    video.srcObject = stream;
    await video.play();
    await new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("Screen capture produced no video frame")), 15000);
      video.requestVideoFrameCallback(() => { clearTimeout(timer); resolve(); });
    });
    if (video.videoWidth <= 0 || video.videoHeight <= 0) {
      throw new Error("The selected display surface has no valid image dimensions");
    }
    const canvas = document.createElement("canvas");
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;
    const context = canvas.getContext("2d");
    if (context === null) throw new Error("The browser could not create a screen capture canvas");
    context.drawImage(video, 0, 0);
    const blob = await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((value) => {
        if (value === null) reject(new Error("The browser could not encode the screen capture"));
        else resolve(value);
      }, "image/png");
    });
    return new Uint8Array(await blob.arrayBuffer());
  } finally {
    video.pause();
    video.srcObject = null;
    stream.getTracks().forEach((track) => track.stop());
  }
}

/** Runs OCR exclusively with the language and engine assets in the local app bundle. */
export async function recognizeBrowserText(
  bytes: Uint8Array, language: string, quality: string, engine: BrowserOcrEngine, assetRoot: URL,
): Promise<{ text: string }> {
  const languages: Record<string, string> = {
    LATIN: "eng", CHINESE: "chi_sim", JAPANESE: "jpn", KOREAN: "kor",
  };
  const code = languages[language];
  if (code === undefined) throw new Error(`Unsupported OCR language: ${language}`);
  if (quality !== "LOW" && quality !== "HIGH") throw new Error(`Unsupported OCR quality: ${quality}`);
  if (bytes.byteLength === 0) throw new Error("OCR image content is empty");
  const worker = await engine.createWorker(code, 1, {
    workerPath: new URL("worker.min.js", assetRoot).href,
    corePath: new URL("core/", assetRoot).href,
    langPath: new URL("languages", assetRoot).href,
    workerBlobURL: false, gzip: true,
  });
  try {
    const result = await worker.recognize(new Blob([Uint8Array.from(bytes)]));
    if (typeof result.data.text !== "string") throw new Error("The OCR engine returned no text result");
    return { text: result.data.text };
  } finally {
    await worker.terminate();
  }
}
