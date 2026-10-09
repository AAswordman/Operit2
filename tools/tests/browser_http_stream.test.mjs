import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { once } from 'node:events';
import test from 'node:test';
import { createBrowserHttpStreamHost } from '../../apps/flutter/app/web/runtime/src/browser_http_stream.ts';

function deferred() {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
}

async function fixture(context, handler) {
  const server = createServer(handler);
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  context.after(() => {
    server.closeAllConnections();
    return new Promise(resolve => server.close(resolve));
  });
  return `http://127.0.0.1:${server.address().port}`;
}

function open(context, host, url, options = {}, id = 'fixture') {
  const head = deferred();
  const chunk = deferred();
  const closed = deferred();
  const state = { heads: [], chunks: [], closes: [], order: [] };
  host.openHttpResponseStream(id, {
    method: 'GET', url, followRedirects: true,
    connectTimeoutSeconds: 2, readTimeoutSeconds: 2, ...options,
  }, value => {
    state.heads.push(value); state.order.push('head'); head.resolve(value);
  }, value => {
    state.chunks.push(value); state.order.push('chunk'); chunk.resolve(value);
  }, error => {
    state.closes.push(error); state.order.push('closed'); closed.resolve(error);
  });
  context.after(() => host.closeHttpByteStream(id));
  return { head: head.promise, chunk: chunk.promise, closed: closed.promise, state };
}

const timeout = { timeout: 5000 };

test('real Fetch delivers SSE metadata and bytes while the HTTP connection is held open', timeout, async context => {
  const url = await fixture(context, (_, response) => {
    response.writeHead(200, { 'Content-Type': 'text/event-stream', 'Mcp-Session-Id': 'fixture-session' });
    response.flushHeaders();
    response.write('data: {"jsonrpc":"2.0","id":1,"result":{}}\n\n');
    // Deliberately no response.end(): protocol consumers must not wait for EOF.
  });
  const host = createBrowserHttpStreamHost();
  const stream = open(context, host, url);
  const head = await stream.head;
  const bytes = await stream.chunk;
  assert.equal(head.statusCode, 200);
  assert.ok(head.headers.some(([key, value]) => key === 'mcp-session-id' && value === 'fixture-session'));
  assert.match(new TextDecoder().decode(bytes), /"id":1/);
  assert.deepEqual(stream.state.order, ['head', 'chunk']);
  assert.deepEqual(stream.state.closes, []);
  host.closeHttpByteStream('fixture');
  host.closeHttpByteStream('fixture');
  assert.match(await stream.closed, /cancelled/);
  assert.equal(stream.state.closes.length, 1);
});

test('non-2xx response metadata precedes its streamed error body', timeout, async context => {
  const url = await fixture(context, (_, response) => {
    response.writeHead(401, { 'Content-Type': 'application/json' });
    response.end('{"error":"authentication required"}');
  });
  const stream = open(context, createBrowserHttpStreamHost(), url);
  assert.equal((await stream.head).statusCode, 401);
  assert.equal(await stream.closed, null);
  assert.deepEqual(stream.state.order, ['head', 'chunk', 'closed']);
  assert.match(new TextDecoder().decode(stream.state.chunks[0]), /authentication required/);
});

test('response header timeout aborts Fetch and notifies close exactly once', timeout, async context => {
  const url = await fixture(context, () => {});
  const host = createBrowserHttpStreamHost();
  const stream = open(context, host, url, { connectTimeoutSeconds: 0.05 });
  assert.match(await stream.closed, /response headers timed out/);
  assert.deepEqual(stream.state.heads, []);
  host.closeHttpByteStream('fixture');
  assert.equal(stream.state.closes.length, 1);
});

test('idle body timeout applies after successful response headers', timeout, async context => {
  const url = await fixture(context, (_, response) => {
    response.writeHead(200, { 'Content-Type': 'text/event-stream' });
    response.flushHeaders();
  });
  const stream = open(context, createBrowserHttpStreamHost(), url, { readTimeoutSeconds: 0.05 });
  assert.equal((await stream.head).statusCode, 200);
  assert.match(await stream.closed, /response body timed out/);
  assert.equal(stream.state.closes.length, 1);
});

test('bodyless accepted notifications expose status without waiting for content', timeout, async context => {
  const url = await fixture(context, (_, response) => { response.writeHead(204); response.end(); });
  const stream = open(context, createBrowserHttpStreamHost(), url);
  assert.equal((await stream.head).statusCode, 204);
  assert.equal(await stream.closed, null);
  assert.deepEqual(stream.state.chunks, []);
});

test('stream id is released before close callback and can be reused reentrantly', timeout, async context => {
  const url = await fixture(context, (_, response) => { response.writeHead(204); response.end(); });
  const host = createBrowserHttpStreamHost();
  const done = deferred();
  const heads = [];
  let count = 0;
  const request = { method: 'GET', url, followRedirects: true };
  const reopen = () => host.openHttpResponseStream('reused', request, head => heads.push(head.statusCode),
    () => assert.fail('unexpected body'), error => {
      assert.equal(error, null);
      if (++count === 1) reopen(); else done.resolve();
    });
  reopen();
  context.after(() => host.closeHttpByteStream('reused'));
  await done.promise;
  assert.deepEqual(heads, [204, 204]);
  assert.equal(count, 2);
});

test('raw chunks preserve split UTF-8 instead of decoding or replacing bytes', timeout, async context => {
  const bytes = new TextEncoder().encode('data: {"text":"实时读取正常"}\n\n');
  const url = await fixture(context, (_, response) => {
    response.writeHead(200, { 'Content-Type': 'text/event-stream' });
    response.write(bytes.subarray(0, 18)); // splits a multi-byte character
    setTimeout(() => response.end(bytes.subarray(18)), 20);
  });
  const stream = open(context, createBrowserHttpStreamHost(), url);
  assert.equal(await stream.closed, null);
  assert.deepEqual(Buffer.concat(stream.state.chunks), Buffer.from(bytes));
});

test('legacy byte stream remains success-only and closes on HTTP errors', timeout, async context => {
  const url = await fixture(context, (_, response) => { response.writeHead(503); response.end('unavailable'); });
  const host = createBrowserHttpStreamHost();
  const done = deferred();
  host.openHttpByteStream('byte', { method: 'GET', url, followRedirects: true },
    () => assert.fail('error must not signal opened'), () => assert.fail('error body must not be delivered'),
    error => done.resolve(error));
  context.after(() => host.closeHttpByteStream('byte'));
  assert.match(await done.promise, /HTTP 503/);
});
