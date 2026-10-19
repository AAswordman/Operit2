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

/** Carries structured values in acknowledged frames without a fixed message size limit. */
export class JavaScriptTransport {
  private readonly header: Int32Array;
  private readonly bytes: Uint8Array;
  private seen = 0;

  /** Attaches one endpoint to the command-owned shared buffer. */
  constructor(readonly buffer: SharedArrayBuffer) {
    this.header = new Int32Array(buffer, 0, headerBytes / 4);
    this.bytes = new Bytes(buffer, headerBytes);
  }

  /** Allocates the bounded reusable frame storage for one command. */
  static create(): JavaScriptTransport {
    return new JavaScriptTransport(new SharedArrayBuffer(headerBytes + frameBytes));
  }

  /** Publishes each encoded frame and waits until its receiver has copied it. */
  send(kind: number, value: unknown, check: () => void): void {
    const encoded = encode(value);
    for (let offset = 0; offset < encoded.byteLength; offset += frameBytes) {
      check();
      const chunk = encoded.subarray(offset, offset + frameBytes);
      this.bytes.set(chunk);
      store(this.header, 0, kind);
      store(this.header, 1, chunk.byteLength);
      store(this.header, 2, offset + chunk.byteLength === encoded.byteLength ? 1 : 0);
      const sequence = add(this.header, 5, 1) + 1;
      this.seen = sequence;
      store(this.header, 3, sequence);
      notify(this.header, 3);
      while (load(this.header, 4) !== sequence) {
        check();
        wait(this.header, 4, load(this.header, 4), 5);
      }
    }
  }

  /** Receives a complete structured message and acknowledges every copied frame. */
  receive(check: () => void): { kind: number; value: unknown } {
    const chunks: Uint8Array[] = [];
    let length = 0;
    let kind = 0;
    for (;;) {
      while (load(this.header, 3) === this.seen) {
        check();
        wait(this.header, 3, this.seen, 5);
      }
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
      notify(this.header, 4);
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
