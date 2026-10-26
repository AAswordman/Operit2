import assert from "node:assert/strict";
import test from "node:test";
import { fixture } from "./fixtures.mjs";
import { loadModule } from "./runtime.mjs";

/** Renders all memory libraries while exposing invalid primary selections for explicit repair. */
test("memory library page isolates missing primary bindings and keeps valid libraries accessible", () => {
  const { memoryPageBody } = loadModule("web/features/memory/library-view.ts");
  for (const sharedId of [null, "", "  ", "deleted-store"]) {
    const snapshot = fixture().snapshot;
    Object.assign(snapshot.cards[0], { memoryBindingMode: "SHARED", sharedMemoryId: sharedId });
    const before = structuredClone(snapshot);
    const html = memoryPageBody(snapshot, new Map());
    const [damaged, healthy] = html.match(/<article\b[\s\S]*?<\/article>/gu);
    assert.match(damaged, /需要重新绑定记忆库/u);
    assert.match(damaged, /data-action="edit-card" data-id="operit"/u);
    assert.doesNotMatch(damaged, /data-action="owner-(graph|user|settings)"|data-owner=/u);
    assert.match(healthy, /data-owner="shared:shared-main"/u);
    assert.match(healthy, /data-action="owner-graph"/u);
    assert.deepEqual(snapshot, before);
    snapshot.cards[0].sharedMemoryId = snapshot.stores[0].id;
    const repaired = memoryPageBody(snapshot, new Map()).match(/<article\b[\s\S]*?<\/article>/gu)[0];
    assert.match(repaired, /data-owner="shared:shared-main"/u);
    assert.doesNotMatch(repaired, /需要重新绑定记忆库/u);
  }
});
