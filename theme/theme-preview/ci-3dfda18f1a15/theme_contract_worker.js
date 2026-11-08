// Trusted application code, not code supplied in a theme. Deploy this file,
// the release descriptor and the wasm-bindgen artifact as one immutable release.
import initialize, {
  normalizeThemeDraft,
  validateThemePreviewDraft,
  exportToolPkgFromTheme,
  validateAuthoredToolPkg,
} from "./validator/operit_plugin_theme.js";

const descriptorResponse = await fetch(new URL("./renderer-release.json", import.meta.url));
if (!descriptorResponse.ok) throw new Error("renderer release descriptor is unavailable");
const descriptor = await descriptorResponse.json();
if (typeof descriptor.rendererReleaseId !== "string" ||
    !/^[A-Za-z0-9._-]{1,128}$/.test(descriptor.rendererReleaseId) ||
    descriptor.validatorVersion !== 1) {
  throw new Error("renderer release descriptor is invalid");
}
await initialize();

function bytes(value, limit, name) {
  if (!(value instanceof Uint8Array) || value.byteLength > limit) {
    throw new Error(`${name} must be bounded binary bytes`);
  }
  return value;
}

self.onmessage = (event) => {
  const request = event.data;
  if (!request || !Number.isSafeInteger(request.requestId) || request.requestId < 1) return;
  try {
    if (request.rendererReleaseId !== descriptor.rendererReleaseId) {
      throw new Error("contract worker belongs to another renderer release");
    }
    let result;
    switch (request.type) {
      case "normalize":
        result = normalizeThemeDraft(bytes(request.themeJson, 256 * 1024, "themeJson"));
        break;
      case "validate-preview":
        result = validateThemePreviewDraft(
          bytes(request.themeJson, 256 * 1024, "themeJson"),
          bytes(request.assetDirectory, 64 * 1024, "assetDirectory"),
          bytes(request.assetBytes, 128 * 1024 * 1024, "assetBytes"),
        );
        break;
      case "export":
        result = exportToolPkgFromTheme(
          bytes(request.metadataJson, 64 * 1024 + 25 * 128 * 1024, "metadataJson"),
          bytes(request.themeJson, 256 * 1024, "themeJson"),
          bytes(request.previewPng, 4 * 1024 * 1024, "previewPng"),
          bytes(request.assetDirectory, 64 * 1024, "assetDirectory"),
          bytes(request.assetBytes, 128 * 1024 * 1024, "assetBytes"),
        );
        break;
      case "validate-toolpkg":
        if (typeof request.manifestPath !== "string") throw new Error("manifestPath must be text");
        result = validateAuthoredToolPkg(
          bytes(request.archiveBytes, 64 * 1024 * 1024, "archiveBytes"),
          request.manifestPath,
        );
        break;
      default:
        throw new Error("unregistered contract worker operation");
    }
    const response = {
      requestId: request.requestId, rendererReleaseId: descriptor.rendererReleaseId,
      ok: true, result,
    };
    if (result instanceof Uint8Array) {
      self.postMessage(response, [result.buffer]);
    } else {
      self.postMessage(response);
    }
  } catch (error) {
    self.postMessage({
      requestId: request.requestId, rendererReleaseId: descriptor.rendererReleaseId,
      ok: false, code: "theme-contract-operation-failed", message: String(error),
    });
  }
};
self.postMessage({ type: "ready", ...descriptor });
