import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";

/** Supplies only the shared file API required by file imports. */
function host(files) {
  const reads = [];
  return {
    reads,
    Tools: {
      Files: {
        /** Reads only declared fixture paths and reports absent files explicitly. */
        async read(path) {
          reads.push(path);
          if (!Object.prototype.hasOwnProperty.call(files, path)) throw new Error("File does not exist: " + path);
          return { path, content: files[path], size: files[path].length };
        },
      },
    },
  };
}

/** Verifies pasted JSON remains inline and never touches the host file service. */
test("pasted JSON is classified inline and returned unchanged", async () => {
  const api = loadModule("src/import-source.ts");
  const source = api.parseImportSource('  {"version":"2.0"}  ');
  assert.deepEqual(plain(source), { kind: "inline", content: '{"version":"2.0"}' });
  assert.equal(await api.readImportSource(source), '{"version":"2.0"}');
});

/** Keeps malformed JSON classified as JSON so its syntax error is reported directly instead of as a path. */
test("malformed JSON that starts like a document remains inline", () => {
  const api = loadModule("src/import-source.ts");
  assert.deepEqual(plain(api.parseImportSource('{"version":')), { kind: "inline", content: '{"version":' });
  assert.deepEqual(plain(api.parseImportSource("[1,")), { kind: "inline", content: "[1," });
});

/** Imports VFS addresses without requiring platform information or rewriting the address. */
test("file imports use the shared file API and strip a UTF-8 BOM", async () => {
  const paths = [
    "/app/data/backup.json",
    "/operitapp/data/backup.json",
    "/mnt/shared/backup.json",
    "/virtual/shared/backup.json",
  ];
  for (const path of paths) {
    const disk = host({ [path]: '\uFEFF{"version":"2.0"}' });
    const api = loadModule("src/import-source.ts", { Tools: disk.Tools });
    const source = api.parseImportSource(path);
    assert.deepEqual(plain(source), { kind: "file", path });
    assert.equal(await api.readImportSource(source), '{"version":"2.0"}');
    assert.deepEqual(disk.reads, [path]);
  }
});

/** Leaves address validation to the file API and preserves its error. */
test("file service errors propagate without platform probing or path rewriting", async () => {
  const failure = new Error("Invalid VFS address");
  for (const path of ["backup.json", "/tmp/missing.json", "C:\\Users\\test\\backup.json", "content://documents/backup.json"]) {
    const reads = [];
    const api = loadModule("src/import-source.ts", { Tools: { Files: { async read(path) { reads.push(path); throw failure; } } } });
    await assert.rejects(api.readImportSource(api.parseImportSource(path)), error => error === failure);
    assert.deepEqual(reads, [path]);
  }
});

/** Rejects empty import arguments and blank file contents explicitly. */
test("empty arguments and empty files remain explicit failures", async () => {
  const api = loadModule("src/import-source.ts");
  assert.throws(() => api.parseImportSource("   "), /JSON content or a file path/);

  const path = "/app/data/backup.json";
  const disk = host({ [path]: " \n" });
  const empty = loadModule("src/import-source.ts", { Tools: disk.Tools });
  await assert.rejects(empty.readImportSource({ kind: "file", path }), /import file is empty/);
  assert.deepEqual(disk.reads, [path]);
});
