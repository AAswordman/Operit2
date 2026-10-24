let wasmMemory: WebAssembly.Memory | undefined;
const BAD_DESCRIPTOR = 8;
const ILLEGAL_SEEK = 70;

/** Installs the linear memory required by imported WASI Preview 1 functions. */
export function setWasiMemory(memory: WebAssembly.Memory): void {
  if (!(memory instanceof WebAssembly.Memory)) throw new TypeError('WASI requires WebAssembly memory');
  wasmMemory = memory;
}

/** Requires initialized memory instead of constructing a substitute runtime. */
function memory(): WebAssembly.Memory {
  if (wasmMemory === undefined) throw new Error('WASI memory has not been initialized');
  return wasmMemory;
}

/** Recognizes only the three process-standard stream descriptors. */
function standardDescriptor(descriptor: number): boolean { return Number.isInteger(descriptor) && descriptor >= 0 && descriptor <= 2; }

/** Supplies the browser wall clock or monotonic clock in nanoseconds. */
export function clock_time_get(clock: number, _precision: bigint, result: number): number {
  const milliseconds = clock === 1 ? performance.timeOrigin + performance.now() : Date.now();
  new DataView(memory().buffer).setBigUint64(result, BigInt(Math.trunc(milliseconds)) * 1000000n, true);
  return 0;
}

/** Closes an existing standard stream descriptor at the browser Host boundary. */
export function fd_close(descriptor: number): number { return standardDescriptor(descriptor) ? 0 : BAD_DESCRIPTOR; }

/** Identifies standard streams as character devices to the WASI guest. */
export function fd_fdstat_get(descriptor: number, result: number): number {
  if (!standardDescriptor(descriptor)) return BAD_DESCRIPTOR;
  new Uint8Array(memory().buffer, result, 24).fill(0);
  new DataView(memory().buffer).setUint8(result, 2);
  return 0;
}

/** Rejects seeking on process-standard streams. */
export function fd_seek(descriptor: number, _offset: bigint, _whence: number, _result: number): number {
  return standardDescriptor(descriptor) ? ILLEGAL_SEEK : BAD_DESCRIPTOR;
}

/** Writes guest diagnostic buffers only to the corresponding browser console stream. */
export function fd_write(descriptor: number, vectors: number, count: number, result: number): number {
  if (descriptor !== 1 && descriptor !== 2) return BAD_DESCRIPTOR;
  const view = new DataView(memory().buffer);
  const decoder = new TextDecoder();
  let size = 0;
  for (let index = 0; index < count; index += 1) {
    const pointer = view.getUint32(vectors + index * 8, true);
    const length = view.getUint32(vectors + index * 8 + 4, true);
    const output = decoder.decode(new Uint8Array(memory().buffer, pointer, length), { stream: index + 1 < count });
    if (descriptor === 2) console.error(output); else console.log(output);
    size += length;
  }
  view.setUint32(result, size, true);
  return 0;
}
