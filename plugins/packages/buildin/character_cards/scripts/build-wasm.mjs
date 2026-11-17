import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

/** Builds the plugin-owned USearch module with the repository Python environment. */
export function buildWasm() {
  const repository = new URL("../../../../../", import.meta.url);
  const python = fileURLToPath(new URL(process.platform === "win32" ? ".venv/Scripts/python.exe" : ".venv/bin/python", repository));
  execFileSync(python, [fileURLToPath(new URL("../vendor/usearch/build.py", import.meta.url))], { stdio: "inherit" });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) buildWasm();
