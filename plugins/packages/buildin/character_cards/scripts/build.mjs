import { createRequire } from "node:module";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { checkProject } from "./check.mjs";
const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const { build } = require("esbuild");
const { zipSync } = require("fflate");
const ts = require("typescript");
const root = fileURLToPath(new URL("../", import.meta.url));

/** Bundles current typed browser sources into the sole offline IIFE used by the WebView. */
export async function buildBrowserScript() {
  const result = await build({ absWorkingDir: root, entryPoints: [path.join(root, "web/app.ts")], tsconfig: path.join(root, "tsconfig.web.json"), bundle: true, format: "iife", platform: "browser", target: "es2020", write: false, legalComments: "none" });
  if (result.outputFiles.length !== 1) throw new Error("The offline editor must have exactly one browser script");
  return result.outputFiles[0].text;
}

/** Bundles the real sidebar entry, including its native bridge startup. */
export async function buildSidebarScript() {
  const result = await build({ absWorkingDir: root, entryPoints: [path.join(root, "web/sidebar.ts")], tsconfig: path.join(root, "tsconfig.web.json"), bundle: true, format: "iife", platform: "browser", target: "es2020", write: false, legalComments: "none" });
  if (result.outputFiles.length !== 1) throw new Error("The offline sidebar must have exactly one browser script");
  return result.outputFiles[0].text;
}

/** Produces the installed sidebar document from the same typed source as the actual bridge. */
export async function createSidebarHtmlDocument() {
  const script = await buildSidebarScript(), sections = [];
  for (const file of ["web/style.css", "web/shared/ui/presentation.css", "web/features/sidebar/style.css"]) sections.push(await readFile(path.join(root, file), "utf8"));
  return `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: blob:; connect-src 'none'; font-src 'none'"><title>角色侧栏</title><style>${sections.join("\n")}</style></head><body><main id="app" aria-label="角色侧栏"><div role="status">正在加载侧栏…</div></main><script>${script.replaceAll("</script", "<\\/script")}</script></body></html>`;
}

/** Includes every declared static resource in both build integrity checks and the archive. */
async function staticResourcePaths() {
  const manifest = JSON.parse(await readFile(path.join(root, "manifest.json"), "utf8"));
  const generated = new Set(["resources/character-memory.html", "resources/character-sidebar.html"]);
  return manifest.resources.map(
    /** Uses the manifest's exact resource path; missing files must fail the build. */
    resource => resource.path,
  ).filter(
    /** Only the two documents generated from typed browser entries are excluded from static inputs. */
    file => !generated.has(file),
  );
}

/** Bundles graph geometry from its actual typed source for isolated Node/browser tests. */
export async function buildGraphModule() {
  const result = await build({ absWorkingDir: root, entryPoints: [path.join(root, "web/features/graph/layout.ts")], tsconfig: path.join(root, "tsconfig.web.json"), bundle: true, format: "cjs", platform: "neutral", target: "es2020", write: false, legalComments: "none" });
  if (result.outputFiles.length !== 1) throw new Error("Graph geometry must have exactly one module bundle");
  return result.outputFiles[0].text;
}

/** Lowers async/await to scoped Promise continuations so shared services retain each execution owner. */
export async function buildMainScript() {
  const result = await build({ absWorkingDir: root, entryPoints: [path.join(root, "src/main.ts")], bundle: true, format: "cjs", platform: "neutral", target: "es2020", supported: { "async-await": false }, write: false });
  if (result.outputFiles.length !== 1) throw new Error("The package must have exactly one main provider script");
  return result.outputFiles[0].contents;
}

/** Requires the transferred memory tools and exact metadata-to-TypeScript exports, including newly implemented tools. */
export function validateRuntimeToolsMetadata(sourceFile, source, metadata) {
  if (metadata === null || typeof metadata !== "object" || metadata.name !== "character_memory" || !Array.isArray(metadata.tools)) throw new Error("The memory subpackage must declare its actual name and tools");
  const required = ["get_memory_owner_key", "query_memory", "get_memory_by_title", "create_memory", "update_memory", "delete_memory", "move_memory", "update_user_preferences", "link_memories", "query_memory_links", "update_memory_link", "delete_memory_link"];
  const names = new Set();
  for (const tool of metadata.tools) {
    if (tool === null || typeof tool !== "object" || typeof tool.name !== "string" || tool.name.trim() === "" || names.has(tool.name)) throw new Error("Runtime-tools metadata contains an invalid or duplicate tool name");
    names.add(tool.name);
  }
  for (const name of required) if (!names.has(name)) throw new Error("Runtime-tools metadata omitted a transferred memory tool: " + name);
  const syntax = ts.createSourceFile(sourceFile, source, ts.ScriptTarget.Latest, true), exports = new Set();
  for (const statement of syntax.statements) {
    if (!ts.isFunctionDeclaration(statement) || statement.name === undefined || statement.modifiers === undefined) continue;
    if (statement.modifiers.some(
      /** Identifies a genuine exported function using the TypeScript syntax tree, not generated JavaScript text. */
      modifier => modifier.kind === ts.SyntaxKind.ExportKeyword,
    )) exports.add(statement.name.text);
  }
  for (const name of names) if (!exports.has(name)) throw new Error("Runtime-tools metadata has no implemented TypeScript function export: " + name);
  for (const name of exports) if (!names.has(name)) throw new Error("Runtime-tools metadata omitted an implemented tool export: " + name);
}

/** Bundles a confirmed runtime-tools entry while preserving its actual host-readable METADATA block. */
export async function buildRuntimeToolsScript() {
  const sourceFile = "src/runtime-tools/tools.ts";
  const source = await readFile(path.join(root, sourceFile), "utf8");
  const blocks = [...source.matchAll(/\/\* METADATA\s*([\s\S]*?)\*\//g)];
  if (blocks.length !== 1) throw new Error("The runtime-tools entry must declare exactly one METADATA block");
  const metadata = JSON.parse(blocks[0][1]);
  validateRuntimeToolsMetadata(sourceFile, source, metadata);
  const result = await build({ absWorkingDir: root, entryPoints: [path.join(root, sourceFile)], bundle: true, format: "cjs", platform: "neutral", target: "es2020", write: false, legalComments: "none", banner: { js: blocks[0][0] } });
  if (result.outputFiles.length !== 1) throw new Error("The memory subpackage must produce exactly one executable bundle");
  return result.outputFiles[0].contents;
}

/** Creates the exact offline document from current TypeScript sources for packaging and UI tests. */
export async function createHtmlDocument() {
  const script = await buildBrowserScript();
  const styles = ["web/style.css", "web/shared/ui/presentation.css"];
  const sections = [];
  for (const style of styles) sections.push(await readFile(path.join(root, style), "utf8"));
  const css = sections.join("\n");
  return `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: blob:; connect-src 'none'; font-src 'none'"><title>角色卡</title><style>${css}</style></head><body><main id="app" aria-label="角色卡"><div class="loading" role="status">正在加载角色卡…</div></main><div id="dialogs"></div><div id="snackbar" role="status" aria-live="polite"></div><script>${script.replaceAll("</script", "<\\/script")}</script></body></html>`;
}

/** Enumerates every regular source file so new modules cannot silently disappear from archives. */
export async function sourceFiles(directory) {
  const result = [];
  const entries = await readdir(path.join(root, directory), { withFileTypes: true });
  for (const entry of entries) {
    const relative = `${directory}/${entry.name}`;
    if (entry.isDirectory()) result.push(...await sourceFiles(relative));
    else if (entry.isFile()) result.push(relative);
    else throw new Error(`Unsupported package source entry: ${relative}`);
  }
  return result.sort();
}

/** Captures every package input so concurrent source changes cannot produce a mixed production archive. */
export async function captureBuildInputs() {
  const files = ["manifest.json", "README.md", "tsconfig.json", "tsconfig.web.json", ...await sourceFiles("scripts"), ...await sourceFiles("src"), ...await sourceFiles("web"), ...await staticResourcePaths()];
  const inputs = new Map();
  for (const file of files.sort()) inputs.set(file, createHash("sha256").update(await readFile(path.join(root, file))).digest("hex"));
  return inputs;
}

/** Rejects packaging when another worker has changed any captured source or added a module. */
export async function assertBuildInputsUnchanged(inputs) {
  const current = await captureBuildInputs();
  if (JSON.stringify([...current]) !== JSON.stringify([...inputs])) throw new Error("Package sources changed during bundling; coordinate a stable source snapshot before packaging");
}

/** Creates an archive from real current bundles and all host/browser source modules. */
export async function createPackageArchive(mainScript, toolsScript, htmlDocument, sidebarDocument = undefined) {
  if (!(mainScript instanceof Uint8Array) || mainScript.length === 0) throw new Error("The archive requires the actual main bundle bytes");
  if (!(toolsScript instanceof Uint8Array) || toolsScript.length === 0) throw new Error("The archive requires the actual memory-tools bundle bytes");
  if (typeof htmlDocument !== "string" || !htmlDocument.startsWith("<!doctype html>")) throw new Error("The archive requires the actual offline HTML document");
  const sidebar = sidebarDocument ?? await createSidebarHtmlDocument();
  if (typeof sidebar !== "string" || !sidebar.startsWith("<!doctype html>")) throw new Error("The archive requires the actual offline sidebar document");
  const entries = { "dist/main.js": new Uint8Array(mainScript), "dist/tools.js": new Uint8Array(toolsScript), "resources/character-memory.html": new TextEncoder().encode(htmlDocument), "resources/character-sidebar.html": new TextEncoder().encode(sidebar) };
  const files = ["manifest.json", "README.md", "tsconfig.json", "tsconfig.web.json", ...await sourceFiles("scripts"), ...await sourceFiles("src"), ...await sourceFiles("web"), ...await staticResourcePaths()];
  for (const file of files) entries[file] = new Uint8Array(await readFile(path.join(root, file)));
  const manifest = JSON.parse(new TextDecoder().decode(entries["manifest.json"]));
  for (const resource of manifest.resources) {
    if (!(entries[resource.path] instanceof Uint8Array) || entries[resource.path].length === 0) throw new Error("Declared resource is missing from package: " + resource.path);
  }
  return zipSync(entries);
}

/** Packages one typed character-and-memory provider with the same document exercised by UI tests. */
export async function buildPackage() {
  const inputs = await captureBuildInputs();
  if (checkProject("tsconfig.json") !== 0) throw new Error("Main-runtime TypeScript diagnostics must be resolved before packaging");
  if (checkProject("tsconfig.web.json") !== 0) throw new Error("Browser TypeScript diagnostics must be resolved before packaging");
  const manifest = JSON.parse(await readFile(path.join(root, "manifest.json"), "utf8"));
  if (manifest.toolpkg_id !== "com.operit.character_cards") throw new Error("Unexpected plugin identity");
  if (manifest.public_api !== "src/api.ts") throw new Error("Character cards public_api must identify the real src/api.ts contract");
  if (!Array.isArray(manifest.subpackages) || manifest.subpackages.length !== 1 || manifest.subpackages[0].id !== "character_memory" || manifest.subpackages[0].entry !== "dist/tools.js") throw new Error("Character memory must declare its actual executable subpackage");
  const mainScript = await buildMainScript();
  const toolsScript = await buildRuntimeToolsScript();
  const htmlDocument = await createHtmlDocument();
  const sidebarDocument = await createSidebarHtmlDocument();
  const archive = await createPackageArchive(mainScript, toolsScript, htmlDocument, sidebarDocument);
  await assertBuildInputsUnchanged(inputs);
  await mkdir(path.join(root, "dist"), { recursive: true });
  await writeFile(path.join(root, "dist/main.js"), mainScript);
  await writeFile(path.join(root, "dist/tools.js"), toolsScript);
  await mkdir(path.join(root, "resources"), { recursive: true });
  await writeFile(path.join(root, "resources/character-memory.html"), htmlDocument);
  await writeFile(path.join(root, "resources/character-sidebar.html"), sidebarDocument);
  await writeFile(path.join(root, "dist/character_cards.toolpkg"), archive);
  console.log("PACKED: character_cards.toolpkg (typed browser IIFE, exact executable tools metadata, public_api, all source modules included)");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await buildPackage();
