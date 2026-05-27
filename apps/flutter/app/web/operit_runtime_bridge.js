(function () {
  const textEncoder = new TextEncoder();
  const textDecoder = new TextDecoder();
  const runtimePrefix = "operit2.runtime.";
  const filePrefix = "operit2.files.";
  const sqliteConnections = new Map();
  let sqliteConnectionIndex = 0;

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
        const id = `sqlite-${++sqliteConnectionIndex}`;
        sqliteConnections.set(id, { path });
        return id;
      },
      executeBatch() {
        unavailable("sqlite.executeBatch");
      },
      execute() {
        unavailable("sqlite.execute");
      },
      query() {
        unavailable("sqlite.query");
      },
      lastInsertRowId() {
        return "0";
      },
      beginTransaction() {
        unavailable("sqlite.beginTransaction");
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
        await module.default("./operit_flutter_bridge_bg.wasm");
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
  };
})();
