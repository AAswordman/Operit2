/** Creates one UUID v4 for a newly created domain record. */
export function uuid(): string {
  const bytes: number[] = [];
  for (let index = 0; index < 16; index += 1) bytes.push(Math.floor(Math.random() * 256));
  bytes[6] = (bytes[6] & 15) | 64; bytes[8] = (bytes[8] & 63) | 128;
  const text = bytes.map(
    /** Encodes every byte as two hexadecimal digits. */
    byte => byte.toString(16).padStart(2, "0"),
  ).join("");
  return text.slice(0, 8) + "-" + text.slice(8, 12) + "-" + text.slice(12, 16) + "-" + text.slice(16, 20) + "-" + text.slice(20);
}
