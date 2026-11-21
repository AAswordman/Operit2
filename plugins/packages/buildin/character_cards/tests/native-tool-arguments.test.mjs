import assert from "node:assert/strict";
import test from "node:test";
import { createNativeToolRuntime, nativeToolParameters, toolModuleScript } from "./native-tool-call-runtime.mjs";

/** Runs the real automation export so runtime fields cannot become element selectors. */
test("automation click forwards only the authored selector through the production JS bridge", async () => {
  const calls = [], starts = [];
  const runtime = createNativeToolRuntime({}, (channel, payload) => {
    starts.push({ channel, payload: structuredClone(payload) });
    return null;
  }, (type, name, params) => {
    calls.push({ type, name, params: structuredClone(params) });
    return { actionType: "click_element" };
  });
  const result = await runtime.invoke(toolModuleScript("../automatic_ui_base/src/tools.ts"), "click_element", {
    ...nativeToolParameters("participant-a"), text: "性能",
  }, false);
  assert.equal(result.success, true, result.message);
  assert.deepEqual(calls, [{ type: "default", name: "click_element", params: { text: "性能" } }]);
  assert.deepEqual(starts, [{ channel: "ui_automation.begin", payload: { chatId: "fixture.chat" } }]);
});

/** Preserves complete MCP business data while identity remains available through native getters. */
test("MCP forwarding preserves nested values and separates concurrent native callers", async () => {
  const calls = [];
  const runtime = createNativeToolRuntime({}, undefined, async (type, name, params) => {
    calls.push({ type, name, params: structuredClone(params) });
    await Promise.resolve();
    return params;
  });
  const script = `
    exports.forward = async function(input) {
      var participant = getCallerParticipantId(), caller = getCallerName();
      var result = await toolCall('mcp:echo', input);
      return { input, result, participant, caller, chatId: getChatId() };
    };
  `;
  const business = { text: '性能', nested: { __operit_environment_owner: 'literal user data', values: [null, true, 1, '{"literal":1}'] }, __operit_custom_business_field: 'declared input' };
  const [left, right] = await Promise.all(["left", "right"].map(participant => runtime.invoke(script, "forward", Object.freeze({
    ...nativeToolParameters(participant), ...business,
  }), false)));
  for (const [index, result] of [left, right].entries()) {
    assert.deepEqual(structuredClone(result), { input: business, result: business, participant: index ? "right" : "left", caller: "Fixture Speaker", chatId: "fixture.chat" });
  }
  assert.deepEqual(calls, [0, 1].map(() => ({ type: "default", name: "mcp:echo", params: business })));
});

/** Retains declared lifecycle fields while removing private bootstrap metadata from hook input. */
test("hook payload keeps its declared package identity independently of runtime context", async () => {
  const runtime = createNativeToolRuntime({});
  const params = { ...nativeToolParameters("participant-a"), __operit_toolpkg_runtime_kind: "main", eventPayload: { text: "hook data" } };
  const result = await runtime.invoke("exports.inspect = input => input;", "inspect", params);
  assert.deepEqual(structuredClone(result), { containerPackageName: "com.operit.character_cards", toolPkgId: "com.operit.character_cards", eventPayload: { text: "hook data" } });
});
