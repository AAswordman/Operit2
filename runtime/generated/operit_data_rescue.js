const accessLock = 'operit2.data.access';
const globals = globalThis;
let runtimeAccess = null;
let maintenanceWorker = null;
let nextRequest = 0;
let nextJob = 0;
let job = null;
const pending = new Map();
export function isDataRescuePage() {
    const url = new URL(location.href);
    return url.searchParams.get('mode') === 'data-rescue' || url.pathname.endsWith('/data-rescue.html');
}
async function assertNoPendingPublication() {
    const root = await navigator.storage.getDirectory();
    let directory;
    try {
        directory = await root.getDirectoryHandle('data-rescue');
    }
    catch (error) {
        if (error instanceof DOMException && error.name === 'NotFoundError')
            return;
        throw error;
    }
    for (const name of ['commit.json']) {
        try {
            await directory.getFileHandle(name);
        }
        catch (error) {
            if (error instanceof DOMException && error.name === 'NotFoundError')
                continue;
            throw error;
        }
        throw new Error('存在中断的数据导入，请从数据救援入口完成导入。');
    }
}
export function acquireRuntimeAccess() {
    if (runtimeAccess !== null)
        return runtimeAccess;
    runtimeAccess = new Promise((resolve, reject) => {
        void navigator.locks.request(accessLock, { mode: 'shared', ifAvailable: true }, async (lock) => {
            if (lock === null)
                throw new Error('数据救援正在使用存储，请关闭维护页面。');
            await assertNoPendingPublication();
            resolve();
            await new Promise(release => addEventListener('pagehide', (event) => { if (!event.persisted)
                release(); }));
        }).catch(reject);
    });
    return runtimeAccess;
}
function localSnapshot() {
    const values = {};
    for (let index = 0; index < localStorage.length; index += 1) {
        const key = localStorage.key(index);
        if (key !== null && key.startsWith('operit2.')) {
            const value = localStorage.getItem(key);
            if (value === null)
                throw new Error(`Browser storage changed: ${key}`);
            values[key] = value;
        }
    }
    return values;
}
function worker() {
    if (maintenanceWorker !== null)
        return maintenanceWorker;
    const created = new Worker(new URL('./operit_data_rescue_worker.js', import.meta.url), { type: 'module', name: 'operit-data-rescue' });
    created.onmessage = event => {
        const message = event.data;
        if (message.progress !== undefined) {
            if (job !== null && message.id === job.requestId)
                job.progress = message.progress;
            return;
        }
        const request = pending.get(message.id);
        if (request === undefined)
            return;
        pending.delete(message.id);
        if (message.error !== undefined)
            request.reject(new Error(message.error));
        else if (message.value !== undefined)
            request.resolve(message.value);
        else
            request.reject(new Error('Invalid maintenance Worker response'));
    };
    created.onerror = event => {
        const error = new Error(event.message || 'Maintenance Worker failed');
        for (const request of pending.values())
            request.reject(error);
        pending.clear();
        created.terminate();
        if (maintenanceWorker === created)
            maintenanceWorker = null;
    };
    maintenanceWorker = created;
    return created;
}
function send(request) {
    const target = worker();
    const id = ++nextRequest;
    if (job !== null)
        job.requestId = id;
    const response = new Promise((resolve, reject) => { pending.set(id, { resolve, reject }); target.postMessage({ id, request, localStorage: localSnapshot() }); });
    return response.catch(error => {
        target.terminate();
        if (maintenanceWorker === target)
            maintenanceWorker = null;
        throw error;
    });
}
function publishLocalStorage(values) {
    for (const key of Object.keys(values))
        if (!key.startsWith('operit2.') || typeof values[key] !== 'string')
            throw new Error('Invalid browser record');
    const previous = Object.keys(localSnapshot());
    const entries = Object.entries(values);
    let completed = 0;
    const total = previous.length + entries.length;
    if (job !== null)
        job.progress = { phase: 'publishSettings', completed, total, unit: 'items' };
    for (const key of previous) {
        localStorage.removeItem(key);
        completed += 1;
    }
    for (const [key, value] of entries) {
        localStorage.setItem(key, value);
        completed += 1;
    }
    if (job !== null)
        job.progress = { phase: 'publishSettings', completed, total, unit: 'items' };
}
async function executeWorker(request) {
    let response = await send(request);
    if (response.ok === true && typeof response.value === 'object' && response.value !== null) {
        const result = response.value;
        if (result.requiresLocalStorageCommit === true) {
            publishLocalStorage(result.localStorage);
            response = await send({ operation: 'finishPublication' });
        }
    }
    if (request.operation === 'close' && response.ok === true) {
        maintenanceWorker?.terminate();
        maintenanceWorker = null;
    }
    return response;
}
async function execute(encoded) {
    try {
        const request = JSON.parse(encoded);
        if (request.operation === 'launchMode')
            return JSON.stringify({ ok: true, value: { rescue: isDataRescuePage() } });
        if (request.operation === 'runtimeLease') {
            await acquireRuntimeAccess();
            return JSON.stringify({ ok: true, value: null });
        }
        if (!isDataRescuePage())
            throw new Error('请通过独立数据救援页面打开维护功能。');
        if (request.operation === 'close') {
            const active = job;
            if (active !== null) {
                await active.completion;
                if (job === active)
                    job = null;
            }
            if (maintenanceWorker === null)
                return JSON.stringify({ ok: true, value: null });
            return JSON.stringify(await executeWorker(request));
        }
        if (request.operation === 'poll') {
            if (job === null || request.jobId !== job.id)
                throw new Error('Unknown maintenance job');
            if (!job.done)
                return JSON.stringify({ ok: true, value: { done: false, progress: job.progress } });
            const completed = job;
            job = null;
            if (completed.response?.ok !== true)
                return JSON.stringify(completed.response);
            return JSON.stringify({ ok: true, value: { done: true, progress: completed.progress, value: completed.response.value } });
        }
        if (job !== null)
            throw new Error('A maintenance job is already active');
        if (request.operation === 'start') {
            const operation = request.request;
            if (!['export', 'inspect', 'restore', 'resumeRestore', 'configureAutomatic'].includes(operation.operation))
                throw new Error('Unsupported background maintenance operation');
            let complete;
            const completion = new Promise(resolve => { complete = resolve; });
            const started = { id: ++nextJob, requestId: 0, progress: null, done: false, completion };
            job = started;
            void executeWorker(operation).then(response => { started.response = response; started.done = true; }, error => {
                started.response = { ok: false, error: String(error) };
                started.done = true;
            }).finally(complete);
            return JSON.stringify({ ok: true, value: { jobId: started.id } });
        }
        return JSON.stringify(await executeWorker(request));
    }
    catch (error) {
        return JSON.stringify({ ok: false, error: String(error) });
    }
}
globals.__operitDataRescue = execute;
