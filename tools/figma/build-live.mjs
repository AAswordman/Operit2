import { build } from "esbuild";
import { chmod, mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { zipSync, strToU8 } from "fflate";
const root = new URL("./", import.meta.url);
const result = await build({
  absWorkingDir: fileURLToPath(root),
  entryPoints: ["src/live_main.ts"],
  bundle: true,
  write: false,
  target: "es2022",
  format: "iife",
});
const template = await readFile(new URL("src/live.html", root), "utf8");
const html = template.replace(
  "/* OPERIT_LIVE_SCRIPT */",
  result.outputFiles[0].text.replaceAll("</script", "<\\/script"),
);
await mkdir(new URL("dist/live/", root), { recursive: true });
await writeFile(new URL("dist/live/index.html", root), html);
const files = {
  "index.html": html,
  "package.json": JSON.stringify(
    {
      name: "operit-live-design",
      private: true,
      type: "module",
      scripts: {
        dev: "node serve.mjs",
        start: "node serve.mjs",
        build: "node build.mjs",
      },
    },
    null,
    2,
  ),
  "package-lock.json": JSON.stringify(
    {
      name: "operit-live-design",
      lockfileVersion: 3,
      requires: true,
      packages: { "": { name: "operit-live-design" } },
    },
    null,
    2,
  ),
  ".figma/make/install":
    "#!/usr/bin/env bash\nset -euo pipefail\nexec npm ci --ignore-scripts --no-audit --no-fund\n",
  ".figma/make/dev":
    "#!/usr/bin/env bash\nset -euo pipefail\nexec node serve.mjs\n",
  ".figma/make/deploy-preview":
    "#!/usr/bin/env bash\nset -euo pipefail\nnpm run build\n[[ -d dist ]]\nexec figma make deploy-preview --build-dir dist\n",
  ".figma/make/deploy":
    "#!/usr/bin/env bash\nset -euo pipefail\nnpm run build\n[[ -d dist ]]\nexec figma make deploy --build-dir dist\n",
  "build.mjs":
    "import {mkdir,copyFile} from 'node:fs/promises';await mkdir('dist',{recursive:true});await copyFile('index.html','dist/index.html');\n",
  "serve.mjs":
    "import {createServer} from 'node:http';import {readFile} from 'node:fs/promises';const html=await readFile(new URL('./index.html',import.meta.url));createServer((req,res)=>{res.writeHead(200,{'Content-Type':'text/html; charset=utf-8'});res.end(html);}).listen(Number(process.env.PORT??5173),process.env.FIGMA_DEV_SERVER_HOST??'0.0.0.0');\n",
  "README.md":
    "# Operit live design client\n\nRun `npm run dev` or `npm run build`. Paste a local connection URL from `tools/figma/live.mjs` after loading. A debug Operit instance is required on the same computer. This is a frontend adapter, not a standalone copy of Operit. Figma container access, native Point and edit, full software coverage and changes to original sources are still awaiting acceptance. Never commit connection URLs or tokens.\n",
};
const project = new URL("dist/live-code-layer/", root);
await mkdir(project, { recursive: true });
for (const [path, content] of Object.entries(files)) {
  const destination = new URL(path, project);
  await mkdir(new URL("./", destination), { recursive: true });
  await writeFile(destination, content);
  await chmod(destination, path.startsWith(".figma/make/") ? 0o755 : 0o644);
}
await writeFile(
  new URL("dist/operit-live-code-layer.zip", root),
  zipSync(
    Object.fromEntries(
      Object.entries(files).map(([path, content]) => [
        path,
        [
          strToU8(content),
          {
            os: 3,
            attrs:
              ((path.startsWith(".figma/make/") ? 0o100755 : 0o100644) << 16) >>> 0,
          },
        ],
      ]),
    ),
  ),
);
console.log(
  "Built live frontend and dist/operit-live-code-layer.zip; Figma acceptance pending",
);
