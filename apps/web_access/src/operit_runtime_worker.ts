import { MessagePack } from "./operit_messagepack.js";

export {};

/** Creates a RFC 4122 version 4 identifier with the Web Crypto random byte API. */
function createRandomUuid(): string {
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

type WorkerCoreOperation =
  | "call"
  | "controlCall"
  | "pushOpen"
  | "pushItem"
  | "pushClose"
  | "watchSnapshot"
  | "watchStream"
  | "closeWatchStream";

interface WorkerRuntimeBridge {
  call(request: Uint8Array): Promise<Uint8Array>;
  controlCall(request: Uint8Array): Promise<Uint8Array>;
  pushOpen(request: Uint8Array): Promise<Uint8Array>;
  pushItem(item: Uint8Array): Promise<Uint8Array>;
  pushClose(pushId: string): Promise<Uint8Array>;
  watchSnapshot(request: Uint8Array): Promise<Uint8Array>;
  watchStream(request: Uint8Array, onEvent: (event: Uint8Array) => void): Promise<Uint8Array>;
  closeWatchStream(subscriptionId: string): Promise<Uint8Array>;
}

interface WorkerSyncAccessHandle {
  getSize(): number;
  read(buffer: Uint8Array, options?: { at?: number }): number;
  write(buffer: Uint8Array, options?: { at?: number }): number;
  truncate(size: number): void;
  flush(): void;
  close(): void;
}

interface WorkerFileHandle {
  createSyncAccessHandle(): Promise<WorkerSyncAccessHandle>;
}

interface WorkerDirectoryHandle {
  getFileHandle(name: string, options: { create: boolean }): Promise<WorkerFileHandle>;
  getDirectoryHandle(name: string, options: { create: boolean }): Promise<WorkerDirectoryHandle>;
}

interface WorkerStorageManager {
  getDirectory(): Promise<WorkerDirectoryHandle>;
}

interface WorkerStorageRecord {
  offset: number;
  byteLength: number;
}

interface WorkerStorageIndex {
  records: Array<[string, WorkerStorageRecord]>;
}

interface WorkerRuntimeStorageBridge {
  read(prefix: string, path: string): Uint8Array;
  readRange(prefix: string, path: string, offset: number, length: number): Uint8Array;
  write(prefix: string, path: string, content: Uint8Array): void;
  append(prefix: string, path: string, content: Uint8Array): void;
  hasFile(prefix: string, path: string): boolean;
  exists(prefix: string, path: string): boolean;
  delete(prefix: string, path: string, recursive: boolean): void;
  list(prefix: string, path: string): WorkerStorageEntry[];
  createWriteSession(path: string): string;
  writeSessionChunk(sessionId: string, content: Uint8Array): void;
  commitWriteSession(sessionId: string): void;
  discardWriteSession(sessionId: string): void;
}

interface WorkerArchiveStagingBridge {
  createArchive(archiveId: string, expectedByteLength: number): void;
  appendArchive(archiveId: string, content: Uint8Array): void;
  sealArchive(archiveId: string): number;
  readArchive(archiveId: string, offset: number, length: number): Uint8Array;
  removeArchive(archiveId: string): void;
}

interface WorkerStorageEntry {
  path: string;
  isDirectory: boolean;
  size: number;
}

interface WorkerWriteSession {
  key: string;
  offset: number;
  byteLength: number;
}

interface WorkerArchiveRecord {
  offset: number;
  expectedByteLength: number;
  byteLength: number;
  sealed: boolean;
}

interface WorkerCoreRequest {
  type: "coreRequest";
  id: number;
  operation: WorkerCoreOperation;
  payload: Uint8Array | string;
  clientId?: string;
}

type WorkerCoreOperationHandler = (
  runtime: WorkerRuntimeBridge,
  message: WorkerCoreRequest,
) => Promise<Uint8Array>;

type WorkerCoreOperationExecution = "serialized" | "parallel";

interface WorkerCoreOperationRegistration {
  execution: WorkerCoreOperationExecution;
  invoke: WorkerCoreOperationHandler;
}

interface WorkerHostCall {
  type: "hostCall";
  id: number;
  module: string;
  method: string;
  args: unknown[];
  control: SharedArrayBuffer;
  clientId?: string;
}

interface WorkerHostPayload {
  type: "hostPayload";
  id: number;
  payload: SharedArrayBuffer;
  control: SharedArrayBuffer;
  clientId?: string;
}

interface WorkerConfiguration {
  type: "configure";
  clientId: string;
}

interface WorkerShutdownRequest {
  type: "shutdown";
}

type WorkerInboundMessage = WorkerCoreRequest | WorkerHostPayload | WorkerConfiguration | WorkerShutdownRequest;

interface WorkerInboundMessageRegistration {
  validate(value: unknown): value is WorkerInboundMessage;
  handle(message: WorkerInboundMessage): void;
}

interface WorkerRuntimeGlobals {
  __OPERIT_RUNTIME_WORKER__?: boolean;
  __operitRuntime?: WorkerRuntimeBridge;
  __operitHost?: object;
  __operitRuntimeWorkerEnsureStorage?: () => Promise<void>;
  __operitRuntimeWorkerStorage?: WorkerRuntimeStorageBridge;
  __operitRuntimeWorkerArchiveStaging?: WorkerArchiveStagingBridge;
  __operitWorkerHostModules?: WeakSet<object>;
  __operitMainHostModules?: WeakSet<object>;
}

const workerGlobal = globalThis as typeof globalThis & WorkerRuntimeGlobals;
const controlStateIndex = 0;
const controlLengthIndex = 1;
const controlReady = 1;
const controlPayloadReady = 2;
const runtimeStorageDataFileName = "operit_runtime_storage.data";
const runtimeStorageIndexFileName = "operit_runtime_storage.index";
const archiveStagingDataFileName = "operit_archive_staging.data";
const runtimeStoragePrefix = "operit2.runtime.";
const runtimeIdentityId = requiredRuntimeIdentityId();
const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

let runtimeStorage: RuntimeWorkerStorage | null = null;
let archiveStaging: RuntimeWorkerArchiveStaging | null = null;
let runtimeStorageInitialization: Promise<void> | null = null;
let nextHostCallId = 0;
const mainHostModules = new Map<string, object>();
const workerInboundMessageRegistrations = new Map<string, WorkerInboundMessageRegistration>();
let activeHostClientId: string | null = null;
let runtimeOperationQueue: Promise<void> = Promise.resolve();
const runtimeParallelOperations = new Set<Promise<void>>();
let shutdownRequested = false;
let shutdownPromise: Promise<void> | null = null;

workerGlobal.__OPERIT_RUNTIME_WORKER__ = true;
workerGlobal.__operitRuntimeWorkerEnsureStorage = initializeRuntimeWorkerStorage;
workerGlobal.__operitRuntimeWorkerStorage = runtimeStorageBridge();
workerGlobal.__operitRuntimeWorkerArchiveStaging = archiveStagingBridge();
registerWorkerInboundMessages();
const workerReady = initializeRuntimeWorker();
workerGlobal.addEventListener("message", handleWorkerMessage);

/** Initializes the worker-owned OPFS stores and then loads the shared runtime bridge. */
async function initializeRuntimeWorker(): Promise<void> {
  await initializeRuntimeWorkerStorage();
  await importWorkerScript("./operit_runtime_bridge.js");
  installWorkerHostProxy();
}

/** Dynamically imports one runtime module relative to this worker. */
function importWorkerScript(path: string): Promise<void> {
  const dynamicImport = new Function("path", "return import(path)") as (
    modulePath: string,
  ) => Promise<void>;
  return dynamicImport(path);
}

/** Handles a Core command or a completed main-thread host payload. */
function handleWorkerMessage(event: MessageEvent<unknown>): void {
  const message = event.data;
  if (typeof message !== "object" || message === null) {
    return;
  }
  const type = Reflect.get(message, "type");
  if (typeof type !== "string") {
    return;
  }
  const registration = workerInboundMessageRegistrations.get(type);
  if (registration === undefined || !registration.validate(message)) {
    return;
  }
  registration.handle(message);
}

/** Registers every inbound worker protocol message with its validator and handler. */
function registerWorkerInboundMessages(): void {
  registerWorkerInboundMessage("configure", isWorkerConfiguration, configureWorker);
  registerWorkerInboundMessage("shutdown", isWorkerShutdownRequest, requestWorkerShutdown);
  registerWorkerInboundMessage("hostPayload", isWorkerHostPayload, receiveMainHostPayload);
  registerWorkerInboundMessage("coreRequest", isWorkerCoreRequest, enqueueCoreRequest);
}

/** Adds one typed message contract to the inbound worker protocol registry. */
function registerWorkerInboundMessage<T extends WorkerInboundMessage>(
  type: T["type"],
  validate: (value: unknown) => value is T,
  handle: (message: T) => void,
): void {
  if (workerInboundMessageRegistrations.has(type)) {
    throw new Error(`worker inbound message is registered more than once: ${type}`);
  }
  workerInboundMessageRegistrations.set(type, {
    validate,
    handle: (message: WorkerInboundMessage): void => handle(message as T),
  });
}

/** Validates the owner identity sent before this worker can process runtime requests. */
function isWorkerConfiguration(value: unknown): value is WorkerConfiguration {
  return typeof value === "object" && value !== null &&
    (value as Partial<WorkerConfiguration>).type === "configure" &&
    typeof (value as Partial<WorkerConfiguration>).clientId === "string";
}

/** Stores the browser page identity used by worker-owned background operations. */
function configureWorker(message: WorkerConfiguration): void {
  if (activeHostClientId !== null && activeHostClientId !== message.clientId) {
    throw new Error("runtime worker owner identity cannot change");
  }
  activeHostClientId = message.clientId;
}

/** Serializes Core calls so synchronous host callbacks retain their originating page. */
function enqueueCoreRequest(message: WorkerCoreRequest): void {
  if (shutdownRequested) {
    postWorkerMessage({
      type: "coreError",
      id: message.id,
      message: "runtime worker is shutting down",
      clientId: message.clientId,
    });
    return;
  }
  const execute = async (): Promise<void> => {
    try {
      await workerReady;
      await executeCoreRequest(message);
    } catch (error) {
      postWorkerMessage(
        { type: "coreError", id: message.id, message: errorMessage(error), clientId: message.clientId },
      );
    }
  };
  const registration = workerCoreOperationRegistrations.get(message.operation);
  if (registration?.execution === "parallel") {
    const operation = execute();
    runtimeParallelOperations.add(operation);
    void operation.finally(() => runtimeParallelOperations.delete(operation));
    return;
  }
  runtimeOperationQueue = runtimeOperationQueue.then(execute, execute);
}

/** Validates the lifecycle message that asks this worker to release its OPFS handles. */
function isWorkerShutdownRequest(value: unknown): value is WorkerShutdownRequest {
  return typeof value === "object" && value !== null && (value as Partial<WorkerShutdownRequest>).type === "shutdown";
}

/** Closes the runtime worker stores only after initialization and queued Core work have settled. */
function requestWorkerShutdown(): void {
  if (shutdownPromise !== null) {
    return;
  }
  shutdownRequested = true;
  shutdownPromise = (async () => {
    try {
      await workerReady;
    } catch {
      // Initialization failure still requires the handles opened before the failure to be released.
    }
    await runtimeOperationQueue;
    await Promise.all(runtimeParallelOperations);
    archiveStaging?.close();
    runtimeStorage?.close();
    archiveStaging = null;
    runtimeStorage = null;
    postWorkerMessage({ type: "shutdownComplete" });
  })();
}

/** Validates one Core command received from the browser UI thread. */
function isWorkerCoreRequest(value: unknown): value is WorkerCoreRequest {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const message = value as Partial<WorkerCoreRequest>;
  return message.type === "coreRequest" &&
    typeof message.id === "number" &&
    typeof message.operation === "string" &&
    (message.payload instanceof Uint8Array || typeof message.payload === "string");
}

/** Validates the second phase of a synchronous main-thread host response. */
function isWorkerHostPayload(value: unknown): value is WorkerHostPayload {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const message = value as Partial<WorkerHostPayload>;
  return message.type === "hostPayload" &&
    typeof message.id === "number" &&
    message.payload instanceof SharedArrayBuffer &&
    message.control instanceof SharedArrayBuffer;
}

/** Dispatches one Core request to the worker-owned WebAssembly runtime. */
async function executeCoreRequest(message: WorkerCoreRequest): Promise<void> {
  const previousClientId = activeHostClientId;
  activeHostClientId = message.clientId ?? null;
  try {
    const runtime = workerGlobal.__operitRuntime;
    if (runtime === undefined) {
      throw new Error("worker runtime bridge is unavailable");
    }
    const result = await invokeRuntimeOperation(runtime, message);
    const response = Uint8Array.from(result);
    postWorkerMessage(
      { type: "coreResult", id: message.id, response, clientId: message.clientId },
      [response.buffer],
    );
  } catch (error) {
    postWorkerMessage(
      { type: "coreError", id: message.id, message: errorMessage(error), clientId: message.clientId },
    );
  } finally {
    activeHostClientId = previousClientId;
  }
}

/** Invokes exactly one public Core Link operation on the worker runtime. */
function invokeRuntimeOperation(
  runtime: WorkerRuntimeBridge,
  message: WorkerCoreRequest,
): Promise<Uint8Array> {
  const registration = workerCoreOperationRegistrations.get(message.operation);
  if (registration === undefined) {
    throw new Error(`worker Core operation is not registered: ${message.operation}`);
  }
  return registration.invoke(runtime, message);
}

const workerCoreOperationRegistrations = new Map<WorkerCoreOperation, WorkerCoreOperationRegistration>([
  ["call", { execution: "serialized", invoke: (runtime, message) => runtime.call(requireBytesPayload(message)) }],
  ["controlCall", { execution: "parallel", invoke: (runtime, message) => runtime.controlCall(requireBytesPayload(message)) }],
  ["pushOpen", { execution: "serialized", invoke: (runtime, message) => runtime.pushOpen(requireBytesPayload(message)) }],
  ["pushItem", { execution: "serialized", invoke: (runtime, message) => runtime.pushItem(requireBytesPayload(message)) }],
  ["pushClose", { execution: "serialized", invoke: (runtime, message) => runtime.pushClose(requireStringPayload(message)) }],
  ["watchSnapshot", { execution: "serialized", invoke: (runtime, message) => runtime.watchSnapshot(requireBytesPayload(message)) }],
  ["watchStream", { execution: "serialized", invoke: (runtime, message) => runtime.watchStream(requireBytesPayload(message), event => {
    const copiedEvent = Uint8Array.from(event);
    postWorkerMessage(
      { type: "coreWatchEvent", id: message.id, event: copiedEvent, clientId: message.clientId },
      [copiedEvent.buffer],
    );
  }) }],
  ["closeWatchStream", { execution: "serialized", invoke: (runtime, message) => runtime.closeWatchStream(requireStringPayload(message)) }],
]);

/** Returns the binary payload required by a binary Core Link operation. */
function requireBytesPayload(message: WorkerCoreRequest): Uint8Array {
  if (!(message.payload instanceof Uint8Array)) {
    throw new Error(`worker Core operation ${message.operation} requires binary payload`);
  }
  return message.payload;
}

/** Returns the string payload required by a close operation. */
function requireStringPayload(message: WorkerCoreRequest): string {
  if (typeof message.payload !== "string") {
    throw new Error(`worker Core operation ${message.operation} requires string payload`);
  }
  return message.payload;
}

/** Replaces DOM-bound host modules with synchronous calls to the browser UI thread. */
function installWorkerHostProxy(): void {
  const localHost = workerGlobal.__operitHost;
  if (localHost === undefined) {
    throw new Error("worker runtime did not install a local host bridge");
  }
  const workerHostModules = workerGlobal.__operitWorkerHostModules;
  if (workerHostModules === undefined) {
    throw new Error("worker runtime did not install its host module registry");
  }
  const mainHostModuleRegistry = workerGlobal.__operitMainHostModules;
  if (mainHostModuleRegistry === undefined) {
    throw new Error("worker runtime did not install its main host module registry");
  }
  const localModules = localHost as Record<string, object>;
  workerGlobal.__operitHost = new Proxy(localModules, {
    get(target, property): unknown {
      if (typeof property !== "string") {
        return Reflect.get(target, property);
      }
      const module = Reflect.get(target, property) as object | undefined;
      if (module === undefined) {
        throw new Error(`worker host module is not registered: ${property}`);
      }
      if (workerHostModules.has(module)) {
        return module;
      }
      if (mainHostModuleRegistry.has(module)) {
        return mainHostModule(property);
      }
      throw new Error(`worker host module has no execution owner: ${property}`);
    },
  });
}

/** Returns one proxy module whose methods synchronously execute on the UI thread. */
function mainHostModule(module: string): object {
  const cached = mainHostModules.get(module);
  if (cached !== undefined) {
    return cached;
  }
  const proxy = new Proxy({}, {
    get(_target, property): unknown {
      if (typeof property !== "string") {
        return undefined;
      }
      return (...args: unknown[]): unknown => callMainHost(module, property, args);
    },
  });
  mainHostModules.set(module, proxy);
  return proxy;
}

/** Calls one UI-thread host method while blocking only this dedicated runtime worker. */
function callMainHost(module: string, method: string, args: unknown[]): unknown {
  if (activeHostClientId === null) {
    throw new Error("runtime worker host call has no client identity");
  }
  const id = ++nextHostCallId;
  const controlBuffer = new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT * 2);
  const control = new Int32Array(controlBuffer);
  postWorkerMessage({
    type: "hostCall",
    id,
    module,
    method,
    args,
    control: controlBuffer,
    clientId: activeHostClientId,
  });
  Atomics.wait(control, controlStateIndex, 0);
  if (Atomics.load(control, controlStateIndex) !== controlReady) {
    throw new Error("main-thread host did not provide response metadata");
  }
  const payloadLength = Atomics.load(control, controlLengthIndex);
  if (payloadLength <= 0) {
    throw new Error("main-thread host returned an invalid response length");
  }
  const payloadBuffer = new SharedArrayBuffer(payloadLength);
  Atomics.store(control, controlStateIndex, 0);
  postWorkerMessage({
    type: "hostPayload",
    id,
    payload: payloadBuffer,
    control: controlBuffer,
    clientId: activeHostClientId,
  });
  Atomics.wait(control, controlStateIndex, 0);
  if (Atomics.load(control, controlStateIndex) !== controlPayloadReady) {
    throw new Error("main-thread host did not provide response payload");
  }
  const encoded = Uint8Array.from(new Uint8Array(payloadBuffer));
  const envelope = MessagePack.decode(encoded);
  if (!Array.isArray(envelope) || envelope.length !== 2 || typeof envelope[0] !== "number") {
    throw new Error("main-thread host response is invalid");
  }
  if (envelope[0] !== 0) {
    throw new Error(String(envelope[1]));
  }
  return envelope[1];
}

/** Completes the second synchronous response phase after the UI thread filled the shared buffer. */
function receiveMainHostPayload(message: WorkerHostPayload): void {
  const control = new Int32Array(message.control);
  Atomics.store(control, controlStateIndex, controlPayloadReady);
  Atomics.notify(control, controlStateIndex);
}


/** Initializes OPFS-backed runtime storage and discards stale temporary archive bytes. */
async function initializeRuntimeWorkerStorage(): Promise<void> {
  if (runtimeStorage !== null && archiveStaging !== null) {
    return;
  }
  const activeInitialization = runtimeStorageInitialization;
  if (activeInitialization !== null) {
    return activeInitialization;
  }
  const initialization = initializeRuntimeWorkerStorageOnce();
  runtimeStorageInitialization = initialization;
  try {
    await initialization;
  } finally {
    if (runtimeStorageInitialization === initialization) {
      runtimeStorageInitialization = null;
    }
  }
}

/** Opens and publishes the worker-owned OPFS stores as one all-or-nothing operation. */
async function initializeRuntimeWorkerStorageOnce(): Promise<void> {
  const root = await runtimeOpfsRoot();
  const storage = await RuntimeWorkerStorage.open(root);
  let staging: RuntimeWorkerArchiveStaging | null = null;
  try {
    staging = await RuntimeWorkerArchiveStaging.open(root);
    runtimeStorage = storage;
    archiveStaging = staging;
  } catch (error) {
    staging?.close();
    storage.close();
    throw error;
  }
}

/** Returns the OPFS root available only to the dedicated runtime worker. */
async function runtimeOpfsRoot(): Promise<WorkerDirectoryHandle> {
  const storage = (navigator as Navigator & { storage: WorkerStorageManager }).storage;
  if (storage === undefined || typeof storage.getDirectory !== "function") {
    throw new Error("OPFS is unavailable for the Web runtime worker");
  }
  const root = (await storage.getDirectory()) as unknown as WorkerDirectoryHandle;
  const runtimeRoot = await root.getDirectoryHandle("runtime", { create: true });
  const identitiesRoot = await runtimeRoot.getDirectoryHandle("identities", { create: true });
  return identitiesRoot.getDirectoryHandle(runtimeIdentityId, { create: true });
}

/** Reads and validates the identity carried by this dedicated Runtime worker URL. */
function requiredRuntimeIdentityId(): string {
  const identityId = new URL(workerGlobal.location.href).searchParams.get("identity");
  if (identityId === null || !/^identity-[a-z0-9-]+$/.test(identityId)) {
    throw new Error("Runtime worker identity is missing or invalid");
  }
  return identityId;
}

/** Returns the initialized worker-owned runtime storage service. */
function requiredRuntimeStorage(): RuntimeWorkerStorage {
  if (runtimeStorage === null) {
    throw new Error("runtime OPFS storage is not initialized");
  }
  return runtimeStorage;
}

/** Returns the initialized worker-owned archive staging service. */
function requiredArchiveStaging(): RuntimeWorkerArchiveStaging {
  if (archiveStaging === null) {
    throw new Error("archive OPFS staging is not initialized");
  }
  return archiveStaging;
}

/** Exposes OPFS runtime storage through the shared Rust Web host function names. */
function runtimeStorageBridge(): WorkerRuntimeStorageBridge {
  return {
    read(prefix: string, path: string): Uint8Array {
      return requiredRuntimeStorage().read(storageKey(prefix, path));
    },
    readRange(prefix: string, path: string, offset: number, length: number): Uint8Array {
      return requiredRuntimeStorage().readRange(storageKey(prefix, path), offset, length);
    },
    write(prefix: string, path: string, content: Uint8Array): void {
      requiredRuntimeStorage().write(storageKey(prefix, path), content);
    },
    append(prefix: string, path: string, content: Uint8Array): void {
      requiredRuntimeStorage().append(storageKey(prefix, path), content);
    },
    hasFile(prefix: string, path: string): boolean {
      return requiredRuntimeStorage().hasFile(storageKey(prefix, path));
    },
    exists(prefix: string, path: string): boolean {
      return requiredRuntimeStorage().exists(storageKey(prefix, path));
    },
    delete(prefix: string, path: string, recursive: boolean): void {
      requiredRuntimeStorage().delete(storageKey(prefix, path), recursive);
    },
    list(prefix: string, path: string): WorkerStorageEntry[] {
      return requiredRuntimeStorage().list(prefix, path);
    },
    createWriteSession(path: string): string {
      return requiredRuntimeStorage().createWriteSession(storageKey(runtimeStoragePrefix, path));
    },
    writeSessionChunk(sessionId: string, content: Uint8Array): void {
      requiredRuntimeStorage().writeSessionChunk(sessionId, content);
    },
    commitWriteSession(sessionId: string): void {
      requiredRuntimeStorage().commitWriteSession(sessionId);
    },
    discardWriteSession(sessionId: string): void {
      requiredRuntimeStorage().discardWriteSession(sessionId);
    },
  };
}

/** Exposes OPFS temporary archive staging through the shared Rust Web host function names. */
function archiveStagingBridge(): WorkerArchiveStagingBridge {
  return {
    createArchive(archiveId: string, expectedByteLength: number): void {
      requiredArchiveStaging().create(archiveId, expectedByteLength);
    },
    appendArchive(archiveId: string, content: Uint8Array): void {
      requiredArchiveStaging().append(archiveId, content);
    },
    sealArchive(archiveId: string): number {
      return requiredArchiveStaging().seal(archiveId);
    },
    readArchive(archiveId: string, offset: number, length: number): Uint8Array {
      return requiredArchiveStaging().read(archiveId, offset, length);
    },
    removeArchive(archiveId: string): void {
      requiredArchiveStaging().remove(archiveId);
    },
  };
}

/** Builds the legacy browser storage key used by the existing Web host contracts. */
function storageKey(prefix: string, path: string): string {
  const normalized = path.replaceAll("\\", "/").replace(/^\/+/, "");
  return `${prefix}${normalized}`;
}

/** Owns an append-only OPFS data file and a compact persisted index of live records. */
class RuntimeWorkerStorage {
  private readonly records: Map<string, WorkerStorageRecord>;
  private readonly sessions = new Map<string, WorkerWriteSession>();

  private constructor(
    private readonly data: WorkerSyncAccessHandle,
    private readonly index: WorkerSyncAccessHandle,
    records: Map<string, WorkerStorageRecord>,
  ) {
    this.records = records;
  }

  /** Opens the persistent runtime OPFS files and validates the existing metadata index. */
  static async open(root: WorkerDirectoryHandle): Promise<RuntimeWorkerStorage> {
    let data: WorkerSyncAccessHandle | null = null;
    let index: WorkerSyncAccessHandle | null = null;
    try {
      data = await openSyncAccessHandle(root, runtimeStorageDataFileName);
      index = await openSyncAccessHandle(root, runtimeStorageIndexFileName);
      const loaded = readRuntimeStorageIndex(index, data.getSize());
      return new RuntimeWorkerStorage(data, index, loaded.records);
    } catch (error) {
      index?.close();
      data?.close();
      throw error;
    }
  }

  /** Releases the worker-owned OPFS access handles after an unsuccessful initialization. */
  close(): void {
    this.index.close();
    this.data.close();
  }

  /** Reads one exact live record from runtime OPFS storage. */
  read(itemKey: string): Uint8Array {
    const record = this.records.get(itemKey);
    return record === undefined ? new Uint8Array() : readRecord(this.data, record);
  }

  /** Reads one bounded range from an exact live OPFS record. */
  readRange(itemKey: string, offset: number, length: number): Uint8Array {
    const record = this.records.get(itemKey);
    if (record === undefined) {
      throw new Error("runtime OPFS storage file does not exist");
    }
    if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(length) || offset < 0 || length < 0) {
      throw new Error("runtime OPFS storage range is invalid");
    }
    if (offset > record.byteLength) {
      throw new Error("runtime OPFS storage range starts after the file");
    }
    const byteLength = Math.min(length, record.byteLength - offset);
    return readRecord(this.data, { offset: record.offset + offset, byteLength });
  }

  /** Appends and atomically publishes one complete runtime storage value. */
  write(itemKey: string, content: Uint8Array): void {
    this.writeRecord(itemKey, content, true);
  }

  /** Appends bytes to one runtime OPFS record and publishes its new length. */
  append(itemKey: string, content: Uint8Array): void {
    const previous = this.records.get(itemKey);
    const offset = this.data.getSize();
    if (previous !== undefined) {
      writeExact(this.data, readRecord(this.data, previous), offset);
    }
    writeExact(this.data, content, offset + (previous?.byteLength ?? 0));
    this.data.flush();
    this.records.set(itemKey, {
      offset,
      byteLength: (previous?.byteLength ?? 0) + content.byteLength,
    });
    this.persistIndex();
  }

  /** Reports whether one file or virtual directory exists in runtime OPFS storage. */
  exists(itemKey: string): boolean {
    if (this.records.has(itemKey)) {
      return true;
    }
    const directory = itemKey.endsWith("/") ? itemKey : `${itemKey}/`;
    return Array.from(this.records.keys()).some(key => key.startsWith(directory));
  }

  /** Reports whether one exact runtime OPFS key is a persisted file. */
  hasFile(itemKey: string): boolean {
    return this.records.has(itemKey);
  }

  /** Removes one file or all entries under one virtual directory. */
  delete(itemKey: string, recursive: boolean): void {
    const directory = itemKey.endsWith("/") ? itemKey : `${itemKey}/`;
    this.records.delete(itemKey);
    if (recursive) {
      for (const key of Array.from(this.records.keys())) {
        if (key.startsWith(directory)) {
          this.records.delete(key);
        }
      }
    }
    this.persistIndex();
  }

  /** Lists the immediate children of one virtual storage directory. */
  list(prefix: string, path: string): WorkerStorageEntry[] {
    const root = storageKey(prefix, path);
    const directory = root.endsWith(".") || root.endsWith("/") ? root : `${root}/`;
    const entries = new Map<string, WorkerStorageEntry>();
    for (const [itemKey, record] of this.records) {
      if (!itemKey.startsWith(directory)) {
        continue;
      }
      const remainder = itemKey.slice(directory.length);
      const separator = remainder.indexOf("/");
      if (separator < 0) {
        const path = itemKey.slice(prefix.length);
        entries.set(path, { path, isDirectory: false, size: record.byteLength });
        continue;
      }
      const path = `${directory}${remainder.slice(0, separator)}`.slice(prefix.length);
      entries.set(path, { path, isDirectory: true, size: 0 });
    }
    return Array.from(entries.values()).sort((left, right) => left.path.localeCompare(right.path));
  }

  /** Opens one uncommitted sequential writer in the same storage namespace. */
  createWriteSession(itemKey: string): string {
    const sessionId = `runtime-write-${createRandomUuid()}`;
    this.sessions.set(sessionId, {
      key: itemKey,
      offset: this.data.getSize(),
      byteLength: 0,
    });
    return sessionId;
  }

  /** Appends one bounded chunk to an existing uncommitted runtime storage writer. */
  writeSessionChunk(sessionId: string, content: Uint8Array): void {
    const session = this.requiredSession(sessionId);
    writeExact(this.data, content, session.offset + session.byteLength);
    session.byteLength += content.byteLength;
    this.data.flush();
  }

  /** Publishes a fully written runtime storage session into the persistent index. */
  commitWriteSession(sessionId: string): void {
    const session = this.requiredSession(sessionId);
    this.records.set(session.key, {
      offset: session.offset,
      byteLength: session.byteLength,
    });
    this.sessions.delete(sessionId);
    this.persistIndex();
  }

  /** Discards an incomplete runtime storage writer without publishing its bytes. */
  discardWriteSession(sessionId: string): void {
    if (!this.sessions.delete(sessionId)) {
      throw new Error("runtime storage write session does not exist");
    }
  }

  /** Appends bytes to the data file and updates the in-memory record index. */
  private writeRecord(itemKey: string, content: Uint8Array, persist: boolean): void {
    const offset = this.data.getSize();
    writeExact(this.data, content, offset);
    this.data.flush();
    this.records.set(itemKey, { offset, byteLength: content.byteLength });
    if (persist) {
      this.persistIndex();
    }
  }

  /** Returns an active sequential writer by its opaque session identifier. */
  private requiredSession(sessionId: string): WorkerWriteSession {
    const session = this.sessions.get(sessionId);
    if (session === undefined) {
      throw new Error("runtime storage write session does not exist");
    }
    return session;
  }

  /** Persists the compact record index after a visible storage mutation. */
  private persistIndex(): void {
    const serialized = textEncoder.encode(JSON.stringify({
      records: Array.from(this.records.entries()),
    } satisfies WorkerStorageIndex));
    this.index.truncate(0);
    writeExact(this.index, serialized, 0);
    this.index.flush();
  }
}

/** Owns temporary append-only archive bytes for the lifetime of the runtime worker. */
class RuntimeWorkerArchiveStaging {
  private readonly archives = new Map<string, WorkerArchiveRecord>();

  private constructor(private readonly data: WorkerSyncAccessHandle) {}

  /** Opens the temporary OPFS archive container and removes stale abandoned contents. */
  static async open(root: WorkerDirectoryHandle): Promise<RuntimeWorkerArchiveStaging> {
    const data = await openSyncAccessHandle(root, archiveStagingDataFileName);
    try {
      data.truncate(0);
      data.flush();
      return new RuntimeWorkerArchiveStaging(data);
    } catch (error) {
      data.close();
      throw error;
    }
  }

  /** Releases the temporary archive OPFS handle after an unsuccessful initialization. */
  close(): void {
    this.data.close();
  }

  /** Reserves one exact archive range under an opaque runtime-owned identifier. */
  create(archiveId: string, expectedByteLength: number): void {
    validateArchiveId(archiveId);
    if (!Number.isSafeInteger(expectedByteLength) || expectedByteLength < 0) {
      throw new Error("archive staging byte length is invalid");
    }
    if (this.archives.has(archiveId)) {
      throw new Error("archive staging ID already exists");
    }
    const offset = this.data.getSize();
    const end = offset + expectedByteLength;
    if (!Number.isSafeInteger(end)) {
      throw new Error("archive staging capacity exceeds OPFS numeric range");
    }
    this.data.truncate(end);
    this.data.flush();
    this.archives.set(archiveId, {
      offset,
      expectedByteLength,
      byteLength: 0,
      sealed: false,
    });
  }

  /** Appends one ordered upload chunk before the archive becomes immutable. */
  append(archiveId: string, content: Uint8Array): void {
    const archive = this.requiredArchive(archiveId);
    if (archive.sealed) {
      throw new Error("archive staging upload is already sealed");
    }
    if (content.byteLength > archive.expectedByteLength - archive.byteLength) {
      throw new Error("archive staging upload exceeds its declared byte length");
    }
    writeExact(this.data, content, archive.offset + archive.byteLength);
    archive.byteLength += content.byteLength;
    this.data.flush();
  }

  /** Seals an archive and returns its immutable persisted byte length. */
  seal(archiveId: string): number {
    const archive = this.requiredArchive(archiveId);
    if (archive.byteLength !== archive.expectedByteLength) {
      throw new Error("archive staging upload does not match its declared byte length");
    }
    archive.sealed = true;
    this.data.flush();
    return archive.byteLength;
  }

  /** Reads one bounded byte range from a sealed archive. */
  read(archiveId: string, offset: number, length: number): Uint8Array {
    const archive = this.requiredArchive(archiveId);
    if (!archive.sealed) {
      throw new Error("archive staging upload is not sealed");
    }
    if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(length) || offset < 0 || length < 0) {
      throw new Error("archive staging range is invalid");
    }
    if (offset > archive.byteLength) {
      throw new Error("archive staging range starts after the sealed archive");
    }
    const byteLength = Math.min(length, archive.byteLength - offset);
    return readRecord(this.data, { offset: archive.offset + offset, byteLength });
  }

  /** Removes one staged archive and releases the temporary container when it becomes empty. */
  remove(archiveId: string): void {
    if (!this.archives.delete(archiveId)) {
      throw new Error("archive staging ID does not exist");
    }
    if (this.archives.size === 0) {
      this.data.truncate(0);
      this.data.flush();
    }
  }

  /** Returns one existing archive record. */
  private requiredArchive(archiveId: string): WorkerArchiveRecord {
    validateArchiveId(archiveId);
    const archive = this.archives.get(archiveId);
    if (archive === undefined) {
      throw new Error("archive staging ID does not exist");
    }
    return archive;
  }
}

/** Opens one pre-created OPFS file as a synchronous worker-only access handle. */
async function openSyncAccessHandle(
  root: WorkerDirectoryHandle,
  fileName: string,
): Promise<WorkerSyncAccessHandle> {
  const handle = await root.getFileHandle(fileName, { create: true });
  return handle.createSyncAccessHandle();
}

/** Reads and validates the compact runtime storage index. */
function readRuntimeStorageIndex(
  handle: WorkerSyncAccessHandle,
  dataSize: number,
): { records: Map<string, WorkerStorageRecord> } {
  const indexSize = handle.getSize();
  if (indexSize === 0) {
    return { records: new Map() };
  }
  const bytes = new Uint8Array(indexSize);
  if (handle.read(bytes, { at: 0 }) !== indexSize) {
    throw new Error("runtime OPFS index could not be read completely");
  }
  const parsed = JSON.parse(textDecoder.decode(bytes)) as Partial<WorkerStorageIndex>;
  if (!Array.isArray(parsed.records)) {
    throw new Error("runtime OPFS index is invalid");
  }
  const records = new Map<string, WorkerStorageRecord>();
  for (const entry of parsed.records) {
    if (!Array.isArray(entry) || entry.length !== 2 || typeof entry[0] !== "string") {
      throw new Error("runtime OPFS index entry is invalid");
    }
    const record = entry[1] as Partial<WorkerStorageRecord>;
    if (!isValidStorageRecord(record, dataSize)) {
      throw new Error("runtime OPFS index record is invalid");
    }
    records.set(entry[0], { offset: record.offset!, byteLength: record.byteLength! });
  }
  return { records };
}

/** Validates a persisted OPFS record against the current data file size. */
function isValidStorageRecord(value: Partial<WorkerStorageRecord>, dataSize: number): boolean {
  return Number.isSafeInteger(value.offset) &&
    Number.isSafeInteger(value.byteLength) &&
    value.offset! >= 0 &&
    value.byteLength! >= 0 &&
    value.offset! + value.byteLength! <= dataSize;
}

/** Reads one exact byte range from a synchronous OPFS data file. */
function readRecord(handle: WorkerSyncAccessHandle, record: WorkerStorageRecord): Uint8Array {
  const bytes = new Uint8Array(record.byteLength);
  if (record.byteLength > 0 && handle.read(bytes, { at: record.offset }) !== record.byteLength) {
    throw new Error("OPFS data record could not be read completely");
  }
  return bytes;
}

/** Writes one complete byte buffer at an exact OPFS offset. */
function writeExact(handle: WorkerSyncAccessHandle, content: Uint8Array, offset: number): void {
  if (!Number.isSafeInteger(offset) || offset < 0) {
    throw new Error("OPFS write offset is invalid");
  }
  if (content.byteLength > 0 && handle.write(content, { at: offset }) !== content.byteLength) {
    throw new Error("OPFS data record could not be written completely");
  }
}

/** Validates the opaque archive identifier passed through the Core API. */
function validateArchiveId(archiveId: string): void {
  if (!/^[A-Za-z0-9_-]+$/.test(archiveId)) {
    throw new Error("archive staging ID is invalid");
  }
}

/** Posts one structured message to the browser UI thread. */
function postWorkerMessage(
  message: object,
  transferables: Transferable[] = [],
): void {
  const post = workerGlobal.postMessage as unknown as (
    payload: object,
    transfers: Transferable[],
  ) => void;
  post.call(workerGlobal, message, transferables);
}

/** Converts one runtime or host exception into a transport-safe message. */
function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
