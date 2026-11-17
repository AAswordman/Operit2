import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const records = new Map(); let nextVersion = 0;
const clone = value => structuredClone(value);
globalThis.ToolPkg = { getSpaceDataDir: () => '/test' };
globalThis.Tools = { Storage: { objects: { open: async () => ({
  collection: () => ({ get: async key => records.has(key) ? clone(records.get(key)) : null }),
  commit: async ([mutation]) => {
    const current = records.get(mutation.key);
    if ((current?.version ?? null) !== mutation.expectedVersion) throw new Error('version conflict');
    records.set(mutation.key, { value: clone(mutation.value), version: String(++nextVersion) });
  },
}) } } };
const { Repository } = require('../dist/storage.js');
test('stale ticket writes cannot overwrite a concurrent update', async () => {
  const a = new Repository(), b = new Repository();
  await a.save({schemaVersion:1,id:'one',notes:[]}, true);
  const first = await a.get('one'), stale = await b.get('one');
  first.notes.push({text:'new'}); await a.save(first);
  stale.notes.push({text:'stale'}); await assert.rejects(b.save(stale), /version conflict/);
  assert.equal((await a.get('one')).notes[0].text, 'new');
});
