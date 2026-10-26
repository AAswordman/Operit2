import { createRequire } from "node:module";
import { readFile, readdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { posix } from "node:path";
import vm from "node:vm";

const require = createRequire(new URL("../../../buildin/workflow/package.json", import.meta.url));
const { zipSync, unzipSync } = require("fflate");
const root = new URL("../", import.meta.url);
const check = spawnSync(process.execPath, [fileURLToPath(new URL("check.mjs", import.meta.url)), "--emit"], { stdio: "inherit", shell: false });
if (check.error) throw check.error;
if (check.status !== 0) throw new Error("Subagent checks failed; package was not written");
const files = {
  "manifest.json": new Uint8Array(await readFile(new URL("manifest.json", root))),
  "README.md": new Uint8Array(await readFile(new URL("README.md", root))),
};

/** Adds only emitted JavaScript modules, preserving relative CommonJS imports in the ToolPkg archive. */
async function collect(relative) {
  const entries = await readdir(new URL(relative + "/", root), { withFileTypes: true });
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    const path = relative + "/" + entry.name;
    if (entry.isDirectory()) await collect(path);
    else if (entry.name.endsWith(".js")) files[path] = new Uint8Array(await readFile(new URL(path, root)));
  }
}

await collect("dist");
const manifest = JSON.parse(Buffer.from(files["manifest.json"]).toString("utf8"));
for (const entry of [manifest.main, ...manifest.subpackages.map(subpackage => subpackage.entry)]) {
  if (!Object.hasOwn(files, entry)) throw new Error("Missing executable package entry: " + entry);
}
const archive = zipSync(files, { level: 6 });
const decoded = unzipSync(archive);
for (const [path, contents] of Object.entries(files)) {
  if (!Buffer.from(decoded[path]).equals(Buffer.from(contents))) throw new Error("Package round-trip mismatch: " + path);
}
// Execute the emitted archive itself so missing relative modules or missing tool exports fail packaging.
const registrations = { channels: [], slots: [] };
const sandbox = vm.createContext({
  console, setTimeout, clearTimeout, AbortController,
  ToolPkg: {
    ipc: {
      /** Records service declaration without invoking a business operation. */
      on(channel) { registrations.channels.push(channel); },
    },
    /** Records the actual emitted composer contribution. */
    registerChatComposerSlot(slot) { registrations.slots.push(slot); },
  },
  Tools: new Proxy({}, {
    /** Rejects host work during archive loading and registration. */
    get() { throw new Error("Archive registration attempted to invoke host tools"); },
  }),
});
const modules = new Map();

/** Loads emitted modules exclusively from the checked archive, with exact relative identities. */
function loadModule(path) {
  if (modules.has(path)) return modules.get(path).exports;
  if (!Object.hasOwn(decoded, path)) throw new Error("Missing archive module: " + path);
  const module = { exports: {} };
  modules.set(path, module);
  const source = Buffer.from(decoded[path]).toString("utf8");
  const execute = vm.runInContext(`(function(require,module,exports){
${source}
})`, sandbox, { filename: path });
  /** Resolves the package's authored relative CommonJS import without accessing local disk modules. */
  const requireModule = specifier => {
    if (!specifier.startsWith(".") || !specifier.endsWith(".js")) throw new Error("Runtime imports must use explicit relative JavaScript paths: " + specifier);
    return loadModule(posix.normalize(posix.join(posix.dirname(path), specifier)));
  };
  execute(requireModule, module, module.exports);
  return module.exports;
}

const main = loadModule(manifest.main);
if (main.registerToolPkg() !== true || registrations.channels.length !== 1 || registrations.slots.length !== 1 || registrations.slots[0].slot !== "above_input") throw new Error("Archive main registration failed");
for (const subpackage of manifest.subpackages) {
  const source = Buffer.from(decoded[subpackage.entry]).toString("utf8");
  const block = source.match(/\/\*\s*METADATA\s+([\s\S]*?)\*\//);
  if (block === null) throw new Error("Missing subpackage metadata: " + subpackage.id);
  const metadata = JSON.parse(block[1]);
  const exported = loadModule(subpackage.entry);
  if (metadata.name !== subpackage.id) throw new Error("Subpackage metadata identity mismatch");
  for (const tool of metadata.tools) if (typeof exported[tool.name] !== "function") throw new Error("Missing archived tool export: " + tool.name);
}
console.log("PASS: emitted archive module loading, composer registration and tool exports (controlled host)");
const output = new URL("dist/subagents.toolpkg", root);
await writeFile(output, archive);
console.log("PACKED: " + fileURLToPath(output) + " (" + archive.length + " bytes, " + Object.keys(files).length + " files)");
