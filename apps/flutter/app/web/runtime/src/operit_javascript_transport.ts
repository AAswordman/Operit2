import { MessagePack } from "./operit_messagepack.js";

export const javascriptFrame = { hostCall: 1, hostResult: 2, result: 3, error: 4 } as const;
const headerBytes = 24;
const frameBytes = 65_536;
const load = Atomics.load;
const store = Atomics.store;
const notify = Atomics.notify;
const wait = Atomics.wait;
const add = Atomics.add;
const Bytes = Uint8Array;
const encode = MessagePack.encode;
const decode = MessagePack.decode;
const now = Date.now;

/** Carries structured frames and cancellation through explicit atomic notifications. */
export class JavaScriptTransport {
  private readonly header: Int32Array;
  private readonly bytes: Uint8Array;
  private seen = 0;

  constructor(readonly buffer: SharedArrayBuffer, private readonly control: Int32Array) {
    this.header = new Int32Array(buffer, 0, headerBytes / 4);
    this.bytes = new Bytes(buffer, headerBytes);
  }

  static create(control: Int32Array): JavaScriptTransport {
    return new JavaScriptTransport(new SharedArrayBuffer(headerBytes + frameBytes), control);
  }

  private signal(): void {
    add(this.control, 0, 1);
    notify(this.control, 0);
  }

  /** Waits for a message, acknowledgment, cancellation, or the exact execution deadline. */
  private until(ready: () => boolean): void {
    for (;;) {
      const version = load(this.control, 0);
      if (load(this.control, 1) !== 0) throw new Error("JavaScript execution was interrupted");
      const deadline = (load(this.control, 2) >>> 0) + (load(this.control, 3) >>> 0) * 0x1_0000_0000;
      const remaining = deadline === 0 ? Infinity : deadline - now();
      if (remaining <= 0) throw new Error("JavaScript execution deadline expired");
      if (ready()) return;
      wait(this.control, 0, version, remaining);
    }
  }

  send(kind: number, value: unknown): void {
    const encoded = encode(value);
    for (let offset = 0; offset < encoded.byteLength; offset += frameBytes) {
      const chunk = encoded.subarray(offset, offset + frameBytes);
      this.bytes.set(chunk);
      store(this.header, 0, kind);
      store(this.header, 1, chunk.byteLength);
      store(this.header, 2, offset + chunk.byteLength === encoded.byteLength ? 1 : 0);
      const sequence = add(this.header, 5, 1) + 1;
      this.seen = sequence;
      store(this.header, 3, sequence);
      this.signal();
      this.until(() => load(this.header, 4) === sequence);
    }
  }

  receive(): { kind: number; value: unknown } {
    const chunks: Uint8Array[] = [];
    let length = 0;
    let kind = 0;
    for (;;) {
      this.until(() => load(this.header, 3) !== this.seen);
      const sequence = load(this.header, 3);
      const nextKind = load(this.header, 0);
      const byteLength = load(this.header, 1);
      const final = load(this.header, 2) === 1;
      if (byteLength <= 0 || byteLength > frameBytes || (kind !== 0 && nextKind !== kind)) {
        throw new Error("Invalid JavaScript worker transport frame");
      }
      kind = nextKind;
      chunks.push(this.bytes.slice(0, byteLength));
      length += byteLength;
      this.seen = sequence;
      store(this.header, 4, sequence);
      this.signal();
      if (final) break;
    }
    const encoded = new Bytes(length);
    let offset = 0;
    for (const chunk of chunks) {
      encoded.set(chunk, offset);
      offset += chunk.byteLength;
    }
    return { kind, value: decode(encoded) };
  }
}
