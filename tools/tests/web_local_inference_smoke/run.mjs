import { mkdtempSync, rmSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const fixtureRoot = process.argv[2];
const chromePath = process.argv[3];
if (!fixtureRoot || !chromePath) {
  throw new Error("Usage: node run.mjs <fixture-root> <chrome-path>");
}

/** Allocates one currently unused loopback TCP port. */
function freePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address !== null ? address.port : 0;
      server.close((error) => {
        if (error) reject(error);
        else resolve(port);
      });
    });
  });
}

/** Waits until one child process emits a line containing an exact marker. */
function waitForOutput(child, marker) {
  return new Promise((resolve, reject) => {
    let output = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      output += chunk;
      if (output.split(/\r?\n/).some((line) => line.startsWith(marker))) {
        resolve();
      }
    });
    child.stderr.on("data", (chunk) => process.stderr.write(chunk));
    child.once("exit", (code) => reject(new Error(`Child exited before ${marker}: ${code}`)));
    child.once("error", reject);
  });
}

/** Waits for Chrome to publish one inspectable page target. */
async function waitForPageTarget(port) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      const targets = await response.json();
      const page = targets.find((target) => target.type === "page");
      if (page?.webSocketDebuggerUrl) {
        return page.webSocketDebuggerUrl;
      }
    } catch (error) {
      if (Date.now() >= deadline) throw error;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("Chrome DevTools page target did not become ready");
}

/** Opens one Chrome DevTools websocket with request-response correlation. */
async function openDevTools(url) {
  const socket = new WebSocket(url);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  let nextId = 1;
  const pending = new Map();
  const events = [];
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const request = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) request.reject(new Error(JSON.stringify(message.error)));
      else request.resolve(message.result);
      return;
    }
    events.push(message);
  });

  /** Sends one Chrome DevTools command. */
  function send(method, params = {}) {
    const id = nextId++;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }
  return { socket, send, events };
}

/** Polls the smoke-test document until it reports a terminal result. */
async function waitForSmokeResult(devTools) {
  const deadline = Date.now() + 300_000;
  while (Date.now() < deadline) {
    const evaluated = await devTools.send("Runtime.evaluate", {
      expression: "document.getElementById('result')?.textContent || ''",
      returnByValue: true,
    });
    const value = evaluated.result.value;
    if (typeof value === "string" && value.startsWith("PASS")) {
      return value;
    }
    if (typeof value === "string" && value.startsWith("FAIL")) {
      throw new Error(value);
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  const exceptions = devTools.events
    .filter((event) => event.method === "Runtime.exceptionThrown")
    .map((event) => event.params.exceptionDetails.text);
  throw new Error(`Web local inference smoke test timed out: ${exceptions.join(" | ")}`);
}

/** Stops one child process and its Windows subprocess tree. */
function stopChild(child) {
  if (!child || child.exitCode !== null) {
    return;
  }
  if (process.platform === "win32") {
    spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
  } else {
    child.kill("SIGTERM");
  }
}

/** Runs the complete browser local inference smoke test. */
async function main() {
  const appPort = await freePort();
  const devToolsPort = await freePort();
  const profile = mkdtempSync(join(tmpdir(), "operit-web-smoke-"));
  const server = spawn(
    process.execPath,
    [fileURLToPath(new URL("./server.mjs", import.meta.url)), fixtureRoot, String(appPort)],
    { stdio: ["ignore", "pipe", "pipe"] },
  );
  let chrome;
  let devTools;
  try {
    await waitForOutput(server, "READY ");
    chrome = spawn(
      chromePath,
      [
        "--headless=new",
        "--no-sandbox",
        "--disable-gpu",
        "--disable-dev-shm-usage",
        `--remote-debugging-port=${devToolsPort}`,
        `--user-data-dir=${profile}`,
        `http://127.0.0.1:${appPort}/`,
      ],
      { stdio: ["ignore", "ignore", "pipe"] },
    );
    chrome.stderr.setEncoding("utf8");
    chrome.stderr.on("data", (chunk) => process.stderr.write(chunk));
    const debuggerUrl = await waitForPageTarget(devToolsPort);
    devTools = await openDevTools(debuggerUrl);
    await devTools.send("Runtime.enable");
    console.log(await waitForSmokeResult(devTools));
  } finally {
    devTools?.socket.close();
    stopChild(chrome);
    stopChild(server);
    rmSync(profile, { recursive: true, force: true });
  }
}

await main();
