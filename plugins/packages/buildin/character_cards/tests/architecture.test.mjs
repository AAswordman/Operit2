import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { coreArchitectureViolations, flutterArchitectureViolations, pluginArchitectureViolations } from "./architecture.mjs";

/** Rejects dedicated plugin Host/DTO/bridge/import contracts in actual Core and SDK source. */
test("Core and SDK do not own character-plugin-specific Host DTO bridge or import contracts", () => {
  const violations = coreArchitectureViolations();
  assert.deepEqual(violations, [], `Core architecture cleanup is incomplete:\n${violations.join("\n")}`);
});

/** Scans all Dart production source so unmounted legacy screens cannot survive plugin extraction. */
test("Flutter does not retain character or memory business screens stores or generated proxies", () => {
  const violations = flutterArchitectureViolations();
  assert.deepEqual(violations, [], `Flutter architecture cleanup is incomplete:\n${violations.join("\n")}`);
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

/** CLI command names and descriptions belong to enabled plugin registrations, including independently installed providers. */
test("CLI resolves plugin commands and help from the actual catalog without role command literals", () => {
  const cli = readFileSync(new URL("../../../../../apps/cli/src/cli/mod.rs", import.meta.url), "utf8");
  assert.doesNotMatch(cli, /"(?:character|group|tag|active-prompt|memory)"/);
  assert.match(cli, /read_cli_plugin_commands/);
  assert.match(cli, /pluginCommands\.iter\(\)\.any\(\|entry\| entry\.name == command\)/);
  assert.match(cli, /for command in pluginCommands/);
  assert.match(cli, /command\.name, command\.description/);
});

/** Generic host deletion delegates domain cleanup to the authenticated provider and requires its exact receipt. */
test("Core configuration deletion does not invoke plugin binding business APIs", () => {
  const support = readFileSync(new URL("../../../../../core/crates/runtime/application/src/services/ProviderRuntimeSupportService.rs", import.meta.url), "utf8");
  assert.doesNotMatch(support, /chat\.configuration\.binding/);
  assert.match(support, /"chat\.configuration\.delete"/);
  assert.match(support, /json!\(\{"deleted":true\}\)/);
});
