import { DatabaseSync } from "node:sqlite";
import path from "node:path";

/** Adapts generic object-storage calls to real SQLite transactions without implementing character business logic. */
export function createStorageHost(directory, attempted) {
  const handles = new Set();
  /** Opens a real SQLite file and exposes the generic object collection contract used by the repository. */
  async function open(options) {
    const target = path.resolve(options.path), relative = path.relative(directory, target);
    if (path.isAbsolute(relative) || relative.split(path.sep)[0] === "..") throw new Error("Storage path escapes the test directory");
    attempted("storage.open", [options]);
    const sql = new DatabaseSync(target); handles.add(sql);
    sql.exec("CREATE TABLE IF NOT EXISTS records(collection TEXT NOT NULL,key TEXT NOT NULL,value TEXT NOT NULL,version TEXT NOT NULL,PRIMARY KEY(collection,key));CREATE TABLE IF NOT EXISTS revisions(id INTEGER PRIMARY KEY AUTOINCREMENT)");
    /** Reads one existing object and its exact version without treating stored null as absence. */
    function get(collection, key) {
      const row = sql.prepare("SELECT value,version FROM records WHERE collection=? AND key=?").get(collection, key);
      if (row === undefined) return null;
      return { value: JSON.parse(row.value), version: row.version };
    }
    /** Executes one real rollback-capable SQLite transaction and checks every supplied record precondition. */
    async function commit(mutations) {
      attempted("storage.commit", [structuredClone(mutations)]);
      if (mutations.length < 1 || mutations.length > 1000) throw new Error("Storage transaction limit");
      sql.exec("BEGIN IMMEDIATE");
      try {
        const revision = String(sql.prepare("INSERT INTO revisions DEFAULT VALUES").run().lastInsertRowid);
        const identities = new Set();
        for (const mutation of mutations) {
          if (typeof mutation.key !== "string" || Buffer.byteLength(mutation.key) > 1024 || mutation.key === "" || /[\x00-\x1f\x7f]/.test(mutation.key)) throw new Error("Invalid storage mutation key");
          const identity = mutation.collection + "\0" + mutation.key;
          if (identities.has(identity)) throw new Error("Duplicate storage mutation"); identities.add(identity);
          const entry = get(mutation.collection, mutation.key);
          if (Object.hasOwn(mutation, "expectedVersion") && (entry === null ? null : entry.version) !== mutation.expectedVersion) throw new Error("Storage version conflict");
          if (mutation.op === "delete") sql.prepare("DELETE FROM records WHERE collection=? AND key=?").run(mutation.collection, mutation.key);
          else if (mutation.op === "put") sql.prepare("INSERT INTO records VALUES(?,?,?,?) ON CONFLICT(collection,key) DO UPDATE SET value=excluded.value,version=excluded.version").run(mutation.collection, mutation.key, JSON.stringify(mutation.value), revision);
          else throw new Error("Unknown storage mutation");
        }
        sql.exec("COMMIT"); return { version: revision };
      } catch (error) { sql.exec("ROLLBACK"); throw error; }
    }
    return {
      path: options.path,
      /** Exposes one named generic collection with primary-key pagination and atomic writes. */
      collection(name) {
        return {
          /** Reads an exact object entry from SQLite. */
          async get(key) { attempted("storage.get", [name, key]); return get(name, key); },
          /** Pages actual rows by their declared primary keys. */
          async list(after = null, limit = 100) {
            attempted("storage.list", [name, after, limit]);
            const rows = after === null ? sql.prepare("SELECT * FROM records WHERE collection=? ORDER BY key LIMIT ?").all(name, limit) : sql.prepare("SELECT * FROM records WHERE collection=? AND key>? ORDER BY key LIMIT ?").all(name, after, limit);
            return rows.map(row => ({ key: row.key, value: JSON.parse(row.value), version: row.version }));
          },
          /** Commits a single structured object through the same real transaction. */
          async put(key, value, options = {}) { return commit([{ op: "put", collection: name, key, value, ...options }]); },
          /** Removes a single object through the same real transaction. */
          async remove(key, options = {}) { return commit([{ op: "delete", collection: name, key, ...options }]); },
        };
      },
      commit,
      /** Releases the actual SQLite handle without removing its file. */
      async close() { sql.close(); handles.delete(sql); },
    };
  }
  return {
    objects: { open },
    /** Closes every actual SQLite handle before the temporary test directory is removed. */
    close() { for (const sql of handles) sql.close(); handles.clear(); },
    /** Reads raw collection entries for independent test assertions and deliberate corruption cases. */
    inspect(callback) {
      const sql = new DatabaseSync(path.join(directory, "characters.sqlite"));
      try { return callback(sql); } finally { sql.close(); }
    },
  };
}
