import { build } from "esbuild";
import { mkdir, readFile, writeFile } from "node:fs/promises";
await mkdir(new URL("./dist/", import.meta.url), { recursive: true });
await build({
  entryPoints: ["src/plugin.ts"],
  bundle: true,
  outfile: "dist/plugin.js",
  target: "es2018",
  format: "iife",
});
const result = await build({
  entryPoints: ["src/ui.ts"],
  bundle: true,
  write: false,
  target: "es2020",
  format: "iife",
});
const template = await readFile(
  new URL("./src/ui.html", import.meta.url),
  "utf8",
);
await writeFile(
  new URL("./dist/ui.html", import.meta.url),
  template.replace(
    "/* OPERIT_UI_SCRIPT */",
    result.outputFiles[0].text.replaceAll("</script", "<\\/script"),
  ),
);
console.log("Figma plugin built: tools/figma/manifest.json");
