import { mkdtemp, mkdir, readFile, writeFile, rename, stat, rm, readdir } from "node:fs/promises";
import path from "node:path";
import { tmpdir } from "node:os";

/** Creates real disk IO for the existing Files API; it implements no plugin records or persistence policy. */
export async function createDiskHarness(testContext) {
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
    if (faults.has(method)) {
      const failure = faults.get(method); faults.delete(method); throw failure;
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
  const tools = new Proxy({ Files: files }, {
    /** Permits only real disk IO and rejects legacy CLI, memory tools, or invented production services. */
    get(target, property) {
      if (property !== "Files") throw new Error("Disk harness permits Tools.Files IO only: " + String(property));
      return target.Files;
    },
  });

  /** Removes the exact created temporary directory after checking that it remains beneath the known temp root. */
  async function cleanup() {
    const resolved = path.resolve(directory), relative = path.relative(temporaryRoot, resolved);
    if (relative === "" || path.isAbsolute(relative) || relative.split(path.sep)[0] === "..") throw new Error("Unsafe disk harness cleanup path");
    await rm(resolved, { recursive: true, force: false });
  }
  testContext.after(cleanup);
  return {
    directory, calls, files,
    globals: {
      Tools: tools,
      ToolPkg: {
        /** Implements only the existing authenticated config-directory IO binding, not provider or storage services. */
        getConfigDir(pluginId) {
          if (pluginId !== undefined && pluginId !== "com.operit.character_cards") throw new Error("Disk harness cannot expose another plugin directory");
          return directory.replaceAll("\\", "/");
        },
      },
    },
    /** Injects one negative IO event; no successful domain operation is implemented by the harness. */
    failNext(method, failure) {
      if (!Object.hasOwn(methods, method)) throw new Error("Cannot fail an unsupported disk method: " + method);
      if (faults.has(method)) throw new Error("A disk failure is already scheduled for " + method);
      faults.set(method, failure);
    },
    /** Returns exact native bytes for independent assertions outside the adapted host API. */
    readBytes(file) { return readFile(confined(file)); },
    /** Lists actual on-disk entries without projecting them into domain records. */
    entries() { return readdir(directory); },
    /** Clears only the IO audit log; files, module state, and scheduled failures are unchanged. */
    clearCalls() { calls.length = 0; },
  };
}
