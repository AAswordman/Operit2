import assert from "node:assert/strict";
import test from "node:test";
import { createRequire } from "node:module";
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { readFile, readdir } from "node:fs/promises";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import path from "node:path";
import vm from "node:vm";
import { buildMainScript, buildRuntimeToolsScript, createHtmlDocument, createSidebarHtmlDocument, createPackageArchive } from "../scripts/build.mjs";

const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const { unzipSync } = require("fflate");
const root = fileURLToPath(new URL("../", import.meta.url));
const repositoryRoot = fileURLToPath(new URL("../../../../../", import.meta.url));
const executeFile = promisify(execFile);

/** Builds every esbuild output in an isolated read-only child without changing the parent's working directory. */
async function buildFromWorkingDirectory(cwd) {
  const code = [
    `import { buildBrowserScript, buildGraphModule, buildMainScript, buildRuntimeToolsScript, createHtmlDocument } from ${JSON.stringify(new URL("../scripts/build.mjs", import.meta.url).href)};`,
    "/** Encodes exact artifact bytes for lossless transport from the isolated build process. */",
    "function encode(value) { return Buffer.from(value).toString('base64'); }",
    "const outputs = { main: encode(await buildMainScript()), tools: encode(await buildRuntimeToolsScript()), browser: encode(await buildBrowserScript()), graph: encode(await buildGraphModule()), html: encode(await createHtmlDocument()) };",
    "process.stdout.write(JSON.stringify(outputs));",
  ].join("\n");
  const { stdout, stderr } = await executeFile(process.execPath, ["--input-type=module", "--eval", code], { cwd, encoding: "utf8", maxBuffer: 8 * 1024 * 1024 });
  assert.equal(stderr, "", "The isolated current-source build reported an unexpected diagnostic");
  return JSON.parse(stdout);
}

/** Requires exact bytes from both supported invocation directories without normalizing generated paths or hashes. */
test("all esbuild outputs are byte-identical from repository and plugin working directories", async () => {
  const originalDirectory = process.cwd();
  const [repositoryOutputs, pluginOutputs] = await Promise.all([buildFromWorkingDirectory(repositoryRoot), buildFromWorkingDirectory(root)]);
  const names = ["main", "tools", "browser", "graph", "html"];
  assert.deepEqual(Object.keys(repositoryOutputs), names);
  assert.deepEqual(Object.keys(pluginOutputs), names);
  for (const name of names) {
    const repositoryBytes = Buffer.from(repositoryOutputs[name], "base64"), pluginBytes = Buffer.from(pluginOutputs[name], "base64");
    assert.ok(repositoryBytes.length > 0, "The isolated build omitted artifact bytes: " + name);
    assert.equal(Buffer.compare(repositoryBytes, pluginBytes), 0, "Working-directory-dependent artifact bytes: " + name);
  }
  assert.equal(process.cwd(), originalDirectory, "Cross-directory builds must not mutate the parent process working directory");
});

/** Computes the exact artifact identity without printing complete generated source on failure. */
function digest(bytes) { return createHash("sha256").update(bytes).digest("hex"); }

/** Independently enumerates every production module rather than using the archive owner's enumeration helper. */
async function sourcePaths(relative) {
  const files = [];
  for (const entry of await readdir(path.join(root, relative), { withFileTypes: true })) {
    const file = relative + "/" + entry.name;
    if (entry.isDirectory()) files.push(...await sourcePaths(file));
    else if (entry.isFile()) files.push(file);
    else throw new Error("Unsupported package source entry in archive acceptance: " + file);
  }
  return files.sort();
}

/** Checks the declared installed main artifact separately from tests that transpile current TypeScript source. */
test("manifest main artifact matches current production TypeScript instead of a stale unconnected runtime", async () => {
  const manifest = JSON.parse(await readFile(path.join(root, "manifest.json"), "utf8"));
  assert.equal(manifest.main, "dist/main.js");
  const current = await readFile(path.join(root, manifest.main)), expected = await buildMainScript();
  assert.equal(digest(current), digest(expected), "dist/main.js is stale: source-bundle tests do not establish the installed production entry; Web/build owner must regenerate the artifact");
});

/** Requires the installed offline document to be the current strict-TS browser build, not only an in-memory UI fixture build. */
test("manifest offline HTML artifact matches current typed Web modules", async () => {
  const manifest = JSON.parse(await readFile(path.join(root, "manifest.json"), "utf8"));
  const resources = manifest.resources.filter(
    /** Resolves exactly the resource requested by the real host WebView registration. */
    resource => resource.key === "character_memory_html",
  );
  assert.equal(resources.length, 1);
  const current = await readFile(path.join(root, resources[0].path)), expected = await createHtmlDocument();
  assert.equal(digest(current), digest(Buffer.from(expected, "utf8")), "resources/character-memory.html is stale: isolated current-source UI checks are not installed UI acceptance");
});

/** Inspects a real in-memory archive without claiming its bundles have run in the application host. */
test("archive includes all current nested source modules with their exact bytes", async () => {
  const main = await buildMainScript(), tools = await buildRuntimeToolsScript(), html = await createHtmlDocument();
  const entries = unzipSync(await createPackageArchive(main, tools, html));
  const files = [...await sourcePaths("src"), ...await sourcePaths("web"), ...await sourcePaths("scripts")];
  for (const file of files) {
    assert.ok(Object.hasOwn(entries, file), "Package archive omitted current source module: " + file);
    assert.equal(digest(entries[file]), digest(await readFile(path.join(root, file))), "Package archive has wrong source bytes: " + file);
  }
  assert.equal(digest(entries["dist/main.js"]), digest(main));
  assert.equal(digest(entries["dist/tools.js"]), digest(tools));
  assert.equal(digest(entries["resources/character-memory.html"]), digest(Buffer.from(html, "utf8")));
});

/** Verifies actual subpackage bytes, host-readable metadata and every implemented current tool export. */
test("manifest memory subpackage is the current executable bundle with complete real metadata exports", async () => {
  const manifest = JSON.parse(await readFile(path.join(root, "manifest.json"), "utf8"));
  assert.deepEqual(manifest.subpackages, [{ id: "character_memory", entry: "dist/tools.js" }]);
  const installed = await readFile(path.join(root, manifest.subpackages[0].entry)), current = await buildRuntimeToolsScript();
  assert.equal(digest(installed), digest(current), "Installed memory-tools subpackage is stale");
  const blocks = [...installed.toString("utf8").matchAll(/\/\* METADATA\s*([\s\S]*?)\*\//g)];
  assert.equal(blocks.length, 1);
  const metadata = JSON.parse(blocks[0][1]); assert.equal(metadata.name, "character_memory");
  const source = await readFile(path.join(root, "src/runtime-tools/tools.ts"), "utf8"), sourceBlocks = [...source.matchAll(/\/\* METADATA\s*([\s\S]*?)\*\//g)];
  assert.equal(sourceBlocks.length, 1); assert.deepEqual(metadata, JSON.parse(sourceBlocks[0][1]));
  const names = metadata.tools.map(
    /** Requires the declared tool identities to remain unique and complete. */
    tool => tool.name,
  );
  assert.equal(new Set(names).size, names.length); assert.ok(names.indexOf("list_character_cards") >= 0);
  const module = { exports: {} }, forbidden = new Proxy({}, {
    /** Rejects every eager host call instead of providing successful tool or storage responses. */
    get(_target, key) { throw new Error("Tool evaluation must not access the host: " + String(key)); },
  });
  vm.runInNewContext(installed.toString("utf8"), { module, exports: module.exports, ToolPkg: forbidden, Tools: forbidden, console, TextEncoder, TextDecoder, setTimeout, clearTimeout });
  assert.deepEqual(Object.keys(module.exports).sort(), [...names].sort());
  for (const name of names) assert.equal(typeof module.exports[name], "function", name);
});

/** Requires the exact production asset to contain current bundles, metadata and every production source module. */
test("single Core production archive exactly matches the packaged plugin and its current source artifacts", async () => {
  const packaged = await readFile(path.join(root, "dist/character_cards.toolpkg"));
  const productionPath = fileURLToPath(new URL("../../../../../core/crates/runtime/application/assets/plugins/buildin/character_cards.toolpkg", import.meta.url));
  const production = await readFile(productionPath);
  assert.equal(digest(production), digest(packaged), "Core still loads a different or stale character_cards.toolpkg asset");
  const entries = unzipSync(production), main = await buildMainScript(), tools = await buildRuntimeToolsScript(), html = await createHtmlDocument();
  assert.equal(digest(entries["dist/main.js"]), digest(main));
  assert.equal(digest(entries["dist/tools.js"]), digest(tools));
  assert.equal(digest(entries["resources/character-memory.html"]), digest(Buffer.from(html, "utf8")));
  assert.equal(digest(entries["manifest.json"]), digest(await readFile(path.join(root, "manifest.json"))));
  const manifest = JSON.parse(Buffer.from(entries["manifest.json"]).toString("utf8"));
  assert.equal(manifest.public_api, "src/api.ts"); assert.ok(Object.hasOwn(entries, manifest.public_api));
  assert.deepEqual(manifest.subpackages, [{ id: "character_memory", entry: "dist/tools.js" }]);
  for (const file of [...await sourcePaths("src"), ...await sourcePaths("web"), ...await sourcePaths("scripts")]) {
    assert.ok(Object.hasOwn(entries, file), "Production archive omitted module: " + file);
    assert.equal(digest(entries[file]), digest(await readFile(path.join(root, file))), "Production archive has stale module: " + file);
  }
  const host = Buffer.from(entries["src/host.ts"]).toString("utf8");
  assert.match(host, /surface:\s*"chat_attachments"/); assert.match(host, /toolpkg:\$\{definition\.id\}:ui:memory-attachment/);
  assert.equal(host.indexOf("chat.attachments.sources"), -1);
  const mainSource = Buffer.from(entries["src/main.ts"]).toString("utf8");
  assert.equal([...mainSource.matchAll(/registerMemoryJobHooks\(\)/g)].length, 1);
  assert.match(mainSource, /connectDirectorySources\(/);
});

/** Guards the complete manifest contract exercised eagerly by the native JS runtime at startup. */
test("all declared resources are packaged and the installed sidebar matches its actual typed entry", async () => {
  const entries = unzipSync(await readFile(path.join(root, "dist/character_cards.toolpkg")));
  const manifest = JSON.parse(Buffer.from(entries["manifest.json"]).toString("utf8"));
  for (const resource of manifest.resources) {
    assert.ok(Object.hasOwn(entries, resource.path), "Missing declared resource: " + resource.path);
    const installed = await readFile(path.join(root, resource.path));
    assert.ok(installed.length > 0);
    assert.equal(digest(entries[resource.path]), digest(installed), "Wrong resource bytes: " + resource.path);
  }
  const sidebar = await createSidebarHtmlDocument();
  assert.equal(digest(entries["resources/character-sidebar.html"]), digest(Buffer.from(sidebar, "utf8")));
  assert.match(sidebar, /CharacterSidebarHost/);
  assert.match(sidebar, /sidebar-view/);
});

/** The installed provider must lower native awaits so shared initialization uses call-scoped continuations. */
test("main provider lowers every async function while retaining the ES2020 runtime features", async () => {
  const ts = require("typescript");
  const text = Buffer.from(await buildMainScript()).toString("utf8");
  const tree = ts.createSourceFile("main.js", text, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
  let nativeAsyncFunctions = 0;
  /** Inspects syntax nodes, not comments or strings containing the word async. */
  function visit(node) {
    if (ts.isFunctionLike(node) && node.modifiers?.some(modifier => modifier.kind === ts.SyntaxKind.AsyncKeyword)) nativeAsyncFunctions++;
    ts.forEachChild(node, visit);
  }
  visit(tree);
  assert.equal(nativeAsyncFunctions, 0, "Native await bypasses call-scoped Promise.then in the host runtime");
  assert.match(text, /1n/u, "Lowering async must not remove supported BigInt identifiers");
});
