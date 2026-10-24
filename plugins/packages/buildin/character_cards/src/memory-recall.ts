import type { MemoryQueryResult } from "./api";
import { dispatch } from "./service-runtime";
import { memoryRecallQuery } from "./public-api";

/** Bounds every injected memory summary; the persisted limit shares the same validated range. */
const MEMORY_RECALL_MARKER = '<memory_context source="character-memory">';
const MODEL_PARTICIPANT_ID = "model:chat";

/** Collapses structured whitespace so one memory stays one readable block. */
function collapse(value: string, maxLength: number): string {
  const normalized = String(value ?? "").replace(/\s+/gu, " ").trim();
  if (normalized === "") return "-";
  return normalized.length <= maxLength ? normalized : normalized.slice(0, maxLength) + "...";
}

/** Removes host markup so the authored user text alone drives retrieval. */
function searchQuery(processedInput: string): string {
  return String(processedInput ?? "")
    .replace(/<attachment\b[\s\S]*?<\/attachment>/gi, " ")
    .replace(/<workspace_attachment\b[\s\S]*?<\/workspace_attachment>/gi, " ")
    .replace(/<reply_to\b[\s\S]*?<\/reply_to>/gi, " ")
    .replace(/<proxy_sender\b[^>]*\/?>/gi, " ")
    .replace(/\[\s*From [^\]]+\]\s*/gi, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/[\r\n\t]+/g, " ")
    .replace(/[|]+/g, " ")
    .trim();
}

/** Resolves the exact character execution participant of this model request. */
function executionParticipant(payload: ToolPkg.PromptFinalizeHookEvent["eventPayload"]): { chatId: string; participantId: string } | null {
  const context = payload.metadata?.executionContext;
  if (context === undefined || context === null) return null;
  const chatId = String(context.chatId ?? "").trim();
  const participantId = String(context.participantId ?? "").trim();
  if (chatId === "" || participantId === "" || participantId === MODEL_PARTICIPANT_ID) return null;
  return { chatId, participantId };
}

/** Formats typed matches into the injected block; an empty result injects nothing. */
function memoryBlock(result: MemoryQueryResult): string | null {
  if (result.memories.length === 0) return null;
  const lines: string[] = [MEMORY_RECALL_MARKER, "【相关记忆】", `命中数量: ${result.memories.length}`];
  result.memories.forEach((memory, index) => {
    lines.push(
      `#${index + 1}`,
      `标题: ${collapse(memory.title, 80)}`,
      `内容: ${collapse(memory.content, 220)}`,
      `时间: ${collapse(memory.createdAt, 40)}`,
    );
    if (memory.tags.length > 0) lines.push(`标签: ${memory.tags.map(tag => collapse(tag, 40)).join(", ")}`);
    if (memory.chunkInfo !== null) lines.push(`分块: ${collapse(memory.chunkInfo, 80)}`);
  });
  lines.push("</memory_context>");
  return lines.join("\n");
}

/** Injects participant-scoped memory matches into the final model request without persisting them. */
export async function onMemoryRecallPromptFinalize(event: ToolPkg.PromptFinalizeHookEvent): Promise<string | null> {
  const stage = String(event.eventPayload.stage ?? event.eventName ?? "");
  if (stage !== "before_send_to_model") return null;
  const participant = executionParticipant(event.eventPayload);
  if (participant === null) return null;
  const processedInput = String(event.eventPayload.processedInput ?? event.eventPayload.rawInput ?? "");
  if (processedInput.includes(MEMORY_RECALL_MARKER)) return null;
  const query = searchQuery(processedInput);
  if (query === "") return null;
  try {
    const settings = await dispatch({ action: "readMemoryRecallSettings" });
    if (!settings.enabled) return null;
    const result = await memoryRecallQuery(participant.participantId, query, settings.limit, settings.allowRepeated ? null : participant.chatId);
    const block = memoryBlock(result);
    if (block === null) return null;
    return `${processedInput.replace(/\s+$/, "")}\n\n${block}`;
  } catch (error) {
    console.error("Memory recall injection failed: " + String(error));
    return null;
  }
}

/** Registers the built-in recall hook without opening storage during package registration. */
export function registerMemoryRecallHook(): void {
  ToolPkg.registerPromptFinalizeHook({ id: "character-memory-recall", function: onMemoryRecallPromptFinalize });
}
