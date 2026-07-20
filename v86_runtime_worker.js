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
const v86RuntimeAssetBaseUrl = "https://models.operit.app/v86-runtime/i686-buildroot-node20-python312-20260720/";
const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
const workerGlobal = globalThis;
let emulator = null;
let outputHeader = null;
let outputBytes = null;
let outputCapacity = 0;
let startupFrames = [];
let queuedInputFrames = [];
let bootLine = "";
let runtimeLineBytes = [];
let guestReady = false;
let terminating = false;
workerGlobal.addEventListener("message", handleWorkerMessage);
function handleWorkerMessage(event) {
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
function isV86RuntimeWorkerMessage(value) {
    if (typeof value !== "object" || value === null) {
        return false;
    }
    const message = value;
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
async function bootGuest(message) {
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
        const module = await import(modulePath);
        const nextEmulator = new module.V86({
            wasm_path: v86AssetUrl("v86.wasm"),
            memory_size: 512 * 1024 * 1024,
            vga_memory_size: 2 * 1024 * 1024,
            bios: { url: v86AssetUrl("seabios.bin") },
            vga_bios: { url: v86AssetUrl("vgabios.bin") },
            bzimage: { url: v86RuntimeAssetUrl("operit-runtime-bzimage.bin") },
            initrd: { url: v86RuntimeAssetUrl("operit-runtime-initrd.cpio.gz") },
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
    }
    catch (error) {
        failGuest(error);
    }
}
function v86AssetUrl(name) {
    return new URL(`./v86/${name}`, import.meta.url).href;
}
function v86RuntimeAssetUrl(name) {
    return new URL(name, v86RuntimeAssetBaseUrl).href;
}
function handleSerialOutputByte(value) {
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
function consumeBootstrapByte(byte) {
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
function consumeRuntimeByte(byte) {
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
function flushStartupFrames() {
    for (const frame of startupFrames) {
        sendSerialLine(frame);
    }
    startupFrames = [];
    for (const frame of queuedInputFrames) {
        sendSerialLine(frame);
    }
    queuedInputFrames = [];
}
function acceptInputFrames(lines) {
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
function sendSerialLine(line) {
    if (emulator === null) {
        failGuest(new Error("V86 runtime serial port is unavailable"));
        return;
    }
    emulator.serial_send_bytes(0, textEncoder.encode(`${line}\n`));
}
function appendOutputByte(byte) {
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
function failGuest(error) {
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
async function stopGuest() {
    if (terminating) {
        return;
    }
    terminating = true;
    const instance = emulator;
    emulator = null;
    if (instance !== null) {
        try {
            await instance.destroy();
        }
        catch (error) {
            if (runtimeState() !== runtimeFailed) {
                failGuest(error);
            }
        }
    }
    if (runtimeState() !== runtimeFailed) {
        setRuntimeState(runtimeStopped);
    }
}
function recordGuestExit(line) {
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
function requiredOutputHeader() {
    if (outputHeader === null) {
        throw new Error("V86 runtime output header is unavailable");
    }
    return outputHeader;
}
function requiredOutputBytes() {
    if (outputBytes === null) {
        throw new Error("V86 runtime output buffer is unavailable");
    }
    return outputBytes;
}
function runtimeState() {
    return outputHeader === null ? runtimeFailed : Atomics.load(outputHeader, runtimeStateIndex);
}
function setRuntimeState(state) {
    const header = outputHeader;
    if (header === null) {
        return;
    }
    Atomics.store(header, runtimeStateIndex, state);
    Atomics.notify(header, outputWriteIndex);
}
export {};
