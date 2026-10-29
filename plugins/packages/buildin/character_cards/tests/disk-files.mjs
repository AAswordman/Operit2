import { createStorageHost } from "./storage-host.mjs";
import { loadModule } from "./runtime.mjs";
import { mkdtemp, mkdir, readFile, writeFile, rename, stat, rm, readdir, copyFile } from "node:fs/promises";
import path from "node:path";
import vm from "node:vm";
import { readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";

/** Creates real disk IO for the existing Files API; it implements no plugin records or persistence policy. */
export async function createDiskHarness(testContext, { legacyIdentity = null } = {}) {
  const temporaryRoot = path.resolve(tmpdir());
  const directory = path.resolve(await mkdtemp(path.join(temporaryRoot, "character-cards-disk-")));
  const calls = [], faults = new Map();

  /** Resolves one exact IO path and rejects every escape from the explicitly created plugin directory. */
  function confined(value) {
    if (typeof value !== "string" || value.trim() === "") throw new Error("Disk harness requires a nonblank file path");
    const target = path.resolve(value), relative = path.relative(directory, target);
    if (path.isAbsolute(relative) || relative.split(path.sep)[0] === "..") throw new Error("Disk harness path escapes the plugin directory");
    return target;
  }

  /** Records an actual IO attempt and throws a deliberately injected original failure before disk mutation. */
  function attempted(method, args) {
    calls.push({ method, args: [...args] });
    const fault = faults.get(method);
    if (fault !== undefined && --fault.remaining <= 0) {
      faults.delete(method); throw fault.failure;
    }
  }

  /** Produces the existing file-operation result shape after the real filesystem operation succeeded. */
  function operationResult(operation, file) {
    return { operation, path: file, successful: true, details: "Real Node disk IO completed",
      /** Formats the IO result without synthesizing a domain response. */
      toString() { return this.details; },
    };
  }

  const methods = {
    /** Lists native directory entries for migration resource traversal. */
    async list(file) {
      const target = confined(file); attempted("list", [file]);
      const entries = await readdir(target, { withFileTypes: true });
      return { path: file, entries: entries.map(entry => ({ name: entry.name, isDirectory: entry.isDirectory() })) };
    },
    /** Copies exact file bytes through the existing generic Files capability. */
    async copy(source, destination, recursive = false) {
      const from = confined(source), to = confined(destination); attempted("copy", [source, destination, recursive]);
      if (recursive) throw new Error("Disk harness copy requires an explicit file");
      await copyFile(from, to); return operationResult("copy", destination);
    },
    /** Reports native stat results; only the exact not-found outcome denotes a nonexistent file. */
    async exists(file) {
      const target = confined(file); attempted("exists", [file]);
      let entry;
      try { entry = await stat(target); }
      catch (failure) {
        if (failure.code !== "ENOENT") throw failure;
        return { path: file, exists: false, isDirectory: false, size: 0,
          /** Formats the explicit not-found IO result. */
          toString() { return "File does not exist"; },
        };
      }
      return { path: file, exists: true, isDirectory: entry.isDirectory(), size: entry.size,
        /** Formats native existence metadata. */
        toString() { return "File exists"; },
      };
    },
    /** Creates only the explicitly requested directory with the declared recursive flag. */
    async mkdir(file, createParents = false) {
      const target = confined(file); attempted("mkdir", [file, createParents]);
      await mkdir(target, { recursive: createParents }); return operationResult("mkdir", file);
    },
    /** Reads complete UTF-8 file bytes without parsing, repairing, or replacing plugin data. */
    async readBinary(file) {
      const target = confined(file); attempted("readBinary", [file]);
      const content = await readFile(target);
      return { path: file, contentBase64: content.toString("base64"), size: content.length };
    },
    async read(file) {
      const target = confined(file); attempted("read", [file]);
      const bytes = await readFile(target);
      return { path: file, content: bytes.toString("utf8"), size: bytes.length,
        /** Formats the exact file content supplied by native disk IO. */
        toString() { return this.content; },
      };
    },
    /** Writes exact binary bytes through the same confined generic file boundary. */
    async writeBinary(file, contentBase64) {
      const target = confined(file); attempted("writeBinary", [file, contentBase64]);
      await writeFile(target, Buffer.from(contentBase64, "base64"));
      return operationResult("writeBinary", file);
    },
    /** Writes the supplied content exactly once and does not create missing parent directories. */
    async write(file, content, append = false) {
      const target = confined(file); attempted("write", [file, content, append]);
      if (typeof content !== "string") throw new Error("Disk harness requires string file content");
      await writeFile(target, content, { encoding: "utf8", flag: append ? "a" : "w" });
      return operationResult("write", file);
    },
    /** Publishes the actual source file with native rename after validating both absolute paths. */
    async move(source, destination) {
      const from = confined(source), to = confined(destination); attempted("move", [source, destination]);
      await rename(from, to); return operationResult("move", destination);
    },
    /** Removes an explicitly scoped file or nested directory and forbids deleting the harness root through host IO. */
    async deleteFile(file, recursive = false) {
      const target = confined(file);
      if (target === directory) throw new Error("Disk harness root cannot be deleted by plugin IO");
      attempted("deleteFile", [file, recursive]);
      await rm(target, { recursive, force: false }); return operationResult("delete", file);
    },
  };
  const files = new Proxy(methods, {
    /** Rejects undeclared harness operations rather than fabricating a successful host capability. */
    get(target, property) {
      if (!Object.hasOwn(target, property)) throw new Error("Unsupported Files IO in disk harness: " + String(property));
      return target[property];
    },
  });
  const storage = createStorageHost(directory, attempted);
  const tools = new Proxy({ Files: files, Storage: storage }, {
    /** Permits only real disk IO and rejects legacy CLI, memory tools, or invented production services. */
    get(target, property) {
      if (!Object.hasOwn(target, property)) throw new Error("Disk harness permits Files and Storage only: " + String(property));
      return target[property];
    },
  });

  /** Removes the exact created temporary directory after checking that it remains beneath the known temp root. */
  async function cleanup() {
    const resolved = path.resolve(directory), relative = path.relative(temporaryRoot, resolved);
    if (relative === "" || path.isAbsolute(relative) || relative.split(path.sep)[0] === "..") throw new Error("Unsafe disk harness cleanup path");
    storage.close();
    await rm(resolved, { recursive: true, force: false });
  }
  testContext.after(cleanup);
  if (legacyIdentity !== null) {
    const state = loadModule("src/storage/state.ts").createInitialState(123);
    const schema = loadModule("src/storage/records.ts");
    const ownerKey = "character:" + legacyIdentity;
    state.cards[0].id = legacyIdentity;
    state.active = { CharacterCard: { id: legacyIdentity } };
    state.owners[0].ownerKey = ownerKey;
    state.owners[0].userDocumentPath = "owners/" + encodeURIComponent(ownerKey) + "/USER.md";
    const database = await storage.objects.open({ path: directory + "/memory.sqlite" });
    const names = ["cards", "groups", "tags", "stores", "conversationGroups"];
    for (const name of names) writeFileSync(path.join(directory, name + ".preferences.json"), JSON.stringify(Object.fromEntries(schema.records(state).filter(row => row.collection === name).map(row => ["record_" + row.key, row.value]))), "utf8");
    writeFileSync(path.join(directory, "meta.preferences.json"), JSON.stringify({ __operit_preferences_schema_version: 1, state: { version: 1, active: state.active, memoryPreferences: state.memoryPreferences } }), "utf8");
    await database.commit(schema.records(state).filter(row => !names.includes(row.collection)).map(row => ({ op: "put", ...row, expectedVersion: null })));
    await database.close();
    const profile = path.join(directory, state.owners[0].userDocumentPath);
    await mkdir(path.dirname(profile), { recursive: true });
    await writeFile(profile, "");
    calls.length = 0;
  }
  const wasmContext = vm.createContext({ Uint8Array, WebAssembly, console,
    __operitNativeLoadToolPkgWasm(_target, moduleId) {
      attempted("wasm.load", [moduleId]);
      const manifest = JSON.parse(readFileSync(new URL("../manifest.json", import.meta.url), "utf8"));
      const module = manifest.wasm_modules.find(module => module.id === moduleId);
      if (!module) throw new Error("Undeclared WASM module");
      return { bytes: new Uint8Array(readFileSync(new URL("../" + module.path, import.meta.url))), metadata: { exports: module.exports } };
    },
  });
  vm.runInContext(readFileSync(new URL("../../../../../core/crates/plugin/sdk/src/toolpkg/ToolPkgWasmApi.js", import.meta.url), "utf8"), wasmContext);
  const wasm = wasmContext.__operitCreateToolPkgWasmApi(() => "com.operit.character_cards", false);
  return {
    directory, calls, files, storage,
    globals: {
      Tools: tools,
      ToolPkg: {
        wasm,
        /** Returns the test plugin's persistent Space data namespace. */
        getSpaceDataDir() { return directory.replaceAll("\\", "/"); },
        /** Implements only the existing authenticated config-directory IO binding, not provider or storage services. */
        getConfigDir(pluginId) {
          if (pluginId !== undefined && pluginId !== "com.operit.character_cards") throw new Error("Disk harness cannot expose another plugin directory");
          return directory.replaceAll("\\", "/");
        },
      },
    },
    /** Injects one negative IO event on the Nth call of a method; no successful domain operation is implemented by the harness. */
    failNext(method, failure, occurrence = 1) {
      if (!Object.hasOwn(methods, method) && !["storage.changes", "storage.commit", "storage.open", "storage.list", "storage.get", "storage.transaction", "storage.query", "storage.execute", "storage.getMany", "storage.preferences.open", "storage.preferences.edit", "storage.preferences.data"].includes(method)) throw new Error("Cannot fail an unsupported disk method: " + method);
      if (faults.has(method)) throw new Error("A disk failure is already scheduled for " + method);
      if (!Number.isInteger(occurrence) || occurrence < 1) throw new Error("Disk failure occurrence must be a positive integer");
      faults.set(method, { failure, remaining: occurrence });
    },
    /** Reconstructs assertion data independently from real preference files and memory SQLite rows. */
    readState() {
      const schema = loadModule("src/storage/records.ts"), names = ["cards", "groups", "tags", "stores", "conversationGroups"];
      const metadata = storage.inspectPreferences("meta", values => values.state);
      if (metadata === undefined) throw new Error("Test preferences have no metadata");
      const rows = [];
      for (const name of names) storage.inspectPreferences(name, values => {
        for (const [key, value] of Object.entries(values).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)) if (key.startsWith("record_")) rows.push({ collection: name, key: key.slice(7), value, version: null });
      });
      storage.inspect(sql => {
        for (const row of sql.prepare("SELECT collection,key,value,version FROM records ORDER BY collection,key").all())
          if (schema.collections.includes(row.collection)) rows.push({ ...row, value: JSON.parse(row.value) });
      });
      return schema.decodeStateFromRecords(metadata, rows);
    },
    /** Returns deterministic assertion bytes for the current authoritative records. */
    stateBytes() { return Buffer.from(JSON.stringify(this.readState())); },
    /** Writes deliberately supplied test records into their actual independent storage owners. */
    replaceState(state) {
      const schema = loadModule("src/storage/records.ts"), rows = schema.records(state), names = ["cards", "groups", "tags", "stores", "conversationGroups"];
      for (const name of names) writeFileSync(path.join(directory, name + ".preferences.json"), JSON.stringify(Object.fromEntries(rows.filter(row => row.collection === name).map(row => ["record_" + row.key, row.value]))), "utf8");
      writeFileSync(path.join(directory, "meta.preferences.json"), JSON.stringify({ __operit_preferences_schema_version: 1, state: { version: state.version, active: state.active, memoryPreferences: state.memoryPreferences, memoryRecall: state.memoryRecall } }), "utf8");
      storage.inspect(sql => {
        sql.exec("BEGIN");
        try {
          sql.exec("DELETE FROM records");
          for (const row of rows.filter(row => !names.includes(row.collection))) sql.prepare("INSERT INTO records VALUES(?,?,?,?)").run(row.collection, row.key, JSON.stringify(row.value), "test-edit");
          sql.exec("COMMIT");
        } catch (error) { sql.exec("ROLLBACK"); throw error; }
      });
    },
    /** Returns exact native bytes for independent assertions outside the adapted host API. */
    readBytes(file) { return readFile(confined(file)); },
    /** Lists actual on-disk entries without projecting them into domain records. */
    entries() { return readdir(directory); },
    /** Clears only the IO audit log; files, module state, and scheduled failures are unchanged. */
    clearCalls() { calls.length = 0; },
  };
}
