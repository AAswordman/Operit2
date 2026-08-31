function parseBrowserNotificationActivation(activationJson) {
    const activation = JSON.parse(activationJson);
    if (activation.type === "open_application") {
        return { type: activation.type };
    }
    if (activation.type === "open_chat" && typeof activation.chatId === "string" && activation.chatId.length > 0) {
        return { type: activation.type, chatId: activation.chatId };
    }
    throw new Error("Browser notification activation is invalid");
}
function browserNotificationActivationUrl(activation, activationJson) {
    if (activation.type === "open_application") {
        return null;
    }
    const target = new URL(window.location.href);
    target.searchParams.set("operitNotificationActivation", activationJson);
    return target.toString();
}
function showBrowserNotification(title, message, activation, activationJson) {
    const notification = new Notification(title, { body: message, tag: "operit" });
    notification.onclick = () => {
        notification.close();
        window.focus();
        const target = browserNotificationActivationUrl(activation, activationJson);
        if (target !== null) {
            window.location.assign(target);
        }
    };
}
function createRandomUuid() {
    const bytes = new Uint8Array(16);
    globalThis.crypto.getRandomValues(bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    const hex = Array.from(bytes, byte => byte.toString(16).padStart(2, "0"));
    return [
        hex.slice(0, 4).join(""),
        hex.slice(4, 6).join(""),
        hex.slice(6, 8).join(""),
        hex.slice(8, 10).join(""),
        hex.slice(10, 16).join(""),
    ].join("-");
}
(function () {
    const runtimeGlobal = globalThis;
    const workerHostModules = new WeakSet();
    const mainHostModules = new WeakSet();
    runtimeGlobal.__operitWorkerHostModules = workerHostModules;
    runtimeGlobal.__operitMainHostModules = mainHostModules;
    const browserNavigator = navigator;
    const importRuntimeScript = (path) => import(path);
    const blobPart = (bytes) => Uint8Array.from(bytes);
    const ownedBytes = (bytes) => Uint8Array.from(bytes);
    function registerWorkerHostModule(module) {
        workerHostModules.add(module);
        return module;
    }
    function registerMainHostModule(module) {
        mainHostModules.add(module);
        return module;
    }
    const textEncoder = new TextEncoder();
    const textDecoder = new TextDecoder();
    const runtimeStorageConfigKey = "operit2.client.runtime.local_storage";
    const runtimeIdentityId = resolveRuntimeIdentityId();
    const runtimeIdentityNamespace = `operit2.identities.${runtimeIdentityId}`;
    const runtimePrefix = "operit2.runtime.";
    const filePrefix = "operit2.files.";
    const sqlitePrefix = "operit2.sqlite.";
    const secretPrefix = `${runtimeIdentityNamespace}.secrets.`;
    const storageDatabaseName = `${runtimeIdentityNamespace}.host.storage`;
    const httpDownloadDatabaseName = `${runtimeIdentityNamespace}.http.downloads`;
    const runtimeCoordinatorName = `${runtimeIdentityNamespace}.runtime.coordinator`;
    const runtimeOwnerLockName = `${runtimeIdentityNamespace}.runtime.owner`;
    const storageObjectStoreName = "entries";
    const httpDownloadObjectStoreName = "downloads";
    const storageCache = new Map();
    const workerChangedStorageKeys = new Set();
    const workerSecrets = new Map();
    const workerDownloads = new Map();
    const workerDownloadRequests = new Map();
    const workerChangedSecretKeys = new Set();
    const fileDirectories = new Set();
    const sqliteConnections = new Map();
    const sqliteTransactions = new Map();
    let sqliteConnectionIndex = 0;
    let sqliteTransactionIndex = 0;
    let sqliteModulePromise = null;
    let SQLite = null;
    let storageDatabasePromise = null;
    let httpDownloadDatabasePromise = null;
    let httpDownloadStatusCachePromise = null;
    const httpDownloadStatusCache = new Map();
    const activeHttpDownloadControllers = new Map();
    const activeHttpByteStreamControllers = new Map();
    const activeHttpDownloadPromises = new Map();
    const activeModelInstallAborters = new Map();
    const activeModelInstallPromises = new Map();
    const modelInstallTaskGenerations = new Map();
    let modelInstallTaskGeneration = 0;
    let modelInstallCommitQueue = Promise.resolve();
    let storageReadyPromise = null;
    let webLocalInferenceReadyPromise = null;
    let webLocalInferenceState = null;
    const linuxVmSessions = new Map();
    const linuxVmOutputLimit = 4 * 1024 * 1024;
    let linuxVmCommandIndex = 0;
    const managedRuntimeProcesses = new Map();
    const managedRuntimeHeaderLength = 4;
    const managedRuntimeOutputWriteIndex = 0;
    const managedRuntimeOutputReadIndex = 1;
    const managedRuntimeStateIndex = 2;
    const managedRuntimeExitCodeIndex = 3;
    const managedRuntimeStarting = 0;
    const managedRuntimeRunning = 1;
    const managedRuntimeFailed = 2;
    const managedRuntimeStopped = 3;
    const managedRuntimeOutputCapacity = 8 * 1024 * 1024;
    const managedRuntimeCommandTimeoutMs = 180_000;
    let managedRuntimeProcessIndex = 0;
    function resolveRuntimeIdentityId() {
        const urlIdentityId = new URL(globalThis.location.href).searchParams.get("identity");
        if (urlIdentityId !== null) {
            validateRuntimeIdentityId(urlIdentityId);
            return urlIdentityId;
        }
        if (!isMainRuntimeHost()) {
            throw new Error("Web runtime worker URL is missing its identity");
        }
        const encoded = localStorage.getItem(runtimeStorageConfigKey);
        if (encoded === null) {
            return createInitialRuntimeIdentity();
        }
        return parseRuntimeBootstrapConfig(encoded).activeIdentityId;
    }
    function createInitialRuntimeIdentity() {
        const now = Date.now();
        const identityId = `identity-${now}-${createRandomUuid()}`;
        validateRuntimeIdentityId(identityId);
        localStorage.setItem(runtimeStorageConfigKey, JSON.stringify({
            confirmed: false,
            runtimeRoot: "",
            workspaceRoot: "",
            identities: [{ id: identityId, name: "", createdAt: now }],
            activeIdentityId: identityId,
            updatedAt: now,
        }));
        return identityId;
    }
    function validateRuntimeIdentityId(identityId) {
        if (!/^identity-[a-z0-9-]+$/.test(identityId)) {
            throw new Error(`Web runtime identity is invalid: ${identityId}`);
        }
    }
    function parseRuntimeBootstrapConfig(encoded) {
        const parsed = JSON.parse(encoded);
        if (typeof parsed.confirmed !== "boolean" ||
            typeof parsed.runtimeRoot !== "string" ||
            typeof parsed.workspaceRoot !== "string" ||
            !Array.isArray(parsed.identities) ||
            typeof parsed.activeIdentityId !== "string" ||
            typeof parsed.updatedAt !== "number" ||
            !Number.isSafeInteger(parsed.updatedAt) || parsed.updatedAt <= 0) {
            throw new Error("Web runtime bootstrap record is invalid");
        }
        if (parsed.confirmed &&
            (parsed.runtimeRoot.trim().length === 0 || parsed.workspaceRoot.trim().length === 0)) {
            throw new Error("Web runtime bootstrap storage roots are invalid");
        }
        const identityIds = new Set();
        const identities = parsed.identities.map(value => {
            if (typeof value !== "object" || value === null) {
                throw new Error("Web runtime bootstrap identity is invalid");
            }
            const identity = value;
            if (typeof identity.id !== "string" || typeof identity.name !== "string" ||
                typeof identity.createdAt !== "number" || !Number.isSafeInteger(identity.createdAt) ||
                identity.createdAt <= 0 || identity.name.trim().length > 80) {
                throw new Error("Web runtime bootstrap identity is invalid");
            }
            validateRuntimeIdentityId(identity.id);
            if (identityIds.has(identity.id)) {
                throw new Error(`Web runtime bootstrap identity is duplicated: ${identity.id}`);
            }
            identityIds.add(identity.id);
            return identity;
        });
        if (identities.length === 0 || !identityIds.has(parsed.activeIdentityId)) {
            throw new Error("Web runtime active identity does not exist");
        }
        return {
            confirmed: parsed.confirmed,
            runtimeRoot: parsed.runtimeRoot,
            workspaceRoot: parsed.workspaceRoot,
            identities,
            activeIdentityId: parsed.activeIdentityId,
            updatedAt: parsed.updatedAt,
        };
    }
    function readRuntimeBootstrapConfig() {
        let encoded = localStorage.getItem(runtimeStorageConfigKey);
        if (encoded === null) {
            createInitialRuntimeIdentity();
            encoded = localStorage.getItem(runtimeStorageConfigKey);
            if (encoded === null) {
                throw new Error("Web runtime bootstrap record was not persisted");
            }
        }
        parseRuntimeBootstrapConfig(encoded);
        return encoded;
    }
    function defaultRuntimeStoragePaths() {
        return {
            runtimeRoot: "browser-storage/runtime",
            workspaceRoot: "browser-storage/workspaces",
        };
    }
    function normalizeRuntimeStoragePaths(runtimeRoot, workspaceRoot) {
        const normalizedRuntimeRoot = runtimeRoot.trim();
        const normalizedWorkspaceRoot = workspaceRoot.trim();
        if (normalizedRuntimeRoot.length === 0 || normalizedWorkspaceRoot.length === 0) {
            throw new Error("Runtime and workspace roots must not be empty");
        }
        return {
            runtimeRoot: normalizedRuntimeRoot,
            workspaceRoot: normalizedWorkspaceRoot,
        };
    }
    function installRuntimeStoragePaths(runtimeRoot, workspaceRoot) {
        normalizeRuntimeStoragePaths(runtimeRoot, workspaceRoot);
    }
    function writeRuntimeBootstrapConfig(content) {
        parseRuntimeBootstrapConfig(content);
        localStorage.setItem(runtimeStorageConfigKey, content);
    }
    function restartRuntimeApplication() {
        globalThis.location.reload();
    }
    function runtimeIdentityWorkerUrl(path) {
        const url = new URL(path, import.meta.url);
        url.searchParams.set("identity", runtimeIdentityId);
        return url;
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
    function key(prefix, path) {
        return prefix + normalizeRuntimePath(path);
    }
    function isModelInstallWorker() {
        return runtimeGlobal.__OPERIT_MODEL_INSTALL_WORKER__ === true;
    }
    function isRuntimeWorker() {
        return runtimeGlobal.__OPERIT_RUNTIME_WORKER__ === true;
    }
    function isMainRuntimeHost() {
        return !isModelInstallWorker() && !isRuntimeWorker();
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
    function collectModelInstallWorkerSecrets() {
        const secrets = [];
        for (let index = 0; index < localStorage.length; index += 1) {
            const storageKey = localStorage.key(index);
            if (storageKey === null || !storageKey.startsWith(secretPrefix)) {
                continue;
            }
            const value = localStorage.getItem(storageKey);
            if (value === null) {
                continue;
            }
            secrets.push({
                key: storageKey.slice(secretPrefix.length),
                bytes: base64ToBytes(value),
            });
        }
        return secrets;
    }
    function setModelInstallWorkerSecrets(secrets) {
        workerSecrets.clear();
        for (const secret of secrets) {
            workerSecrets.set(secret.key, Uint8Array.from(secret.bytes));
        }
    }
    function setModelInstallWorkerDownloads(downloads) {
        workerDownloads.clear();
        for (const download of downloads) {
            workerDownloads.set(download.url, Uint8Array.from(download.bytes));
        }
    }
    function collectModelInstallWorkerDownloadRequests() {
        const requests = Array.from(workerDownloadRequests.values());
        workerDownloadRequests.clear();
        return requests;
    }
    function collectModelInstallWorkerSecretChanges() {
        const changes = Array.from(workerChangedSecretKeys, key => ({
            key,
            bytes: workerSecrets.has(key) ? Uint8Array.from(workerSecrets.get(key)) : null,
        }));
        workerChangedSecretKeys.clear();
        return changes;
    }
    function applyModelInstallWorkerSecretChanges(changes) {
        for (const change of changes) {
            const storageKey = `${secretPrefix}${change.key}`;
            if (change.bytes === null) {
                localStorage.removeItem(storageKey);
            }
            else {
                localStorage.setItem(storageKey, bytesToBase64(change.bytes));
            }
        }
    }
    function nowIso() {
        return new Date().toISOString();
    }
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
    async function ensureBrowserStorage() {
        if (isRuntimeWorker()) {
            const initialize = runtimeGlobal.__operitRuntimeWorkerEnsureStorage;
            if (initialize === undefined) {
                throw new Error("runtime worker OPFS storage is not installed");
            }
            await initialize();
            return;
        }
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
                            storageCache.set(String(cursor.key), new Uint8Array(cursor.value));
                            cursor.continue();
                        }
                    };
                    request.onerror = () => reject(request.error || new Error("indexedDB cursor failed"));
                    transaction.oncomplete = () => resolve();
                    transaction.onerror = () => reject(transaction.error || new Error("indexedDB read failed"));
                });
            })();
        }
        return storageReadyPromise;
    }
    async function persistStorageEntry(itemKey, bytes) {
        const database = await openStorageDatabase();
        await new Promise((resolve, reject) => {
            const transaction = database.transaction(storageObjectStoreName, "readwrite");
            transaction.objectStore(storageObjectStoreName).put(new Uint8Array(bytes), itemKey);
            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error || new Error("indexedDB write failed"));
        });
    }
    async function removeStorageEntry(itemKey) {
        const database = await openStorageDatabase();
        await new Promise((resolve, reject) => {
            const transaction = database.transaction(storageObjectStoreName, "readwrite");
            transaction.objectStore(storageObjectStoreName).delete(itemKey);
            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error || new Error("indexedDB delete failed"));
        });
    }
    async function persistModelInstallStorageChanges(changes) {
        const database = await openStorageDatabase();
        await new Promise((resolve, reject) => {
            const transaction = database.transaction(storageObjectStoreName, "readwrite");
            const store = transaction.objectStore(storageObjectStoreName);
            for (const change of changes) {
                if (change.bytes === null) {
                    store.delete(change.key);
                }
                else {
                    store.put(change.bytes, change.key);
                }
            }
            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error || new Error("model installation storage commit failed"));
        });
    }
    function openHttpDownloadDatabase() {
        if (!httpDownloadDatabasePromise) {
            httpDownloadDatabasePromise = new Promise((resolve, reject) => {
                const request = indexedDB.open(httpDownloadDatabaseName, 2);
                request.onupgradeneeded = event => {
                    if (event.oldVersion === 0) {
                        request.result.createObjectStore(httpDownloadObjectStoreName);
                        return;
                    }
                    if (event.oldVersion !== 1 || request.transaction === null) {
                        throw new Error(`unsupported HTTP download database version: ${event.oldVersion}`);
                    }
                    const store = request.transaction.objectStore(httpDownloadObjectStoreName);
                    const cursorRequest = store.openCursor();
                    cursorRequest.onsuccess = () => {
                        const cursor = cursorRequest.result;
                        if (cursor === null) {
                            return;
                        }
                        const download = cursor.value;
                        cursor.update({ ...download, paused: false });
                        cursor.continue();
                    };
                };
                request.onsuccess = () => resolve(request.result);
                request.onerror = () => reject(request.error || new Error("HTTP download database open failed"));
            });
        }
        return httpDownloadDatabasePromise;
    }
    async function readHttpDownload(url) {
        const database = await openHttpDownloadDatabase();
        return new Promise((resolve, reject) => {
            const transaction = database.transaction(httpDownloadObjectStoreName, "readonly");
            const request = transaction.objectStore(httpDownloadObjectStoreName).get(url);
            request.onsuccess = () => resolve(request.result || null);
            request.onerror = () => reject(request.error || new Error("HTTP download read failed"));
        });
    }
    function httpDownloadStatus(download) {
        return {
            url: download.url,
            fileId: download.fileId,
            expectedBytes: download.expectedBytes,
            downloadedBytes: download.downloadedBytes,
            active: false,
            modelId: download.modelId,
            version: download.version,
            paused: download.paused,
        };
    }
    function ensureHttpDownloadStatusCache() {
        if (httpDownloadStatusCachePromise === null) {
            httpDownloadStatusCachePromise = (async () => {
                const database = await openHttpDownloadDatabase();
                const downloads = await new Promise((resolve, reject) => {
                    const transaction = database.transaction(httpDownloadObjectStoreName, "readonly");
                    const request = transaction.objectStore(httpDownloadObjectStoreName).getAll();
                    request.onsuccess = () => resolve(request.result);
                    request.onerror = () => reject(request.error || new Error("HTTP download status cache load failed"));
                });
                for (const download of downloads) {
                    httpDownloadStatusCache.set(download.url, httpDownloadStatus(download));
                }
            })();
        }
        return httpDownloadStatusCachePromise;
    }
    async function writeHttpDownload(download) {
        await ensureHttpDownloadStatusCache();
        const database = await openHttpDownloadDatabase();
        await new Promise((resolve, reject) => {
            const transaction = database.transaction(httpDownloadObjectStoreName, "readwrite");
            transaction.objectStore(httpDownloadObjectStoreName).put(download, download.url);
            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error || new Error("HTTP download write failed"));
        });
        httpDownloadStatusCache.set(download.url, httpDownloadStatus(download));
    }
    async function deleteHttpDownload(url) {
        await stopHttpDownload(url);
        await ensureHttpDownloadStatusCache();
        const database = await openHttpDownloadDatabase();
        await new Promise((resolve, reject) => {
            const transaction = database.transaction(httpDownloadObjectStoreName, "readwrite");
            transaction.objectStore(httpDownloadObjectStoreName).delete(url);
            transaction.oncomplete = () => resolve();
            transaction.onerror = () => reject(transaction.error || new Error("HTTP download delete failed"));
        });
        httpDownloadStatusCache.delete(url);
    }
    async function listHttpDownloads() {
        await ensureHttpDownloadStatusCache();
        return Array.from(httpDownloadStatusCache.values(), download => ({
            ...download,
            active: activeHttpDownloadPromises.has(download.url),
        }));
    }
    function pauseHttpDownload(url) {
        activeHttpDownloadControllers.get(url)?.abort();
    }
    async function stopHttpDownload(url) {
        const active = activeHttpDownloadPromises.get(url);
        pauseHttpDownload(url);
        if (active !== undefined) {
            await active.then(() => { }, () => { });
        }
    }
    function localModelTaskKey(identity) {
        return `${identity.modelId}@${identity.version}`;
    }
    function startModelInstallTask(taskKey) {
        modelInstallTaskGeneration += 1;
        modelInstallTaskGenerations.set(taskKey, modelInstallTaskGeneration);
        return modelInstallTaskGeneration;
    }
    function invalidateModelInstallTask(taskKey) {
        modelInstallTaskGeneration += 1;
        modelInstallTaskGenerations.set(taskKey, modelInstallTaskGeneration);
        activeModelInstallAborters.get(taskKey)?.();
        activeModelInstallPromises.delete(taskKey);
    }
    function isCurrentModelInstallTask(taskKey, generation) {
        return modelInstallTaskGenerations.get(taskKey) === generation;
    }
    function persistStorageWrite(itemKey, bytes) {
        if (isModelInstallWorker()) {
            return;
        }
        void persistStorageEntry(itemKey, bytes);
    }
    function persistStorageDelete(itemKey) {
        if (isModelInstallWorker()) {
            return;
        }
        void removeStorageEntry(itemKey);
    }
    function recordWorkerStorageChange(itemKey) {
        if (isModelInstallWorker()) {
            workerChangedStorageKeys.add(itemKey);
        }
    }
    async function collectWorkerStorageChanges() {
        const changes = Array.from(workerChangedStorageKeys, itemKey => ({
            key: itemKey,
            bytes: storageCache.has(itemKey) ? storageCache.get(itemKey) : null,
        }));
        workerChangedStorageKeys.clear();
        return changes;
    }
    function storageRead(prefix, path) {
        if (isRuntimeWorker()) {
            const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
            if (storage === undefined) {
                throw new Error("runtime worker OPFS storage is not installed");
            }
            const address = workerStorageAddress(prefix, path);
            return storage.read(address.prefix, address.path);
        }
        return storageCache.get(key(prefix, path)) || new Uint8Array();
    }
    function storageReadRange(prefix, path, offset, length) {
        if (!isRuntimeWorker()) {
            throw new Error("bounded runtime storage reads are owned by the runtime worker");
        }
        const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
        if (storage === undefined) {
            throw new Error("runtime worker OPFS storage is not installed");
        }
        const address = workerStorageAddress(prefix, path);
        return storage.readRange(address.prefix, address.path, offset, length);
    }
    function storageWrite(prefix, path, content) {
        if (isRuntimeWorker()) {
            const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
            if (storage === undefined) {
                throw new Error("runtime worker OPFS storage is not installed");
            }
            const address = workerStorageAddress(prefix, path);
            storage.write(address.prefix, address.path, new Uint8Array(content));
            return;
        }
        const itemKey = key(prefix, path);
        const bytes = new Uint8Array(content);
        storageCache.set(itemKey, bytes);
        persistStorageWrite(itemKey, bytes);
        recordWorkerStorageChange(itemKey);
        if (!isModelInstallWorker() && isLocalModelRegistryPath(prefix, path)) {
            scheduleWebLocalInferenceRefresh();
        }
    }
    function storageAppend(prefix, path, content) {
        if (isRuntimeWorker()) {
            const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
            if (storage === undefined) {
                throw new Error("runtime worker OPFS storage is not installed");
            }
            const address = workerStorageAddress(prefix, path);
            storage.append(address.prefix, address.path, new Uint8Array(content));
            return;
        }
        const itemKey = key(prefix, path);
        const previous = storageCache.get(itemKey) || new Uint8Array();
        const appended = new Uint8Array(previous.byteLength + content.byteLength);
        appended.set(previous);
        appended.set(new Uint8Array(content), previous.byteLength);
        storageWrite(prefix, path, appended);
    }
    function storageHasFile(prefix, path) {
        if (isRuntimeWorker()) {
            const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
            if (storage === undefined) {
                throw new Error("runtime worker OPFS storage is not installed");
            }
            const address = workerStorageAddress(prefix, path);
            return storage.hasFile(address.prefix, address.path);
        }
        return storageCache.has(key(prefix, path));
    }
    function storageExists(prefix, path) {
        if (isRuntimeWorker()) {
            const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
            if (storage === undefined) {
                throw new Error("runtime worker OPFS storage is not installed");
            }
            const address = workerStorageAddress(prefix, path);
            return storage.exists(address.prefix, address.path);
        }
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
        if (isRuntimeWorker()) {
            const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
            if (storage === undefined) {
                throw new Error("runtime worker OPFS storage is not installed");
            }
            const address = workerStorageAddress(prefix, path);
            storage.delete(address.prefix, address.path, recursive);
            return;
        }
        const exact = key(prefix, path);
        const directory = exact.endsWith("/") ? exact : exact + "/";
        storageCache.delete(exact);
        persistStorageDelete(exact);
        recordWorkerStorageChange(exact);
        if (recursive) {
            const keys = [];
            for (const itemKey of storageCache.keys()) {
                if (itemKey.startsWith(directory)) {
                    keys.push(itemKey);
                }
            }
            for (const itemKey of keys) {
                storageCache.delete(itemKey);
                persistStorageDelete(itemKey);
                recordWorkerStorageChange(itemKey);
            }
        }
        if (!isModelInstallWorker() && isLocalModelRegistryPath(prefix, path)) {
            scheduleWebLocalInferenceRefresh();
        }
    }
    function isLocalModelRegistryPath(prefix, path) {
        return prefix === runtimePrefix &&
            normalizeRuntimePath(path) ===
                "runtime/config/preferences/local_model_registry.preferences.json";
    }
    function storageList(prefix, path) {
        if (isRuntimeWorker()) {
            const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
            if (storage === undefined) {
                throw new Error("runtime worker OPFS storage is not installed");
            }
            const address = workerStorageAddress(prefix, path);
            return storage.list(address.prefix, address.path);
        }
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
    function workerStorageAddress(prefix, path) {
        if (prefix !== filePrefix) {
            return { prefix, path };
        }
        const normalized = normalizeRuntimePath(path);
        const root = normalized.split("/", 1)[0];
        if (root === "runtime" || root === "workspaces" || root === "secure") {
            return { prefix: runtimePrefix, path: normalized };
        }
        return { prefix, path: normalized };
    }
    function fileDirectoryKey(path) {
        const normalized = normalizeRuntimePath(path);
        return normalized.length === 0 ? filePrefix : `${filePrefix}${normalized}/`;
    }
    function fileDirectoryExists(path) {
        const directory = fileDirectoryKey(path);
        if (fileDirectories.has(directory)) {
            return true;
        }
        if (isRuntimeWorker()) {
            return storageExists(filePrefix, path);
        }
        for (const itemKey of storageCache.keys()) {
            if (itemKey.startsWith(directory)) {
                return true;
            }
        }
        return false;
    }
    function makeFileDirectory(path, createParents) {
        const normalized = normalizeRuntimePath(path);
        if (normalized.length === 0) {
            return;
        }
        const segments = normalized.split("/");
        if (!createParents) {
            const parent = segments.slice(0, -1).join("/");
            if (parent.length > 0 && !fileDirectoryExists(parent)) {
                throw new Error(`parent directory does not exist: ${parent}`);
            }
            fileDirectories.add(fileDirectoryKey(normalized));
            return;
        }
        for (let index = 1; index <= segments.length; index += 1) {
            fileDirectories.add(fileDirectoryKey(segments.slice(0, index).join("/")));
        }
    }
    function listFileDirectory(path) {
        const directory = fileDirectoryKey(path);
        const entries = new Map();
        for (const candidate of fileDirectories) {
            if (!candidate.startsWith(directory) || candidate === directory) {
                continue;
            }
            const relative = candidate.slice(directory.length).replace(/\/$/, "");
            const separator = relative.indexOf("/");
            const name = separator < 0 ? relative : relative.slice(0, separator);
            entries.set(name, { path: name, isDirectory: true, size: 0 });
        }
        if (isRuntimeWorker()) {
            for (const entry of storageList(filePrefix, path)) {
                const relative = entry.path.slice(normalizeRuntimePath(path).length).replace(/^\/+/, "");
                const separator = relative.indexOf("/");
                const name = separator < 0 ? relative : relative.slice(0, separator);
                if (name.length === 0) {
                    continue;
                }
                entries.set(name, {
                    path: name,
                    isDirectory: entry.isDirectory || separator >= 0,
                    size: entry.isDirectory || separator >= 0 ? 0 : entry.size,
                });
            }
        }
        else {
            for (const [itemKey, bytes] of storageCache.entries()) {
                if (!itemKey.startsWith(directory)) {
                    continue;
                }
                const relative = itemKey.slice(directory.length);
                const separator = relative.indexOf("/");
                const name = separator < 0 ? relative : relative.slice(0, separator);
                if (separator < 0) {
                    entries.set(name, { path: name, isDirectory: false, size: bytes.length });
                }
                else {
                    entries.set(name, { path: name, isDirectory: true, size: 0 });
                }
            }
        }
        return Array.from(entries.values());
    }
    function copyFileSystemEntry(source, destination, recursive) {
        const normalizedSource = normalizeRuntimePath(source);
        const normalizedDestination = normalizeRuntimePath(destination);
        const sourceIsFile = storageHasFile(filePrefix, normalizedSource);
        const sourceIsDirectory = !sourceIsFile && fileDirectoryExists(normalizedSource);
        if (!sourceIsFile && !sourceIsDirectory) {
            throw new Error(`Source path does not exist: ${source}`);
        }
        if (sourceIsFile) {
            const separator = normalizedDestination.lastIndexOf("/");
            if (separator >= 0) {
                makeFileDirectory(normalizedDestination.slice(0, separator), true);
            }
            storageWrite(filePrefix, normalizedDestination, storageRead(filePrefix, normalizedSource));
            return;
        }
        if (!recursive) {
            throw new Error("Source is a directory and recursive flag is not set");
        }
        makeFileDirectory(normalizedDestination, true);
        for (const entry of listFileDirectory(normalizedSource)) {
            copyFileSystemEntry(`${normalizedSource}/${entry.path}`, `${normalizedDestination}/${entry.path}`, true);
        }
    }
    function deleteFileDirectory(path, recursive) {
        const directory = fileDirectoryKey(path);
        fileDirectories.delete(directory);
        if (!recursive) {
            return;
        }
        for (const candidate of Array.from(fileDirectories)) {
            if (candidate.startsWith(directory)) {
                fileDirectories.delete(candidate);
            }
        }
    }
    async function loadScript(src) {
        if (isModelInstallWorker() || isRuntimeWorker()) {
            const response = await fetch(src);
            if (!response.ok) {
                throw new Error(`failed to load ${src}: HTTP ${response.status}`);
            }
            const source = await response.text();
            const execute = new Function(`${source}\nglobalThis.initSqlJs = initSqlJs;`);
            execute();
            return;
        }
        return new Promise((resolve, reject) => {
            const existing = document.querySelector(`script[src="${src}"]`);
            if (existing) {
                existing.addEventListener("load", () => resolve(), { once: true });
                existing.addEventListener("error", () => reject(new Error(`failed to load ${src}`)), { once: true });
                return;
            }
            const script = document.createElement("script");
            script.src = src;
            script.onload = () => resolve();
            script.onerror = () => reject(new Error(`failed to load ${src}`));
            document.head.appendChild(script);
        });
    }
    async function ensureSqlite() {
        if (!sqliteModulePromise) {
            sqliteModulePromise = (async () => {
                await loadScript("sql-wasm.js");
                const initializeSqlJs = runtimeGlobal.initSqlJs;
                if (initializeSqlJs === undefined) {
                    throw new Error("sql.js initializer is not loaded");
                }
                SQLite = await initializeSqlJs({
                    locateFile(file) {
                        return file;
                    },
                });
            })();
        }
        await sqliteModulePromise;
    }
    function sqliteKey(path) {
        return key(runtimePrefix, path);
    }
    function saveSqliteDatabase(connection) {
        storageWrite(runtimePrefix, connection.path, connection.db.export());
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
        throw new Error("unknown sqlite value kind");
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
        }
        finally {
            statement.free();
        }
        return rows;
    }
    function fileInfo(path) {
        const isFile = storageHasFile(filePrefix, path);
        const isDirectory = !isFile && fileDirectoryExists(path);
        const exists = isFile || isDirectory;
        const bytes = isFile ? storageRead(filePrefix, path) : new Uint8Array();
        return {
            path,
            exists,
            fileType: isFile ? "file" : isDirectory ? "directory" : "missing",
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
            const voice = synthesis().getVoices().find((candidate) => candidate.voiceURI === voiceName || candidate.name === voiceName);
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
                activeAudioUrl = URL.createObjectURL(new Blob([blobPart(bytes)], { type: audioContentType(audioPath) }));
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
                }
                else {
                    synthesis().pause();
                }
                lastDetails = "browser speech synthesis paused";
                return currentStatus(lastDetails);
            },
            resumeSpeech() {
                if (activeAudio !== null) {
                    void activeAudio.play();
                    activeAudioPaused = false;
                }
                else {
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
            const api = browserNavigator.bluetooth;
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
                    supported: !!browserNavigator.bluetooth,
                    enabled: !!browserNavigator.bluetooth,
                    state: browserNavigator.bluetooth ? "available" : "unavailable",
                };
            },
            requestEnableBluetooth() {
                browserBluetooth();
                return "browser_bluetooth_enable_controlled_by_system";
            },
            listBluetoothBondedDevices() {
                return {
                    devices: [],
                };
            },
            scanBluetoothDevices(request) {
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
            bluetoothBleConnect() {
                return browserBluetooth().requestDevice({ acceptAllDevices: true }).then((device) => device.gatt.connect().then((server) => {
                    const sessionId = `web-ble-${createRandomUuid()}`;
                    bleSessions.set(sessionId, { device, server, characteristics: new Map() });
                    notifications.set(sessionId, []);
                    return { sessionId, address: device.id, mode: "ble" };
                }));
            },
            bluetoothBleDiscoverServices(sessionId) {
                const value = session(sessionId);
                return value.server.getPrimaryServices().then((services) => Promise.all(services.map((service) => service.getCharacteristics().then((characteristics) => {
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
                }))).then((items) => ({ sessionId, services: items })));
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
                    .then(() => characteristic(request.sessionId, request.readServiceUuid, request.readCharacteristicUuid).readValue())
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
        if (properties.read)
            names.push("read");
        if (properties.write)
            names.push("write");
        if (properties.writeWithoutResponse)
            names.push("write_without_response");
        if (properties.notify)
            names.push("notify");
        if (properties.indicate)
            names.push("indicate");
        return names;
    }
    function scheduleWebLocalInferenceRefresh() {
        webLocalInferenceReadyPromise = null;
        queueMicrotask(() => {
            void ensureWebLocalInference().catch((error) => {
                console.warn("[Operit local inference]", error);
            });
        });
    }
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
                }
                catch (error) {
                    disposeWebLocalInferenceState(state);
                    throw error;
                }
                disposeWebLocalInferenceState(webLocalInferenceState);
                webLocalInferenceState = state;
                runtimeGlobal.__operitLocalInference = {
                    transcribeLocalSpeech: transcribeWebLocalSpeech,
                    synthesizeLocalSpeech: synthesizeWebLocalSpeech,
                };
            })();
        }
        return webLocalInferenceReadyPromise;
    }
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
    function runtimePathsExist(paths) {
        return paths.every((path) => storageExists(runtimePrefix, path));
    }
    function runtimeBlobUrl(path, contentType, state) {
        const bytes = storageRead(runtimePrefix, path);
        if (bytes.length === 0) {
            throw new Error(`runtime file is empty or missing: ${path}`);
        }
        const url = URL.createObjectURL(new Blob([blobPart(bytes)], { type: contentType }));
        state.blobUrls.push(url);
        return url;
    }
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
    function loadClassicScriptUrl(src) {
        return new Promise((resolve, reject) => {
            const script = document.createElement("script");
            script.src = src;
            script.onload = () => resolve();
            script.onerror = () => reject(new Error(`failed to load ${src}`));
            document.head.appendChild(script);
        });
    }
    async function createWebAsrBundle(paths, state) {
        requireCrossOriginIsolation("ASR");
        const urls = {
            recognizerScript: runtimeJavaScriptUrl(paths.recognizerScript, "globalThis.__operitSherpaAsrClasses = { OfflineRecognizer };", state),
            runtimeScript: runtimeBlobUrl(paths.runtimeScript, "text/javascript", state),
            runtimeWasm: runtimeBlobUrl(paths.runtimeWasm, "application/wasm", state),
            runtimeData: runtimeBlobUrl(paths.runtimeData, "application/octet-stream", state),
        };
        const moduleValue = {};
        const ready = new Promise((resolve, reject) => {
            moduleValue.mainScriptUrlOrBlob = urls.runtimeScript;
            moduleValue.locateFile = (path) => {
                if (path === "sherpa-onnx-wasm-main-vad-asr.wasm")
                    return urls.runtimeWasm;
                if (path === "sherpa-onnx-wasm-main-vad-asr.data")
                    return urls.runtimeData;
                return path;
            };
            moduleValue.setStatus = (status) => console.debug("[Operit ASR]", status);
            moduleValue.onRuntimeInitialized = () => resolve();
            moduleValue.onAbort = (reason) => reject(new Error(reason));
        });
        runtimeGlobal.Module = moduleValue;
        await loadClassicScriptUrl(urls.runtimeScript);
        await ready;
        await loadClassicScriptUrl(urls.recognizerScript);
        const classes = runtimeGlobal.__operitSherpaAsrClasses;
        if (!classes || typeof classes.OfflineRecognizer !== "function") {
            throw new Error("Web ASR recognizer class was not exported");
        }
        const recognizer = new classes.OfflineRecognizer(webAsrConfig(), moduleValue);
        return { recognizer, moduleValue };
    }
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
    function generateWebTtsWav(bundle, text, speaker, speed) {
        const requestId = createRandomUuid();
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
    function requireCrossOriginIsolation(capability) {
        if (globalThis.crossOriginIsolated !== true) {
            throw new Error(`Web local ${capability} requires Cross-Origin-Opener-Policy: same-origin and ` +
                "Cross-Origin-Embedder-Policy: require-corp");
        }
    }
    function transcribeWebLocalSpeech(requestJson) {
        const state = requireWebLocalInferenceState();
        const request = JSON.parse(requestJson);
        const driver = parseTaggedDriver(request.driverJson, "SherpaOnnxWebAsrBundle");
        const root = runtimeDirectoryForDriver(request.modelDirectory, driver.recognizerScript);
        const bundle = state.asrBundles.get(root);
        if (!bundle) {
            throw new Error(`Web ASR bundle is not initialized: ${root}`);
        }
        const wav = decodeMonoPcmWav(storageRead(filePrefix, request.audioPath));
        const stream = bundle.recognizer.createStream();
        try {
            stream.acceptWaveform(wav.sampleRate, wav.samples);
            bundle.recognizer.decode(stream);
            const result = bundle.recognizer.getResult(stream);
            return JSON.stringify({
                text: result.text || "",
                resultJson: JSON.stringify(result),
            });
        }
        finally {
            stream.free();
        }
    }
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
            throw new Error(`Web TTS speaker count mismatch: manifest=${driver.speakerCount}, ` +
                `engine=${bundle.numSpeakers}`);
        }
        const wav = generateWebTtsWav(bundle, String(request.text), speaker, Number(request.speed));
        storageWrite(runtimePrefix, request.outputPath, wav);
        return JSON.stringify({
            audioPath: request.outputPath,
            outputFormat: "wav",
        });
    }
    function requireWebLocalInferenceState() {
        if (!webLocalInferenceState) {
            throw new Error("Web local inference runner is still initializing");
        }
        return webLocalInferenceState;
    }
    function parseTaggedDriver(driverJson, expectedTag) {
        const root = JSON.parse(driverJson);
        const keys = Object.keys(root);
        if (keys.length !== 1 || keys[0] !== expectedTag) {
            throw new Error(`Web local inference driver must be ${expectedTag}`);
        }
        return root[expectedTag];
    }
    function runtimeDirectoryForDriver(modelDirectory, relativeFilePath) {
        const directory = normalizeRuntimePath(modelDirectory);
        const filePath = normalizeRuntimePath(relativeFilePath);
        const slash = filePath.lastIndexOf("/");
        if (slash < 0) {
            return directory;
        }
        return normalizeRuntimePath(`${directory}/${filePath.slice(0, slash)}`);
    }
    function normalizeRuntimePath(path) {
        return String(path)
            .replace(/\\/g, "/")
            .split("/")
            .filter((segment) => segment.length > 0 && segment !== ".")
            .join("/");
    }
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
            }
            else if (chunkId === 0x61746164) {
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
    function localInferenceRunner(method) {
        const runner = runtimeGlobal.__operitLocalInference;
        if (!runner || typeof runner[method] !== "function") {
            throw new Error(`web local inference method is not installed: ${method}`);
        }
        return function runLocalInference(requestJson) {
            const responseJson = runner[method](requestJson);
            if (typeof responseJson !== "string") {
                throw new Error(`web local inference method returned non-string JSON: ${method}`);
            }
            return responseJson;
        };
    }
    function v86AssetUrl(name) {
        return new URL(`./v86/${name}`, import.meta.url).href;
    }
    function v86RuntimeAssetUrl(name) {
        return new URL(name, "https://models.operit.app/v86-runtime/i686-buildroot-node20-python312-20260720/").href;
    }
    function validateManagedRuntimeRequest(request) {
        if (!Array.isArray(request.args) || !request.args.every(argument => typeof argument === "string")) {
            throw new Error("managed runtime request arguments must be strings");
        }
        if (typeof request.env !== "object" || request.env === null || Array.isArray(request.env)) {
            throw new Error("managed runtime request environment is invalid");
        }
        for (const [name, value] of Object.entries(request.env)) {
            if (typeof name !== "string" || typeof value !== "string") {
                throw new Error("managed runtime request environment must contain string values");
            }
        }
    }
    function guestRuntimeExecutable(program) {
        switch (program) {
            case "node":
                return { program: "node", executable: "/usr/local/bin/node" };
            case "python":
                return { program: "python3", executable: "/usr/local/bin/python3" };
            default:
                throw new Error(`unsupported V86 managed runtime program: ${program}`);
        }
    }
    function guestWorkspacePath(path) {
        const segments = String(path || "")
            .replace(/\\/g, "/")
            .split("/")
            .filter(segment => segment.length > 0 && segment !== ".");
        if (segments.some(segment => segment === "..")) {
            throw new Error("managed runtime workspace path escapes the guest workspace");
        }
        return segments.length === 0 ? "." : segments.join("/");
    }
    function managedRuntimeWorkspaceFrames(workingDirectory) {
        const prefix = workingDirectory === "." ? "" : `${workingDirectory}/`;
        const frames = [];
        for (const [storageKey, bytes] of storageCache.entries()) {
            if (!storageKey.startsWith(filePrefix)) {
                continue;
            }
            const filePath = guestWorkspacePath(storageKey.slice(filePrefix.length));
            if (prefix.length > 0 && filePath !== workingDirectory && !filePath.startsWith(prefix)) {
                continue;
            }
            frames.push(JSON.stringify({
                kind: "file",
                path: filePath,
                base64: bytesToBase64(bytes),
            }));
        }
        return frames;
    }
    function managedRuntimeEnvironment(environment, hostWorkingDirectory, guestWorkingDirectory) {
        const result = {};
        const hostRoot = String(hostWorkingDirectory || "").replace(/\\/g, "/").replace(/\/+$/, "");
        const guestRoot = guestWorkingDirectory === "."
            ? "/workspace"
            : `/workspace/${guestWorkingDirectory}`;
        for (const [name, value] of Object.entries(environment)) {
            const normalizedValue = value.replace(/\\/g, "/");
            if (hostRoot.length > 0 && normalizedValue === hostRoot) {
                result[name] = guestRoot;
            }
            else if (hostRoot.length > 0 && normalizedValue.startsWith(`${hostRoot}/`)) {
                result[name] = `${guestRoot}/${normalizedValue.slice(hostRoot.length + 1)}`;
            }
            else {
                result[name] = value;
            }
        }
        return result;
    }
    function postManagedRuntimeWorkerMessage(worker, message) {
        worker.postMessage(message);
    }
    function startManagedRuntimeProcess(request) {
        requireCrossOriginIsolation("managed Node/Python runtime");
        validateManagedRuntimeRequest(request);
        const runtime = guestRuntimeExecutable(request.program);
        const executablePath = request.executablePath?.trim();
        if (executablePath && executablePath !== runtime.executable) {
            throw new Error(`V86 managed runtime cannot execute host path: ${executablePath}`);
        }
        const workingDirectory = guestWorkspacePath(request.cwd);
        const startupFrames = managedRuntimeWorkspaceFrames(workingDirectory);
        startupFrames.push(JSON.stringify({
            kind: "start",
            program: runtime.program,
            arguments: request.args,
            environment: managedRuntimeEnvironment(request.env, request.cwd, workingDirectory),
            workingDirectory,
        }));
        const serialBuffer = new SharedArrayBuffer(managedRuntimeHeaderLength * Int32Array.BYTES_PER_ELEMENT + managedRuntimeOutputCapacity);
        const header = new Int32Array(serialBuffer, 0, managedRuntimeHeaderLength);
        const output = new Uint8Array(serialBuffer, managedRuntimeHeaderLength * Int32Array.BYTES_PER_ELEMENT);
        const id = `v86-runtime-${++managedRuntimeProcessIndex}`;
        const worker = new Worker(new URL("./v86_runtime_worker.js", import.meta.url), {
            type: "module",
            name: id,
        });
        const process = {
            id,
            worker,
            header,
            output,
            decoder: new TextDecoder(),
            activityVersion: 0,
            activityWaiters: new Set(),
            stdout: "",
            stderr: "",
        };
        worker.addEventListener("message", event => {
            if (event.data?.type !== "activity") {
                return;
            }
            process.activityVersion += 1;
            for (const resolve of process.activityWaiters) {
                resolve();
            }
            process.activityWaiters.clear();
        });
        worker.addEventListener("error", event => {
            process.stderr += `[V86 runtime worker failed: ${event.message}]\n`;
            Atomics.store(process.header, managedRuntimeStateIndex, managedRuntimeFailed);
            process.activityVersion += 1;
            for (const resolve of process.activityWaiters) {
                resolve();
            }
            process.activityWaiters.clear();
        });
        managedRuntimeProcesses.set(id, process);
        postManagedRuntimeWorkerMessage(worker, {
            type: "boot",
            serialBuffer,
            outputCapacity: managedRuntimeOutputCapacity,
            startupFrames,
        });
        return id;
    }
    function managedRuntimeProcess(id) {
        const process = managedRuntimeProcesses.get(id);
        if (process === undefined) {
            throw new Error(`managed runtime process does not exist: ${id}`);
        }
        return process;
    }
    function drainManagedRuntimeOutputBytes(process) {
        const writeIndex = Atomics.load(process.header, managedRuntimeOutputWriteIndex);
        const readIndex = Atomics.load(process.header, managedRuntimeOutputReadIndex);
        const count = writeIndex - readIndex;
        if (count === 0) {
            return new Uint8Array();
        }
        if (count < 0 || count > managedRuntimeOutputCapacity) {
            throw new Error("V86 managed runtime serial ring is corrupt");
        }
        const bytes = new Uint8Array(count);
        const firstLength = Math.min(count, managedRuntimeOutputCapacity - (readIndex % managedRuntimeOutputCapacity));
        bytes.set(process.output.subarray(readIndex % managedRuntimeOutputCapacity, (readIndex % managedRuntimeOutputCapacity) + firstLength));
        if (firstLength < count) {
            bytes.set(process.output.subarray(0, count - firstLength), firstLength);
        }
        Atomics.store(process.header, managedRuntimeOutputReadIndex, readIndex + count);
        return bytes;
    }
    function refreshManagedRuntimeStdout(process) {
        const bytes = drainManagedRuntimeOutputBytes(process);
        if (bytes.length > 0) {
            process.stdout += process.decoder.decode(bytes, { stream: true });
        }
    }
    function managedRuntimeState(process) {
        return Atomics.load(process.header, managedRuntimeStateIndex);
    }
    async function waitForManagedRuntimeChange(process, deadline) {
        const remainingMs = Math.ceil(deadline - performance.now());
        if (remainingMs <= 0) {
            return;
        }
        const observedVersion = process.activityVersion;
        await new Promise(resolve => {
            const timer = setTimeout(() => {
                process.activityWaiters.delete(complete);
                resolve();
            }, remainingMs);
            const complete = () => {
                clearTimeout(timer);
                process.activityWaiters.delete(complete);
                resolve();
            };
            process.activityWaiters.add(complete);
            if (process.activityVersion !== observedVersion) {
                complete();
            }
        });
    }
    async function readManagedRuntimeStdoutLine(id, timeoutMs) {
        const process = managedRuntimeProcess(id);
        const deadline = performance.now() + Math.max(0, timeoutMs);
        for (;;) {
            refreshManagedRuntimeStdout(process);
            const newlineIndex = process.stdout.indexOf("\n");
            if (newlineIndex >= 0) {
                const line = process.stdout.slice(0, newlineIndex).replace(/\r$/, "");
                process.stdout = process.stdout.slice(newlineIndex + 1);
                return line;
            }
            const state = managedRuntimeState(process);
            if (state === managedRuntimeFailed) {
                process.stderr += process.stdout;
                process.stdout = "";
                return null;
            }
            if (state === managedRuntimeStopped || performance.now() >= deadline) {
                return null;
            }
            await waitForManagedRuntimeChange(process, deadline);
        }
    }
    function writeManagedRuntimeLines(id, lines) {
        const process = managedRuntimeProcess(id);
        if (!Array.isArray(lines) || !lines.every(line => typeof line === "string")) {
            throw new Error("managed runtime input lines must be strings");
        }
        const state = managedRuntimeState(process);
        if (state === managedRuntimeFailed || state === managedRuntimeStopped) {
            throw new Error(`managed runtime process is not running: ${id}`);
        }
        postManagedRuntimeWorkerMessage(process.worker, { type: "input", lines });
    }
    function drainManagedRuntimeStderr(id) {
        const process = managedRuntimeProcess(id);
        if (managedRuntimeState(process) === managedRuntimeFailed) {
            refreshManagedRuntimeStdout(process);
            process.stderr += process.stdout;
            process.stdout = "";
        }
        const output = process.stderr;
        process.stderr = "";
        return output;
    }
    function isManagedRuntimeRunning(id) {
        const state = managedRuntimeState(managedRuntimeProcess(id));
        return state === managedRuntimeStarting || state === managedRuntimeRunning;
    }
    function killManagedRuntimeProcess(id) {
        const process = managedRuntimeProcess(id);
        const state = managedRuntimeState(process);
        if (state === managedRuntimeFailed || state === managedRuntimeStopped) {
            return;
        }
        Atomics.store(process.header, managedRuntimeStateIndex, managedRuntimeStopped);
        postManagedRuntimeWorkerMessage(process.worker, { type: "kill" });
    }
    async function runManagedRuntimeCommand(request) {
        const id = startManagedRuntimeProcess(request);
        const process = managedRuntimeProcess(id);
        const deadline = performance.now() + managedRuntimeCommandTimeoutMs;
        for (;;) {
            refreshManagedRuntimeStdout(process);
            const state = managedRuntimeState(process);
            if (state === managedRuntimeStopped) {
                process.stdout += process.decoder.decode();
                const exitCode = Atomics.load(process.header, managedRuntimeExitCodeIndex);
                const result = {
                    exitCode: exitCode >= 0 ? exitCode : null,
                    stdout: process.stdout,
                    stderr: drainManagedRuntimeStderr(id),
                };
                managedRuntimeProcesses.delete(id);
                process.worker.terminate();
                return result;
            }
            if (state === managedRuntimeFailed) {
                const stderr = drainManagedRuntimeStderr(id);
                managedRuntimeProcesses.delete(id);
                process.worker.terminate();
                throw new Error(stderr.length > 0 ? stderr : "V86 managed runtime failed");
            }
            if (performance.now() >= deadline) {
                killManagedRuntimeProcess(id);
                managedRuntimeProcesses.delete(id);
                process.worker.terminate();
                throw new Error("V86 managed runtime command timed out");
            }
            await waitForManagedRuntimeChange(process, deadline);
        }
    }
    function linuxVmSession(sessionId) {
        const session = linuxVmSessions.get(sessionId);
        if (session === undefined) {
            throw new Error(`Linux VM terminal session does not exist: ${sessionId}`);
        }
        return session;
    }
    function appendLinuxVmOutput(session, bytes) {
        const requiredLength = session.outputLength + bytes.length;
        if (requiredLength > linuxVmOutputLimit) {
            failLinuxVmSession(session, new Error("Linux VM terminal output exceeded 4 MiB"));
            return;
        }
        if (requiredLength > session.output.length) {
            const nextLength = Math.min(linuxVmOutputLimit, Math.max(requiredLength, session.output.length * 2));
            const expanded = new Uint8Array(nextLength);
            expanded.set(session.output.subarray(0, session.outputLength));
            session.output = expanded;
        }
        session.output.set(bytes, session.outputLength);
        session.outputLength = requiredLength;
    }
    function appendLinuxVmCommandOutput(command, byte) {
        const requiredLength = command.outputLength + 1;
        if (requiredLength > linuxVmOutputLimit) {
            throw new Error("Linux VM command output exceeded 4 MiB");
        }
        if (requiredLength > command.output.length) {
            const expanded = new Uint8Array(Math.min(linuxVmOutputLimit, Math.max(requiredLength, command.output.length * 2)));
            expanded.set(command.output.subarray(0, command.outputLength));
            command.output = expanded;
        }
        command.output[command.outputLength] = byte;
        command.outputLength = requiredLength;
    }
    function deliverLinuxVmCommandByte(session, command, byte) {
        appendLinuxVmOutput(session, Uint8Array.of(byte));
        if (command.collectingOutput) {
            appendLinuxVmCommandOutput(command, byte);
        }
    }
    function parseLinuxVmCommandMarker(command, bytes) {
        const marker = textDecoder.decode(Uint8Array.from(bytes));
        const start = `\x1b]1337;OPERIT_COMMAND_START;${command.token}\x07`;
        if (marker === start) {
            return { kind: "start" };
        }
        const prefix = `\x1b]1337;OPERIT_COMMAND_END;${command.token};`;
        if (!marker.startsWith(prefix) || !marker.endsWith("\x07")) {
            return null;
        }
        const fields = marker.slice(prefix.length, -1).split(";");
        if (fields.length !== 2 || !/^-?\d+$/.test(fields[0]) || fields[1].length === 0) {
            return null;
        }
        const exitCode = Number.parseInt(fields[0], 10);
        if (!Number.isSafeInteger(exitCode)) {
            return null;
        }
        return {
            kind: "end",
            exitCode,
            workingDir: textDecoder.decode(base64ToBytes(fields[1])),
        };
    }
    function completeLinuxVmCommand(session, command, result) {
        if (session.activeCommand !== command) {
            return;
        }
        clearTimeout(command.timeoutHandle);
        session.activeCommand = null;
        command.resolve(result);
        startNextLinuxVmCommand(session);
    }
    function rejectLinuxVmCommands(session, reason) {
        const active = session.activeCommand;
        if (active !== null) {
            clearTimeout(active.timeoutHandle);
            session.activeCommand = null;
            active.reject(reason);
        }
        for (const command of session.pendingCommands) {
            clearTimeout(command.timeoutHandle);
            command.reject(reason);
        }
        session.pendingCommands = [];
    }
    function linuxVmCommandInput(command) {
        const encodedCommand = bytesToBase64(textEncoder.encode(command.command));
        const input = [
            `printf '\\033]1337;OPERIT_COMMAND_START;${command.token}\\007'`,
            `eval "$(printf %s ${encodedCommand} | base64 -d)"`,
            "__operit_status=$?",
            `printf '\\033]1337;OPERIT_COMMAND_END;${command.token};%s;' \"$__operit_status\"`,
            "printf %s \"$PWD\" | base64 | tr -d '\\n'",
            "printf '\\007'\r",
        ].join("; ");
        return textEncoder.encode(input);
    }
    function startNextLinuxVmCommand(session) {
        if (session.activeCommand !== null || session.state !== "running") {
            return;
        }
        const command = session.pendingCommands.shift();
        if (command === undefined) {
            return;
        }
        session.activeCommand = command;
        try {
            const emulator = session.emulator;
            if (emulator === null) {
                throw new Error(`Linux VM terminal emulator is unavailable: ${session.id}`);
            }
            emulator.serial_send_bytes(0, linuxVmCommandInput(command));
        }
        catch (error) {
            session.activeCommand = null;
            clearTimeout(command.timeoutHandle);
            command.reject(error);
            startNextLinuxVmCommand(session);
        }
    }
    function consumeLinuxVmCommandByte(session, command, byte) {
        if (command.markerBytes.length === 0 && byte !== 0x1b) {
            deliverLinuxVmCommandByte(session, command, byte);
            return;
        }
        command.markerBytes.push(byte);
        if (command.markerBytes.length === 2 && command.markerBytes[1] !== 0x5d) {
            for (const markerByte of command.markerBytes) {
                deliverLinuxVmCommandByte(session, command, markerByte);
            }
            command.markerBytes = [];
            return;
        }
        if (byte !== 0x07 && command.markerBytes.length < 4096) {
            return;
        }
        const markerBytes = command.markerBytes;
        command.markerBytes = [];
        const marker = byte === 0x07 ? parseLinuxVmCommandMarker(command, markerBytes) : null;
        if (marker === null) {
            for (const markerByte of markerBytes) {
                deliverLinuxVmCommandByte(session, command, markerByte);
            }
            return;
        }
        if (marker.kind === "start") {
            command.collectingOutput = true;
            return;
        }
        if (command.collectingOutput) {
            completeLinuxVmCommand(session, command, {
                output: textDecoder.decode(command.output.subarray(0, command.outputLength)).trim(),
                exitCode: marker.exitCode,
                timedOut: false,
                workingDir: marker.workingDir,
            });
            return;
        }
        for (const markerByte of markerBytes) {
            deliverLinuxVmCommandByte(session, command, markerByte);
        }
    }
    function executeLinuxVmCommand(sessionId, command, timeoutMs) {
        const session = linuxVmSession(sessionId);
        if (typeof command !== "string" || command.trim().length === 0) {
            throw new Error("Linux VM terminal command must not be blank");
        }
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1) {
            throw new Error("Linux VM terminal timeout must be a positive integer");
        }
        if (session.state === "failed" || session.state === "closed") {
            throw new Error(`Linux VM terminal is not running: ${sessionId}`);
        }
        return new Promise((resolve, reject) => {
            const token = String(++linuxVmCommandIndex);
            const commandState = {
                token,
                command,
                timeoutHandle: globalThis.setTimeout(() => {
                    if (session.activeCommand !== commandState) {
                        return;
                    }
                    const output = textDecoder.decode(commandState.output.subarray(0, commandState.outputLength)).trim();
                    session.emulator?.serial_send_bytes(0, Uint8Array.of(0x03));
                    completeLinuxVmCommand(session, commandState, {
                        output,
                        exitCode: -1,
                        timedOut: true,
                        workingDir: "/",
                    });
                }, timeoutMs),
                resolve,
                reject,
                collectingOutput: false,
                markerBytes: [],
                output: new Uint8Array(4096),
                outputLength: 0,
            };
            session.pendingCommands.push(commandState);
            startNextLinuxVmCommand(session);
        });
    }
    function renderLinuxVmProgress(session, message) {
        session.progressVisible = true;
        appendLinuxVmOutput(session, textEncoder.encode(`\r\x1b[2K${message}`));
    }
    function finishLinuxVmProgress(session, message) {
        if (!session.progressVisible) {
            return;
        }
        session.progressVisible = false;
        const suffix = message === null ? "" : `${message}\r\n`;
        appendLinuxVmOutput(session, textEncoder.encode(`\r\x1b[2K${suffix}`));
    }
    function linuxVmDownloadStatus(value, columns) {
        if (typeof value !== "object" || value === null) {
            return null;
        }
        const progress = value;
        const fileIndex = progress.file_index;
        const fileCount = progress.file_count;
        const loaded = progress.loaded;
        const total = progress.total;
        if (typeof fileIndex !== "number" ||
            typeof fileCount !== "number" ||
            typeof loaded !== "number" ||
            typeof total !== "number" ||
            !Number.isInteger(fileIndex) ||
            !Number.isInteger(fileCount) ||
            fileIndex < 0 ||
            fileCount < 1 ||
            loaded < 0 ||
            total < 1) {
            return null;
        }
        const percentage = Math.min(100, Math.floor((loaded * 100) / total));
        const prefix = `${fileIndex + 1}/${fileCount}`;
        const suffix = `${percentage}%`;
        const availableBarWidth = columns - prefix.length - suffix.length - 4;
        const width = Math.max(1, Math.min(24, availableBarWidth));
        const filled = Math.round((percentage * width) / 100);
        const bar = `${"=".repeat(filled)}${"-".repeat(width - filled)}`;
        return `${prefix} [${bar}] ${suffix}`;
    }
    function markLinuxVmReady(session) {
        if (session.state !== "starting") {
            return;
        }
        session.state = "running";
        finishLinuxVmProgress(session, "Runtime ready");
        flushLinuxVmInput(session);
        startNextLinuxVmCommand(session);
    }
    function failLinuxVmSession(session, error) {
        if (session.state === "closed" || session.state === "failed") {
            return;
        }
        finishLinuxVmProgress(session, null);
        session.state = "failed";
        session.exitCode = 1;
        rejectLinuxVmCommands(session, error);
        const message = error instanceof Error ? error.message : String(error);
        const output = textEncoder.encode(`\r\n[Linux VM failed: ${message}]\r\n`);
        if (session.outputLength + output.length <= linuxVmOutputLimit) {
            appendLinuxVmOutput(session, output);
        }
        if (session.emulator !== null) {
            void session.emulator.destroy().catch((destroyError) => {
                console.error("Failed to stop Linux VM terminal after an error", destroyError);
            });
        }
    }
    function flushLinuxVmInput(session) {
        const emulator = session.emulator;
        if (emulator === null || session.state !== "running") {
            return;
        }
        for (const data of session.inputQueue) {
            emulator.serial_send_bytes(0, data);
        }
        session.inputQueue = [];
    }
    async function startLinuxVm(session) {
        try {
            renderLinuxVmProgress(session, "Preparing runtime");
            const modulePath = v86AssetUrl("libv86.mjs");
            const module = await import(modulePath);
            if (!linuxVmSessions.has(session.id) || session.state === "closed") {
                return;
            }
            renderLinuxVmProgress(session, "Downloading runtime");
            const emulator = new module.V86({
                wasm_path: v86AssetUrl("v86.wasm"),
                memory_size: 512 * 1024 * 1024,
                vga_memory_size: 2 * 1024 * 1024,
                bios: { url: v86AssetUrl("seabios.bin") },
                vga_bios: { url: v86AssetUrl("vgabios.bin") },
                bzimage: { url: v86RuntimeAssetUrl("operit-runtime-bzimage.bin") },
                initrd: { url: v86RuntimeAssetUrl("operit-runtime-initrd.cpio.gz") },
                cmdline: `console=ttyS0 operit.mode=terminal operit.rows=${session.rows} operit.cols=${session.cols} tsc=reliable mitigations=off random.trust_cpu=on`,
                autostart: true,
                disable_keyboard: true,
                disable_mouse: true,
                disable_speaker: true,
            });
            session.emulator = emulator;
            emulator.add_listener("serial0-output-byte", (value) => {
                if (typeof value === "number" && session.state !== "closed") {
                    const byte = value & 0xff;
                    finishLinuxVmProgress(session, "Starting Linux");
                    const activeCommand = session.activeCommand;
                    if (activeCommand === null) {
                        appendLinuxVmOutput(session, Uint8Array.of(byte));
                    }
                    else {
                        consumeLinuxVmCommandByte(session, activeCommand, byte);
                    }
                    if (session.state === "starting") {
                        session.startupText = `${session.startupText}${String.fromCharCode(byte)}`.slice(-128);
                        if (session.startupText.includes("OPERIT_TERMINAL_READY")) {
                            markLinuxVmReady(session);
                        }
                    }
                }
            });
            emulator.add_listener("emulator-started", () => {
                if (session.state === "starting") {
                    renderLinuxVmProgress(session, "Starting Linux");
                }
            });
            emulator.add_listener("download-progress", (value) => {
                const status = linuxVmDownloadStatus(value, session.cols);
                if (status !== null && status !== session.lastDownloadProgress) {
                    session.lastDownloadProgress = status;
                    renderLinuxVmProgress(session, status);
                }
            });
            emulator.add_listener("emulator-stopped", () => {
                if (session.state !== "closed" && session.state !== "failed") {
                    session.state = "closed";
                    session.exitCode = 0;
                    rejectLinuxVmCommands(session, new Error(`Linux VM terminal stopped: ${session.id}`));
                }
            });
            emulator.add_listener("download-error", (value) => {
                failLinuxVmSession(session, new Error(`Linux VM asset download failed: ${String(value)}`));
            });
        }
        catch (error) {
            failLinuxVmSession(session, error);
        }
    }
    function startLinuxVmSession(sessionId, rows, cols) {
        if (linuxVmSessions.has(sessionId)) {
            throw new Error(`Linux VM terminal session already exists: ${sessionId}`);
        }
        if (!Number.isInteger(rows) || rows < 1 || !Number.isInteger(cols) || cols < 1) {
            throw new Error(`Invalid Linux VM terminal dimensions: ${rows}x${cols}`);
        }
        const session = {
            id: sessionId,
            emulator: null,
            state: "starting",
            exitCode: null,
            rows,
            cols,
            output: new Uint8Array(4096),
            outputLength: 0,
            inputQueue: [],
            startupText: "",
            lastDownloadProgress: null,
            progressVisible: false,
            activeCommand: null,
            pendingCommands: [],
        };
        linuxVmSessions.set(sessionId, session);
        void startLinuxVm(session);
    }
    function readLinuxVmPty(sessionId) {
        const session = linuxVmSession(sessionId);
        const output = session.output.slice(0, session.outputLength);
        session.outputLength = 0;
        return output;
    }
    function writeLinuxVmPty(sessionId, data) {
        const session = linuxVmSession(sessionId);
        if (session.state === "failed" || session.state === "closed") {
            throw new Error(`Linux VM terminal is not running: ${sessionId}`);
        }
        const bytes = new Uint8Array(data);
        if (session.state === "starting") {
            session.inputQueue.push(bytes);
        }
        else {
            const emulator = session.emulator;
            if (emulator === null) {
                throw new Error(`Linux VM terminal emulator is unavailable: ${sessionId}`);
            }
            emulator.serial_send_bytes(0, bytes);
        }
        return bytes.length;
    }
    function resizeLinuxVmPty(sessionId, rows, cols) {
        const session = linuxVmSession(sessionId);
        if (!Number.isInteger(rows) || rows < 1 || !Number.isInteger(cols) || cols < 1) {
            throw new Error(`Invalid Linux VM terminal dimensions: ${rows}x${cols}`);
        }
        session.rows = rows;
        session.cols = cols;
        if (session.state === "running" && session.emulator !== null) {
            const resize = `\x1b]1337;OPERIT_RESIZE;${rows};${cols}\x07`;
            session.emulator.serial_send_bytes(0, textEncoder.encode(resize));
        }
    }
    function linuxVmPtyExitCode(sessionId) {
        return linuxVmSession(sessionId).exitCode;
    }
    function closeLinuxVmPty(sessionId) {
        const session = linuxVmSession(sessionId);
        session.state = "closed";
        rejectLinuxVmCommands(session, new Error(`Linux VM terminal closed: ${sessionId}`));
        linuxVmSessions.delete(sessionId);
        if (session.emulator !== null) {
            void session.emulator.destroy().catch((error) => {
                console.error("Failed to stop Linux VM terminal", error);
            });
        }
    }
    function installWebLocalInferenceTestApi() {
        if (runtimeGlobal.__OPERIT_LOCAL_INFERENCE_TEST__ !== true) {
            return;
        }
        runtimeGlobal.__operitLocalInferenceTest = {
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
    runtimeGlobal.__operitHost = {
        terminal: registerMainHostModule({
            startPty(sessionId, rows, cols) {
                startLinuxVmSession(sessionId, rows, cols);
            },
            readPty(sessionId) {
                return readLinuxVmPty(sessionId);
            },
            writePty(sessionId, data) {
                return writeLinuxVmPty(sessionId, data);
            },
            executePtyCommand(sessionId, command, timeoutMs) {
                return executeLinuxVmCommand(sessionId, command, timeoutMs);
            },
            resizePty(sessionId, rows, cols) {
                resizeLinuxVmPty(sessionId, rows, cols);
            },
            exitCode(sessionId) {
                return linuxVmPtyExitCode(sessionId);
            },
            closePty(sessionId) {
                closeLinuxVmPty(sessionId);
            },
        }),
        runtimeStorage: registerWorkerHostModule({
            readBytes(path) {
                return storageRead(runtimePrefix, path);
            },
            readBytesRange(path, offset, length) {
                return storageReadRange(runtimePrefix, path, offset, length);
            },
            writeBytes(path, content) {
                storageWrite(runtimePrefix, path, content);
            },
            appendBytes(path, content) {
                storageAppend(runtimePrefix, path, content);
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
            createWriteSession(path) {
                const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
                if (storage === undefined) {
                    throw new Error("runtime worker OPFS storage is not installed");
                }
                return storage.createWriteSession(path);
            },
            writeSessionChunk(sessionId, content) {
                const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
                if (storage === undefined) {
                    throw new Error("runtime worker OPFS storage is not installed");
                }
                storage.writeSessionChunk(sessionId, content);
            },
            commitWriteSession(sessionId) {
                const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
                if (storage === undefined) {
                    throw new Error("runtime worker OPFS storage is not installed");
                }
                storage.commitWriteSession(sessionId);
            },
            discardWriteSession(sessionId) {
                const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
                if (storage === undefined) {
                    throw new Error("runtime worker OPFS storage is not installed");
                }
                storage.discardWriteSession(sessionId);
            },
        }),
        archiveStaging: registerWorkerHostModule({
            createArchive(archiveId, expectedByteLength) {
                const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
                if (staging === undefined) {
                    throw new Error("runtime worker archive staging is not installed");
                }
                staging.createArchive(archiveId, expectedByteLength);
            },
            appendArchive(archiveId, content) {
                const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
                if (staging === undefined) {
                    throw new Error("runtime worker archive staging is not installed");
                }
                staging.appendArchive(archiveId, content);
            },
            sealArchive(archiveId) {
                const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
                if (staging === undefined) {
                    throw new Error("runtime worker archive staging is not installed");
                }
                return staging.sealArchive(archiveId);
            },
            readArchive(archiveId, offset, length) {
                const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
                if (staging === undefined) {
                    throw new Error("runtime worker archive staging is not installed");
                }
                return staging.readArchive(archiveId, offset, length);
            },
            removeArchive(archiveId) {
                const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
                if (staging === undefined) {
                    throw new Error("runtime worker archive staging is not installed");
                }
                staging.removeArchive(archiveId);
            },
        }),
        hostSecretStore: registerMainHostModule({
            readSecret(key) {
                if (isModelInstallWorker()) {
                    const value = workerSecrets.get(key);
                    return value === undefined ? null : Uint8Array.from(value);
                }
                const value = localStorage.getItem(`${secretPrefix}${key}`);
                return value === null ? null : base64ToBytes(value);
            },
            writeSecret(key, content) {
                if (isModelInstallWorker()) {
                    workerSecrets.set(key, Uint8Array.from(content));
                    workerChangedSecretKeys.add(key);
                    return;
                }
                localStorage.setItem(`${secretPrefix}${key}`, bytesToBase64(new Uint8Array(content)));
            },
            deleteSecret(key) {
                if (isModelInstallWorker()) {
                    workerSecrets.delete(key);
                    workerChangedSecretKeys.add(key);
                    return;
                }
                localStorage.removeItem(`${secretPrefix}${key}`);
            },
        }),
        sqlite: registerWorkerHostModule({
            open(path) {
                if (!SQLite) {
                    throw new Error("sqlite host is not initialized");
                }
                const id = `sqlite-${++sqliteConnectionIndex}`;
                const bytes = storageRead(runtimePrefix, path);
                sqliteConnections.set(id, {
                    path,
                    db: bytes.byteLength === 0 ? new SQLite.Database() : new SQLite.Database(bytes),
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
                const value = rows[0]?.values[0];
                return value !== undefined &&
                    (value.kind === "integer" || value.kind === "real" || value.kind === "text")
                    ? value.value
                    : "0";
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
                const value = rows[0]?.values[0];
                return value !== undefined &&
                    (value.kind === "integer" || value.kind === "real" || value.kind === "text")
                    ? value.value
                    : "0";
            },
            commitTransaction(id) {
                const connection = sqliteTransaction(id);
                connection.db.run("COMMIT");
                saveSqliteDatabase(connection);
                sqliteTransactions.delete(id);
            },
        }),
        fileSystem: registerWorkerHostModule({
            validatePath() { },
            listFiles(path) {
                return listFileDirectory(path).map((entry) => ({
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
                deleteFileDirectory(path, recursive);
            },
            fileExists(path) {
                const isFile = storageHasFile(filePrefix, path);
                const isDirectory = !isFile && fileDirectoryExists(path);
                return {
                    exists: isFile || isDirectory,
                    isDirectory,
                    size: isFile ? storageRead(filePrefix, path).length : 0,
                };
            },
            moveFile(source, destination) {
                const content = storageRead(filePrefix, source);
                storageWrite(filePrefix, destination, content);
                storageDelete(filePrefix, source, false);
            },
            copyFile(source, destination, recursive) {
                copyFileSystemEntry(source, destination, recursive);
            },
            makeDirectory(path, createParents) {
                makeFileDirectory(path, createParents);
            },
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
            openFile() { },
            shareFile() { },
        }),
        webVisit: registerMainHostModule({
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
        }),
        localInference: registerMainHostModule({
            transcribeLocalSpeech(requestJson) {
                return localInferenceRunner("transcribeLocalSpeech")(requestJson);
            },
            synthesizeLocalSpeech(requestJson) {
                return localInferenceRunner("synthesizeLocalSpeech")(requestJson);
            },
        }),
        http: registerMainHostModule({
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
                        form.append(part.fieldName, new Blob([new Uint8Array(part.content)], { type: part.contentType }), part.fileName);
                    }
                    body = form;
                }
                else if (request.body && request.body.length) {
                    body = ownedBytes(request.body);
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
            openHttpByteStream(streamId, request, onOpened, onChunk, onClosed) {
                if (activeHttpByteStreamControllers.has(streamId)) {
                    throw new Error(`HTTP byte stream is already open: ${streamId}`);
                }
                const controller = new AbortController();
                activeHttpByteStreamControllers.set(streamId, controller);
                void (async () => {
                    try {
                        const headers = new Headers();
                        for (const pair of request.headers || []) {
                            const name = Array.isArray(pair) ? pair[0] : pair.key;
                            const value = Array.isArray(pair) ? pair[1] : pair.value;
                            headers.set(name, value);
                        }
                        let body;
                        if ((request.fileParts && request.fileParts.length) || (request.formFields && request.formFields.length)) {
                            const form = new FormData();
                            for (const pair of request.formFields || []) {
                                const name = Array.isArray(pair) ? pair[0] : pair.key;
                                const value = Array.isArray(pair) ? pair[1] : pair.value;
                                form.append(name, value);
                            }
                            for (const part of request.fileParts || []) {
                                form.append(part.fieldName, new Blob([new Uint8Array(part.content)], { type: part.contentType }), part.fileName);
                            }
                            body = form;
                        }
                        else if (request.body && request.body.length) {
                            body = ownedBytes(request.body);
                        }
                        const response = await fetch(request.url, {
                            method: request.method,
                            headers,
                            body,
                            signal: controller.signal,
                            redirect: request.followRedirects ? "follow" : "manual",
                        });
                        if (!response.ok) {
                            throw new Error(`HTTP ${response.status} ${await response.text()}`);
                        }
                        if (response.body === null) {
                            throw new Error("HTTP byte stream response has no body");
                        }
                        onOpened();
                        const reader = response.body.getReader();
                        while (true) {
                            const result = await reader.read();
                            if (result.done) {
                                break;
                            }
                            onChunk(Uint8Array.from(result.value));
                        }
                        onClosed(null);
                    }
                    catch (error) {
                        onClosed(controller.signal.aborted ? null : String(error));
                    }
                    finally {
                        activeHttpByteStreamControllers.delete(streamId);
                    }
                })();
            },
            closeHttpByteStream(streamId) {
                const controller = activeHttpByteStreamControllers.get(streamId);
                if (controller === undefined) {
                    throw new Error(`HTTP byte stream is not open: ${streamId}`);
                }
                controller.abort();
            },
            downloadFile(request) {
                const bytes = workerDownloads.get(request.url);
                if (bytes === undefined) {
                    workerDownloadRequests.set(request.url, structuredClone(request));
                    throw new Error(`download ${request.fileId} is pending in the browser HTTP host`);
                }
                if (typeof request.expectedBytes === "number" && bytes.length !== request.expectedBytes) {
                    throw new Error(`download ${request.fileId} size mismatch: ${bytes.length} != ${request.expectedBytes}`);
                }
                storageWrite(runtimePrefix, request.targetPath, bytes);
                return {
                    fileId: String(request.fileId),
                    finalUrl: request.url,
                    targetPath: String(request.targetPath),
                    downloadedBytes: bytes.length,
                };
            },
        }),
        managedRuntime: registerMainHostModule({
            runtimeWorkspaceDir() {
                return "operit2/workspace";
            },
            resolveRuntimeExecutable(program, executablePath) {
                const runtime = guestRuntimeExecutable(program);
                const requestedPath = executablePath?.trim();
                if (requestedPath && requestedPath !== runtime.executable) {
                    throw new Error(`V86 managed runtime cannot execute host path: ${requestedPath}`);
                }
                return runtime.executable;
            },
            startRuntimeProcess(request) {
                return startManagedRuntimeProcess(request);
            },
            runRuntimeCommand(request) {
                return runManagedRuntimeCommand(request);
            },
        }),
        managedRuntimeProcess: registerMainHostModule({
            writeLine(id, line) {
                writeManagedRuntimeLines(id, [line]);
            },
            writeLines(id, lines) {
                writeManagedRuntimeLines(id, lines);
            },
            readStdoutLine(id, timeoutMs) {
                return readManagedRuntimeStdoutLine(id, timeoutMs);
            },
            drainStderr(id) {
                return drainManagedRuntimeStderr(id);
            },
            isRunning(id) {
                return isManagedRuntimeRunning(id);
            },
            kill(id) {
                killManagedRuntimeProcess(id);
            },
        }),
        musicPlayback: registerMainHostModule(musicPlayback),
        bluetooth: registerMainHostModule(bluetooth),
        ttsPlayback: registerMainHostModule(ttsPlayback),
        systemOperation: registerMainHostModule({
            getSystemLanguageCode() {
                return { languageCode: navigator.language };
            },
            toast(message) {
                console.info("[Operit toast]", message);
            },
            sendNotification(title, message, activationJson) {
                if (!("Notification" in globalThis)) {
                    throw new Error("Browser Notification API is unavailable");
                }
                const activation = parseBrowserNotificationActivation(activationJson);
                if (Notification.permission === "granted") {
                    showBrowserNotification(title, message, activation, activationJson);
                    return;
                }
                if (Notification.permission === "denied") {
                    throw new Error("Browser notification permission is denied");
                }
                void Notification.requestPermission().then((permission) => {
                    if (permission === "granted") {
                        showBrowserNotification(title, message, activation, activationJson);
                    }
                });
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
        }),
    };
    let bridgePromise = null;
    async function initializeWasmBridge(module, wasi) {
        await ensureBrowserStorage();
        await ensureSqlite();
        if (isMainRuntimeHost()) {
            await ensureWebLocalInference();
        }
        const wasm = await module.default({ module_or_path: "./operit_flutter_bridge_bg.wasm" });
        wasi.setWasiMemory(wasm.memory);
        return new module.OperitFlutterBridgeWasm();
    }
    async function bridge() {
        if (!bridgePromise) {
            const wasmModulePath = isModelInstallWorker() || isRuntimeWorker()
                ? "./operit_flutter_bridge_worker.js"
                : "./operit_flutter_bridge.js";
            const wasmModule = importRuntimeScript(wasmModulePath);
            const wasiModule = importRuntimeScript("./wasi_snapshot_preview1.js");
            bridgePromise = Promise.all([wasmModule, wasiModule])
                .then(([module, wasi]) => initializeWasmBridge(module, wasi));
        }
        return bridgePromise;
    }
    function isLocalModelInstallRequest(request) {
        const decoded = MessagePack.decode(request);
        if (!Array.isArray(decoded) || decoded.length !== 4) {
            return false;
        }
        const targetPath = decoded[1];
        return Array.isArray(targetPath) &&
            targetPath.length === 2 &&
            targetPath[0] === "services" &&
            targetPath[1] === "localModelService" &&
            decoded[2] === "installModel";
    }
    function localModelIdentity(request) {
        const decoded = MessagePack.decode(request);
        if (!Array.isArray(decoded) || decoded.length !== 4) {
            throw new Error("local model service request is invalid");
        }
        const args = decoded[3];
        if (typeof args !== "object" || args === null || Array.isArray(args)) {
            throw new Error("local model service arguments are invalid");
        }
        const modelId = args.modelId;
        const version = args.version;
        if (typeof modelId !== "string" || typeof version !== "string") {
            throw new Error("local model service request requires modelId and version");
        }
        return { modelId, version };
    }
    async function downloadModelInstallRequests(requests, identity) {
        return Promise.all(requests.map(request => downloadHttpRequest(request, identity)));
    }
    function downloadHttpRequest(request, identity) {
        const active = activeHttpDownloadPromises.get(request.url);
        if (active !== undefined) {
            return active;
        }
        const promise = executeHttpDownloadRequest(request, identity).finally(() => {
            activeHttpDownloadPromises.delete(request.url);
            activeHttpDownloadControllers.delete(request.url);
        });
        activeHttpDownloadPromises.set(request.url, promise);
        return promise;
    }
    async function executeHttpDownloadRequest(request, identity) {
        if (typeof request.expectedBytes !== "number") {
            throw new Error(`download ${request.fileId} does not declare its byte size`);
        }
        let persisted = await readHttpDownload(request.url);
        if (persisted === null) {
            persisted = {
                url: request.url,
                fileId: request.fileId,
                expectedBytes: request.expectedBytes,
                downloadedBytes: 0,
                content: new Blob(),
                modelId: identity.modelId,
                version: identity.version,
                paused: false,
            };
            await writeHttpDownload(persisted);
        }
        if (persisted.fileId !== request.fileId ||
            persisted.expectedBytes !== request.expectedBytes ||
            persisted.downloadedBytes !== persisted.content.size
            || persisted.modelId !== identity.modelId
            || persisted.version !== identity.version
            || typeof persisted.paused !== "boolean") {
            throw new Error(`persisted HTTP download metadata mismatch: ${request.fileId}`);
        }
        if (persisted.paused) {
            persisted.paused = false;
            await writeHttpDownload(persisted);
        }
        if (persisted.downloadedBytes < persisted.expectedBytes) {
            const headers = new Headers();
            if (request.headers !== undefined) {
                for (const pair of request.headers) {
                    headers.set(Array.isArray(pair) ? pair[0] : pair.key, Array.isArray(pair) ? pair[1] : pair.value);
                }
            }
            if (persisted.downloadedBytes > 0) {
                headers.set("Range", `bytes=${persisted.downloadedBytes}-`);
            }
            const controller = new AbortController();
            activeHttpDownloadControllers.set(request.url, controller);
            const response = await fetch(request.url, { headers, signal: controller.signal });
            const expectedStatus = persisted.downloadedBytes === 0 ? 200 : 206;
            if (response.status !== expectedStatus) {
                throw new Error(`download ${request.fileId} expected HTTP ${expectedStatus}, got ${response.status}`);
            }
            const body = response.body;
            if (body === null) {
                throw new Error(`download ${request.fileId} has no response body`);
            }
            const reader = body.getReader();
            for (;;) {
                const chunk = await reader.read();
                if (chunk.done) {
                    break;
                }
                persisted.content = new Blob([persisted.content, blobPart(chunk.value)]);
                persisted.downloadedBytes += chunk.value.length;
                if (persisted.downloadedBytes > persisted.expectedBytes) {
                    throw new Error(`download ${request.fileId} exceeded its declared byte size`);
                }
                await writeHttpDownload(persisted);
            }
        }
        if (persisted.downloadedBytes !== persisted.expectedBytes) {
            throw new Error(`download ${request.fileId} size mismatch: ${persisted.downloadedBytes} != ${persisted.expectedBytes}`);
        }
        return {
            url: request.url,
            bytes: new Uint8Array(await persisted.content.arrayBuffer()),
        };
    }
    async function reloadModelInstallDownloads(downloads) {
        await Promise.all(downloads.map(async (download) => {
            const persisted = await readHttpDownload(download.url);
            if (persisted === null) {
                throw new Error(`model installation download is missing: ${download.url}`);
            }
            if (persisted.downloadedBytes !== persisted.expectedBytes) {
                throw new Error(`model installation download is incomplete: ${download.url}`);
            }
            download.bytes = new Uint8Array(await persisted.content.arrayBuffer());
        }));
    }
    async function applyModelInstallWorkerStorageChanges(changes) {
        let registryChanged = false;
        const registryKey = key(runtimePrefix, "runtime/config/preferences/local_model_registry.preferences.json");
        await persistModelInstallStorageChanges(changes);
        for (const change of changes) {
            if (change.bytes === null) {
                storageCache.delete(change.key);
            }
            else {
                storageCache.set(change.key, change.bytes);
            }
            if (change.key === registryKey) {
                registryChanged = true;
            }
        }
        if (registryChanged) {
            scheduleWebLocalInferenceRefresh();
        }
    }
    function installLocalModelInWorker(request) {
        const identity = localModelIdentity(request);
        const taskKey = localModelTaskKey(identity);
        const active = activeModelInstallPromises.get(taskKey);
        if (active !== undefined) {
            return active;
        }
        const generation = startModelInstallTask(taskKey);
        const operation = executeLocalModelInstall(request, identity, taskKey, generation)
            .finally(() => {
            if (activeModelInstallPromises.get(taskKey) === operation) {
                activeModelInstallPromises.delete(taskKey);
            }
        });
        activeModelInstallPromises.set(taskKey, operation);
        return operation;
    }
    async function executeLocalModelInstall(request, identity, taskKey, generation) {
        const downloads = [];
        for (;;) {
            let message = downloads.length === 0
                ? await executeModelInstallWorker(request, downloads, taskKey, generation)
                : await enqueueModelInstallWorker(request, downloads, taskKey, generation);
            if (downloads.length === 0 && message.type === "result") {
                message = await enqueueModelInstallWorker(request, downloads, taskKey, generation);
            }
            if (message.type === "result") {
                for (const download of downloads) {
                    httpDownloadStatusCache.delete(download.url);
                }
                await Promise.all(downloads.map(download => deleteHttpDownload(download.url)));
                return message.response;
            }
            const [, completedDownloads] = await Promise.all([
                reloadModelInstallDownloads(downloads),
                downloadModelInstallRequests(message.requests, identity),
            ]);
            downloads.push(...completedDownloads);
        }
    }
    function enqueueModelInstallWorker(request, downloads, taskKey, generation) {
        const result = modelInstallCommitQueue.then(async () => {
            const message = await executeModelInstallWorker(request, downloads, taskKey, generation);
            if (message.type === "result") {
                await applyModelInstallWorkerStorageChanges(message.changes);
                applyModelInstallWorkerSecretChanges(message.secretChanges);
            }
            return message;
        });
        modelInstallCommitQueue = result.then(() => { }, () => { });
        return result;
    }
    function executeModelInstallWorker(request, downloads, taskKey, generation) {
        if (!isCurrentModelInstallTask(taskKey, generation)) {
            return Promise.reject(new Error(`local model installation paused: ${taskKey}`));
        }
        return new Promise((resolve, reject) => {
            const worker = new Worker(runtimeIdentityWorkerUrl("./operit_model_install_worker.js"), {
                type: "module",
            });
            let settled = false;
            let aborter;
            const close = () => {
                worker.terminate();
                if (activeModelInstallAborters.get(taskKey) === aborter) {
                    activeModelInstallAborters.delete(taskKey);
                }
            };
            const fail = (error) => {
                if (settled) {
                    return;
                }
                settled = true;
                close();
                reject(error);
            };
            const succeed = (message) => {
                if (settled) {
                    return;
                }
                settled = true;
                close();
                resolve(message);
            };
            aborter = () => {
                fail(new Error(`local model installation paused: ${taskKey}`));
            };
            activeModelInstallAborters.set(taskKey, aborter);
            if (!isCurrentModelInstallTask(taskKey, generation)) {
                aborter();
                return;
            }
            worker.addEventListener("message", (event) => {
                const message = event.data;
                if (message.type === "error") {
                    fail(new Error(message.message));
                    return;
                }
                succeed(message);
            }, { once: true });
            worker.addEventListener("error", event => {
                fail(event.error instanceof Error ? event.error : new Error(event.message));
            }, { once: true });
            const ownedRequest = Uint8Array.from(request);
            const secrets = collectModelInstallWorkerSecrets().map(secret => ({
                key: secret.key,
                bytes: Uint8Array.from(secret.bytes),
            }));
            const transferables = [
                ownedRequest.buffer,
                ...secrets.map(secret => secret.bytes.buffer),
                ...downloads.map(download => download.bytes.buffer),
            ];
            worker.postMessage({ type: "install", request: ownedRequest, secrets, downloads }, transferables);
        });
    }
    function localModelInstallStatus(downloads) {
        if (downloads.length === 0) {
            throw new Error("local model download status requires at least one file");
        }
        const first = downloads[0];
        return {
            operationId: `${first.modelId}@${first.version}`,
            modelId: first.modelId,
            version: first.version,
            phase: activeModelInstallPromises.has(localModelTaskKey(first)) &&
                !downloads.every(download => download.paused)
                ? "Model"
                : "Cancelled",
            currentFile: downloads.map(download => download.fileId).join(", "),
            downloadedBytes: downloads.reduce((total, download) => total + download.downloadedBytes, 0),
            totalBytes: downloads.reduce((total, download) => total + download.expectedBytes, 0),
            error: null,
        };
    }
    function groupLocalModelDownloads(downloads) {
        const groups = new Map();
        for (const download of downloads) {
            const taskKey = localModelTaskKey(download);
            const group = groups.get(taskKey);
            if (group === undefined) {
                groups.set(taskKey, [download]);
            }
            else {
                group.push(download);
            }
        }
        return Array.from(groups.values());
    }
    async function handleLocalModelDownloadCall(request) {
        const decoded = MessagePack.decode(request);
        if (!Array.isArray(decoded) || decoded.length !== 4) {
            return { handled: false, response: new Uint8Array() };
        }
        const targetPath = decoded[1];
        if (!Array.isArray(targetPath) || targetPath.length !== 2 ||
            targetPath[0] !== "services" || targetPath[1] !== "localModelService") {
            return { handled: false, response: new Uint8Array() };
        }
        const method = decoded[2];
        if (method !== "getInstallStatuses" && method !== "getInstallStatus" &&
            method !== "cancelInstall" && method !== "deleteModel") {
            return { handled: false, response: new Uint8Array() };
        }
        const downloads = await listHttpDownloads();
        if (method === "getInstallStatuses") {
            return {
                handled: true,
                response: MessagePack.encode([
                    0,
                    groupLocalModelDownloads(downloads).map(localModelInstallStatus),
                ]),
            };
        }
        const identity = localModelIdentity(request);
        const matching = downloads.filter(download => download.modelId === identity.modelId && download.version === identity.version);
        if (method === "getInstallStatus") {
            return {
                handled: true,
                response: MessagePack.encode([0, matching.length === 0 ? null : localModelInstallStatus(matching)]),
            };
        }
        if (matching.length === 0) {
            return { handled: false, response: new Uint8Array() };
        }
        if (method === "cancelInstall") {
            const taskKey = localModelTaskKey(identity);
            invalidateModelInstallTask(taskKey);
            await Promise.all(matching.map(download => stopHttpDownload(download.url)));
            for (const download of matching) {
                const persisted = await readHttpDownload(download.url);
                if (persisted === null) {
                    throw new Error(`HTTP download disappeared while pausing: ${download.url}`);
                }
                persisted.paused = true;
                await writeHttpDownload(persisted);
                download.active = false;
                download.paused = true;
            }
            return {
                handled: true,
                response: MessagePack.encode([0, localModelInstallStatus(matching)]),
            };
        }
        const taskKey = localModelTaskKey(identity);
        invalidateModelInstallTask(taskKey);
        await Promise.all(matching.map(download => deleteHttpDownload(download.url)));
        return { handled: true, response: MessagePack.encode([0, null]) };
    }
    function installRuntimeWorkerProxy() {
        const clientId = createRandomUuid();
        const coordinator = new BroadcastChannel(runtimeCoordinatorName);
        const locks = navigator.locks;
        if (locks === undefined) {
            throw new Error("Web Locks API is unavailable for the Web runtime");
        }
        let runtimeWorker = null;
        let releaseLeadership = null;
        let closed = false;
        let nextRequestId = 0;
        let nextRuntimeRequestId = 0;
        const pendingRequests = new Map();
        const pendingCommands = new Map();
        const watchCallbacks = new Map();
        const watchSubscriptions = new Map();
        const hostPayloads = new Map();
        const leaderRequests = new Map();
        const leaderRequestIds = new Map();
        const leaderWatchRequestIds = new Map();
        const leaderHostCalls = new Map();
        const disconnectedHostPayloads = new Map();
        coordinator.addEventListener("message", event => handleCoordinatorMessage(event.data));
        runtimeGlobal.addEventListener("pagehide", disconnectRuntimeWorker);
        void locks.request(runtimeOwnerLockName, { mode: "exclusive" }, runRuntimeLeadership).catch(rejectRuntimeRequests);
        async function runRuntimeLeadership() {
            if (closed) {
                return;
            }
            const worker = new Worker(runtimeIdentityWorkerUrl("./operit_runtime_worker.js"), {
                type: "module",
                name: `operit-runtime-storage-${runtimeIdentityId}`,
            });
            runtimeWorker = worker;
            worker.addEventListener("message", handleRuntimeWorkerMessage);
            worker.postMessage({ type: "configure", clientId });
            coordinator.postMessage({ type: "leaderReady" });
            forwardPendingRequests();
            await new Promise(resolve => {
                releaseLeadership = resolve;
            });
            worker.terminate();
            runtimeWorker = null;
            releaseLeadership = null;
        }
        function shutdownRuntimeWorker(worker) {
            return new Promise(resolve => {
                const handleMessage = (event) => {
                    const message = event.data;
                    if (message.type !== "shutdownComplete") {
                        return;
                    }
                    worker.removeEventListener("message", handleMessage);
                    resolve();
                };
                worker.addEventListener("message", handleMessage);
                worker.postMessage({ type: "shutdown" });
            });
        }
        function handleCoordinatorMessage(value) {
            if (typeof value !== "object" || value === null) {
                return;
            }
            if (isLeaderCoreRequest(value)) {
                if (runtimeWorker !== null) {
                    forwardLeaderRequest(value);
                }
                return;
            }
            const message = value;
            if (message.type === "clientDisconnect" && typeof message.clientId === "string") {
                if (runtimeWorker !== null) {
                    disconnectLeaderClient(message.clientId);
                }
                return;
            }
            if (message.type === "leaderReady") {
                forwardPendingRequests();
                return;
            }
            if (message.clientId !== clientId) {
                return;
            }
            handleClientRuntimeMessage(message);
        }
        function forwardPendingRequest(id, command) {
            const request = {
                type: "coreRequest",
                clientId,
                clientRequestId: id,
                operation: command.operation,
                payload: command.payload,
            };
            if (runtimeWorker !== null) {
                forwardLeaderRequest(request);
                return;
            }
            coordinator.postMessage(request);
        }
        function forwardPendingRequests() {
            for (const [id, command] of pendingCommands) {
                forwardPendingRequest(id, command);
            }
        }
        function forwardLeaderRequest(request) {
            const routedRequests = leaderRequestIdsFor(request.clientId);
            if (routedRequests.has(request.clientRequestId)) {
                return;
            }
            const worker = runtimeWorker;
            if (worker === null) {
                throw new Error("runtime owner worker is unavailable");
            }
            const id = ++nextRuntimeRequestId;
            routedRequests.set(request.clientRequestId, id);
            leaderRequests.set(id, {
                clientId: request.clientId,
                clientRequestId: request.clientRequestId,
                operation: request.operation,
                payload: request.payload,
                disconnected: false,
            });
            worker.postMessage({ ...request, id });
        }
        function handleRuntimeWorkerMessage(event) {
            const message = event.data;
            if (message.type === "hostCall") {
                forwardLeaderHostCall(message);
                return;
            }
            if (message.type === "hostPayload") {
                forwardLeaderHostPayload(message);
                return;
            }
            if (message.type === "coreResult" || message.type === "coreError" || message.type === "coreWatchEvent") {
                forwardLeaderRuntimeReply(message);
            }
        }
        function forwardLeaderRuntimeReply(message) {
            if (typeof message.id !== "number") {
                throw new Error("runtime worker reply is missing its request identifier");
            }
            const route = leaderRequests.get(message.id);
            if (route === undefined) {
                return;
            }
            if (message.type === "coreResult" && route.operation === "watchStream") {
                const subscriptionId = watchSubscriptionId(message.response);
                if (route.disconnected) {
                    if (subscriptionId !== null) {
                        closeDisconnectedWatch(route.clientId, subscriptionId);
                    }
                    forgetLeaderRequest(message.id, route);
                    return;
                }
                if (subscriptionId !== null) {
                    leaderWatchRequestIdsFor(route.clientId).set(subscriptionId, message.id);
                }
                deliverCoordinatorMessage(route.clientId, { ...message, id: route.clientRequestId });
                return;
            }
            if (message.type === "coreResult" && route.operation === "closeWatchStream" && typeof route.payload === "string") {
                const watchRequests = leaderWatchRequestIds.get(route.clientId);
                const watchRequestId = watchRequests?.get(route.payload);
                if (watchRequestId !== undefined) {
                    const watchRoute = leaderRequests.get(watchRequestId);
                    if (watchRoute !== undefined) {
                        forgetLeaderRequest(watchRequestId, watchRoute);
                    }
                    watchRequests?.delete(route.payload);
                    if (watchRequests?.size === 0) {
                        leaderWatchRequestIds.delete(route.clientId);
                    }
                }
            }
            if (!route.disconnected) {
                deliverCoordinatorMessage(route.clientId, { ...message, id: route.clientRequestId });
            }
            if (message.type !== "coreWatchEvent") {
                forgetLeaderRequest(message.id, route);
            }
        }
        function forwardLeaderHostCall(message) {
            if (typeof message.id !== "number" || typeof message.clientId !== "string" || !(message.control instanceof SharedArrayBuffer)) {
                throw new Error("runtime worker host call is invalid");
            }
            if (leaderClientHasDisconnectedRoute(message.clientId)) {
                respondToDisconnectedHostCall(message.id, message.control);
                return;
            }
            leaderHostCalls.set(message.id, { clientId: message.clientId, control: message.control });
            deliverCoordinatorMessage(message.clientId, message);
        }
        function forwardLeaderHostPayload(message) {
            if (typeof message.id !== "number" || typeof message.clientId !== "string" || !(message.payload instanceof SharedArrayBuffer)) {
                throw new Error("runtime worker host payload is invalid");
            }
            const disconnectedPayload = disconnectedHostPayloads.get(message.id);
            if (disconnectedPayload !== undefined) {
                completeDisconnectedHostPayload(message.id, message.payload, message.control, disconnectedPayload);
                return;
            }
            leaderHostCalls.delete(message.id);
            deliverCoordinatorMessage(message.clientId, message);
        }
        function deliverCoordinatorMessage(targetClientId, message) {
            if (targetClientId === clientId) {
                handleClientRuntimeMessage(message);
                return;
            }
            coordinator.postMessage(message);
        }
        function handleClientRuntimeMessage(message) {
            if (message.type === "coreResult" && typeof message.id === "number" && message.response instanceof Uint8Array) {
                const pending = pendingRequests.get(message.id);
                if (pending !== undefined) {
                    pendingRequests.delete(message.id);
                    pendingCommands.delete(message.id);
                    pending.resolve(Uint8Array.from(message.response));
                }
                return;
            }
            if (message.type === "coreError" && typeof message.id === "number") {
                const pending = pendingRequests.get(message.id);
                if (pending !== undefined) {
                    pendingRequests.delete(message.id);
                    pendingCommands.delete(message.id);
                    watchCallbacks.delete(message.id);
                    pending.reject(new Error(String(message.message)));
                }
                return;
            }
            if (message.type === "coreWatchEvent" && typeof message.id === "number" && message.event instanceof Uint8Array) {
                watchCallbacks.get(message.id)?.(Uint8Array.from(message.event));
                return;
            }
            if (message.type === "hostCall") {
                void handleRuntimeWorkerHostCall(message, hostPayloads);
                return;
            }
            if (message.type === "hostPayload") {
                handleRuntimeWorkerHostPayload(message, hostPayloads);
            }
        }
        function request(operation, payload, onWatchEvent) {
            if (closed) {
                return Promise.reject(new Error("runtime page coordinator is closed"));
            }
            const id = ++nextRequestId;
            const response = new Promise((resolve, reject) => {
                pendingRequests.set(id, { resolve, reject });
            });
            pendingCommands.set(id, { operation, payload });
            if (onWatchEvent !== undefined) {
                watchCallbacks.set(id, onWatchEvent);
            }
            forwardPendingRequest(id, { operation, payload });
            return response.then(value => ({ id, response: value }));
        }
        function forgetWatchSubscription(subscriptionId) {
            const requestId = watchSubscriptions.get(subscriptionId);
            if (requestId !== undefined) {
                watchSubscriptions.delete(subscriptionId);
                watchCallbacks.delete(requestId);
                pendingCommands.delete(requestId);
            }
        }
        function disconnectRuntimeWorker(event) {
            if (event.persisted || closed) {
                return;
            }
            closed = true;
            if (runtimeWorker !== null) {
                disconnectLeaderClient(clientId);
            }
            else {
                coordinator.postMessage({ type: "clientDisconnect", clientId });
            }
            coordinator.close();
            void releaseLeadershipAfterShutdown();
        }
        async function releaseLeadershipAfterShutdown() {
            const worker = runtimeWorker;
            const release = releaseLeadership;
            if (worker === null || release === null) {
                return;
            }
            await shutdownRuntimeWorker(worker);
            release();
        }
        function disconnectLeaderClient(disconnectedClientId) {
            const watches = leaderWatchRequestIds.get(disconnectedClientId);
            if (watches !== undefined) {
                for (const [subscriptionId, requestId] of watches) {
                    const route = leaderRequests.get(requestId);
                    if (route !== undefined) {
                        forgetLeaderRequest(requestId, route);
                    }
                    closeDisconnectedWatch(disconnectedClientId, subscriptionId);
                }
                leaderWatchRequestIds.delete(disconnectedClientId);
            }
            for (const [requestId, route] of leaderRequests) {
                if (route.clientId !== disconnectedClientId) {
                    continue;
                }
                route.disconnected = true;
            }
            for (const [hostCallId, route] of leaderHostCalls) {
                if (route.clientId === disconnectedClientId) {
                    leaderHostCalls.delete(hostCallId);
                    respondToDisconnectedHostCall(hostCallId, route.control);
                }
            }
        }
        function leaderClientHasDisconnectedRoute(disconnectedClientId) {
            for (const route of leaderRequests.values()) {
                if (route.clientId === disconnectedClientId && route.disconnected) {
                    return true;
                }
            }
            return false;
        }
        function closeDisconnectedWatch(disconnectedClientId, subscriptionId) {
            const worker = runtimeWorker;
            if (worker === null) {
                throw new Error("runtime owner worker is unavailable while closing a disconnected Watch");
            }
            worker.postMessage({
                type: "coreRequest",
                id: ++nextRuntimeRequestId,
                operation: "closeWatchStream",
                payload: subscriptionId,
                clientId: disconnectedClientId,
            });
        }
        function respondToDisconnectedHostCall(id, controlBuffer) {
            const payload = MessagePack.encode([1, "web host page disconnected"]);
            disconnectedHostPayloads.set(id, payload);
            const control = new Int32Array(controlBuffer);
            Atomics.store(control, 1, payload.byteLength);
            Atomics.store(control, 0, 1);
            Atomics.notify(control, 0);
        }
        function completeDisconnectedHostPayload(id, payloadBuffer, controlBuffer, payload) {
            if (!(controlBuffer instanceof SharedArrayBuffer)) {
                throw new Error("runtime worker disconnected host payload is missing its control buffer");
            }
            const destination = new Uint8Array(payloadBuffer);
            if (destination.byteLength !== payload.byteLength) {
                throw new Error("runtime worker disconnected host payload length is invalid");
            }
            destination.set(payload);
            disconnectedHostPayloads.delete(id);
            const control = new Int32Array(controlBuffer);
            Atomics.store(control, 0, 2);
            Atomics.notify(control, 0);
        }
        function leaderRequestIdsFor(targetClientId) {
            let requests = leaderRequestIds.get(targetClientId);
            if (requests === undefined) {
                requests = new Map();
                leaderRequestIds.set(targetClientId, requests);
            }
            return requests;
        }
        function leaderWatchRequestIdsFor(targetClientId) {
            let watches = leaderWatchRequestIds.get(targetClientId);
            if (watches === undefined) {
                watches = new Map();
                leaderWatchRequestIds.set(targetClientId, watches);
            }
            return watches;
        }
        function forgetLeaderRequest(runtimeRequestId, route) {
            leaderRequests.delete(runtimeRequestId);
            const requests = leaderRequestIds.get(route.clientId);
            requests?.delete(route.clientRequestId);
            if (requests?.size === 0) {
                leaderRequestIds.delete(route.clientId);
            }
        }
        function rejectRuntimeRequests(error) {
            const failure = error instanceof Error ? error : new Error(String(error));
            for (const pending of pendingRequests.values()) {
                pending.reject(failure);
            }
            pendingRequests.clear();
            pendingCommands.clear();
            watchCallbacks.clear();
        }
        function isLeaderCoreRequest(value) {
            if (typeof value !== "object" || value === null) {
                return false;
            }
            const request = value;
            return request.type === "coreRequest" &&
                typeof request.clientId === "string" &&
                typeof request.clientRequestId === "number" &&
                typeof request.operation === "string" &&
                (request.payload instanceof Uint8Array || typeof request.payload === "string");
        }
        function watchSubscriptionId(response) {
            if (response === undefined) {
                return null;
            }
            const decoded = MessagePack.decode(response);
            return Array.isArray(decoded) && decoded.length === 2 && decoded[0] === 0 && typeof decoded[1] === "string"
                ? decoded[1]
                : null;
        }
        runtimeGlobal.__operitRuntime = {
            async runtimeStorageDefaults() {
                return defaultRuntimeStoragePaths();
            },
            async runtimeStoragePaths(runtimeRoot, workspaceRoot) {
                return normalizeRuntimeStoragePaths(runtimeRoot, workspaceRoot);
            },
            async setRuntimeStorageRoots(runtimeRoot, workspaceRoot) {
                installRuntimeStoragePaths(runtimeRoot, workspaceRoot);
            },
            async runtimeBootstrapRead() {
                return readRuntimeBootstrapConfig();
            },
            async runtimeBootstrapWrite(content) {
                writeRuntimeBootstrapConfig(content);
            },
            async restartApplication() {
                restartRuntimeApplication();
            },
            async call(requestBytes) {
                return (await request("call", requestBytes)).response;
            },
            async controlCall(requestBytes) {
                return (await request("controlCall", requestBytes)).response;
            },
            async pushOpen(requestBytes) {
                return (await request("pushOpen", requestBytes)).response;
            },
            async pushItem(item) {
                return (await request("pushItem", item)).response;
            },
            async pushClose(pushId) {
                return (await request("pushClose", pushId)).response;
            },
            async watchSnapshot(requestBytes) {
                return (await request("watchSnapshot", requestBytes)).response;
            },
            async watchStream(requestBytes, onEvent) {
                const result = await request("watchStream", requestBytes, onEvent);
                const subscriptionId = watchSubscriptionId(result.response);
                if (subscriptionId !== null) {
                    watchSubscriptions.set(subscriptionId, result.id);
                }
                return result.response;
            },
            async closeWatchStream(subscriptionId) {
                const response = (await request("closeWatchStream", subscriptionId)).response;
                forgetWatchSubscription(subscriptionId);
                return response;
            },
        };
    }
    async function handleRuntimeWorkerHostCall(message, hostPayloads) {
        let encoded;
        try {
            const host = runtimeGlobal.__operitHost;
            const module = host?.[message.module];
            const method = module?.[message.method];
            if (typeof method !== "function") {
                throw new Error(`web host method is not available: ${message.module}.${message.method}`);
            }
            const value = await method.apply(module, message.args);
            encoded = MessagePack.encode([0, value === undefined ? null : value]);
        }
        catch (error) {
            encoded = MessagePack.encode([1, runtimeWorkerErrorMessage(error)]);
        }
        if (encoded.byteLength <= 0 || encoded.byteLength > 0x7fffffff) {
            throw new Error("web host response is outside the synchronous worker limit");
        }
        hostPayloads.set(message.id, encoded);
        const control = new Int32Array(message.control);
        Atomics.store(control, 1, encoded.byteLength);
        Atomics.store(control, 0, 1);
        Atomics.notify(control, 0);
    }
    function handleRuntimeWorkerHostPayload(message, hostPayloads) {
        const encoded = hostPayloads.get(message.id);
        if (encoded === undefined) {
            throw new Error("web host response payload is no longer available");
        }
        const payload = new Uint8Array(message.payload);
        if (payload.byteLength !== encoded.byteLength) {
            throw new Error("web host response payload length does not match metadata");
        }
        payload.set(encoded);
        hostPayloads.delete(message.id);
        const control = new Int32Array(message.control);
        Atomics.store(control, 0, 2);
        Atomics.notify(control, 0);
    }
    function runtimeWorkerErrorMessage(error) {
        return error instanceof Error ? error.message : String(error);
    }
    if (isMainRuntimeHost()) {
        installRuntimeWorkerProxy();
        return;
    }
    runtimeGlobal.__operitRuntime = {
        async runtimeStorageDefaults() {
            return defaultRuntimeStoragePaths();
        },
        async runtimeStoragePaths(runtimeRoot, workspaceRoot) {
            return normalizeRuntimeStoragePaths(runtimeRoot, workspaceRoot);
        },
        async setRuntimeStorageRoots(runtimeRoot, workspaceRoot) {
            installRuntimeStoragePaths(runtimeRoot, workspaceRoot);
        },
        async runtimeBootstrapRead() {
            return readRuntimeBootstrapConfig();
        },
        async runtimeBootstrapWrite(content) {
            writeRuntimeBootstrapConfig(content);
        },
        async restartApplication() {
            restartRuntimeApplication();
        },
        async call(request) {
            if (isMainRuntimeHost() && isLocalModelInstallRequest(request)) {
                return installLocalModelInWorker(request);
            }
            if (isMainRuntimeHost()) {
                const downloadCall = await handleLocalModelDownloadCall(request);
                if (downloadCall.handled) {
                    return downloadCall.response;
                }
            }
            return (await bridge()).call(request);
        },
        async controlCall(request) {
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
    runtimeGlobal.__operitHttpDownloadManager = {
        list: listHttpDownloads,
        pause: pauseHttpDownload,
        delete: deleteHttpDownload,
    };
    if (isModelInstallWorker()) {
        runtimeGlobal.__operitModelInstallWorkerStorageChanges = collectWorkerStorageChanges;
        runtimeGlobal.__operitModelInstallWorkerSetSecrets = setModelInstallWorkerSecrets;
        runtimeGlobal.__operitModelInstallWorkerSetDownloads = setModelInstallWorkerDownloads;
        runtimeGlobal.__operitModelInstallWorkerDownloadRequests =
            collectModelInstallWorkerDownloadRequests;
        runtimeGlobal.__operitModelInstallWorkerSecretChanges =
            collectModelInstallWorkerSecretChanges;
    }
})();
export {};
