import { readFile, readdir, writeFile } from "node:fs/promises";
import { zipSync } from "fflate";
const files = {
  "manifest.json": new Uint8Array(await readFile("manifest.json")),
  "resources/workflow.html": new Uint8Array(
    await readFile("resources/workflow.html"),
  ),
};
files["resources/THIRD_PARTY_NOTICES.txt"] = new Uint8Array(
  await readFile("resources/THIRD_PARTY_NOTICES.txt"),
);
files["resources/logo.svg"] = new Uint8Array(
  await readFile("resources/logo.svg"),
);
/** Adds executable package modules while excluding previous archives. */
async function collect(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = directory + "/" + entry.name;
    if (entry.isDirectory()) await collect(path);
    else if (entry.name.endsWith(".js"))
      files[path] = new Uint8Array(await readFile(path));
  }
}
const manifest = JSON.parse(await readFile("manifest.json", "utf8"));
if (typeof manifest.public_api !== "string" || manifest.public_api !== "src/api.ts") {
  throw new Error("Workflow public_api must identify src/api.ts");
}
files[manifest.public_api] = new Uint8Array(await readFile(manifest.public_api));
await collect("dist");
await writeFile("dist/workflow.toolpkg", zipSync(files, { mtime: new Date(1980, 0, 1) }));
