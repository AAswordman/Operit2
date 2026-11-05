import { readFile, readdir, writeFile } from "node:fs/promises";
import { zipSync } from "fflate";
const files = {}; const manifest = JSON.parse(await readFile("manifest.json", "utf8"));
const add = async path => { files[path] = new Uint8Array(await readFile(path)); };
await add("manifest.json"); await add(manifest.public_api); await add("resources/studio.html"); await add("resources/dsp/studio.wasm"); await add("resources/THIRD_PARTY_NOTICES.txt");
async function collect(path) { for (const e of (await readdir(path, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) { if (e.isDirectory()) await collect(path + "/" + e.name); else if (e.name.endsWith(".js")) await add(path + "/" + e.name); } }
await collect("dist");
for (const entry of [manifest.main, ...manifest.subpackages.map(s => s.entry)]) if (!files[entry]) throw new Error(`Missing executable ${entry}`);
const archive = zipSync(files, { level: 9, mtime: new Date(1980, 0, 1) });
await writeFile("dist/music_studio.toolpkg", archive); console.log(`Packed dist/music_studio.toolpkg: ${(archive.length / 1024).toFixed(1)} KiB`);
