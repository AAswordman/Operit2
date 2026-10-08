import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { architectureViolations } from "../tests/architecture.mjs";
const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const ts = require("typescript");
const root = fileURLToPath(new URL("../", import.meta.url));

/** Checks current sources against their real declarations without emitting host or browser code. */
export function checkProject(configName) {
  const config = fileURLToPath(new URL(`../${configName}`, import.meta.url));
  const source = ts.readConfigFile(config, ts.sys.readFile);
  if (source.error) throw new Error(ts.flattenDiagnosticMessageText(source.error.messageText, "\n"));
  const parsed = ts.parseJsonConfigFileContent(source.config, ts.sys, root);
  if (parsed.options.strict !== true || parsed.options.noEmit !== true || parsed.options.noImplicitAny === false) throw new Error(`${configName} must use strict no-emit checking without relaxing implicit any`);
  if (parsed.options.lib === undefined) throw new Error(`${configName} must explicitly declare its runtime libraries`);
  const hasDom = parsed.options.lib.some(
    /** Identifies the compiler's exact DOM library rather than weakening the host project. */
    library => /^lib\.dom(?:\.iterable)?\.d\.ts$/.test(library),
  );
  if (configName === "tsconfig.json" && hasDom) throw new Error("Host TypeScript must not use DOM libraries");
  if (configName === "tsconfig.web.json" && !hasDom) throw new Error("Browser TypeScript must declare its separate DOM libraries");
  const contractTests = configName === "tsconfig.json" ? ts.sys.readDirectory(`${root}tests`, [".ts"], [], ["**/*.ts"]) : [];
  const program = ts.createProgram([...parsed.fileNames, ...contractTests], { ...parsed.options, skipLibCheck: false });
  const diagnostics = [...parsed.errors, ...ts.getPreEmitDiagnostics(program)];
  if (diagnostics.length !== 0) {
    console.error(ts.formatDiagnosticsWithColorAndContext(diagnostics, {
      /** Preserves the compiler's canonical source identity. */
      getCanonicalFileName: file => file,
      /** Reports the package directory used by this no-emit check. */
      getCurrentDirectory: () => root,
      /** Uses the standard source diagnostic separator. */
      getNewLine: () => "\n",
    }));
    return diagnostics.length;
  }
  console.log(`CHECKED: character_cards ${configName} (strict, no emit)`);
  return 0;
}

/** Reports dedicated contracts and mounted old business paths as real failures, without running strict checks. */
export function checkArchitecture() {
  const violations = architectureViolations();
  for (const violation of violations) console.error(`ARCHITECTURE: ${violation}`);
  if (violations.length === 0) console.log("CHECKED: dedicated-contract and explicit mounted-business architecture checks (static only)");
  return violations.length;
}

/** Executes the routing owner's existing static checker without adding a second router implementation. */
export function checkRouting() {
  const file = fileURLToPath(new URL("../../../../../core/crates/command/core/scripts/check-routing.mjs", import.meta.url));
  const result = spawnSync(process.execPath, [file], { stdio: "inherit", shell: false });
  if (result.error) throw result.error;
  if (result.status === null) throw new Error(`Routing checker terminated with ${result.signal}`);
  return result.status;
}

/** Labels each actual check result without treating a scoped or fixture success as full acceptance. */
function resultLabel(count) { return count === 0 ? "PASS" : "FAIL (" + count + ")"; }

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const browserOnly = process.argv[2] === "--web";
  const architecture = browserOnly ? 0 : checkArchitecture();
  const host = browserOnly ? 0 : checkProject("tsconfig.json");
  const browser = checkProject("tsconfig.web.json");
  const routing = browserOnly ? 0 : checkRouting();
  if (architecture + host + browser + routing !== 0) process.exitCode = 1;
  if (browserOnly) console.log("RESULT: browser strict " + resultLabel(browser) + "; browser-only scope, not full acceptance");
  else console.log("RESULT: architecture " + resultLabel(architecture) + "; host strict " + resultLabel(host) + "; browser strict " + resultLabel(browser) + "; routing static " + resultLabel(routing));
  console.log("SCOPE: no Rust or Flutter compilation; Node source/declaration checks only");
}
