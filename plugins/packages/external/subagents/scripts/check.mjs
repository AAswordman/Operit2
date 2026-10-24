import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const require = createRequire(new URL("../../../buildin/workflow/package.json", import.meta.url));
const ts = require("typescript");
const root = fileURLToPath(new URL("../", import.meta.url));
const configFile = fileURLToPath(new URL("../tsconfig.json", import.meta.url));
const config = ts.readConfigFile(configFile, ts.sys.readFile);
if (config.error) throw new Error(ts.flattenDiagnosticMessageText(config.error.messageText, "\n"));
const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, root);
if (parsed.options.strict !== true || parsed.options.noEmit !== true) throw new Error("Subagents must use strict no-emit checking by default");
const emit = process.argv.slice(2).indexOf("--emit") !== -1;
const program = ts.createProgram(parsed.fileNames, { ...parsed.options, skipLibCheck: false, noEmit: !emit });
const diagnostics = [...parsed.errors, ...ts.getPreEmitDiagnostics(program)];
if (diagnostics.length !== 0) {
  console.error(ts.formatDiagnosticsWithColorAndContext(diagnostics, {
    /** Preserves the compiler's exact path identity in diagnostics. */
    getCanonicalFileName(file) { return file; },
    /** Resolves diagnostics relative to this package. */
    getCurrentDirectory() { return root; },
    /** Uses ordinary newlines in diagnostics on every host. */
    getNewLine() { return "\n"; },
  }));
  process.exitCode = 1;
} else {
  if (emit) {
    const result = program.emit();
    if (result.emitSkipped || result.diagnostics.length !== 0) throw new Error("Subagent JavaScript emission failed");
  }
  console.log("PASS: subagents strict TypeScript" + (emit ? "; emitted plugin JavaScript" : "; no emit"));
  console.log("SCOPE: plugin source/declaration checks; no Rust or Flutter build, live model, or platform acceptance");
}
