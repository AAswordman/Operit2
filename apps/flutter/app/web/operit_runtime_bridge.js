(function () {
  const textEncoder = new TextEncoder();
  const textDecoder = new TextDecoder();
  const runtimePrefix = "operit2.runtime.";
  const filePrefix = "operit2.files.";
  const sqlitePrefix = "operit2.sqlite.";
  const sqliteConnections = new Map();
  const sqliteTransactions = new Map();
  let sqliteConnectionIndex = 0;
  let sqliteTransactionIndex = 0;
  let sqliteModulePromise;
  let SQLite;

  const webAccessSessionStorageKey = "operit2.webAccess.session";
  const pairingServiceVersion = 1;
  const webAccessConfig = globalThis.__OPERIT_WEB_ACCESS__;
  if (webAccessConfig && webAccessConfig.mode === "pair") {
    installPairingWebRuntime(webAccessConfig);
    return;
  }

  function installPairingWebRuntime(config) {
    const baseUrl = String(config.baseUrl || "").replace(/\/+$/, "");
    const runtimePromise = webAccessSession(baseUrl).then(createLinkedWebRuntime);
    globalThis.__operitRuntime = {
      async call(request) {
        return (await runtimePromise).call(request);
      },
      async watchSnapshot(request) {
        return (await runtimePromise).watchSnapshot(request);
      },
      async watchStream(request) {
        return (await runtimePromise).watchStream(request);
      },
      async pollWatchStream(subscriptionId) {
        return (await runtimePromise).pollWatchStream(subscriptionId);
      },
      async closeWatchStream(subscriptionId) {
        return (await runtimePromise).closeWatchStream(subscriptionId);
      },
      async hostDescriptor() {
        return (await runtimePromise).hostDescriptor();
      },
      async currentPermissionRequest() {
        return (await runtimePromise).currentPermissionRequest();
      },
      async handlePermissionResult(result) {
        return (await runtimePromise).handlePermissionResult(result);
      },
    };
  }

  async function webAccessSession(baseUrl) {
    const savedSession = localStorage.getItem(webAccessSessionStorageKey);
    if (savedSession !== null) {
      return JSON.parse(savedSession);
    }
    const session = await pairWebAccessSession(baseUrl);
    localStorage.setItem(webAccessSessionStorageKey, JSON.stringify(session));
    return session;
  }

  async function pairWebAccessSession(baseUrl) {
    const token = globalThis.prompt("Operit Web Access token");
    if (token === null || token.trim().length === 0) {
      throw new Error("web access token is required");
    }
    const keyPair = await crypto.subtle.generateKey(
      { name: "X25519" },
      true,
      ["deriveBits"],
    );
    const clientPublicKey = bytesToBase64(
      new Uint8Array(await crypto.subtle.exportKey("raw", keyPair.publicKey)),
    );
    const clientDeviceId = `web-client-${crypto.randomUUID()}`;
    const clientNonce = crypto.randomUUID();
    const start = await postJson(`${baseUrl}/link/pair/start`, {
      pairingServiceVersion,
      tokenHash: await linkTokenHash(token.trim()),
      clientDeviceId,
      clientDeviceInfo: webDeviceInfo(),
      clientPublicKey,
      clientNonce,
    });
    const corePublicKey = await crypto.subtle.importKey(
      "raw",
      base64ToBytes(start.corePublicKey),
      { name: "X25519" },
      false,
      [],
    );
    const sharedSecret = new Uint8Array(
      await crypto.subtle.deriveBits(
        { name: "X25519", public: corePublicKey },
        keyPair.privateKey,
        256,
      ),
    );
    const pairingCode = globalThis.prompt("Operit Web Access pairing code");
    if (pairingCode === null || pairingCode.trim().length === 0) {
      throw new Error("web access pairing code is required");
    }
    const finish = await postJson(`${baseUrl}/link/pair/finish`, {
      pairingId: start.pairingId,
      pairingCode: pairingCode.trim(),
      clientProof: await proof(sharedSecret, clientNonce, start.serverNonce, "client"),
    });
    const expectedCoreProof = await proof(sharedSecret, clientNonce, start.serverNonce, "core");
    if (finish.coreProof !== expectedCoreProof) {
      throw new Error("web access core proof mismatch");
    }
    return {
      baseUrl,
      sessionId: finish.sessionId,
      deviceId: clientDeviceId,
      coreDeviceId: start.coreDeviceId,
      remoteDeviceInfo: start.coreDeviceInfo,
      pairingServiceVersion: finish.pairingServiceVersion,
      sessionSecret: await sessionSecret(sharedSecret, clientNonce, start.serverNonce),
    };
  }

  async function postJson(url, body) {
    const response = await fetch(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    const text = await response.text();
    if (!response.ok) {
      throw new Error(text);
    }
    return JSON.parse(text);
  }

  function webDeviceInfo() {
    return {
      platform: navigator.platform,
      model: browserName(navigator.userAgent),
    };
  }

  function browserName(userAgent) {
    const match = /(Edg|OPR|Firefox|Chrome|CriOS|FxiOS|Version)\/([0-9]+)/.exec(userAgent);
    if (match === null) {
      throw new Error("browser name is not available in userAgent");
    }
    const name = {
      Edg: "Edge",
      OPR: "Opera",
      CriOS: "Chrome iOS",
      FxiOS: "Firefox iOS",
      Version: "Safari",
    }[match[1]] || match[1];
    return `${name} ${match[2]}`;
  }

  async function proof(sharedSecret, clientNonce, serverNonce, role) {
    return bytesToBase64(
      new Uint8Array(
        await crypto.subtle.digest(
          "SHA-256",
          concatBytes(
            sharedSecret,
            textEncoder.encode(clientNonce),
            textEncoder.encode(serverNonce),
            textEncoder.encode(role),
          ),
        ),
      ),
    );
  }

  async function linkTokenHash(token) {
    return bytesToBase64(
      new Uint8Array(
        await crypto.subtle.digest("SHA-256", textEncoder.encode(token)),
      ),
    );
  }

  async function sessionSecret(sharedSecret, clientNonce, serverNonce) {
    return bytesToBase64(
      new Uint8Array(
        await crypto.subtle.digest(
          "SHA-256",
          concatBytes(
            sharedSecret,
            textEncoder.encode(clientNonce),
            textEncoder.encode(serverNonce),
            textEncoder.encode("session"),
          ),
        ),
      ),
    );
  }

  function concatBytes(...parts) {
    const length = parts.reduce((sum, part) => sum + part.length, 0);
    const output = new Uint8Array(length);
    let offset = 0;
    for (const part of parts) {
      output.set(part, offset);
      offset += part.length;
    }
    return output;
  }

  function createLinkedWebRuntime(config) {
    const baseUrl = String(config.baseUrl || "").replace(/\/+$/, "");
    const sessionId = String(config.sessionId);
    const deviceId = String(config.deviceId);
    const sessionSecret = String(config.sessionSecret);
    const streamQueues = new Map();
    const streamChannels = new Map();
    const channels = new Map();
    const permissionRequestTimes = new Map();
    let currentPermissionRequestId = null;
    let hmacKeyPromise = null;
    let openingChannelPromise = null;
    const maxSubscriptionsPerChannel = 16;

    function linkPath(path) {
      return `${baseUrl}${path}`;
    }

    async function hmacKey() {
      if (!hmacKeyPromise) {
        hmacKeyPromise = crypto.subtle.importKey(
          "raw",
          base64ToBytes(sessionSecret),
          { name: "HMAC", hash: "SHA-256" },
          false,
          ["sign"],
        );
      }
      return hmacKeyPromise;
    }

    async function linkHeaders(bodyText) {
      const signature = await crypto.subtle.sign(
        "HMAC",
        await hmacKey(),
        textEncoder.encode(bodyText),
      );
      return {
        "content-type": "application/json",
        "x-operit-session": sessionId,
        "x-operit-device": deviceId,
        "x-operit-signature": bytesToBase64(new Uint8Array(signature)),
      };
    }

    async function postText(path, body, signal) {
      const bodyText = JSON.stringify(body);
      const response = await fetch(linkPath(path), {
        method: "POST",
        headers: await linkHeaders(bodyText),
        body: bodyText,
        signal,
      });
      const text = await response.text();
      if (!response.ok) {
        throw new Error(text);
      }
      return text;
    }

    async function openChannel() {
      const channelId = `watch-channel-${crypto.randomUUID()}`;
      const controller = new AbortController();
      const channel = {
        channelId,
        controller,
        subscriptionCount: 0,
      };
      const body = { channelId };
      const bodyText = JSON.stringify(body);
      const response = await fetch(linkPath("/link/watch/channel/events"), {
        method: "POST",
        headers: await linkHeaders(bodyText),
        body: bodyText,
        signal: controller.signal,
      });
      const errorText = response.ok ? null : await response.text();
      if (errorText !== null) {
        throw new Error(errorText);
      }
      channels.set(channelId, channel);
      readWatchChannel(channel, response);
      return channel;
    }

    async function readWatchChannel(channel, response) {
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";
      try {
        while (true) {
          const chunk = await reader.read();
          if (chunk.done) {
            break;
          }
          buffer += decoder.decode(chunk.value, { stream: true });
          let newlineIndex = buffer.indexOf("\n");
          while (newlineIndex >= 0) {
            const line = buffer.substring(0, newlineIndex).trim();
            buffer = buffer.substring(newlineIndex + 1);
          if (line.length > 0) {
              const event = JSON.parse(line);
              const queue = streamQueues.get(event.subscriptionId);
              if (queue) {
                queue.push(event.event);
              }
            }
            newlineIndex = buffer.indexOf("\n");
          }
        }
        const tail = buffer.trim();
        if (tail.length > 0) {
          const event = JSON.parse(tail);
          const queue = streamQueues.get(event.subscriptionId);
          if (queue) {
            queue.push(event.event);
          }
        }
      } catch (error) {
        for (const [subscriptionId, channelId] of streamChannels.entries()) {
          if (channelId === channel.channelId) {
            const queue = streamQueues.get(subscriptionId);
            if (queue) {
              queue.push({
                requestId: null,
                targetPath: { segments: [] },
                propertyName: "watch",
                kind: "Completed",
                value: {
                  code: "LINK_WATCH_CHANNEL_ERROR",
                  message: String(error),
                },
              });
            }
          }
        }
      } finally {
        channels.delete(channel.channelId);
      }
    }

    async function acquireChannel() {
      for (const channel of channels.values()) {
        if (channel.subscriptionCount < maxSubscriptionsPerChannel) {
          return channel;
        }
      }
      if (!openingChannelPromise) {
        openingChannelPromise = openChannel().finally(() => {
          openingChannelPromise = null;
        });
      }
      return openingChannelPromise;
    }

    function permissionResult(value) {
      if (value === "ALLOW") {
        return "allow";
      }
      if (value === "ALWAYS_ALLOW") {
        return "always_allow";
      }
      if (value === "DENY") {
        return "deny";
      }
      throw new Error(`unknown permission result: ${value}`);
    }

    return {
      async call(request) {
        return postText("/link/call", {
          request: JSON.parse(request),
        });
      },
      async watchSnapshot(request) {
        return postText("/link/watch/snapshot", {
          request: JSON.parse(request),
        });
      },
      async watchStream(request) {
        const channel = await acquireChannel();
        const subscriptionId = `watch-${crypto.randomUUID()}`;
        streamQueues.set(subscriptionId, []);
        streamChannels.set(subscriptionId, channel.channelId);
        channel.subscriptionCount += 1;
        try {
          const responseText = await postText("/link/watch/channel/open", {
            channelId: channel.channelId,
            subscriptionId,
            request: JSON.parse(request),
          });
          const response = JSON.parse(responseText);
          if (response.subscriptionId !== subscriptionId) {
            throw new Error("watch channel subscription id mismatch");
          }
          return JSON.stringify({ subscriptionId });
        } catch (error) {
          channel.subscriptionCount -= 1;
          streamQueues.delete(subscriptionId);
          streamChannels.delete(subscriptionId);
          throw error;
        }
      },
      async pollWatchStream(subscriptionId) {
        const queue = streamQueues.get(subscriptionId);
        if (!queue) {
          throw new Error(`link watch stream not found: ${subscriptionId}`);
        }
        const events = queue.splice(0, queue.length);
        return JSON.stringify(events);
      },
      async closeWatchStream(subscriptionId) {
        const channelId = streamChannels.get(subscriptionId);
        if (!channelId) {
          throw new Error(`link watch stream not found: ${subscriptionId}`);
        }
        const channel = channels.get(channelId);
        await postText("/link/watch/channel/close", {
          channelId,
          subscriptionId,
        });
        streamChannels.delete(subscriptionId);
        streamQueues.delete(subscriptionId);
        if (channel) {
          channel.subscriptionCount -= 1;
          if (channel.subscriptionCount === 0) {
            channel.controller.abort();
            channels.delete(channelId);
          }
        }
        return "{}";
      },
      async hostDescriptor() {
        const nonce = `web-${Date.now()}`;
        const status = JSON.parse(await postText("/link/session", { nonce }));
        return JSON.stringify({
          id: `web-access:${status.coreDeviceId}`,
          displayName: "Web Access Operit Core",
          pathStyleDescriptionEn: "Web access core path style",
          pathStyleDescriptionCn: "Web Access 核心路径风格",
          examplePaths: [],
          usesEnvironmentParameter: false,
          environmentParameterDescriptionEn: "",
          environmentParameterDescriptionCn: "",
          capabilities: status.transports,
          fileSystemHost: true,
          webVisitHost: true,
          systemOperationHost: true,
          managedRuntimeHost: true,
          runtimeStorageHost: true,
          runtimeSqliteHost: true,
        });
      },
      async currentPermissionRequest() {
        const text = await postText("/host/interaction/poll", {
          timeoutMs: 0,
        });
        const response = JSON.parse(text);
        const request = response.request;
        if (request === null) {
          return "null";
        }
        if (request.kind !== "tool_permission") {
          return "null";
        }
        currentPermissionRequestId = request.requestId;
        let requestedAtMillis = permissionRequestTimes.get(request.requestId);
        if (requestedAtMillis === undefined) {
          requestedAtMillis = Date.now();
          permissionRequestTimes.set(request.requestId, requestedAtMillis);
        }
        return JSON.stringify({
          tool: request.payload.tool,
          description: request.payload.description,
          requestedAtMillis,
        });
      },
      async handlePermissionResult(result) {
        if (currentPermissionRequestId === null) {
          throw new Error("no pending web access permission request");
        }
        const requestId = currentPermissionRequestId;
        currentPermissionRequestId = null;
        permissionRequestTimes.delete(requestId);
        await postText("/host/interaction/respond", {
          requestId,
          response: {
            result: permissionResult(result),
          },
        });
        return "{}";
      },
    };
  }

  function key(prefix, path) {
    return prefix + String(path).replace(/^\/+/, "");
  }

  function bytesToBase64(bytes) {
    let binary = "";
    for (const byte of bytes) {
      binary += String.fromCharCode(byte);
    }
    return btoa(binary);
  }

  function base64ToBytes(value) {
    const binary = atob(value || "");
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }
    return bytes;
  }

  function nowIso() {
    return new Date().toISOString();
  }

  function storageRead(prefix, path) {
    return base64ToBytes(localStorage.getItem(key(prefix, path)));
  }

  function storageWrite(prefix, path, content) {
    localStorage.setItem(key(prefix, path), bytesToBase64(new Uint8Array(content)));
  }

  function storageExists(prefix, path) {
    const exact = key(prefix, path);
    const directory = exact.endsWith("/") ? exact : exact + "/";
    if (localStorage.getItem(exact) !== null) {
      return true;
    }
    for (let index = 0; index < localStorage.length; index += 1) {
      const itemKey = localStorage.key(index);
      if (itemKey && itemKey.startsWith(directory)) {
        return true;
      }
    }
    return false;
  }

  function storageDelete(prefix, path, recursive) {
    const exact = key(prefix, path);
    const directory = exact.endsWith("/") ? exact : exact + "/";
    localStorage.removeItem(exact);
    if (recursive) {
      const keys = [];
      for (let index = 0; index < localStorage.length; index += 1) {
        const itemKey = localStorage.key(index);
        if (itemKey && itemKey.startsWith(directory)) {
          keys.push(itemKey);
        }
      }
      for (const itemKey of keys) {
        localStorage.removeItem(itemKey);
      }
    }
  }

  function storageList(prefix, path) {
    const root = key(prefix, path);
    const directory = root.endsWith(".") || root.endsWith("/") ? root : root + "/";
    const entries = [];
    for (let index = 0; index < localStorage.length; index += 1) {
      const itemKey = localStorage.key(index);
      if (!itemKey || !itemKey.startsWith(directory)) {
        continue;
      }
      const pathValue = itemKey.substring(prefix.length);
      entries.push({
        path: pathValue,
        isDirectory: false,
        size: base64ToBytes(localStorage.getItem(itemKey)).length,
      });
    }
    return entries;
  }

  function loadScript(src) {
    return new Promise((resolve, reject) => {
      const existing = document.querySelector(`script[src="${src}"]`);
      if (existing) {
        existing.addEventListener("load", resolve, { once: true });
        existing.addEventListener("error", reject, { once: true });
        return;
      }
      const script = document.createElement("script");
      script.src = src;
      script.onload = resolve;
      script.onerror = () => reject(new Error(`failed to load ${src}`));
      document.head.appendChild(script);
    });
  }

  async function ensureSqlite() {
    if (!sqliteModulePromise) {
      sqliteModulePromise = (async () => {
        await loadScript("sql-wasm.js");
        SQLite = await globalThis.initSqlJs({
          locateFile(file) {
            return file;
          },
        });
      })();
    }
    await sqliteModulePromise;
  }

  function sqliteKey(path) {
    return key(sqlitePrefix, path);
  }

  function saveSqliteDatabase(connection) {
    localStorage.setItem(sqliteKey(connection.path), bytesToBase64(connection.db.export()));
  }

  function sqliteConnection(id) {
    const connection = sqliteConnections.get(id);
    if (!connection) {
      throw new Error(`sqlite connection not found: ${id}`);
    }
    return connection;
  }

  function sqliteTransaction(id) {
    const transaction = sqliteTransactions.get(id);
    if (!transaction) {
      throw new Error(`sqlite transaction not found: ${id}`);
    }
    return transaction;
  }

  function sqliteParam(param) {
    if (param.kind === "null") {
      return null;
    }
    if (param.kind === "integer") {
      return Number(param.value);
    }
    if (param.kind === "real") {
      return param.value;
    }
    if (param.kind === "text") {
      return param.value;
    }
    if (param.kind === "blob") {
      return new Uint8Array(param.value);
    }
    throw new Error(`unknown sqlite value kind: ${param.kind}`);
  }

  function sqliteParams(params) {
    return (params || []).map(sqliteParam);
  }

  function sqliteValue(value) {
    if (value === null || value === undefined) {
      return { kind: "null" };
    }
    if (value instanceof Uint8Array) {
      return { kind: "blob", value };
    }
    if (typeof value === "number") {
      return Number.isInteger(value)
        ? { kind: "integer", value: String(value) }
        : { kind: "real", value };
    }
    return { kind: "text", value: String(value) };
  }

  function querySqlite(db, sql, params) {
    const statement = db.prepare(sql);
    const rows = [];
    try {
      statement.bind(sqliteParams(params));
      const columns = statement.getColumnNames();
      while (statement.step()) {
        rows.push({
          columns,
          values: statement.get().map(sqliteValue),
        });
      }
    } finally {
      statement.free();
    }
    return rows;
  }

  function fileInfo(path) {
    const exists = storageExists(filePrefix, path);
    const bytes = exists ? storageRead(filePrefix, path) : new Uint8Array();
    return {
      path,
      exists,
      fileType: exists ? "file" : "missing",
      size: bytes.length,
      permissions: "rw",
      owner: "web",
      group: "web",
      lastModified: nowIso(),
      rawStatOutput: "",
    };
  }

  function unavailable(name) {
    throw new Error(`${name} is not available in the browser host`);
  }

  globalThis.__operitHost = {
    runtimeStorage: {
      readBytes(path) {
        return storageRead(runtimePrefix, path);
      },
      writeBytes(path, content) {
        storageWrite(runtimePrefix, path, content);
      },
      delete(path, recursive) {
        storageDelete(runtimePrefix, path, recursive);
      },
      exists(path) {
        return storageExists(runtimePrefix, path);
      },
      list(prefix) {
        return storageList(runtimePrefix, prefix);
      },
    },
    sqlite: {
      open(path) {
        if (!SQLite) {
          throw new Error("sqlite host is not initialized");
        }
        const id = `sqlite-${++sqliteConnectionIndex}`;
        const stored = localStorage.getItem(sqliteKey(path));
        const bytes = stored === null ? undefined : base64ToBytes(stored);
        sqliteConnections.set(id, {
          path,
          db: bytes === undefined ? new SQLite.Database() : new SQLite.Database(bytes),
        });
        return id;
      },
      executeBatch(id, sql) {
        const connection = sqliteConnection(id);
        connection.db.exec(sql);
        saveSqliteDatabase(connection);
      },
      execute(id, sql, params) {
        const connection = sqliteConnection(id);
        connection.db.run(sql, sqliteParams(params));
        saveSqliteDatabase(connection);
        return connection.db.getRowsModified();
      },
      query(id, sql, params) {
        return querySqlite(sqliteConnection(id).db, sql, params);
      },
      lastInsertRowId(id) {
        const rows = querySqlite(sqliteConnection(id).db, "SELECT last_insert_rowid()", []);
        return rows.length === 0 ? "0" : rows[0].values[0].value;
      },
      beginTransaction(id) {
        const transactionId = `sqlite-tx-${++sqliteTransactionIndex}`;
        const connection = sqliteConnection(id);
        connection.db.run("BEGIN IMMEDIATE");
        sqliteTransactions.set(transactionId, connection);
        return transactionId;
      },
      transactionExecute(id, sql, params) {
        const connection = sqliteTransaction(id);
        connection.db.run(sql, sqliteParams(params));
        return connection.db.getRowsModified();
      },
      transactionQuery(id, sql, params) {
        return querySqlite(sqliteTransaction(id).db, sql, params);
      },
      transactionLastInsertRowId(id) {
        const rows = querySqlite(sqliteTransaction(id).db, "SELECT last_insert_rowid()", []);
        return rows.length === 0 ? "0" : rows[0].values[0].value;
      },
      commitTransaction(id) {
        const connection = sqliteTransaction(id);
        connection.db.run("COMMIT");
        saveSqliteDatabase(connection);
        sqliteTransactions.delete(id);
      },
    },
    fileSystem: {
      validatePath() {},
      listFiles(path) {
        return storageList(filePrefix, path).map((entry) => ({
          name: entry.path.split("/").pop() || entry.path,
          isDirectory: entry.isDirectory,
          size: entry.size,
          permissions: "rw",
          lastModified: nowIso(),
        }));
      },
      readFile(path) {
        return textDecoder.decode(storageRead(filePrefix, path));
      },
      readFileWithLimit(path, maxBytes) {
        return textDecoder.decode(storageRead(filePrefix, path).slice(0, maxBytes));
      },
      readFileBytes(path) {
        return storageRead(filePrefix, path);
      },
      writeFile(path, content, append) {
        const previous = append && storageExists(filePrefix, path)
          ? textDecoder.decode(storageRead(filePrefix, path))
          : "";
        storageWrite(filePrefix, path, textEncoder.encode(previous + content));
      },
      writeFileBytes(path, content) {
        storageWrite(filePrefix, path, content);
      },
      deleteFile(path, recursive) {
        storageDelete(filePrefix, path, recursive);
      },
      fileExists(path) {
        const exists = storageExists(filePrefix, path);
        return {
          exists,
          isDirectory: false,
          size: exists ? storageRead(filePrefix, path).length : 0,
        };
      },
      moveFile(source, destination) {
        const content = storageRead(filePrefix, source);
        storageWrite(filePrefix, destination, content);
        storageDelete(filePrefix, source, false);
      },
      copyFile(source, destination) {
        storageWrite(filePrefix, destination, storageRead(filePrefix, source));
      },
      makeDirectory() {},
      findFiles() {
        return [];
      },
      fileInfo,
      grepCode() {
        return { matches: [], totalMatches: 0, filesSearched: 0 };
      },
      zipFiles() {
        unavailable("fileSystem.zipFiles");
      },
      unzipFiles() {
        unavailable("fileSystem.unzipFiles");
      },
      openFile() {},
      shareFile() {},
    },
    webVisit: {
      visitWeb(request) {
        return {
          url: request.url,
          title: request.url,
          content: "",
          metadata: [],
          links: [],
          imageLinks: [],
        };
      },
    },
    http: {
      executeHttpRequest(request) {
        const xhr = new XMLHttpRequest();
        xhr.open(request.method, request.url, false);
        xhr.overrideMimeType("text/plain; charset=x-user-defined");
        for (const pair of request.headers || []) {
          const name = Array.isArray(pair) ? pair[0] : pair.key;
          const value = Array.isArray(pair) ? pair[1] : pair.value;
          xhr.setRequestHeader(name, value);
        }
        let body = null;
        if ((request.fileParts && request.fileParts.length) || (request.formFields && request.formFields.length)) {
          const form = new FormData();
          for (const pair of request.formFields || []) {
            const name = Array.isArray(pair) ? pair[0] : pair.key;
            const value = Array.isArray(pair) ? pair[1] : pair.value;
            form.append(name, value);
          }
          for (const part of request.fileParts || []) {
            form.append(
              part.fieldName,
              new Blob([new Uint8Array(part.content)], { type: part.contentType }),
              part.fileName,
            );
          }
          body = form;
        } else if (request.body && request.body.length) {
          body = new Uint8Array(request.body);
        }
        xhr.send(body);
        const raw = xhr.responseText || "";
        const responseBytes = new Uint8Array(raw.length);
        for (let index = 0; index < raw.length; index += 1) {
          responseBytes[index] = raw.charCodeAt(index) & 0xff;
        }
        return {
          finalUrl: xhr.responseURL || request.url,
          statusCode: xhr.status,
          statusMessage: xhr.statusText || "",
          headers: xhr.getAllResponseHeaders()
            .trim()
            .split(/\r?\n/)
            .filter((line) => line.length > 0)
            .map((line) => {
              const index = line.indexOf(":");
              return [line.slice(0, index).trim(), line.slice(index + 1).trim()];
            }),
          body: responseBytes,
        };
      },
    },
    managedRuntime: {
      runtimeWorkspaceDir() {
        return "operit2/workspace";
      },
      resolveRuntimeExecutable(program) {
        return program;
      },
      startRuntimeProcess() {
        unavailable("managedRuntime.startRuntimeProcess");
      },
      runRuntimeCommand() {
        unavailable("managedRuntime.runRuntimeCommand");
      },
    },
    managedRuntimeProcess: {
      writeLine() {},
      readStdoutLine() {
        return null;
      },
      drainStderr() {
        return "";
      },
      isRunning() {
        return false;
      },
      kill() {},
    },
    systemOperation: {
      toast(message) {
        console.info("[Operit toast]", message);
      },
      sendNotification(title, message) {
        console.info("[Operit notification]", title, message);
      },
      modifySystemSetting(namespace, setting, value) {
        return { namespace, setting, value };
      },
      getSystemSetting(namespace, setting) {
        return { namespace, setting, value: "" };
      },
      installApp(path) {
        return { operationType: "install", packageName: path, success: false, details: "" };
      },
      uninstallApp(packageName) {
        return { operationType: "uninstall", packageName, success: false, details: "" };
      },
      listInstalledApps(includeSystemApps) {
        return { includesSystemApps: includeSystemApps, packages: [] };
      },
      startApp(packageName) {
        return { operationType: "start", packageName, success: false, details: "" };
      },
      stopApp(packageName) {
        return { operationType: "stop", packageName, success: false, details: "" };
      },
      getNotifications() {
        return { notifications: [], timestamp: Date.now() };
      },
      getAppUsageTime(packageName, sinceHours, limit, includeSystemApps) {
        return {
          startTime: Date.now(),
          endTime: Date.now(),
          sinceHours,
          requestedPackageName: packageName,
          includesSystemApps: includeSystemApps,
          totalEntries: 0,
          entries: [],
        };
      },
      getDeviceLocation() {
        return {
          latitude: 0,
          longitude: 0,
          accuracy: 0,
          provider: "web",
          timestamp: Date.now(),
          rawData: "",
          address: "",
          city: "",
          province: "",
          country: "",
        };
      },
      getDeviceInfo() {
        return {
          deviceId: "web",
          model: browserName(navigator.userAgent),
          manufacturer: "browser",
          androidVersion: "",
          sdkVersion: 0,
          screenResolution: `${screen.width}x${screen.height}`,
          screenDensity: devicePixelRatio,
          totalMemory: "",
          availableMemory: "",
          totalStorage: "",
          availableStorage: "",
          batteryLevel: 0,
          batteryCharging: false,
          cpuInfo: "",
          networkType: navigator.onLine ? "online" : "offline",
          additionalInfo: {},
        };
      },
    },
  };

  let bridgePromise;

  async function bridge() {
    if (!bridgePromise) {
      bridgePromise = import("./operit_flutter_bridge.js").then(async (module) => {
        await ensureSqlite();
        await module.default({ module_or_path: "./operit_flutter_bridge_bg.wasm" });
        return new module.OperitFlutterBridgeWasm();
      });
    }
    return bridgePromise;
  }

  globalThis.__operitRuntime = {
    async call(request) {
      return (await bridge()).call(request);
    },
    async watchSnapshot(request) {
      return (await bridge()).watchSnapshot(request);
    },
    async watchStream(request) {
      return (await bridge()).watchStream(request);
    },
    async pollWatchStream(subscriptionId) {
      return (await bridge()).pollWatchStream(subscriptionId);
    },
    async closeWatchStream(subscriptionId) {
      return (await bridge()).closeWatchStream(subscriptionId);
    },
    async hostDescriptor() {
      return (await bridge()).hostDescriptor();
    },
    async currentPermissionRequest() {
      return (await bridge()).currentPermissionRequest();
    },
    async handlePermissionResult(result) {
      return (await bridge()).handlePermissionResult(result);
    },
  };
})();
