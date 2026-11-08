/** Independent browser entry and maintenance Worker transport. */
const accessLock = 'operit2.data.access';
const globals = globalThis as typeof globalThis & { __operitDataRescue?: (request: string) => Promise<string> };
let runtimeAccess: Promise<void> | null = null;
let maintenanceWorker: Worker | null = null;
let nextRequest = 0;
let nextJob = 0;
interface Job { id: number; requestId: number; progress: unknown; done: boolean; completion: Promise<void>; response?: Record<string, unknown>; }
let job: Job | null = null;
const pending = new Map<number, { resolve: (value: Record<string, unknown>) => void; reject: (error: Error) => void }>();

/** Reads the explicit route without decoding application configuration. */
export function isDataRescuePage(): boolean {
  const url = new URL(location.href);
  return url.searchParams.get('mode') === 'data-rescue' || url.pathname.endsWith('/data-rescue.html');
}

/** Blocks application startup until an interrupted import is explicitly completed. */
async function assertNoPendingPublication(): Promise<void> {
  const root = await navigator.storage.getDirectory();
  let directory: FileSystemDirectoryHandle;
  try { directory = await root.getDirectoryHandle('data-rescue'); }
  catch (error) { if (error instanceof DOMException && error.name === 'NotFoundError') return; throw error; }
  for (const name of ['commit.json']) {
    try { await directory.getFileHandle(name); }
    catch (error) { if (error instanceof DOMException && error.name === 'NotFoundError') continue; throw error; }
    throw new Error('存在中断的数据导入，请从数据救援入口完成导入。');
  }
}

/** Holds shared origin access before the normal runtime Worker is imported. */
export function acquireRuntimeAccess(): Promise<void> {
  if (runtimeAccess !== null) return runtimeAccess;
  runtimeAccess = new Promise<void>((resolve, reject) => {
    void navigator.locks.request(accessLock, { mode: 'shared', ifAvailable: true }, async lock => {
      if (lock === null) throw new Error('数据救援正在使用存储，请关闭维护页面。');
      await assertNoPendingPublication();
      resolve();
      await new Promise<void>(release => addEventListener('pagehide', (event: PageTransitionEvent) => { if (!event.persisted) release(); }));
    }).catch(reject);
  });
  return runtimeAccess;
}

/** Captures raw Operit browser values including bootstrap bytes and Host secrets. */
function localSnapshot(): Record<string, string> {
  const values: Record<string, string> = {};
  for (let index = 0; index < localStorage.length; index += 1) {
    const key = localStorage.key(index);
    if (key !== null && key.startsWith('operit2.')) {
      const value = localStorage.getItem(key);
      if (value === null) throw new Error(`Browser storage changed: ${key}`);
      values[key] = value;
    }
  }
  return values;
}

/** Creates the offline Worker whose lifetime owns exclusive storage access. */
function worker(): Worker {
  if (maintenanceWorker !== null) return maintenanceWorker;
  const created = new Worker(new URL('./operit_data_rescue_worker.js', import.meta.url), { type: 'module', name: 'operit-data-rescue' });
  created.onmessage = event => {
    const message = event.data as { id: number; value?: Record<string, unknown>; error?: string; progress?: unknown };
    if (message.progress !== undefined) {
      if (job !== null && message.id === job.requestId) job.progress = message.progress;
      return;
    }
    const request = pending.get(message.id);
    if (request === undefined) return;
    pending.delete(message.id);
    if (message.error !== undefined) request.reject(new Error(message.error));
    else if (message.value !== undefined) request.resolve(message.value);
    else request.reject(new Error('Invalid maintenance Worker response'));
  };
  created.onerror = event => {
    const error = new Error(event.message || 'Maintenance Worker failed');
    for (const request of pending.values()) request.reject(error);
    pending.clear();
    created.terminate();
    if (maintenanceWorker === created) maintenanceWorker = null;
  };
  maintenanceWorker = created;
  return created;
}

/** Sends one command with a raw browser value snapshot. */
function send(request: Record<string, unknown>): Promise<Record<string, unknown>> {
  const target = worker();
  const id = ++nextRequest;
  if (job !== null) job.requestId = id;
  const response = new Promise<Record<string, unknown>>((resolve, reject) => { pending.set(id, { resolve, reject }); target.postMessage({ id, request, localStorage: localSnapshot() }); });
  return response.catch(error => {
    target.terminate();
    if (maintenanceWorker === target) maintenanceWorker = null;
    throw error;
  });
}

/** Publishes browser values after the full import has been durably prepared. */
function publishLocalStorage(values: Record<string, string>): void {
  for (const key of Object.keys(values)) if (!key.startsWith('operit2.') || typeof values[key] !== 'string') throw new Error('Invalid browser record');
  const previous = Object.keys(localSnapshot());
  const entries = Object.entries(values);
  let completed = 0;
  const total = previous.length + entries.length;
  if (job !== null) job.progress = { phase: 'publishSettings', completed, total, unit: 'items' };
  for (const key of previous) { localStorage.removeItem(key); completed += 1; }
  for (const [key, value] of entries) { localStorage.setItem(key, value); completed += 1; }
  if (job !== null) job.progress = { phase: 'publishSettings', completed, total, unit: 'items' };
}

/** Completes worker publication and preserves its structured operation result. */
async function executeWorker(request: Record<string, unknown>): Promise<Record<string, unknown>> {
  let response = await send(request);
  if (response.ok === true && typeof response.value === 'object' && response.value !== null) {
    const result = response.value as Record<string, unknown>;
    if (result.requiresLocalStorageCommit === true) {
      publishLocalStorage(result.localStorage as Record<string, string>);
      response = await send({ operation: 'finishPublication' });
    }
  }
  if (request.operation === 'close' && response.ok === true) { maintenanceWorker?.terminate(); maintenanceWorker = null; }
  return response;
}

/** Executes maintenance without importing the application runtime bridge. */
async function execute(encoded: string): Promise<string> {
  try {
    const request = JSON.parse(encoded) as Record<string, unknown>;
    if (request.operation === 'launchMode') return JSON.stringify({ ok: true, value: { rescue: isDataRescuePage() } });
    if (request.operation === 'runtimeLease') { await acquireRuntimeAccess(); return JSON.stringify({ ok: true, value: null }); }
    if (!isDataRescuePage()) throw new Error('请通过独立数据救援页面打开维护功能。');
    if (request.operation === 'close') {
      const active = job;
      if (active !== null) {
        await active.completion;
        if (job === active) job = null;
      }
      if (maintenanceWorker === null) return JSON.stringify({ ok: true, value: null });
      return JSON.stringify(await executeWorker(request));
    }
    if (request.operation === 'poll') {
      if (job === null || request.jobId !== job.id) throw new Error('Unknown maintenance job');
      if (!job.done) return JSON.stringify({ ok: true, value: { done: false, progress: job.progress } });
      const completed = job;
      job = null;
      if (completed.response?.ok !== true) return JSON.stringify(completed.response);
      return JSON.stringify({ ok: true, value: { done: true, progress: completed.progress, value: completed.response.value } });
    }
    if (job !== null) throw new Error('A maintenance job is already active');
    if (request.operation === 'start') {
      const operation = request.request as Record<string, unknown>;
      if (!['export', 'inspect', 'restore', 'resumeRestore', 'configureAutomatic'].includes(operation.operation as string)) throw new Error('Unsupported background maintenance operation');
      let complete!: () => void;
      const completion = new Promise<void>(resolve => { complete = resolve; });
      const started: Job = { id: ++nextJob, requestId: 0, progress: null, done: false, completion };
      job = started;
      void executeWorker(operation).then(response => { started.response = response; started.done = true; }, error => {
        started.response = { ok: false, error: String(error) }; started.done = true;
      }).finally(complete);
      return JSON.stringify({ ok: true, value: { jobId: started.id } });
    }
    return JSON.stringify(await executeWorker(request));
  } catch (error) { return JSON.stringify({ ok: false, error: String(error) }); }
}

globals.__operitDataRescue = execute;
