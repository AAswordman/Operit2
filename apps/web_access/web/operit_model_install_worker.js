const workerGlobal = globalThis;
workerGlobal.__OPERIT_MODEL_INSTALL_WORKER__ = true;
const workerReady = initializeWorkerRuntime();
workerGlobal.addEventListener("message", handleWorkerMessage);
function handleWorkerMessage(event) {
    const message = event.data;
    if (!isInstallWorkerRequest(message)) {
        return;
    }
    void workerReady.then(() => installModel(message), error => {
        postWorkerMessage({ type: "error", message: errorMessage(error) });
    });
}
async function initializeWorkerRuntime() {
    await importWorkerScript("./operit_runtime_bridge.js");
}
function importWorkerScript(path) {
    const dynamicImport = new Function("path", "return import(path)");
    return dynamicImport(path);
}
function isInstallWorkerRequest(value) {
    if (typeof value !== "object" || value === null) {
        return false;
    }
    const message = value;
    return message.type === "install" && message.request instanceof Uint8Array;
}
async function installModel(message) {
    try {
        const runtime = workerGlobal.__operitRuntime;
        const collectStorageChanges = workerGlobal.__operitModelInstallWorkerStorageChanges;
        const setSecrets = workerGlobal.__operitModelInstallWorkerSetSecrets;
        const setDownloads = workerGlobal.__operitModelInstallWorkerSetDownloads;
        const collectDownloadRequests = workerGlobal.__operitModelInstallWorkerDownloadRequests;
        const collectSecretChanges = workerGlobal.__operitModelInstallWorkerSecretChanges;
        if (runtime === undefined ||
            collectStorageChanges === undefined ||
            setSecrets === undefined ||
            setDownloads === undefined ||
            collectDownloadRequests === undefined ||
            collectSecretChanges === undefined) {
            throw new Error("local model installation worker runtime is unavailable");
        }
        setSecrets(message.secrets);
        setDownloads(message.downloads);
        const response = Uint8Array.from(await runtime.call(message.request));
        const downloadRequests = collectDownloadRequests();
        if (downloadRequests.length > 0) {
            postWorkerMessage({ type: "downloadRequests", requests: downloadRequests });
            return;
        }
        const changes = await collectStorageChanges();
        const secretChanges = collectSecretChanges();
        const transferables = [response.buffer];
        const serializedChanges = changes.map(change => {
            if (change.bytes !== null) {
                transferables.push(change.bytes.buffer);
            }
            return change;
        });
        const serializedSecretChanges = secretChanges.map(change => {
            if (change.bytes !== null) {
                transferables.push(change.bytes.buffer);
            }
            return change;
        });
        postWorkerMessage({
            type: "result",
            response,
            changes: serializedChanges,
            secretChanges: serializedSecretChanges,
        }, transferables);
    }
    catch (error) {
        postWorkerMessage({ type: "error", message: errorMessage(error) });
    }
}
function postWorkerMessage(message, transferables = []) {
    const post = workerGlobal.postMessage;
    post.call(workerGlobal, message, transferables);
}
function errorMessage(error) {
    return error instanceof Error ? error.message : String(error);
}
export {};
