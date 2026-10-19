// Shared by the trusted preview shell and Studio. No ZIP implementation here.
const MAX_ASSET_BYTES = 128 * 1024 * 1024;
const MAX_TASK_MILLIS = 15_000;

function packAssets(assets) {
  if (!Array.isArray(assets) || assets.length > 24) throw new Error("asset entry budget exceeded");
  const ordered = [...assets].sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  let size = 0;
  const paths = new Set();
  const directory = [];
  for (const asset of ordered) {
    if (typeof asset.path !== "string" || typeof asset.mediaType !== "string" ||
        !(asset.bytes instanceof Uint8Array) || paths.has(asset.path)) {
      throw new Error("invalid binary asset transfer");
    }
    paths.add(asset.path);
    directory.push({ path: asset.path, mediaType: asset.mediaType, offset: size, bytes: asset.bytes.byteLength });
    size += asset.bytes.byteLength;
    if (size > MAX_ASSET_BYTES) throw new Error("asset byte budget exceeded");
  }
  const payload = new Uint8Array(size);
  for (let i = 0; i < ordered.length; i++) payload.set(ordered[i].bytes, directory[i].offset);
  return { assetDirectory: new TextEncoder().encode(JSON.stringify(directory)), assetBytes: payload };
}

export class ThemeContractWorker {
  constructor(workerUrl, rendererReleaseId) {
    if (!/^[A-Za-z0-9._-]{1,128}$/.test(rendererReleaseId)) throw new Error("renderer release ID is required");
    this.rendererReleaseId = rendererReleaseId;
    this.nextRequestId = 1;
    this.pending = new Map();
    this.closed = false;
    this.worker = new Worker(workerUrl, { type: "module", name: "operit-plugin-theme" });
    this.ready = new Promise((resolve, reject) => {
      this.resolveReady = resolve;
      this.rejectReady = reject;
    });
    this.startupTimer = setTimeout(() => this.close(new Error("theme contract startup timed out")), MAX_TASK_MILLIS);
    this.worker.onerror = (event) => this.close(new Error(event.message));
    this.worker.onmessageerror = () => this.close(new Error("theme contract response could not be cloned"));
    this.worker.onmessage = (event) => {
      const response = event.data;
      if (!response || response.rendererReleaseId !== rendererReleaseId) {
        this.close(new Error("theme contract renderer release mismatch"));
        return;
      }
      if (response.type === "ready") {
        if (response.validatorVersion !== 1) {
          this.close(new Error("unsupported theme validator version"));
          return;
        }
        clearTimeout(this.startupTimer);
        this.resolveReady();
        return;
      }
      const pending = this.pending.get(response.requestId);
      if (!pending) return;
      clearTimeout(pending.timer);
      this.pending.delete(response.requestId);
      if (response.ok === true) pending.resolve(response.result);
      else pending.reject(new Error(response.message));
    };
  }

  async call(type, makeFields) {
    await this.ready;
    if (this.closed) throw new Error("theme contract worker is closed");
    if (this.pending.size !== 0) throw new Error("theme contract worker is busy; serialize authoring requests");
    const fields = makeFields();
    const requestId = this.nextRequestId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => this.close(new Error("theme contract task timed out")), MAX_TASK_MILLIS);
      this.pending.set(requestId, { resolve, reject, timer });
      try {
        const buffers = Object.values(fields).filter(value => value instanceof Uint8Array).map(value => value.buffer);
        this.worker.postMessage({ type, requestId, rendererReleaseId: this.rendererReleaseId, ...fields }, buffers);
      } catch (error) {
        this.close(error);
      }
    });
  }

  normalizeDraft(themeJson) {
    return this.call("normalize", () => ({ themeJson: Uint8Array.from(themeJson) }));
  }

  validatePreview(themeJson, assets) {
    return this.call("validate-preview", () => ({ themeJson: Uint8Array.from(themeJson), ...packAssets(assets) }));
  }

  exportPackage(input) {
    return this.call("export", () => ({
      metadataJson: Uint8Array.from(input.metadataJson),
      themeJson: Uint8Array.from(input.themeJson),
      previewPng: Uint8Array.from(input.previewPng),
      ...packAssets(input.assets),
    }));
  }

  /** Validates the final archive through the same bounded worker as template export. */
  validatePackage(archiveBytes, manifestPath) {
    return this.call("validate-toolpkg", () => ({
      archiveBytes: Uint8Array.from(archiveBytes), manifestPath,
    }));
  }

  close(error = new Error("theme contract worker was closed")) {
    if (this.closed) return;
    this.closed = true;
    clearTimeout(this.startupTimer);
    this.worker.terminate();
    this.rejectReady(error);
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }
}

let previewWorker;
function requirePreviewWorker(rendererReleaseId) {
  if (!previewWorker) {
    previewWorker = new ThemeContractWorker(new URL("./theme_contract_worker.js", import.meta.url), rendererReleaseId);
  }
  if (previewWorker.rendererReleaseId !== rendererReleaseId) throw new Error("preview renderer release changed");
  return previewWorker;
}
globalThis.operitThemeContractInitialize = (rendererReleaseId) => requirePreviewWorker(rendererReleaseId).ready;
globalThis.operitThemeContractValidate = async (rendererReleaseId, themeJson, assets) => {
  return JSON.parse(await requirePreviewWorker(rendererReleaseId).validatePreview(new TextEncoder().encode(themeJson), assets));
};
globalThis.operitThemeContractClose = () => { if (previewWorker) previewWorker.close(); };
