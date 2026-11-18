const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
export function toBase64(bytes: Uint8Array): string {
  let result = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const n = (bytes[i] << 16) | ((bytes[i + 1] ?? 0) << 8) | (bytes[i + 2] ?? 0);
    result += alphabet[n >>> 18] + alphabet[(n >>> 12) & 63] + (i + 1 < bytes.length ? alphabet[(n >>> 6) & 63] : "=") + (i + 2 < bytes.length ? alphabet[n & 63] : "=");
  }
  return result;
}
export function fromBase64(value: string): Uint8Array {
  if (value.length % 4 !== 0 || /[^A-Za-z0-9+/=]/.test(value)) throw Error("Invalid Feishu frame encoding");
  const result = new Uint8Array(value.length * 3 / 4 - (value.endsWith("==") ? 2 : value.endsWith("=") ? 1 : 0));
  for (let i = 0, j = 0; i < value.length; i += 4) {
    const n = (alphabet.indexOf(value[i]) << 18) | (alphabet.indexOf(value[i + 1]) << 12) | (Math.max(0, alphabet.indexOf(value[i + 2])) << 6) | Math.max(0, alphabet.indexOf(value[i + 3]));
    result[j++] = n >>> 16; result[j++] = n >>> 8; result[j++] = n;
  }
  return result;
}
export function utf8(value: string): Uint8Array {
  const encoded = encodeURIComponent(value), result: number[] = [];
  for (let i = 0; i < encoded.length; i++) {
    if (encoded[i] === "%") { result.push(parseInt(encoded.slice(i + 1, i + 3), 16)); i += 2; }
    else result.push(encoded.charCodeAt(i));
  }
  return Uint8Array.from(result);
}
export function string(bytes: Uint8Array): string {
  let escaped = "";
  for (const b of bytes) escaped += `%${b.toString(16).padStart(2, "0")}`;
  return decodeURIComponent(escaped);
}
type Field = [number, bigint | Uint8Array];
function decode(bytes: Uint8Array): Field[] {
  let offset = 0;
  function read(): bigint {
    let n = 0n;
    for (let i = 0n; i < 70n && offset < bytes.length; i += 7n) {
      const b = bytes[offset++]; n |= BigInt(b & 127) << i;
      if (!(b & 128)) return n;
    }
    throw Error("Invalid Feishu protobuf varint");
  }
  const fields: Field[] = [];
  while (offset < bytes.length) {
    const tag = Number(read()), id = tag >>> 3;
    switch (tag & 7) {
      case 0: fields.push([id, read()]); break;
      case 2: {
        const size = Number(read());
        if (size > bytes.length - offset) throw Error("Truncated Feishu protobuf field");
        fields.push([id, bytes.subarray(offset, offset + size)]); offset += size; break;
      }
      case 1: offset += 8; break;
      case 5: offset += 4; break;
      default: throw Error("Unsupported Feishu protobuf field");
    }
  }
  if (offset !== bytes.length) throw Error("Truncated Feishu protobuf frame");
  return fields;
}
function varint(n: bigint): number[] {
  n = BigInt.asUintN(64, n);
  const bytes: number[] = [];
  while (n >= 128n) { bytes.push(Number(n & 127n) | 128); n >>= 7n; }
  bytes.push(Number(n)); return bytes;
}
function encode(fields: Field[]): Uint8Array {
  const bytes: number[] = [];
  for (const [id, value] of fields) {
    if (typeof value === "bigint") bytes.push(...varint(BigInt(id * 8)), ...varint(value));
    else { bytes.push(...varint(BigInt(id * 8 + 2)), ...varint(BigInt(value.length))); for (const b of value) bytes.push(b); }
  }
  return Uint8Array.from(bytes);
}
export interface Frame { seq: bigint; log: bigint; service: number; method: number; headers: Record<string, string>; payload: Uint8Array; extra?: Field[] }
export function readFrame(base64: string): Frame {
  const fields = decode(fromBase64(base64)), headers: Record<string, string> = Object.create(null);
  const field = (id: number) => fields.find(f => f[0] === id)?.[1];
  const uint = (id: number) => { const value = field(id); if (typeof value !== "bigint") throw Error("Invalid Feishu frame header"); return value; };
  for (const [id, bytes] of fields) if (id === 5 && bytes instanceof Uint8Array) {
    const header = decode(bytes), key = header.find(f => f[0] === 1)?.[1], value = header.find(f => f[0] === 2)?.[1];
    if (key instanceof Uint8Array && value instanceof Uint8Array) headers[string(key)] = string(value);
  }
  const payload = field(8);
  return { seq: uint(1), log: uint(2), service: Number(BigInt.asIntN(32, uint(3))), method: Number(uint(4)), headers,
    payload: payload instanceof Uint8Array ? payload : new Uint8Array(), extra: fields.filter(([id]) => id === 9) };
}
export function writeFrame(frame: Frame): string {
  const fields: Field[] = [[1, frame.seq], [2, frame.log], [3, BigInt(frame.service)], [4, BigInt(frame.method)]];
  for (const [key, value] of Object.entries(frame.headers)) fields.push([5, encode([[1, utf8(key)], [2, utf8(value)]])]);
  if (frame.payload.length) fields.push([8, frame.payload]);
  return toBase64(encode([...fields, ...(frame.extra ?? [])]));
}

export class Fragments {
  private readonly cache = new Map<string, { parts: Map<number, Uint8Array>; total: number; expires: number }>();
  join(frame: Frame): Uint8Array | null {
    const total = Number(frame.headers.sum ?? 1), index = Number(frame.headers.seq ?? 0), id = frame.headers.message_id;
    if (!Number.isInteger(total) || total < 1 || total > 32 || !Number.isInteger(index) || index < 0 || index >= total) throw Error("Invalid Feishu fragment metadata");
    if (total === 1) return frame.payload;
    if (!id) throw Error("Feishu fragmented message missing ID");
    for (const [key, value] of this.cache) if (value.expires < Date.now()) this.cache.delete(key);
    if (this.cache.size >= 32 && !this.cache.has(id)) throw Error("Too many Feishu fragments");
    const entry = this.cache.get(id) ?? { parts: new Map(), total, expires: Date.now() + 15000 };
    if (entry.total !== total) { this.cache.delete(id); throw Error("Feishu fragment total changed"); }
    entry.parts.set(index, frame.payload); this.cache.set(id, entry);
    const size = [...entry.parts.values()].reduce((sum, part) => sum + part.length, 0);
    if (size > 1024 * 1024) { this.cache.delete(id); throw Error("Feishu event exceeds 1 MiB"); }
    if (entry.parts.size !== total) return null;
    const merged = new Uint8Array(size);
    for (let cursor = 0, i = 0; i < total; i++) { const part = entry.parts.get(i)!; merged.set(part, cursor); cursor += part.length; }
    this.cache.delete(id); return merged;
  }
}
