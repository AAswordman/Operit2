import { test } from "node:test";
import assert from "node:assert/strict";
import { once } from "node:events";
import { WebSocketServer } from "ws";
import { connectRuntime, createLiveServer, validateRequest } from "../live.mjs";
const snapshot = (revision) => ({
  protocol: "operit-design-runtime",
  version: 1,
  revision,
  document: {
    format: "operit-design",
    version: 1,
    name: "Fixture",
    origin: "flutter-runtime",
    scenes: [],
    assets: [],
    warnings: [],
  },
  edits: [],
});
const headers = (token) => ({
  Authorization: `Bearer ${token}`,
  "Content-Type": "application/json",
});
async function setup(t, request = async () => snapshot(1), origins = []) {
  const live = await createLiveServer({
    runtime: { request, close() {} },
    html: "<!doctype html><title>Live</title>",
    origins,
  });
  t.after(() => live.close());
  return live;
}
test("only runtime operations and finite input values cross the transport", () => {
  assert.deepEqual(
    validateRequest({
      operation: "event",
      event: { type: "input", target: "live-1", text: "设计", ignored: true },
    }),
    {
      operation: "event",
      event: { type: "input", target: "live-1", text: "设计" },
    },
  );
  for (const value of [
    null,
    {},
    { operation: "evaluate" },
    { operation: "edit", target: "live-1" },
    { operation: "event", event: { type: "tap", target: "live-1", x: NaN } },
    {
      operation: "event",
      event: { type: "input", target: "live-1", text: "x".repeat(65537) },
    },
  ])
    assert.throws(() => validateRequest(value));
});
test("unauthorized requests, foreign origins, oversized payloads and invalid operations do not reach the app", async (t) => {
  const calls = [],
    live = await setup(t, async (request) => {
      calls.push(request);
      return snapshot(1);
    });
  const post = (body, extra = {}) =>
    fetch(live.origin + "/runtime", {
      method: "POST",
      headers: { ...headers(live.token), ...extra },
      body: JSON.stringify(body),
    });
  assert.equal(
    (await post({ operation: "snapshot" }, { Authorization: "Bearer wrong" }))
      .status,
    401,
  );
  assert.equal(
    (await post({ operation: "snapshot" }, { Origin: "https://other.test" }))
      .status,
    403,
  );
  assert.equal((await post({ operation: "evaluate" })).status, 400);
  assert.equal(
    (
      await post({
        operation: "edit",
        target: "a",
        properties: { text: "a".repeat(270000) },
      })
    ).status,
    413,
  );
  assert.equal(
    (await fetch(live.origin + "/runtime?token=" + live.token)).status,
    404,
  );
  assert.deepEqual(calls, []);
  assert.equal((await post({ operation: "snapshot" })).status, 200);
  assert.deepEqual(calls, [{ operation: "snapshot" }]);
});
test("explicit preview origins are allowed and runtime errors do not stall the ordered request queue", async (t) => {
  let active = 0,
    maxActive = 0,
    revision = 0;
  const live = await setup(
    t,
    async (request) => {
      active++;
      maxActive = Math.max(maxActive, active);
      await new Promise((resolve) => setTimeout(resolve, 20));
      active--;
      if (request.operation === "edit") throw new Error("unsupported");
      return snapshot(++revision);
    },
    ["https://preview.test"],
  );
  const options = {
    method: "OPTIONS",
    headers: {
      Origin: "https://preview.test",
      "Access-Control-Request-Method": "POST",
    },
  };
  const preflight = await fetch(live.origin + "/runtime", options);
  assert.equal(preflight.status, 204);
  assert.equal(
    preflight.headers.get("Access-Control-Allow-Origin"),
    "https://preview.test",
  );
  const requests = [
    { operation: "edit", target: "a", properties: { fill: "#000000ff" } },
    { operation: "snapshot" },
    { operation: "snapshot" },
  ];
  const responses = await Promise.all(
    requests.map((request) =>
      fetch(live.origin + "/runtime", {
        method: "POST",
        headers: { ...headers(live.token), Origin: "https://preview.test" },
        body: JSON.stringify(request),
      }),
    ),
  );
  assert.deepEqual(
    responses.map((r) => r.status),
    [502, 200, 200],
  );
  assert.equal(maxActive, 1);
});
test("the VM adapter finds the UI isolate and forwards scope and operations to the real extension method", async (t) => {
  const server = new WebSocketServer({ host: "127.0.0.1", port: 0 });
  await once(server, "listening");
  t.after(() => {
    for (const client of server.clients) client.terminate();
    server.close();
  });
  const calls = [];
  server.on("connection", (socket) =>
    socket.on("message", (data) => {
      const request = JSON.parse(data.toString());
      calls.push(request);
      const result =
        request.method === "getVM"
          ? { isolates: [{ id: "worker" }, { id: "ui" }] }
          : request.method === "getIsolate"
            ? {
                extensionRPCs:
                  request.params.isolateId === "ui"
                    ? ["ext.operit.design.runtime"]
                    : [],
              }
            : snapshot(1);
      socket.send(JSON.stringify({ jsonrpc: "2.0", id: request.id, result }));
    }),
  );
  const runtime = await connectRuntime(
    `http://127.0.0.1:${server.address().port}/auth=/`,
    "Operit",
  );
  t.after(() => runtime.close());
  const message = {
    operation: "event",
    event: { type: "tap", target: "live-3", x: 2, y: 4 },
  };
  assert.equal((await runtime.request(message)).revision, 1);
  assert.equal(calls.at(-1).method, "ext.operit.design.runtime");
  assert.equal(calls.at(-1).params.scope, "Operit");
  assert.equal(calls.at(-1).params.isolateId, "ui");
  assert.deepEqual(JSON.parse(calls.at(-1).params.request), message);
});
