import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import vm from "node:vm";
const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const { buildSync } = require("esbuild");
const ts = require("typescript");

/** Reads the current TypeScript syntax tree without evaluating a production service or replacing dependencies. */
function typescriptSource(relative) {
  const file = fileURLToPath(new URL(relative, import.meta.url));
  return ts.createSourceFile(file, readFileSync(file, "utf8"), ts.ScriptTarget.Latest, true);
}

/** Copies pure domain-test values across isolated VM realms. */
export function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Loads current plugin TypeScript without inventing a production host or business data source. */
export function loadModule(relative, globals = {}) {
  const file = fileURLToPath(new URL(`../${relative}`, import.meta.url));
  const result = buildSync({ entryPoints: [file], bundle: true, format: "cjs", platform: "neutral", target: "es2020", write: false });
  if (result.outputFiles.length !== 1) throw new Error("Unexpected test bundle output count");
  const module = { exports: {} };
  const forbidden = new Proxy({}, {
    /** Rejects every invocation of the removed CLI or lossy summary-tool bridge. */
    get(_target, property) { throw new Error(`Forbidden production Tools access: ${String(property)}`); },
  });
  vm.runInNewContext(result.outputFiles[0].text, { module, exports: module.exports, console, TextEncoder, TextDecoder, setTimeout, clearTimeout, Tools: forbidden, ...globals }, { filename: file });
  return module.exports;
}

/** Reads declared interface members from current plugin or real generic registry types. */
export function interfaceMembers(relative, name) {
  const source = typescriptSource(relative);
  const declarations = [];
  /** Locates an exact named interface including declarations nested in SDK namespaces. */
  function visit(node) {
    if (ts.isInterfaceDeclaration(node) && node.name.text === name) declarations.push(node);
    ts.forEachChild(node, visit);
  }
  visit(source);
  if (declarations.length !== 1) throw new Error(`Expected one ${name} interface in ${relative}`);
  return declarations[0].members.map(
    /** Preserves the complete declared field or method name. */
    member => { const text = member.name.getText(source); return ts.isStringLiteral(member.name) ? member.name.text : text; },
  );
}

/** Counts actual calls to one named runtime import, including explicitly aliased bindings. */
export function importedCallCount(relative, moduleName, exportName) {
  const source = typescriptSource(relative), bindings = [];
  for (const statement of source.statements) {
    if (!ts.isImportDeclaration(statement) || !ts.isStringLiteral(statement.moduleSpecifier) || statement.moduleSpecifier.text !== moduleName) continue;
    const clause = statement.importClause;
    if (clause === undefined || clause.isTypeOnly || clause.namedBindings === undefined || !ts.isNamedImports(clause.namedBindings)) continue;
    for (const element of clause.namedBindings.elements) {
      const imported = element.propertyName === undefined ? element.name.text : element.propertyName.text;
      if (!element.isTypeOnly && imported === exportName) bindings.push(element.name.text);
    }
  }
  if (bindings.length !== 1) throw new Error(`Expected exactly one runtime import of ${exportName} from ${moduleName} in ${relative}`);
  let calls = 0;
  /** Counts real direct call expressions rather than comments or documentation strings. */
  function visit(node) {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === bindings[0]) calls++;
    ts.forEachChild(node, visit);
  }
  visit(source);
  return calls;
}
