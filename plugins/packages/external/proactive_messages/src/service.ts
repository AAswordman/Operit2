import type { Storage } from "../../../../types/storage";
import { defaultRule, hits, MAX_PENDING, nextOccurrence, pending, validateRule, validateState, type Job, type Rule, type State } from "./model";

export interface Snapshot { state: State; foreground: boolean; chats: { id: string; title: string }[] }
let state: State | null = null;
let store: Storage.DataStore | null = null;
let foreground = false;
let queue: Promise<unknown> = Promise.resolve();
let storageFailure: Error | null = null;
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));
const errorText = (error: unknown): string => error instanceof Error ? error.message : String(error);

/** All UI, lifecycle and clock operations share the main runtime owner. */
function serial<T>(action: () => Promise<T>): Promise<T> {
  const result = queue.then(action);
  queue = result.catch(error => { console.error("[proactive messages]", errorText(error)); });
  return result;
}
async function open(): Promise<State> {
  if (storageFailure) throw storageFailure;
  if (state) return state;
  store = await Tools.Storage.dataStore.open({ path: `${ToolPkg.getLocalDataDir()}/proactive-messages.preferences.json` });
  const saved = (await store.data()).get<State>("state");
  state = saved === undefined ? { version: 1, rules: [], jobs: [] } : validateState(saved);
  for (const job of state.jobs) {
    if (job.status === "preparing") { job.status = "failed"; job.error = "内容准备被中断，请重新准备计划"; }
    // Accepted scheduling is recovered from the host-owned receipt after interruption.
    if (job.status === "delivering") job.status = "scheduling";
  }
  await persist();
  return state;
}
async function persist(): Promise<void> {
  if (storageFailure) throw storageFailure;
  if (!store || !state) throw new Error("主动消息存储未打开");
  try { await store.edit(preferences => { preferences.set("state", state); }); }
  catch (error) {
    storageFailure = new Error(`主动消息存储失败，已停止本次运行中的发送：${errorText(error)}`);
    throw storageFailure;
  }
}
function liveState(): State { if (!state) throw new Error("主动消息未初始化"); return state; }
function ruleFor(job: Job): Rule | undefined { return liveState().rules.find(rule => rule.chatId === job.chatId); }
function planned(job: Job): boolean { return pending(job) || job.status === "skipped" || job.status === "failed"; }

async function cancelJob(job: Job): Promise<void> {
  if (["scheduled", "scheduling"].includes(job.status)) {
    try { await Tools.Chat.cancelScheduledMessage(job.id); }
    catch (error) { job.error = `预约撤销失败，稍后重试：${errorText(error)}`; await persist(); throw error; }
  }
  job.status = "cancelled";
  job.error = "";
  await persist();
}
async function cancelChat(chatId: string): Promise<void> {
  for (const job of liveState().jobs) if (job.chatId === chatId && (pending(job) || (planned(job) && job.dueAt > Date.now()))) await cancelJob(job);
}

async function schedule(job: Job): Promise<void> {
  if (!foreground) return;
  job.status = "scheduling";
  await persist();
  try {
    const receipt = await Tools.Chat.scheduleMessage({ id: job.id, chatId: job.chatId, title: job.title || "Operit", text: job.text, timestamp: job.dueAt });
    job.status = receipt.message.status as Job["status"];
    job.error = receipt.message.error || "";
  } catch (error) {
    // The host may have accepted a request before its response was lost. The same ID is retryable.
    job.error = errorText(error);
  }
  await persist();
}
async function reconcile(): Promise<void> {
  const receipts = new Map((await Tools.Chat.listScheduledMessages()).messages.map(message => [message.id, message]));
  for (const job of liveState().jobs) {
    if (!pending(job)) continue;
    if (!ruleFor(job)?.enabled) { await cancelJob(job); continue; }
    const receipt = receipts.get(job.id);
    if (receipt) {
      job.status = receipt.status as Job["status"];
      job.error = receipt.error || "";
      await persist();
    } else if (job.status === "ready" || job.status === "scheduling") {
      if (foreground) await schedule(job);
    }
  }
}

/** Calls the functional chat model without creating a fake user message in the chat. */
async function generate(rule: Rule, dueAt: number): Promise<string> {
  const history = await Tools.Chat.getMessages(rule.chatId, { order: "desc", limit: 20 });
  const previous = liveState().jobs.filter(job => job.chatId === rule.chatId && job.text && pending(job)).slice(-8);
  const result = await Tools.Chat.call({ functionType: "CHAT", recordTokenUsage: true, enableThinking: false, turns: [
    { kind: "system", content: `${rule.prompt}\n你正在提前准备一条主动消息，预计发送时间：${new Date(dueAt).toISOString()}。参考对话中的称呼、语气和语言。不要声称掌握准备时尚未知的实时信息。只输出消息正文。` },
    ...[...history.messages].reverse().map(message => ({ kind: message.sender === "user" ? "user" : "assistant", content: message.content.slice(0, 6000) })),
    { kind: "user", content: "请为上述时间准备一条主动消息。避免重复以下已准备内容：\n" + previous.map(job => job.text).join("\n") },
  ] });
  const text = result.text.trim();
  if (!text || text.length > 12000) throw new Error("AI 返回了空白或过长的主动消息");
  return text;
}

async function fill(): Promise<void> {
  if (!foreground) return;
  const current = liveState();
  for (const rule of current.rules) {
    if (!rule.enabled || !foreground) continue;
    const remaining = current.jobs.filter(job => job.chatId === rule.chatId && planned(job) && job.dueAt > Date.now()).length;
    for (let index = remaining; index < rule.prepareCount && foreground; index++) {
      if (current.jobs.filter(job => pending(job) || (planned(job) && job.dueAt > Date.now())).length >= MAX_PENDING) break;
      const dueAt = nextOccurrence(rule, Math.max(rule.nextAt, Date.now()));
      // Keep native notification scheduling inside its accepted 30-day horizon.
      if (dueAt - Date.now() > 30 * 86400000) break;
      rule.nextAt = dueAt;
      const job: Job = { id: `${rule.chatId}:${dueAt}:${Math.random().toString(36).slice(2, 10)}`, chatId: rule.chatId, title: rule.title,
        dueAt, text: "", status: hits(rule.probability) ? "preparing" : "skipped", error: "" };
      current.jobs.push(job);
      await persist();
      if (job.status === "skipped") continue;
      try {
        job.text = await generate(rule, dueAt);
        job.status = "ready";
        await persist();
        await schedule(job);
      } catch (error) {
        if (storageFailure) throw storageFailure;
        job.status = "failed";
        job.error = errorText(error);
        await persist();
        // Stop this batch after a model failure rather than burning every slot.
        break;
      }
    }
  }
  // Retain all future checks and pending deliveries, plus the last 80 outcomes.
  const keep = current.jobs.filter(job => pending(job) || (planned(job) && job.dueAt > Date.now()));
  const history = current.jobs.filter(job => !keep.includes(job)).slice(-80);
  current.jobs = [...history, ...keep].sort((a, b) => a.dueAt - b.dueAt);
  await persist();
}
async function work(prepare: boolean): Promise<void> {
  await open();
  await reconcile();
  if (prepare && foreground) await fill();
}
export function tick(): Promise<void> { return serial(() => work(foreground)); }
/** Lifecycle flags update immediately even when a model call still holds the owner queue. */
export function setForeground(value: boolean): Promise<void> {
  foreground = value;
  return value ? tick() : Promise.resolve();
}
export function snapshot(includeChats = true): Promise<Snapshot> {
  return serial(async () => {
    await open();
    const chats = includeChats ? (await Tools.Chat.listAll()).chats.map(chat => ({ id: chat.id, title: chat.title })) : [];
    return { state: clone(liveState()), foreground, chats };
  });
}
export function saveRule(input: Rule): Promise<Snapshot> {
  const rule = validateRule(input);
  return serial(async () => {
    const current = await open();
    if (!(await Tools.Chat.listAll()).chats.some(chat => chat.id === rule.chatId)) throw new Error("该对话已不存在");
    // Persist a disabled rule before cancelling host-owned requests. If cancellation fails,
    // every later clock/resume retries it and cannot deliver an old queued message.
    const position = current.rules.findIndex(item => item.chatId === rule.chatId);
    if (position < 0 && current.rules.length >= 8) throw new Error("最多可配置 8 个对话");
    if (position >= 0) current.rules[position] = { ...rule, enabled: false };
    else current.rules.push({ ...rule, enabled: false });
    await persist();
    await cancelChat(rule.chatId);
    const stored = current.rules.find(item => item.chatId === rule.chatId)!;
    stored.enabled = rule.enabled;
    await persist();
    await work(true);
    return { state: clone(current), foreground, chats: [] };
  });
}
export function removeRule(chatId: string): Promise<void> {
  return serial(async () => {
    const current = await open();
    current.rules = current.rules.filter(rule => rule.chatId !== chatId);
    await persist();
    await cancelChat(chatId);
    });
}
export function prepare(): Promise<void> { return tick(); }
export function stop(): void { foreground = false; }
export { defaultRule };
