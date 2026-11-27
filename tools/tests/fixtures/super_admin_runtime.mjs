import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';

const root = new URL('../../../', import.meta.url);
const require = createRequire(new URL('.ci-tools/typescript/package.json', root));
const ts = require('typescript');

/** Loads production plugin modules into one retained container realm without writing build artifacts. */
export function loadSuperAdmin(context) {
  const modules = new Map();
  const handlers = new Map();
  const hooks = [];
  context.ToolPkg = {
    ipc: {
      /** Registers the production owner's IPC channel. */
      on(channel, handler) { handlers.set(channel, handler); },
      /** Preserves the asynchronous main-runtime dispatch boundary. */
      async call(channel, request, options) {
        assert.equal(options.targetRuntime, 'main');
        assert.ok(handlers.has(channel));
        return handlers.get(channel)(structuredClone(request));
      },
    },
    /** Captures the actual periodic Host registration. */
    registerHostEventHook(hook) { hooks.push(hook); },
    /** Captures model prompt registration separately from periodic cleanup. */
    registerPromptFinalizeHook() {},
    /** Captures estimation registration without changing cleanup assertions. */
    registerPromptEstimateFinalizeHook() {},
  };
  /** Resolves the plugin's real CommonJS module graph with a shared module cache. */
  function load(name) {
    if (modules.has(name)) return modules.get(name).exports;
    const module = { exports: {} };
    modules.set(name, module);
    const source = readFileSync(new URL(`plugins/packages/buildin/super_admin/src/${name}.ts`, root), 'utf8');
    const javascript = ts.transpileModule(source, {
      compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
    }).outputText;
    const run = vm.runInContext(`(function(require,module,exports){${javascript}\n})`, context);
    run(request => {
      assert.match(request, /^\.\/[a-z]+$/);
      return load(request.slice(2));
    }, module, module.exports);
    return module.exports;
  }
  const main = load('main');
  assert.equal(main.registerToolPkg(), true);
  return { tools: load('tools'), main, hooks };
}
