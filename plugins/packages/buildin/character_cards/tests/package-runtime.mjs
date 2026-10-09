import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import vm from "node:vm";

/** Reads an actual Rust-owned SDK script; the Node harness does not execute Rust or native hosts. */
export function sdkScript(file) {
  const source = readFileSync(new URL("../../../../../core/crates/plugin/sdk/src/toolpkg/" + file, import.meta.url), "utf8");
  const start = source.indexOf('r#"'), end = source.indexOf('"#', start + 3);
  assert.ok(start >= 0 && end > start, "Missing SDK script: " + file);
  return source.slice(start + 3, end);
}

/** Uses the engine's actual path matching, registration placeholders and root export-tagging behavior. */
function engineModuleHelpers() {
  const source = readFileSync(new URL("../../../../../core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs", import.meta.url), "utf8");
  const sections = [
    ["__operitNormalizePath", "__operitDirname"],
    ["__operitTagModuleExports", "__operitText"],
    ["__operitCreateRegistrationScreenPlaceholder", "__operitFindTargetFunction"],
  ];
  return sections.map(([first, next]) => {
    const start = source.indexOf("        function " + first + "("), end = source.indexOf("        function " + next + "(", start);
    assert.ok(start >= 0 && end > start, "Missing actual engine helper: " + first);
    return source.slice(start, end).replaceAll("{{", "{").replaceAll("}}", "}");
  }).join("\n");
}

/** Evaluates exact CJS artifact bytes with only relative package loading and real engine metadata tagging. */
export function packageRuntime(modules, globals = {}, registrationOnly = false) {
  const context = vm.createContext({ console, TextEncoder, TextDecoder, setTimeout, clearTimeout, ...globals });
  vm.runInContext(engineModuleHelpers(), context);
  const cache = new Map();
  function load(modulePath) {
    const normalized = path.posix.normalize(modulePath);
    if (registrationOnly && context.__operitIsLocalUiModulePath(normalized)) return context.__operitCreateRegistrationScreenPlaceholder(normalized);
    if (cache.has(normalized)) return cache.get(normalized).exports;
    assert.ok(Object.hasOwn(modules, normalized), "Package omitted registered module: " + normalized);
    const module = { exports: {} };
    cache.set(normalized, module);
    const factory = vm.runInContext("(function(module, exports, require) {\n" + Buffer.from(modules[normalized]).toString("utf8") + "\n})", context, { filename: normalized });
    factory(module, module.exports, request => {
      assert.ok(request.startsWith("."), "Only declared relative modules may be loaded: " + request);
      return load(path.posix.join(path.posix.dirname(normalized), request));
    });
    context.__operitTagModuleExports(normalized, module.exports);
    return module.exports;
  }
  return { context, load };
}

