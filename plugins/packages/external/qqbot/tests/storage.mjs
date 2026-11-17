/** Minimal transactional ObjectDatabase host used to exercise the authored inbox. */
export function objectStorage() {
  const entries = new Map(); let version = 0, failCommit = false;
  const id = (collection, key) => JSON.stringify([collection, key]);
  const clone = value => structuredClone(value);
  const db = {
    collection(name) { return {
      async get(key) { const row = entries.get(id(name, key)); return row ? clone(row) : null; },
      async list(after = null, limit = 20) { return [...entries.entries()].map(([key, row]) => ({ pair: JSON.parse(key), row })).filter(x => x.pair[0] === name && (after === null || x.pair[1] > after)).sort((a,b) => a.pair[1] < b.pair[1] ? -1 : a.pair[1] > b.pair[1] ? 1 : 0).slice(0, limit).map(x => ({key:x.pair[1], ...clone(x.row)})); },
      async remove(key, options = {}) { const old = entries.get(id(name,key)); if (!old || (options.expectedVersion !== undefined && old.version !== options.expectedVersion)) throw new Error('version conflict'); entries.delete(id(name,key)); },
    }; },
    async commit(mutations, guards = []) {
      if (failCommit) throw new Error('disk unavailable');
      for (const guard of guards) { const old = entries.get(id(guard.collection,guard.key)); if ((old?.version ?? null) !== guard.expectedVersion) throw new Error('version conflict'); }
      for (const m of mutations) { const old = entries.get(id(m.collection,m.key)); if (m.expectedVersion !== undefined && (old?.version ?? null) !== m.expectedVersion) throw new Error('version conflict'); }
      for (const m of mutations) entries.set(id(m.collection,m.key), {value: clone(m.value), version: String(++version)});
    },
  };
  return { db, open: async () => db, fail(value) { failCommit = value; } };
}
