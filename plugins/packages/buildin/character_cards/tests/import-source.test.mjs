import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";

/** Supplies only the platform identity and file reads the import source contract declares. */
function host(platform, files) {
  const reads = [];
  return {
    reads,
    Tools: {
      System: { terminal: { async info() { return { platform }; } } },
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

/** Verifies each documented host path maps through its platform mount and a UTF-8 BOM is removed. */
test("file imports map through the platform mount and strip a UTF-8 BOM", async () => {
  const cases = [
    ["linux", "/home/test/backup.json", "/mnt/linux/home/test/backup.json"],
    ["macos", "/Users/test/backup.json", "/mnt/macos/Users/test/backup.json"],
    ["windows", "C:\\Users\\test\\backup.json", "/mnt/windows/c/Users/test/backup.json"],
    ["android", "/sdcard/backup.json", "/mnt/android/root/sdcard/backup.json"],
  ];
  for (const [platform, argument, mapped] of cases) {
    const disk = host(platform, { [mapped]: '\uFEFF{"version":"2.0"}' });
    const api = loadModule("src/import-source.ts", { Tools: disk.Tools });
    const source = api.parseImportSource(argument);
    assert.deepEqual(plain(source), { kind: "file", path: argument });
    assert.equal(await api.readImportSource(source), '{"version":"2.0"}');
    assert.deepEqual(disk.reads, [mapped]);
  }
});

/** Rejects unsupported and empty file sources without silently importing one as JSON. */
test("unsupported paths, missing files, and empty files remain explicit failures", async () => {
  const api = loadModule("src/import-source.ts");
  assert.throws(() => api.parseImportSource("   "), /JSON content or a file path/);

  const relative = loadModule("src/import-source.ts", { Tools: host("linux", {}).Tools });
  await assert.rejects(relative.readImportSource({ kind: "file", path: "backup.json" }), /unsupported import file path on linux/);

  const missing = loadModule("src/import-source.ts", { Tools: host("linux", {}).Tools });
  await assert.rejects(missing.readImportSource({ kind: "file", path: "/tmp/missing.json" }), /File does not exist/);

  const empty = loadModule("src/import-source.ts", { Tools: host("linux", { "/mnt/linux/tmp/backup.json": " \n" }).Tools });
  await assert.rejects(empty.readImportSource({ kind: "file", path: "/tmp/backup.json" }), /import file is empty/);
});
