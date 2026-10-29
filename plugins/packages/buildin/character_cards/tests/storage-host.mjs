import { DatabaseSync } from "node:sqlite";
import path from "node:path";
import { readFileSync, writeFileSync, existsSync } from "node:fs";

/** Adapts generic object-storage calls to real SQLite transactions without implementing character business logic. */
export function createStorageHost(directory, attempted) {
  const handles = new Set();
  let transactionGate = null;
  let preferenceGate = null;
  /** Opens a real SQLite file and exposes the generic object collection contract used by the repository. */
  async function open(options) {
    const target = path.resolve(options.path), relative = path.relative(directory, target);
    if (path.isAbsolute(relative) || relative.split(path.sep)[0] === "..") throw new Error("Storage path escapes the test directory");
    attempted("storage.open", [options]);
    const sql = new DatabaseSync(target); handles.add(sql);
    sql.exec("CREATE TABLE IF NOT EXISTS records(collection TEXT NOT NULL,key TEXT NOT NULL,value TEXT NOT NULL,version TEXT NOT NULL,PRIMARY KEY(collection,key));CREATE TABLE IF NOT EXISTS revisions(id INTEGER PRIMARY KEY AUTOINCREMENT)");
    sql.exec("CREATE VIEW IF NOT EXISTS plugin_objects AS SELECT collection,key,value,version FROM records");
    sql.exec(`CREATE TABLE IF NOT EXISTS changes(revision INTEGER PRIMARY KEY AUTOINCREMENT,collection TEXT NOT NULL,key TEXT NOT NULL);
      CREATE TRIGGER IF NOT EXISTS record_insert AFTER INSERT ON records BEGIN INSERT INTO changes(collection,key) VALUES(NEW.collection,NEW.key); END;
      CREATE TRIGGER IF NOT EXISTS record_update AFTER UPDATE ON records BEGIN INSERT INTO changes(collection,key) VALUES(NEW.collection,NEW.key); END;
      CREATE TRIGGER IF NOT EXISTS record_delete AFTER DELETE ON records BEGIN INSERT INTO changes(collection,key) VALUES(OLD.collection,OLD.key); END;`);
    /** Reads one existing object and its exact version without treating stored null as absence. */
    function get(collection, key) {
      const row = sql.prepare("SELECT value,version FROM records WHERE collection=? AND key=?").get(collection, key);
      if (row === undefined) return null;
      return { value: JSON.parse(row.value), version: row.version };
    }
    /** Executes one real rollback-capable SQLite transaction and checks every supplied record precondition. */
    async function commit(mutations, guards = []) {
      attempted("storage.commit", [structuredClone(mutations)]);
      if (transactionGate !== null && transactionGate.predicate(mutations)) await transactionGate.promise;
      if (mutations.length < 1 || mutations.length > 1000) throw new Error("Storage transaction limit");
      sql.exec("BEGIN IMMEDIATE");
      try {
        const revision = String(sql.prepare("INSERT INTO revisions DEFAULT VALUES").run().lastInsertRowid);
        for (const guard of guards) { const actual = get(guard.collection, guard.key); if ((actual === null ? null : actual.version) !== guard.expectedVersion) throw new Error("Storage record guard failed"); }
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
      async changes(after, limit = 100) {
        attempted("storage.changes", [after, limit]);
        if (!Number.isInteger(limit) || limit < 1 || limit > 1000) throw new Error("Change page limit");
        sql.exec("BEGIN");
        try {
          const head = sql.prepare("SELECT COALESCE((SELECT revision FROM changes ORDER BY revision DESC LIMIT 1),0) AS revision").get().revision;
          const prefix = target + "#", position = after === null ? head : Number(after.slice(after.lastIndexOf("#") + 1));
          const reset = after !== null && (!after.startsWith(prefix) || position > head || !Number.isSafeInteger(position) || position < 0);
          const rows = reset || after === null ? [] : sql.prepare("SELECT revision,collection,key FROM changes WHERE revision>? ORDER BY revision LIMIT ?").all(position, limit);
          const last = rows.length ? rows.at(-1).revision : reset ? head : position;
          const identities = new Map(rows.map(row => [row.collection + "\0" + row.key, row]));
          const result = { cursor: prefix + last, reset, hasMore: !reset && last < head, changes: [...identities.values()].map(row => ({ collection: row.collection, key: row.key, entry: get(row.collection, row.key) })) };
          sql.exec("COMMIT"); return result;
        } catch (error) { sql.exec("ROLLBACK"); throw error; }
      },
      /** Executes the production read-only object SQL contract in real SQLite. */
      async query(statement, params = []) { attempted("storage.query", [statement, structuredClone(params)]); return sql.prepare(statement).all(...params); },
      /** Exposes one named generic collection with primary-key pagination and atomic writes. */
      collection(name) {
        return {
          /** Reads an exact object entry from SQLite. */
          async get(key) { attempted("storage.get", [name, key]); return get(name, key); },
          /** Reads ordered object identities from one real SQLite snapshot. */
          async getMany(keys) {
            attempted("storage.getMany", [name, [...keys]]);
            if (keys.length > 1000) throw new Error("Storage batch read limit");
            sql.exec("BEGIN");
            try { const entries = keys.map(key => get(name, key)); sql.exec("COMMIT"); return entries; }
            catch (error) { sql.exec("ROLLBACK"); throw error; }
          },
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
  /** Copies JSON values and freezes every nested object at the preference boundary. */
  function immutable(value) {
    if (value !== null && typeof value === "object") {
      for (const child of Object.values(value)) immutable(child);
      Object.freeze(value);
    }
    return value;
  }
  /** Reads a new or existing JSON preference file without hiding parse and IO errors. */
  function readPreferences(target) {
    if (!existsSync(target)) return {};
    return JSON.parse(readFileSync(target, "utf8"));
  }
  /** Opens real JSON persistence with the generic SDK preference callback contract. */
  async function openPreferences(options) {
    const target = path.resolve(options.path), relative = path.relative(directory, target);
    if (path.isAbsolute(relative) || relative.split(path.sep)[0] === "..") throw new Error("Preference path escapes the test directory");
    if (!target.endsWith(".preferences.json")) throw new Error("Preference file suffix required");
    attempted("storage.preferences.open", [options]);
    let closed = false, declaration = null, started = false, queue = Promise.resolve();
    /** Exposes immutable values and callback-scoped explicit mutations. */
    function view(values, mutable, active) {
      const result = {
        /** Reads and clones a typed JSON preference. */
        get(key) { return Object.hasOwn(values, key) ? immutable(structuredClone(values[key])) : undefined; },
        /** Preserves the difference between absence and a stored null. */
        contains(key) { return Object.hasOwn(values, key); },
        /** Enumerates application values without exposing native metadata. */
        entries() { return Object.entries(values).filter(([key]) => !key.startsWith("__operit_")).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0).map(([key, value]) => [key, immutable(structuredClone(value))]); },
      };
      if (mutable) Object.assign(result, {
        /** Records one complete JSON value during the owning callback. */
        set(key, value) { if (!active()) throw new Error("Preference edit completed"); values[key] = structuredClone(value); },
        /** Removes one key during the owning callback. */
        remove(key) { if (!active()) throw new Error("Preference edit completed"); delete values[key]; },
      });
      return result;
    }
    /** Serializes callbacks while retaining each original error for its caller. */
    function enqueue(action) {
      const result = queue.then(action);
      queue = result.then(() => undefined, () => undefined);
      return result;
    }
    /** Runs native-style schema steps and the edit against the same original JSON snapshot. */
    async function perform(callback) {
      if (closed) throw new Error("Preference handle is closed");
      started = true;
      const before = readPreferences(target), values = structuredClone(before);
      let active = true;
      const preferences = view(values, true, () => active);
      try {
        const version = Object.hasOwn(values, "__operit_preferences_schema_version") ? values.__operit_preferences_schema_version : 0;
        if (declaration !== null) {
          if (version > declaration.version) throw new Error("Preference schema version is newer than the declared schema");
          for (let step = version; step < declaration.version; step += 1) await declaration.migrate(step, preferences);
          values.__operit_preferences_schema_version = declaration.version;
        }
        if (callback !== null) await callback(preferences);
      } finally { active = false; }
      if (JSON.stringify(before) !== JSON.stringify(readPreferences(target))) throw new Error("DataStore edit conflict: preferences changed while the callback was running");
      if (callback !== null || JSON.stringify(before) !== JSON.stringify(values)) attempted("storage.preferences.edit", [options.path, structuredClone(values)]);
      if (preferenceGate !== null && preferenceGate.predicate(options.path, values)) await preferenceGate.promise;
      if (JSON.stringify(before) !== JSON.stringify(values)) {
        writeFileSync(target, JSON.stringify(values), "utf8");
      }
      return view(values, false, () => false);
    }
    const handle = {
      path: options.path,
      async changes(after, limit = 100) {
        attempted("storage.changes", [after, limit]);
        if (!Number.isInteger(limit) || limit < 1 || limit > 1000) throw new Error("Change page limit");
        sql.exec("BEGIN");
        try {
          const head = sql.prepare("SELECT COALESCE((SELECT revision FROM changes ORDER BY revision DESC LIMIT 1),0) AS revision").get().revision;
          const prefix = target + "#", position = after === null ? head : Number(after.slice(after.lastIndexOf("#") + 1));
          const reset = after !== null && (!after.startsWith(prefix) || position > head || !Number.isSafeInteger(position) || position < 0);
          const rows = reset || after === null ? [] : sql.prepare("SELECT revision,collection,key FROM changes WHERE revision>? ORDER BY revision LIMIT ?").all(position, limit);
          const last = rows.length ? rows.at(-1).revision : reset ? head : position;
          const identities = new Map(rows.map(row => [row.collection + "\0" + row.key, row]));
          const result = { cursor: prefix + last, reset, hasMore: !reset && last < head, changes: [...identities.values()].map(row => ({ collection: row.collection, key: row.key, entry: get(row.collection, row.key) })) };
          sql.exec("COMMIT"); return result;
        } catch (error) { sql.exec("ROLLBACK"); throw error; }
      },
      /** Declares one schema before this handle starts reading or editing. */
      withSchema(version, migrate) { if (started || declaration !== null) throw new Error("Preference schema must be declared before operations"); declaration = { version, migrate }; return handle; },
      /** Returns one complete immutable snapshot after declared schema upgrades. */
      data() { attempted("storage.preferences.data", [options.path]); return enqueue(() => perform(null)); },
      /** Publishes exactly the mutations made by the owning callback. */
      edit(callback) { return enqueue(async () => { await perform(callback); }); },
      /** Releases only this execution handle. */
      async close() { closed = true; },
    };
    return handle;
  }
  return {
    objects: { open },
    dataStore: { open: openPreferences },
    sqlite: {
      /** Opens a real SQL database without providing any plugin business behavior. */
      async open(options) {
        const target = path.resolve(options.path), relative = path.relative(directory, target);
        if (path.isAbsolute(relative) || relative.split(path.sep)[0] === "..") throw new Error("Storage path escapes the test directory");
        attempted("storage.open", [options]);
        const sql = new DatabaseSync(target); handles.add(sql);
        /** Converts a VM byte view into the SQLite driver's supported binary value. */
        function params(values = []) { return values.map(value => Object.prototype.toString.call(value) === "[object Uint8Array]" ? Buffer.from(value) : value); }
        return {
          path: options.path,
      async changes(after, limit = 100) {
        attempted("storage.changes", [after, limit]);
        if (!Number.isInteger(limit) || limit < 1 || limit > 1000) throw new Error("Change page limit");
        sql.exec("BEGIN");
        try {
          const head = sql.prepare("SELECT COALESCE((SELECT revision FROM changes ORDER BY revision DESC LIMIT 1),0) AS revision").get().revision;
          const prefix = target + "#", position = after === null ? head : Number(after.slice(after.lastIndexOf("#") + 1));
          const reset = after !== null && (!after.startsWith(prefix) || position > head || !Number.isSafeInteger(position) || position < 0);
          const rows = reset || after === null ? [] : sql.prepare("SELECT revision,collection,key FROM changes WHERE revision>? ORDER BY revision LIMIT ?").all(position, limit);
          const last = rows.length ? rows.at(-1).revision : reset ? head : position;
          const identities = new Map(rows.map(row => [row.collection + "\0" + row.key, row]));
          const result = { cursor: prefix + last, reset, hasMore: !reset && last < head, changes: [...identities.values()].map(row => ({ collection: row.collection, key: row.key, entry: get(row.collection, row.key) })) };
          sql.exec("COMMIT"); return result;
        } catch (error) { sql.exec("ROLLBACK"); throw error; }
      },
          /** Declares exactly the supplied scalar columns and primary identity. */
          async defineTable(table) {
            sql.exec('CREATE TABLE IF NOT EXISTS "' + table.name + '" (' + table.columns.map(column => '"' + column.name + '" ' + column.affinity.toUpperCase() + (column.nullable ? "" : " NOT NULL") + (column.name === table.primaryKey ? " PRIMARY KEY" : "")).join(",") + ")");
            return { changed: true };
          },
          /** Queries actual SQLite rows with the supplied parameters. */
          async query(query, values = []) { attempted("storage.query", [query, values]); return sql.prepare(query).all(...params(values)); },
          /** Executes one real SQL statement through the generic storage interface. */
          async execute(query, values = []) { attempted("storage.execute", [query, values]); const result = sql.prepare(query).run(...params(values)); return { affectedRows: result.changes, lastInsertId: String(result.lastInsertRowid) }; },
          /** Rolls back the entire SQL batch when a real SQLite constraint rejects a stale write. */
          async transaction(statements, options = {}) {
            attempted("storage.transaction", [structuredClone(statements)]);
            if (transactionGate !== null && transactionGate.predicate(statements)) await transactionGate.promise;
            if (statements.length < 1 || statements.length > 1000) throw new Error("Storage transaction limit");
            sql.exec("BEGIN IMMEDIATE");
            try {
              if (options.whenAbsent !== undefined) {
                const { collection, key } = options.whenAbsent;
                if (!/^[A-Za-z_][A-Za-z_0-9]*$/.test(collection)) throw new Error("Invalid transaction guard collection");
                if (sql.prepare('SELECT 1 FROM "' + collection + '" WHERE "_key" = ?').get(key) !== undefined) {
                  sql.exec("COMMIT"); return [];
                }
              }
              const receipts = statements.map(statement => { const result = sql.prepare(statement.sql).run(...params(statement.params)); if (statement.expectedAffectedRows !== undefined && result.changes !== statement.expectedAffectedRows) throw new Error("SQLite statement affected-row precondition failed"); return { affectedRows: result.changes, lastInsertId: String(result.lastInsertRowid) }; });
              sql.exec("COMMIT"); return receipts;
            } catch (error) { sql.exec("ROLLBACK"); throw error; }
          },
          /** Closes only this execution handle. */
          async close() { sql.close(); handles.delete(sql); },
        };
      },
    },
    /** Holds a preference publication to exercise explicit concurrent edit conflicts. */
    holdPreferencesEdit(predicate) {
      if (preferenceGate !== null) throw new Error("A preference edit is already held");
      let release;
      const promise = new Promise(resolve => { release = resolve; });
      preferenceGate = { predicate, promise };
      return { release() { if (preferenceGate === null) throw new Error("Preference edit already released"); preferenceGate = null; release(); } };
    },
    /** Holds a real SQL transaction at the host boundary for explicit concurrency tests. */
    holdTransaction(predicate) {
      if (transactionGate !== null) throw new Error("A SQL transaction is already held");
      let release;
      const promise = new Promise(resolve => { release = resolve; });
      transactionGate = { predicate, promise };
      return {
        /** Releases the pending transaction once without manufacturing a receipt. */
        release() {
          if (transactionGate === null) throw new Error("SQL transaction was already released");
          transactionGate = null;
          release();
        },
      };
    },
    /** Closes every actual SQLite handle before the temporary test directory is removed. */
    close() { for (const sql of handles) sql.close(); handles.clear(); },
    /** Reads and deliberately edits a real JSON preference file for independent assertions. */
    inspectPreferences(name, callback) {
      const target = path.join(directory, name + ".preferences.json"), values = readPreferences(target);
      const before = JSON.stringify(values), result = callback(values);
      if (JSON.stringify(values) !== before) writeFileSync(target, JSON.stringify(values), "utf8");
      return result;
    },
    /** Reads raw collection entries for independent test assertions and deliberate corruption cases. */
    inspect(callback) {
      const sql = new DatabaseSync(path.join(directory, "memory.sqlite"));
      try { return callback(sql); } finally { sql.close(); }
    },
  };
}
