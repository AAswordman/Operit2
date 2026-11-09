import type { MemoryQueryResult } from "./api";
import { dispatch } from "./service-runtime";
import { memoryRecallQuery } from "./public-api";

/** Uses the attachment shape recognized as supplementary context by the Dart UI. */
const MEMORY_RECALL_MARKER = '<attachment id="character_memory_recall"';
const LEGACY_MEMORY_RECALL_MARKER = '<memory_context source="character-memory">';
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
function executionParticipant(payload: ToolPkg.PromptInputHookEvent["eventPayload"]): { chatId: string; participantId: string } | null {
  const context = payload.metadata?.executionContext;
  if (context === undefined || context === null) return null;
  const chatId = String(context.chatId ?? "").trim();
  const participantId = String(context.participantId ?? "").trim();
  if (chatId === "" || participantId === "" || participantId === MODEL_PARTICIPANT_ID) return null;
  return { chatId, participantId };
}

/** Keeps recalled text inside the attachment body, including literal host markup. */
function escapeXml(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;").replace(/'/g, "&apos;");
}

/** Formats typed matches into the injected block; an empty result injects nothing. */
function memoryBlock(result: MemoryQueryResult): string | null {
  if (result.memories.length === 0) return null;
  const lines: string[] = ["【相关记忆】", `命中数量: ${result.memories.length}`];
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
  const content = lines.join("\n");
  return `${MEMORY_RECALL_MARKER} filename="相关记忆" type="text/plain" size="${content.length}">${escapeXml(content)}</attachment>`;
}

/** Adds participant-scoped memories before the host commits the user message to history. */
export async function onMemoryRecallPromptInput(event: ToolPkg.PromptInputHookEvent): Promise<string | null> {
  const stage = String(event.eventPayload.stage ?? event.eventName ?? "");
  if (stage !== "before_process") return null;
  const participant = executionParticipant(event.eventPayload);
  if (participant === null) return null;
  const processedInput = String(event.eventPayload.processedInput ?? event.eventPayload.rawInput ?? "");
  if (processedInput.includes(MEMORY_RECALL_MARKER) || processedInput.includes(LEGACY_MEMORY_RECALL_MARKER)) return null;
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
  ToolPkg.registerPromptInputHook({ id: "character-memory-recall", function: onMemoryRecallPromptInput });
}
