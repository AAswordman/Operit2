import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

/** Runs the real bridge, web snapshot wiring and OkHttp3 wrapper in one sandbox. */
function runtime(native = {}) {
  const requests = [];
  const context = vm.createContext({
    ...native,
    __operitCurrentCallId: 'owner',
    __operitExpose(name, value) { context[name] = value; },
    __operitRetainCallReference() {},
    __operitReleaseCallReference() {},
    __operitActivateCall() {},
    __operitNativeCallToolStructured(scope, type, name, params) {
      requests.push({ type, name, params });
      return Promise.resolve({
        success: true,
        data: { statusCode: 200, statusMessage: 'OK', content: '{"status":"ok"}', headers: '', size: 2 },
      });
    },
  });
  vm.runInContext(readFileSync(new URL('../../hosts/web/src/javascript_promises.js', import.meta.url), 'utf8'), context);
  vm.runInContext(readFileSync(new URL('../../core/crates/plugin/sdk/src/JsExecutionRuntimeBridge.script.js', import.meta.url), 'utf8'), context);
  vm.runInContext(readFileSync(new URL('../../core/crates/plugin/javascript-bridge/src/javascript/OkHttp3.script.js', import.meta.url), 'utf8'), context);
  return { context, requests };
}

/** Flushes the deferred reference release after the public Promise completes. */
async function flush() {
  await Promise.resolve();
  await Promise.resolve();
}

test('GET requests omit the body field so the structured walker accepts them', async () => {
  const { context, requests } = runtime();
  const response = await context.OkHttp.newClient()
    .get('https://api.crossref.org/works?query=machine+learning', { 'User-Agent': 'Operit/1.0' });
  assert.equal(requests.length, 1);
  assert.equal(requests[0].type, 'default');
  assert.equal(requests[0].name, 'http_request');
  assert.equal(requests[0].params.url, 'https://api.crossref.org/works?query=machine+learning');
  assert.equal(requests[0].params.method, 'GET');
  assert.equal(requests[0].params.headers, JSON.stringify({ 'User-Agent': 'Operit/1.0' }));
  assert.equal(Object.hasOwn(requests[0].params, 'body'), false);
  assert.equal(response.statusCode, 200);
  assert.equal(response.isSuccessful(), true);
  await flush();
});

test('POST requests keep their body alongside the other request fields', async () => {
  const { context, requests } = runtime();
  await context.OkHttp.newClient()
    .post('https://example.com/v1', 'payload', { 'Content-Type': 'text/plain' });
  assert.equal(requests.length, 1);
  assert.equal(requests[0].params.method, 'POST');
  assert.equal(requests[0].params.body, 'payload');
  assert.equal(requests[0].params.body_type, 'text');
  await flush();
});

test('requests still reject genuinely non-JSON bodies', async () => {
  const { context, requests } = runtime();
  await assert.rejects(
    context.OkHttp.newClient().post('https://example.com/v1', () => {}, {}),
    /non-JSON value/,
  );
  assert.equal(requests.length, 0);
});
