import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile, writeFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { interfaceMembers } from "./runtime.mjs";

/** Checks real file bytes and the existing SDK result shapes without claiming repository or host integration. */
test("disk IO harness writes and reads real temporary plugin files with current SDK result fields", async t => {
  const disk = await createDiskHarness(t), file = path.join(disk.directory, "records.json");
  const absent = await disk.files.exists(file);
  assert.equal(absent.exists, false);
  assert.deepEqual(Object.keys(absent).sort(), interfaceMembers("../../../../types/results.d.ts", "FileExistsData").sort());
  const content = '{"text":"真实磁盘内容","id":"9223372036854775807"}';
  const written = await disk.files.write(file, content);
  assert.equal(written.successful, true);
  assert.deepEqual(Object.keys(written).sort(), interfaceMembers("../../../../types/results.d.ts", "FileOperationData").sort());
  assert.equal(await readFile(file, "utf8"), content);
  const read = await disk.files.read(file);
  assert.deepEqual(Object.keys(read).sort(), interfaceMembers("../../../../types/results.d.ts", "FileContentData").sort());
  assert.equal(read.content, content); assert.equal(read.size, Buffer.byteLength(content));
  const persisted = await disk.readBytes(file);
  assert.equal(persisted.toString("utf8"), content);
});

/** Requires injected and native IO failures to leave real bytes untouched and remain explicit. */
test("disk IO harness exposes original failures and never repairs bad JSON or creates missing parents", async t => {
  const disk = await createDiskHarness(t), file = path.join(disk.directory, "records.json");
  await writeFile(file, "{broken", "utf8");
  const failure = new Error("TEST_DISK_WRITE_REJECTED"); disk.failNext("write", failure);
  await assert.rejects(disk.files.write(file, "{}"),
    /** Requires the exact deliberate IO failure, not an empty successful result. */
    error => error === failure,
  );
  assert.equal(await readFile(file, "utf8"), "{broken");
  assert.equal((await disk.files.read(file)).content, "{broken");
  await assert.rejects(disk.files.write(path.join(disk.directory, "uncreated", "record.json"), "{}"), { code: "ENOENT" });
  await assert.rejects(disk.files.read(path.join(disk.directory, "missing.json")), { code: "ENOENT" });
});

/** Verifies rename and confinement with actual files without implementing domain commit policy. */
test("disk IO harness moves real bytes and rejects escapes and non-IO production dependencies", async t => {
  const disk = await createDiskHarness(t), source = path.join(disk.directory, "staged.json"), destination = path.join(disk.directory, "records.json");
  await disk.files.write(source, "published"); await disk.files.move(source, destination);
  assert.equal((await disk.files.exists(source)).exists, false);
  assert.equal(await readFile(destination, "utf8"), "published");
  await assert.rejects(disk.files.write(path.join(disk.directory, "..", "escaped.json"), "forbidden"), /escapes the plugin directory/);
  await assert.rejects(disk.files.move(destination, path.join(disk.directory, "..", "escaped.json")), /escapes the plugin directory/);
  assert.throws(
    /** Attempts the prohibited old business transport directly. */
    () => disk.globals.Tools.Memory,
    /Tools.Files IO only/,
  );
  assert.throws(
    /** Requires unsupported IO capabilities to fail visibly rather than be silently synthesized. */
    () => disk.files.unsupported,
    /Unsupported Files IO/,
  );
  assert.equal(await readFile(destination, "utf8"), "published");
});
