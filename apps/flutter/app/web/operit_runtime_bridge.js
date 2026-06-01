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
          model: navigator.userAgent,
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
    async nextBrowserAutomationRequest() {
      return (await bridge()).nextBrowserAutomationRequest();
    },
    async handleBrowserAutomationResult(result) {
      return (await bridge()).handleBrowserAutomationResult(result);
    },
  };
})();
