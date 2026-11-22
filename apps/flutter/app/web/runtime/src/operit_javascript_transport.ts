import { MessagePack } from "./operit_messagepack.js";

export const javascriptFrame = { hostCall: 1, hostResult: 2, result: 3, error: 4 } as const;
const headerBytes = 24, frameBytes = 65_536;
const { load, store, notify, add } = Atomics;

/** Shares one framing protocol between the guest's synchronous calls and Core's async observer. */
export class JavaScriptTransport {
  private readonly header: Int32Array;
  private readonly bytes: Uint8Array;
  private seen = 0;

  /** Binds an existing channel to its producer notification word. */
  constructor(readonly buffer: SharedArrayBuffer, private readonly control: Int32Array) {
    this.header = new Int32Array(buffer, 0, headerBytes / 4);
    this.bytes = new Uint8Array(buffer, headerBytes);
  }

  /** Allocates one command channel independently of the execution deadline. */
  static create(control: Int32Array): JavaScriptTransport {
    return new JavaScriptTransport(new SharedArrayBuffer(headerBytes + frameBytes), control);
  }

  /** Publishes a durable frame or acknowledgement notification. */
  private signal(): void {
    add(this.control, 0, 1);
    notify(this.control, 0);
  }

  /** Yields only while awaiting a producer event, cancellation, or the exact deadline. */
  private *until(ready: () => boolean): Generator<[number, number], void, unknown> {
    for (;;) {
      const version = load(this.control, 0);
      if (load(this.control, 1) !== 0) throw new Error("JavaScript execution was interrupted");
      const deadline = (load(this.control, 2) >>> 0) + (load(this.control, 3) >>> 0) * 0x1_0000_0000;
      const remaining = deadline === 0 ? Infinity : deadline - Date.now();
      if (remaining <= 0) throw new Error("JavaScript execution deadline expired");
      if (ready()) return;
      yield [version, remaining];
    }
  }

  /** Fragments one structured frame and waits for each consumer acknowledgement. */
  private *sending(kind: number, value: unknown): Generator<[number, number], void, unknown> {
    const encoded = MessagePack.encode(value);
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
      yield* this.until(() => load(this.header, 4) === sequence);
    }
  }

  /** Reassembles a complete frame before releasing its final acknowledgement. */
  private *receiving(): Generator<[number, number], { kind: number; value: unknown }, unknown> {
    const chunks: Uint8Array[] = [];
    let length = 0;
    let kind = 0;
    for (;;) {
      yield* this.until(() => load(this.header, 3) !== this.seen);
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
    const encoded = new Uint8Array(length);
    let offset = 0;
    for (const chunk of chunks) {
      encoded.set(chunk, offset);
      offset += chunk.byteLength;
    }
    return { kind, value: MessagePack.decode(encoded) };
  }

  /** Parks only the dedicated guest worker or an explicit synchronous startup boundary. */
  private blocking<T>(operation: Generator<[number, number], T, unknown>): T {
    for (;;) {
      const next = operation.next();
      if (next.done === true) return next.value;
      Atomics.wait(this.control, 0, ...next.value);
    }
  }

  /** Leaves the Core event loop available while the guest executes or awaits a callback. */
  private async cooperative<T>(operation: Generator<[number, number], T, unknown>): Promise<T> {
    for (;;) {
      const next = operation.next();
      if (next.done === true) return next.value;
      await Atomics.waitAsync(this.control, 0, ...next.value).value;
    }
  }

  /** Sends from the dedicated guest or an explicit synchronous startup boundary. */
  send(kind: number, value: unknown): void { this.blocking(this.sending(kind, value)); }
  /** Receives at an explicit synchronous boundary. */
  receive(): { kind: number; value: unknown } { return this.blocking(this.receiving()); }
  /** Sends without blocking the Core Worker. */
  sendAsync(kind: number, value: unknown): Promise<void> { return this.cooperative(this.sending(kind, value)); }
  /** Receives without blocking timers, other realms, or package progress reads. */
  receiveAsync(): Promise<{ kind: number; value: unknown }> { return this.cooperative(this.receiving()); }
}
