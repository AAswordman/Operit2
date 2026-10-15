import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";

const participants = [{ id: "role-b", name: "Second named role", avatarUri: null }, { id: "role-a", name: "First named role", avatarUri: null }];
const planner = loadModule("src/group-execution/planner.ts");

/** The formal schema preserves rounds and skips, without translating names, fences or alternate containers. */
test("planner: exact rounds preserve explicit speaking order, skipped entries and normal no-response plans", () => {
  const plan = { rounds: [[{ id: "role-a", speak: true }, { id: "role-b", speak: false }], [], [{ id: "role-b", speak: true }, { id: "role-a", speak: true }]] };
  assert.deepEqual(plain(planner.parseGroupResponsePlan(JSON.stringify(plan), participants)), plan);
  assert.deepEqual(plain(planner.parseGroupResponsePlan('{"rounds":[[]]}', participants)), { rounds: [[]] });
  for (const invalid of ["not-json", '{"members":["role-a"]}', '{"rounds":[[{"id":"First named role","speak":true}]]}', '{"rounds":[[{"id":"role-a"}]]}',
    '{"rounds":[[{"id":"role-a","speak":"true"}]]}', '{"rounds":[[{"id":"role-a","speak":true,"rank":0}]]}', '{"rounds":[[{"id":"role-a","speak":true},{"id":"role-a","speak":false}]]}',
    '{"rounds":[]}', JSON.stringify({ rounds: [[], [], [], [], [], []] }), "\x60\x60\x60json\n{\"rounds\":[[]]}\n\x60\x60\x60"]) assert.throws(() => planner.parseGroupResponsePlan(invalid, participants));
});

/** Functional planning uses the real existing SDK method, with the original prompt and no speculative send API. */
test("planner: actual Tools.Chat.call receives ROLE_RESPONSE_PLANNER, saved identities, exact text and thinking disabled", async () => {
  const calls = [], expected = { rounds: [[{ id: "role-b", speak: true }]] }, text = "Original full user request\n第二行";
  const api = loadModule("src/group-execution/planner.ts", { Tools: { Chat: {
    /** Models only the explicitly controlled functional AI boundary and returns its scheduled assistant JSON. */
    async call(input) { calls.push(plain(input)); return { text: JSON.stringify(expected), turns: [], finishReason: "stop", inputTokens: 1, outputTokens: 1 }; },
  } } });
  assert.deepEqual(plain(await api.planGroupResponse(participants, text)), expected);
  assert.equal(calls.length, 1); assert.equal(calls[0].functionType, "ROLE_RESPONSE_PLANNER"); assert.equal(calls[0].enableThinking, false);
  assert.equal(calls[0].recordTokenUsage, true); assert.deepEqual(calls[0].turns, [{ kind: "user", content: planner.buildGroupPlannerPrompt(participants, text) }]);
  const content = calls[0].turns[0].content;
  assert.ok(content.indexOf("id: role-b") < content.indexOf("id: role-a")); assert.ok(content.endsWith(text));
});

/** A failed functional model or malformed response cannot silently plan all members or retry the AI request. */
test("planner: original AI failures and malformed JSON propagate without retries or default turns", async () => {
  const failure = new Error("Controlled AI failure"), calls = [];
  const api = loadModule("src/group-execution/planner.ts", { Tools: { Chat: {
    /** Rejects the actual functional call with the same controlled failure object. */
    async call(input) { calls.push(input); throw failure; },
  } } });
  await assert.rejects(() => api.planGroupResponse(participants, "Actual input"),
    /** Requires original error identity, not a fabricated empty or member-order plan. */
    error => error === failure,
  );
  assert.equal(calls.length, 1);
  const malformed = loadModule("src/group-execution/planner.ts", { Tools: { Chat: {
    /** Returns a deliberately malformed model response without providing any alternative response. */
    async call(input) { calls.push(input); return { text: "invalid JSON" }; },
  } } });
  await assert.rejects(() => malformed.planGroupResponse(participants, "Actual input")); assert.equal(calls.length, 2);
  const unavailable = loadModule("src/group-execution/planner.ts", { Tools: { Chat: {} } });
  await assert.rejects(() => unavailable.planGroupResponse(participants, "Actual input"), /call/);
});
