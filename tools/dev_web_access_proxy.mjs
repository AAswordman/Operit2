import http from "node:http";
import net from "node:net";

const DEFAULT_UPSTREAM_HOST = "127.0.0.1";
const DEFAULT_UPSTREAM_PORT = 4835;
const DEFAULT_LISTEN_HOST = "127.0.0.1";
const DEFAULT_LISTEN_PORT = 4836;

/** Parses one positive TCP port supplied through the command line. */
function parsePort(value, option) {
  const port = Number.parseInt(value, 10);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(`${option} must be a TCP port between 1 and 65535`);
  }
  return port;
}

/** Reads the isolated Web development proxy configuration from command-line arguments. */
function parseArguments(argumentsList) {
  const config = {
    upstreamHost: DEFAULT_UPSTREAM_HOST,
    upstreamPort: DEFAULT_UPSTREAM_PORT,
    listenHost: DEFAULT_LISTEN_HOST,
    listenPort: DEFAULT_LISTEN_PORT,
  };

  for (let index = 0; index < argumentsList.length; index += 1) {
    const option = argumentsList[index];
    const value = argumentsList[index + 1];
    if (option === "--upstream-host") {
      if (value === undefined) throw new Error("--upstream-host requires a value");
      config.upstreamHost = value;
      index += 1;
      continue;
    }
    if (option === "--upstream-port") {
      if (value === undefined) throw new Error("--upstream-port requires a value");
      config.upstreamPort = parsePort(value, option);
      index += 1;
      continue;
    }
    if (option === "--listen-host") {
      if (value === undefined) throw new Error("--listen-host requires a value");
      config.listenHost = value;
      index += 1;
      continue;
    }
    if (option === "--listen-port") {
      if (value === undefined) throw new Error("--listen-port requires a value");
      config.listenPort = parsePort(value, option);
      index += 1;
      continue;
    }
    throw new Error(`unknown option: ${option}`);
  }

  return config;
}

/** Returns headers that enable the browser features required by threaded Web inference. */
function isolationHeaders() {
  return {
    "cross-origin-opener-policy": "same-origin",
    "cross-origin-embedder-policy": "require-corp",
    "cross-origin-resource-policy": "same-origin",
  };
}

/** Builds upstream request options while preserving the browser request headers. */
function upstreamRequestOptions(request, config) {
  return {
    host: config.upstreamHost,
    port: config.upstreamPort,
    method: request.method,
    path: request.url,
    headers: {
      ...request.headers,
      host: `${config.upstreamHost}:${config.upstreamPort}`,
    },
  };
}

/** Forwards one HTTP request while adding cross-origin isolation response headers. */
function proxyHttpRequest(request, response, config) {
  const upstreamRequest = http.request(
    upstreamRequestOptions(request, config),
    (upstreamResponse) => {
      response.writeHead(upstreamResponse.statusCode ?? 502, {
        ...upstreamResponse.headers,
        ...isolationHeaders(),
      });
      upstreamResponse.pipe(response);
    },
  );
  upstreamRequest.on("error", (error) => {
    response.writeHead(502, {
      "content-type": "text/plain; charset=utf-8",
      ...isolationHeaders(),
    });
    response.end(`Flutter Web development server is unavailable: ${error.message}`);
  });
  request.pipe(upstreamRequest);
}

/** Forwards a Flutter debug WebSocket upgrade without changing its protocol frames. */
function proxyWebSocketUpgrade(request, socket, head, config) {
  const upstreamSocket = net.connect({ host: config.upstreamHost, port: config.upstreamPort });
  upstreamSocket.once("connect", () => {
    const headers = { ...request.headers, host: `${config.upstreamHost}:${config.upstreamPort}` };
    const requestLines = [`${request.method} ${request.url} HTTP/${request.httpVersion}`];
    for (const [name, value] of Object.entries(headers)) {
      requestLines.push(`${name}: ${value}`);
    }
    upstreamSocket.write(`${requestLines.join("\r\n")}\r\n\r\n`);
    if (head.length > 0) upstreamSocket.write(head);
    socket.pipe(upstreamSocket);
    upstreamSocket.pipe(socket);
  });
  upstreamSocket.once("error", () => socket.destroy());
  socket.once("error", () => upstreamSocket.destroy());
}

/** Starts the development proxy and reports the isolated browser origin. */
function startServer(config) {
  const server = http.createServer((request, response) => {
    proxyHttpRequest(request, response, config);
  });
  server.on("upgrade", (request, socket, head) => {
    proxyWebSocketUpgrade(request, socket, head, config);
  });
  server.listen(config.listenPort, config.listenHost, () => {
    console.log(`Web Access development proxy: http://${config.listenHost}:${config.listenPort}`);
    console.log(`Flutter development server: http://${config.upstreamHost}:${config.upstreamPort}`);
  });
}

startServer(parseArguments(process.argv.slice(2)));
