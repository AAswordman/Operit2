import { presentationPrompt, selectExcerpts } from "./selection.js";

const TOGGLE_ID = "ai_chat_presentation.enabled";
const DECISION_TIMEOUT_MS = 8_000;
const MAX_DECISIONS = 4;
const MAX_REPLY_CHARS = 12_000;
const WAIFU_PACKAGE = "com.operit.waifu";
const pendingDecisions = new Set<Promise<unknown>>();

async function isEnabled(chatId: string): Promise<boolean> {
  const { extension } = await Tools.Chat.readExtension({ kind: "chat", chatId });
  return extension?.enabled === true;
}

/** Defers to Waifu when it owns presentation for this conversation. */
async function waifuIsActive(chatId: string): Promise<boolean> {
  try {
    const state = await ToolPkg.callDependency<{ chatId: string }, { enabled: boolean }>(
      WAIFU_PACKAGE, "mode.status", { chatId },
    );
    return state.enabled !== false;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (/(?:ToolPkg container not found|(?:package|extension owner|public API owner) is disabled): com\.operit\.waifu\b/i.test(message)) return false;
    console.error("[ai_chat_presentation.waifu]", error);
    return true;
  }
}

/** Stores the switch on the conversation identified by the input menu. */
export async function inputMenu(event: ToolPkg.InputMenuToggleHookEvent): Promise<ToolPkg.InputMenuToggleObjectResult | void> {
  const { action, chatId, toggleId } = event.eventPayload;
  const isToggle = action === "toggle" && toggleId === TOGGLE_ID;
  if (!chatId || (action !== "create" && !isToggle)) return;
  const enabled = await isEnabled(chatId);
  if (action === "create") {
    const chinese = getLang().toLowerCase().startsWith("zh");
    return {
      toggles: [{
        id: TOGGLE_ID,
        title: chinese ? "AI 自主回复" : "AI Reply Presentation",
        icon: "AutoAwesome",
        isChecked: enabled,
        description: chinese
          ? "让 AI 选择显示哪些内容和气泡。当前会话开启 Waifu 时优先使用 Waifu。"
          : "Let AI choose what to show in chat bubbles. Waifu takes priority when active.",
      }],
    };
  }
  await Tools.Chat.writeExtension({ kind: "chat", chatId }, { enabled: !enabled });
}

async function plan(chatId: string, messageTimestamp: number, reply: string): Promise<string[] | null> {
  if (reply.length > MAX_REPLY_CHARS || pendingDecisions.size >= MAX_DECISIONS) return null;
  let timer: number | undefined;
  let timedOut = false;
  try {
    const timeout = new Promise<null>(resolve => {
      timer = setTimeout(() => {
        timedOut = true;
        resolve(null);
      }, DECISION_TIMEOUT_MS);
    });
    const decision = (async () => {
      const { messages } = await Tools.Chat.getMessages(chatId, { order: "desc", limit: 16 });
      const user = messages.find(message => message.sender === "user" && message.timestamp <= messageTimestamp)?.content;
      if (timedOut || !user || pendingDecisions.size >= MAX_DECISIONS) return null;
      const request = Tools.Chat.call({
        functionType: "CHAT",
        enableThinking: false,
        recordTokenUsage: true,
        turns: [
          { kind: "SYSTEM", content: presentationPrompt },
          { kind: "USER", content: JSON.stringify({ user: user.slice(0, 2_000), reply }) },
        ],
      });
      pendingDecisions.add(request);
      try {
        const result = await request;
        return timedOut ? null : selectExcerpts(result.text, reply);
      } finally {
        pendingDecisions.delete(request);
      }
    })().catch(error => {
      console.error("[ai_chat_presentation.plan]", error);
      return null;
    });
    return await Promise.race([decision, timeout]);
  } finally {
    if (timer !== undefined) clearTimeout(timer);
  }
}

/** Waits for the complete reply and publishes an append-only display projection. */
export async function responseHook(event: ToolPkg.ChatResponseHookEvent): Promise<ToolPkg.ChatResponseHookOutput> {
  const { chatId, messageTimestamp, parts } = event.eventPayload;
  if (event.event === "started") {
    try {
      return await isEnabled(chatId) && !(await waifuIsActive(chatId));
    } catch (error) {
      console.error("[ai_chat_presentation.start]", error);
      return false;
    }
  }
  if (event.event === "snapshot") return { bubbles: [] };
  if (event.event !== "finished") return;
  const markdown = parts.filter(part => part.kind === "markdown");
  const reply = markdown.map(part => part.content).join("\n\n");
  if (!reply.trim()) return { bubbles: [] };
  const selected = await plan(chatId, messageTimestamp, reply);
  if (selected === null) return { bubbles: [{ parts: markdown, delayMs: 0 }] };
  return { bubbles: selected.map((content, index) => ({
    parts: [{ partId: `ai-presentation-${index}`, sequence: 0, kind: "markdown", content, toolCallId: null, toolName: null, attributes: {} }],
    delayMs: 0,
  })) };
}

export function registerToolPkg(): boolean {
  ToolPkg.registerInputMenuTogglePlugin({ id: "ai_reply_menu", function: inputMenu });
  ToolPkg.registerChatResponseHook({ id: "ai_reply_presentation", function: responseHook });
  return true;
}
