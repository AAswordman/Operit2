import { readFile, mkdir, writeFile } from "node:fs/promises";
import { zipSync, unzipSync } from "fflate";
import { buildWasm } from "../../../buildin/character_cards/scripts/build-wasm.mjs";

buildWasm();
const root = new URL("../", import.meta.url);
const manifest = JSON.parse(await readFile(new URL("manifest.json", root), "utf8"));
const files = {};
for (const file of ["manifest.json", "README.md", manifest.main, ...manifest.wasm_modules.map(module => module.path)]) {
  files[file] = new Uint8Array(await readFile(new URL(file, root)));
}
const archive = zipSync(files, { level: 6, mtime: new Date(1980, 0, 1) });
const decoded = unzipSync(archive);
for (const [file, bytes] of Object.entries(files)) {
  if (!Buffer.from(decoded[file]).equals(Buffer.from(bytes))) throw new Error("Archive mismatch: " + file);
}
await mkdir(new URL("dist/", root), { recursive: true });
await writeFile(new URL("dist/toolpkg_usearch_demo.toolpkg", root), archive);
console.log("Packed toolpkg_usearch_demo.toolpkg with owner-built USearch WASM");
