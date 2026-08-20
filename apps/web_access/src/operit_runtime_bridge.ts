type JsonValue =
  | null
  | boolean
  | number
  | string
  | Uint8Array
  | JsonValue[]
  | { [key: string]: JsonValue };

type DynamicValue = {
  readonly [key: string]: DynamicValue;
  readonly [index: number]: DynamicValue;
} & {
  readonly length: number;
  new (...args: JsonValue[]): DynamicValue;
  (...args: JsonValue[]): DynamicValue;
  slice(start?: number, end?: number): DynamicValue;
  [Symbol.iterator](): IterableIterator<DynamicValue>;
};

type DynamicRecord = Record<string, DynamicValue>;

declare const MessagePack: {
  encode(value: JsonValue | DynamicValue | object): Uint8Array;
  decode(bytes: Uint8Array): DynamicValue;
};

type SqlValue = null | number | string | Uint8Array;

interface SqlStatement {
  bind(values: SqlValue[]): void;
  step(): boolean;
  get(): SqlValue[];
  getColumnNames(): string[];
  free(): void;
}

interface SqlDatabase {
  exec(sql: string): void;
  run(sql: string, values?: SqlValue[]): void;
  prepare(sql: string): SqlStatement;
  export(): Uint8Array;
  getRowsModified(): number;
  close(): void;
}

interface SqlJsModule {
  Database: new (bytes?: Uint8Array) => SqlDatabase;
}

interface SqlJsFactoryConfig {
  locateFile(file: string): string;
}

type SqlJsFactory = (config: SqlJsFactoryConfig) => Promise<SqlJsModule>;

interface SqliteConnection {
  path: string;
  db: SqlDatabase;
}

interface FileStorageEntry {
  path: string;
  isDirectory: boolean;
  size: number;
}

type SqliteParameter =
  | { kind: "null" }
  | { kind: "integer"; value: string }
  | { kind: "real"; value: number }
  | { kind: "text"; value: string }
  | { kind: "blob"; value: Uint8Array };

type SqliteSerializedValue = SqliteParameter;

interface SqliteQueryRow {
  columns: string[];
  values: SqliteSerializedValue[];
}

interface RuntimeStoragePaths {
  runtimeRoot: string;
  workspaceRoot: string;
}

interface RuntimeBridge {
  /** Returns the platform default logical Runtime storage roots. */
  runtimeStorageDefaults(): Promise<RuntimeStoragePaths>;
  /** Normalizes explicit logical Runtime storage roots. */
  runtimeStoragePaths(runtimeRoot: string, workspaceRoot: string): Promise<RuntimeStoragePaths>;
  /** Installs identity-isolated logical Runtime storage roots. */
  setRuntimeStorageRoots(runtimeRoot: string, workspaceRoot: string): Promise<void>;
  /** Reads the client bootstrap record outside active Runtime storage. */
  runtimeBootstrapRead(): Promise<string>;
  /** Writes the client bootstrap record outside active Runtime storage. */
  runtimeBootstrapWrite(content: string): Promise<void>;
  /** Reloads the browser after selecting another identity. */
  restartApplication(): Promise<void>;
  call(request: Uint8Array): Promise<Uint8Array>;
  controlCall(request: Uint8Array): Promise<Uint8Array>;
  pushOpen(request: Uint8Array): Promise<Uint8Array>;
  pushItem(item: Uint8Array): Promise<Uint8Array>;
  pushClose(pushId: string): Promise<Uint8Array>;
  watchSnapshot(request: Uint8Array): Promise<Uint8Array>;
  watchStream(request: Uint8Array, onEvent: (event: Uint8Array) => void): Promise<Uint8Array>;
  closeWatchStream(subscriptionId: string): Promise<Uint8Array>;
}

interface WasmBridgeModule {
  default(options: { module_or_path: string }): Promise<{ memory: WebAssembly.Memory }>;
  OperitFlutterBridgeWasm: new () => RuntimeBridge;
}

interface WasiModule {
  setWasiMemory(memory: WebAssembly.Memory): void;
}

interface WebAccessSession {
  baseUrl: string;
  sessionId: string;
  deviceId: string;
  coreDeviceId: string;
  remoteDeviceInfo: JsonValue;
  pairingServiceVersion: number;
  sessionSecret: string;
}

interface WatchChannel {
  channelId: string;
  controller: AbortController;
  subscriptionCount: number;
}

interface LinkPushOpenRequest {
  requestId: DynamicValue;
  targetPath: { segments: DynamicValue };
  methodName: DynamicValue;
  args: DynamicValue;
}

interface PairStartResponse {
  pairingId: string;
  corePublicKey: string;
  serverNonce: string;
  coreDeviceId: string;
  coreDeviceInfo: JsonValue;
}

interface PairFinishResponse {
  sessionId: string;
  coreProof: string;
  pairingServiceVersion: number;
}

interface WebDeviceInfo {
  platform: string;
  model: string;
}

interface BrowserNotificationActivation {
  type: "open_application" | "open_chat";
  chatId?: string;
}

interface RuntimeBootstrapIdentity {
  id: string;
  name: string;
  createdAt: number;
}

interface RuntimeBootstrapRecord {
  confirmed: boolean;
  runtimeRoot: string;
  workspaceRoot: string;
  identities: RuntimeBootstrapIdentity[];
  activeIdentityId: string;
  updatedAt: number;
}

/** Parses the notification destination supplied by the Web system-operation host. */
function parseBrowserNotificationActivation(activationJson: string): BrowserNotificationActivation {
  const activation = JSON.parse(activationJson) as { type?: unknown; chatId?: unknown };
  if (activation.type === "open_application") {
    return { type: activation.type };
  }
  if (activation.type === "open_chat" && typeof activation.chatId === "string" && activation.chatId.length > 0) {
    return { type: activation.type, chatId: activation.chatId };
  }
  throw new Error("Browser notification activation is invalid");
}

/** Builds the browser URL that restarts Flutter with a selected notification destination. */
function browserNotificationActivationUrl(
  activation: BrowserNotificationActivation,
  activationJson: string,
): string | null {
  if (activation.type === "open_application") {
    return null;
  }
  const target = new URL(window.location.href);
  target.searchParams.set("operitNotificationActivation", activationJson);
  return target.toString();
}

/** Displays a browser notification and forwards clicks to the selected application destination. */
function showBrowserNotification(
  title: string,
  message: string,
  activation: BrowserNotificationActivation,
  activationJson: string,
): void {
  const notification = new Notification(title, { body: message, tag: "operit" });
  notification.onclick = (): void => {
    notification.close();
    window.focus();
    const target = browserNotificationActivationUrl(activation, activationJson);
    if (target !== null) {
      window.location.assign(target);
    }
  };
}

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

interface WebTtsBundle {
  worker: Worker;
  numSpeakers: number;
  sampleRate: number;
}

interface AsrBundlePaths {
  recognizerScript: string;
  runtimeScript: string;
  runtimeWasm: string;
  runtimeData: string;
}

interface TtsBundlePaths {
  ttsScript: string;
  runtimeScript: string;
  runtimeWasm: string;
  runtimeData: string;
}

interface WebTtsWorkerReady {
  numSpeakers: number;
  sampleRate: number;
}

interface AsrSpeechRequest {
  driverJson: string;
  modelDirectory: string;
  audioPath: string;
}

interface TtsSpeechRequest {
  driverJson: string;
  modelDirectory: string;
  voice: string;
  text: string;
  speed: number;
  outputPath: string;
}

interface SherpaAsrDriver {
  recognizerScript: string;
}

interface SherpaTtsDriver {
  ttsScript: string;
  speakerCount: number;
}

interface SherpaStream {
  acceptWaveform(sampleRate: number, samples: Float32Array): void;
  free(): void;
}

interface SherpaRecognizer {
  createStream(): SherpaStream;
  decode(stream: SherpaStream): void;
  getResult(stream: SherpaStream): { text?: string; [key: string]: JsonValue | undefined };
  free(): void;
}

interface SherpaModuleConfig {
  mainScriptUrlOrBlob?: string;
  locateFile?: (path: string) => string;
  setStatus?: (status: string) => void;
  onRuntimeInitialized?: () => void;
  onAbort?: (reason: string) => void;
}

interface SherpaAsrClasses {
  OfflineRecognizer: new (
    config: { modelConfig: JsonValue },
    module: SherpaModuleConfig,
  ) => SherpaRecognizer;
}

interface WebAsrBundle {
  recognizer: SherpaRecognizer;
  moduleValue: SherpaModuleConfig;
}

interface WebLocalInferenceState {
  asrBundles: Map<string, WebAsrBundle>;
  ttsBundles: Map<string, WebTtsBundle>;
  blobUrls: string[];
}

interface WebLocalInferenceRunner {
  transcribeLocalSpeech(requestJson: string): string;
  synthesizeLocalSpeech(requestJson: string): string;
}

interface MusicRequest {
  sourceType: string;
  source: string;
  title?: string | null;
  artist?: string | null;
  loopPlayback?: boolean;
  volume?: number;
  startPositionMs?: number;
}

interface BluetoothCharacteristicProperties {
  read: boolean;
  write: boolean;
  writeWithoutResponse: boolean;
  notify: boolean;
  indicate: boolean;
}

interface BluetoothCharacteristic {
  uuid: string;
  properties: BluetoothCharacteristicProperties;
  readValue(): Promise<DataView>;
  writeValue(value: Uint8Array): Promise<void>;
  startNotifications(): Promise<BluetoothCharacteristic>;
  stopNotifications(): Promise<BluetoothCharacteristic>;
  addEventListener(
    type: "characteristicvaluechanged",
    listener: (event: { target: { value: DataView } }) => void,
  ): void;
}

interface BluetoothService {
  uuid: string;
  getCharacteristics(): Promise<BluetoothCharacteristic[]>;
}

interface BluetoothServer {
  getPrimaryServices(): Promise<BluetoothService[]>;
}

interface BluetoothGatt {
  connected: boolean;
  connect(): Promise<BluetoothServer>;
  disconnect(): void;
}

interface BluetoothDevice {
  id: string;
  name?: string;
  gatt: BluetoothGatt;
}

interface BluetoothApi {
  requestDevice(options: {
    acceptAllDevices: boolean;
    optionalServices?: string[];
  }): Promise<BluetoothDevice>;
}

interface BleSession {
  device: BluetoothDevice;
  server: BluetoothServer;
  characteristics: Map<string, BluetoothCharacteristic>;
}

interface BleNotification {
  characteristicUuid: string;
  bytesRead: number;
  text: string;
  dataBase64: string;
  timestamp: number;
}

interface BluetoothReadAddress {
  sessionId: string;
  serviceUuid: string;
  characteristicUuid: string;
}

interface BluetoothWriteRequest extends BluetoothReadAddress {
  text?: string;
  dataBase64?: string;
}

interface BluetoothWriteAndReadRequest {
  sessionId: string;
  writeServiceUuid: string;
  writeCharacteristicUuid: string;
  readServiceUuid: string;
  readCharacteristicUuid: string;
  text?: string;
  dataBase64?: string;
}

interface BluetoothSubscribeRequest extends BluetoothReadAddress {
  enable: boolean;
}

type HttpHeader = [string, string] | { key: string; value: string };

interface HttpFilePart {
  fieldName: string;
  content: Uint8Array;
  contentType: string;
  fileName: string;
}

interface HttpRequest {
  method: string;
  url: string;
  headers?: HttpHeader[];
  formFields?: HttpHeader[];
  fileParts?: HttpFilePart[];
  body?: Uint8Array;
  followRedirects: boolean;
}

interface DownloadRequest {
  fileId: string;
  url: string;
  headers?: HttpHeader[];
  expectedBytes?: number;
  targetPath: string;
}

interface V86StarterConfiguration {
  wasm_path: string;
  memory_size: number;
  vga_memory_size: number;
  bios: { url: string };
  vga_bios: { url: string };
  bzimage: { url: string };
  initrd?: { url: string };
  cmdline: string;
  autostart: boolean;
  disable_keyboard: boolean;
  disable_mouse: boolean;
  disable_speaker: boolean;
}

interface V86StarterInstance {
  add_listener(event: string, listener: (value: unknown) => void): void;
  serial_send_bytes(serial: number, data: Uint8Array): void;
  destroy(): Promise<void>;
}

interface V86Module {
  V86: new (configuration: V86StarterConfiguration) => V86StarterInstance;
}

type LinuxVmSessionState = "starting" | "running" | "failed" | "closed";

interface LinuxVmSession {
  readonly id: string;
  emulator: V86StarterInstance | null;
  state: LinuxVmSessionState;
  exitCode: number | null;
  rows: number;
  cols: number;
  output: Uint8Array;
  outputLength: number;
  inputQueue: Uint8Array[];
  startupText: string;
  lastDownloadProgress: string | null;
  progressVisible: boolean;
  activeCommand: LinuxVmCommand | null;
  pendingCommands: LinuxVmCommand[];
}

interface LinuxVmCommandResult {
  output: string;
  exitCode: number;
  timedOut: boolean;
  workingDir: string;
}

interface LinuxVmCommand {
  readonly token: string;
  readonly command: string;
  readonly timeoutHandle: number;
  readonly resolve: (result: LinuxVmCommandResult) => void;
  readonly reject: (reason: unknown) => void;
  collectingOutput: boolean;
  markerBytes: number[];
  output: Uint8Array;
  outputLength: number;
}

interface ManagedRuntimeRequest {
  program: string;
  executablePath?: string | null;
  args: string[];
  cwd?: string | null;
  env: Record<string, string>;
}

interface ManagedRuntimeProcess {
  readonly id: string;
  readonly worker: Worker;
  readonly header: Int32Array;
  readonly output: Uint8Array;
  readonly decoder: TextDecoder;
  activityVersion: number;
  activityWaiters: Set<() => void>;
  stdout: string;
  stderr: string;
}

interface RuntimeGlobals {
  __OPERIT_MODEL_INSTALL_WORKER__?: boolean;
  __OPERIT_RUNTIME_WORKER__?: boolean;
  __operitRuntime?: RuntimeBridge;
  __operitRuntimeWorkerEnsureStorage?: () => Promise<void>;
  __operitRuntimeWorkerStorage?: RuntimeWorkerStorageBridge;
  __operitRuntimeWorkerArchiveStaging?: RuntimeWorkerArchiveStagingBridge;
  __operitWorkerHostModules?: WeakSet<object>;
  __operitMainHostModules?: WeakSet<object>;
  __operitModelInstallWorkerStorageChanges?: () => Promise<ModelInstallWorkerStorageChange[]>;
  __operitModelInstallWorkerSetSecrets?: (secrets: ModelInstallWorkerSecret[]) => void;
  __operitModelInstallWorkerSetDownloads?: (downloads: ModelInstallWorkerDownload[]) => void;
  __operitModelInstallWorkerDownloadRequests?: () => DownloadRequest[];
  __operitModelInstallWorkerSecretChanges?: () => ModelInstallWorkerSecretChange[];
  initSqlJs?: SqlJsFactory;
  Module?: SherpaModuleConfig;
  __operitSherpaAsrClasses?: SherpaAsrClasses;
  __operitLocalInference?: WebLocalInferenceRunner;
  __OPERIT_LOCAL_INFERENCE_TEST__?: boolean;
  __operitLocalInferenceTest?: object;
  __operitHost?: object;
  __operitHttpDownloadManager?: {
    list(): Promise<HttpDownloadStatus[]>;
    pause(url: string): void;
    delete(url: string): Promise<void>;
  };
}

interface RuntimeWorkerStorageBridge {
  read(prefix: string, path: string): Uint8Array;
  readRange(prefix: string, path: string, offset: number, length: number): Uint8Array;
  write(prefix: string, path: string, content: Uint8Array): void;
  append(prefix: string, path: string, content: Uint8Array): void;
  hasFile(prefix: string, path: string): boolean;
  exists(prefix: string, path: string): boolean;
  delete(prefix: string, path: string, recursive: boolean): void;
  list(prefix: string, path: string): FileStorageEntry[];
  createWriteSession(path: string): string;
  writeSessionChunk(sessionId: string, content: Uint8Array): void;
  commitWriteSession(sessionId: string): void;
  discardWriteSession(sessionId: string): void;
}

interface RuntimeWorkerArchiveStagingBridge {
  createArchive(archiveId: string, expectedByteLength: number): void;
  appendArchive(archiveId: string, content: Uint8Array): void;
  sealArchive(archiveId: string): number;
  readArchive(archiveId: string, offset: number, length: number): Uint8Array;
  removeArchive(archiveId: string): void;
}

interface RuntimeWorkerCoreRequest {
  type: "coreRequest";
  id: number;
  operation: "call" | "controlCall" | "pushOpen" | "pushItem" | "pushClose" | "watchSnapshot" | "watchStream" | "closeWatchStream";
  payload: Uint8Array | string;
}

interface RuntimeWorkerCoreResult {
  type: "coreResult";
  id: number;
  response: Uint8Array;
}

interface RuntimeWorkerCoreError {
  type: "coreError";
  id: number;
  message: string;
}

interface RuntimeWorkerWatchEvent {
  type: "coreWatchEvent";
  id: number;
  event: Uint8Array;
}

interface RuntimeWorkerHostCall {
  type: "hostCall";
  id: number;
  module: string;
  method: string;
  args: unknown[];
  control: SharedArrayBuffer;
}

interface RuntimeWorkerHostPayload {
  type: "hostPayload";
  id: number;
  payload: SharedArrayBuffer;
  control: SharedArrayBuffer;
}

interface RuntimeWorkerPendingRequest {
  resolve(response: Uint8Array): void;
  reject(error: Error): void;
}

interface RuntimeWorkerPendingCommand {
  operation: RuntimeWorkerCoreRequest["operation"];
  payload: Uint8Array | string;
}

interface RuntimeWorkerLeaderRequest {
  type: "coreRequest";
  clientId: string;
  clientRequestId: number;
  operation: RuntimeWorkerCoreRequest["operation"];
  payload: Uint8Array | string;
}

interface RuntimeWorkerLeaderRequestRoute {
  clientId: string;
  clientRequestId: number;
  operation: RuntimeWorkerCoreRequest["operation"];
  payload: Uint8Array | string;
  disconnected: boolean;
}

interface RuntimeWorkerLeaderHostCallRoute {
  clientId: string;
  control: SharedArrayBuffer;
}

interface RuntimeWorkerLeaderMessage {
  type?: string;
  id?: number;
  clientId?: string;
  clientRequestId?: number;
  operation?: RuntimeWorkerCoreRequest["operation"];
  payload?: Uint8Array | string | SharedArrayBuffer;
  response?: Uint8Array;
  event?: Uint8Array;
  message?: unknown;
  module?: string;
  method?: string;
  args?: unknown[];
  control?: SharedArrayBuffer;
}

interface RuntimeWorkerShutdownComplete {
  type: "shutdownComplete";
}

interface RuntimeWorkerLockManager {
  request(name: string, options: { mode: "exclusive" }, callback: () => Promise<void>): Promise<void>;
}

interface ModelInstallWorkerStorageChange {
  key: string;
  bytes: Uint8Array | null;
}

interface ModelInstallWorkerSecret {
  key: string;
  bytes: Uint8Array;
}

interface ModelInstallWorkerSecretChange {
  key: string;
  bytes: Uint8Array | null;
}

interface ModelInstallWorkerDownload {
  url: string;
  bytes: Uint8Array;
}

interface ModelInstallWorkerResult {
  type: "result";
  response: Uint8Array;
  changes: ModelInstallWorkerStorageChange[];
  secretChanges: ModelInstallWorkerSecretChange[];
}

interface ModelInstallWorkerDownloadRequests {
  type: "downloadRequests";
  requests: DownloadRequest[];
}

interface PersistedHttpDownload {
  url: string;
  fileId: string;
  expectedBytes: number;
  downloadedBytes: number;
  content: Blob;
  modelId: string;
  version: string;
  paused: boolean;
}

interface HttpDownloadStatus {
  url: string;
  fileId: string;
  expectedBytes: number;
  downloadedBytes: number;
  active: boolean;
  modelId: string;
  version: string;
  paused: boolean;
}

interface LocalModelIdentity {
  modelId: string;
  version: string;
}

interface LocalModelDownloadCallResult {
  handled: boolean;
  response: Uint8Array;
}

interface ModelInstallWorkerError {
  type: "error";
  message: string;
}

(function () {
  const runtimeGlobal = globalThis as typeof globalThis & RuntimeGlobals;
  const workerHostModules = new WeakSet<object>();
  const mainHostModules = new WeakSet<object>();
  runtimeGlobal.__operitWorkerHostModules = workerHostModules;
  runtimeGlobal.__operitMainHostModules = mainHostModules;
  const browserNavigator = navigator as Navigator & {
    bluetooth?: BluetoothApi;
  };
  const importRuntimeScript = (path: string): Promise<object> => import(path);
  /** Creates a Blob-compatible copy of browser-hosted bytes. */
  const blobPart = (bytes: Uint8Array): BlobPart => Uint8Array.from(bytes);
  /** Creates an ArrayBuffer-backed byte view for Web Crypto and Fetch. */
  const ownedBytes = (bytes: Uint8Array): Uint8Array<ArrayBuffer> => Uint8Array.from(bytes);

  /** Declares that one host module executes inside the dedicated runtime worker. */
  function registerWorkerHostModule<T extends object>(module: T): T {
    workerHostModules.add(module);
    return module;
  }

  /** Declares that one host module executes through the browser UI thread. */
  function registerMainHostModule<T extends object>(module: T): T {
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
  const storageCache = new Map<string, Uint8Array>();
  const workerChangedStorageKeys = new Set<string>();
  const workerSecrets = new Map<string, Uint8Array>();
  const workerDownloads = new Map<string, Uint8Array>();
  const workerDownloadRequests = new Map<string, DownloadRequest>();
  const workerChangedSecretKeys = new Set<string>();
  const fileDirectories = new Set<string>();
  const sqliteConnections = new Map<string, SqliteConnection>();
  const sqliteTransactions = new Map<string, SqliteConnection>();
  let sqliteConnectionIndex = 0;
  let sqliteTransactionIndex = 0;
  let sqliteModulePromise: Promise<void> | null = null;
  let SQLite: SqlJsModule | null = null;
  let storageDatabasePromise: Promise<IDBDatabase> | null = null;
  let httpDownloadDatabasePromise: Promise<IDBDatabase> | null = null;
  let httpDownloadStatusCachePromise: Promise<void> | null = null;
  const httpDownloadStatusCache = new Map<string, HttpDownloadStatus>();
  const activeHttpDownloadControllers = new Map<string, AbortController>();
  const activeHttpByteStreamControllers = new Map<string, AbortController>();
  const activeHttpDownloadPromises = new Map<string, Promise<ModelInstallWorkerDownload>>();
  const activeModelInstallAborters = new Map<string, () => void>();
  const activeModelInstallPromises = new Map<string, Promise<Uint8Array>>();
  const modelInstallTaskGenerations = new Map<string, number>();
  let modelInstallTaskGeneration = 0;
  let modelInstallCommitQueue: Promise<void> = Promise.resolve();
  let storageReadyPromise: Promise<void> | null = null;
  let webLocalInferenceReadyPromise: Promise<void> | null = null;
  let webLocalInferenceState: WebLocalInferenceState | null = null;
  const linuxVmSessions = new Map<string, LinuxVmSession>();
  const linuxVmOutputLimit = 4 * 1024 * 1024;
  let linuxVmCommandIndex = 0;
  const managedRuntimeProcesses = new Map<string, ManagedRuntimeProcess>();
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

  const webAccessSessionStorageKey = `${runtimeIdentityNamespace}.webAccess.session`;
  const pairingServiceVersion = 1;
  let webAccessSessionReloading = false;
  const webAccessUrl = new URL(globalThis.location.href);
  const webAccessToken = webAccessUrl.searchParams.get("token");
  if (webAccessToken !== null) {
    installPairingWebRuntime(webAccessUrl.origin, webAccessToken.trim());
    return;
  }

  /** Resolves the active identity from a worker URL or the browser bootstrap record. */
  function resolveRuntimeIdentityId(): string {
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

  /** Creates and persists the first browser identity before the local Runtime starts. */
  function createInitialRuntimeIdentity(): string {
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

  /** Validates one identity identifier before it enters browser storage names. */
  function validateRuntimeIdentityId(identityId: string): void {
    if (!/^identity-[a-z0-9-]+$/.test(identityId)) {
      throw new Error(`Web runtime identity is invalid: ${identityId}`);
    }
  }

  /** Parses and validates one browser bootstrap record before it changes identity ownership. */
  function parseRuntimeBootstrapConfig(encoded: string): RuntimeBootstrapRecord {
    const parsed = JSON.parse(encoded) as Partial<RuntimeBootstrapRecord>;
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
    const identityIds = new Set<string>();
    const identities = parsed.identities.map(value => {
      if (typeof value !== "object" || value === null) {
        throw new Error("Web runtime bootstrap identity is invalid");
      }
      const identity = value as Partial<RuntimeBootstrapIdentity>;
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
      return identity as RuntimeBootstrapIdentity;
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

  /** Reads the browser bootstrap record after validating its complete structure. */
  function readRuntimeBootstrapConfig(): string {
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

  /** Returns the browser Host's default logical Runtime storage roots. */
  function defaultRuntimeStoragePaths(): RuntimeStoragePaths {
    return {
      runtimeRoot: "browser-storage/runtime",
      workspaceRoot: "browser-storage/workspaces",
    };
  }

  /** Validates and returns explicit logical Runtime storage roots. */
  function normalizeRuntimeStoragePaths(runtimeRoot: string, workspaceRoot: string): RuntimeStoragePaths {
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

  /** Accepts the identity-isolated logical roots selected before browser Runtime startup. */
  function installRuntimeStoragePaths(runtimeRoot: string, workspaceRoot: string): void {
    normalizeRuntimeStoragePaths(runtimeRoot, workspaceRoot);
  }

  /** Writes one validated browser bootstrap record before application restart. */
  function writeRuntimeBootstrapConfig(content: string): void {
    parseRuntimeBootstrapConfig(content);
    localStorage.setItem(runtimeStorageConfigKey, content);
  }

  /** Reloads the browser after a bootstrap identity selection changes. */
  function restartRuntimeApplication(): void {
    globalThis.location.reload();
  }

  /** Builds one identity-bearing worker URL for isolated background execution. */
  function runtimeIdentityWorkerUrl(path: string): URL {
    const url = new URL(path, import.meta.url);
    url.searchParams.set("identity", runtimeIdentityId);
    return url;
  }

  /** Installs the remote runtime used by a token-bearing Web Access URL. */
  function installPairingWebRuntime(baseUrl: string, token: string): void {
    if (token.length === 0) {
      throw new Error("web access URL requires a token query parameter");
    }
    const runtimePromise = webAccessSession(baseUrl, token).then(createLinkedWebRuntime);
    runtimeGlobal.__operitRuntime = {
      /** Returns local storage roots without waiting for the remote Link session. */
      async runtimeStorageDefaults(): Promise<RuntimeStoragePaths> {
        return defaultRuntimeStoragePaths();
      },
      /** Normalizes local storage roots without waiting for the remote Link session. */
      async runtimeStoragePaths(runtimeRoot: string, workspaceRoot: string): Promise<RuntimeStoragePaths> {
        return normalizeRuntimeStoragePaths(runtimeRoot, workspaceRoot);
      },
      /** Installs local storage roots without waiting for the remote Link session. */
      async setRuntimeStorageRoots(runtimeRoot: string, workspaceRoot: string): Promise<void> {
        installRuntimeStoragePaths(runtimeRoot, workspaceRoot);
      },
      /** Reads local bootstrap state without waiting for the remote Link session. */
      async runtimeBootstrapRead(): Promise<string> {
        return readRuntimeBootstrapConfig();
      },
      /** Writes local bootstrap state without waiting for the remote Link session. */
      async runtimeBootstrapWrite(content: string): Promise<void> {
        writeRuntimeBootstrapConfig(content);
      },
      /** Reloads the local browser after an identity change. */
      async restartApplication(): Promise<void> {
        restartRuntimeApplication();
      },
      async call(request: Uint8Array): Promise<Uint8Array> {
        return (await runtimePromise).call(request);
      },
      async controlCall(request: Uint8Array): Promise<Uint8Array> {
        return (await runtimePromise).controlCall(request);
      },
      async pushOpen(request: Uint8Array): Promise<Uint8Array> {
        return (await runtimePromise).pushOpen(request);
      },
      async pushItem(item: Uint8Array): Promise<Uint8Array> {
        return (await runtimePromise).pushItem(item);
      },
      async pushClose(pushId: string): Promise<Uint8Array> {
        return (await runtimePromise).pushClose(pushId);
      },
      async watchSnapshot(request: Uint8Array): Promise<Uint8Array> {
        return (await runtimePromise).watchSnapshot(request);
      },
      async watchStream(
        request: Uint8Array,
        onEvent: (event: Uint8Array) => void,
      ): Promise<Uint8Array> {
        return (await runtimePromise).watchStream(request, onEvent);
      },
      async closeWatchStream(subscriptionId: string): Promise<Uint8Array> {
        return (await runtimePromise).closeWatchStream(subscriptionId);
      },
    };
  }

  /** Restores or creates the browser session for a token-bearing Web Access link. */
  async function webAccessSession(baseUrl: string, token: string): Promise<WebAccessSession> {
    const persistedSession = readPersistedWebAccessSession(baseUrl);
    if (persistedSession !== null) {
      return persistedSession;
    }
    const session = await pairWebAccessSession(baseUrl, token);
    localStorage.setItem(webAccessSessionStorageKey, JSON.stringify(session));
    return session;
  }

  /** Reads and validates the persisted Web Access session for one server origin. */
  function readPersistedWebAccessSession(baseUrl: string): WebAccessSession | null {
    const serialized = localStorage.getItem(webAccessSessionStorageKey);
    if (serialized === null) {
      return null;
    }
    const session = JSON.parse(serialized) as WebAccessSession;
    if (session.baseUrl !== baseUrl) {
      return null;
    }
    if (
      typeof session.sessionId !== "string" ||
      session.sessionId.length === 0 ||
      typeof session.deviceId !== "string" ||
      session.deviceId.length === 0 ||
      typeof session.sessionSecret !== "string" ||
      session.sessionSecret.length === 0 ||
      typeof session.coreDeviceId !== "string" ||
      session.coreDeviceId.length === 0 ||
      typeof session.pairingServiceVersion !== "number"
    ) {
      throw new Error("persisted Web Access session is invalid");
    }
    return session;
  }

  /** Clears the active browser session and reloads the Web application. */
  function resetWebAccessSession() {
    localStorage.removeItem(webAccessSessionStorageKey);
    if (!webAccessSessionReloading) {
      webAccessSessionReloading = true;
      globalThis.location.reload();
    }
  }

  async function pairWebAccessSession(baseUrl: string, token: string): Promise<WebAccessSession> {
    const keyPair = await crypto.subtle.generateKey(
      { name: "X25519" },
      true,
      ["deriveBits"],
    ) as CryptoKeyPair;
    const clientPublicKey = bytesToBase64(
      new Uint8Array(await crypto.subtle.exportKey("raw", keyPair.publicKey)),
    );
    const clientDeviceId = `web-client-${createRandomUuid()}`;
    const clientNonce = createRandomUuid();
    const start = await postJson<PairStartResponse>(`${baseUrl}/link/pair/start`, {
      pairingServiceVersion,
      tokenHash: await linkTokenHash(token),
      clientDeviceId,
      clientDeviceInfo: webDeviceInfo(),
      clientPublicKey,
      clientNonce,
    });
    const corePublicKey = await crypto.subtle.importKey(
      "raw",
      ownedBytes(base64ToBytes(start.corePublicKey)),
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
    let finish: PairFinishResponse;
    while (true) {
      const pairingCode = globalThis.prompt("Operit Web Access pairing code");
      if (pairingCode === null || pairingCode.trim().length === 0) {
        throw new Error("web access pairing code is required");
      }
      try {
        finish = await postJson<PairFinishResponse>(`${baseUrl}/link/pair/finish`, {
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

  async function postJson<T>(url: string, body: object): Promise<T> {
    const response = await fetch(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    const text = await response.text();
    if (!response.ok) {
      throw new Error(text);
    }
    return JSON.parse(text) as T;
  }

  function webDeviceInfo(): WebDeviceInfo {
    return {
      platform: navigator.platform,
      model: browserName(navigator.userAgent),
    };
  }

  function browserName(userAgent: string): string {
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

  async function proof(
    sharedSecret: Uint8Array,
    clientNonce: string,
    serverNonce: string,
    role: string,
  ): Promise<string> {
    return bytesToBase64(
      new Uint8Array(
        await crypto.subtle.digest(
          "SHA-256",
            ownedBytes(concatBytes(
              sharedSecret,
              textEncoder.encode(clientNonce),
              textEncoder.encode(serverNonce),
              textEncoder.encode(role),
            )),
        ),
      ),
    );
  }

  async function linkTokenHash(token: string): Promise<string> {
    return bytesToBase64(
      new Uint8Array(
        await crypto.subtle.digest("SHA-256", textEncoder.encode(token)),
      ),
    );
  }

  async function sessionSecret(
    sharedSecret: Uint8Array,
    clientNonce: string,
    serverNonce: string,
  ): Promise<string> {
    return bytesToBase64(
      new Uint8Array(
        await crypto.subtle.digest(
          "SHA-256",
          ownedBytes(concatBytes(
            sharedSecret,
            textEncoder.encode(clientNonce),
            textEncoder.encode(serverNonce),
            textEncoder.encode("session"),
          )),
        ),
      ),
    );
  }

  function concatBytes(...parts: Uint8Array[]): Uint8Array {
    const length = parts.reduce((sum, part) => sum + part.length, 0);
    const output = new Uint8Array(length);
    let offset = 0;
    for (const part of parts) {
      output.set(part, offset);
      offset += part.length;
    }
    return output;
  }

  async function createLinkedWebRuntime(config: WebAccessSession): Promise<RuntimeBridge> {
    const baseUrl = String(config.baseUrl || "").replace(/\/+$/, "");
    const sessionId = String(config.sessionId);
    const deviceId = String(config.deviceId);
    const sessionSecret = String(config.sessionSecret);
    const streamCallbacks = new Map<string, (event: Uint8Array) => void>();
    const streamChannels = new Map<string, string>();
    const channels = new Map<string, WatchChannel>();
    let hmacKeyPromise: Promise<CryptoKey> | null = null;
    let openingChannelPromise: Promise<WatchChannel> | null = null;
    const maxSubscriptionsPerChannel = 16;
    let pushSocketPromise: Promise<WebSocket> | null = null;
    let pushSendTail: Promise<void> = Promise.resolve();
    let pushError: Error | null = null;

    function linkPath(path: string): string {
      return `${baseUrl}${path}`;
    }

    async function hmacKey(): Promise<CryptoKey> {
      if (!hmacKeyPromise) {
        hmacKeyPromise = crypto.subtle.importKey(
          "raw",
          ownedBytes(base64ToBytes(sessionSecret)),
          { name: "HMAC", hash: "SHA-256" },
          false,
          ["sign"],
        );
      }
      return hmacKeyPromise;
    }

    /** Encodes one MessagePack body into an exact-length byte buffer. */
    function encodeLinkBody(body: JsonValue | DynamicValue | object): Uint8Array {
      return MessagePack.encode(body).slice();
    }

    async function linkHeaders(bodyBytes: Uint8Array): Promise<Record<string, string>> {
      const signature = await crypto.subtle.sign(
        "HMAC",
        await hmacKey(),
        ownedBytes(bodyBytes),
      );
      return {
        "content-type": "application/msgpack",
        "x-operit-link-version": "3",
        "x-operit-session": sessionId,
        "x-operit-device": deviceId,
        "x-operit-signature": bytesToBase64(new Uint8Array(signature)),
      };
    }

    async function postLink(
      path: string,
      body: JsonValue | DynamicValue | object,
      signal: AbortSignal | undefined = undefined,
    ): Promise<Uint8Array> {
      const bodyBytes = encodeLinkBody(body);
      const response = await fetch(linkPath(path), {
        method: "POST",
        headers: await linkHeaders(bodyBytes),
        body: ownedBytes(bodyBytes),
        signal,
      });
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (!response.ok) {
        throwLinkErrorResponse(response.status, bytes);
      }
      return bytes;
    }

    /** Opens the authenticated binary carrier used by client-owned push streams. */
    function pushSocket(): Promise<WebSocket> {
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
            if (String(response.type) === "Error") {
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
    function sendPushPayload(payload: JsonValue | DynamicValue | object): Promise<void> {
      pushSendTail = pushSendTail.then(async () => {
        if (pushError !== null) throw pushError;
        const bodyBytes = encodeLinkBody(payload);
        const signature = await crypto.subtle.sign("HMAC", await hmacKey(), ownedBytes(bodyBytes));
        const socket = await pushSocket();
        socket.send(ownedBytes(encodeLinkBody({
          protocolVersion: 3,
          sessionId,
          deviceId,
          signature: bytesToBase64(new Uint8Array(signature)),
          payloadBytes: bodyBytes,
        })));
      });
      return pushSendTail;
    }

    /** Returns whether the link error requests a saved Web Access session reset. */
    function shouldResetWebAccessSession(status: number, error: DynamicValue): boolean {
      const details = error.details;
      if (
        status !== 401 ||
        String(error.code) !== "UNAUTHORIZED" ||
        details === null ||
        typeof details !== "object"
      ) {
        return false;
      }
      const authenticatedDetails = details as DynamicValue;
      return String(authenticatedDetails.type) === "remote_session_auth" &&
        String(authenticatedDetails.resetWebAccessSession) === "true";
    }

    /** Decodes and throws one MessagePack Link error response. */
    function throwLinkErrorResponse(status: number, bytes: Uint8Array): never {
      const error = MessagePack.decode(bytes);
      if (shouldResetWebAccessSession(status, error)) {
        resetWebAccessSession();
      }
      throw new Error(`${error.code}: ${error.message}`);
    }

    async function openChannel(): Promise<WatchChannel> {
      const channelId = `watch-channel-${createRandomUuid()}`;
      const controller = new AbortController();
      const channel = {
        channelId,
        controller,
        subscriptionCount: 0,
      };
      const body = { channelId };
      const bodyBytes = encodeLinkBody(body);
      const response = await fetch(linkPath("/link/watch/channel/events"), {
        method: "POST",
        headers: await linkHeaders(bodyBytes),
        body: ownedBytes(bodyBytes),
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

    async function readWatchChannel(channel: WatchChannel, response: Response): Promise<void> {
      if (response.body === null) {
        throw new Error("Link watch channel response has no body");
      }
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
            const subscriptionId = String(event.subscriptionId);
            const callback = streamCallbacks.get(subscriptionId);
            if (callback) {
              callback(MessagePack.encode([
                subscriptionId,
                linkEventToNativeTuple(event.event),
              ]));
            }
          }
        }
        if (buffer.length !== 0) throw new Error("incomplete Link watch frame");
      } catch (error) {
        for (const [subscriptionId, channelId] of streamChannels.entries()) {
          if (channelId === channel.channelId) {
            const callback = streamCallbacks.get(subscriptionId);
            if (callback) {
              callback(MessagePack.encode([
                1,
                subscriptionId,
                "LINK_WATCH_CHANNEL_ERROR",
                String(error),
              ]));
            }
          }
        }
      } finally {
        channels.delete(channel.channelId);
      }
    }

    async function acquireChannel(): Promise<WatchChannel> {
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

    /** Converts one compact native call tuple into a Link CoreCallRequest. */
    function nativeCallTupleToLinkRequest(tuple: DynamicValue): object {
      return {
        requestId: tuple[0],
        targetPath: { segments: tuple[1] },
        methodName: tuple[2],
        args: tuple[3],
      };
    }

    /** Converts one compact native push-open tuple into a Link CorePushRequest. */
    function nativePushOpenTupleToLinkRequest(tuple: DynamicValue): LinkPushOpenRequest {
      return {
        requestId: tuple[0],
        targetPath: { segments: tuple[1] },
        methodName: tuple[2],
        args: tuple[3],
      };
    }

    /** Converts one compact native push item tuple into a Link CorePushItem. */
    function nativePushItemTupleToLinkItem(tuple: DynamicValue): object {
      return {
        pushId: tuple[0],
        sequence: tuple[1],
        args: tuple[2],
      };
    }

    /** Converts one compact native watch tuple into a Link CoreWatchRequest. */
    function nativeWatchTupleToLinkRequest(tuple: DynamicValue): object {
      return {
        requestId: tuple[0],
        targetPath: { segments: tuple[1] },
        propertyName: tuple[2],
        args: tuple[3],
      };
    }

    /** Converts one compact native watch stream tuple into a Link channel request. */
    function nativeWatchStreamTupleToLinkOpen(tuple: DynamicValue): {
      subscriptionId: DynamicValue;
      request: object;
    } {
      return {
        subscriptionId: tuple[0],
        request: {
          requestId: tuple[1],
          targetPath: { segments: tuple[2] },
          propertyName: tuple[3],
          args: tuple[4],
        },
      };
    }

    /** Converts one Link CoreEvent into a compact native event tuple. */
    function linkEventToNativeTuple(event: DynamicValue): DynamicValue[] {
      return [
        event.requestId ?? null,
        event.targetPath.segments,
        event.propertyName,
        event.kind,
        event.value,
      ];
    }

    /** Encodes one Link CoreCallResponse payload as a compact native bridge result. */
    function encodeCallResponseAsNative(bytes: Uint8Array): Uint8Array {
      const response = MessagePack.decode(bytes);
      const result = response.result;
      if (Object.prototype.hasOwnProperty.call(result, "Ok")) {
        return MessagePack.encode([0, result.Ok]);
      }
      const error = result.Err;
      const location = error.location;
      return MessagePack.encode([
        1,
        error.code,
        error.message,
        error.details ?? null,
        location === null || location === undefined
          ? null
          : [location.file, location.line, location.column],
        error.backtrace ?? null,
      ]);
    }

    /** Encodes one Link CoreEvent payload as a compact native watch snapshot result. */
    function encodeWatchSnapshotAsNative(bytes: Uint8Array): Uint8Array {
      return MessagePack.encode([0, linkEventToNativeTuple(MessagePack.decode(bytes))]);
    }

    const sessionNonce = `web-${createRandomUuid()}`;
    const sessionBytes = await postLink("/link/session", { nonce: sessionNonce });
    const sessionInfo = MessagePack.decode(sessionBytes);
    if (Number(sessionInfo.protocolVersion) !== 3) {
      throw new Error(`Link protocol version ${sessionInfo.protocolVersion} is not supported`);
    }

    return {
      /** Returns the local browser Host's default logical storage roots. */
      async runtimeStorageDefaults(): Promise<RuntimeStoragePaths> {
        return defaultRuntimeStoragePaths();
      },
      /** Normalizes explicit logical storage roots through the local browser Host. */
      async runtimeStoragePaths(runtimeRoot: string, workspaceRoot: string): Promise<RuntimeStoragePaths> {
        return normalizeRuntimeStoragePaths(runtimeRoot, workspaceRoot);
      },
      /** Installs identity-isolated logical roots on the local browser Host. */
      async setRuntimeStorageRoots(runtimeRoot: string, workspaceRoot: string): Promise<void> {
        installRuntimeStoragePaths(runtimeRoot, workspaceRoot);
      },
      /** Reads the local browser bootstrap record outside the remote Link session. */
      async runtimeBootstrapRead(): Promise<string> {
        return readRuntimeBootstrapConfig();
      },
      /** Writes the local browser bootstrap record outside the remote Link session. */
      async runtimeBootstrapWrite(content: string): Promise<void> {
        writeRuntimeBootstrapConfig(content);
      },
      /** Reloads the browser application after a bootstrap change. */
      async restartApplication(): Promise<void> {
        restartRuntimeApplication();
      },
      /** Forwards one compact native call through authenticated HTTP Link. */
      async call(request: Uint8Array): Promise<Uint8Array> {
        return encodeCallResponseAsNative(await postLink("/link/call", {
          request: nativeCallTupleToLinkRequest(MessagePack.decode(request)),
        }));
      },
      /** Forwards one control call through the same authenticated HTTP Link. */
      async controlCall(request: Uint8Array): Promise<Uint8Array> {
        return encodeCallResponseAsNative(await postLink("/link/call", {
          request: nativeCallTupleToLinkRequest(MessagePack.decode(request)),
        }));
      },
      /** Opens one compact native push stream through authenticated WebSocket Link. */
      async pushOpen(request: Uint8Array): Promise<Uint8Array> {
        const decoded = nativePushOpenTupleToLinkRequest(MessagePack.decode(request));
        await sendPushPayload({ type: "PushOpen", body: decoded });
        return MessagePack.encode([0, decoded.requestId]);
      },
      /** Sends one compact native push item through authenticated WebSocket Link. */
      async pushItem(item: Uint8Array): Promise<Uint8Array> {
        await sendPushPayload({
          type: "PushItem",
          body: nativePushItemTupleToLinkItem(MessagePack.decode(item)),
        });
        return MessagePack.encode([0, null]);
      },
      /** Closes one compact native push stream through authenticated WebSocket Link. */
      async pushClose(pushId: string): Promise<Uint8Array> {
        await sendPushPayload({ type: "PushClose", body: pushId });
        return MessagePack.encode([0, null]);
      },
      /** Reads one compact native watch snapshot through authenticated HTTP Link. */
      async watchSnapshot(request: Uint8Array): Promise<Uint8Array> {
        return encodeWatchSnapshotAsNative(await postLink("/link/watch/snapshot", {
          request: nativeWatchTupleToLinkRequest(MessagePack.decode(request)),
        }));
      },
      /** Opens one compact native watch stream through authenticated HTTP Link. */
      async watchStream(
        request: Uint8Array,
        onEvent: (event: Uint8Array) => void,
      ): Promise<Uint8Array> {
        if (typeof onEvent !== "function") {
          throw new Error("watchStream expects an event callback");
        }
        const channel = await acquireChannel();
        const envelope = nativeWatchStreamTupleToLinkOpen(MessagePack.decode(request));
        const subscriptionId = String(envelope.subscriptionId);
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
          if (String(response.subscriptionId) !== subscriptionId) {
            throw new Error("watch channel subscription id mismatch");
          }
          return MessagePack.encode([0, subscriptionId]);
        } catch (error) {
          channel.subscriptionCount -= 1;
          streamCallbacks.delete(subscriptionId);
          streamChannels.delete(subscriptionId);
          throw error;
        }
      },
      /** Closes one compact native watch stream through authenticated HTTP Link. */
      async closeWatchStream(subscriptionId: string): Promise<Uint8Array> {
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
        return MessagePack.encode([0, null]);
      },
    };
  }

  function key(prefix: string, path: string): string {
    return prefix + normalizeRuntimePath(path);
  }

  /** Returns whether this bridge is executing in the model installation worker. */
  function isModelInstallWorker(): boolean {
    return runtimeGlobal.__OPERIT_MODEL_INSTALL_WORKER__ === true;
  }

  /** Returns whether this bridge is executing in the dedicated local runtime worker. */
  function isRuntimeWorker(): boolean {
    return runtimeGlobal.__OPERIT_RUNTIME_WORKER__ === true;
  }

  /** Returns whether this bridge owns the browser UI thread and its DOM-bound hosts. */
  function isMainRuntimeHost(): boolean {
    return !isModelInstallWorker() && !isRuntimeWorker();
  }

  function bytesToBase64(bytes: Uint8Array): string {
    let binary = "";
    for (const byte of bytes) {
      binary += String.fromCharCode(byte);
    }
    return btoa(binary);
  }

  function base64ToBytes(value: string | null): Uint8Array {
    const binary = atob(value || "");
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }
    return bytes;
  }

  /** Collects host secrets required by the isolated model installation worker. */
  function collectModelInstallWorkerSecrets(): ModelInstallWorkerSecret[] {
    const secrets: ModelInstallWorkerSecret[] = [];
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

  /** Loads host secrets supplied by the browser UI thread into the worker runtime. */
  function setModelInstallWorkerSecrets(secrets: ModelInstallWorkerSecret[]): void {
    workerSecrets.clear();
    for (const secret of secrets) {
      workerSecrets.set(secret.key, Uint8Array.from(secret.bytes));
    }
  }

  /** Loads archive bytes downloaded by the browser UI thread into the installation worker. */
  function setModelInstallWorkerDownloads(downloads: ModelInstallWorkerDownload[]): void {
    workerDownloads.clear();
    for (const download of downloads) {
      workerDownloads.set(download.url, Uint8Array.from(download.bytes));
    }
  }

  /** Returns HTTP download requests discovered through the existing WebHttpHost interface. */
  function collectModelInstallWorkerDownloadRequests(): DownloadRequest[] {
    const requests = Array.from(workerDownloadRequests.values());
    workerDownloadRequests.clear();
    return requests;
  }

  /** Returns host secret mutations produced by the model installation worker. */
  function collectModelInstallWorkerSecretChanges(): ModelInstallWorkerSecretChange[] {
    const changes = Array.from(workerChangedSecretKeys, key => ({
      key,
      bytes: workerSecrets.has(key) ? Uint8Array.from(workerSecrets.get(key)!) : null,
    }));
    workerChangedSecretKeys.clear();
    return changes;
  }

  /** Applies host secret mutations produced by the model installation worker. */
  function applyModelInstallWorkerSecretChanges(
    changes: ModelInstallWorkerSecretChange[],
  ): void {
    for (const change of changes) {
      const storageKey = `${secretPrefix}${change.key}`;
      if (change.bytes === null) {
        localStorage.removeItem(storageKey);
      } else {
        localStorage.setItem(storageKey, bytesToBase64(change.bytes));
      }
    }
  }

  function nowIso(): string {
    return new Date().toISOString();
  }

  // Opens the browser storage database used for large runtime files.
  function openStorageDatabase(): Promise<IDBDatabase> {
    if (!storageDatabasePromise) {
      storageDatabasePromise = new Promise<IDBDatabase>((resolve, reject) => {
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
  async function ensureBrowserStorage(): Promise<void> {
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
        await new Promise<void>((resolve, reject) => {
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

  // Persists one memory-view entry into IndexedDB.
  async function persistStorageEntry(itemKey: string, bytes: Uint8Array): Promise<void> {
    const database = await openStorageDatabase();
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(storageObjectStoreName, "readwrite");
      transaction.objectStore(storageObjectStoreName).put(new Uint8Array(bytes), itemKey);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error || new Error("indexedDB write failed"));
    });
  }

  // Removes one memory-view entry from IndexedDB.
  async function removeStorageEntry(itemKey: string): Promise<void> {
    const database = await openStorageDatabase();
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(storageObjectStoreName, "readwrite");
      transaction.objectStore(storageObjectStoreName).delete(itemKey);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error || new Error("indexedDB delete failed"));
    });
  }

  /** Commits one installation change set in a single IndexedDB transaction. */
  async function persistModelInstallStorageChanges(
    changes: ModelInstallWorkerStorageChange[],
  ): Promise<void> {
    const database = await openStorageDatabase();
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(storageObjectStoreName, "readwrite");
      const store = transaction.objectStore(storageObjectStoreName);
      for (const change of changes) {
        if (change.bytes === null) {
          store.delete(change.key);
        } else {
          store.put(change.bytes, change.key);
        }
      }
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(
        transaction.error || new Error("model installation storage commit failed"),
      );
    });
  }

  /** Opens the persistent browser HTTP download database. */
  function openHttpDownloadDatabase(): Promise<IDBDatabase> {
    if (!httpDownloadDatabasePromise) {
      httpDownloadDatabasePromise = new Promise<IDBDatabase>((resolve, reject) => {
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
            const download = cursor.value as Omit<PersistedHttpDownload, "paused">;
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

  /** Reads one persistent browser HTTP download by source URL. */
  async function readHttpDownload(url: string): Promise<PersistedHttpDownload | null> {
    const database = await openHttpDownloadDatabase();
    return new Promise((resolve, reject) => {
      const transaction = database.transaction(httpDownloadObjectStoreName, "readonly");
      const request = transaction.objectStore(httpDownloadObjectStoreName).get(url);
      request.onsuccess = () => resolve(request.result || null);
      request.onerror = () => reject(request.error || new Error("HTTP download read failed"));
    });
  }

  /** Builds lightweight status metadata for one persistent HTTP download. */
  function httpDownloadStatus(download: PersistedHttpDownload): HttpDownloadStatus {
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

  /** Loads persistent HTTP download metadata into the Host progress cache once. */
  function ensureHttpDownloadStatusCache(): Promise<void> {
    if (httpDownloadStatusCachePromise === null) {
      httpDownloadStatusCachePromise = (async (): Promise<void> => {
        const database = await openHttpDownloadDatabase();
        const downloads = await new Promise<PersistedHttpDownload[]>((resolve, reject) => {
          const transaction = database.transaction(httpDownloadObjectStoreName, "readonly");
          const request = transaction.objectStore(httpDownloadObjectStoreName).getAll();
          request.onsuccess = () => resolve(request.result);
          request.onerror = () => reject(
            request.error || new Error("HTTP download status cache load failed"),
          );
        });
        for (const download of downloads) {
          httpDownloadStatusCache.set(download.url, httpDownloadStatus(download));
        }
      })();
    }
    return httpDownloadStatusCachePromise;
  }

  /** Persists one resumable browser HTTP download. */
  async function writeHttpDownload(download: PersistedHttpDownload): Promise<void> {
    await ensureHttpDownloadStatusCache();
    const database = await openHttpDownloadDatabase();
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(httpDownloadObjectStoreName, "readwrite");
      transaction.objectStore(httpDownloadObjectStoreName).put(download, download.url);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error || new Error("HTTP download write failed"));
    });
    httpDownloadStatusCache.set(download.url, httpDownloadStatus(download));
  }

  /** Deletes one persistent browser HTTP download. */
  async function deleteHttpDownload(url: string): Promise<void> {
    await stopHttpDownload(url);
    await ensureHttpDownloadStatusCache();
    const database = await openHttpDownloadDatabase();
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(httpDownloadObjectStoreName, "readwrite");
      transaction.objectStore(httpDownloadObjectStoreName).delete(url);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error || new Error("HTTP download delete failed"));
    });
    httpDownloadStatusCache.delete(url);
  }

  /** Lists every persistent browser HTTP download task. */
  async function listHttpDownloads(): Promise<HttpDownloadStatus[]> {
    await ensureHttpDownloadStatusCache();
    return Array.from(httpDownloadStatusCache.values(), download => ({
      ...download,
      active: activeHttpDownloadPromises.has(download.url),
    }));
  }

  /** Pauses one active browser HTTP download while preserving persisted bytes. */
  function pauseHttpDownload(url: string): void {
    activeHttpDownloadControllers.get(url)?.abort();
  }

  /** Stops one HTTP download loop and waits until it can no longer persist chunks. */
  async function stopHttpDownload(url: string): Promise<void> {
    const active = activeHttpDownloadPromises.get(url);
    pauseHttpDownload(url);
    if (active !== undefined) {
      await active.then(
        (): void => {},
        (): void => {},
      );
    }
  }

  /** Builds the stable model installation task key used by Host controls. */
  function localModelTaskKey(identity: LocalModelIdentity): string {
    return `${identity.modelId}@${identity.version}`;
  }

  /** Starts a new generation for one exact model installation task. */
  function startModelInstallTask(taskKey: string): number {
    modelInstallTaskGeneration += 1;
    modelInstallTaskGenerations.set(taskKey, modelInstallTaskGeneration);
    return modelInstallTaskGeneration;
  }

  /** Invalidates every older generation for one exact model installation task. */
  function invalidateModelInstallTask(taskKey: string): void {
    modelInstallTaskGeneration += 1;
    modelInstallTaskGenerations.set(taskKey, modelInstallTaskGeneration);
    activeModelInstallAborters.get(taskKey)?.();
    activeModelInstallPromises.delete(taskKey);
  }

  /** Checks whether an installation generation still owns its model task. */
  function isCurrentModelInstallTask(taskKey: string, generation: number): boolean {
    return modelInstallTaskGenerations.get(taskKey) === generation;
  }

  /** Writes one storage entry through the correct browser execution context. */
  function persistStorageWrite(itemKey: string, bytes: Uint8Array): void {
    if (isModelInstallWorker()) {
      return;
    }
    void persistStorageEntry(itemKey, bytes);
  }

  /** Removes one storage entry through the correct browser execution context. */
  function persistStorageDelete(itemKey: string): void {
    if (isModelInstallWorker()) {
      return;
    }
    void removeStorageEntry(itemKey);
  }

  /** Records one changed storage key during worker-owned model installation. */
  function recordWorkerStorageChange(itemKey: string): void {
    if (isModelInstallWorker()) {
      workerChangedStorageKeys.add(itemKey);
    }
  }

  /** Returns isolated storage changes produced by the model installation worker. */
  async function collectWorkerStorageChanges(): Promise<ModelInstallWorkerStorageChange[]> {
    const changes = Array.from(workerChangedStorageKeys, itemKey => ({
      key: itemKey,
      bytes: storageCache.has(itemKey) ? storageCache.get(itemKey)! : null,
    }));
    workerChangedStorageKeys.clear();
    return changes;
  }

  function storageRead(prefix: string, path: string): Uint8Array {
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

  /** Reads one bounded range from worker-owned runtime storage. */
  function storageReadRange(prefix: string, path: string, offset: number, length: number): Uint8Array {
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

  function storageWrite(prefix: string, path: string, content: Uint8Array | ArrayBuffer): void {
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

  /** Appends bytes to one browser runtime storage entry and persists the result. */
  function storageAppend(prefix: string, path: string, content: Uint8Array | ArrayBuffer): void {
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

  /** Reports whether one exact virtual storage path is a persisted file. */
  function storageHasFile(prefix: string, path: string): boolean {
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

  function storageExists(prefix: string, path: string): boolean {
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

  function storageDelete(prefix: string, path: string, recursive: boolean): void {
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

  // Returns whether one storage mutation commits the local model registry.
  function isLocalModelRegistryPath(prefix: string, path: string): boolean {
    return prefix === runtimePrefix &&
      normalizeRuntimePath(path) ===
        "runtime/config/preferences/local_model_registry.preferences.json";
  }

  function storageList(prefix: string, path: string): FileStorageEntry[] {
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
    const entries: FileStorageEntry[] = [];
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

  /** Maps runtime and workspace file-system paths onto canonical runtime OPFS storage. */
  function workerStorageAddress(prefix: string, path: string): { prefix: string; path: string } {
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

  // Builds the canonical in-memory key for one browser-host directory.
  function fileDirectoryKey(path: string): string {
    const normalized = normalizeRuntimePath(path);
    return normalized.length === 0 ? filePrefix : `${filePrefix}${normalized}/`;
  }

  // Reports whether the browser host can resolve one directory from explicit or file-backed state.
  function fileDirectoryExists(path: string): boolean {
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

  // Creates one browser-host directory and, when requested, all of its parents.
  function makeFileDirectory(path: string, createParents: boolean): void {
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

  // Lists immediate file-system children from browser-host directory and file state.
  function listFileDirectory(path: string): FileStorageEntry[] {
    const directory = fileDirectoryKey(path);
    const entries = new Map<string, FileStorageEntry>();
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
    } else {
      for (const [itemKey, bytes] of storageCache.entries()) {
        if (!itemKey.startsWith(directory)) {
          continue;
        }
        const relative = itemKey.slice(directory.length);
        const separator = relative.indexOf("/");
        const name = separator < 0 ? relative : relative.slice(0, separator);
        if (separator < 0) {
          entries.set(name, { path: name, isDirectory: false, size: bytes.length });
        } else {
          entries.set(name, { path: name, isDirectory: true, size: 0 });
        }
      }
    }
    return Array.from(entries.values());
  }

  /** Copies one browser-host file-system entry while preserving directory recursion semantics. */
  function copyFileSystemEntry(source: string, destination: string, recursive: boolean): void {
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
      storageWrite(
        filePrefix,
        normalizedDestination,
        storageRead(filePrefix, normalizedSource),
      );
      return;
    }
    if (!recursive) {
      throw new Error("Source is a directory and recursive flag is not set");
    }
    makeFileDirectory(normalizedDestination, true);
    for (const entry of listFileDirectory(normalizedSource)) {
      copyFileSystemEntry(
        `${normalizedSource}/${entry.path}`,
        `${normalizedDestination}/${entry.path}`,
        true,
      );
    }
  }

  // Removes one browser-host directory from the in-memory directory index.
  function deleteFileDirectory(path: string, recursive: boolean): void {
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

  /** Loads one classic runtime script in the browser window or model installation worker. */
  async function loadScript(src: string): Promise<void> {
    if (isModelInstallWorker() || isRuntimeWorker()) {
      const response = await fetch(src);
      if (!response.ok) {
        throw new Error(`failed to load ${src}: HTTP ${response.status}`);
      }
      const source = await response.text();
      const execute = new Function(
        `${source}\nglobalThis.initSqlJs = initSqlJs;`,
      ) as () => void;
      execute();
      return;
    }
    return new Promise<void>((resolve, reject) => {
      const existing = document.querySelector<HTMLScriptElement>(`script[src="${src}"]`);
      if (existing) {
        existing.addEventListener("load", () => resolve(), { once: true });
        existing.addEventListener(
          "error",
          () => reject(new Error(`failed to load ${src}`)),
          { once: true },
        );
        return;
      }
      const script = document.createElement("script");
      script.src = src;
      script.onload = () => resolve();
      script.onerror = () => reject(new Error(`failed to load ${src}`));
      document.head.appendChild(script);
    });
  }

  async function ensureSqlite(): Promise<void> {
    if (!sqliteModulePromise) {
      sqliteModulePromise = (async () => {
        await loadScript("sql-wasm.js");
        const initializeSqlJs = runtimeGlobal.initSqlJs;
        if (initializeSqlJs === undefined) {
          throw new Error("sql.js initializer is not loaded");
        }
        SQLite = await initializeSqlJs({
          locateFile(file: string): string {
            return file;
          },
        });
      })();
    }
    await sqliteModulePromise;
  }

  function sqliteKey(path: string): string {
    return key(runtimePrefix, path);
  }

  function saveSqliteDatabase(connection: SqliteConnection): void {
    storageWrite(runtimePrefix, connection.path, connection.db.export());
  }

  function sqliteConnection(id: string): SqliteConnection {
    const connection = sqliteConnections.get(id);
    if (!connection) {
      throw new Error(`sqlite connection not found: ${id}`);
    }
    return connection;
  }

  function sqliteTransaction(id: string): SqliteConnection {
    const transaction = sqliteTransactions.get(id);
    if (!transaction) {
      throw new Error(`sqlite transaction not found: ${id}`);
    }
    return transaction;
  }

  function sqliteParam(param: SqliteParameter): SqlValue {
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

  function sqliteParams(params: SqliteParameter[] | undefined): SqlValue[] {
    return (params || []).map(sqliteParam);
  }

  function sqliteValue(value: SqlValue | undefined): SqliteSerializedValue {
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

  function querySqlite(
    db: SqlDatabase,
    sql: string,
    params: SqliteParameter[] | undefined,
  ): SqliteQueryRow[] {
    const statement = db.prepare(sql);
    const rows: SqliteQueryRow[] = [];
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

  function fileInfo(path: string): {
    path: string;
    exists: boolean;
    fileType: string;
    size: number;
    permissions: string;
    owner: string;
    group: string;
    lastModified: string;
    rawStatOutput: string;
  } {
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

  function unavailable(name: string): never {
    throw new Error(`${name} is not available in the browser host`);
  }

  const ttsPlayback = (() => {
    let activeUtterance: SpeechSynthesisUtterance | null = null;
    let activeAudio: HTMLAudioElement | null = null;
    let activeAudioUrl: string | null = null;
    let activeAudioPaused = false;
    let activePath = "";
    let utteranceIndex = 0;
    let lastDetails = "browser speech synthesis idle";

    function synthesis(): SpeechSynthesis {
      const value = globalThis.speechSynthesis;
      if (value === undefined || value === null) {
        throw new Error("browser speechSynthesis is not available");
      }
      return value;
    }

    function requireText(value: JsonValue | DynamicValue, name: string): string {
      if (typeof value !== "string") {
        throw new Error(`${name} must be a string`);
      }
      return value.trim();
    }

    function requireNumber(value: JsonValue | DynamicValue, name: string): number {
      if (typeof value !== "number" || !Number.isFinite(value)) {
        throw new Error(`${name} must be a finite number`);
      }
      return value;
    }

    function requireBoolean(value: JsonValue | DynamicValue, name: string): boolean {
      if (typeof value !== "boolean") {
        throw new Error(`${name} must be a boolean`);
      }
      return value;
    }

    function selectedVoice(voiceName: string): SpeechSynthesisVoice | null {
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

    function currentStatus(details: string): {
      path: string;
      active: boolean;
      paused: boolean;
      details: string;
    } {
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
    function audioContentType(path: string): string {
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
    function releaseAudio(): void {
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
      playAudio(path: JsonValue | DynamicValue) {
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
          new Blob([blobPart(bytes)], { type: audioContentType(audioPath) })
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
      speakText(request: DynamicValue) {
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
    let audio: HTMLAudioElement | null = null;
    let source: string | null = null;
    let sourceType: string | null = null;
    let title: string | null = null;
    let artist: string | null = null;
    let loopPlayback = false;
    let volume = 1;
    let state = "idle";
    let message = "browser music player idle";

    function currentStatus(details: string) {
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

    function bufferedPositionMs(activeAudio: HTMLAudioElement | null): number {
      if (!activeAudio || activeAudio.buffered.length === 0) {
        return activeAudio ? Math.round(activeAudio.currentTime * 1000) : 0;
      }
      return Math.round(activeAudio.buffered.end(activeAudio.buffered.length - 1) * 1000);
    }

    function setSource(activeAudio: HTMLAudioElement, request: MusicRequest): void {
      if (request.sourceType === "path" || request.sourceType === "url" || request.sourceType === "uri") {
        activeAudio.src = request.source;
        return;
      }
      throw new Error(`unsupported music sourceType: ${request.sourceType}`);
    }

    return {
      playAudio(path: string) {
        const oneShot = new Audio(String(path));
        oneShot.play();
        return { path: String(path), started: true, details: "browser audio playback started" };
      },
      playMusic(request: MusicRequest) {
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
      seekMusic(positionMs: number) {
        if (audio === null) {
          throw new Error("browser music player is not initialized");
        }
        audio.currentTime = Math.max(Number(positionMs || 0), 0) / 1000;
        message = "browser music playback seeked";
        return currentStatus(message);
      },
      setMusicVolume(value: number) {
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
    const bleSessions = new Map<string, BleSession>();
    const notifications = new Map<string, BleNotification[]>();

    function browserBluetooth(): BluetoothApi {
      const api = browserNavigator.bluetooth;
      if (!api) {
        throw new Error("browser Web Bluetooth is not available");
      }
      return api;
    }

    function bytesFromPayload(payload: BluetoothWriteRequest | BluetoothWriteAndReadRequest): Uint8Array {
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

    function readData(sessionId: string, bytes: DataView | Uint8Array): {
      sessionId: string;
      bytesRead: number;
      text: string;
      dataBase64: string;
    } {
      const value = bytes instanceof DataView ? new Uint8Array(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength)) : new Uint8Array(bytes);
      return {
        sessionId,
        bytesRead: value.length,
        text: textDecoder.decode(value),
        dataBase64: bytesToBase64(value),
      };
    }

    function session(id: string): BleSession {
      const value = bleSessions.get(id);
      if (!value) {
        throw new Error(`BLE session not found: ${id}`);
      }
      return value;
    }

    function characteristic(
      sessionId: string,
      serviceUuid: string,
      characteristicUuid: string,
    ): BluetoothCharacteristic {
      const value = session(sessionId);
      const key = `${serviceUuid}:${characteristicUuid}`;
      const cached = value.characteristics.get(key);
      if (!cached) {
        throw new Error(`BLE characteristic not discovered: ${key}`);
      }
      return cached;
    }

    function classicUnavailable(name: string): never {
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
          devices: [] as Array<{
            name: string | null;
            address: string;
            type: string;
            bondState: string;
            source: string;
            rssi: number | null;
          }>,
        };
      },
      scanBluetoothDevices(request: { durationMs?: number }) {
        const optionalServices: string[] = [];
        const deviceRequest = { acceptAllDevices: true, optionalServices };
        return browserBluetooth().requestDevice(deviceRequest).then((device) => ({
          devices: [{
            name: device.name || null,
            address: device.id,
            type: "ble",
            bondState: "unknown",
            source: "browser.web_bluetooth",
            rssi: null as number | null,
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
      bluetoothClose(sessionId: string) {
        const value = bleSessions.get(sessionId);
        if (value && value.device.gatt.connected) {
          value.device.gatt.disconnect();
        }
        bleSessions.delete(sessionId);
        notifications.delete(sessionId);
        return `browser_bluetooth_session_closed:${sessionId}`;
      },
      bluetoothBleConnect() {
        return browserBluetooth().requestDevice({ acceptAllDevices: true }).then((device) =>
          device.gatt.connect().then((server) => {
            const sessionId = `web-ble-${createRandomUuid()}`;
            bleSessions.set(sessionId, { device, server, characteristics: new Map() });
            notifications.set(sessionId, []);
            return { sessionId, address: device.id, mode: "ble" };
          })
        );
      },
      bluetoothBleDiscoverServices(sessionId: string) {
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
      bluetoothBleReadCharacteristic(address: BluetoothReadAddress) {
        return characteristic(address.sessionId, address.serviceUuid, address.characteristicUuid)
          .readValue()
          .then((value) => readData(address.sessionId, value));
      },
      bluetoothBleWriteCharacteristic(request: BluetoothWriteRequest) {
        const bytes = bytesFromPayload(request);
        return characteristic(request.sessionId, request.serviceUuid, request.characteristicUuid)
          .writeValue(bytes)
          .then(() => ({ sessionId: request.sessionId, bytesWritten: bytes.length }));
      },
      bluetoothBleWriteAndReadCharacteristic(request: BluetoothWriteAndReadRequest) {
        const bytes = bytesFromPayload(request);
        return characteristic(request.sessionId, request.writeServiceUuid, request.writeCharacteristicUuid)
          .writeValue(bytes)
          .then(() =>
            characteristic(request.sessionId, request.readServiceUuid, request.readCharacteristicUuid).readValue()
          )
          .then((value) => readData(request.sessionId, value));
      },
      bluetoothBleSubscribeCharacteristic(request: BluetoothSubscribeRequest) {
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
      bluetoothBleReadNotifications(sessionId: string, limit: number) {
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

  function characteristicPropertyNames(properties: BluetoothCharacteristicProperties): string[] {
    const names: string[] = [];
    if (properties.read) names.push("read");
    if (properties.write) names.push("write");
    if (properties.writeWithoutResponse) names.push("write_without_response");
    if (properties.notify) names.push("notify");
    if (properties.indicate) names.push("indicate");
    return names;
  }

  // Schedules browser local inference discovery after storage changes.
  function scheduleWebLocalInferenceRefresh(): void {
    webLocalInferenceReadyPromise = null;
    queueMicrotask(() => {
      void ensureWebLocalInference().catch((error) => {
        console.warn("[Operit local inference]", error);
      });
    });
  }

  // Initializes installed browser local inference bundles.
  async function ensureWebLocalInference(): Promise<void> {
    if (!webLocalInferenceReadyPromise) {
      webLocalInferenceReadyPromise = (async () => {
        const state: WebLocalInferenceState = {
          asrBundles: new Map<string, WebAsrBundle>(),
          ttsBundles: new Map<string, WebTtsBundle>(),
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
        runtimeGlobal.__operitLocalInference = {
          transcribeLocalSpeech: transcribeWebLocalSpeech,
          synthesizeLocalSpeech: synthesizeWebLocalSpeech,
        };
      })();
    }
    return webLocalInferenceReadyPromise;
  }

  // Releases all native objects and Blob URLs owned by one Web inference state.
  function disposeWebLocalInferenceState(state: WebLocalInferenceState | null): void {
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
  async function loadInstalledWebAsrBundles(state: WebLocalInferenceState): Promise<void> {
    const roots = runtimeBundleRoots("sherpa-onnx-asr.js");
    for (const root of roots) {
      const paths: AsrBundlePaths = {
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
  async function loadInstalledWebTtsBundles(state: WebLocalInferenceState): Promise<void> {
    const roots = runtimeBundleRoots("sherpa-onnx-tts.js");
    for (const root of roots) {
      const paths: TtsBundlePaths = {
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
  function runtimeBundleRoots(fileName: string): string[] {
    const suffix = `/${fileName}`;
    const roots: string[] = [];
    for (const itemKey of storageCache.keys()) {
      if (!itemKey.startsWith(runtimePrefix) || !itemKey.endsWith(suffix)) {
        continue;
      }
      roots.push(itemKey.substring(runtimePrefix.length, itemKey.length - suffix.length));
    }
    return roots;
  }

  // Checks that every runtime path is present in the synchronous storage view.
  function runtimePathsExist(paths: string[]): boolean {
    return paths.every((path) => storageExists(runtimePrefix, path));
  }

  // Creates a blob URL for one runtime-storage file.
  function runtimeBlobUrl(
    path: string,
    contentType: string,
    state: WebLocalInferenceState,
  ): string {
    const bytes = storageRead(runtimePrefix, path);
    if (bytes.length === 0) {
      throw new Error(`runtime file is empty or missing: ${path}`);
    }
    const url = URL.createObjectURL(new Blob([blobPart(bytes)], { type: contentType }));
    state.blobUrls.push(url);
    return url;
  }

  // Creates a JavaScript Blob URL with one exact source suffix.
  function runtimeJavaScriptUrl(
    path: string,
    suffix: string,
    state: WebLocalInferenceState,
  ): string {
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
  function loadClassicScriptUrl(src: string): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      const script = document.createElement("script");
      script.src = src;
      script.onload = () => resolve();
      script.onerror = () => reject(new Error(`failed to load ${src}`));
      document.head.appendChild(script);
    });
  }

  // Builds one browser ASR bundle from installed Sherpa files.
  async function createWebAsrBundle(
    paths: AsrBundlePaths,
    state: WebLocalInferenceState,
  ): Promise<WebAsrBundle> {
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
    const moduleValue: SherpaModuleConfig = {};
    const ready = new Promise<void>((resolve, reject) => {
      moduleValue.mainScriptUrlOrBlob = urls.runtimeScript;
      moduleValue.locateFile = (path: string): string => {
        if (path === "sherpa-onnx-wasm-main-vad-asr.wasm") return urls.runtimeWasm;
        if (path === "sherpa-onnx-wasm-main-vad-asr.data") return urls.runtimeData;
        return path;
      };
      moduleValue.setStatus = (status: string): void => console.debug("[Operit ASR]", status);
      moduleValue.onRuntimeInitialized = () => resolve();
      moduleValue.onAbort = (reason: string): void => reject(new Error(reason));
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

  // Returns the Paraformer ASR config embedded in the Web bundle.
  function webAsrConfig(): { modelConfig: JsonValue } {
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
  async function createWebTtsBundle(
    paths: TtsBundlePaths,
    state: WebLocalInferenceState,
  ): Promise<WebTtsBundle> {
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
  function webTtsWorkerSource(urls: {
    ttsScript: string;
    runtimeScript: string;
    runtimeWasm: string;
    runtimeData: string;
  }): string {
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
  function waitForWebTtsWorker(worker: Worker): Promise<WebTtsWorkerReady> {
    return new Promise((resolve, reject) => {
      const onMessage = (event: MessageEvent<WebTtsWorkerReady & { type: string; status?: string; message?: string }>) => {
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
      const onError = (event: ErrorEvent): void => {
        worker.removeEventListener("message", onMessage);
        worker.removeEventListener("error", onError);
        reject(new Error(event.message || "Web TTS worker initialization failed"));
      };
      worker.addEventListener("message", onMessage);
      worker.addEventListener("error", onError);
    });
  }

  // Runs one synchronous command against an initialized Web TTS worker.
  function generateWebTtsWav(
    bundle: WebTtsBundle,
    text: string,
    speaker: number,
    speed: number,
  ): Uint8Array {
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

  // Waits for one exact worker state while preserving worker error text.
  function waitForWebTtsControl(control: Int32Array, expectedState: number) {
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
  function requireCrossOriginIsolation(capability: string): void {
    if (globalThis.crossOriginIsolated !== true) {
      throw new Error(
        `Web local ${capability} requires Cross-Origin-Opener-Policy: same-origin and ` +
          "Cross-Origin-Embedder-Policy: require-corp",
      );
    }
  }

  // Transcribes one local Web speech request.
  function transcribeWebLocalSpeech(requestJson: string): string {
    const state = requireWebLocalInferenceState();
    const request = JSON.parse(requestJson) as AsrSpeechRequest;
    const driver = parseTaggedDriver<SherpaAsrDriver>(request.driverJson, "SherpaOnnxWebAsrBundle");
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
    } finally {
      stream.free();
    }
  }

  // Synthesizes one local Web speech request.
  function synthesizeWebLocalSpeech(requestJson: string): string {
    const state = requireWebLocalInferenceState();
    const request = JSON.parse(requestJson) as TtsSpeechRequest;
    const driver = parseTaggedDriver<SherpaTtsDriver>(request.driverJson, "SherpaOnnxWebTtsBundle");
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
  function requireWebLocalInferenceState(): WebLocalInferenceState {
    if (!webLocalInferenceState) {
      throw new Error("Web local inference runner is still initializing");
    }
    return webLocalInferenceState;
  }

  // Parses one externally tagged local model driver.
  function parseTaggedDriver<T>(driverJson: string, expectedTag: string): T {
    const root = JSON.parse(driverJson) as Record<string, T>;
    const keys = Object.keys(root);
    if (keys.length !== 1 || keys[0] !== expectedTag) {
      throw new Error(`Web local inference driver must be ${expectedTag}`);
    }
    return root[expectedTag];
  }

  // Resolves a model bundle root from model directory and driver script path.
  function runtimeDirectoryForDriver(modelDirectory: string, relativeFilePath: string): string {
    const directory = normalizeRuntimePath(modelDirectory);
    const filePath = normalizeRuntimePath(relativeFilePath);
    const slash = filePath.lastIndexOf("/");
    if (slash < 0) {
      return directory;
    }
    return normalizeRuntimePath(`${directory}/${filePath.slice(0, slash)}`);
  }

  // Normalizes runtime storage paths to canonical slash-separated segments.
  function normalizeRuntimePath(path: string): string {
    return String(path)
      .replace(/\\/g, "/")
      .split("/")
      .filter((segment) => segment.length > 0 && segment !== ".")
      .join("/");
  }

  // Decodes one mono PCM WAV byte payload into Float32 samples.
  function decodeMonoPcmWav(bytes: Uint8Array): { sampleRate: number; samples: Float32Array } {
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
  function localInferenceRunner(
    method: keyof WebLocalInferenceRunner,
  ): (requestJson: string) => string {
    const runner = runtimeGlobal.__operitLocalInference;
    if (!runner || typeof runner[method] !== "function") {
      throw new Error(`web local inference method is not installed: ${method}`);
    }
    // Calls the resolved runner and validates its JSON string contract.
    return function runLocalInference(requestJson: string): string {
      const responseJson = runner[method](requestJson);
      if (typeof responseJson !== "string") {
        throw new Error(`web local inference method returned non-string JSON: ${method}`);
      }
      return responseJson;
    };
  }

  /** Returns one local URL for a browser-hosted v86 runtime artifact. */
  function v86AssetUrl(name: string): string {
    return new URL(`./v86/${name}`, import.meta.url).href;
  }

  /** Returns one immutable public URL for a V86 Linux guest asset. */
  function v86RuntimeAssetUrl(name: string): string {
    return new URL(
      name,
      "https://models.operit.app/v86-runtime/i686-buildroot-node20-python312-20260720/",
    ).href;
  }

  /** Validates the structured process request received from the WebAssembly runtime host. */
  function validateManagedRuntimeRequest(request: ManagedRuntimeRequest): void {
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

  /** Maps one managed runtime program to its installed executable inside the guest. */
  function guestRuntimeExecutable(program: string): { program: string; executable: string } {
    switch (program) {
      case "node":
        return { program: "node", executable: "/usr/local/bin/node" };
      case "python":
        return { program: "python3", executable: "/usr/local/bin/python3" };
      default:
        throw new Error(`unsupported V86 managed runtime program: ${program}`);
    }
  }

  /** Resolves a browser virtual-file-system path into a guest-relative workspace path. */
  function guestWorkspacePath(path: string | null | undefined): string {
    const segments = String(path || "")
      .replace(/\\/g, "/")
      .split("/")
      .filter(segment => segment.length > 0 && segment !== ".");
    if (segments.some(segment => segment === "..")) {
      throw new Error("managed runtime workspace path escapes the guest workspace");
    }
    return segments.length === 0 ? "." : segments.join("/");
  }

  /** Selects virtual files belonging to the requested managed runtime working directory. */
  function managedRuntimeWorkspaceFrames(workingDirectory: string): string[] {
    const prefix = workingDirectory === "." ? "" : `${workingDirectory}/`;
    const frames: string[] = [];
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

  /** Rewrites working-directory environment paths to their corresponding guest paths. */
  function managedRuntimeEnvironment(
    environment: Record<string, string>,
    hostWorkingDirectory: string | null | undefined,
    guestWorkingDirectory: string,
  ): Record<string, string> {
    const result: Record<string, string> = {};
    const hostRoot = String(hostWorkingDirectory || "").replace(/\\/g, "/").replace(/\/+$/, "");
    const guestRoot = guestWorkingDirectory === "."
      ? "/workspace"
      : `/workspace/${guestWorkingDirectory}`;
    for (const [name, value] of Object.entries(environment)) {
      const normalizedValue = value.replace(/\\/g, "/");
      if (hostRoot.length > 0 && normalizedValue === hostRoot) {
        result[name] = guestRoot;
      } else if (hostRoot.length > 0 && normalizedValue.startsWith(`${hostRoot}/`)) {
        result[name] = `${guestRoot}/${normalizedValue.slice(hostRoot.length + 1)}`;
      } else {
        result[name] = value;
      }
    }
    return result;
  }

  /** Sends one structured command to a dedicated V86 worker. */
  function postManagedRuntimeWorkerMessage(worker: Worker, message: object): void {
    worker.postMessage(message);
  }

  /** Creates one worker-backed guest process and schedules its serial deployment frames. */
  function startManagedRuntimeProcess(request: ManagedRuntimeRequest): string {
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
    const serialBuffer = new SharedArrayBuffer(
      managedRuntimeHeaderLength * Int32Array.BYTES_PER_ELEMENT + managedRuntimeOutputCapacity,
    );
    const header = new Int32Array(serialBuffer, 0, managedRuntimeHeaderLength);
    const output = new Uint8Array(serialBuffer, managedRuntimeHeaderLength * Int32Array.BYTES_PER_ELEMENT);
    const id = `v86-runtime-${++managedRuntimeProcessIndex}`;
    const worker = new Worker(new URL("./v86_runtime_worker.js", import.meta.url), {
      type: "module",
      name: id,
    });
    const process: ManagedRuntimeProcess = {
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

  /** Resolves one managed runtime process by its host-issued identifier. */
  function managedRuntimeProcess(id: string): ManagedRuntimeProcess {
    const process = managedRuntimeProcesses.get(id);
    if (process === undefined) {
      throw new Error(`managed runtime process does not exist: ${id}`);
    }
    return process;
  }

  /** Copies all serial bytes currently available from a guest's shared output ring. */
  function drainManagedRuntimeOutputBytes(process: ManagedRuntimeProcess): Uint8Array {
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

  /** Decodes newly available guest stdout bytes into the process line buffer. */
  function refreshManagedRuntimeStdout(process: ManagedRuntimeProcess): void {
    const bytes = drainManagedRuntimeOutputBytes(process);
    if (bytes.length > 0) {
      process.stdout += process.decoder.decode(bytes, { stream: true });
    }
  }

  /** Returns the current guest lifecycle state from the shared serial header. */
  function managedRuntimeState(process: ManagedRuntimeProcess): number {
    return Atomics.load(process.header, managedRuntimeStateIndex);
  }

  /** Waits for the V86 worker to report output, a lifecycle change, or deadline expiry. */
  async function waitForManagedRuntimeChange(
    process: ManagedRuntimeProcess,
    deadline: number,
  ): Promise<void> {
    const remainingMs = Math.ceil(deadline - performance.now());
    if (remainingMs <= 0) {
      return;
    }
    const observedVersion = process.activityVersion;
    await new Promise<void>(resolve => {
      const timer = setTimeout(() => {
        process.activityWaiters.delete(complete);
        resolve();
      }, remainingMs);
      const complete = (): void => {
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

  /** Reads one newline-delimited stdout frame without relying on browser event-loop progress. */
  async function readManagedRuntimeStdoutLine(id: string, timeoutMs: number): Promise<string | null> {
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

  /** Writes protocol lines to one guest process serial input. */
  function writeManagedRuntimeLines(id: string, lines: string[]): void {
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

  /** Drains guest diagnostics accumulated after a V86 worker failure. */
  function drainManagedRuntimeStderr(id: string): string {
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

  /** Returns whether a guest process has not exited or failed. */
  function isManagedRuntimeRunning(id: string): boolean {
    const state = managedRuntimeState(managedRuntimeProcess(id));
    return state === managedRuntimeStarting || state === managedRuntimeRunning;
  }

  /** Terminates a guest V86 process and releases its emulator worker. */
  function killManagedRuntimeProcess(id: string): void {
    const process = managedRuntimeProcess(id);
    const state = managedRuntimeState(process);
    if (state === managedRuntimeFailed || state === managedRuntimeStopped) {
      return;
    }
    Atomics.store(process.header, managedRuntimeStateIndex, managedRuntimeStopped);
    postManagedRuntimeWorkerMessage(process.worker, { type: "kill" });
  }

  /** Runs one finite guest command and collects all serial output before releasing its worker. */
  async function runManagedRuntimeCommand(request: ManagedRuntimeRequest): Promise<{
    exitCode: number | null;
    stdout: string;
    stderr: string;
  }> {
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

  /** Returns one active Linux VM session or raises a terminal lifecycle error. */
  function linuxVmSession(sessionId: string): LinuxVmSession {
    const session = linuxVmSessions.get(sessionId);
    if (session === undefined) {
      throw new Error(`Linux VM terminal session does not exist: ${sessionId}`);
    }
    return session;
  }

  /** Adds bytes to one session's terminal output buffer without dropping data. */
  function appendLinuxVmOutput(session: LinuxVmSession, bytes: Uint8Array): void {
    const requiredLength = session.outputLength + bytes.length;
    if (requiredLength > linuxVmOutputLimit) {
      failLinuxVmSession(session, new Error("Linux VM terminal output exceeded 4 MiB"));
      return;
    }
    if (requiredLength > session.output.length) {
      const nextLength = Math.min(
        linuxVmOutputLimit,
        Math.max(requiredLength, session.output.length * 2),
      );
      const expanded = new Uint8Array(nextLength);
      expanded.set(session.output.subarray(0, session.outputLength));
      session.output = expanded;
    }
    session.output.set(bytes, session.outputLength);
    session.outputLength = requiredLength;
  }

  /** Appends one command-output byte while enforcing the terminal output limit. */
  function appendLinuxVmCommandOutput(command: LinuxVmCommand, byte: number): void {
    const requiredLength = command.outputLength + 1;
    if (requiredLength > linuxVmOutputLimit) {
      throw new Error("Linux VM command output exceeded 4 MiB");
    }
    if (requiredLength > command.output.length) {
      const expanded = new Uint8Array(Math.min(
        linuxVmOutputLimit,
        Math.max(requiredLength, command.output.length * 2),
      ));
      expanded.set(command.output.subarray(0, command.outputLength));
      command.output = expanded;
    }
    command.output[command.outputLength] = byte;
    command.outputLength = requiredLength;
  }

  /** Delivers one serial byte to the visible terminal stream and active command capture. */
  function deliverLinuxVmCommandByte(
    session: LinuxVmSession,
    command: LinuxVmCommand,
    byte: number,
  ): void {
    appendLinuxVmOutput(session, Uint8Array.of(byte));
    if (command.collectingOutput) {
      appendLinuxVmCommandOutput(command, byte);
    }
  }

  /** Parses one exact OSC command lifecycle marker emitted by the guest shell. */
  function parseLinuxVmCommandMarker(
    command: LinuxVmCommand,
    bytes: number[],
  ): { kind: "start" } | { kind: "end"; exitCode: number; workingDir: string } | null {
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

  /** Resolves one active command and starts the next command queued for its terminal session. */
  function completeLinuxVmCommand(
    session: LinuxVmSession,
    command: LinuxVmCommand,
    result: LinuxVmCommandResult,
  ): void {
    if (session.activeCommand !== command) {
      return;
    }
    clearTimeout(command.timeoutHandle);
    session.activeCommand = null;
    command.resolve(result);
    startNextLinuxVmCommand(session);
  }

  /** Rejects every active or queued command after its Linux VM session becomes unavailable. */
  function rejectLinuxVmCommands(session: LinuxVmSession, reason: unknown): void {
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

  /** Returns the complete one-line shell program used to execute one persistent terminal command. */
  function linuxVmCommandInput(command: LinuxVmCommand): Uint8Array {
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

  /** Starts the next serialized command once its persistent Linux guest terminal is ready. */
  function startNextLinuxVmCommand(session: LinuxVmSession): void {
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
    } catch (error) {
      session.activeCommand = null;
      clearTimeout(command.timeoutHandle);
      command.reject(error);
      startNextLinuxVmCommand(session);
    }
  }

  /** Handles one active command protocol byte while retaining ordinary terminal output. */
  function consumeLinuxVmCommandByte(
    session: LinuxVmSession,
    command: LinuxVmCommand,
    byte: number,
  ): void {
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

  /** Queues one shell command for execution inside an existing persistent Linux VM terminal. */
  function executeLinuxVmCommand(
    sessionId: string,
    command: string,
    timeoutMs: number,
  ): Promise<LinuxVmCommandResult> {
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
    return new Promise<LinuxVmCommandResult>((resolve, reject) => {
      const token = String(++linuxVmCommandIndex);
      const commandState: LinuxVmCommand = {
        token,
        command,
        timeoutHandle: globalThis.setTimeout(() => {
          if (session.activeCommand !== commandState) {
            return;
          }
          const output = textDecoder.decode(
            commandState.output.subarray(0, commandState.outputLength),
          ).trim();
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

  /** Redraws the single PTY progress line used while the Linux VM starts. */
  function renderLinuxVmProgress(session: LinuxVmSession, message: string): void {
    session.progressVisible = true;
    appendLinuxVmOutput(session, textEncoder.encode(`\r\x1b[2K${message}`));
  }

  /** Clears the PTY progress line before terminal output or a completed startup stage. */
  function finishLinuxVmProgress(session: LinuxVmSession, message: string | null): void {
    if (!session.progressVisible) {
      return;
    }
    session.progressVisible = false;
    const suffix = message === null ? "" : `${message}\r\n`;
    appendLinuxVmOutput(session, textEncoder.encode(`\r\x1b[2K${suffix}`));
  }

  /** Formats one V86 asset download event to fit on one interactive terminal line. */
  function linuxVmDownloadStatus(value: unknown, columns: number): string | null {
    if (typeof value !== "object" || value === null) {
      return null;
    }
    const progress = value as Record<string, unknown>;
    const fileIndex = progress.file_index;
    const fileCount = progress.file_count;
    const loaded = progress.loaded;
    const total = progress.total;
    if (
      typeof fileIndex !== "number" ||
      typeof fileCount !== "number" ||
      typeof loaded !== "number" ||
      typeof total !== "number" ||
      !Number.isInteger(fileIndex) ||
      !Number.isInteger(fileCount) ||
      fileIndex < 0 ||
      fileCount < 1 ||
      loaded < 0 ||
      total < 1
    ) {
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

  /** Marks the Linux VM terminal ready only after the guest init program accepts serial input. */
  function markLinuxVmReady(session: LinuxVmSession): void {
    if (session.state !== "starting") {
      return;
    }
    session.state = "running";
    finishLinuxVmProgress(session, "Runtime ready");
    flushLinuxVmInput(session);
    startNextLinuxVmCommand(session);
  }

  /** Marks one Linux VM session as failed and makes the reason visible in its terminal stream. */
  function failLinuxVmSession(session: LinuxVmSession, error: unknown): void {
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
      void session.emulator.destroy().catch((destroyError: unknown) => {
        console.error("Failed to stop Linux VM terminal after an error", destroyError);
      });
    }
  }

  /** Flushes terminal input accepted before the Linux guest reaches its running state. */
  function flushLinuxVmInput(session: LinuxVmSession): void {
    const emulator = session.emulator;
    if (emulator === null || session.state !== "running") {
      return;
    }
    for (const data of session.inputQueue) {
      emulator.serial_send_bytes(0, data);
    }
    session.inputQueue = [];
  }

  /** Starts the v86 Linux guest and connects its virtual serial console to one terminal session. */
  async function startLinuxVm(session: LinuxVmSession): Promise<void> {
    try {
      renderLinuxVmProgress(session, "Preparing runtime");
      const modulePath = v86AssetUrl("libv86.mjs");
      const module = await import(modulePath) as unknown as V86Module;
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
      emulator.add_listener("serial0-output-byte", (value: unknown) => {
        if (typeof value === "number" && session.state !== "closed") {
          const byte = value & 0xff;
          finishLinuxVmProgress(session, "Starting Linux");
          const activeCommand = session.activeCommand;
          if (activeCommand === null) {
            appendLinuxVmOutput(session, Uint8Array.of(byte));
          } else {
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
      emulator.add_listener("download-progress", (value: unknown) => {
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
      emulator.add_listener("download-error", (value: unknown) => {
        failLinuxVmSession(session, new Error(`Linux VM asset download failed: ${String(value)}`));
      });
    } catch (error) {
      failLinuxVmSession(session, error);
    }
  }

  /** Allocates one browser-local Linux VM terminal session and begins guest boot. */
  function startLinuxVmSession(sessionId: string, rows: number, cols: number): void {
    if (linuxVmSessions.has(sessionId)) {
      throw new Error(`Linux VM terminal session already exists: ${sessionId}`);
    }
    if (!Number.isInteger(rows) || rows < 1 || !Number.isInteger(cols) || cols < 1) {
      throw new Error(`Invalid Linux VM terminal dimensions: ${rows}x${cols}`);
    }
    const session: LinuxVmSession = {
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

  /** Drains the raw virtual serial bytes emitted by one Linux VM terminal session. */
  function readLinuxVmPty(sessionId: string): Uint8Array {
    const session = linuxVmSession(sessionId);
    const output = session.output.slice(0, session.outputLength);
    session.outputLength = 0;
    return output;
  }

  /** Writes raw terminal bytes into one Linux guest virtual serial console. */
  function writeLinuxVmPty(sessionId: string, data: Uint8Array): number {
    const session = linuxVmSession(sessionId);
    if (session.state === "failed" || session.state === "closed") {
      throw new Error(`Linux VM terminal is not running: ${sessionId}`);
    }
    const bytes = new Uint8Array(data);
    if (session.state === "starting") {
      session.inputQueue.push(bytes);
    } else {
      const emulator = session.emulator;
      if (emulator === null) {
        throw new Error(`Linux VM terminal emulator is unavailable: ${sessionId}`);
      }
      emulator.serial_send_bytes(0, bytes);
    }
    return bytes.length;
  }

  /** Records the terminal dimensions requested by the browser-side terminal renderer. */
  function resizeLinuxVmPty(sessionId: string, rows: number, cols: number): void {
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

  /** Returns the Linux VM process exit code once the virtual machine has stopped. */
  function linuxVmPtyExitCode(sessionId: string): number | null {
    return linuxVmSession(sessionId).exitCode;
  }

  /** Stops and releases one browser-local Linux VM terminal session. */
  function closeLinuxVmPty(sessionId: string): void {
    const session = linuxVmSession(sessionId);
    session.state = "closed";
    rejectLinuxVmCommands(session, new Error(`Linux VM terminal closed: ${sessionId}`));
    linuxVmSessions.delete(sessionId);
    if (session.emulator !== null) {
      void session.emulator.destroy().catch((error: unknown) => {
        console.error("Failed to stop Linux VM terminal", error);
      });
    }
  }

  // Installs an isolated smoke-test API when explicitly requested by the test page.
  function installWebLocalInferenceTestApi(): void {
    if (runtimeGlobal.__OPERIT_LOCAL_INFERENCE_TEST__ !== true) {
      return;
    }
    runtimeGlobal.__operitLocalInferenceTest = {
      putRuntimeFile(path: string, content: Uint8Array) {
        storageCache.set(key(runtimePrefix, path), new Uint8Array(content));
      },
      readRuntimeFile(path: string): Uint8Array {
        return storageRead(runtimePrefix, path);
      },
      async initialize() {
        webLocalInferenceReadyPromise = null;
        await ensureWebLocalInference();
      },
      transcribe(request: AsrSpeechRequest) {
        return JSON.parse(transcribeWebLocalSpeech(JSON.stringify(request)));
      },
      synthesize(request: TtsSpeechRequest) {
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
      /** Starts one browser-local Linux VM terminal session. */
      startPty(sessionId: string, rows: number, cols: number): void {
        startLinuxVmSession(sessionId, rows, cols);
      },
      /** Drains raw virtual serial bytes from one browser-local Linux VM terminal. */
      readPty(sessionId: string): Uint8Array {
        return readLinuxVmPty(sessionId);
      },
      /** Writes raw terminal bytes into one browser-local Linux VM terminal. */
      writePty(sessionId: string, data: Uint8Array): number {
        return writeLinuxVmPty(sessionId, data);
      },
      /** Executes one command through the existing browser-local Linux VM terminal shell. */
      executePtyCommand(
        sessionId: string,
        command: string,
        timeoutMs: number,
      ): Promise<LinuxVmCommandResult> {
        return executeLinuxVmCommand(sessionId, command, timeoutMs);
      },
      /** Records terminal dimensions requested for one browser-local Linux VM terminal. */
      resizePty(sessionId: string, rows: number, cols: number): void {
        resizeLinuxVmPty(sessionId, rows, cols);
      },
      /** Returns one browser-local Linux VM terminal exit code after shutdown. */
      exitCode(sessionId: string): number | null {
        return linuxVmPtyExitCode(sessionId);
      },
      /** Stops and releases one browser-local Linux VM terminal. */
      closePty(sessionId: string): void {
        closeLinuxVmPty(sessionId);
      },
    }),
    runtimeStorage: registerWorkerHostModule({
      readBytes(path: string): Uint8Array {
        return storageRead(runtimePrefix, path);
      },
      readBytesRange(path: string, offset: number, length: number): Uint8Array {
        return storageReadRange(runtimePrefix, path, offset, length);
      },
      writeBytes(path: string, content: Uint8Array): void {
        storageWrite(runtimePrefix, path, content);
      },
      appendBytes(path: string, content: Uint8Array): void {
        storageAppend(runtimePrefix, path, content);
      },
      delete(path: string, recursive: boolean): void {
        storageDelete(runtimePrefix, path, recursive);
      },
      exists(path: string): boolean {
        return storageExists(runtimePrefix, path);
      },
      list(prefix: string): FileStorageEntry[] {
        return storageList(runtimePrefix, prefix);
      },
      /** Opens one sequential writer owned by the runtime worker storage host. */
      createWriteSession(path: string): string {
        const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
        if (storage === undefined) {
          throw new Error("runtime worker OPFS storage is not installed");
        }
        return storage.createWriteSession(path);
      },
      /** Appends one chunk to a runtime worker storage writer. */
      writeSessionChunk(sessionId: string, content: Uint8Array): void {
        const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
        if (storage === undefined) {
          throw new Error("runtime worker OPFS storage is not installed");
        }
        storage.writeSessionChunk(sessionId, content);
      },
      /** Commits one runtime worker storage writer. */
      commitWriteSession(sessionId: string): void {
        const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
        if (storage === undefined) {
          throw new Error("runtime worker OPFS storage is not installed");
        }
        storage.commitWriteSession(sessionId);
      },
      /** Discards one runtime worker storage writer. */
      discardWriteSession(sessionId: string): void {
        const storage = runtimeGlobal.__operitRuntimeWorkerStorage;
        if (storage === undefined) {
          throw new Error("runtime worker OPFS storage is not installed");
        }
        storage.discardWriteSession(sessionId);
      },
    }),
    archiveStaging: registerWorkerHostModule({
      /** Creates one fixed-size archive staging record. */
      createArchive(archiveId: string, expectedByteLength: number): void {
        const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
        if (staging === undefined) {
          throw new Error("runtime worker archive staging is not installed");
        }
        staging.createArchive(archiveId, expectedByteLength);
      },
      /** Appends one byte chunk to an archive staging record. */
      appendArchive(archiveId: string, content: Uint8Array): void {
        const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
        if (staging === undefined) {
          throw new Error("runtime worker archive staging is not installed");
        }
        staging.appendArchive(archiveId, content);
      },
      /** Seals one complete archive staging record. */
      sealArchive(archiveId: string): number {
        const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
        if (staging === undefined) {
          throw new Error("runtime worker archive staging is not installed");
        }
        return staging.sealArchive(archiveId);
      },
      /** Reads one exact byte range from a sealed archive staging record. */
      readArchive(archiveId: string, offset: number, length: number): Uint8Array {
        const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
        if (staging === undefined) {
          throw new Error("runtime worker archive staging is not installed");
        }
        return staging.readArchive(archiveId, offset, length);
      },
      /** Removes one archive staging record. */
      removeArchive(archiveId: string): void {
        const staging = runtimeGlobal.__operitRuntimeWorkerArchiveStaging;
        if (staging === undefined) {
          throw new Error("runtime worker archive staging is not installed");
        }
        staging.removeArchive(archiveId);
      },
    }),
    hostSecretStore: registerMainHostModule({
      // Reads a host-owned secret from browser-local protected storage.
      readSecret(key: string): Uint8Array | null {
        if (isModelInstallWorker()) {
          const value = workerSecrets.get(key);
          return value === undefined ? null : Uint8Array.from(value);
        }
        const value = localStorage.getItem(`${secretPrefix}${key}`);
        return value === null ? null : base64ToBytes(value);
      },
      // Writes a host-owned secret into browser-local protected storage.
      writeSecret(key: string, content: Uint8Array): void {
        if (isModelInstallWorker()) {
          workerSecrets.set(key, Uint8Array.from(content));
          workerChangedSecretKeys.add(key);
          return;
        }
        localStorage.setItem(`${secretPrefix}${key}`, bytesToBase64(new Uint8Array(content)));
      },
      // Deletes a host-owned secret from browser-local protected storage.
      deleteSecret(key: string): void {
        if (isModelInstallWorker()) {
          workerSecrets.delete(key);
          workerChangedSecretKeys.add(key);
          return;
        }
        localStorage.removeItem(`${secretPrefix}${key}`);
      },
    }),
    sqlite: registerWorkerHostModule({
      open(path: string): string {
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
      executeBatch(id: string, sql: string): void {
        const connection = sqliteConnection(id);
        connection.db.exec(sql);
        saveSqliteDatabase(connection);
      },
      execute(id: string, sql: string, params: SqliteParameter[]): number {
        const connection = sqliteConnection(id);
        connection.db.run(sql, sqliteParams(params));
        saveSqliteDatabase(connection);
        return connection.db.getRowsModified();
      },
      query(id: string, sql: string, params: SqliteParameter[]): SqliteQueryRow[] {
        return querySqlite(sqliteConnection(id).db, sql, params);
      },
      lastInsertRowId(id: string): string | number {
        const rows = querySqlite(sqliteConnection(id).db, "SELECT last_insert_rowid()", []);
        const value = rows[0]?.values[0];
        return value !== undefined &&
          (value.kind === "integer" || value.kind === "real" || value.kind === "text")
          ? value.value
          : "0";
      },
      beginTransaction(id: string): string {
        const transactionId = `sqlite-tx-${++sqliteTransactionIndex}`;
        const connection = sqliteConnection(id);
        connection.db.run("BEGIN IMMEDIATE");
        sqliteTransactions.set(transactionId, connection);
        return transactionId;
      },
      transactionExecute(id: string, sql: string, params: SqliteParameter[]): number {
        const connection = sqliteTransaction(id);
        connection.db.run(sql, sqliteParams(params));
        return connection.db.getRowsModified();
      },
      transactionQuery(id: string, sql: string, params: SqliteParameter[]): SqliteQueryRow[] {
        return querySqlite(sqliteTransaction(id).db, sql, params);
      },
      transactionLastInsertRowId(id: string): string | number {
        const rows = querySqlite(sqliteTransaction(id).db, "SELECT last_insert_rowid()", []);
        const value = rows[0]?.values[0];
        return value !== undefined &&
          (value.kind === "integer" || value.kind === "real" || value.kind === "text")
          ? value.value
          : "0";
      },
      commitTransaction(id: string): void {
        const connection = sqliteTransaction(id);
        connection.db.run("COMMIT");
        saveSqliteDatabase(connection);
        sqliteTransactions.delete(id);
      },
    }),
    fileSystem: registerWorkerHostModule({
      validatePath() {},
      listFiles(path: string) {
        return listFileDirectory(path).map((entry) => ({
          name: entry.path.split("/").pop() || entry.path,
          isDirectory: entry.isDirectory,
          size: entry.size,
          permissions: "rw",
          lastModified: nowIso(),
        }));
      },
      readFile(path: string): string {
        return textDecoder.decode(storageRead(filePrefix, path));
      },
      readFileWithLimit(path: string, maxBytes: number): string {
        return textDecoder.decode(storageRead(filePrefix, path).slice(0, maxBytes));
      },
      readFileBytes(path: string): Uint8Array {
        return storageRead(filePrefix, path);
      },
      writeFile(path: string, content: string, append: boolean): void {
        const previous = append && storageExists(filePrefix, path)
          ? textDecoder.decode(storageRead(filePrefix, path))
          : "";
        storageWrite(filePrefix, path, textEncoder.encode(previous + content));
      },
      writeFileBytes(path: string, content: Uint8Array): void {
        storageWrite(filePrefix, path, content);
      },
      deleteFile(path: string, recursive: boolean): void {
        storageDelete(filePrefix, path, recursive);
        deleteFileDirectory(path, recursive);
      },
      fileExists(path: string) {
        const isFile = storageHasFile(filePrefix, path);
        const isDirectory = !isFile && fileDirectoryExists(path);
        return {
          exists: isFile || isDirectory,
          isDirectory,
          size: isFile ? storageRead(filePrefix, path).length : 0,
        };
      },
      moveFile(source: string, destination: string): void {
        const content = storageRead(filePrefix, source);
        storageWrite(filePrefix, destination, content);
        storageDelete(filePrefix, source, false);
      },
      copyFile(source: string, destination: string, recursive: boolean): void {
        copyFileSystemEntry(source, destination, recursive);
      },
      makeDirectory(path: string, createParents: boolean): void {
        makeFileDirectory(path, createParents);
      },
      findFiles(): FileStorageEntry[] {
        return [];
      },
      fileInfo,
      grepCode(): { matches: string[]; totalMatches: number; filesSearched: number } {
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
    }),
    webVisit: registerMainHostModule({
      visitWeb(request: { url: string }) {
        return {
          url: request.url,
          title: request.url,
          content: "",
          metadata: [] as string[],
          links: [] as string[],
          imageLinks: [] as string[],
        };
      },
    }),
    localInference: registerMainHostModule({
      // Transcribes one local speech request through the installed browser runner.
      transcribeLocalSpeech(requestJson: string): string {
        return localInferenceRunner("transcribeLocalSpeech")(requestJson);
      },
      // Synthesizes one local speech request through the installed browser runner.
      synthesizeLocalSpeech(requestJson: string): string {
        return localInferenceRunner("synthesizeLocalSpeech")(requestJson);
      },
    }),
    http: registerMainHostModule({
      executeHttpRequest(request: HttpRequest) {
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
      /** Opens one Fetch response body and reports its bytes through stable host callbacks. */
      openHttpByteStream(
        streamId: string,
        request: HttpRequest,
        onOpened: () => void,
        onChunk: (chunk: Uint8Array) => void,
        onClosed: (error: string | null) => void,
      ): void {
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
            let body: BodyInit | undefined;
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
          } catch (error) {
            onClosed(controller.signal.aborted ? null : String(error));
          } finally {
            activeHttpByteStreamControllers.delete(streamId);
          }
        })();
      },
      /** Cancels one browser-owned Fetch response body. */
      closeHttpByteStream(streamId: string): void {
        const controller = activeHttpByteStreamControllers.get(streamId);
        if (controller === undefined) {
          throw new Error(`HTTP byte stream is not open: ${streamId}`);
        }
        controller.abort();
      },
      downloadFile(request: DownloadRequest) {
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
      resolveRuntimeExecutable(program: string, executablePath: string | null): string {
        const runtime = guestRuntimeExecutable(program);
        const requestedPath = executablePath?.trim();
        if (requestedPath && requestedPath !== runtime.executable) {
          throw new Error(`V86 managed runtime cannot execute host path: ${requestedPath}`);
        }
        return runtime.executable;
      },
      startRuntimeProcess(request: ManagedRuntimeRequest): string {
        return startManagedRuntimeProcess(request);
      },
      runRuntimeCommand(request: ManagedRuntimeRequest) {
        return runManagedRuntimeCommand(request);
      },
    }),
    managedRuntimeProcess: registerMainHostModule({
      writeLine(id: string, line: string): void {
        writeManagedRuntimeLines(id, [line]);
      },
      writeLines(id: string, lines: string[]): void {
        writeManagedRuntimeLines(id, lines);
      },
      readStdoutLine(id: string, timeoutMs: number): Promise<string | null> {
        return readManagedRuntimeStdoutLine(id, timeoutMs);
      },
      drainStderr(id: string): string {
        return drainManagedRuntimeStderr(id);
      },
      isRunning(id: string): boolean {
        return isManagedRuntimeRunning(id);
      },
      kill(id: string): void {
        killManagedRuntimeProcess(id);
      },
    }),
    musicPlayback: registerMainHostModule(musicPlayback),
    bluetooth: registerMainHostModule(bluetooth),
    ttsPlayback: registerMainHostModule(ttsPlayback),
    systemOperation: registerMainHostModule({
      /** Returns the locale declared by the browser for runtime language selection. */
      getSystemLanguageCode() {
        return { languageCode: navigator.language };
      },
      toast(message: string): void {
        console.info("[Operit toast]", message);
      },
      /** Sends a browser system notification and requests the required permission. */
      sendNotification(title: string, message: string, activationJson: string): void {
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
      modifySystemSetting(namespace: string, setting: string, value: string) {
        return { namespace, setting, value };
      },
      getSystemSetting(namespace: string, setting: string) {
        return { namespace, setting, value: "" };
      },
      installApp(path: string) {
        return { operationType: "install", packageName: path, success: false, details: "" };
      },
      uninstallApp(packageName: string) {
        return { operationType: "uninstall", packageName, success: false, details: "" };
      },
      listInstalledApps(includeSystemApps: boolean) {
        return { includesSystemApps: includeSystemApps, packages: [] as string[] };
      },
      startApp(packageName: string) {
        return { operationType: "start", packageName, success: false, details: "" };
      },
      stopApp(packageName: string) {
        return { operationType: "stop", packageName, success: false, details: "" };
      },
      getNotifications() {
        return { notifications: [] as string[], timestamp: Date.now() };
      },
      getAppUsageTime(
        packageName: string,
        sinceHours: number,
        limit: number,
        includeSystemApps: boolean,
      ) {
        return {
          startTime: Date.now(),
          endTime: Date.now(),
          sinceHours,
          requestedPackageName: packageName,
          includesSystemApps: includeSystemApps,
          totalEntries: 0,
          entries: [] as string[],
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

  let bridgePromise: Promise<RuntimeBridge> | null = null;

  /** Initializes the WebAssembly bridge with its browser WASI host. */
  async function initializeWasmBridge(
    module: WasmBridgeModule,
    wasi: WasiModule,
  ): Promise<RuntimeBridge> {
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
      const wasmModule = importRuntimeScript(wasmModulePath) as Promise<WasmBridgeModule>;
      const wasiModule = importRuntimeScript("./wasi_snapshot_preview1.js") as Promise<WasiModule>;
      bridgePromise = Promise.all([wasmModule, wasiModule])
        .then(([module, wasi]) => initializeWasmBridge(module, wasi));
    }
    return bridgePromise;
  }

  /** Checks whether one encoded Core request installs a local model. */
  function isLocalModelInstallRequest(request: Uint8Array): boolean {
    const decoded: unknown = MessagePack.decode(request);
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

  /** Reads the exact local model identity from one structured service request. */
  function localModelIdentity(request: Uint8Array): LocalModelIdentity {
    const decoded: unknown = MessagePack.decode(request);
    if (!Array.isArray(decoded) || decoded.length !== 4) {
      throw new Error("local model service request is invalid");
    }
    const args = decoded[3];
    if (typeof args !== "object" || args === null || Array.isArray(args)) {
      throw new Error("local model service arguments are invalid");
    }
    const modelId = (args as Record<string, unknown>).modelId;
    const version = (args as Record<string, unknown>).version;
    if (typeof modelId !== "string" || typeof version !== "string") {
      throw new Error("local model service request requires modelId and version");
    }
    return { modelId, version };
  }

  /** Downloads requests discovered through the existing browser HTTP host interface. */
  async function downloadModelInstallRequests(
    requests: DownloadRequest[],
    identity: LocalModelIdentity,
  ): Promise<ModelInstallWorkerDownload[]> {
    return Promise.all(requests.map(request => downloadHttpRequest(request, identity)));
  }

  /** Deduplicates concurrent consumers of one browser HTTP download task. */
  function downloadHttpRequest(
    request: DownloadRequest,
    identity: LocalModelIdentity,
  ): Promise<ModelInstallWorkerDownload> {
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

  /** Downloads or resumes one browser HTTP host request and persists every received chunk. */
  async function executeHttpDownloadRequest(
    request: DownloadRequest,
    identity: LocalModelIdentity,
  ): Promise<ModelInstallWorkerDownload> {
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
    if (
      persisted.fileId !== request.fileId ||
      persisted.expectedBytes !== request.expectedBytes ||
      persisted.downloadedBytes !== persisted.content.size
      || persisted.modelId !== identity.modelId
      || persisted.version !== identity.version
      || typeof persisted.paused !== "boolean"
    ) {
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
          headers.set(
            Array.isArray(pair) ? pair[0] : pair.key,
            Array.isArray(pair) ? pair[1] : pair.value,
          );
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
        throw new Error(
          `download ${request.fileId} expected HTTP ${expectedStatus}, got ${response.status}`,
        );
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
      throw new Error(
        `download ${request.fileId} size mismatch: ${persisted.downloadedBytes} != ${persisted.expectedBytes}`,
      );
    }
    return {
      url: request.url,
      bytes: new Uint8Array(await persisted.content.arrayBuffer()),
    };
  }

  /** Reloads transferred model archives from their complete persistent Blobs. */
  async function reloadModelInstallDownloads(
    downloads: ModelInstallWorkerDownload[],
  ): Promise<void> {
    await Promise.all(downloads.map(async download => {
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

  /** Persists one atomic model installation result received from the worker. */
  async function applyModelInstallWorkerStorageChanges(
    changes: ModelInstallWorkerStorageChange[],
  ): Promise<void> {
    let registryChanged = false;
    const registryKey = key(
      runtimePrefix,
      "runtime/config/preferences/local_model_registry.preferences.json",
    );
    await persistModelInstallStorageChanges(changes);
    for (const change of changes) {
      if (change.bytes === null) {
        storageCache.delete(change.key);
      } else {
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

  /** Installs one local model through an isolated browser worker. */
  function installLocalModelInWorker(request: Uint8Array): Promise<Uint8Array> {
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

  /** Runs downloads and installation commits for one owned model task generation. */
  async function executeLocalModelInstall(
    request: Uint8Array,
    identity: LocalModelIdentity,
    taskKey: string,
    generation: number,
  ): Promise<Uint8Array> {
    const downloads: ModelInstallWorkerDownload[] = [];
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

  /** Serializes model extraction and registry commits after parallel downloads complete. */
  function enqueueModelInstallWorker(
    request: Uint8Array,
    downloads: ModelInstallWorkerDownload[],
    taskKey: string,
    generation: number,
  ): Promise<ModelInstallWorkerResult | ModelInstallWorkerDownloadRequests> {
    const result = modelInstallCommitQueue.then(async () => {
      const message = await executeModelInstallWorker(request, downloads, taskKey, generation);
      if (message.type === "result") {
        await applyModelInstallWorkerStorageChanges(message.changes);
        applyModelInstallWorkerSecretChanges(message.secretChanges);
      }
      return message;
    });
    modelInstallCommitQueue = result.then(
      (): void => {},
      (): void => {},
    );
    return result;
  }

  /** Executes one installation attempt and returns its result or discovered host downloads. */
  function executeModelInstallWorker(
    request: Uint8Array,
    downloads: ModelInstallWorkerDownload[],
    taskKey: string,
    generation: number,
  ): Promise<ModelInstallWorkerResult | ModelInstallWorkerDownloadRequests> {
    if (!isCurrentModelInstallTask(taskKey, generation)) {
      return Promise.reject(new Error(`local model installation paused: ${taskKey}`));
    }
    return new Promise((resolve, reject) => {
      const worker = new Worker(runtimeIdentityWorkerUrl("./operit_model_install_worker.js"), {
        type: "module",
      });
      let settled = false;
      let aborter: () => void;
      /** Terminates the worker and releases its Host control entry. */
      const close = (): void => {
        worker.terminate();
        if (activeModelInstallAborters.get(taskKey) === aborter) {
          activeModelInstallAborters.delete(taskKey);
        }
      };
      /** Rejects the installation worker exactly once. */
      const fail = (error: Error): void => {
        if (settled) {
          return;
        }
        settled = true;
        close();
        reject(error);
      };
      /** Resolves the installation worker exactly once. */
      const succeed = (
        message: ModelInstallWorkerResult | ModelInstallWorkerDownloadRequests,
      ): void => {
        if (settled) {
          return;
        }
        settled = true;
        close();
        resolve(message);
      };
      /** Stops this exact installation task through the Host control path. */
      aborter = (): void => {
        fail(new Error(`local model installation paused: ${taskKey}`));
      };
      activeModelInstallAborters.set(taskKey, aborter);
      if (!isCurrentModelInstallTask(taskKey, generation)) {
        aborter();
        return;
      }
      worker.addEventListener("message", (event: MessageEvent<
        ModelInstallWorkerResult | ModelInstallWorkerDownloadRequests | ModelInstallWorkerError
      >) => {
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
      const transferables: Transferable[] = [
        ownedRequest.buffer,
        ...secrets.map(secret => secret.bytes.buffer),
        ...downloads.map(download => download.bytes.buffer),
      ];
      worker.postMessage(
        { type: "install", request: ownedRequest, secrets, downloads },
        transferables,
      );
    });
  }

  /** Builds one existing LocalModelInstallStatus payload from a Host download task. */
  function localModelInstallStatus(downloads: HttpDownloadStatus[]): object {
    if (downloads.length === 0) {
      throw new Error("local model download status requires at least one file");
    }
    const first = downloads[0];
    return {
      operationId: `${first.modelId}@${first.version}`,
      modelId: first.modelId,
      version: first.version,
      phase:
        activeModelInstallPromises.has(localModelTaskKey(first)) &&
          !downloads.every(download => download.paused)
        ? "Model"
        : "Cancelled",
      currentFile: downloads.map(download => download.fileId).join(", "),
      downloadedBytes: downloads.reduce((total, download) => total + download.downloadedBytes, 0),
      totalBytes: downloads.reduce((total, download) => total + download.expectedBytes, 0),
      error: null,
    };
  }

  /** Groups persistent download files by their exact local model release. */
  function groupLocalModelDownloads(
    downloads: HttpDownloadStatus[],
  ): HttpDownloadStatus[][] {
    const groups = new Map<string, HttpDownloadStatus[]>();
    for (const download of downloads) {
      const taskKey = localModelTaskKey(download);
      const group = groups.get(taskKey);
      if (group === undefined) {
        groups.set(taskKey, [download]);
      } else {
        group.push(download);
      }
    }
    return Array.from(groups.values());
  }

  /** Handles existing local model status and control methods through the Host download manager. */
  async function handleLocalModelDownloadCall(
    request: Uint8Array,
  ): Promise<LocalModelDownloadCallResult> {
    const decoded: unknown = MessagePack.decode(request);
    if (!Array.isArray(decoded) || decoded.length !== 4) {
      return { handled: false, response: new Uint8Array() };
    }
    const targetPath = decoded[1];
    if (
      !Array.isArray(targetPath) || targetPath.length !== 2 ||
      targetPath[0] !== "services" || targetPath[1] !== "localModelService"
    ) {
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
    const matching = downloads.filter(download =>
      download.modelId === identity.modelId && download.version === identity.version
    );
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

  /** Installs an origin-wide coordinator with exactly one lock-owning Dedicated Worker. */
  function installRuntimeWorkerProxy(): void {
    const clientId = createRandomUuid();
    const coordinator = new BroadcastChannel(runtimeCoordinatorName);
    const locks = (navigator as Navigator & { locks?: RuntimeWorkerLockManager }).locks;
    if (locks === undefined) {
      throw new Error("Web Locks API is unavailable for the Web runtime");
    }
    let runtimeWorker: Worker | null = null;
    let releaseLeadership: (() => void) | null = null;
    let closed = false;
    let nextRequestId = 0;
    let nextRuntimeRequestId = 0;
    const pendingRequests = new Map<number, RuntimeWorkerPendingRequest>();
    const pendingCommands = new Map<number, RuntimeWorkerPendingCommand>();
    const watchCallbacks = new Map<number, (event: Uint8Array) => void>();
    const watchSubscriptions = new Map<string, number>();
    const hostPayloads = new Map<number, Uint8Array>();
    const leaderRequests = new Map<number, RuntimeWorkerLeaderRequestRoute>();
    const leaderRequestIds = new Map<string, Map<number, number>>();
    const leaderWatchRequestIds = new Map<string, Map<string, number>>();
    const leaderHostCalls = new Map<number, RuntimeWorkerLeaderHostCallRoute>();
    const disconnectedHostPayloads = new Map<number, Uint8Array>();

    coordinator.addEventListener("message", event => handleCoordinatorMessage(event.data));
    runtimeGlobal.addEventListener("pagehide", disconnectRuntimeWorker);
    void locks.request(runtimeOwnerLockName, { mode: "exclusive" }, runRuntimeLeadership).catch(rejectRuntimeRequests);

    /** Holds the origin-wide lock for this page while its Dedicated Worker owns OPFS handles. */
    async function runRuntimeLeadership(): Promise<void> {
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
      await new Promise<void>(resolve => {
        releaseLeadership = resolve;
      });
      worker.terminate();
      runtimeWorker = null;
      releaseLeadership = null;
    }

    /** Waits for the owner Worker to close every OPFS handle before it is terminated. */
    function shutdownRuntimeWorker(worker: Worker): Promise<void> {
      return new Promise<void>(resolve => {
        const handleMessage = (event: MessageEvent<unknown>): void => {
          const message = event.data as Partial<RuntimeWorkerShutdownComplete>;
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

    /** Routes one cross-page coordination message received from a different browser page. */
    function handleCoordinatorMessage(value: unknown): void {
      if (typeof value !== "object" || value === null) {
        return;
      }
      if (isLeaderCoreRequest(value)) {
        if (runtimeWorker !== null) {
          forwardLeaderRequest(value);
        }
        return;
      }
      const message = value as RuntimeWorkerLeaderMessage;
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

    /** Sends one page-local Core request to the active owner or to the origin coordinator. */
    function forwardPendingRequest(id: number, command: RuntimeWorkerPendingCommand): void {
      const request: RuntimeWorkerLeaderRequest = {
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

    /** Replays outstanding page-local requests after this page or another page acquires the lock. */
    function forwardPendingRequests(): void {
      for (const [id, command] of pendingCommands) {
        forwardPendingRequest(id, command);
      }
    }

    /** Registers one globally routed request and dispatches it through the owner worker. */
    function forwardLeaderRequest(request: RuntimeWorkerLeaderRequest): void {
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

    /** Handles a result, event, or synchronous host message produced by the owner worker. */
    function handleRuntimeWorkerMessage(event: MessageEvent<unknown>): void {
      const message = event.data as RuntimeWorkerLeaderMessage;
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

    /** Routes one owner-worker reply to the page that issued its original Core request. */
    function forwardLeaderRuntimeReply(message: RuntimeWorkerLeaderMessage): void {
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

    /** Routes one synchronous Host call to the page that initiated its Core operation. */
    function forwardLeaderHostCall(message: RuntimeWorkerLeaderMessage): void {
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

    /** Routes the second synchronous Host payload phase to the page that prepared its response. */
    function forwardLeaderHostPayload(message: RuntimeWorkerLeaderMessage): void {
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

    /** Delivers a coordinator message locally or through BroadcastChannel to its target page. */
    function deliverCoordinatorMessage(targetClientId: string, message: RuntimeWorkerLeaderMessage): void {
      if (targetClientId === clientId) {
        handleClientRuntimeMessage(message);
        return;
      }
      coordinator.postMessage(message);
    }

    /** Handles a result, Watch event, or synchronous Host phase delivered to this browser page. */
    function handleClientRuntimeMessage(message: RuntimeWorkerLeaderMessage): void {
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
        void handleRuntimeWorkerHostCall(message as RuntimeWorkerHostCall, hostPayloads);
        return;
      }
      if (message.type === "hostPayload") {
        handleRuntimeWorkerHostPayload(message as RuntimeWorkerHostPayload, hostPayloads);
      }
    }

    /** Sends one Core command through the origin-wide runtime owner and retains its request identity. */
    function request(
      operation: RuntimeWorkerCoreRequest["operation"],
      payload: Uint8Array | string,
      onWatchEvent?: (event: Uint8Array) => void,
    ): Promise<{ id: number; response: Uint8Array }> {
      if (closed) {
        return Promise.reject(new Error("runtime page coordinator is closed"));
      }
      const id = ++nextRequestId;
      const response = new Promise<Uint8Array>((resolve, reject) => {
        pendingRequests.set(id, { resolve, reject });
      });
      pendingCommands.set(id, { operation, payload });
      if (onWatchEvent !== undefined) {
        watchCallbacks.set(id, onWatchEvent);
      }
      forwardPendingRequest(id, { operation, payload });
      return response.then(value => ({ id, response: value }));
    }

    /** Removes a callback after a watch stream has been explicitly closed. */
    function forgetWatchSubscription(subscriptionId: string): void {
      const requestId = watchSubscriptions.get(subscriptionId);
      if (requestId !== undefined) {
        watchSubscriptions.delete(subscriptionId);
        watchCallbacks.delete(requestId);
        pendingCommands.delete(requestId);
      }
    }

    /** Releases this page's client routes and, when applicable, its origin-wide ownership lock. */
    function disconnectRuntimeWorker(event: PageTransitionEvent): void {
      if (event.persisted || closed) {
        return;
      }
      closed = true;
      if (runtimeWorker !== null) {
        disconnectLeaderClient(clientId);
      } else {
        coordinator.postMessage({ type: "clientDisconnect", clientId });
      }
      coordinator.close();
      void releaseLeadershipAfterShutdown();
    }

    /** Closes the owner Worker before releasing the origin-wide Web Lock. */
    async function releaseLeadershipAfterShutdown(): Promise<void> {
      const worker = runtimeWorker;
      const release = releaseLeadership;
      if (worker === null || release === null) {
        return;
      }
      await shutdownRuntimeWorker(worker);
      release();
    }

    /** Cancels routes and synchronous Host phases belonging to a closed browser page. */
    function disconnectLeaderClient(disconnectedClientId: string): void {
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

    /** Reports whether a queued Core operation still belongs to a disconnected page. */
    function leaderClientHasDisconnectedRoute(disconnectedClientId: string): boolean {
      for (const route of leaderRequests.values()) {
        if (route.clientId === disconnectedClientId && route.disconnected) {
          return true;
        }
      }
      return false;
    }

    /** Closes one Watch belonging to a page that has already disconnected. */
    function closeDisconnectedWatch(disconnectedClientId: string, subscriptionId: string): void {
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

    /** Wakes a blocked Dedicated Worker with a concrete disconnected-page Host error. */
    function respondToDisconnectedHostCall(id: number, controlBuffer: SharedArrayBuffer): void {
      const payload = MessagePack.encode([1, "web host page disconnected"]);
      disconnectedHostPayloads.set(id, payload);
      const control = new Int32Array(controlBuffer);
      Atomics.store(control, 1, payload.byteLength);
      Atomics.store(control, 0, 1);
      Atomics.notify(control, 0);
    }

    /** Completes the Dedicated Worker's second Host payload phase after page disconnection. */
    function completeDisconnectedHostPayload(
      id: number,
      payloadBuffer: SharedArrayBuffer,
      controlBuffer: SharedArrayBuffer | undefined,
      payload: Uint8Array,
    ): void {
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

    /** Returns the globally routed request map for one client page. */
    function leaderRequestIdsFor(targetClientId: string): Map<number, number> {
      let requests = leaderRequestIds.get(targetClientId);
      if (requests === undefined) {
        requests = new Map<number, number>();
        leaderRequestIds.set(targetClientId, requests);
      }
      return requests;
    }

    /** Returns the active Watch map for one client page. */
    function leaderWatchRequestIdsFor(targetClientId: string): Map<string, number> {
      let watches = leaderWatchRequestIds.get(targetClientId);
      if (watches === undefined) {
        watches = new Map<string, number>();
        leaderWatchRequestIds.set(targetClientId, watches);
      }
      return watches;
    }

    /** Removes one terminal request from the owner routing tables. */
    function forgetLeaderRequest(runtimeRequestId: number, route: RuntimeWorkerLeaderRequestRoute): void {
      leaderRequests.delete(runtimeRequestId);
      const requests = leaderRequestIds.get(route.clientId);
      requests?.delete(route.clientRequestId);
      if (requests?.size === 0) {
        leaderRequestIds.delete(route.clientId);
      }
    }

    /** Rejects local requests when acquiring or maintaining the Web Lock fails. */
    function rejectRuntimeRequests(error: unknown): void {
      const failure = error instanceof Error ? error : new Error(String(error));
      for (const pending of pendingRequests.values()) {
        pending.reject(failure);
      }
      pendingRequests.clear();
      pendingCommands.clear();
      watchCallbacks.clear();
    }

    /** Validates a Core request transported between browser pages. */
    function isLeaderCoreRequest(value: unknown): value is RuntimeWorkerLeaderRequest {
      if (typeof value !== "object" || value === null) {
        return false;
      }
      const request = value as Partial<RuntimeWorkerLeaderRequest>;
      return request.type === "coreRequest" &&
        typeof request.clientId === "string" &&
        typeof request.clientRequestId === "number" &&
        typeof request.operation === "string" &&
        (request.payload instanceof Uint8Array || typeof request.payload === "string");
    }

    /** Decodes a successful Watch subscription response emitted by the Core protocol. */
    function watchSubscriptionId(response: Uint8Array | undefined): string | null {
      if (response === undefined) {
        return null;
      }
      const decoded = MessagePack.decode(response);
      return Array.isArray(decoded) && decoded.length === 2 && decoded[0] === 0 && typeof decoded[1] === "string"
        ? decoded[1]
        : null;
    }

    runtimeGlobal.__operitRuntime = {
      /** Returns the browser Host's default logical storage roots. */
      async runtimeStorageDefaults(): Promise<RuntimeStoragePaths> {
        return defaultRuntimeStoragePaths();
      },
      /** Normalizes explicit logical storage roots through the browser Host. */
      async runtimeStoragePaths(runtimeRoot: string, workspaceRoot: string): Promise<RuntimeStoragePaths> {
        return normalizeRuntimeStoragePaths(runtimeRoot, workspaceRoot);
      },
      /** Installs identity-isolated logical roots on the browser Host. */
      async setRuntimeStorageRoots(runtimeRoot: string, workspaceRoot: string): Promise<void> {
        installRuntimeStoragePaths(runtimeRoot, workspaceRoot);
      },
      /** Reads the local browser bootstrap record. */
      async runtimeBootstrapRead(): Promise<string> {
        return readRuntimeBootstrapConfig();
      },
      /** Writes the local browser bootstrap record. */
      async runtimeBootstrapWrite(content: string): Promise<void> {
        writeRuntimeBootstrapConfig(content);
      },
      /** Reloads the browser after a bootstrap identity change. */
      async restartApplication(): Promise<void> {
        restartRuntimeApplication();
      },
      async call(requestBytes: Uint8Array): Promise<Uint8Array> {
        return (await request("call", requestBytes)).response;
      },
      async controlCall(requestBytes: Uint8Array): Promise<Uint8Array> {
        return (await request("controlCall", requestBytes)).response;
      },
      async pushOpen(requestBytes: Uint8Array): Promise<Uint8Array> {
        return (await request("pushOpen", requestBytes)).response;
      },
      async pushItem(item: Uint8Array): Promise<Uint8Array> {
        return (await request("pushItem", item)).response;
      },
      async pushClose(pushId: string): Promise<Uint8Array> {
        return (await request("pushClose", pushId)).response;
      },
      async watchSnapshot(requestBytes: Uint8Array): Promise<Uint8Array> {
        return (await request("watchSnapshot", requestBytes)).response;
      },
      async watchStream(
        requestBytes: Uint8Array,
        onEvent: (event: Uint8Array) => void,
      ): Promise<Uint8Array> {
        const result = await request("watchStream", requestBytes, onEvent);
        const subscriptionId = watchSubscriptionId(result.response);
        if (subscriptionId !== null) {
          watchSubscriptions.set(subscriptionId, result.id);
        }
        return result.response;
      },
      async closeWatchStream(subscriptionId: string): Promise<Uint8Array> {
        const response = (await request("closeWatchStream", subscriptionId)).response;
        forgetWatchSubscription(subscriptionId);
        return response;
      },
    };
  }

  /** Executes one synchronous host method requested by the dedicated runtime worker. */
  async function handleRuntimeWorkerHostCall(
    message: RuntimeWorkerHostCall,
    hostPayloads: Map<number, Uint8Array>,
  ): Promise<void> {
    let encoded: Uint8Array;
    try {
      const host = runtimeGlobal.__operitHost as Record<string, Record<string, unknown>> | undefined;
      const module = host?.[message.module];
      const method = module?.[message.method];
      if (typeof method !== "function") {
        throw new Error(`web host method is not available: ${message.module}.${message.method}`);
      }
      const value = await method.apply(module, message.args);
      encoded = MessagePack.encode([0, value === undefined ? null : value]);
    } catch (error) {
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

  /** Copies one prepared host response into the worker-provided shared buffer. */
  function handleRuntimeWorkerHostPayload(
    message: RuntimeWorkerHostPayload,
    hostPayloads: Map<number, Uint8Array>,
  ): void {
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

  /** Converts one local runtime worker exception into a stable host error message. */
  function runtimeWorkerErrorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  if (isMainRuntimeHost()) {
    installRuntimeWorkerProxy();
    return;
  }

  runtimeGlobal.__operitRuntime = {
    /** Returns the browser Host's default logical storage roots. */
    async runtimeStorageDefaults(): Promise<RuntimeStoragePaths> {
      return defaultRuntimeStoragePaths();
    },
    /** Normalizes explicit logical storage roots through the browser Host. */
    async runtimeStoragePaths(runtimeRoot: string, workspaceRoot: string): Promise<RuntimeStoragePaths> {
      return normalizeRuntimeStoragePaths(runtimeRoot, workspaceRoot);
    },
    /** Installs identity-isolated logical roots on the browser Host. */
    async setRuntimeStorageRoots(runtimeRoot: string, workspaceRoot: string): Promise<void> {
      installRuntimeStoragePaths(runtimeRoot, workspaceRoot);
    },
    /** Reads the local browser bootstrap record. */
    async runtimeBootstrapRead(): Promise<string> {
      return readRuntimeBootstrapConfig();
    },
    /** Writes the local browser bootstrap record. */
    async runtimeBootstrapWrite(content: string): Promise<void> {
      writeRuntimeBootstrapConfig(content);
    },
    /** Reloads the browser after a bootstrap identity change. */
    async restartApplication(): Promise<void> {
      restartRuntimeApplication();
    },
    async call(request: Uint8Array): Promise<Uint8Array> {
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
    async controlCall(request: Uint8Array): Promise<Uint8Array> {
      return (await bridge()).call(request);
    },
    async pushOpen(request: Uint8Array): Promise<Uint8Array> {
      return (await bridge()).pushOpen(request);
    },
    async pushItem(item: Uint8Array): Promise<Uint8Array> {
      return (await bridge()).pushItem(item);
    },
    async pushClose(pushId: string): Promise<Uint8Array> {
      return (await bridge()).pushClose(pushId);
    },
    async watchSnapshot(request: Uint8Array): Promise<Uint8Array> {
      return (await bridge()).watchSnapshot(request);
    },
    async watchStream(
      request: Uint8Array,
      onEvent: (event: Uint8Array) => void,
    ): Promise<Uint8Array> {
      return (await bridge()).watchStream(request, onEvent);
    },
    async closeWatchStream(subscriptionId: string): Promise<Uint8Array> {
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
