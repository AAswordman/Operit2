import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import { wrapComposeScript } from '../runtime/compose_scripts.mjs';
import { compileComposeModule } from '../../plugins/tools/compile_compose_module.mjs';

const sdk = new URL('../../core/crates/plugin/sdk/src/toolpkg/', import.meta.url);
/** Loads only the production compiler entry; artifact loading must not need Acorn. */
function runtime() {
  const context = vm.createContext({});
  vm.runInContext(readFileSync(new URL('compose/Artifact.js', sdk), 'utf8'), context);
  vm.runInContext(readFileSync(new URL('compose/Compiler.js', sdk), 'utf8'), context);
  return context;
}
const source = `module.exports.default = function(ctx) {
  const [count, setCount] = ctx.useState('count', 0);
  return ctx.UI.Button({ onClick: () => setCount(count + 1) }, [ctx.UI.Text({ text: count })]);
};`;

test('build artifact is deterministic, portable, idempotent and loads without parser or lowering', () => {
  const artifact = compileComposeModule(source, 'test:screen.js');
  assert.match(artifact, /^\/\/ @operit-compose-artifact 4\n/);
  assert.match(artifact, /OperitComposeReactive\.frame/);
  assert.equal(artifact, compileComposeModule(source, 'test:screen.js'));
  assert.equal(artifact, compileComposeModule(artifact, 'test:screen.js'));
  assert.notEqual(artifact, compileComposeModule(source, 'other:screen.js'));
  const context = runtime();
  assert.equal(context.OperitComposeCompiler.compile(artifact, 'runtime:screen.js'), artifact);
  context.OperitComposeCompiler.compile = () => { throw Error('runtime lowering must not run'); };
  assert.equal(context.OperitComposeCompiler.prepare(artifact, 'runtime:screen.js'), artifact);
  // Build-time compilation must not execute top-level plugin code.
  assert.doesNotThrow(() => compileComposeModule('throw Error("not build code");\n' + source, 'test:unexecuted.js'));
});

test('unsupported or incomplete artifacts fail before execution rather than falling back to recompilation', () => {
  const { OperitComposeCompiler: compiler } = runtime();
  for (const input of ['// @operit-compose-artifact 999\nthrow Error("never run")', '// @operit-compose-artifact 1']) {
    for (const method of ['prepare', 'compile', 'artifact']) assert.throws(() => compiler[method](input, 'test'), /Unsupported Compose artifact version/);
  }
});

test('legacy sources use the unchanged compiler and strings/comments do not masquerade as artifacts', () => {
  const context = runtime();
  const calls = [];
  context.OperitComposeCompiler.compile = (text, identity) => { calls.push([text, identity]); return 'lowered'; };
  for (const text of [source, 'const text = "// @operit-compose-artifact 1";', '/* // @operit-compose-artifact 1 */\n' + source]) {
    assert.equal(context.OperitComposeCompiler.prepare(text, 'legacy'), 'lowered');
    assert.deepEqual(calls.at(-1), [text, 'legacy']);
  }
});

test('artifact body exactly matches runtime lowering from the same SDK compiler', () => {
  const context = runtime();
  context.module = { exports: {} }; context.exports = context.module.exports;
  vm.runInContext(readFileSync(new URL('vendor/acorn.js', sdk), 'utf8'), context);
  context.__operitAcorn = context.module.exports;
  const artifact = compileComposeModule(source, 'test:screen.js');
  assert.equal(artifact.slice(artifact.indexOf('\n') + 1), context.OperitComposeCompiler.compile(source, 'test:screen.js'));
  assert.throws(() => compileComposeModule('function {', 'invalid'), /Unexpected token/);
});

/** Uses the production lazy parser loader with an explicit trusted SDK source. */
function parserRuntime(parserSource = readFileSync(new URL('vendor/acorn.js', sdk), 'utf8')) {
  const context = runtime();
  vm.runInContext(readFileSync(new URL('compose/Parser.js', sdk), 'utf8')
    .replace('__OPERIT_COMPOSE_PARSER_SOURCE__', () => 'function() { return ' + JSON.stringify(parserSource) + '; }'), context);
  vm.runInContext(`
    globalThis.parserInitializations = 0;
    const originalFunction = Function;
    globalThis.Function = function(...args) { parserInitializations++; return originalFunction(...args); };
  `, context);
  return context;
}

test('precompiled pages never initialize the parser; old source initializes it once per engine', () => {
  const artifact = compileComposeModule(source, 'lazy-test');
  const context = parserRuntime();
  assert.equal(context.parserInitializations, 0);
  assert.equal(context.OperitComposeCompiler.prepare(artifact, 'lazy-test'), artifact);
  assert.equal(context.parserInitializations, 0);
  assert.throws(() => context.OperitComposeCompiler.prepare('// @operit-compose-artifact 999\n', 'bad'), /Unsupported/);
  assert.equal(context.parserInitializations, 0, 'version errors must not load the parser');
  assert.equal(context.OperitComposeCompiler.prepare(source, 'lazy-test'), artifact.slice(artifact.indexOf('\n') + 1));
  assert.equal(context.parserInitializations, 1);
  assert.throws(() => context.OperitComposeCompiler.prepare('function {', 'invalid'), /Unexpected token/);
  context.OperitComposeCompiler.prepare(source, 'second');
  assert.equal(context.parserInitializations, 1, 'syntax errors do not invalidate an initialized parser');
  const other = parserRuntime();
  assert.equal(other.parserInitializations, 0);
  other.OperitComposeCompiler.prepare(source, 'other-page');
  assert.equal(other.parserInitializations, 1);
});

test('failed parser initialization preserves the exact error without retry or partial globals', () => {
  for (const failure of [new Error('Injected initialization failure'), undefined, null, false, 0, '']) {
    const context = parserRuntime('exports.parse = function() {}; throw globalThis.parserFailure;');
    context.parserFailure = failure;
    for (let attempt = 0; attempt < 3; attempt++) {
      let caught = false;
      try { context.OperitComposeCompiler.prepare(source, 'failed-parser'); }
      catch (error) { caught = true; assert.equal(error, failure); }
      assert.equal(caught, true);
    }
    assert.equal(context.parserInitializations, 1);
    assert.equal(context.exports, undefined);
    assert.equal(context.module, undefined);
    assert.equal(context.acorn, undefined);
  }
});

test('runtime registration preserves a screen expression without a trailing semicolon', () => {
  const context = vm.createContext({ module: { exports: {} } });
  vm.runInContext(wrapComposeScript(`module.exports.default = function() {
    throw Error('Screen entry must only run when rendered');
  }`), context);
  assert.equal(typeof context.module.exports.default, 'function');
  assert.equal(typeof context.__operit_render_compose_dsl, 'function');
});

/** Removed executable ABIs must be rebuilt rather than retaining a second renderer. */
test('old artifacts require rebuilding with the program ABI', () => {
  const context = runtime();
  for (const version of [1, 2, 3]) {
    const artifact = '// @operit-compose-artifact ' + version + '\nmodule.exports.default = ctx => ctx.UI.Text({text: "existing"});';
    for (const method of ['prepare', 'compile', 'artifact'])
      assert.throws(() => context.OperitComposeCompiler[method](artifact, 'existing'), /Unsupported Compose artifact version/);
  }
});
