import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import test from 'node:test';
import { selectorEnvironment } from '../../plugins/packages/buildin/character_cards/tests/selector-environment.mjs';

const output = new URL('../../target/dsl-performance/', import.meta.url);

/** Summarizes exact IO attempts for one real plugin operation. */
function callsSince(runtime, start) {
  const counts = {};
  for (const call of runtime.disk.calls.slice(start)) {
    counts[call.method] = Object.hasOwn(counts, call.method) ? counts[call.method] + 1 : 1;
  }
  return counts;
}

/** Measures actual file-backed plugin business logic using the existing isolated host-contract harness. */
test('reports repeated readonly repository IO without modifying user data', async context => {
  const results = [];
  for (const cardCount of [1, 10, 50]) {
    await context.test('cards-' + cardCount, async subtest => {
      const runtime = await selectorEnvironment(subtest);
      await runtime.api('character.list', {});
      for (let index = 1; index < cardCount; index += 1) {
        await runtime.api('character.create', { values: { name: 'Diagnostic ' + index, characterSetting: 'x'.repeat(16384) } });
      }
      const samples = [];
      for (let index = 0; index < 12; index += 1) {
        const start = runtime.disk.calls.length;
        const watch = performance.now();
        const cards = await runtime.api('character.list', {});
        const durationMs = performance.now() - watch;
        assert.equal(cards.length, cardCount);
        const calls = callsSince(runtime, start);
        assert.equal(calls.read, cardCount, 'Every readonly list reads every owner USER.md');
        assert.equal(calls.exists, cardCount, 'Every readonly list checks every owner pending file');
        samples.push(durationMs);
      }
      samples.sort(
        /** Orders measured durations numerically. */
        (left, right) => left - right,
      );
      const start = runtime.disk.calls.length;
      const watch = performance.now();
      await Promise.all([runtime.api('character.list', {}), runtime.api('group.list', {})]);
      const pairMs = performance.now() - watch;
      const pairCalls = callsSince(runtime, start);
      assert.equal(pairCalls.read, cardCount * 2, 'Both Promise.all reads individually verify every owner');
      assert.equal(pairCalls.exists, cardCount * 2);
      results.push({ cardCount, characterSettingBytesPerAddedCard: 16384, listMs: { p50: samples[5], p95: samples[11] },
        readonlyIoCalls: { read: cardCount, exists: cardCount }, parallelRequestPairMs: pairMs, parallelRequestPairIoCalls: pairCalls });
    });
  }
  mkdirSync(output, { recursive: true });
  const report = { scope: 'Production plugin service and repository with genuine isolated Node filesystem IO; V8, existing contract host harness, no native Rust IPC, UI or user records', samples: 12, results };
  writeFileSync(new URL('storage-probe.json', output), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
});
