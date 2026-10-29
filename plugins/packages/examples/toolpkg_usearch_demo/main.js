/* A real USearch WASM exercise through the public ToolPkg instance/memory API. */
async function createIndex(dimensions, capacity) {
    let wasm;
    const imports = {
        env: { emscripten_notify_memory_growth() {} },
        wasi_snapshot_preview1: {
            clock_time_get(_clock, _precision, pointer) {
                new DataView(wasm.memory().buffer).setBigUint64(pointer, BigInt(Date.now()) * 1000000n, true);
                return 0;
            },
            fd_close() { return 8; },
        },
    };
    wasm = await ToolPkg.wasm.open('usearch', imports);
    function call(name, ...args) {
        return wasm.call(name, args.map(value => ({ type: typeof value === 'bigint' ? 'i64' : 'i32', value: typeof value === 'bigint' ? value.toString() : value })));
    }
    function checked(name, ...args) {
        const result = call(name, ...args), pointer = call('index_error');
        if (pointer) {
            const memory = wasm.memory(); let message = '';
            for (let i = pointer; i < memory.length && memory[i] && i < pointer + 4096; i++) message += String.fromCharCode(memory[i]);
            throw new Error('USearch: ' + message);
        }
        return result;
    }
    function alloc(length) {
        const pointer = call('malloc', length);
        if (!pointer) throw new Error('USearch allocation failed');
        return pointer;
    }
    call('_initialize');
    const index = checked('index_create', dimensions, capacity);
    if (!index) { wasm.close(); throw new Error('USearch index creation failed'); }
    let closed = false;
    function active() { if (closed) throw new Error('USearch index is closed'); }
    return {
        add(keys, vectors) {
            active();
            if (!(keys instanceof BigUint64Array) || !(vectors instanceof Float32Array) || vectors.length !== keys.length * dimensions) throw new Error('Invalid vector batch');
            const keysPointer = alloc(keys.byteLength), vectorsPointer = alloc(vectors.byteLength);
            try {
                wasm.writeMemory(keysPointer, new Uint8Array(keys.buffer, keys.byteOffset, keys.byteLength));
                wasm.writeMemory(vectorsPointer, new Uint8Array(vectors.buffer, vectors.byteOffset, vectors.byteLength));
                if (checked('index_add_batch', index, keysPointer, vectorsPointer, keys.length, dimensions) !== keys.length) throw new Error('Incomplete vector batch');
            } finally { call('free', vectorsPointer); call('free', keysPointer); }
        },
        search(vector, count) {
            active();
            if (!(vector instanceof Float32Array) || vector.length !== dimensions || !Number.isInteger(count) || count < 1) throw new Error('Invalid vector query');
            const queryPointer = alloc(vector.byteLength), keysPointer = alloc(count * 8), distancesPointer = alloc(count * 4);
            try {
                wasm.writeMemory(queryPointer, new Uint8Array(vector.buffer, vector.byteOffset, vector.byteLength));
                const found = checked('index_search', index, queryPointer, count, keysPointer, distancesPointer);
                return {
                    keys: Array.from(new BigUint64Array(wasm.readMemory(keysPointer, found * 8).buffer), String),
                    distances: Array.from(new Float32Array(wasm.readMemory(distancesPointer, found * 4).buffer)),
                };
            } finally { call('free', distancesPointer); call('free', keysPointer); call('free', queryPointer); }
        },
        get size() { active(); return checked('index_size', index); },
        remove(key) { active(); return checked('index_remove', index, BigInt(key)); },
        save() {
            active();
            const length = checked('index_save_size', index), pointer = alloc(length);
            try {
                checked('index_save', index, pointer, length);
                return wasm.readMemory(pointer, length);
            } finally { call('free', pointer); }
        },
        load(bytes) {
            active();
            const pointer = alloc(bytes.byteLength);
            try { wasm.writeMemory(pointer, bytes); checked('index_load', index, pointer, bytes.byteLength); }
            finally { call('free', pointer); }
        },
        close() { if (!closed) { call('index_destroy', index); wasm.close(); closed = true; } },
    };
}
exports.createIndex = createIndex;

/** Fixed seed and exact L2 ranking make performance and recall independently checkable. */
exports.run = async function(params = {}) {
    const count = params.count === undefined ? 2000 : params.count;
    const dimensions = params.dimensions === undefined ? 128 : params.dimensions;
    const queries = params.queries === undefined ? 20 : params.queries;
    if (!Number.isInteger(count) || count < 20 || count > 50000 || !Number.isInteger(dimensions) || dimensions < 2 || dimensions > 2048 || !Number.isInteger(queries) || queries < 1 || queries > 100) throw new Error('Demo dimensions/count/queries are out of range');
    const vectors = new Float32Array(count * dimensions), keys = new BigUint64Array(count);
    const firstKey = 9007199254740993n;
    let seed = 42;
    function random() { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296; }
    for (let i = 0; i < vectors.length; i++) vectors[i] = random() - 0.5;
    for (let i = 0; i < count; i++) keys[i] = firstKey + BigInt(i);
    const index = await createIndex(dimensions, count + 1);
    let restored;
    try {
        const buildStart = Date.now();
        for (let start = 0; start < count; start += 256) {
            const end = Math.min(start + 256, count);
            index.add(keys.subarray(start, end), vectors.subarray(start * dimensions, end * dimensions));
        }
        const buildMs = Date.now() - buildStart;
        const queryVectors = Array.from({ length: queries }, (_, q) => {
            const vector = new Float32Array(dimensions);
            for (let d = 0; d < dimensions; d++) vector[d] = random() - 0.5;
            return vector;
        });
        const exactStart = Date.now();
        const exact = queryVectors.map(query => {
            const best = [];
            for (let i = 0; i < count; i++) {
                let distance = 0;
                for (let d = 0; d < dimensions; d++) { const delta = query[d] - vectors[i * dimensions + d]; distance += delta * delta; }
                if (best.length < 10 || distance < best[best.length - 1].distance) {
                    best.push({ i, distance }); best.sort((a, b) => a.distance - b.distance);
                    if (best.length > 10) best.pop();
                }
            }
            return new Set(best.map(item => String(keys[item.i])));
        });
        const exactMs = Date.now() - exactStart;
        for (const query of queryVectors) index.search(query, 10); // Warm the complete public API path.
        const searchStart = Date.now();
        const hits = queryVectors.map(query => index.search(query, 10));
        const searchMs = Date.now() - searchStart;
        const recallAt10 = hits.reduce((sum, result, q) => sum + result.keys.filter(key => exact[q].has(key)).length, 0) / (queries * 10);
        const sourceQuery = vectors.slice(0, dimensions), original = index.search(sourceQuery, 10);
        if (original.keys[0] !== String(firstKey)) throw new Error('Source vector identity was lost');
        const snapshot = index.save();
        restored = await createIndex(dimensions, count + 1); restored.load(snapshot);
        const recovered = restored.search(sourceQuery, 10);
        if (JSON.stringify(original) !== JSON.stringify(recovered)) throw new Error('Restored index results differ');
        if (restored.remove(firstKey) !== 1 || restored.size !== count - 1 || restored.search(sourceQuery, 10).keys.includes(String(firstKey))) throw new Error('Deletion did not update the index');
        return { count, dimensions, queries, buildMs, searchMs, averageSearchMs: searchMs / queries, exactJsScanMs: exactMs, recallAt10, snapshotBytes: snapshot.byteLength, restored: true, deletionVerified: true, firstKey: original.keys[0] };
    } finally { index.close(); if (restored) restored.close(); }
};
