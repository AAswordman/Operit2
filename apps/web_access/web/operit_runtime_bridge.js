(function () {
  const textEncoder = new TextEncoder();
  const textDecoder = new TextDecoder();
  const runtimePrefix = "operit2.runtime.";
  const filePrefix = "operit2.files.";
  const sqlitePrefix = "operit2.sqlite.";
  const secretPrefix = "operit2.secrets.";
  const storageDatabaseName = "operit2.host.storage";
  const storageObjectStoreName = "entries";
  const storageCache = new Map();
  const sqliteConnections = new Map();
  const sqliteTransactions = new Map();
  let sqliteConnectionIndex = 0;
  let sqliteTransactionIndex = 0;
  let sqliteModulePromise;
  let SQLite;
  let storageDatabasePromise;
  let storageReadyPromise;
  let webLocalInferenceReadyPromise;
  let webLocalInferenceState;

  const webAccessSessionStorageKey = "operit2.webAccess.session";
  const pairingServiceVersion = 1;
  let webAccessSessionReloading = false;
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
      async pushOpen(request) {
        return (await runtimePromise).pushOpen(request);
      },
      async pushItem(item) {
        return (await runtimePromise).pushItem(item);
      },
      async pushClose(pushId) {
        return (await runtimePromise).pushClose(pushId);
      },
      async watchSnapshot(request) {
        return (await runtimePromise).watchSnapshot(request);
      },
      async watchStream(request, onEvent) {
        return (await runtimePromise).watchStream(request, onEvent);
      },
      async closeWatchStream(subscriptionId) {
        return (await runtimePromise).closeWatchStream(subscriptionId);
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

  function resetWebAccessSession() {
    localStorage.removeItem(webAccessSessionStorageKey);
    if (!webAccessSessionReloading) {
      webAccessSessionReloading = true;
      globalThis.location.reload();
    }
  }

  async function pairWebAccessSession(baseUrl) {
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
    let start;
    while (true) {
      const token = globalThis.prompt("Operit Web Access token");
      if (token === null || token.trim().length === 0) {
        throw new Error("web access token is required");
      }
      try {
        start = await postJson(`${baseUrl}/link/pair/start`, {
          pairingServiceVersion,
          tokenHash: await linkTokenHash(token.trim()),
          clientDeviceId,
          clientDeviceInfo: webDeviceInfo(),
          clientPublicKey,
          clientNonce,
        });
        break;
      } catch (error) {
        globalThis.alert(`Operit Web Access token rejected: ${error.message}`);
      }
    }
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
    let finish;
    while (true) {
      const pairingCode = globalThis.prompt("Operit Web Access pairing code");
      if (pairingCode === null || pairingCode.trim().length === 0) {
        throw new Error("web access pairing code is required");
      }
      try {
        finish = await postJson(`${baseUrl}/link/pair/finish`, {
          pairingId: start.pairingId,
          pairingCode: pairingCode.trim(),
          clientProof: await proof(sharedSecret, clientNonce, start.serverNonce, "client"),
        });
        break;
      } catch (error) {
        globalThis.alert(`Operit Web Access pairing code rejected: ${error.message}`);
      }
    }
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

  async function createLinkedWebRuntime(config) {
    const baseUrl = String(config.baseUrl || "").replace(/\/+$/, "");
    const sessionId = String(config.sessionId);
    const deviceId = String(config.deviceId);
    const sessionSecret = String(config.sessionSecret);
    const streamCallbacks = new Map();
    const streamChannels = new Map();
    const channels = new Map();
    let hmacKeyPromise = null;
    let openingChannelPromise = null;
    const maxSubscriptionsPerChannel = 16;
    let pushSocketPromise = null;
    let pushSendTail = Promise.resolve();
    let pushError = null;

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

    async function linkHeaders(bodyBytes) {
      const signature = await crypto.subtle.sign(
        "HMAC",
        await hmacKey(),
        bodyBytes,
      );
      return {
        "content-type": "application/msgpack",
        "x-operit-link-version": "3",
        "x-operit-session": sessionId,
        "x-operit-device": deviceId,
        "x-operit-signature": bytesToBase64(new Uint8Array(signature)),
      };
    }

    async function postLink(path, body, signal) {
      const bodyBytes = MessagePack.encode(body);
      const response = await fetch(linkPath(path), {
        method: "POST",
        headers: await linkHeaders(bodyBytes),
        body: bodyBytes,
        signal,
      });
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (!response.ok) {
        throwLinkErrorResponse(response.status, bytes);
      }
      return bytes;
    }

    /** Opens the authenticated binary carrier used by client-owned push streams. */
    function pushSocket() {
      if (pushSocketPromise === null) {
        pushSocketPromise = new Promise((resolve, reject) => {
          const url = new URL(linkPath("/link/ws"), globalThis.location.href);
          url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
          const socket = new WebSocket(url);
          socket.binaryType = "arraybuffer";
          socket.addEventListener("open", () => resolve(socket), { once: true });
          socket.addEventListener("error", () => reject(new Error("Link push socket failed to open")), { once: true });
          socket.addEventListener("message", (event) => {
            const response = MessagePack.decode(new Uint8Array(event.data));
            if (response.type === "Error") {
              pushError = new Error(`${response.body.code}: ${response.body.message}`);
            }
          });
          socket.addEventListener("close", () => {
            pushError = new Error("Link push socket closed");
          });
        });
      }
      return pushSocketPromise;
    }

    /** Signs and queues one push protocol frame without waiting for a per-item acknowledgement. */
    function sendPushPayload(payload) {
      pushSendTail = pushSendTail.then(async () => {
        if (pushError !== null) throw pushError;
        const bodyBytes = MessagePack.encode(payload);
        const signature = await crypto.subtle.sign("HMAC", await hmacKey(), bodyBytes);
        const socket = await pushSocket();
        socket.send(MessagePack.encode({
          protocolVersion: 3,
          sessionId,
          deviceId,
          signature: bytesToBase64(new Uint8Array(signature)),
          payload,
        }));
      });
      return pushSendTail;
    }

    /** Returns whether the link error requests a saved Web Access session reset. */
    function shouldResetWebAccessSession(status, error) {
      const details = error.details;
      return status === 401 &&
        error.code === "UNAUTHORIZED" &&
        details !== null &&
        typeof details === "object" &&
        details.type === "remote_session_auth" &&
        details.resetWebAccessSession === true;
    }

    /** Decodes and throws one MessagePack Link error response. */
    function throwLinkErrorResponse(status, bytes) {
      const error = MessagePack.decode(bytes);
      if (shouldResetWebAccessSession(status, error)) {
        resetWebAccessSession();
      }
      throw new Error(`${error.code}: ${error.message}`);
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
      const bodyBytes = MessagePack.encode(body);
      const response = await fetch(linkPath("/link/watch/channel/events"), {
        method: "POST",
        headers: await linkHeaders(bodyBytes),
        body: bodyBytes,
        signal: controller.signal,
      });
      const errorBytes = response.ok ? null : new Uint8Array(await response.arrayBuffer());
      if (errorBytes !== null) {
        throwLinkErrorResponse(response.status, errorBytes);
      }
      channels.set(channelId, channel);
      readWatchChannel(channel, response);
      return channel;
    }

    async function readWatchChannel(channel, response) {
      const reader = response.body.getReader();
      let buffer = new Uint8Array();
      try {
        while (true) {
          const chunk = await reader.read();
          if (chunk.done) {
            break;
          }
          const joined = new Uint8Array(buffer.length + chunk.value.length);
          joined.set(buffer);
          joined.set(chunk.value, buffer.length);
          buffer = joined;
          while (buffer.length >= 4) {
            const frameLength = new DataView(buffer.buffer, buffer.byteOffset, 4).getUint32(0);
            if (buffer.length < 4 + frameLength) break;
            const frame = buffer.slice(4, 4 + frameLength);
            buffer = buffer.slice(4 + frameLength);
            const event = MessagePack.decode(frame);
            const callback = streamCallbacks.get(event.subscriptionId);
            if (callback) callback(frame);
          }
        }
        if (buffer.length !== 0) throw new Error("incomplete Link watch frame");
      } catch (error) {
        for (const [subscriptionId, channelId] of streamChannels.entries()) {
          if (channelId === channel.channelId) {
            const callback = streamCallbacks.get(subscriptionId);
            if (callback) {
              callback(MessagePack.encode({
                subscriptionId,
                errorCode: "LINK_WATCH_CHANNEL_ERROR",
                errorMessage: String(error),
              }));
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

    const sessionNonce = `web-${crypto.randomUUID()}`;
    const sessionBytes = await postLink("/link/session", { nonce: sessionNonce });
    const sessionInfo = MessagePack.decode(sessionBytes);
    if (sessionInfo.protocolVersion !== 3) {
      throw new Error(`Link protocol version ${sessionInfo.protocolVersion} is not supported`);
    }

    return {
      async call(request) {
        return postLink("/link/call", {
          request: MessagePack.decode(request),
        });
      },
      async pushOpen(request) {
        const decoded = MessagePack.decode(request);
        await sendPushPayload({ type: "PushOpen", body: decoded });
        return MessagePack.encode({ pushId: decoded.requestId });
      },
      async pushItem(item) {
        await sendPushPayload({ type: "PushItem", body: MessagePack.decode(item) });
        return MessagePack.encode({});
      },
      async pushClose(pushId) {
        await sendPushPayload({ type: "PushClose", body: pushId });
        return MessagePack.encode({});
      },
      async watchSnapshot(request) {
        return postLink("/link/watch/snapshot", {
          request: MessagePack.decode(request),
        });
      },
      async watchStream(request, onEvent) {
        if (typeof onEvent !== "function") {
          throw new Error("watchStream expects an event callback");
        }
        const channel = await acquireChannel();
        const envelope = MessagePack.decode(request);
        const subscriptionId = envelope.subscriptionId;
        streamCallbacks.set(subscriptionId, onEvent);
        streamChannels.set(subscriptionId, channel.channelId);
        channel.subscriptionCount += 1;
        try {
          const responseBytes = await postLink("/link/watch/channel/open", {
            channelId: channel.channelId,
            subscriptionId,
            request: envelope.request,
          });
          const response = MessagePack.decode(responseBytes);
          if (response.subscriptionId !== subscriptionId) {
            throw new Error("watch channel subscription id mismatch");
          }
          return MessagePack.encode({ subscriptionId });
        } catch (error) {
          channel.subscriptionCount -= 1;
          streamCallbacks.delete(subscriptionId);
          streamChannels.delete(subscriptionId);
          throw error;
        }
      },
      async closeWatchStream(subscriptionId) {
        const channelId = streamChannels.get(subscriptionId);
        if (!channelId) {
          throw new Error(`link watch stream not found: ${subscriptionId}`);
        }
        const channel = channels.get(channelId);
        await postLink("/link/watch/channel/close", {
          channelId,
          subscriptionId,
        });
        streamChannels.delete(subscriptionId);
        streamCallbacks.delete(subscriptionId);
        if (channel) {
          channel.subscriptionCount -= 1;
          if (channel.subscriptionCount === 0) {
            channel.controller.abort();
            channels.delete(channelId);
          }
        }
        return MessagePack.encode({});
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

  // Opens the browser storage database used for large runtime files.
  function openStorageDatabase() {
    if (!storageDatabasePromise) {
      storageDatabasePromise = new Promise((resolve, reject) => {
        const request = indexedDB.open(storageDatabaseName, 1);
        request.onupgradeneeded = () => {
          request.result.createObjectStore(storageObjectStoreName);
        };
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error || new Error("indexedDB open failed"));
      });
    }
    return storageDatabasePromise;
  }

  // Loads the persisted storage entries into the synchronous memory view.
  async function ensureBrowserStorage() {
    if (!storageReadyPromise) {
      storageReadyPromise = (async () => {
        const database = await openStorageDatabase();
        await new Promise((resolve, reject) => {
          const transaction = database.transaction(storageObjectStoreName, "readonly");
          const store = transaction.objectStore(storageObjectStoreName);
          const request = store.openCursor();
          request.onsuccess = () => {
            const cursor = request.result;
            if (cursor) {
              storageCache.set(cursor.key, new Uint8Array(cursor.value));
              cursor.continue();
            }
          };
          request.onerror = () => reject(request.error || new Error("indexedDB cursor failed"));
          transaction.oncomplete = resolve;
          transaction.onerror = () => reject(transaction.error || new Error("indexedDB read failed"));
        });
        migrateLocalStorageEntries(runtimePrefix);
        migrateLocalStorageEntries(filePrefix);
        migrateLocalStorageEntries(sqlitePrefix);
      })();
    }
    return storageReadyPromise;
  }

  // Copies existing localStorage-hosted entries into the synchronous storage view.
  function migrateLocalStorageEntries(prefix) {
    const migratedKeys = [];
    for (let index = 0; index < localStorage.length; index += 1) {
      const itemKey = localStorage.key(index);
      if (itemKey && itemKey.startsWith(prefix)) {
        storageCache.set(itemKey, base64ToBytes(localStorage.getItem(itemKey)));
        persistStorageEntry(itemKey, storageCache.get(itemKey));
        migratedKeys.push(itemKey);
      }
    }
    for (const itemKey of migratedKeys) {
      localStorage.removeItem(itemKey);
    }
  }

  // Persists one memory-view entry into IndexedDB.
  async function persistStorageEntry(itemKey, bytes) {
    const database = await openStorageDatabase();
    await new Promise((resolve, reject) => {
      const transaction = database.transaction(storageObjectStoreName, "readwrite");
      transaction.objectStore(storageObjectStoreName).put(new Uint8Array(bytes), itemKey);
      transaction.oncomplete = resolve;
      transaction.onerror = () => reject(transaction.error || new Error("indexedDB write failed"));
    });
  }

  // Removes one memory-view entry from IndexedDB.
  async function removeStorageEntry(itemKey) {
    const database = await openStorageDatabase();
    await new Promise((resolve, reject) => {
      const transaction = database.transaction(storageObjectStoreName, "readwrite");
      transaction.objectStore(storageObjectStoreName).delete(itemKey);
      transaction.oncomplete = resolve;
      transaction.onerror = () => reject(transaction.error || new Error("indexedDB delete failed"));
    });
  }

  function storageRead(prefix, path) {
    return storageCache.get(key(prefix, path)) || new Uint8Array();
  }

  function storageWrite(prefix, path, content) {
    const itemKey = key(prefix, path);
    const bytes = new Uint8Array(content);
    storageCache.set(itemKey, bytes);
    void persistStorageEntry(itemKey, bytes);
    if (isLocalModelRegistryPath(prefix, path)) {
      scheduleWebLocalInferenceRefresh();
    }
  }

  function storageExists(prefix, path) {
    const exact = key(prefix, path);
    const directory = exact.endsWith("/") ? exact : exact + "/";
    if (storageCache.has(exact)) {
      return true;
    }
    for (const itemKey of storageCache.keys()) {
      if (itemKey.startsWith(directory)) {
        return true;
      }
    }
    return false;
  }

  function storageDelete(prefix, path, recursive) {
    const exact = key(prefix, path);
    const directory = exact.endsWith("/") ? exact : exact + "/";
    storageCache.delete(exact);
    void removeStorageEntry(exact);
    if (recursive) {
      const keys = [];
      for (const itemKey of storageCache.keys()) {
        if (itemKey.startsWith(directory)) {
          keys.push(itemKey);
        }
      }
      for (const itemKey of keys) {
        storageCache.delete(itemKey);
        void removeStorageEntry(itemKey);
      }
    }
    if (isLocalModelRegistryPath(prefix, path)) {
      scheduleWebLocalInferenceRefresh();
    }
  }

  // Returns whether one storage mutation commits the local model registry.
  function isLocalModelRegistryPath(prefix, path) {
    return prefix === runtimePrefix &&
      normalizeRuntimePath(path) ===
        "runtime/config/preferences/local_model_registry.preferences.json";
  }

  function storageList(prefix, path) {
    const root = key(prefix, path);
    const directory = root.endsWith(".") || root.endsWith("/") ? root : root + "/";
    const entries = [];
    for (const itemKey of storageCache.keys()) {
      if (!itemKey.startsWith(directory)) {
        continue;
      }
      const pathValue = itemKey.substring(prefix.length);
      entries.push({
        path: pathValue,
        isDirectory: false,
        size: storageCache.get(itemKey).length,
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
    storageWrite(sqlitePrefix, connection.path, connection.db.export());
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

  const ttsPlayback = (() => {
    let activeUtterance = null;
    let activeAudio = null;
    let activeAudioUrl = null;
    let activeAudioPaused = false;
    let activePath = "";
    let utteranceIndex = 0;
    let lastDetails = "browser speech synthesis idle";

    function synthesis() {
      const value = globalThis.speechSynthesis;
      if (value === undefined || value === null) {
        throw new Error("browser speechSynthesis is not available");
      }
      return value;
    }

    function requireText(value, name) {
      if (typeof value !== "string") {
        throw new Error(`${name} must be a string`);
      }
      return value.trim();
    }

    function requireNumber(value, name) {
      if (typeof value !== "number" || !Number.isFinite(value)) {
        throw new Error(`${name} must be a finite number`);
      }
      return value;
    }

    function requireBoolean(value, name) {
      if (typeof value !== "boolean") {
        throw new Error(`${name} must be a boolean`);
      }
      return value;
    }

    function selectedVoice(voiceName) {
      if (voiceName.length === 0) {
        return null;
      }
      const voice = synthesis().getVoices().find((candidate) =>
        candidate.voiceURI === voiceName || candidate.name === voiceName
      );
      if (voice === undefined) {
        throw new Error(`tts voice not found: ${voiceName}`);
      }
      return voice;
    }

    function currentStatus(details) {
      if (activeAudio !== null) {
        return {
          path: activePath,
          active: !activeAudio.ended,
          paused: activeAudioPaused,
          details,
        };
      }
      const engine = synthesis();
      const active = activeUtterance !== null || engine.speaking || engine.pending;
      return {
        path: activePath,
        active,
        paused: engine.paused,
        details,
      };
    }

    // Resolves the media type for one generated TTS resource path.
    function audioContentType(path) {
      const extension = path.slice(path.lastIndexOf(".") + 1).toLowerCase();
      switch (extension) {
        case "aac": return "audio/aac";
        case "flac": return "audio/flac";
        case "m4a": return "audio/mp4";
        case "mp3": return "audio/mpeg";
        case "ogg":
        case "oga":
        case "opus": return "audio/ogg";
        case "wav": return "audio/wav";
        case "webm": return "audio/webm";
        default: return "application/octet-stream";
      }
    }

    // Releases the browser audio element and its object URL.
    function releaseAudio() {
      if (activeAudio !== null) {
        activeAudio.pause();
        activeAudio.onended = null;
        activeAudio.onerror = null;
        activeAudio = null;
        activeAudioPaused = false;
      }
      if (activeAudioUrl !== null) {
        URL.revokeObjectURL(activeAudioUrl);
        activeAudioUrl = null;
      }
    }

    return {
      playAudio(path) {
        const audioPath = requireText(path, "tts audio path");
        if (audioPath.length === 0) {
          throw new Error("tts audio path is empty");
        }
        const bytes = storageRead(runtimePrefix, audioPath);
        if (bytes.length === 0) {
          throw new Error(`tts audio resource is empty or missing: ${audioPath}`);
        }
        synthesis().cancel();
        activeUtterance = null;
        releaseAudio();
        activeAudioUrl = URL.createObjectURL(
          new Blob([bytes], { type: audioContentType(audioPath) })
        );
        const audio = new Audio(activeAudioUrl);
        activeAudio = audio;
        activeAudioPaused = false;
        activePath = audioPath;
        lastDetails = "browser generated TTS playback started";
        audio.onended = () => {
          if (activeAudio === audio) {
            releaseAudio();
            lastDetails = "browser generated TTS playback completed";
          }
        };
        audio.onerror = () => {
          if (activeAudio === audio) {
            releaseAudio();
            lastDetails = "browser generated TTS playback error";
          }
        };
        void audio.play().catch((error) => {
          if (activeAudio === audio) {
            releaseAudio();
            lastDetails = `browser generated TTS playback error: ${error}`;
          }
        });
        return currentStatus(lastDetails);
      },
      speakText(request) {
        const text = requireText(request.text, "tts text");
        if (text.length === 0) {
          throw new Error("tts text is empty");
        }
        const voiceName = requireText(request.voice, "tts voice");
        const locale = requireText(request.locale, "tts locale");
        const speed = requireNumber(request.speed, "tts speed");
        const pitch = requireNumber(request.pitch, "tts pitch");
        const interrupt = requireBoolean(request.interrupt, "tts interrupt");
        const engine = synthesis();
        if (interrupt) {
          engine.cancel();
          activeUtterance = null;
        }
        releaseAudio();
        const utterance = new SpeechSynthesisUtterance(text);
        const voice = selectedVoice(voiceName);
        if (voice !== null) {
          utterance.voice = voice;
        }
        if (locale.length > 0) {
          utterance.lang = locale;
        }
        utterance.rate = speed;
        utterance.pitch = pitch;
        const path = `web-tts://${++utteranceIndex}`;
        activePath = path;
        activeUtterance = utterance;
        lastDetails = "browser speech synthesis started";
        utterance.onend = () => {
          if (activeUtterance === utterance) {
            activeUtterance = null;
            lastDetails = "browser speech synthesis completed";
          }
        };
        utterance.onerror = (event) => {
          if (activeUtterance === utterance) {
            activeUtterance = null;
            lastDetails = `browser speech synthesis error: ${event.error}`;
          }
        };
        engine.speak(utterance);
        return currentStatus(lastDetails);
      },
      pauseSpeech() {
        if (activeAudio !== null) {
          activeAudio.pause();
          activeAudioPaused = true;
        } else {
          synthesis().pause();
        }
        lastDetails = "browser speech synthesis paused";
        return currentStatus(lastDetails);
      },
      resumeSpeech() {
        if (activeAudio !== null) {
          void activeAudio.play();
          activeAudioPaused = false;
        } else {
          synthesis().resume();
        }
        lastDetails = "browser speech synthesis resumed";
        return currentStatus(lastDetails);
      },
      stopSpeech() {
        synthesis().cancel();
        activeUtterance = null;
        releaseAudio();
        lastDetails = "browser speech synthesis stopped";
        return {
          path: activePath,
          active: false,
          paused: false,
          details: lastDetails,
        };
      },
      speechState() {
        return currentStatus(lastDetails);
      },
    };
  })();

  const musicPlayback = (() => {
    let audio = null;
    let source = null;
    let sourceType = null;
    let title = null;
    let artist = null;
    let loopPlayback = false;
    let volume = 1;
    let state = "idle";
    let message = "browser music player idle";

    function currentStatus(details) {
      const activeAudio = audio;
      return {
        state,
        source,
        sourceType,
        title,
        artist,
        durationMs: activeAudio && Number.isFinite(activeAudio.duration) ? Math.round(activeAudio.duration * 1000) : null,
        positionMs: activeAudio ? Math.round(activeAudio.currentTime * 1000) : 0,
        bufferedPositionMs: bufferedPositionMs(activeAudio),
        volume,
        loopPlayback,
        message: details,
      };
    }

    function bufferedPositionMs(activeAudio) {
      if (!activeAudio || activeAudio.buffered.length === 0) {
        return activeAudio ? Math.round(activeAudio.currentTime * 1000) : 0;
      }
      return Math.round(activeAudio.buffered.end(activeAudio.buffered.length - 1) * 1000);
    }

    function setSource(activeAudio, request) {
      if (request.sourceType === "path" || request.sourceType === "url" || request.sourceType === "uri") {
        activeAudio.src = request.source;
        return;
      }
      throw new Error(`unsupported music sourceType: ${request.sourceType}`);
    }

    return {
      playAudio(path) {
        const oneShot = new Audio(String(path));
        oneShot.play();
        return { path: String(path), started: true, details: "browser audio playback started" };
      },
      playMusic(request) {
        if (audio !== null) {
          audio.pause();
        }
        const activeAudio = new Audio();
        setSource(activeAudio, request);
        source = String(request.source || "");
        sourceType = String(request.sourceType || "");
        title = request.title || null;
        artist = request.artist || null;
        loopPlayback = request.loopPlayback === true;
        volume = Number.isFinite(request.volume) ? Math.min(Math.max(request.volume, 0), 1) : 1;
        activeAudio.loop = loopPlayback;
        activeAudio.volume = volume;
        activeAudio.currentTime = Math.max(Number(request.startPositionMs || 0), 0) / 1000;
        activeAudio.onended = () => {
          state = "completed";
          message = "browser music playback completed";
        };
        activeAudio.onerror = () => {
          state = "error";
          message = "browser music playback error";
        };
        audio = activeAudio;
        state = "playing";
        message = "browser music playback started";
        activeAudio.play();
        return currentStatus(message);
      },
      pauseMusic() {
        if (audio === null) {
          throw new Error("browser music player is not initialized");
        }
        audio.pause();
        state = "paused";
        message = "browser music playback paused";
        return currentStatus(message);
      },
      resumeMusic() {
        if (audio === null) {
          throw new Error("browser music player is not initialized");
        }
        audio.play();
        state = "playing";
        message = "browser music playback resumed";
        return currentStatus(message);
      },
      stopMusic() {
        if (audio !== null) {
          audio.pause();
          audio.removeAttribute("src");
          audio.load();
          audio = null;
        }
        state = "stopped";
        message = "browser music playback stopped";
        return currentStatus(message);
      },
      seekMusic(positionMs) {
        if (audio === null) {
          throw new Error("browser music player is not initialized");
        }
        audio.currentTime = Math.max(Number(positionMs || 0), 0) / 1000;
        message = "browser music playback seeked";
        return currentStatus(message);
      },
      setMusicVolume(value) {
        if (audio === null) {
          throw new Error("browser music player is not initialized");
        }
        volume = Math.min(Math.max(Number(value), 0), 1);
        audio.volume = volume;
        message = "browser music playback volume changed";
        return currentStatus(message);
      },
      musicStatus() {
        return currentStatus(message);
      },
    };
  })();

  const bluetooth = (() => {
    const bleSessions = new Map();
    const notifications = new Map();

    function browserBluetooth() {
      const api = navigator.bluetooth;
      if (!api) {
        throw new Error("browser Web Bluetooth is not available");
      }
      return api;
    }

    function bytesFromPayload(payload) {
      if (payload.text && payload.dataBase64) {
        throw new Error("Provide exactly one of text or dataBase64");
      }
      if (payload.text) {
        return textEncoder.encode(String(payload.text));
      }
      if (payload.dataBase64) {
        return base64ToBytes(String(payload.dataBase64));
      }
      throw new Error("Provide exactly one of text or dataBase64");
    }

    function readData(sessionId, bytes) {
      const value = bytes instanceof DataView ? new Uint8Array(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength)) : new Uint8Array(bytes);
      return {
        sessionId,
        bytesRead: value.length,
        text: textDecoder.decode(value),
        dataBase64: bytesToBase64(value),
      };
    }

    function session(id) {
      const value = bleSessions.get(id);
      if (!value) {
        throw new Error(`BLE session not found: ${id}`);
      }
      return value;
    }

    function characteristic(sessionId, serviceUuid, characteristicUuid) {
      const value = session(sessionId);
      const key = `${serviceUuid}:${characteristicUuid}`;
      const cached = value.characteristics.get(key);
      if (!cached) {
        throw new Error(`BLE characteristic not discovered: ${key}`);
      }
      return cached;
    }

    function classicUnavailable(name) {
      throw new Error(`browser Bluetooth classic ${name} is not available`);
    }

    return {
      requestBluetoothPermission() {
        browserBluetooth();
        return "browser_web_bluetooth_user_gesture_required";
      },
      bluetoothState() {
        return {
          supported: !!navigator.bluetooth,
          enabled: !!navigator.bluetooth,
          state: navigator.bluetooth ? "available" : "unavailable",
        };
      },
      requestEnableBluetooth() {
        browserBluetooth();
        return "browser_bluetooth_enable_controlled_by_system";
      },
      listBluetoothBondedDevices() {
        return { devices: [] };
      },
      scanBluetoothDevices(request) {
        const filters = [];
        const optionalServices = [];
        const deviceRequest = { acceptAllDevices: true, optionalServices };
        return browserBluetooth().requestDevice(deviceRequest).then((device) => ({
          devices: [{
            name: device.name || null,
            address: device.id,
            type: "ble",
            bondState: "unknown",
            source: "browser.web_bluetooth",
            rssi: null,
          }],
          durationMs: request.durationMs || 0,
          includesBle: true,
        }));
      },
      bluetoothConnect() { classicUnavailable("connect"); },
      bluetoothListen() { classicUnavailable("listen"); },
      bluetoothAccept() { classicUnavailable("accept"); },
      bluetoothSend() { classicUnavailable("send"); },
      bluetoothRead() { classicUnavailable("read"); },
      bluetoothSendAndRead() { classicUnavailable("sendAndRead"); },
      bluetoothClose(sessionId) {
        const value = bleSessions.get(sessionId);
        if (value && value.device.gatt.connected) {
          value.device.gatt.disconnect();
        }
        bleSessions.delete(sessionId);
        notifications.delete(sessionId);
        return `browser_bluetooth_session_closed:${sessionId}`;
      },
      bluetoothBleConnect(request) {
        return browserBluetooth().requestDevice({ acceptAllDevices: true }).then((device) =>
          device.gatt.connect().then((server) => {
            const sessionId = `web-ble-${crypto.randomUUID()}`;
            bleSessions.set(sessionId, { device, server, characteristics: new Map() });
            notifications.set(sessionId, []);
            return { sessionId, address: device.id, mode: "ble" };
          })
        );
      },
      bluetoothBleDiscoverServices(sessionId) {
        const value = session(sessionId);
        return value.server.getPrimaryServices().then((services) =>
          Promise.all(services.map((service) =>
            service.getCharacteristics().then((characteristics) => {
              for (const item of characteristics) {
                value.characteristics.set(`${service.uuid}:${item.uuid}`, item);
              }
              return {
                uuid: service.uuid,
                characteristics: characteristics.map((item) => ({
                  uuid: item.uuid,
                  properties: characteristicPropertyNames(item.properties),
                })),
              };
            })
          )).then((items) => ({ sessionId, services: items }))
        );
      },
      bluetoothBleReadCharacteristic(address) {
        return characteristic(address.sessionId, address.serviceUuid, address.characteristicUuid)
          .readValue()
          .then((value) => readData(address.sessionId, value));
      },
      bluetoothBleWriteCharacteristic(request) {
        const bytes = bytesFromPayload(request);
        return characteristic(request.sessionId, request.serviceUuid, request.characteristicUuid)
          .writeValue(bytes)
          .then(() => ({ sessionId: request.sessionId, bytesWritten: bytes.length }));
      },
      bluetoothBleWriteAndReadCharacteristic(request) {
        const bytes = bytesFromPayload(request);
        return characteristic(request.sessionId, request.writeServiceUuid, request.writeCharacteristicUuid)
          .writeValue(bytes)
          .then(() =>
            characteristic(request.sessionId, request.readServiceUuid, request.readCharacteristicUuid).readValue()
          )
          .then((value) => readData(request.sessionId, value));
      },
      bluetoothBleSubscribeCharacteristic(request) {
        const item = characteristic(request.sessionId, request.serviceUuid, request.characteristicUuid);
        if (!request.enable) {
          return item.stopNotifications().then(() => ({ sessionId: request.sessionId, bytesWritten: 0 }));
        }
        item.addEventListener("characteristicvaluechanged", (event) => {
          const value = new Uint8Array(event.target.value.buffer.slice(event.target.value.byteOffset, event.target.value.byteOffset + event.target.value.byteLength));
          notifications.get(request.sessionId).push({
            characteristicUuid: item.uuid,
            bytesRead: value.length,
            text: textDecoder.decode(value),
            dataBase64: bytesToBase64(value),
            timestamp: Date.now(),
          });
        });
        return item.startNotifications().then(() => ({ sessionId: request.sessionId, bytesWritten: 0 }));
      },
      bluetoothBleReadNotifications(sessionId, limit) {
        const queue = notifications.get(sessionId);
        if (!queue) {
          throw new Error(`BLE session not found: ${sessionId}`);
        }
        return {
          sessionId,
          notifications: queue.splice(0, Math.max(Number(limit || 50), 0)),
        };
      },
    };
  })();

  function characteristicPropertyNames(properties) {
    const names = [];
    if (properties.read) names.push("read");
    if (properties.write) names.push("write");
    if (properties.writeWithoutResponse) names.push("write_without_response");
    if (properties.notify) names.push("notify");
    if (properties.indicate) names.push("indicate");
    return names;
  }

  // Schedules browser local inference discovery after storage changes.
  function scheduleWebLocalInferenceRefresh() {
    webLocalInferenceReadyPromise = null;
    queueMicrotask(() => {
      void ensureWebLocalInference().catch((error) => {
        console.warn("[Operit local inference]", error);
      });
    });
  }

  // Initializes installed browser local inference bundles.
  async function ensureWebLocalInference() {
    if (!webLocalInferenceReadyPromise) {
      webLocalInferenceReadyPromise = (async () => {
        const state = {
          asrBundles: new Map(),
          ttsBundles: new Map(),
          blobUrls: [],
        };
        try {
          await loadInstalledWebTtsBundles(state);
          await loadInstalledWebAsrBundles(state);
        } catch (error) {
          disposeWebLocalInferenceState(state);
          throw error;
        }
        disposeWebLocalInferenceState(webLocalInferenceState);
        webLocalInferenceState = state;
        globalThis.__operitLocalInference = {
          transcribeLocalSpeech: transcribeWebLocalSpeech,
          synthesizeLocalSpeech: synthesizeWebLocalSpeech,
        };
      })();
    }
    return webLocalInferenceReadyPromise;
  }

  // Releases all native objects and Blob URLs owned by one Web inference state.
  function disposeWebLocalInferenceState(state) {
    if (!state) {
      return;
    }
    for (const bundle of state.asrBundles.values()) {
      bundle.recognizer.free();
    }
    for (const bundle of state.ttsBundles.values()) {
      bundle.worker.terminate();
    }
    for (const url of state.blobUrls) {
      URL.revokeObjectURL(url);
    }
  }

  // Loads every complete browser ASR bundle visible in runtime storage.
  async function loadInstalledWebAsrBundles(state) {
    const roots = runtimeBundleRoots("sherpa-onnx-asr.js");
    for (const root of roots) {
      const paths = {
        recognizerScript: `${root}/sherpa-onnx-asr.js`,
        runtimeScript: `${root}/sherpa-onnx-wasm-main-vad-asr.js`,
        runtimeWasm: `${root}/sherpa-onnx-wasm-main-vad-asr.wasm`,
        runtimeData: `${root}/sherpa-onnx-wasm-main-vad-asr.data`,
      };
      if (runtimePathsExist(Object.values(paths))) {
        state.asrBundles.set(root, await createWebAsrBundle(paths, state));
      }
    }
  }

  // Loads every complete browser TTS bundle visible in runtime storage.
  async function loadInstalledWebTtsBundles(state) {
    const roots = runtimeBundleRoots("sherpa-onnx-tts.js");
    for (const root of roots) {
      const paths = {
        ttsScript: `${root}/sherpa-onnx-tts.js`,
        runtimeScript: `${root}/sherpa-onnx-wasm-main-tts.js`,
        runtimeWasm: `${root}/sherpa-onnx-wasm-main-tts.wasm`,
        runtimeData: `${root}/sherpa-onnx-wasm-main-tts.data`,
      };
      if (runtimePathsExist(Object.values(paths))) {
        state.ttsBundles.set(root, await createWebTtsBundle(paths, state));
      }
    }
  }

  // Returns storage roots ending with one exact bundle file name.
  function runtimeBundleRoots(fileName) {
    const suffix = `/${fileName}`;
    const roots = [];
    for (const itemKey of storageCache.keys()) {
      if (!itemKey.startsWith(runtimePrefix) || !itemKey.endsWith(suffix)) {
        continue;
      }
      roots.push(itemKey.substring(runtimePrefix.length, itemKey.length - suffix.length));
    }
    return roots;
  }

  // Checks that every runtime path is present in the synchronous storage view.
  function runtimePathsExist(paths) {
    return paths.every((path) => storageExists(runtimePrefix, path));
  }

  // Creates a blob URL for one runtime-storage file.
  function runtimeBlobUrl(path, contentType, state) {
    const bytes = storageRead(runtimePrefix, path);
    if (bytes.length === 0) {
      throw new Error(`runtime file is empty or missing: ${path}`);
    }
    const url = URL.createObjectURL(new Blob([bytes], { type: contentType }));
    state.blobUrls.push(url);
    return url;
  }

  // Creates a JavaScript Blob URL with one exact source suffix.
  function runtimeJavaScriptUrl(path, suffix, state) {
    const bytes = storageRead(runtimePrefix, path);
    if (bytes.length === 0) {
      throw new Error(`runtime file is empty or missing: ${path}`);
    }
    const source = `${textDecoder.decode(bytes)}\n${suffix}\n`;
    const url = URL.createObjectURL(new Blob([source], { type: "text/javascript" }));
    state.blobUrls.push(url);
    return url;
  }

  // Loads a classic script from a blob URL.
  function loadClassicScriptUrl(src) {
    return new Promise((resolve, reject) => {
      const script = document.createElement("script");
      script.src = src;
      script.onload = resolve;
      script.onerror = () => reject(new Error(`failed to load ${src}`));
      document.head.appendChild(script);
    });
  }

  // Builds one browser ASR bundle from installed Sherpa files.
  async function createWebAsrBundle(paths, state) {
    requireCrossOriginIsolation("ASR");
    const urls = {
      recognizerScript: runtimeJavaScriptUrl(
        paths.recognizerScript,
        "globalThis.__operitSherpaAsrClasses = { OfflineRecognizer };",
        state,
      ),
      runtimeScript: runtimeBlobUrl(paths.runtimeScript, "text/javascript", state),
      runtimeWasm: runtimeBlobUrl(paths.runtimeWasm, "application/wasm", state),
      runtimeData: runtimeBlobUrl(paths.runtimeData, "application/octet-stream", state),
    };
    const moduleValue = {};
    const ready = new Promise((resolve, reject) => {
      moduleValue.mainScriptUrlOrBlob = urls.runtimeScript;
      moduleValue.locateFile = (path) => {
        if (path === "sherpa-onnx-wasm-main-vad-asr.wasm") return urls.runtimeWasm;
        if (path === "sherpa-onnx-wasm-main-vad-asr.data") return urls.runtimeData;
        return path;
      };
      moduleValue.setStatus = (status) => console.debug("[Operit ASR]", status);
      moduleValue.onRuntimeInitialized = () => resolve(moduleValue);
      moduleValue.onAbort = (reason) => reject(new Error(String(reason)));
    });
    globalThis.Module = moduleValue;
    await loadClassicScriptUrl(urls.runtimeScript);
    await ready;
    await loadClassicScriptUrl(urls.recognizerScript);
    const classes = globalThis.__operitSherpaAsrClasses;
    if (!classes || typeof classes.OfflineRecognizer !== "function") {
      throw new Error("Web ASR recognizer class was not exported");
    }
    const recognizer = new classes.OfflineRecognizer(webAsrConfig(), moduleValue);
    return { recognizer, moduleValue };
  }

  // Returns the Paraformer ASR config embedded in the Web bundle.
  function webAsrConfig() {
    return {
      modelConfig: {
        debug: 0,
        tokens: "./tokens.txt",
        paraformer: {
          model: "./paraformer.onnx",
        },
      },
    };
  }

  // Builds one browser TTS bundle from installed Sherpa files.
  async function createWebTtsBundle(paths, state) {
    requireCrossOriginIsolation("TTS");
    const urls = {
      ttsScript: runtimeBlobUrl(paths.ttsScript, "text/javascript", state),
      runtimeScript: runtimeBlobUrl(paths.runtimeScript, "text/javascript", state),
      runtimeWasm: runtimeBlobUrl(paths.runtimeWasm, "application/wasm", state),
      runtimeData: runtimeBlobUrl(paths.runtimeData, "application/octet-stream", state),
    };
    const workerSource = webTtsWorkerSource(urls);
    const workerUrl = URL.createObjectURL(new Blob([workerSource], { type: "text/javascript" }));
    state.blobUrls.push(workerUrl);
    const worker = new Worker(workerUrl, { type: "module", name: "operit-web-tts" });
    const instance = await waitForWebTtsWorker(worker);
    return {
      worker,
      numSpeakers: instance.numSpeakers,
      sampleRate: instance.sampleRate,
    };
  }

  // Builds the isolated module-worker source required by Sherpa Web TTS.
  function webTtsWorkerSource(urls) {
    return `
import createModule from ${JSON.stringify(urls.runtimeScript)};
import { createOfflineTts } from ${JSON.stringify(urls.ttsScript)};

const pendingAudio = new Map();

// Writes one worker failure into the shared control buffer.
function writeError(controlBuffer, error) {
  const control = new Int32Array(controlBuffer, 0, 3);
  const payload = new Uint8Array(controlBuffer, 12);
  const message = error instanceof Error ? error.stack || error.message : String(error);
  const bytes = new TextEncoder().encode(message);
  const length = Math.min(bytes.length, payload.length);
  payload.set(bytes.subarray(0, length));
  Atomics.store(control, 1, length);
  Atomics.store(control, 0, -1);
  Atomics.notify(control, 0);
}

// Encodes Float32 samples into mono PCM16 WAV bytes.
function encodeWav(samples, sampleRate) {
  const bytes = new Uint8Array(44 + samples.length * 2);
  const view = new DataView(bytes.buffer);
  view.setUint32(0, 0x46464952, true);
  view.setUint32(4, 36 + samples.length * 2, true);
  view.setUint32(8, 0x45564157, true);
  view.setUint32(12, 0x20746d66, true);
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  view.setUint32(36, 0x61746164, true);
  view.setUint32(40, samples.length * 2, true);
  for (let index = 0; index < samples.length; index += 1) {
    const value = Math.max(-1, Math.min(1, samples[index]));
    view.setInt16(44 + index * 2, value * 32767, true);
  }
  return bytes;
}

let tts = null;
try {
  const moduleValue = await createModule({
    mainScriptUrlOrBlob: ${JSON.stringify(urls.runtimeScript)},
    locateFile(path) {
      if (path === "sherpa-onnx-wasm-main-tts.wasm") return ${JSON.stringify(urls.runtimeWasm)};
      if (path === "sherpa-onnx-wasm-main-tts.data") return ${JSON.stringify(urls.runtimeData)};
      return path;
    },
    setStatus(status) {
      self.postMessage({ type: "status", status });
    },
  });
  tts = createOfflineTts(moduleValue, {
    offlineTtsModelConfig: {
      offlineTtsVitsModelConfig: {
        model: "./en_US-libritts_r-medium.onnx",
        lexicon: "",
        tokens: "./tokens.txt",
        dataDir: "./espeak-ng-data",
        noiseScale: 0.667,
        noiseScaleW: 0.8,
        lengthScale: 1.0,
      },
      numThreads: 1,
      debug: 0,
      provider: "cpu",
    },
    ruleFsts: "",
    ruleFars: "",
    maxNumSentences: 1,
    silenceScale: 0.2,
  });
  self.postMessage({
    type: "ready",
    numSpeakers: tts.numSpeakers,
    sampleRate: tts.sampleRate,
  });
} catch (error) {
  const message = error instanceof Error ? error.stack || error.message : String(error);
  self.postMessage({ type: "initError", message });
}

self.onmessage = (event) => {
  const message = event.data;
  try {
    if (message.type === "generate") {
      const audio = tts.generate({
        text: message.text,
        sid: message.sid,
        speed: message.speed,
      });
      const bytes = encodeWav(audio.samples, audio.sampleRate || tts.sampleRate);
      pendingAudio.set(message.requestId, bytes);
      const control = new Int32Array(message.controlBuffer, 0, 3);
      Atomics.store(control, 1, bytes.length);
      Atomics.store(control, 0, 1);
      Atomics.notify(control, 0);
      return;
    }
    if (message.type === "copy") {
      const bytes = pendingAudio.get(message.requestId);
      if (!bytes) throw new Error("Web TTS pending audio is missing");
      const output = new Uint8Array(message.outputBuffer);
      if (output.length !== bytes.length) throw new Error("Web TTS output buffer length mismatch");
      output.set(bytes);
      pendingAudio.delete(message.requestId);
      const control = new Int32Array(message.controlBuffer, 0, 3);
      Atomics.store(control, 0, 2);
      Atomics.notify(control, 0);
      return;
    }
    throw new Error("Web TTS worker method is unknown");
  } catch (error) {
    writeError(message.controlBuffer, error);
  }
};
`;
  }

  // Waits for one Web TTS worker to initialize its model instance.
  function waitForWebTtsWorker(worker) {
    return new Promise((resolve, reject) => {
      const onMessage = (event) => {
        const message = event.data;
        if (message.type === "status") {
          console.debug("[Operit TTS]", message.status);
          return;
        }
        if (message.type === "ready") {
          worker.removeEventListener("message", onMessage);
          worker.removeEventListener("error", onError);
          resolve(message);
          return;
        }
        if (message.type === "initError") {
          worker.removeEventListener("message", onMessage);
          worker.removeEventListener("error", onError);
          reject(new Error(message.message));
        }
      };
      const onError = (event) => {
        worker.removeEventListener("message", onMessage);
        worker.removeEventListener("error", onError);
        reject(new Error(event.message || "Web TTS worker initialization failed"));
      };
      worker.addEventListener("message", onMessage);
      worker.addEventListener("error", onError);
    });
  }

  // Runs one synchronous command against an initialized Web TTS worker.
  function generateWebTtsWav(bundle, text, speaker, speed) {
    const requestId = crypto.randomUUID();
    const controlBuffer = new SharedArrayBuffer(65_536);
    const control = new Int32Array(controlBuffer, 0, 3);
    bundle.worker.postMessage({
      type: "generate",
      requestId,
      text,
      sid: speaker,
      speed,
      controlBuffer,
    });
    waitForWebTtsControl(control, 1);
    const byteLength = Atomics.load(control, 1);
    if (byteLength <= 44) {
      throw new Error(`Web TTS worker returned an invalid WAV length: ${byteLength}`);
    }
    const outputBuffer = new SharedArrayBuffer(byteLength);
    Atomics.store(control, 0, 0);
    bundle.worker.postMessage({
      type: "copy",
      requestId,
      outputBuffer,
      controlBuffer,
    });
    waitForWebTtsControl(control, 2);
    return new Uint8Array(outputBuffer);
  }

  // Waits for one exact worker state while preserving worker error text.
  function waitForWebTtsControl(control, expectedState) {
    const deadline = performance.now() + 600_000;
    while (true) {
      const state = Atomics.load(control, 0);
      if (state === expectedState) {
        return;
      }
      if (state === -1) {
        const length = Atomics.load(control, 1);
        const bytes = new Uint8Array(control.buffer, 12, length);
        throw new Error(new TextDecoder().decode(bytes));
      }
      if (performance.now() >= deadline) {
        throw new Error("Web TTS worker command timed out");
      }
    }
  }

  // Requires the response isolation headers needed by threaded Sherpa WASM.
  function requireCrossOriginIsolation(capability) {
    if (globalThis.crossOriginIsolated !== true) {
      throw new Error(
        `Web local ${capability} requires Cross-Origin-Opener-Policy: same-origin and ` +
          "Cross-Origin-Embedder-Policy: require-corp",
      );
    }
  }

  // Transcribes one local Web speech request.
  function transcribeWebLocalSpeech(requestJson) {
    const state = requireWebLocalInferenceState();
    const request = JSON.parse(requestJson);
    const driver = parseTaggedDriver(request.driverJson, "SherpaOnnxWebAsrBundle");
    const root = runtimeDirectoryForDriver(request.modelDirectory, driver.recognizerScript);
    const bundle = state.asrBundles.get(root);
    if (!bundle) {
      throw new Error(`Web ASR bundle is not initialized: ${root}`);
    }
    const wav = decodeMonoPcmWav(storageRead(runtimePrefix, request.audioPath));
    const stream = bundle.recognizer.createStream();
    try {
      stream.acceptWaveform(wav.sampleRate, wav.samples);
      bundle.recognizer.decode(stream);
      const result = bundle.recognizer.getResult(stream);
      return JSON.stringify({
        text: result.text || "",
        resultJson: JSON.stringify(result),
      });
    } finally {
      stream.free();
    }
  }

  // Synthesizes one local Web speech request.
  function synthesizeWebLocalSpeech(requestJson) {
    const state = requireWebLocalInferenceState();
    const request = JSON.parse(requestJson);
    const driver = parseTaggedDriver(request.driverJson, "SherpaOnnxWebTtsBundle");
    const speaker = Number.parseInt(String(request.voice), 10);
    if (!Number.isInteger(speaker) || speaker < 0 || speaker >= driver.speakerCount) {
      throw new Error(`Web TTS speaker is outside 0..${driver.speakerCount - 1}`);
    }
    const root = runtimeDirectoryForDriver(request.modelDirectory, driver.ttsScript);
    const bundle = state.ttsBundles.get(root);
    if (!bundle) {
      throw new Error(`Web TTS bundle is not initialized: ${root}`);
    }
    if (bundle.numSpeakers !== driver.speakerCount) {
      throw new Error(
        `Web TTS speaker count mismatch: manifest=${driver.speakerCount}, ` +
          `engine=${bundle.numSpeakers}`,
      );
    }
    const wav = generateWebTtsWav(
      bundle,
      String(request.text),
      speaker,
      Number(request.speed),
    );
    storageWrite(runtimePrefix, request.outputPath, wav);
    return JSON.stringify({
      audioPath: request.outputPath,
      outputFormat: "wav",
    });
  }

  // Returns the initialized Web local inference state.
  function requireWebLocalInferenceState() {
    if (!webLocalInferenceState) {
      throw new Error("Web local inference runner is still initializing");
    }
    return webLocalInferenceState;
  }

  // Parses one externally tagged local model driver.
  function parseTaggedDriver(driverJson, expectedTag) {
    const root = JSON.parse(driverJson);
    const keys = Object.keys(root);
    if (keys.length !== 1 || keys[0] !== expectedTag) {
      throw new Error(`Web local inference driver must be ${expectedTag}`);
    }
    return root[expectedTag];
  }

  // Resolves a model bundle root from model directory and driver script path.
  function runtimeDirectoryForDriver(modelDirectory, relativeFilePath) {
    const directory = normalizeRuntimePath(modelDirectory);
    const filePath = normalizeRuntimePath(relativeFilePath);
    const slash = filePath.lastIndexOf("/");
    if (slash < 0) {
      return directory;
    }
    return normalizeRuntimePath(`${directory}/${filePath.slice(0, slash)}`);
  }

  // Normalizes runtime storage paths to slash separators.
  function normalizeRuntimePath(path) {
    return String(path).replace(/\\/g, "/").replace(/^\/+/, "").replace(/\/+$/, "");
  }

  // Decodes one mono PCM WAV byte payload into Float32 samples.
  function decodeMonoPcmWav(bytes) {
    if (bytes.length < 44) {
      throw new Error("WAV input is too small");
    }
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    if (view.getUint32(0, true) !== 0x46464952 || view.getUint32(8, true) !== 0x45564157) {
      throw new Error("WAV input has invalid RIFF header");
    }
    let offset = 12;
    let sampleRate = 0;
    let channels = 0;
    let bitsPerSample = 0;
    let audioFormat = 0;
    let dataOffset = -1;
    let dataSize = 0;
    while (offset + 8 <= view.byteLength) {
      const chunkId = view.getUint32(offset, true);
      const chunkSize = view.getUint32(offset + 4, true);
      const chunkDataOffset = offset + 8;
      if (chunkId === 0x20746d66) {
        audioFormat = view.getUint16(chunkDataOffset, true);
        channels = view.getUint16(chunkDataOffset + 2, true);
        sampleRate = view.getUint32(chunkDataOffset + 4, true);
        bitsPerSample = view.getUint16(chunkDataOffset + 14, true);
      } else if (chunkId === 0x61746164) {
        dataOffset = chunkDataOffset;
        dataSize = chunkSize;
      }
      offset = chunkDataOffset + chunkSize + (chunkSize % 2);
    }
    if (audioFormat !== 1 || channels !== 1 || bitsPerSample !== 16 || sampleRate <= 0) {
      throw new Error("Web local STT requires mono PCM16 WAV input");
    }
    if (dataOffset < 0 || dataOffset + dataSize > view.byteLength) {
      throw new Error("WAV input has no complete data chunk");
    }
    const sampleCount = dataSize / 2;
    const samples = new Float32Array(sampleCount);
    for (let index = 0; index < sampleCount; index += 1) {
      samples[index] = view.getInt16(dataOffset + index * 2, true) / 32768;
    }
    return { sampleRate, samples };
  }

  // Resolves a synchronous browser local inference runner method.
  function localInferenceRunner(method) {
    const runner = globalThis.__operitLocalInference;
    if (!runner || typeof runner[method] !== "function") {
      throw new Error(`web local inference method is not installed: ${method}`);
    }
    // Calls the resolved runner and validates its JSON string contract.
    return function runLocalInference(requestJson) {
      const responseJson = runner[method](requestJson);
      if (typeof responseJson !== "string") {
        throw new Error(`web local inference method returned non-string JSON: ${method}`);
      }
      return responseJson;
    };
  }

  // Installs an isolated smoke-test API when explicitly requested by the test page.
  function installWebLocalInferenceTestApi() {
    if (globalThis.__OPERIT_LOCAL_INFERENCE_TEST__ !== true) {
      return;
    }
    globalThis.__operitLocalInferenceTest = {
      putRuntimeFile(path, content) {
        storageCache.set(key(runtimePrefix, path), new Uint8Array(content));
      },
      readRuntimeFile(path) {
        return storageRead(runtimePrefix, path);
      },
      async initialize() {
        webLocalInferenceReadyPromise = null;
        await ensureWebLocalInference();
      },
      transcribe(request) {
        return JSON.parse(transcribeWebLocalSpeech(JSON.stringify(request)));
      },
      synthesize(request) {
        return JSON.parse(synthesizeWebLocalSpeech(JSON.stringify(request)));
      },
      dispose() {
        disposeWebLocalInferenceState(webLocalInferenceState);
        webLocalInferenceState = null;
        webLocalInferenceReadyPromise = null;
      },
    };
  }

  installWebLocalInferenceTestApi();

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
    hostSecretStore: {
      // Reads a host-owned secret from browser-local protected storage.
      readSecret(key) {
        const value = localStorage.getItem(`${secretPrefix}${key}`);
        return value === null ? null : base64ToBytes(value);
      },
      // Writes a host-owned secret into browser-local protected storage.
      writeSecret(key, content) {
        localStorage.setItem(`${secretPrefix}${key}`, bytesToBase64(new Uint8Array(content)));
      },
      // Deletes a host-owned secret from browser-local protected storage.
      deleteSecret(key) {
        localStorage.removeItem(`${secretPrefix}${key}`);
      },
    },
    sqlite: {
      open(path) {
        if (!SQLite) {
          throw new Error("sqlite host is not initialized");
        }
        const id = `sqlite-${++sqliteConnectionIndex}`;
        const bytes = storageCache.get(sqliteKey(path));
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
    localInference: {
      // Transcribes one local speech request through the installed browser runner.
      transcribeLocalSpeech(requestJson) {
        return localInferenceRunner("transcribeLocalSpeech")(requestJson);
      },
      // Synthesizes one local speech request through the installed browser runner.
      synthesizeLocalSpeech(requestJson) {
        return localInferenceRunner("synthesizeLocalSpeech")(requestJson);
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
      downloadFile(request) {
        const xhr = new XMLHttpRequest();
        xhr.open("GET", request.url, false);
        xhr.overrideMimeType("text/plain; charset=x-user-defined");
        for (const pair of request.headers || []) {
          const name = Array.isArray(pair) ? pair[0] : pair.key;
          const value = Array.isArray(pair) ? pair[1] : pair.value;
          xhr.setRequestHeader(name, value);
        }
        xhr.send(null);
        if (xhr.status < 200 || xhr.status >= 300) {
          throw new Error(`download ${request.fileId} failed with HTTP ${xhr.status}`);
        }
        const raw = xhr.responseText || "";
        const bytes = new Uint8Array(raw.length);
        for (let index = 0; index < raw.length; index += 1) {
          bytes[index] = raw.charCodeAt(index) & 0xff;
        }
        if (typeof request.expectedBytes === "number" && bytes.length !== request.expectedBytes) {
          throw new Error(`download ${request.fileId} size mismatch: ${bytes.length} != ${request.expectedBytes}`);
        }
        storageWrite(runtimePrefix, request.targetPath, bytes);
        return {
          fileId: String(request.fileId),
          finalUrl: xhr.responseURL || request.url,
          targetPath: String(request.targetPath),
          downloadedBytes: bytes.length,
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
      writeLines() {},
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
    musicPlayback,
    bluetooth,
    ttsPlayback,
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
        await ensureBrowserStorage();
        await ensureSqlite();
        await ensureWebLocalInference();
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
    async pushOpen(request) {
      return (await bridge()).pushOpen(request);
    },
    async pushItem(item) {
      return (await bridge()).pushItem(item);
    },
    async pushClose(pushId) {
      return (await bridge()).pushClose(pushId);
    },
    async watchSnapshot(request) {
      return (await bridge()).watchSnapshot(request);
    },
    async watchStream(request, onEvent) {
      return (await bridge()).watchStream(request, onEvent);
    },
    async closeWatchStream(subscriptionId) {
      return (await bridge()).closeWatchStream(subscriptionId);
    },
  };
})();
