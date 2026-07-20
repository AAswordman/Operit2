export {};

interface V86StarterConfiguration {
  wasm_path: string;
  memory_size: number;
  vga_memory_size: number;
  bios: { url: string };
  vga_bios: { url: string };
  bzimage: { url: string };
  initrd: { url: string };
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

interface V86RuntimeBootMessage {
  type: "boot";
  serialBuffer: SharedArrayBuffer;
  outputCapacity: number;
  startupFrames: string[];
}

interface V86RuntimeInputMessage {
  type: "input";
  lines: string[];
}

interface V86RuntimeKillMessage {
  type: "kill";
}

type V86RuntimeWorkerMessage = V86RuntimeBootMessage | V86RuntimeInputMessage | V86RuntimeKillMessage;

const outputWriteIndex = 0;
const outputReadIndex = 1;
const runtimeStateIndex = 2;
const runtimeExitCodeIndex = 3;
const runtimeHeaderLength = 4;
const runtimeStarting = 0;
const runtimeRunning = 1;
const runtimeFailed = 2;
const runtimeStopped = 3;
const readyMarker = "OPERIT_RUNTIME_READY";
const exitMarkerPrefix = "OPERIT_RUNTIME_EXIT:";
const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
const workerGlobal = globalThis as typeof globalThis;

let emulator: V86StarterInstance | null = null;
let outputHeader: Int32Array | null = null;
let outputBytes: Uint8Array | null = null;
let outputCapacity = 0;
let startupFrames: string[] = [];
let queuedInputFrames: string[] = [];
let bootLine = "";
let runtimeLineBytes: number[] = [];
let guestReady = false;
let terminating = false;

workerGlobal.addEventListener("message", handleWorkerMessage);

/** Routes one managed-runtime command from the browser bridge. */
function handleWorkerMessage(event: MessageEvent<unknown>): void {
  const message = event.data;
  if (!isV86RuntimeWorkerMessage(message)) {
    return;
  }
  if (message.type === "boot") {
    void bootGuest(message);
    return;
  }
  if (message.type === "input") {
    acceptInputFrames(message.lines);
    return;
  }
  void stopGuest();
}

/** Validates one command sent to this dedicated V86 runtime worker. */
function isV86RuntimeWorkerMessage(value: unknown): value is V86RuntimeWorkerMessage {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const message = value as Partial<V86RuntimeWorkerMessage>;
  if (message.type === "boot") {
    return message.serialBuffer instanceof SharedArrayBuffer &&
      typeof message.outputCapacity === "number" &&
      Array.isArray(message.startupFrames) &&
      message.startupFrames.every(frame => typeof frame === "string");
  }
  if (message.type === "input") {
    return Array.isArray(message.lines) && message.lines.every(line => typeof line === "string");
  }
  return message.type === "kill";
}

/** Boots the isolated Buildroot runtime suite and attaches its first serial port. */
async function bootGuest(message: V86RuntimeBootMessage): Promise<void> {
  try {
    if (emulator !== null || outputHeader !== null) {
      throw new Error("V86 runtime worker is already booted");
    }
    outputCapacity = message.outputCapacity;
    outputHeader = new Int32Array(message.serialBuffer, 0, runtimeHeaderLength);
    outputBytes = new Uint8Array(message.serialBuffer, runtimeHeaderLength * Int32Array.BYTES_PER_ELEMENT);
    if (outputBytes.length !== outputCapacity) {
      throw new Error("V86 runtime serial buffer capacity is invalid");
    }
    Atomics.store(outputHeader, outputWriteIndex, 0);
    Atomics.store(outputHeader, outputReadIndex, 0);
    Atomics.store(outputHeader, runtimeExitCodeIndex, -1);
    Atomics.store(outputHeader, runtimeStateIndex, runtimeStarting);
    startupFrames = message.startupFrames.slice();
    const modulePath = new URL("./v86/libv86.mjs", import.meta.url).href;
    const module = await import(modulePath) as unknown as V86Module;
    const nextEmulator = new module.V86({
      wasm_path: v86AssetUrl("v86.wasm"),
      memory_size: 512 * 1024 * 1024,
      vga_memory_size: 2 * 1024 * 1024,
      bios: { url: v86AssetUrl("seabios.bin") },
      vga_bios: { url: v86AssetUrl("vgabios.bin") },
      bzimage: { url: v86AssetUrl("runtime/operit-runtime-bzimage.bin") },
      initrd: { url: v86AssetUrl("runtime/operit-runtime-initrd.cpio.gz") },
      cmdline: "console=ttyS0 operit.mode=agent tsc=reliable mitigations=off random.trust_cpu=on",
      autostart: true,
      disable_keyboard: true,
      disable_mouse: true,
      disable_speaker: true,
    });
    emulator = nextEmulator;
    nextEmulator.add_listener("serial0-output-byte", handleSerialOutputByte);
    nextEmulator.add_listener("download-error", value => {
      failGuest(new Error(`V86 runtime asset download failed: ${String(value)}`));
    });
    nextEmulator.add_listener("emulator-stopped", () => {
      if (!terminating) {
        setRuntimeState(runtimeStopped);
      }
    });
  } catch (error) {
    failGuest(error);
  }
}

/** Produces a URL for one V86 asset relative to this worker module. */
function v86AssetUrl(name: string): string {
  return new URL(`./v86/${name}`, import.meta.url).href;
}

/** Processes one serial byte and removes guest bootstrap control frames. */
function handleSerialOutputByte(value: unknown): void {
  if (typeof value !== "number") {
    return;
  }
  const byte = value & 0xff;
  if (!guestReady) {
    consumeBootstrapByte(byte);
    return;
  }
  consumeRuntimeByte(byte);
}

/** Detects the guest agent handshake without exposing boot logs to MCP stdout. */
function consumeBootstrapByte(byte: number): void {
  if (byte === 10) {
    const line = bootLine.replace(/\r$/, "");
    bootLine = "";
    if (line === readyMarker) {
      guestReady = true;
      setRuntimeState(runtimeRunning);
      flushStartupFrames();
    }
    return;
  }
  if (byte !== 13 && bootLine.length < 4096) {
    bootLine += textDecoder.decode(Uint8Array.of(byte));
  }
}

/** Filters guest lifecycle control lines while preserving runtime stdout byte-for-byte. */
function consumeRuntimeByte(byte: number): void {
  runtimeLineBytes.push(byte);
  if (runtimeLineBytes.length > outputCapacity) {
    failGuest(new Error("V86 runtime emitted a serial line larger than its shared output buffer"));
    return;
  }
  if (byte !== 10) {
    return;
  }
  const lineBytes = Uint8Array.from(runtimeLineBytes);
  runtimeLineBytes = [];
  const decoded = textDecoder.decode(lineBytes).replace(/\r?\n$/, "");
  if (recordGuestExit(decoded)) {
    return;
  }
  for (const outputByte of lineBytes) {
    appendOutputByte(outputByte);
  }
}

/** Sends all precomputed deployment frames after the guest agent is listening. */
function flushStartupFrames(): void {
  for (const frame of startupFrames) {
    sendSerialLine(frame);
  }
  startupFrames = [];
  for (const frame of queuedInputFrames) {
    sendSerialLine(frame);
  }
  queuedInputFrames = [];
}

/** Delivers runtime stdin frames immediately after agent startup or queues them during boot. */
function acceptInputFrames(lines: string[]): void {
  if (runtimeState() === runtimeFailed || runtimeState() === runtimeStopped) {
    return;
  }
  if (!guestReady) {
    queuedInputFrames.push(...lines);
    return;
  }
  for (const line of lines) {
    sendSerialLine(line);
  }
}

/** Writes one newline-terminated protocol frame into the V86 serial port. */
function sendSerialLine(line: string): void {
  if (emulator === null) {
    failGuest(new Error("V86 runtime serial port is unavailable"));
    return;
  }
  emulator.serial_send_bytes(0, textEncoder.encode(`${line}\n`));
}

/** Appends one guest serial byte to the shared output ring. */
function appendOutputByte(byte: number): void {
  const header = requiredOutputHeader();
  const bytes = requiredOutputBytes();
  const writeIndex = Atomics.load(header, outputWriteIndex);
  const readIndex = Atomics.load(header, outputReadIndex);
  if (writeIndex - readIndex >= outputCapacity) {
    failGuest(new Error("V86 runtime stdout exceeded its shared serial buffer"));
    return;
  }
  bytes[writeIndex % outputCapacity] = byte;
  Atomics.store(header, outputWriteIndex, writeIndex + 1);
  Atomics.notify(header, outputWriteIndex);
}

/** Marks the process as failed and emits its diagnostic through the output ring. */
function failGuest(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  const header = outputHeader;
  if (header === null || runtimeState() === runtimeFailed) {
    return;
  }
  const bytes = textEncoder.encode(`[V86 runtime failed: ${message}]\n`);
  for (const byte of bytes) {
    const writeIndex = Atomics.load(header, outputWriteIndex);
    const readIndex = Atomics.load(header, outputReadIndex);
    if (writeIndex - readIndex >= outputCapacity) {
      break;
    }
    requiredOutputBytes()[writeIndex % outputCapacity] = byte;
    Atomics.store(header, outputWriteIndex, writeIndex + 1);
  }
  setRuntimeState(runtimeFailed);
  void stopGuest();
}

/** Stops the V86 instance after a process exit or explicit managed-process kill. */
async function stopGuest(): Promise<void> {
  if (terminating) {
    return;
  }
  terminating = true;
  const instance = emulator;
  emulator = null;
  if (instance !== null) {
    try {
      await instance.destroy();
    } catch (error) {
      if (runtimeState() !== runtimeFailed) {
        failGuest(error);
      }
    }
  }
  if (runtimeState() !== runtimeFailed) {
    setRuntimeState(runtimeStopped);
  }
}

/** Records the guest process exit code advertised by the serial agent. */
function recordGuestExit(line: string): boolean {
  if (!line.startsWith(exitMarkerPrefix)) {
    return false;
  }
  const exitCode = Number(line.slice(exitMarkerPrefix.length));
  if (!Number.isInteger(exitCode)) {
    failGuest(new Error("V86 runtime emitted an invalid process exit code"));
    return true;
  }
  const header = requiredOutputHeader();
  Atomics.store(header, runtimeExitCodeIndex, exitCode);
  setRuntimeState(runtimeStopped);
  void stopGuest();
  return true;
}

/** Returns the shared header after the worker has initialized its serial transport. */
function requiredOutputHeader(): Int32Array {
  if (outputHeader === null) {
    throw new Error("V86 runtime output header is unavailable");
  }
  return outputHeader;
}

/** Returns the shared output bytes after the worker has initialized its serial transport. */
function requiredOutputBytes(): Uint8Array {
  if (outputBytes === null) {
    throw new Error("V86 runtime output buffer is unavailable");
  }
  return outputBytes;
}

/** Reads the current lifecycle state from the shared serial header. */
function runtimeState(): number {
  return outputHeader === null ? runtimeFailed : Atomics.load(outputHeader, runtimeStateIndex);
}

/** Stores one lifecycle state and wakes main-thread polling readers. */
function setRuntimeState(state: number): void {
  const header = outputHeader;
  if (header === null) {
    return;
  }
  Atomics.store(header, runtimeStateIndex, state);
  Atomics.notify(header, outputWriteIndex);
}
