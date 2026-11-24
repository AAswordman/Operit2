import type { Chat } from "../../../../types/chat";

export interface Rule {
  chatId: string; title: string; enabled: boolean; originalChatId: string | null;
  prompt: string; charDelayMs: number; importCursor: number; historyCopied: boolean;
  links: { display: number; original: number }[];
}
export interface Task {
  id: string; chatId: string; originalChatId: string; runtime: Chat.Runtime;
  input: Chat.TurnInput; notifyReply: boolean;
  status: "queued" | "running" | "completed" | "failed" | "interrupted";
  createdAt: number; error: string; displayed: number;
}
export interface State { version: 2; rules: Rule[]; tasks: Task[] }
export interface Segment { text: string; delayMs?: number }
export interface PublishRequest { sessionId: string; messageId: string; segments: Segment[] }
export const DEFAULT_PROMPT = "根据当前交流，自主决定向用户展示什么内容，以及拆成哪些消息。每段表达一个自然的意思，保留用户需要的答案、说明和结果。可以先回应，再使用工具，再补充结果。无需把所有过程都展示给用户。";
export function defaultRule(chatId: string, title: string): Rule {
  return { chatId, title, enabled: false, originalChatId: null, prompt: DEFAULT_PROMPT, charDelayMs: 35, importCursor: 0, historyCopied: false, links: [] };
}
export function validateRule(value: Rule): Rule {
  if (!value || typeof value.chatId !== "string" || !value.chatId.trim() || typeof value.title !== "string" ||
      typeof value.enabled !== "boolean" || typeof value.prompt !== "string" || !value.prompt.trim() || value.prompt.length > 8000 ||
      !Number.isInteger(value.charDelayMs) || value.charDelayMs < 0 || value.charDelayMs > 150)
    throw new Error("请选择对话、填写展示提示词，并将每字延迟设为 0～150 毫秒");
  return value;
}
export function validateSegments(value: unknown): Segment[] {
  if (!Array.isArray(value) || !value.length || value.length > 32) throw new Error("每次须提供 1～32 段消息");
  let length = 0;
  for (const segment of value) {
    if (!segment || typeof segment.text !== "string" || !segment.text.trim() || segment.text.length > 12000 ||
        (segment.delayMs !== undefined && (!Number.isInteger(segment.delayMs) || segment.delayMs < 0 || segment.delayMs > 3000)))
      throw new Error("消息正文须为 1～12000 字，段间延迟须为 0～3000 毫秒");
    length += segment.text.length;
  }
  if (length > 48000) throw new Error("每次展示的正文总长度不能超过 48000 字");
  return value.map(item => ({ text: item.text, ...(item.delayMs === undefined ? {} : { delayMs: item.delayMs }) }));
}
export function validateState(value: unknown): State {
  const state = value as State;
  if (!state || state.version !== 2 || !Array.isArray(state.rules) || !Array.isArray(state.tasks)) throw new Error("分段对话存储格式错误");
  const ids = new Set<string>();
  for (const rule of state.rules) {
    validateRule(rule);
    if (ids.has(rule.chatId) || (rule.originalChatId !== null && (typeof rule.originalChatId !== "string" || !rule.originalChatId || rule.originalChatId === rule.chatId)) ||
        !Number.isSafeInteger(rule.importCursor) || rule.importCursor < 0 || typeof rule.historyCopied !== "boolean" ||
        !Array.isArray(rule.links) || rule.links.some(link => !Number.isSafeInteger(link.display) || !Number.isSafeInteger(link.original)))
      throw new Error("分段对话配置损坏");
    ids.add(rule.chatId);
  }
  if (state.rules.some(rule => rule.originalChatId !== null && ids.has(rule.originalChatId))) throw new Error("原始子对话不能同时作为展示对话");
  const taskIds = new Set<string>();
  for (const task of state.tasks) {
    if (!task || typeof task.id !== "string" || taskIds.has(task.id) || typeof task.chatId !== "string" || typeof task.originalChatId !== "string" ||
        !["queued", "running", "completed", "failed", "interrupted"].includes(task.status) || !task.input || typeof task.input.text !== "string" ||
        !Array.isArray(task.input.attachments) || (task.runtime !== "main" && task.runtime !== "floating") ||
        !Number.isSafeInteger(task.createdAt) || !Number.isInteger(task.displayed) || task.displayed < 0 || typeof task.error !== "string")
      throw new Error("分段对话任务记录损坏");
    taskIds.add(task.id);
  }
  return state;
}
