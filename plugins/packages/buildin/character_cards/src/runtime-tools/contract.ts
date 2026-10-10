import type { DomainInput, DomainOperation, DomainOutput } from "../api";
import type { LinkInfo, QueryResult } from "./memory";

/** Calls an exact typed operation on the single main-runtime domain service. */
export interface DomainCall {
  <K extends DomainOperation>(operation: K, input: DomainInput<K>): Promise<DomainOutput<K>>;
}
/** Preserves the twelve migrated executable tool names. */
export const memoryToolNames = ["get_memory_owner_key", "query_memory", "get_memory_by_title", "create_memory", "update_memory", "delete_memory", "move_memory", "update_user_preferences", "link_memories", "query_memory_links", "update_memory_link", "delete_memory_link"] as const;
export type MemoryToolName = typeof memoryToolNames[number];
export type ToolParameters = Record<string, unknown>;
/** Enumerates the exact runtime fields injected by JsToolManager and JsEngine, excluding tool-specific business parameters. */
export const hostRuntimeParameterNames = [
  "__operit_package_lang", "__operit_package_state", "__operit_package_caller_name",
  "__operit_package_chat_id", "__operit_package_caller_participant_id", "__operit_package_caller_owner", "__operit_package_name",
  "__operit_toolpkg_runtime_kind", "__operit_toolpkg_api_version", "__operit_execution_context_key",
  "__operit_toolpkg_subpackage_id", "containerPackageName", "toolPkgId", "__operit_ui_package_name",
  "__operit_script_screen",
] as const;
/** Carries only the host-injected execution identity understood by this plugin. */
export interface CallerContext { chatId: string | null; participantId: string | null; callerName: string | null }
/** Calls the existing package-owned domain IPC channel without a Core memory executor. */
export const callDomain: DomainCall = <K extends DomainOperation>(operation: K, input: DomainInput<K>): Promise<DomainOutput<K>> => ToolPkg.ipc.call<{ operation: K; input: DomainInput<K> }, DomainOutput<K>>("character-memory.domain", { operation, input }, { targetRuntime: "main" });
/** Validates parameter objects before applying any tool-specific parsing. */
export function parameters(value: unknown): ToolParameters {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("Tool parameters must be an object");
  return value as ToolParameters;
}
/** Reads optional text without coercing a supplied invalid value or losing authored whitespace. */
export function optionalText(input: ToolParameters, name: string): string | undefined {
  if (!Object.prototype.hasOwnProperty.call(input, name)) return undefined;
  const value = input[name];
  if (typeof value !== "string") throw new Error(name + " must be a string");
  return value;
}
/** Requires nonblank text while retaining the complete authored value. */
export function requiredText(input: ToolParameters, name: string): string {
  const value = optionalText(input, name);
  if (value === undefined || value.trim() === "") throw new Error("Missing or empty required parameter: " + name);
  return value;
}
/** Reads a supplied numeric parameter and rejects nonfinite values rather than substituting one. */
export function optionalNumber(input: ToolParameters, name: string): number | undefined {
  if (!Object.prototype.hasOwnProperty.call(input, name)) return undefined;
  const value = input[name];
  if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(name + " must be a finite number");
  return value;
}
/** Applies the declared positive-count semantics without accepting an invalid supplied count. */
export function limit(input: ToolParameters, defaultLimit: number): number {
  const value = optionalNumber(input, "limit");
  if (value === undefined) return defaultLimit;
  if (!Number.isSafeInteger(value) || value < 0) throw new Error("Invalid limit. Expected a nonnegative integer.");
  return Math.max(1, value);
}
/** Enforces the score range on every explicitly supplied score. */
export function score(input: ToolParameters, name: string): number | undefined {
  const value = optionalNumber(input, name);
  if (value !== undefined && (value < 0 || value > 1)) throw new Error(name + " must be between 0 and 1");
  return value;
}
/** Decodes optional host-injected identity fields using their exact reserved names. */
export function callerContext(input: ToolParameters): CallerContext {
  /** Converts one genuinely absent execution field to its explicit optional-context spelling. */
  const field = (name: string): string | null => {
    const value = optionalText(input, name);
    if (value === undefined) return null;
    if (value.trim() === "") throw new Error(name + " must not be blank");
    return value;
  };
  return { chatId: field("__operit_package_chat_id"), participantId: field("__operit_package_caller_participant_id"), callerName: field("__operit_package_caller_name") };
}
/** Formats the original local-time memory timestamp contract without an invalid-date substitute. */
export function formatTime(value: number): string {
  const date = new Date(value);
  if (!Number.isSafeInteger(value) || Number.isNaN(date.getTime())) throw new Error("Invalid memory timestamp");
  /** Pads one local calendar component to its documented width. */
  const pad = (part: number): string => String(part).padStart(2, "0");
  return date.getFullYear() + "-" + pad(date.getMonth() + 1) + "-" + pad(date.getDate()) + " " + pad(date.getHours()) + ":" + pad(date.getMinutes());
}
/** Parses only the two documented local-time boundary spellings and rejects invalid calendar dates. */
export function timeBoundary(input: ToolParameters, name: "start_time" | "end_time"): number | null {
  const value = optionalText(input, name);
  if (value === undefined) return null;
  const match = /^(\d{4})-(\d{2})-(\d{2})(?: (\d{2}):(\d{2}))?$/.exec(value.trim());
  if (match === null) throw new Error("Invalid " + name + ". Expected format YYYY-MM-DD or YYYY-MM-DD HH:mm.");
  const end = name === "end_time", year = Number(match[1]), month = Number(match[2]), day = Number(match[3]);
  const hour = match[4] === undefined ? (end ? 23 : 0) : Number(match[4]);
  const minute = match[5] === undefined ? (end ? 59 : 0) : Number(match[5]);
  const date = new Date(0); date.setFullYear(year, month - 1, day); date.setHours(hour, minute, end ? 59 : 0, end ? 999 : 0);
  if (month < 1 || month > 12 || date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day || date.getHours() !== hour || date.getMinutes() !== minute) throw new Error("Local time is invalid: " + value);
  for (const offset of [-120, -90, -60, -30, 30, 60, 90, 120]) {
    const other = new Date(date.getTime() + offset * 60000);
    if (other.getFullYear() === year && other.getMonth() === month - 1 && other.getDate() === day && other.getHours() === hour && other.getMinutes() === minute) throw new Error("Local time is ambiguous: " + value);
  }
  return date.getTime();
}

/** Maps each registered executable tool to its actual result contract. */
export interface MemoryToolResults {
  get_memory_owner_key: string;
  query_memory: QueryResult;
  get_memory_by_title: string | QueryResult;
  create_memory: string;
  update_memory: string;
  delete_memory: string;
  move_memory: string;
  update_user_preferences: "Successfully updated USER.md";
  link_memories: { sourceTitle: string; targetTitle: string; linkType: string; weight: number; description: string; };
  query_memory_links: { totalCount: number; links: LinkInfo[]; };
  update_memory_link: { totalCount: number; links: LinkInfo[]; };
  delete_memory_link: string;
}
/** Retains the selected tool result type throughout the executable API. */
export type MemoryToolResult<N extends MemoryToolName = MemoryToolName> = MemoryToolResults[N];
