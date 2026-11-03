/** The browser storage operations exposed to a Linux guest through virtio-9p. */
export interface TerminalFilesystem {
  call(method: string, args: unknown[]): Promise<unknown>;
}
interface Entry { isDirectory: boolean; size: number; }
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const directoryMode = 0x80000000;

class Reader {
  offset = 0;
  constructor(readonly bytes: Uint8Array) {}
  number(width: number): number {
    if (this.offset + width > this.bytes.length) throw new Error("invalid 9P packet");
    const view = new DataView(this.bytes.buffer, this.bytes.byteOffset + this.offset, width);
    this.offset += width;
    return width === 8 ? Number(view.getBigUint64(0, true))
      : width === 4 ? view.getUint32(0, true) : width === 2 ? view.getUint16(0, true) : view.getUint8(0);
  }
  string(): string {
    const length = this.number(2);
    if (this.offset + length > this.bytes.length) throw new Error("invalid 9P string");
    const value = decoder.decode(this.bytes.subarray(this.offset, this.offset + length));
    this.offset += length;
    return value;
  }
}
class Writer {
  bytes: number[] = [];
  number(value: number, width: number): this {
    let remaining = BigInt(Math.trunc(value));
    for (let index = 0; index < width; index++) {
      this.bytes.push(Number(remaining & 255n)); remaining >>= 8n;
    }
    return this;
  }
  string(value: string): this { const bytes = encoder.encode(value); return this.number(bytes.length, 2).data(bytes); }
  data(bytes: Uint8Array): this { for (const byte of bytes) this.bytes.push(byte); return this; }
  result(): Uint8Array { return Uint8Array.from(this.bytes); }
}

/** Serves the real browser filesystem; file bytes are never staged in guest RAM. */
export function createTerminal9p(filesystem: TerminalFilesystem): (packet: Uint8Array, reply: (packet: Uint8Array) => void) => void {
  const fids = new Map<number, string>();
  const inodes = new Map<string, number>();
  let nextInode = 1;
  let messageSize = 65536;
  let queue = Promise.resolve();
  const call = (method: string, ...args: unknown[]) => filesystem.call(method, args);
  const pathFor = (fid: number): string => {
    const path = fids.get(fid); if (path === undefined) throw new Error("unknown 9P fid"); return path;
  };
  const child = (parent: string, name: string): string => {
    if (name === ".") return parent;
    if (name === "..") return parent.split("/").slice(0, -1).join("/");
    if (!name || name.includes("/") || name.includes("\0")) throw new Error("invalid directory entry");
    return parent ? `${parent}/${name}` : name;
  };
  const stat = async (path: string): Promise<Entry> => await call("stat", path) as Entry;
  function qid(writer: Writer, path: string, entry: Entry): void {
    let inode = inodes.get(path);
    if (inode === undefined) { inode = nextInode++; inodes.set(path, inode); }
    writer.number(entry.isDirectory ? 0x80 : 0, 1).number(0, 4).number(inode, 8);
  }
  function statBytes(path: string, entry: Entry): Uint8Array {
    const writer = new Writer().number(0, 2).number(0, 4);
    qid(writer, path, entry);
    writer.number(entry.isDirectory ? directoryMode + 0o777 : 0o666, 4)
      .number(0, 4).number(0, 4).number(entry.size, 8)
      .string(path.split("/").pop() || "/").string("root").string("root").string("root");
    return new Writer().number(writer.bytes.length, 2).data(writer.result()).result();
  }
  async function handle(packet: Uint8Array): Promise<Uint8Array> {
    const reader = new Reader(packet);
    const size = reader.number(4), type = reader.number(1), tag = reader.number(2);
    const body = new Writer();
    let replyType = type + 1;
    try {
      if (size !== packet.length || size > 65536) throw new Error("invalid 9P packet size");
      switch (type) {
        case 100: { // Tversion
          messageSize = Math.min(reader.number(4), 65536);
          const version = reader.string();
          fids.clear();
          body.number(messageSize, 4).string(version.startsWith("9P2000") ? "9P2000" : "unknown");
          break;
        }
        case 104: { // Tattach
          const fid = reader.number(4); reader.number(4); reader.string(); reader.string();
          fids.set(fid, ""); qid(body, "", await stat("")); break;
        }
        case 108: break; // Tflush; requests are serialized before acknowledgements.
        case 110: { // Twalk
          const fid = reader.number(4), target = reader.number(4), count = reader.number(2);
          let path = pathFor(fid); const walked = new Writer(); let completed = 0;
          for (let index = 0; index < count; index++) {
            const next = child(path, reader.string());
            try { qid(walked, next, await stat(next)); path = next; completed++; }
            catch (error) { if (!completed) throw error; break; }
          }
          if (completed === count) fids.set(target, path);
          body.number(completed, 2).data(walked.result()); break;
        }
        case 112: { // Topen
          const path = pathFor(reader.number(4)), mode = reader.number(1);
          let entry = await stat(path);
          if (mode & 16) { await call("truncate", path, 0); entry = await stat(path); }
          qid(body, path, entry); body.number(messageSize - 24, 4); break;
        }
        case 114: { // Tcreate
          const fid = reader.number(4), path = child(pathFor(fid), reader.string());
          const mode = reader.number(4); reader.number(1);
          await call(mode & directoryMode ? "mkdir" : "create", path);
          fids.set(fid, path); qid(body, path, await stat(path)); body.number(messageSize - 24, 4); break;
        }
        case 116: { // Tread, including packed directory stat records.
          const path = pathFor(reader.number(4)), offset = reader.number(8);
          const count = Math.min(reader.number(4), messageSize - 11);
          const entry = await stat(path);
          let bytes: Uint8Array;
          if (entry.isDirectory) {
            const entries = await call("list", path) as Array<Entry & { name: string }>;
            const directory = new Writer();
            for (const value of entries) directory.data(statBytes(child(path, value.name), value));
            // Directory records must remain whole; Linux advances by their byte lengths.
            const all = directory.result(); let end = offset;
            while (end + 2 <= all.length) {
              const length = all[end] + (all[end + 1] << 8) + 2;
              if (end + length > offset + count) break;
              end += length;
            }
            bytes = all.subarray(offset, end);
          } else bytes = await call("read", path, offset, count) as Uint8Array;
          body.number(bytes.length, 4).data(bytes); break;
        }
        case 118: { // Twrite
          const path = pathFor(reader.number(4)), offset = reader.number(8), count = reader.number(4);
          if (reader.offset + count > packet.length) throw new Error("invalid 9P write length");
          await call("write", path, offset, packet.subarray(reader.offset, reader.offset + count));
          body.number(count, 4); break;
        }
        case 120: fids.delete(reader.number(4)); break; // Tclunk
        case 122: { // Tremove
          const fid = reader.number(4); await call("remove", pathFor(fid)); fids.delete(fid); break;
        }
        case 124: { // Tstat; the outer nstat includes the stat record's own size.
          const path = pathFor(reader.number(4)), bytes = statBytes(path, await stat(path));
          body.number(bytes.length, 2).data(bytes); break;
        }
        case 126: { // Twstat: chmod/timestamps are advisory on browser storage.
          const fid = reader.number(4); reader.number(2); reader.number(2);
          reader.number(2); reader.number(4); reader.number(1); reader.number(4); reader.number(8);
          reader.number(4); reader.number(4); reader.number(4);
          const lengthBytes = packet.subarray(reader.offset, reader.offset + 8);
          const length = reader.number(8), name = reader.string();
          let path = pathFor(fid);
          if (!lengthBytes.every(byte => byte === 255)) await call("truncate", path, length);
          if (name && name !== path.split("/").pop()) {
            const target = child(path.split("/").slice(0, -1).join("/"), name);
            await call("rename", path, target);
            for (const [id, current] of fids) if (current === path || current.startsWith(`${path}/`)) {
              fids.set(id, target + current.slice(path.length));
            }
            for (const [current, inode] of [...inodes]) if (current === path || current.startsWith(`${path}/`)) {
              inodes.delete(current); inodes.set(target + current.slice(path.length), inode);
            }
            path = target;
          }
          break;
        }
        default: throw new Error(`unsupported 9P operation ${type}`);
      }
    } catch (error) {
      replyType = 107;
      body.bytes = [];
      body.string(error instanceof Error ? error.message : String(error));
    }
    return new Writer().number(body.bytes.length + 7, 4).number(replyType, 1).number(tag, 2).data(body.result()).result();
  }
  return (packet, reply) => {
    queue = queue.then(async () => reply(await handle(packet)), async () => reply(await handle(packet)));
  };
}
