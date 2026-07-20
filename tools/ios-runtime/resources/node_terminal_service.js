"use strict";

const fs = require("fs");
const net = require("net");
const path = require("path");
const util = require("util");
const vm = require("vm");
const { createRequire } = require("module");

const readyPath = process.argv[2];
const resourceRoot = process.argv[3];
const sessions = new Map();
let nextSessionId = 1;

/** Serializes one result object as a single UTF-8 JSON line. */
function writeResponse(socket, value) {
  socket.end(`${JSON.stringify(value)}\n`);
}

/** Creates the Node globals and captured console used by one interpreter session. */
function createContext(workingDirectory, output) {
  const requireFromBundle = createRequire(path.join(resourceRoot, "runtime-entry.js"));
  const consoleProxy = {};
  for (const level of ["log", "info", "warn", "error", "dir"]) {
    consoleProxy[level] = (...args) => {
      output.push(`${util.format(...args)}\n`);
    };
  }
  return vm.createContext({
    Buffer,
    URL,
    URLSearchParams,
    clearInterval,
    clearTimeout,
    console: consoleProxy,
    global: undefined,
    process: {
      arch: process.arch,
      cwd: () => workingDirectory,
      env: Object.freeze({ ...process.env }),
      platform: process.platform,
      version: process.version,
      versions: process.versions,
    },
    queueMicrotask,
    require: requireFromBundle,
    setInterval,
    setTimeout,
    TextDecoder,
    TextEncoder,
  });
}

/** Allocates one persistent embedded Node interpreter session. */
function createSession(request) {
  const sessionId = `ios-node-${nextSessionId++}`;
  const output = [`Node.js ${process.version}\n> `];
  const workingDirectory = request.workingDir;
  const context = createContext(workingDirectory, output);
  context.global = context;
  const session = {
    commandRunning: false,
    exitCode: null,
    input: "",
    output,
    rows: request.rows,
    cols: request.cols,
    screen: output.join(""),
    sessionId,
    sessionName: request.sessionName,
    terminalType: "node",
    workingDirectory,
    context,
  };
  sessions.set(sessionId, session);
  return session;
}

/** Runs every complete JavaScript line currently buffered for one terminal session. */
function executeInput(session) {
  const lines = session.input.split(/\r?\n/);
  session.input = lines.pop();
  let emitted = "";
  for (const line of lines) {
    const before = session.output.length;
    session.commandRunning = true;
    session.screen += `> ${line}\n`;
    try {
      const result = vm.runInContext(line, session.context, {
        filename: "ios-terminal",
        displayErrors: false,
      });
      if (result !== undefined) {
        session.output.push(`${util.inspect(result)}\n`);
      }
    } catch (error) {
      session.output.push(`${error.stack || error}\n`);
    }
    session.commandRunning = false;
    session.output.push("> ");
    emitted += session.output.slice(before).join("");
    session.screen += session.output.slice(before).join("");
  }
  return emitted;
}

/** Resolves a session identifier into the corresponding active Node terminal. */
function requiredSession(sessionId) {
  const session = sessions.get(sessionId);
  if (session === undefined) {
    throw new Error(`iOS Node terminal session does not exist: ${sessionId}`);
  }
  return session;
}

/** Handles one native bridge protocol request. */
function handleRequest(request) {
  switch (request.command) {
    case "create": {
      const session = createSession(request);
      return { sessionId: session.sessionId };
    }
    case "write": {
      const session = requiredSession(request.sessionId);
      const acceptedChars = [...request.input].length;
      session.input += request.input;
      return { acceptedChars, output: executeInput(session) };
    }
    case "read": {
      const session = requiredSession(request.sessionId);
      const output = session.output.join("");
      session.output.length = 0;
      return { output };
    }
    case "resize": {
      const session = requiredSession(request.sessionId);
      session.rows = request.rows;
      session.cols = request.cols;
      return {};
    }
    case "poll": {
      const session = requiredSession(request.sessionId);
      return { exitCode: session.exitCode };
    }
    case "close": {
      requiredSession(request.sessionId);
      sessions.delete(request.sessionId);
      return {};
    }
    case "screen": {
      const session = requiredSession(request.sessionId);
      return {
        cols: session.cols,
        commandRunning: session.commandRunning,
        content: session.screen,
        rows: session.rows,
        terminalType: session.terminalType,
      };
    }
    case "list": {
      return {
        sessions: [...sessions.values()].map((session) => ({
          commandRunning: session.commandRunning,
          sessionId: session.sessionId,
          sessionKind: "embedded-interpreter",
          sessionName: session.sessionName,
          terminalType: session.terminalType,
          workingDir: session.workingDirectory,
        })),
      };
    }
    default:
      throw new Error(`unsupported iOS Node runtime command: ${request.command}`);
  }
}

/** Parses and routes one newline-delimited native bridge request. */
function handleConnection(socket) {
  let source = "";
  socket.setEncoding("utf8");
  socket.on("data", (chunk) => {
    source += chunk;
  });
  socket.on("end", () => {
    try {
      const request = JSON.parse(source);
      writeResponse(socket, { ok: true, result: handleRequest(request) });
    } catch (error) {
      writeResponse(socket, { ok: false, error: String(error.stack || error) });
    }
  });
}

/** Starts the loopback-only native bridge server and publishes its selected port. */
function startServer() {
  const server = net.createServer(handleConnection);
  server.listen(0, "127.0.0.1", () => {
    const address = server.address();
    fs.writeFileSync(readyPath, String(address.port), "utf8");
  });
}

startServer();
