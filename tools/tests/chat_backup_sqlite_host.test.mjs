import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import vm from 'node:vm';
import test from 'node:test';

const source = readFileSync(new URL('../../apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts', import.meta.url), 'utf8');

/** Executes the actual export and connection lifecycle functions without launching a browser. */
function fixture() {
  let foreignKeys = '1';
  let closed = false;
  const image = new TextEncoder().encode('SQLite format 3\0native-pages');
  const writes = new Map();
  const connections = new Map([['source', { path: 'runtime/live.sqlite', db: {
    export() { foreignKeys = '0'; return image; },
    run(sql) { foreignKeys = sql.split(' = ')[1]; },
    close() { closed = true; },
  } }]]);
  const helperStart = source.indexOf('  function exportSqliteDatabase(');
  const helperEnd = source.indexOf('  function sqliteTransaction(', helperStart);
  const methodsStart = source.indexOf('      close(id: string): void {', source.indexOf('    sqlite: registerWorkerHostModule({'));
  const methodsEnd = source.indexOf('      open(path: string): string {', methodsStart);
  assert.ok(helperStart >= 0 && helperEnd > helperStart && methodsEnd > methodsStart);
  const code = `${source.slice(helperStart, helperEnd)}\nconst host = {${source.slice(methodsStart, methodsEnd)}}; host;`;
  const host = vm.runInNewContext(stripTypeScriptTypes(code), {
    runtimePrefix: 'runtime',
    sqliteConnections: connections,
    querySqlite: () => [{ values: [{ kind: 'integer', value: foreignKeys }] }],
    storageHasFile: (_, path) => writes.has(path),
    storageWrite: (_, path, bytes) => writes.set(path, bytes),
  });
  return { host, image, writes, connections, foreignKeys: () => foreignKeys, closed: () => closed };
}

test('physical snapshot export preserves live foreign keys and refuses overwrites', () => {
  const f = fixture();
  f.host.snapshotDatabase('source', 'runtime/backup.sqlite');
  assert.equal(f.writes.get('runtime/backup.sqlite'), f.image);
  assert.equal(f.foreignKeys(), '1');
  assert.equal(f.writes.has('runtime/live.sqlite'), false);
  assert.throws(() => f.host.snapshotDatabase('source', 'runtime/backup.sqlite'), /already exists/);
});

test('closing staged snapshots releases their database memory and registry entry', () => {
  const f = fixture();
  f.host.close('source');
  assert.equal(f.closed(), true);
  assert.equal(f.connections.size, 0);
});
