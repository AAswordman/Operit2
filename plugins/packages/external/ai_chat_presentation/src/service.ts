import type { Chat } from "../../../../types/chat";
import type { Storage } from "../../../../types/storage";
import { defaultRule, validateRule, validateSegments, validateState, type PublishRequest, type Rule, type Segment, type State, type Task } from "./model";

export interface Snapshot { state: State; chats: { id: string; title: string }[] }
interface Batch { fingerprint: string; segments: Segment[]; next: number; timestamps: number[] }
interface Session { id: string; task: Task; assistantTimestamp: number | null; lastTimestamp: number; batches: Map<string, Batch>; queue: Promise<unknown> }
let state: State | null = null;
let opening: Promise<State> | null = null;
let store: Storage.DataStore | null = null;
let mutations: Promise<unknown> = Promise.resolve();
let storageFailure: Error | null = null;
const workers = new Map<string, Promise<void>>();
const sessions = new Map<string, Session>();
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));
const errorText = (error: unknown): string => error instanceof Error ? error.message : String(error);
const wait = (ms: number): Promise<void> => new Promise(resolve => setTimeout(resolve, ms));
const identity = (): string => `${Date.now()}-${Math.random().toString(36).slice(2)}-${Math.random().toString(36).slice(2)}`;
function serial<T>(action: () => Promise<T>): Promise<T> {
  const operation = mutations.then(action);
  mutations = operation.catch(() => {});
  return operation;
}
async function persist(): Promise<void> {
  if (storageFailure) throw storageFailure;
  if (!store || !state) throw new Error("分段对话存储尚未打开");
  try { const saved = clone(state); await store.edit(preferences => { preferences.set("state", saved); }); }
  catch (error) { storageFailure = new Error(`分段对话保存失败：${errorText(error)}`); throw storageFailure; }
}
async function open(): Promise<State> {
  if (storageFailure) throw storageFailure;
  if (state) return state;
  if (opening) return opening;
  opening = (async () => {
    store = await Tools.Storage.dataStore.open({ path: `${ToolPkg.getLocalDataDir()}/ai-chat-presentation.preferences.json` });
    const saved = (await store.data()).get<unknown>("state");
    if (saved === undefined) state = { version: 2, rules: [], tasks: [] };
    else state = clone(validateState(saved));
    for (const task of state.tasks) if (task.status === "running" || task.status === "queued") {
      task.status = "interrupted";
      task.error = "应用或插件已重启，请查看原始子对话后重新发送";
    }
    await persist();
    return state;
  })();
  try { return await opening; } finally { opening = null; }
}
function binding(chatId: string): Rule | undefined { return state?.rules.find(rule => rule.chatId === chatId); }
function link(rule: Rule, display: number, original: number): void {
  const existing = rule.links.find(item => item.display === display);
  if (existing) existing.original = original;
  else rule.links.push({ display, original });
  if (rule.historyCopied && rule.links.length > 1024) rule.links.splice(0, rule.links.length - 1024);
}
function committed(receipt: Awaited<ReturnType<typeof Tools.Chat.sendMessage>>, label: string): number {
  if (receipt.outcome.type !== "committed" || receipt.outcome.status !== "completed")
    throw new Error(`${label}未完成：${JSON.stringify(receipt.outcome)}`);
  if (receipt.outcome.userMessageTimestamp === undefined) throw new Error(`${label}缺少用户消息记录`);
  return receipt.outcome.userMessageTimestamp;
}

/** Copies pre-existing visible history once, then native execution owns the original transcript. */
async function prepareOriginal(rule: Rule): Promise<void> {
  if (rule.originalChatId === null) {
    const result = await Tools.Chat.createNew({ setAsCurrentChat: false, sourceChatId: rule.chatId, parentChatId: rule.chatId });
    rule.originalChatId = result.chatId;
    await persist();
    await Tools.Chat.updateTitle(result.chatId, "原始对话");
  }
  const originalChatId = rule.originalChatId;
  if (!(await Tools.Chat.listAll()).chats.some(chat => chat.id === originalChatId)) throw new Error("原始子对话已被删除，请移除此配置后重新添加");
  while (!rule.historyCopied) {
    const page = await Tools.Chat.getMessagesRange(rule.chatId, { order: "asc", start: rule.importCursor, end: rule.importCursor + 99 });
    for (const message of page.messages) {
      let original = message.timestamp;
      if (message.sender === "user") {
        // Recover an accepted import after losing its response or the following storage write.
        const last = (await Tools.Chat.getMessages(originalChatId, { order: "desc", limit: 1 })).messages[0];
        if (last?.sender === "user" && last.content === message.content && !rule.links.some(item => item.original === last.timestamp)) original = last.timestamp;
        else {
          const receipt = await Tools.Chat.sendMessage({ kind: "submit", chatId: originalChatId, runtime: "main",
            input: { text: message.content, attachments: [], replyToMessageTimestamp: null }, turn: { kind: "record_only" }, notifyReply: false });
          original = committed(receipt, "历史用户消息复制");
        }
      } else await Tools.Chat.deliverPrepared({ chatId: originalChatId, text: message.content || "（空消息）", timestamp: original, notify: false });
      link(rule, message.timestamp, original);
      rule.importCursor++;
      await persist();
    }
    if (page.messages.length < 100) { rule.historyCopied = true; await persist(); }
  }
}
export function snapshot(includeChats = true): Promise<Snapshot> {
  return serial(async () => {
    const current = await open();
    const originals = new Set(current.rules.map(rule => rule.originalChatId));
    const chats = includeChats ? (await Tools.Chat.listAll()).chats.filter(chat => !originals.has(chat.id)).map(chat => ({ id: chat.id, title: chat.title })) : [];
    return { state: clone(current), chats };
  });
}
export function saveRule(input: Rule): Promise<Snapshot> {
  validateRule(input);
  return serial(async () => {
    const current = await open();
    if (!(await Tools.Chat.listAll()).chats.some(chat => chat.id === input.chatId)) throw new Error("该对话已不存在");
    if (current.rules.some(rule => rule.originalChatId === input.chatId)) throw new Error("请选择主对话，原始子对话保留正常聊天流程");
    if (workers.has(input.chatId) || (await Tools.Chat.agentStatus(input.chatId)).isProcessing) throw new Error("请等待当前回复完成后再修改配置");
    let rule = binding(input.chatId);
    if (!rule) {
      if (current.rules.length >= 8) throw new Error("最多可配置 8 个对话");
      rule = defaultRule(input.chatId, input.title);
      current.rules.push(rule);
    }
    rule.enabled = false;
    rule.prompt = input.prompt; rule.title = input.title; rule.charDelayMs = input.charDelayMs;
    await persist();
    if (input.enabled) await prepareOriginal(rule);
    rule.enabled = input.enabled;
    await persist();
    return { state: clone(current), chats: [] };
  });
}
export async function cancelChat(chatId: string): Promise<void> {
  await serial(async () => {
    const current = await open();
    for (const task of current.tasks) if (task.chatId === chatId && ["queued", "running"].includes(task.status)) {
      task.status = "interrupted"; task.error = "用户已停止本次回复";
    }
    await persist();
  });
  const original = binding(chatId)?.originalChatId;
  if (original) await Tools.Chat.cancel(original);
}
export async function removeRule(chatId: string): Promise<void> {
  await cancelChat(chatId);
  await Tools.Chat.writeExtension({ kind: "chat", chatId }, { enabled: false });
  await serial(async () => { const current = await open(); current.rules = current.rules.filter(rule => rule.chatId !== chatId); await persist(); });
}
export async function openOriginal(chatId: string): Promise<void> {
  await open();
  const original = binding(chatId)?.originalChatId;
  if (!original) throw new Error("启用后会自动创建原始子对话");
  await Tools.Chat.switchTo(original);
}
function active(session: Session): Rule {
  if (storageFailure) throw storageFailure;
  const rule = binding(session.task.chatId);
  if (!rule?.enabled || rule.originalChatId !== session.task.originalChatId || session.task.status !== "running" || sessions.get(session.id) !== session)
    throw new Error("本次展示会话已结束或配置已关闭");
  return rule;
}

/** Executes only the original child; the visible parent never receives raw model output. */
async function execute(task: Task): Promise<void> {
  let session: Session | null = null;
  try {
    await serial(async () => {
      if (task.status !== "queued") throw new Error("本次输入已停止");
      const rule = binding(task.chatId);
      if (!rule?.enabled || rule.originalChatId !== task.originalChatId) throw new Error("分段对话配置已关闭");
      task.status = "running"; await persist();
    });
    const rule = binding(task.chatId)!;
    const originalReply = task.input.replyToMessageTimestamp === null ? null : rule.links.find(item => item.display === task.input.replyToMessageTimestamp)?.original;
    if (task.input.replyToMessageTimestamp !== null && originalReply === undefined) throw new Error("引用的消息未关联原始记录，请取消引用后重试");
    const receipt = await Tools.Chat.sendMessage({ kind: "submit", chatId: task.chatId, runtime: task.runtime,
      input: task.input, turn: { kind: "record_only" }, notifyReply: false });
    const userTimestamp = committed(receipt, "用户消息保存");
    session = { id: identity(), task, assistantTimestamp: null, lastTimestamp: userTimestamp, batches: new Map(), queue: Promise.resolve() };
    sessions.set(session.id, session);
    const currentSession = session;
    let completed = false;
    for await (const event of Tools.Chat.sendMessageStreaming({ kind: "submit", chatId: task.originalChatId, runtime: task.runtime,
      input: { ...task.input, replyToMessageTimestamp: originalReply ?? null }, turn: { kind: "execute" }, notifyReply: false })) {
      active(currentSession);
      if (event.type === "part") currentSession.assistantTimestamp = event.messageTimestamp;
      else {
        const originalUser = committed(event.result, "原始对话生成");
        await serial(async () => { link(rule, userTimestamp, originalUser); await persist(); });
        completed = true;
      }
    }
    if (!completed) throw new Error("原始对话未返回完成结果");
    await currentSession.queue;
    await serial(async () => { active(currentSession); task.status = "completed"; await persist(); });
  } catch (error) {
    await serial(async () => {
      if (task.status !== "interrupted") { task.status = "failed"; task.error = errorText(error); }
      await persist();
    });
    await Tools.System.toast(`分段对话：${task.error}`);
  } finally { if (session) sessions.delete(session.id); }
}
function startWorker(chatId: string): void {
  if (workers.has(chatId)) return;
  const operation = (async () => {
    // Leave the synchronous input hook before entering native message execution.
    await wait(0);
    for (;;) {
      const task = state?.tasks.find(item => item.chatId === chatId && item.status === "queued");
      if (!task) return;
      await execute(task);
    }
  })();
  workers.set(chatId, operation);
  void operation.catch(error => { console.error("[分段对话]", errorText(error)); }).finally(() => {
    workers.delete(chatId);
    if (!storageFailure && state?.tasks.some(task => task.chatId === chatId && task.status === "queued")) startWorker(chatId);
  });
}
export async function inputHook(event: ToolPkg.ChatInputHookEvent): Promise<ToolPkg.ChatInputHookObjectResult | void> {
  const payload = event.eventPayload;
  if (event.event !== "submit_requested" || !payload.chatId || payload.source === "Sequence" || payload.source === "Tool") return;
  await open();
  const rule = binding(payload.chatId);
  if (!rule?.enabled) return;
  if (payload.isProcessing) return { action: "Block", message: "请等待原有回复完成后再发送" };
  if (!rule.originalChatId || !rule.historyCopied) return { action: "Block", message: "原始子对话尚未准备好" };
  if (typeof payload.text !== "string" || !Array.isArray(payload.attachments) || (payload.runtime !== "main" && payload.runtime !== "floating"))
    throw new Error("分段对话需要完整的原生输入记录");
  const input: Chat.TurnInput = { text: payload.text, attachments: clone(payload.attachments), replyToMessageTimestamp: payload.replyToMessageTimestamp };
  if (!input.text.trim() && !input.attachments.length) return;
  await serial(async () => {
    activeRuleCheck(rule);
    if (state!.tasks.filter(task => task.chatId === rule.chatId && ["queued", "running"].includes(task.status)).length >= 16) throw new Error("待回复输入已达 16 条，请稍后再发送");
    state!.tasks = state!.tasks.filter(task => ["queued", "running"].includes(task.status)).concat(state!.tasks.filter(task => !["queued", "running"].includes(task.status)).slice(-79));
    state!.tasks.push({ id: identity(), chatId: rule.chatId, originalChatId: rule.originalChatId!, runtime: payload.runtime as Chat.Runtime,
      input, notifyReply: payload.notifyReply, status: "queued", createdAt: Date.now(), error: "", displayed: 0 });
    await persist();
  });
  startWorker(rule.chatId);
  return { action: "Consume", clearInput: true, metadata: { autonomousReply: true } };
}
function activeRuleCheck(rule: Rule): void {
  if (!rule.enabled || binding(rule.chatId) !== rule) throw new Error("分段对话配置已变化，请重新发送");
}
export async function composePrompt(event: ToolPkg.SystemPromptComposeHookEvent): Promise<ToolPkg.PromptHookObjectResult | void> {
  if (event.event !== "after_compose_system_prompt") return;
  const session = [...sessions.values()].find(item => item.task.originalChatId === event.eventPayload.chatId);
  if (!session) return;
  const rule = active(session), prompt = event.eventPayload.systemPrompt;
  if (typeof prompt !== "string") throw new Error("分段对话需要已组合的系统提示词");
  return { systemPrompt: `${prompt}\n\n[分段对话展示]\n${rule.prompt}\n你正在原始子对话中运行。这里的普通回复、思考和工具记录保留供用户查看，不会直接出现在主对话。\n要让用户看到消息，必须调用 ai_chat_presentation:show_messages；也可以选择不展示。你可以在一次回复中多次调用，按交流需要决定内容和分段。每段 text 是实际展示的 Markdown，delayMs 是展示前等待的 0～3000 毫秒；省略时使用配置的打字间隔。\n工具参数 session_id 固定为 ${session.id}；每次调用使用不同的 message_id，重试同一次调用时保持 message_id 和消息内容一致。segments_json 为消息数组，例如 [{"text":"我想到一个办法"},{"text":"要不要试试？","delayMs":600}]。\n工具调用成功即已展示，不要为了展示而重复调用。结束时可在原始记录中补充完整说明；需要用户知道的结果和错误也应通过展示工具发送。` };
}

/** Publishes an idempotent batch in the AI's chosen order through the existing prepared-message API. */
export async function publish(request: PublishRequest): Promise<{ displayed: number }> {
  if (!request || typeof request.sessionId !== "string" || typeof request.messageId !== "string" || !request.messageId.trim() || request.messageId.length > 120) throw new Error("展示请求缺少有效会话或消息标识");
  const segments = validateSegments(request.segments), session = sessions.get(request.sessionId);
  if (!session) throw new Error("展示会话不存在或已经结束");
  active(session);
  const fingerprint = JSON.stringify(segments);
  let batch = session.batches.get(request.messageId);
  if (batch && batch.fingerprint !== fingerprint) throw new Error("同一 message_id 不能用于不同内容");
  if (!batch) {
    const rule = active(session);
    const totalDelay = segments.reduce((sum, segment, index) => sum + (segment.delayMs ?? (session.task.displayed === 0 && index === 0 ? 0 : Math.min(3000, Array.from(segment.text).length * rule.charDelayMs))), 0);
    if (totalDelay > 30000) throw new Error("单次展示等待总时长不能超过 30 秒，请拆成多次工具调用");
    if ([...session.batches.values()].reduce((sum, item) => sum + item.segments.length, 0) + segments.length > 128) throw new Error("本次回复最多展示 128 段消息");
    batch = { fingerprint, segments, next: 0, timestamps: [] }; session.batches.set(request.messageId, batch);
  }
  const current = batch;
  const operation = session.queue.catch(() => {}).then(async () => {
    while (current.next < current.segments.length) {
      const rule = active(session), segment = current.segments[current.next];
      if (current.timestamps[current.next] === undefined) {
        const delayMs = segment.delayMs ?? (session.task.displayed === 0 ? 0 : Math.min(3000, Array.from(segment.text).length * rule.charDelayMs));
        await wait(Math.max(delayMs, session.lastTimestamp + 1 - Date.now(), 0));
        active(session);
        current.timestamps[current.next] = Date.now(); session.lastTimestamp = current.timestamps[current.next];
      }
      const timestamp = current.timestamps[current.next];
      const receipt = await Tools.Chat.deliverPrepared({ chatId: rule.chatId, text: segment.text, timestamp, notify: session.task.notifyReply && session.task.displayed === 0 });
      if (receipt.notificationError) console.error("[分段对话通知]", receipt.notificationError);
      current.next++; session.task.displayed++;
      await serial(async () => {
        if (session.assistantTimestamp !== null) link(rule, timestamp, session.assistantTimestamp);
        await persist();
      });
    }
    return { displayed: current.next };
  });
  session.queue = operation;
  return operation;
}
export { defaultRule };
