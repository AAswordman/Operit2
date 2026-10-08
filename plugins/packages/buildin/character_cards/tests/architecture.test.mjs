import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { coreArchitectureViolations, pluginArchitectureViolations } from "./architecture.mjs";

/** Rejects dedicated plugin Host/DTO/bridge/import contracts in actual Core and SDK source. */
test("Core and SDK do not own character-plugin-specific Host DTO bridge or import contracts", () => {
  const violations = coreArchitectureViolations();
  assert.deepEqual(violations, [], `Core architecture cleanup is incomplete:\n${violations.join("\n")}`);
});

/** Keeps missing production globals and withdrawn SDK contracts visible rather than installing test stubs. */
test("plugin production source does not depend on withdrawn business globals DTOs or CLI exec", () => {
  const violations = pluginArchitectureViolations();
  assert.deepEqual(violations, [], `Plugin business connection is incomplete:\n${violations.join("\n")}`);
});

/** Keeps known SDK payloads typed through plugin internals and leaves unknown only at actual raw boundaries. */
test("plugin internal contracts do not erase known SDK and domain result types", () => {
  const hooks = readFileSync(new URL("../src/group-execution/hooks.ts", import.meta.url), "utf8");
  const lifecycle = readFileSync(new URL("../src/chat-lifecycle.ts", import.meta.url), "utf8");
  const service = readFileSync(new URL("../src/service.ts", import.meta.url), "utf8");
  const contracts = readFileSync(new URL("../src/group-execution/contracts.ts", import.meta.url), "utf8");
  assert.match(hooks, /event: ToolPkgTypes\.ChatInputHookEvent/);
  assert.doesNotMatch(hooks, /event\.eventPayload\s+as\s+unknown/);
  assert.match(lifecycle, /event: ToolPkgTypes\.ChatLifecycleHookEvent/);
  assert.match(service, /dispatch<R extends Request>\(request: R\): Promise<RequestOutput<R>>/);
  assert.match(contracts, /type GroupSubmissionAttachment = Chat\.SendAttachment/);
});
