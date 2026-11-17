import { build } from "esbuild";
export async function load(file) {
  const result = await build({ entryPoints: [file], bundle: true, write: false, format: "esm", platform: "node", target: "node20", logLevel: "silent" });
  return import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString("base64")}`);
}
