export interface Rule {
  chatId: string; title: string; enabled: boolean;
  intervalMinutes: number; randomMinutes: number; probability: number;
  prepareCount: number; prompt: string; nextAt: number;
}
export type JobStatus = "preparing" | "ready" | "scheduling" | "scheduled" | "delivering" | "delivered" | "skipped" | "failed" | "cancelled" | "invalidated";
export interface Job {
  id: string; chatId: string; title: string; dueAt: number; text: string;
  status: JobStatus; error: string;
  instruction?: string;
  notificationAttempted?: boolean;
  notificationRegistered?: boolean;
  notificationError?: string | null;
}
export interface State { version: 1; generationVersion?: 2; rules: Rule[]; jobs: Job[] }
export const MAX_PENDING = 32;
export const DEFAULT_PROMPT = "根据最近的对话，自然地向用户发起一次简短交流，可以关心近况、延续话题或分享一个想法。不要假装用户刚刚发送了消息，不要解释定时机制，不要编造当前发生的事情。只输出要发送的消息正文。";
export function defaultRule(chatId: string, title: string): Rule {
  return { chatId, title, enabled: false, intervalMinutes: 60, randomMinutes: 0,
    probability: 50, prepareCount: 4, prompt: DEFAULT_PROMPT, nextAt: 0 };
}
export function validateRule(rule: Rule): Rule {
  if (!rule || typeof rule.chatId !== "string" || !rule.chatId.trim() || typeof rule.title !== "string" ||
      typeof rule.enabled !== "boolean" || typeof rule.prompt !== "string" || !rule.prompt.trim() || rule.prompt.length > 8000)
    throw new Error("请选择对话并填写主动消息提示词（最多 8000 字）");
  for (const [name, value, min, max] of [
    ["固定间隔", rule.intervalMinutes, 1, 10080], ["随机追加间隔", rule.randomMinutes, 0, 10080],
    ["触发概率", rule.probability, 0, 100], ["预备检查次数", rule.prepareCount, 1, 8],
  ] as const) {
    if (!Number.isInteger(value) || value < min || value > max) throw new Error(`${name}须为 ${min}～${max} 的整数`);
  }
  return { ...rule, nextAt: 0 };
}
/** Each occurrence uses a fresh independent delay, followed by its own probability draw. */
export function nextOccurrence(rule: Rule, after: number, random: () => number = Math.random): number {
  return after + rule.intervalMinutes * 60000 + Math.floor(random() * (rule.randomMinutes * 60000 + 1));
}
export function hits(probability: number, random: () => number = Math.random): boolean {
  return probability === 100 || (probability > 0 && random() * 100 < probability);
}
export function pending(job: Job): boolean {
  return ["preparing", "ready", "scheduling", "scheduled", "delivering"].includes(job.status);
}
/** Fail closed on corrupt persistent state rather than creating fresh schedules. */
export function validateState(value: unknown): State {
  const state = value as State;
  if (!state || state.version !== 1 || !Array.isArray(state.rules) || !Array.isArray(state.jobs)) throw new Error("主动消息存储格式错误");
  const chats = new Set<string>(), ids = new Set<string>();
  for (const rule of state.rules) {
    validateRule(rule);
    if (chats.has(rule.chatId) || !Number.isSafeInteger(rule.nextAt) || rule.nextAt < 0) throw new Error("主动消息规则损坏");
    chats.add(rule.chatId);
  }
  if (state.generationVersion !== undefined && state.generationVersion !== 2) throw new Error("主动消息生成版本错误");
  const statuses: JobStatus[] = ["preparing", "ready", "scheduling", "scheduled", "delivering", "delivered", "skipped", "failed", "cancelled", "invalidated"];
  for (const job of state.jobs) {
    if (!job || typeof job.id !== "string" || !job.id || ids.has(job.id) || typeof job.chatId !== "string" || !job.chatId ||
        typeof job.title !== "string" || typeof job.text !== "string" || typeof job.error !== "string" ||
        !statuses.includes(job.status) || !Number.isSafeInteger(job.dueAt) || job.dueAt <= 0 ||
        (job.instruction !== undefined && (typeof job.instruction !== "string" || !job.instruction.trim()))) throw new Error("主动消息发送计划损坏");
    ids.add(job.id);
  }
  if (state.jobs.filter(pending).length > MAX_PENDING) throw new Error("待发送消息数量超过上限");
  return state;
}
