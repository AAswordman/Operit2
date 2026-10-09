import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import path from 'node:path';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const require = createRequire(new URL('plugins/packages/buildin/workflow/package.json', root));
const ts = require('typescript');

/** Reads a repository source without using stale compiled plugin artifacts. */
function source(path) { return readFileSync(new URL(path, root), 'utf8'); }

/** Extracts production JavaScript embedded in the first Rust raw string. */
function embedded(path) {
  const text = source(path);
  const start = text.indexOf('r#"') + 3;
  return text.slice(start, text.indexOf('"#', start));
}

/** Copies cross-realm JSON values for exact assertions. */
function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Installs the public transport using its real active-call lifecycle contract. */
function transport() {
  const calls = [];
  const pending = [];
  let retained = 0;
  const context = vm.createContext({
    __operitCurrentCallId: 'caller',
    /** Returns the caller execution parameters. */
    __operitGetCallState() { return { params: { __operit_toolpkg_api_version: '2.0.0' } }; },
    /** Installs the API namespace. */
    __operitExpose(name, value) { context[name] = value; },
    /** Retains the owner while the native request is pending. */
    __operitRetainCallReference() { retained++; },
    /** Releases the matching owner reference. */
    __operitReleaseCallReference() { retained--; },
    /** Restores the owner before delivering an asynchronous completion. */
    __operitActivateCall(id) { context.__operitCurrentCallId = id; },
    /** Captures the native ABI, which contains no caller identity argument. */
    __operitNativeCallDependency(...args) {
      calls.push(args);
      return new Promise((resolve, reject) => pending.push({ resolve, reject }));
    },
  });
  vm.runInContext(embedded('core/crates/plugin/sdk/src/toolpkg/ToolPkgApiRuntimeScript.rs'), context);
  vm.runInContext(embedded('core/crates/plugin/sdk/src/toolpkg/ToolPkgRegistrationBridge.rs')
    .replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'true'), context);
  return { context, calls, pending, retained: () => retained };
}

/** Checks public methods are captured only as durable exported function references. */
test('registerApi captures public names independently from implementation names', () => {
  const { context } = transport();
  vm.runInContext(`
    /** Implements the public operation in the fixture package. */
    async function actualRun(event) { return event.payload; }
    /** Exposes the module's durable function identity. */
    __operitGetActiveModuleExports = function() { return { actualRun: actualRun }; };
    ToolPkg.registerApi({ name: 'run', function: actualRun });
  `, context);
  assert.deepEqual(JSON.parse(context.__operitToolPkgRegistrationCapture.publicApis[0]), { id: 'run', function: 'actualRun' });
  assert.throws(() => vm.runInContext("ToolPkg.registerApi({name: 'bad', function: 'privateName'})", context), /function reference/);
  assert.throws(() => vm.runInContext("ToolPkg.registerApi({name: '', function: actualRun})", context), /name/);
});

/** Checks structured payload and caller identity remain separate at the native boundary. */
test('dependency call preserves application results and releases the owning call', async () => {
  const { context, calls, pending, retained } = transport();
  const result = context.ToolPkg.callDependency('provider', 'run', { callerPackage: 'forged', value: 1 });
  assert.equal(retained(), 1);
  assert.equal(calls[0].length, 4);
  assert.deepEqual(calls[0].slice(0, 3), ['caller', 'provider', 'run']);
  assert.deepEqual(plain(calls[0][3]), { callerPackage: 'forged', value: 1 });
  pending[0].resolve({ success: true, value: { success: false, application: 'value' } });
  assert.deepEqual(plain(await result), { success: false, application: 'value' });
  await Promise.resolve();
  assert.equal(retained(), 0);
  assert.equal(Object.keys(context).some(key => key.startsWith('__operit_dependency_')), false);
});

/** Checks registration, remote failures, and submission errors never execute another route. */
test('dependency call rejects forbidden phases and failures without retry', async () => {
  const { context, calls, pending, retained } = transport();
  const failed = context.ToolPkg.callDependency('provider', 'private', {});
  pending[0].resolve({ success: false, message: 'Public API not found' });
  await assert.rejects(failed, /Public API not found/);
  await Promise.resolve();
  assert.equal(retained(), 0);
  context.__operitNativeCallDependency = () => { throw new Error('transport stopped'); };
  await assert.rejects(context.ToolPkg.callDependency('provider', 'run', {}), /transport stopped/);
  await Promise.resolve();
  assert.equal(retained(), 0);
  context.__operitGetCallState = () => ({ params: { __operit_registration_mode: true } });
  await assert.rejects(context.ToolPkg.callDependency('provider', 'run', {}), /registration/);
  assert.equal(calls.length, 1);
});

/** Loads current TypeScript sources in memory for contract tests; writes no build artifacts. */
function workflowRuntime() {
  const cache = new Map();
  const publicMethods = new Map();
  const context = vm.createContext({
    console,
    PluginConfig: {
      /** Creates the fixture database without reading or writing host storage. */
      async use(_name, initial) { return structuredClone(initial); },
      /** Acknowledges the fixture's in-memory persistence boundary. */
      async flush() {},
    },
    /** Returns fixture tool output during actual workflow engine execution. */
    async toolCall(name) { if (name === 'fail') throw new Error('fixture tool failure'); return 'ok'; },
    Tools: {},
    /** Returns a deterministic locale for manifest extension loading. */
    getLang() { return 'en'; },
    ToolPkg: {
      /** Captures the registered public methods for the fixture host. */
      registerApi(definition) { publicMethods.set(definition.name, definition.function); },
      /** Executes only an explicitly published public operation. */
      async callDependency(packageName, method, payload) {
        assert.equal(packageName, 'com.operit.workflow');
        assert.ok(publicMethods.has(method));
        return publicMethods.get(method)({ callerPackage: 'consumer', payload: plain(payload) });
      },
    },
  });
  /** Resolves only local modules from the workflow source tree. */
  function load(name) {
    if (cache.has(name)) return cache.get(name).exports;
    const module = { exports: {} };
    cache.set(name, module);
    const javascript = ts.transpileModule(source(`plugins/packages/buildin/workflow/src/${name}.ts`), {
      compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
    }).outputText;
    const factory = vm.runInContext(`(function(module,exports,require){${javascript}\n})`, context);
    factory(module, module.exports, request => {
      assert.match(request, /^\.\/[a-z-]+$/);
      return load(request.slice(2));
    });
    return module.exports;
  }
  const api = load('public-api');
  for (const [name, implementation] of Object.entries({ getAll: api.getAll, create: api.create, get: api.get,
    update: api.updateWorkflow, patch: api.patchWorkflow, setEnabled: api.setEnabled,
    delete: api.deleteWorkflow, trigger: api.triggerWorkflow })) {
    publicMethods.set(name, implementation);
  }
  const client = load('api').workflow;
  vm.runInContext(source('core/crates/plugin/sdk/src/compat/v1/workflow.js'), context);
  const legacy = context.__operitCreateV1Workflow((name, payload) => context.ToolPkg.callDependency('com.operit.workflow', name, payload));
  return { context, client, legacy, service: load('service') };
}

/** Exercises the copied client and v1 adapter against the same real provider service. */
test('workflow public client and v1 API share create/read/update/patch/enable/delete', async () => {
  const { client, legacy } = workflowRuntime();
  const created = await legacy.create('example', 'description', [
    { id: 'start', type: 'trigger', name: 'Start' },
    { id: 'execute', type: 'execute', name: 'Execute', actionType: 'echo', actionConfig: { text: 'hello' } },
  ], [{ id: 'edge', sourceNodeId: 'start', targetNodeId: 'execute' }], true);
  assert.equal(created.name, 'example');
  assert.deepEqual(plain(created.nodes[1].actionConfig), { text: { value: 'hello' } });
  assert.equal((await client.getAll()).totalCount, 1);
  assert.equal((await client.get(created.id)).id, created.id);
  const renamed = await client.update(created.id, { name: 'renamed' });
  assert.equal(renamed.name, 'renamed');
  assert.equal(renamed.nodes.length, 2);
  const patched = await legacy.patch(created.id, { node_patches: [{ op: 'update', id: 'execute', node: { type: 'execute', name: 'new name' } }] });
  assert.equal(patched.nodes[1].name, 'new name');
  assert.equal(patched.nodes[1].actionType, 'echo');
  assert.equal((await legacy.disable(created.id)).enabled, false);
  await assert.rejects(client.trigger(created.id), /停用/);
  assert.equal((await client.enable(created.id)).enabled, true);
  assert.match(await legacy.trigger(created.id), /completed successfully/);
  assert.equal((await client.get(created.id)).successfulExecutions, 1);
  assert.match(await legacy.delete(created.id), /Deleted workflow/);
  assert.equal((await client.getAll()).totalCount, 0);
});

/** Checks invalid graph changes never partially replace stored data. */
test('workflow public mutations validate before commit and report unsupported legacy semantics', async () => {
  const { client } = workflowRuntime();
  const created = await client.create('initial');
  await assert.rejects(client.patch(created.id, { node_patches: [{ op: 'update', id: 'missing', node: { type: 'trigger' } }] }), /not found/);
  assert.equal((await client.get(created.id)).name, 'initial');
  await assert.rejects(client.create('unsupported', '', [{ type: 'trigger', triggerType: 'tasker' }]), /触发类型/);
  await assert.rejects(client.create('unsupported', '', [{ type: 'extract', defaultValue: 'hidden result' }]), /defaultValue/);
  assert.equal((await client.getAll()).totalCount, 1);
});

/** Ensures a failed workflow is not converted into a successful public trigger response. */
test('workflow trigger reports the actual failed execution', async () => {
  const { client } = workflowRuntime();
  const created = await client.create('failure', '', [
    { id: 'start', type: 'trigger' }, { id: 'execute', type: 'execute', actionType: 'fail' },
  ], [{ id: 'edge', sourceNodeId: 'start', targetNodeId: 'execute' }]);
  await assert.rejects(client.trigger(created.id), /FAILED/);
  assert.equal((await client.get(created.id)).failedExecutions, 1);
});

/** Checks the distributable contract is a single source file with no external imports. */
test('manifest public_api names a self-contained client and the packer includes it', () => {
  const manifest = JSON.parse(source('plugins/packages/buildin/workflow/manifest.json'));
  const text = source('plugins/packages/buildin/workflow/' + manifest.public_api);
  const ast = ts.createSourceFile('api.ts', text, ts.ScriptTarget.Latest, true);
  assert.equal(ast.statements.some(node => ts.isImportDeclaration(node) || (ts.isExportDeclaration(node) && node.moduleSpecifier)), false);
  assert.match(source('plugins/packages/buildin/workflow/web/scripts/pack.mjs'), /files\[manifest\.public_api\]/);
});

/** Checks the copied client accepts valid calls and rejects invalid method/argument shapes. */
test('workflow single-file client supplies strict positive and negative type contracts', () => {
  const fixture = path.resolve('plugins/packages/buildin/workflow/src/__public_api_type_test__.ts');
  const text = `import {workflow} from './api';
    const name: Promise<string> = workflow.get('id').then(value => value.name);
    workflow.create('name', '', [{type:'trigger', triggerType:'manual'}]);
    // @ts-expect-error The identifier must remain a string.
    workflow.get(123);
    // @ts-expect-error Private methods must not appear in the public client.
    workflow.privateInternalMethod();
    // @ts-expect-error Enabled state must remain boolean.
    workflow.setEnabled('id', 'true');
  `;
  const options = { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS,
    strict: true, noEmit: true, skipLibCheck: true, lib: ['lib.es2020.d.ts'], types: [] };
  const host = ts.createCompilerHost(options);
  const read = host.readFile.bind(host);
  const exists = host.fileExists.bind(host);
  /** Supplies an in-memory test source without writing build artifacts. */
  host.readFile = file => path.resolve(file) === fixture ? text : read(file);
  /** Makes only the virtual fixture visible in addition to the actual source tree. */
  host.fileExists = file => path.resolve(file) === fixture || exists(file);
  const program = ts.createProgram([fixture, path.resolve('plugins/types/index.d.ts')], options, host);
  const diagnostics = ts.getPreEmitDiagnostics(program);
  assert.deepEqual(diagnostics.map(item => ts.flattenDiagnosticMessageText(item.messageText, '\n')), []);
});
