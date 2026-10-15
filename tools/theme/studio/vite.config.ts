import { createReadStream } from "node:fs";
import { cp, mkdir, readFile, realpath, stat, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { defineConfig, type Plugin } from "vite";

const repoRoot = fileURLToPath(new URL("../../../", import.meta.url));
const releaseId = process.env.OPERIT_THEME_RENDERER_RELEASE_ID ?? "development-local-28";
if (!/^[A-Za-z0-9._-]{1,128}$/.test(releaseId) || releaseId === "." || releaseId === "..") {
  throw new Error("OPERIT_THEME_RENDERER_RELEASE_ID must be one safe path component");
}
const basePrefix = (process.env.OPERIT_STUDIO_BASE_PREFIX ?? "").replace(/\/+$/, "");
const releaseRoot = path.join(repoRoot, "build", "theme-preview", releaseId);
const releasePrefix = `${basePrefix}/theme-preview/${releaseId}/`;
const requiredFiles = [
  "index.html", "flutter_bootstrap.js", "main.dart.js",
  "theme_contract_client.js", "theme_contract_worker.js",
  "validator/operit_plugin_theme.js", "validator/operit_plugin_theme_bg.wasm",
];
function setIsolationHeaders(res: import("node:http").ServerResponse): void {
  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  res.setHeader("Cross-Origin-Resource-Policy", "same-origin");
}

let verifiedRelease: Promise<{ ready: boolean; error?: string }> | null = null;
async function checkRelease(): Promise<{ ready: boolean; error?: string; rendererCommit?: string; flutterRevision?: string; fixtureSourceSha256?: string; sourceDirty?: boolean }> {
  try {
    const descriptor = JSON.parse(await readFile(path.join(releaseRoot, "renderer-release.json"), "utf8"));
    if (descriptor.rendererReleaseId !== releaseId || descriptor.validatorVersion !== 1 ||
        !/^[a-f0-9]{40}$/.test(descriptor.coreCommit) || !/^[a-f0-9]{40}$/.test(descriptor.flutterRevision) ||
        !/^[a-f0-9]{64}$/.test(descriptor.fixtureSourceSha256) ||
        typeof descriptor.sourceDirty !== "boolean") {
      throw new Error("renderer-release.json does not match the selected contract release");
    }
    if (!Array.isArray(descriptor.files) || descriptor.files.length < requiredFiles.length ||
        requiredFiles.some(name => !descriptor.files.some((row: unknown) =>
          row !== null && typeof row === "object" && (row as { path?: string }).path === name))) {
      throw new Error("renderer release descriptor has no complete asset inventory");
    }
    const resolvedRoot = await realpath(releaseRoot);
    const seen = new Set<string>();
    for (const row of descriptor.files) {
      if (row === null || typeof row !== "object" || typeof row.path !== "string" ||
          !/^[a-f0-9]{64}$/.test(row.sha256) || !Number.isSafeInteger(row.bytes) || row.bytes < 0 ||
          row.path.split("/").some((part: string) => part === "" || part === "." || part === "..") || row.path.includes("\\")) {
        throw new Error("renderer release descriptor contains an invalid asset entry");
      }
      if (seen.has(row.path)) throw new Error("renderer release descriptor repeats an asset path");
      seen.add(row.path);
      const target = await realpath(path.join(releaseRoot, row.path));
      if (!target.startsWith(resolvedRoot + path.sep)) throw new Error("renderer asset leaves the release directory");
      const bytes = await readFile(target);
      if (bytes.byteLength !== row.bytes || createHash("sha256").update(bytes).digest("hex") !== row.sha256) {
        throw new Error(`renderer asset digest mismatch: ${row.path}`);
      }
    }
    const html = await readFile(path.join(releaseRoot, "index.html"), "utf8");
    if (!html.includes(`<base href="${releasePrefix}">`)) {
      throw new Error(`Flutter preview must be built with --base-href=${releasePrefix}`);
    }
    return { ready: true, rendererCommit: descriptor.coreCommit, flutterRevision: descriptor.flutterRevision,
      fixtureSourceSha256: descriptor.fixtureSourceSha256,
      sourceDirty: descriptor.sourceDirty };
  } catch (error) {
    return { ready: false, error: `Renderer ${releaseId} is unavailable: ${String(error)}. Build it with tools/theme/build_preview.py before preview/export.` };
  }
}
async function releaseState(): Promise<{ ready: boolean; error?: string; rendererCommit?: string; flutterRevision?: string; fixtureSourceSha256?: string; sourceDirty?: boolean }> {
  if (verifiedRelease === null) {
    const pending = checkRelease();
    verifiedRelease = pending;
    pending.then(state => { if (!state.ready) verifiedRelease = null; });
  }
  return verifiedRelease;
}

function localRenderer(): Plugin {
  const mime: Record<string, string> = {
    ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8",
    ".json": "application/json; charset=utf-8", ".wasm": "application/wasm",
    ".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".svg": "image/svg+xml",
    ".ttf": "font/ttf", ".otf": "font/otf", ".bin": "application/octet-stream",
    ".frag": "text/plain; charset=utf-8", ".symbols": "text/plain; charset=utf-8",
  };
  const install = (middlewares: { use: (handler: (req: import("node:http").IncomingMessage, res: import("node:http").ServerResponse, next: (error?: unknown) => void) => void) => void }) => {
    middlewares.use((req, res, next) => {
      if (req.method !== "GET" && req.method !== "HEAD") return next();
      const pathname = new URL(req.url ?? "/", "http://127.0.0.1").pathname;
      if (pathname === "/theme-studio-config.json") {
        void releaseState().then(state => {
          setIsolationHeaders(res);
          res.setHeader("Content-Type", "application/json; charset=utf-8");
          res.setHeader("Cache-Control", "no-store");
          res.end(JSON.stringify({ ...state, rendererReleaseId: releaseId, fixtureVersion: "client-shell-v7", previewUrl: releasePrefix,
            workerUrl: `${releasePrefix}theme_contract_worker.js` }));
        }).catch(next);
        return;
      }
      if (!pathname.startsWith(releasePrefix)) return next();
      void (async () => {
        const state = await releaseState();
        if (!state.ready) {
          res.statusCode = 503;
          res.end(state.error);
          return;
        }
        const relative = decodeURIComponent(pathname.slice(releasePrefix.length)) || "index.html";
        if (relative.split("/").some(part => part === "." || part === ".." || part === "") ||
            relative.includes("\\") || relative.startsWith("/")) {
          res.statusCode = 400;
          res.end("Invalid preview path");
          return;
        }
        const target = path.resolve(releaseRoot, relative);
        if (!target.startsWith(releaseRoot + path.sep)) {
          res.statusCode = 403;
          res.end("Preview path leaves release directory");
          return;
        }
        const realTarget = await realpath(target);
        const realRoot = await realpath(releaseRoot);
        if (!realTarget.startsWith(realRoot + path.sep)) throw new Error("preview asset leaves release directory");
        const entry = await stat(realTarget);
        if (!entry.isFile()) throw new Error("not a preview asset");
        setIsolationHeaders(res);
        res.setHeader("Content-Type", mime[path.extname(target)] ?? "application/octet-stream");
        res.setHeader("Cache-Control", "no-store");
        res.setHeader("X-Content-Type-Options", "nosniff");
        res.setHeader("Cross-Origin-Resource-Policy", "same-origin");
        res.setHeader("Content-Length", entry.size);
        if (req.method === "HEAD") { res.end(); return; }
        createReadStream(realTarget).pipe(res);
      })().catch(() => {
        if (!res.headersSent) {
          res.statusCode = 404;
          res.end("Preview asset not found");
        } else {
          res.destroy();
        }
      });
    });
  };
  return {
    name: "operit-local-renderer",
    configureServer: server => install(server.middlewares),
    configurePreviewServer: server => install(server.middlewares),
    async closeBundle() {
      const state = await releaseState();
      if (!state.ready) {
        throw new Error(state.error ?? `Renderer ${releaseId} is not ready for static build`);
      }
      const distDir = path.resolve(fileURLToPath(new URL(".", import.meta.url)), "dist");
      await mkdir(distDir, { recursive: true });
      const staticConfig = {
        ...state,
        rendererReleaseId: releaseId,
        fixtureVersion: "client-shell-v7",
        previewUrl: `./theme-preview/${releaseId}/`,
        workerUrl: `./theme-preview/${releaseId}/theme_contract_worker.js`,
      };
      await writeFile(
        path.join(distDir, "theme-studio-config.json"),
        JSON.stringify(staticConfig, null, 2) + "\n",
        "utf8",
      );
      await writeFile(path.join(distDir, ".nojekyll"), "", "utf8");
      const destReleaseDir = path.join(distDir, "theme-preview", releaseId);
      await cp(releaseRoot, destReleaseDir, { recursive: true });
    },
  };
}

export default defineConfig({
  base: "./",
  plugins: [localRenderer()],
  server: { host: "127.0.0.1", port: 7474, strictPort: true,
    fs: { allow: [repoRoot] },
    headers: { "Cross-Origin-Opener-Policy": "same-origin", "Cross-Origin-Embedder-Policy": "require-corp" } },
  preview: { host: "127.0.0.1", port: 7474, strictPort: true },
  build: { target: "es2022", sourcemap: true },
});
