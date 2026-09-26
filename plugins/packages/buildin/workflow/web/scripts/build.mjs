import { build } from "esbuild";
import { mkdir, writeFile, readdir, readFile } from "node:fs/promises";

const result = await build({
  entryPoints: ["web/app.tsx"],
  bundle: true,
  minify: true,
  write: false,
  metafile: true,
  outfile: "app.js",
  target: ["es2020"],
  jsx: "automatic",
  legalComments: "eof",
});
const js = result.outputFiles.find((file) => file.path.endsWith(".js")).text;
const css = result.outputFiles.find((file) => file.path.endsWith(".css")).text;
await mkdir("resources", { recursive: true });
await writeFile(
  "resources/workflow.html",
  `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>工作流</title><style>${css}</style></head><body><div id="root"></div><script>${js.replaceAll("</script", "<\\/script")}</script></body></html>`,
);
// Resolve package directories from esbuild metafile inputs. With pnpm, bundled
// inputs live below node_modules/.pnpm/<pkg>/node_modules/<name>; treating
// `.pnpm` as a package name would make the license pass look for the invalid
// path node_modules/.pnpm/package.json.
const packageDirectories = new Map();
for (const inputPath of Object.keys(result.metafile.inputs)) {
  const pnpmMatch = /^(node_modules\/\.pnpm\/[^/]+\/node_modules\/(?:@[^/]+\/)?[^/]+)/.exec(inputPath);
  const directMatch = /^(node_modules\/(?:@[^/]+\/)?[^/]+)/.exec(inputPath);
  const directory = pnpmMatch?.[1] ?? directMatch?.[1];
  if (directory) {
    packageDirectories.set(
      directory,
      directory.slice(directory.lastIndexOf("/node_modules/") + "/node_modules/".length),
    );
  }
}
const notices = [];
for (const [directory, name] of [...packageDirectories.entries()].sort(([left], [right]) => left.localeCompare(right))) {
  const metadata = JSON.parse(
    await readFile(directory + "/package.json", "utf8"),
  );
  const licenses = (await readdir(directory)).filter((file) =>
    /^licen[sc]e(?:\.|$)/i.test(file),
  );
  notices.push(`${name} ${metadata.version} (${metadata.license})`);
  for (const license of licenses)
    notices.push(await readFile(directory + "/" + license, "utf8"));
}
await writeFile("resources/THIRD_PARTY_NOTICES.txt", notices.join("\n\n"));
