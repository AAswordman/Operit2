import type { ToolPkg as ToolPkgTypes } from "../../../../types/toolpkg";
import type { ChatParticipantProfile } from "./api";
import { parseChatSelection } from "./chat-bindings";
import { assertBoolean, assertNullableString, assertObject, assertString, requireId } from "./validation";

/** Carries the plugin's selection in its authenticated chat namespace, never in plugin files. */
export interface ChatMarker { version: 1; selection: string }
/** Freezes the complete identity, model, voice, prompt and permission profiles used for one send. */
export interface MessageMarker {
  version: 1;
  selection: string;
  promptFunctionType: "CHAT" | "VOICE";
  primaryOwnerKey: string;
  profile: ChatParticipantProfile;
  participants: ChatParticipantProfile[];
}

/** Copies validated JSON without coercing undefined, non-finite values or unsupported objects. */
function markerValue(value: unknown, path: string): ToolPkgTypes.JsonValue {
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error(path + " must be a finite JSON number");
    return value;
  }
  if (Array.isArray(value)) return value.map(
    /** Validates and copies every actual nested JSON value. */
    (item, index) => markerValue(item, path + "[" + index + "]"),
  );
  return markerObject(value, path);
}
/** Requires an actual JSON object and preserves every explicitly present namespace field. */
export function markerObject(value: unknown, path: string): ToolPkgTypes.JsonObject {
  assertObject(value, path);
  const result: ToolPkgTypes.JsonObject = {};
  for (const [key, item] of Object.entries(value)) Object.defineProperty(result, key, { value: markerValue(item, path + "." + key), enumerable: true, configurable: true, writable: true });
  return result;
}
/** Requires the exact supported marker version rather than accepting an old or missing marker. */
function markerVersion(value: Record<string, unknown>, path: string): void {
  if (value.version !== 1) throw new Error("Unsupported " + path + " version");
}
/** Validates the chat marker without consulting active selection or legacy character fields. */
export function decodeChatMarker(value: unknown): ChatMarker {
  const extension = markerObject(value, "chat extension"); markerVersion(extension, "chat marker");
  const selection = requireId(extension.selection, "chat marker selection"); parseChatSelection(selection);
  if (Object.prototype.hasOwnProperty.call(extension, "groupId")) throw new Error("Conversation membership is owned by plugin files, not chat extension.groupId");
  return { version: 1, selection };
}
/** Updates only selection fields while preserving validated unrelated business fields in this namespace. */
export function encodeChatMarker(selection: string, extension: ToolPkgTypes.JsonObject | null): ToolPkgTypes.JsonObject {
  parseChatSelection(selection);
  if (extension === null) return { version: 1, selection };
  decodeChatMarker(extension);
  return { ...markerObject(extension, "chat extension"), version: 1, selection };
}
/** Copies an exact unique string-list policy without interpreting source names or adding permissions. */
function policyNames(value: unknown, path: string): string[] {
  if (!Array.isArray(value)) throw new Error(path + " must be an array");
  const names: string[] = [], seen = new Set<string>();
  for (const item of value) {
    const name = requireId(item, path);
    if (seen.has(name)) throw new Error("Duplicate " + path + ": " + name);
    seen.add(name); names.push(name);
  }
  return names;
}
/** Validates and constructs a complete send profile without reading any current character or catalog. */
function markerProfile(value: unknown): ChatParticipantProfile {
  assertObject(value, "message participant");
  const id = requireId(value.id, "message participant id"), name = requireId(value.name, "message participant name");
  assertNullableString(value.avatarUri, "message participant avatar");
  assertString(value.introPrompt, "message participant introPrompt");
  assertString(value.userPreferencesText, "message participant userPreferencesText");
  assertString(value.openingStatement, "message participant openingStatement");
  assertObject(value.modelBinding, "message model binding");
  const providerId = requireId(value.modelBinding.providerId, "message model provider"), modelId = requireId(value.modelBinding.modelId, "message model id");
  const ttsConfigId = requireId(value.ttsConfigId, "message TTS config");
  assertObject(value.toolAccess, "message tool policy"); assertBoolean(value.toolAccess.enabled, "message tool policy enabled");
  const toolAccess: ChatParticipantProfile["toolAccess"] = {
    enabled: value.toolAccess.enabled,
    allowedBuiltinTools: policyNames(value.toolAccess.allowedBuiltinTools, "message builtin tools"),
    allowedPackages: policyNames(value.toolAccess.allowedPackages, "message packages"),
    allowedSkills: policyNames(value.toolAccess.allowedSkills, "message skills"),
    allowedMcpServers: policyNames(value.toolAccess.allowedMcpServers, "message MCP servers"),
  };
  if (!Array.isArray(value.resources)) throw new Error("Message resources must be an array");
  const resources: ChatParticipantProfile["resources"] = [], keys = new Set<string>();
  for (const resource of value.resources) {
    assertObject(resource, "message resource");
    const key = requireId(resource.key, "message resource key");
    if (keys.has(key)) throw new Error("Duplicate message resource: " + key); keys.add(key);
    assertBoolean(resource.readable, "message resource readable"); assertBoolean(resource.writable, "message resource writable");
    resources.push({ key, readable: resource.readable, writable: resource.writable });
  }
  return { id, name, avatarUri: value.avatarUri, introPrompt: value.introPrompt, userPreferencesText: value.userPreferencesText,
    openingStatement: value.openingStatement, modelBinding: { providerId, modelId }, ttsConfigId, toolAccess, resources };
}
/** Decodes one complete historical send snapshot without replacing missing identity with current state. */
export function decodeMessageMarker(value: unknown): MessageMarker {
  const extension = markerObject(value, "message extension"); markerVersion(extension, "message marker");
  const selection = requireId(extension.selection, "message marker selection"), parsed = parseChatSelection(selection);
  if (extension.promptFunctionType !== "CHAT" && extension.promptFunctionType !== "VOICE") throw new Error("Invalid message marker prompt function");
  if (!Array.isArray(extension.participants) || extension.participants.length === 0) throw new Error("Message marker requires complete participants");
  const participants = extension.participants.map(
    /** Constructs each complete saved participant independently from current domain records. */
    item => markerProfile(item),
  ), ids = new Set<string>();
  for (const participant of participants) {
    if (ids.has(participant.id)) throw new Error("Duplicate message participant: " + participant.id); ids.add(participant.id);
  }
  const profile = markerProfile(extension.profile), matches = participants.filter(
    /** Requires the original execution participant to be one exact complete saved participant. */
    participant => participant.id === profile.id,
  );
  if (matches.length !== 1 || JSON.stringify(matches[0]) !== JSON.stringify(profile)) throw new Error("Message execution profile does not match its saved participant snapshot");
  if (parsed.kind === "card" && (participants.length !== 1 || profile.id !== parsed.id)) throw new Error("Message card selection does not match its saved participant");
  const primaryOwnerKey = requireId(extension.primaryOwnerKey, "message primary memory owner");
  if (!/^(character|shared):[^:\s]+$/u.test(primaryOwnerKey)) throw new Error("Invalid message primary memory owner");
  if (!profile.resources.some(
    /** Requires the primary memory destination to be permitted by this exact saved execution profile. */
    resource => resource.key === primaryOwnerKey && resource.writable,
  )) throw new Error("Message primary memory owner is not writable in its saved profile");
  return { version: 1, selection, promptFunctionType: extension.promptFunctionType, primaryOwnerKey, profile, participants };
}
/** Encodes a validated full send snapshot for host-owned message or variant extension persistence. */
export function encodeMessageMarker(marker: MessageMarker): ToolPkgTypes.JsonObject {
  const validated = decodeMessageMarker(marker);
  return markerObject(validated, "message extension");
}
