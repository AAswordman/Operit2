import type { FeedRequest, OpenRequest, OpenResult, Segment, Settings } from "./api.js";
import { modeForChat } from "./mode.js";
import { SentenceSession } from "./sentences.js";
import { identifier, readSettings, writeSettings } from "./settings.js";

interface MessageState { processor: SentenceSession; snapshot: string; finalized: boolean }
interface Session { owner: string; chatId: string; settings: Settings; messages: Map<number, MessageState> }
const sessions = new Map<string, Session>();
let nextSession = 0;

/** Requires the authenticated dependency owner of a live session. */
function owned(id: string, caller: string): Session {
  const session = sessions.get(identifier(id, "sessionId"));
  if (session === undefined) throw new Error("Waifu session does not exist");
  if (session.owner !== caller) throw new Error("Waifu session belongs to another package");
  return session;
}

/** Publishes explicitly selected profile settings. */
export function settingsGet(event: ToolPkg.PublicApiEvent<{ profile: string }>): Promise<Settings> { return readSettings(event.payload.profile); }

/** Applies a validated profile update through the dependency API. */
export function settingsSet(event: ToolPkg.PublicApiEvent<{ profile: string; settings: Settings }>): Promise<Settings> { return writeSettings(event.payload.profile, event.payload.settings); }

/** Opens an owner-scoped response session and activates its prompt rules for the actual chat. */
export async function sessionOpen(event: ToolPkg.PublicApiEvent<OpenRequest>): Promise<OpenResult> {
  const chatId = identifier(event.payload.chatId, "chatId");
  const settings = await readSettings(event.payload.profile);
  if (sessions.size >= 64) throw new Error("Waifu supports at most 64 concurrent sessions");
  if ([...sessions.values()].some(session => session.chatId === chatId)) throw new Error("A waifu session is already active for this chat");
  const sessionId = `waifu-${Date.now()}-${++nextSession}`;
  sessions.set(sessionId, { owner: event.callerPackage, chatId, settings, messages: new Map() });
  return { sessionId, settings };
}

/** Resolves a configured emotion to an actual image through packaged resources or explicit custom sources. */
export async function resolveEmotion(category: string, settings: Settings): Promise<Segment> {
  const images = settings.emojis[category];
  if (images === undefined) throw new Error(`No image is configured for emotion: ${category}`);
  const selected = images[Math.floor(Math.random() * images.length)];
  const resource = /^resource:([a-z0-9_]+)$/.exec(selected);
  const source = resource === null ? selected : await ToolPkg.readResource(resource[1]);
  return { kind: "image", source, emotion: category };
}

/** Feeds one authoritative semantic message snapshot and returns only newly stable delivery segments. */
export async function sessionFeed(event: ToolPkg.PublicApiEvent<FeedRequest>): Promise<Segment[]> {
  const request = event.payload, session = owned(request.sessionId, event.callerPackage);
  if (!Number.isSafeInteger(request.messageTimestamp) || typeof request.final !== "boolean") throw new Error("Waifu feed requires a message timestamp and final boolean");
  let state = session.messages.get(request.messageTimestamp);
  if (state === undefined) {
    if (session.messages.size >= 64) throw new Error("Waifu response exceeds 64 assistant messages");
    state = { processor: new SentenceSession(session.settings), snapshot: "", finalized: false };
    session.messages.set(request.messageTimestamp, state);
  }
  if (state.finalized) {
    if (state.snapshot !== request.snapshot) throw new Error("Chat revised an already finalized waifu message");
    return [];
  }
  const raw = state.processor.feed(request.snapshot, request.final), result: Segment[] = [];
  state.snapshot = request.snapshot; state.finalized = request.final;
  for (const segment of raw) result.push(segment.kind === "emotion" ? await resolveEmotion(segment.category, session.settings) : segment);
  return result;
}

/** Releases only a session owned by the authenticated caller. */
export async function sessionClose(event: ToolPkg.PublicApiEvent<{ sessionId: string }>): Promise<{ closed: true }> {
  owned(event.payload.sessionId, event.callerPackage);
  sessions.delete(event.payload.sessionId);
  return { closed: true };
}

/** Reads only explicitly opened dependency settings for one actual conversation. */
export function externalSettings(chatId: string): Settings | null {
  const active = [...sessions.values()].find(session => session.chatId === chatId);
  return active === undefined ? null : active.settings;
}

/** Adds mode rules for ordinary role-bound chat or an explicitly opened dependency session. */
export async function composePrompt(event: ToolPkg.SystemPromptComposeHookEvent): Promise<ToolPkg.PromptHookObjectResult | void> {
  const payload = event.eventPayload;
  if (event.event !== "after_compose_system_prompt" || typeof payload.chatId !== "string") return;
  const external = externalSettings(payload.chatId);
  const mode = external === null ? await modeForChat(payload.chatId) : null;
  if (external === null && mode === null) return;
  if (typeof payload.systemPrompt !== "string") throw new Error("Waifu system prompt hook requires the composed system prompt");
  const settings = external === null ? mode!.settings : external;
  const rules = [settings.customPrompt];
  if (settings.enableEmoticons) {
    const categories = Object.keys(settings.emojis);
    if (categories.length === 0) throw new Error("Emoticons are enabled without any configured images");
    rules.push(`在句末判断情绪，可使用 <emotion>类别</emotion> 输出表情。只允许这些类别：${categories.join(", ")}。例如：很高兴见到你！<emotion>happy</emotion>。没有合适情绪时无需插入。不要输出未定义的类别。`);
  } else rules.push("不要输出 <emotion> 标签。");
  if (settings.enableSelfie) rules.push(`用户要求自拍时，调用已经提供的绘图工具生成图片，外貌关键词：${settings.selfiePrompt}。按用户要求补充表情、动作、服装和背景。使用 Markdown 图片返回工具实际生成的图片地址。`);
  return { systemPrompt: `${payload.systemPrompt}\n\n[Waifu]\n${rules.join("\n")}` };
}
