import type { ActivePrompt, Card, Group } from "./model";
import { requireId } from "./validation";

/** Describes the plugin-owned meaning of an opaque chat selection token. */
export type ChatSelection = { kind: "card" | "group" | "model"; id: string };

/** Parses the exact plugin selection format without guessing or selecting a default. */
export function parseChatSelection(selection: string): ChatSelection {
  requireId(selection, "chat selection");
  const parsed = /^(card|group|model):([^\s]+)$/u.exec(selection);
  if (parsed === null) throw new Error("Invalid plugin chat selection: " + selection);
  const kind = parsed[1];
  if (kind === "model" && parsed[2] !== "chat") throw new Error("Invalid model selection: " + selection);
  if (kind !== "card" && kind !== "group" && kind !== "model") throw new Error("Invalid chat selection kind: " + kind);
  return { kind, id: parsed[2] };
}

/** Requires the persisted selection to reference exactly one genuine plugin record. */
export function requireChatSelection(selection: string, cards: Card[], groups: Group[]): ChatSelection {
  const parsed = parseChatSelection(selection);
  if (parsed.kind === "model") return parsed;
  const records = parsed.kind === "card" ? cards : groups;
  let matches = 0;
  for (const record of records) if (record.id === parsed.id) matches += 1;
  if (matches !== 1) throw new Error("Chat selection does not identify exactly one stored record: " + selection);
  if (parsed.kind === "group") for (const group of groups) if (group.id === parsed.id && group.members.length === 0) throw new Error("A bound chat group must have actual participants: " + group.id);
  return parsed;
}

/** Serializes the actual current product selection for an explicit new-chat binding. */
export function selectionForActive(active: ActivePrompt): string {
  if ("Model" in active) return "model:chat";
  if ("CharacterCard" in active) return "card:" + active.CharacterCard.id;
  return "group:" + active.CharacterGroup.id;
}
