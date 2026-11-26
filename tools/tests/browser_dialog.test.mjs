import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const require = createRequire(new URL('plugins/packages/buildin/workflow/package.json', root));
const ts = require('typescript');
const source = readFileSync(new URL('plugins/packages/buildin/browser.ts', root), 'utf8');

/** Executes the actual browser package against the canonical host boundary. */
function browser(browserDialog) {
  const context = vm.createContext({ exports: {}, Tools: { Net: { browserDialog } } });
  vm.runInContext(ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
  }).outputText, context);
  return context.exports;
}

/** Confirms the package forwards inspection and exact prompt input to the new SDK. */
test('dialog package forwards actions and prompt whitespace without legacy exports', async () => {
  const calls = [];
  const tools = browser(async options => {
    calls.push(structuredClone(options));
    return '{"dialog":null}';
  });
  for (const options of [{ action: 'get' }, { action: 'dismiss' },
    { action: 'accept', promptText: '' }, { action: 'accept', promptText: '  雪  ' }]) {
    assert.equal(await tools.dialog(options), '{"dialog":null}');
    assert.deepEqual(calls.at(-1), options);
  }
  assert.equal(tools.handle_dialog, undefined);
  assert.match(await tools.main(), /\bdialog\b/);
});

/** Preserves the owning browser's failures instead of reporting success. */
test('dialog package propagates a host failure', async () => {
  const failure = new Error('No active browser dialog');
  const tools = browser(async () => { throw failure; });
  await assert.rejects(tools.dialog({ action: 'accept' }), error => error === failure);
});
