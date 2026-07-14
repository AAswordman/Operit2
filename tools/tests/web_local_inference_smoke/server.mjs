import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, "../../..");
const bridgePath = join(repositoryRoot, "apps/web_access/web/operit_runtime_bridge.js");
const fixtureRoot = resolve(process.argv[2]);
const port = Number.parseInt(process.argv[3] || "18765", 10);
const asrRoot = join(
  fixtureRoot,
  "sherpa-onnx-wasm-simd-1.13.2-vad-asr-zh_en-paraformer_small",
);
const ttsRoot = join(
  fixtureRoot,
  "sherpa-onnx-wasm-simd-1.13.2-vits-piper-en_US-libritts_r-medium",
);

/** Returns the content type for one smoke-test asset. */
function contentType(path) {
  const extension = extname(path);
  if (extension === ".html") return "text/html; charset=utf-8";
  if (extension === ".js" || extension === ".mjs") return "text/javascript; charset=utf-8";
  if (extension === ".wasm") return "application/wasm";
  return "application/octet-stream";
}

/** Resolves one request URL to an exact smoke-test file. */
function requestFile(url) {
  const path = new URL(url, "http://127.0.0.1").pathname;
  if (path === "/") return join(scriptDirectory, "index.html");
  if (path === "/smoke.js") return join(scriptDirectory, "smoke.js");
  if (path === "/bridge.js") return bridgePath;
  const segments = path.split("/").filter((segment) => segment.length > 0);
  if (segments.length === 3 && segments[0] === "fixtures" && segments[1] === "asr") {
    return join(asrRoot, segments[2]);
  }
  if (segments.length === 3 && segments[0] === "fixtures" && segments[1] === "tts") {
    return join(ttsRoot, segments[2]);
  }
  throw new Error(`Unknown smoke-test asset: ${path}`);
}

/** Writes one isolated static response required by threaded WebAssembly. */
function sendFile(response, path) {
  const size = statSync(path).size;
  response.writeHead(200, {
    "Content-Type": contentType(path),
    "Content-Length": size,
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Embedder-Policy": "require-corp",
    "Cross-Origin-Resource-Policy": "same-origin",
    "Cache-Control": "no-store",
  });
  createReadStream(path).pipe(response);
}

/** Handles one smoke-test HTTP request. */
function handleRequest(request, response) {
  try {
    sendFile(response, requestFile(request.url || "/"));
  } catch (error) {
    response.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
    response.end(error instanceof Error ? error.message : String(error));
  }
}

const server = createServer(handleRequest);

/** Reports the exact smoke-test address after the listener starts. */
function reportListening() {
  console.log(`READY http://127.0.0.1:${port}`);
}

server.listen(port, "127.0.0.1", reportListening);
