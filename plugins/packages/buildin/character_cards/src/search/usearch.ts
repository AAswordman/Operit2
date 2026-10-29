/** Memory-owned adapter; no external Plugin SDK client or Node native addon is involved. */
export interface Neighbor { key: bigint; similarity: number }
export class VectorIndex {
  private closed = false;
  private constructor(private readonly wasm: ToolPkg.WasmInstance, private readonly pointer: number, readonly dimensions: number) {}

  static async open(dimensions: number): Promise<VectorIndex> {
    let instance: ToolPkg.WasmInstance;
    instance = await ToolPkg.wasm.open("usearch", {
      env: { emscripten_notify_memory_growth() {} },
      wasi_snapshot_preview1: {
        clock_time_get(_clock: number, _precision: bigint, pointer: number) {
          new DataView(instance.memory().buffer).setBigUint64(pointer, BigInt(Date.now()) * 1000000n, true); return 0;
        },
        fd_close() { return 8; },
      },
    });
    instance.call("_initialize");
    const result = new VectorIndex(instance, 0, dimensions);
    try { return new VectorIndex(instance, result.checked("index_create", dimensions, 128), dimensions); }
    catch (error) { instance.close(); throw error; }
  }
  private call(name: string, ...args: (number | bigint)[]): number {
    if (this.closed) throw new Error("Vector index is closed");
    return Number(this.wasm.call(name, args.map(value => typeof value === "bigint" ? { type: "i64" as const, value: value.toString() } : { type: "i32" as const, value })));
  }
  private checked(name: string, ...args: (number | bigint)[]): number {
    const result = this.call(name, ...args), pointer = this.call("index_error");
    if (pointer) {
      const memory = this.wasm.memory(); let message = "";
      for (let index = pointer; index < memory.length && memory[index] && index < pointer + 4096; index++) message += String.fromCharCode(memory[index]);
      throw new Error("USearch: " + message);
    }
    return result;
  }
  /** Allocations are tracked before the next allocation, so a partial failure cannot leak. */
  private buffers<T>(arrays: readonly ArrayBufferView[], lengths: readonly number[], run: (pointers: number[]) => T): T {
    const allocated: number[] = [];
    try {
      for (const length of [...arrays.map(array => array.byteLength), ...lengths]) {
        const pointer = this.call("malloc", Math.max(8, length));
        if (!pointer) throw new Error("USearch allocation failed");
        allocated.push(pointer);
      }
      arrays.forEach((array, index) => this.wasm.writeMemory(allocated[index], new Uint8Array(array.buffer, array.byteOffset, array.byteLength)));
      return run(allocated);
    } finally { for (const pointer of allocated.reverse()) this.call("free", pointer); }
  }
  add(key: bigint, vector: readonly number[]): void {
    if (vector.length !== this.dimensions || vector.some(component => !Number.isFinite(component))) throw new Error("Invalid vector dimensions/components");
    this.buffers([new BigUint64Array([key]), new Float32Array(vector)], [], ([keys, vectors]) => {
      if (this.checked("index_add_batch", this.pointer, keys, vectors, 1, this.dimensions) !== 1) throw new Error("Incomplete vector insertion");
    });
  }
  remove(key: bigint): void { this.checked("index_remove", this.pointer, key); }
  search(vector: readonly number[], count: number, allowed?: readonly bigint[], excluded: readonly bigint[] = []): Neighbor[] {
    if (vector.length !== this.dimensions) throw new Error("Query vector dimensions changed");
    const sorted = (keys: readonly bigint[]) => new BigUint64Array([...keys].sort((a, b) => a < b ? -1 : a > b ? 1 : 0));
    return this.buffers([new Float32Array(vector), sorted(allowed ?? []), sorted(excluded)], [count * 8, count * 4], ([query, allow, exclude, keys, distances]) => {
      const found = this.checked("index_search_filtered", this.pointer, query, count, keys, distances, allow, allowed?.length ?? 0, exclude, excluded.length, allowed === undefined ? 0 : 1);
      const labels = new BigUint64Array(this.wasm.readMemory(keys, found * 8).buffer);
      const values = new Float32Array(this.wasm.readMemory(distances, found * 4).buffer);
      return Array.from(labels, (key, index) => ({ key, similarity: Math.max(-1, Math.min(1, 1 - values[index] / 2)) }));
    });
  }
  score(vector: readonly number[], keys: readonly bigint[]): number[] {
    if (keys.length === 0) return [];
    return this.buffers([new Float32Array(vector), new BigUint64Array(keys)], [keys.length * 8], ([query, labels, scores]) => {
      if (this.checked("index_score_keys", this.pointer, query, labels, keys.length, this.dimensions, scores) !== keys.length) throw new Error("Missing indexed source vector");
      return Array.from(new Float64Array(this.wasm.readMemory(scores, keys.length * 8).buffer), value => Math.max(-1, Math.min(1, value)));
    });
  }
  close(): void { if (!this.closed) { this.call("index_destroy", this.pointer); this.wasm.close(); this.closed = true; } }
}
