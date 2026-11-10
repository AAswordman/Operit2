import assert from 'node:assert/strict';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { readSdkScript, wrapComposeScript } from '../../../../../tools/runtime/compose_scripts.mjs';
import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';

/** Bundles current package source for isolated execution without replacing its implementation. */
export async function bundle(relative) {
  const output = await build({ entryPoints: [fileURLToPath(new URL(relative, import.meta.url))], bundle: true, write: false, format: 'cjs', platform: 'neutral', target: 'es2020' });
  return output.outputFiles[0].text;
}

/** Projects cross-realm data into ordinary assertion objects. */
export function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Models only Host IO while recording all service requests, uploads and saved artifacts. */
export function host(responses = [], environment = {}) {
  const requests = [], writes = [], uploads = [], dirs = new Set(), clipboard = [];
  let now = 1000;
  const tools = {
    Files: {
      exists: async path => ({ exists: dirs.has(path) }),
      mkdir: async path => { dirs.add(path); return { successful: true }; },
      writeBinary: async (path, data) => { writes.push({ path, data }); return { successful: true }; },
      download: async (url, path) => { writes.push({ path, url }); return { successful: true }; },
      readBinary: async path => ({ contentBase64: Buffer.from(path).toString('base64') }),
    },
    System: { sleep: async ms => { now += ms; }, setClipboardText: async text => { clipboard.push(text); return text; } },
    Net: { uploadFile: async config => { uploads.push(plain(config)); return { statusCode: 200, content: JSON.stringify({ files: { status: 'Success', url: 'https://images.test/uploaded.png' } }) }; } },
    SoftwareSettings: {
      readEnvironmentVariable: async (owner, key) => { assert.equal(owner, 'image_workbench'); return { key, exists: Object.hasOwn(environment, key), value: environment[key] ?? null }; },
      writeEnvironmentVariable: async (owner, key, value) => { assert.equal(owner, 'image_workbench'); environment[key] = value; return { key, value }; },
    },
  };
  /** Emulates the existing Host HTTP builder and consumes one explicit response per request. */
  function newBuilder() {
    const builder = { connectTimeout: () => builder, readTimeout: () => builder, writeTimeout: () => builder, build: () => ({ newRequest() {
      const record = {};
      const request = { url: value => { record.url = value; return request; }, method: value => { record.method = value; return request; }, headers: value => { record.headers = plain(value); return request; }, body: value => { record.body = JSON.parse(value); return request; }, build: () => ({ execute: async () => {
        requests.push(record);
        assert.ok(responses.length > 0, `Unexpected request: ${record.url}`);
        const response = responses.shift();
        const statusCode = response.httpStatus ?? 200;
        return { statusCode, content: response.raw ?? JSON.stringify(response), isSuccessful: () => statusCode >= 200 && statusCode < 300 };
      } }) };
      return request;
    } }) };
    return builder;
  }
  return { tools, environment, requests, writes, uploads, clipboard, globals: { console, Tools: tools, OkHttp: { newBuilder }, getEnv: key => environment[key], getPluginConfigDir: () => '/operitapp/plugins/image_workbench', Date: class extends Date { static now() { return now; } } } };
}

/** Executes the actual unified tools with the provided Host IO boundary. */
export async function toolRuntime(io) {
  const context = vm.createContext({ ...io.globals, module: { exports: {} } });
  vm.runInContext(await bundle('../src/tools.ts'), context);
  return context.module.exports;
}

/** Runs the workbench through the production Compose compiler and streamed-tree protocol. */
export async function screen(io, state = {}, toolCall = async () => { throw new Error('Unexpected tool call'); }) {
  const context = vm.createContext({ console, Tools: io.tools, module: { exports: {} }, toolCall });
  vm.runInContext(readSdkScript('ToolPkgComposeDslBridge.rs'), context);
  vm.runInContext(wrapComposeScript(await bundle('../src/ui/workbench.ui.ts')), context);
  const fixture = composeStreamFixture(context);
  fixture.adapt();
  await context.__operit_render_compose_dsl({ executionContextKey: 'workbench-test', theme: { brightness: 'light', colors: {} }, state });
  const page = { context, fixture,
    /** Reads native nodes, including composed content slots. */
    nodes() {
      /** Visits one committed node and all native descendants. */
      function walk(node) { return [node, ...node.children.flatMap(walk), ...Object.values(node.slots).flatMap(children => children.flatMap(walk))]; }
      return walk(fixture.latest.tree);
    },
    /** Dispatches a mounted callback and commits queued asynchronous state updates. */
    async dispatch(node, property = 'onClick', payload) {
      assert.ok(node?.props[property]?.__actionId, `Missing ${property} on ${node?.type}`);
      await context.__operit_dispatch_compose_dsl_action({ actionId: node.props[property].__actionId, payload });
      await page.settle();
    },
    /** Commits the normal asynchronous effect response without remounting the page. */
    async settle() {
      for (let index = 0; index < 4; index++) { await new Promise(resolve => setImmediate(resolve)); await context.__operit_render_compose_dsl({ executionContextKey: 'workbench-test' }); }
    },
  };
  await page.settle();
  return page;
}
