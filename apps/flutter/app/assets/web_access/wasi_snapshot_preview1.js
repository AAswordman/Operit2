let wasmMemory;

const WASI_ERRNO_BAD_FILE_DESCRIPTOR = 8;
const WASI_ERRNO_ILLEGAL_SEEK = 70;
const WASI_FILETYPE_CHARACTER_DEVICE = 2;
const WASI_STDOUT_FILE_DESCRIPTOR = 1;
const WASI_STDERR_FILE_DESCRIPTOR = 2;

/** Stores the WebAssembly linear memory used by WASI Preview 1 imports. */
export function setWasiMemory(memory) {
  if (!(memory instanceof WebAssembly.Memory)) {
    throw new TypeError("WASI memory must be a WebAssembly.Memory instance.");
  }
  wasmMemory = memory;
}

/** Returns a current view over the bridge WebAssembly linear memory. */
function wasiMemoryView() {
  if (wasmMemory === undefined) {
    throw new Error("WASI memory has not been initialized.");
  }
  return new DataView(wasmMemory.buffer);
}

/** Identifies the process standard input, output, and error file descriptors. */
function isStandardFileDescriptor(fileDescriptor) {
  return fileDescriptor >= 0 && fileDescriptor <= WASI_STDERR_FILE_DESCRIPTOR;
}

/** Writes the current browser clock as a WASI nanosecond timestamp. */
export function clock_time_get(clockId, _precision, resultPointer) {
  const nowMilliseconds = clockId === 1
    ? performance.timeOrigin + performance.now()
    : Date.now();
  wasiMemoryView().setBigUint64(
    resultPointer,
    BigInt(Math.trunc(nowMilliseconds)) * 1000000n,
    true,
  );
  return 0;
}

/** Closes a browser standard stream descriptor. */
export function fd_close(fileDescriptor) {
  return isStandardFileDescriptor(fileDescriptor)
    ? 0
    : WASI_ERRNO_BAD_FILE_DESCRIPTOR;
}

/** Describes a browser standard stream as a WASI character device. */
export function fd_fdstat_get(fileDescriptor, resultPointer) {
  if (!isStandardFileDescriptor(fileDescriptor)) {
    return WASI_ERRNO_BAD_FILE_DESCRIPTOR;
  }
  const view = wasiMemoryView();
  new Uint8Array(wasmMemory.buffer, resultPointer, 24).fill(0);
  view.setUint8(resultPointer, WASI_FILETYPE_CHARACTER_DEVICE);
  return 0;
}

/** Reports that browser standard streams do not support file positioning. */
export function fd_seek(fileDescriptor, _offset, _whence, _resultPointer) {
  return isStandardFileDescriptor(fileDescriptor)
    ? WASI_ERRNO_ILLEGAL_SEEK
    : WASI_ERRNO_BAD_FILE_DESCRIPTOR;
}

/** Writes WASI standard output and error iovecs to the browser console. */
export function fd_write(fileDescriptor, iovecsPointer, iovecsLength, resultPointer) {
  if (
    fileDescriptor !== WASI_STDOUT_FILE_DESCRIPTOR &&
    fileDescriptor !== WASI_STDERR_FILE_DESCRIPTOR
  ) {
    return WASI_ERRNO_BAD_FILE_DESCRIPTOR;
  }
  const view = wasiMemoryView();
  const buffers = [];
  let byteLength = 0;
  for (let index = 0; index < iovecsLength; index += 1) {
    const iovecPointer = iovecsPointer + index * 8;
    const bufferPointer = view.getUint32(iovecPointer, true);
    const bufferLength = view.getUint32(iovecPointer + 4, true);
    buffers.push(new Uint8Array(wasmMemory.buffer, bufferPointer, bufferLength));
    byteLength += bufferLength;
  }
  const outputBytes = new Uint8Array(byteLength);
  let outputOffset = 0;
  for (const buffer of buffers) {
    outputBytes.set(buffer, outputOffset);
    outputOffset += buffer.length;
  }
  const output = new TextDecoder().decode(outputBytes);
  if (fileDescriptor === WASI_STDERR_FILE_DESCRIPTOR) {
    console.error(output);
  } else {
    console.log(output);
  }
  view.setUint32(resultPointer, byteLength, true);
  return 0;
}
