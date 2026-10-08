import type { ChatDisplayParticipant } from "../api";
import { requireId } from "../validation";

/** Preserves the formal multi-round functional planner prompt formerly provided by Core. */
export const GROUP_ROLE_RESPONSE_PLANNER_PROMPT = "你是群聊角色发言规划器。只返回有效的 JSON。\n" +
"任务：规划本轮的发言顺序。你可以规划多轮对话。\n" +
"输出格式：\n" +
'{"rounds":[[{"id":"<成员ID>","speak":true}],[{"id":"<成员ID2>","speak":true}]]}\n' +
"规则：\n" +
"- 每一轮（round）是一个数组，包含该轮应该发言的成员。\n" +
"- 你可以规划多轮对话，让成员之间相互讨论。\n" +
"- 对于简单回应，使用单轮，包含一个或多个成员。\n" +
"- 对于讨论场景，使用多轮（例如：成员A发言，然后成员B回应，然后成员A再回复）。\n" +
"- 你可以省略成员来跳过他们，或设置 speak=false。\n" +
'- 如果没有人应该回应，返回 {"rounds":[[]]}。\n' +
"- 只使用提供的成员 ID。\n" +
"- 最多 5 轮，避免过度来回。";

/** Identifies a formal planner entry without inferring aliases from display names. */
export interface GroupPlanEntry { id: string; speak: boolean }
/** Preserves explicit rounds, including intentional empty rounds and skipped speakers. */
export interface GroupResponsePlan { rounds: GroupPlanEntry[][] }

/** Requires real unique ordered member identities before invoking the functional model. */
function memberIds(participants: ChatDisplayParticipant[]): Set<string> {
  if (participants.length === 0) throw new Error("Group planner requires actual participants");
  const ids = new Set<string>();
  for (const participant of participants) {
    requireId(participant.id, "planner participant id"); requireId(participant.name, "planner participant name");
    if (ids.has(participant.id)) throw new Error("Duplicate planner participant: " + participant.id);
    ids.add(participant.id);
  }
  return ids;
}

/** Builds the original member-list and user-text protocol without adding guessed or default members. */
export function buildGroupPlannerPrompt(participants: ChatDisplayParticipant[], userText: string): string {
  memberIds(participants);
  if (typeof userText !== "string") throw new Error("Planner user text must be a string");
  const lines = participants.map(
    /** Publishes exactly the real ID and display name supplied by the plugin's ordered domain read. */
    participant => "- id: " + participant.id + ", name: " + participant.name,
  );
  return GROUP_ROLE_RESPONSE_PLANNER_PROMPT + "\n成员列表：\n" + lines.join("\n") + "\n\n用户消息：\n" + userText;
}

/** Requires a JSON object with the exact fields of the declared formal planner protocol. */
function object(value: unknown, keys: string[], path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(path + " must be an object");
  const result = value as Record<string, unknown>, actual = Object.keys(result);
  if (actual.length !== keys.length || actual.some(
    /** Rejects alternate containers, names and extra speculative planner fields. */
    key => keys.indexOf(key) < 0,
  )) throw new Error(path + " has unexpected or missing fields");
  return result;
}

/** Parses the single formal plan and rejects malformed output rather than selecting an automatic response order. */
export function parseGroupResponsePlan(raw: string, participants: ChatDisplayParticipant[]): GroupResponsePlan {
  const ids = memberIds(participants), value: unknown = JSON.parse(raw), root = object(value, ["rounds"], "group plan");
  if (!Array.isArray(root.rounds) || root.rounds.length === 0 || root.rounds.length > 5) throw new Error("Group plan requires one to five explicit rounds");
  const rounds: GroupPlanEntry[][] = [];
  for (let roundIndex = 0; roundIndex < root.rounds.length; roundIndex++) {
    const source: unknown = root.rounds[roundIndex];
    if (!Array.isArray(source)) throw new Error("Group plan round must be an array");
    const round: GroupPlanEntry[] = [], seen = new Set<string>();
    for (const sourceEntry of source) {
      const entry = object(sourceEntry, ["id", "speak"], "group plan entry"), id = requireId(entry.id, "planned participant id");
      if (!ids.has(id)) throw new Error("Unknown planned group participant: " + id);
      if (seen.has(id)) throw new Error("Duplicate participant in one group round: " + id);
      if (typeof entry.speak !== "boolean") throw new Error("Group plan speak must be a boolean");
      seen.add(id); round.push({ id, speak: entry.speak });
    }
    rounds.push(round);
  }
  return { rounds };
}

/** Calls the existing functional AI SDK directly and preserves the original model or parse failure without retry. */
export async function planGroupResponse(participants: ChatDisplayParticipant[], userText: string): Promise<GroupResponsePlan> {
  const content = buildGroupPlannerPrompt(participants, userText);
  const result = await Tools.Chat.call({ functionType: "ROLE_RESPONSE_PLANNER", turns: [{ kind: "user", content }], recordTokenUsage: true, enableThinking: false });
  if (typeof result.text !== "string") throw new Error("Group planner AI result has no real assistant text");
  return parseGroupResponsePlan(result.text, participants);
}
