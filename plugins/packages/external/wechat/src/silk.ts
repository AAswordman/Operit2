const SAMPLE_RATE = 24_000;
const MAX_SAMPLES = 120 * SAMPLE_RATE;
const MAX_INPUT_BYTES = 2 * 1024 * 1024;
const BASE64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

export function decodeBase64(value: string): Uint8Array {
  const padding = value.endsWith("==") ? 2 : value.endsWith("=") ? 1 : 0;
  if (value.length % 4) throw new Error("Invalid base64 media data");
  const bytes = new Uint8Array((value.length / 4) * 3 - padding);
  for (let source = 0, dest = 0; source < value.length; source += 4) {
    const a = BASE64.indexOf(value[source]);
    const b = BASE64.indexOf(value[source + 1]);
    const hasThird = source + 2 < value.length - padding;
    const hasFourth = source + 3 < value.length - padding;
    const c = hasThird ? BASE64.indexOf(value[source + 2]) : 0;
    const d = hasFourth ? BASE64.indexOf(value[source + 3]) : 0;
    if (Math.min(a, b, c, d) < 0) throw new Error("Invalid base64 media data");
    bytes[dest++] = (a << 2) | (b >> 4);
    if (hasThird) bytes[dest++] = (b << 4) | (c >> 2);
    if (hasFourth) bytes[dest++] = (c << 6) | d;
  }
  return bytes;
}

function encodeBase64(bytes: Uint8Array): string {
  const parts: string[] = [];
  for (let start = 0; start < bytes.length; start += 12_288) {
    let part = "";
    for (let i = start; i < Math.min(start + 12_288, bytes.length); i += 3) {
      const a = bytes[i], b = bytes[i + 1] ?? 0, c = bytes[i + 2] ?? 0;
      part += BASE64[a >> 2] + BASE64[((a & 3) << 4) | (b >> 4)] +
        (i + 1 < bytes.length ? BASE64[((b & 15) << 2) | (c >> 6)] : "=") +
        (i + 2 < bytes.length ? BASE64[c & 63] : "=");
    }
    parts.push(part);
  }
  return parts.join("");
}

export function isSilkHeader(prefix: Uint8Array): boolean {
  const start = prefix[0] === 2 ? 1 : 0;
  return String.fromCharCode(...prefix.subarray(start, start + 9)) === "#!SILK_V3";
}

function wav(pcm: Uint8Array): Uint8Array {
  const bytes = new Uint8Array(44 + pcm.length);
  const view = new DataView(bytes.buffer);
  view.setUint32(0, 0x52494646); // RIFF
  view.setUint32(4, bytes.length - 8, true);
  view.setUint32(8, 0x57415645); // WAVE
  view.setUint32(12, 0x666d7420); // fmt
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, SAMPLE_RATE, true);
  view.setUint32(28, SAMPLE_RATE * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  view.setUint32(36, 0x64617461); // data
  view.setUint32(40, pcm.length, true);
  bytes.set(pcm, 44);
  return bytes;
}

/** Decodes only the Chat copy; callers keep the original on unsupported audio. */
export async function silkToWav(base64: string): Promise<string | null> {
  if (base64.length > Math.ceil(MAX_INPUT_BYTES / 3) * 4) return null;
  let decoder: ToolPkg.WasmInstance | undefined;
  try {
    const source = decodeBase64(base64);
    if (source.length > MAX_INPUT_BYTES || !isSilkHeader(source)) return null;
    decoder = await ToolPkg.wasm.open("silk-decoder");
    const call = (name: string, ...values: number[]) => decoder!.call(name, values.map(value => ({ type: "i32", value })));
    call("_initialize");
    const input = Number(call("malloc", source.length));
    const output = Number(call("malloc", MAX_SAMPLES * 2));
    if (!input || !output) return null;
    // Both buffers belong to this per-clip instance, released even on a trap.
    decoder.writeMemory(input, source);
    const count = Number(call("silk_decode", input, source.length, output, MAX_SAMPLES));
    if (!Number.isInteger(count) || count <= 0 || count > MAX_SAMPLES) return null;
    return encodeBase64(wav(decoder.readMemory(output, count * 2)));
  } catch {
    return null;
  } finally {
    decoder?.close();
  }
}
