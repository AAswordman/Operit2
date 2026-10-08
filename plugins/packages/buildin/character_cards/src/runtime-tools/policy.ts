import type { CharacterRecord } from "../api";
import { CHARACTER_CARDS_NAMESPACE } from "../chat-lifecycle";
import { decodeMessageMarker } from "../chat-markers";
import { record } from "../domain";

/** Identifies the authenticated hook owner and its isolated snapshot for the already-open turn. */
interface ExecutionContext {
  chatId: string | null;
  participantId: string | null;
  extensionOwner: string;
  messageExtension: Record<string, unknown> | null;
}
/** Requires the actual authenticated registration identity even when this owner contributes no snapshot. */
function requirePolicyOwner(registeredOwner: unknown): asserts registeredOwner is typeof CHARACTER_CARDS_NAMESPACE {
  if (registeredOwner !== CHARACTER_CARDS_NAMESPACE) throw new Error("Tool policy requires this plugin's authenticated registered owner");
}
/** Validates the actual host projection without consulting current chat bindings or character records. */
function executionContext(value: unknown, registeredOwner: unknown): ExecutionContext {
  requirePolicyOwner(registeredOwner);
  const context = record(value, "executionContext"), fields = ["chatId", "participantId", "extensionOwner", "messageExtension"];
  const actual = Object.keys(context);
  if (actual.some(
    /** Rejects complete plugin maps and undocumented alternative snapshot fields. */
    key => fields.indexOf(key) === -1,
  )) throw new Error("executionContext has unexpected fields");
  for (const key of fields) {
    if (!Object.prototype.hasOwnProperty.call(context, key)) throw new Error("executionContext." + key + " is required");
  }
  for (const key of ["chatId", "participantId"]) {
    const value = context[key];
    if (value !== null && (typeof value !== "string" || value.trim() === "" || value.trim() !== value)) throw new Error("executionContext." + key + " must be exact nonblank text or null");
  }
  const chatId = context.chatId, participantId = context.participantId;
  if (chatId !== null && typeof chatId !== "string") throw new Error("Invalid execution chatId");
  if (participantId !== null && typeof participantId !== "string") throw new Error("Invalid execution participantId");
  if (typeof context.extensionOwner !== "string" || context.extensionOwner !== registeredOwner) throw new Error("executionContext.extensionOwner does not match the authenticated registered owner");
  if (chatId === null && participantId !== null) throw new Error("A selected execution participant requires a real chat context");
  const messageExtension = context.messageExtension === null ? null : record(context.messageExtension, "executionContext.messageExtension");
  return { chatId, participantId, extensionOwner: context.extensionOwner, messageExtension };
}
/** Reads permissions only from this turn's validated message marker; null is explicit non-participation. */
function snapshotToolPolicy(context: ExecutionContext): CharacterRecord["toolAccessConfig"] | null {
  if (context.messageExtension === null) return null;
  const marker = decodeMessageMarker(context.messageExtension);
  if (context.participantId === null || marker.profile.id !== context.participantId) throw new Error("Execution participant does not match the frozen message snapshot");
  return marker.profile.toolAccess;
}
/** Checks frozen tool rules using declared tool syntax and exact source identities. */
export function toolAllowed(config: CharacterRecord["toolAccessConfig"], name: string, activationSource?: string): boolean {
  if (!config.enabled) return true;
  const sources = new Set([...config.allowedPackages, ...config.allowedSkills, ...config.allowedMcpServers]);
  if (name === "search") return true;
  if (name === "proxy" || name === "package_proxy") return sources.size !== 0 || config.allowedBuiltinTools.length !== 0;
  if (name === "use_package") {
    if (activationSource === undefined) throw new Error("use_package requires its actual package_name");
    return config.allowedBuiltinTools.some(
      /** Matches the explicit builtin activation permission. */
      tool => tool === "use_package",
    ) && sources.has(activationSource);
  }
  const separator = name.indexOf(":");
  if (separator !== -1) {
    if (separator === 0 || separator === name.length - 1 || name.indexOf(":", separator + 1) !== -1) throw new Error("Invalid registered package tool name: " + name);
    return sources.has(name.slice(0, separator));
  }
  return config.allowedBuiltinTools.some(
    /** Matches exact builtin identities without case normalization or substring guesses. */
    tool => tool === name,
  );
}
/** Intercepts actual tool execution using only this owner's already-resolved message snapshot. */
export async function toolCallPolicy(event: ToolPkg.ToolLifecycleHookEvent): Promise<ToolPkg.ToolLifecycleHookObjectResult | void> {
  if (event.eventName !== "tool_call_intercept") return;
  requirePolicyOwner(event.containerPackageName);
  if (event.eventPayload.runtimeContext === null) return;
  const policy = snapshotToolPolicy(executionContext(event.eventPayload.runtimeContext, event.containerPackageName));
  if (policy === null) return;
  let source: string | undefined;
  if (event.eventPayload.toolName === "use_package") {
    const values = record(event.eventPayload.parameters, "tool parameters");
    if (typeof values.package_name !== "string" || values.package_name.trim() === "") throw new Error("use_package requires its actual package_name");
    source = values.package_name;
  }
  if (toolAllowed(policy, event.eventPayload.toolName, source)) return { action: "allow" };
  return { action: "block", reason: "Selected execution participant is not allowed to access tool: " + event.eventPayload.toolName };
}
/** Filters model-visible tools through the same immutable message snapshot used for actual interception. */
export async function toolPromptPolicy(event: ToolPkg.ToolPromptComposeHookEvent): Promise<ToolPkg.PromptHookObjectResult | void> {
  const payload = event.eventPayload;
  if (payload.stage !== "filter_tool_call_tools" && payload.stage !== "build_tool_prompt") return;
  requirePolicyOwner(event.containerPackageName);
  const metadata = record(payload.metadata, "tool-prompt metadata");
  if (metadata.executionContext === null) return;
  const policy = snapshotToolPolicy(executionContext(metadata.executionContext, event.containerPackageName));
  if (policy === null) return;
  if (!Array.isArray(payload.availableTools)) throw new Error("Tool-prompt availableTools must be the actual host catalog");
  return { availableTools: payload.availableTools.filter(
    /** Preserves host descriptors exactly while selecting the visible registered identities. */
    tool => tool.name === "use_package" && tool.activationSource === undefined
      ? !policy.enabled || (policy.allowedBuiltinTools.some(
        /** Keeps the generic activation entry only when at least one explicit source can be activated. */
        name => name === "use_package",
      ) && policy.allowedPackages.length + policy.allowedSkills.length + policy.allowedMcpServers.length > 0)
      : toolAllowed(policy, tool.name, tool.activationSource),
  ) };
}
/** Registers only the existing concrete execution and model-prompt extension points. */
export function registerToolPolicies(): void {
  ToolPkg.registerToolLifecycleHook({ id: "participant-tool-execution", function: toolCallPolicy });
  ToolPkg.registerToolPromptComposeHook({ id: "participant-tool-visibility", function: toolPromptPolicy });
}
