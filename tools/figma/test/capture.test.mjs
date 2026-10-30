import { test } from "node:test";
import assert from "node:assert/strict";
import { once } from "node:events";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { WebSocketServer } from "ws";
import { main, vmWebSocketUrl } from "../capture.mjs";
test("VM URL conversion preserves authentication and existing WebSocket endpoints", () => {
  assert.equal(
    vmWebSocketUrl("http://127.0.0.1:1234/auth=/"),
    "ws://127.0.0.1:1234/auth=/ws",
  );
  assert.equal(
    vmWebSocketUrl("wss://example.test/auth=/ws"),
    "wss://example.test/auth=/ws",
  );
  assert.throws(() => vmWebSocketUrl("file:///tmp/vm"));
});
test("CLI talks to VM Service, finds the right isolate and writes the returned design without invoking UI actions", async (t) => {
  const server = new WebSocketServer({ port: 0, host: "127.0.0.1" });
  await once(server, "listening");
  t.after(() => {
    for (const client of server.clients) client.terminate();
    server.close();
  });
  const calls = [];
  const document = {
    format: "operit-design",
    version: 1,
    name: "Tool details",
    origin: "flutter-runtime",
    assets: [],
    scenes: [],
    warnings: [],
  };
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
                    ? ["ext.operit.design.capture"]
                    : [],
              }
            : request.params.list
              ? { scopes: ["Operit"] }
              : { document };
      socket.send(JSON.stringify({ jsonrpc: "2.0", id: request.id, result }));
    }),
  );
  const dir = await mkdtemp(join(tmpdir(), "operit-figma-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const file = join(dir, "tool.operit-design.json");
  await main([
    "--vm-url",
    `http://127.0.0.1:${server.address().port}/token=/`,
    "--name",
    "Tool details",
    "--out",
    file,
  ]);
  assert.deepEqual(JSON.parse(await readFile(file, "utf8")), document);
  assert.deepEqual(
    calls.map((c) => c.method),
    ["getVM", "getIsolate", "getIsolate", "ext.operit.design.capture"],
  );
  assert.equal(calls.at(-1).params.isolateId, "ui");
  assert.equal(calls.at(-1).params.name, "Tool details");
  const vmFile = join(dir, "flutter-vm.txt");
  await writeFile(
    vmFile,
    `http://127.0.0.1:${server.address().port}/token=/\n`,
  );
  await main(["--vm-service-file", vmFile, "--out", file]);
  assert.deepEqual(JSON.parse(await readFile(file, "utf8")), document);
});
