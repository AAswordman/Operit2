import assert from "node:assert/strict";
import test from "node:test";
import {
  runtimeManagerViolations, domainTraitViolations, backgroundMemoryViolations,
  chatOwnerResolverViolations, builtinMemoryToolViolations, providerChatMemoryViolations,
} from "./mounted-paths.mjs";

/** Requires mounted production dependencies to disappear rather than accepting unexercised generic replacements. */
function requireNoMountedBusiness(violations) {
  assert.deepEqual(violations, [], `Whole-feature extraction remains incomplete:\n${violations.join("\n")}`);
}

/** Verifies runtime-support adapters no longer invoke original domain managers. */
test("mounted provider and tool runtime supports do not consume old character or memory managers", () => {
  requireNoMountedBusiness(runtimeManagerViolations());
});

/** Verifies exported runtime-support traits no longer expose character/memory-specific contracts. */
test("mounted provider and tool runtime traits do not expose domain members or DTOs", () => {
  requireNoMountedBusiness(domainTraitViolations());
});

/** Verifies background extraction/settings are not still owned by the original Core memory lifecycle. */
test("application and chat do not invoke the old memory management background lifecycle", () => {
  requireNoMountedBusiness(backgroundMemoryViolations());
});

/** Verifies chat send and coordination no longer call the original character-to-memory ownership policy. */
test("mounted chat consumers do not invoke the old character and memory owner resolver", () => {
  requireNoMountedBusiness(chatOwnerResolverViolations());
});

/** Verifies builtin public/internal tools no longer execute the original memory repository implementation. */
test("builtin tool registration does not mount or install old memory business executors", () => {
  requireNoMountedBusiness(builtinMemoryToolViolations());
});

/** Verifies the active provider AI path, background library, and scheduler no longer own plugin business. */
test("mounted AI send paths and background memory libraries do not retain old domain persistence", () => {
  requireNoMountedBusiness(providerChatMemoryViolations());
});
