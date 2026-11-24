import screen from "./ui";
import { cancelChat, composePrompt, defaultRule, inputHook as routeInput, openOriginal, publish, removeRule, saveRule, snapshot } from "./service";
import type { Rule } from "./model";

const TOGGLE_ID = "ai_chat_presentation.enabled";
const WAIFU_PACKAGE = "com.operit.waifu";
const migrations = new Map<string, Promise<Rule>>();
RuntimeContext.register({ cancelChat, openOriginal, removeRule, saveRule, snapshot });
ToolPkg.ipc.on("ai_chat_presentation.publish", publish);
export { composePrompt };

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

async function currentRule(chatId: string): Promise<Rule | null> {
  const { state } = await snapshot(false);
  // Source creation hooks may copy chat extensions; children must keep native execution.
  if (state.rules.some(rule => rule.originalChatId === chatId)) return null;
  const rule = state.rules.find(rule => rule.chatId === chatId);
  if (rule) return rule;
  const { extension } = await Tools.Chat.readExtension({ kind: "chat", chatId });
  return { ...defaultRule(chatId, "分段对话"), enabled: extension?.enabled === true };
}

async function prepareRule(rule: Rule): Promise<Rule> {
  let pending = migrations.get(rule.chatId);
  if (!pending) {
    pending = saveRule(rule).then(result => result.state.rules.find(item => item.chatId === rule.chatId)!);
    migrations.set(rule.chatId, pending);
  }
  try { return await pending; }
  finally { if (migrations.get(rule.chatId) === pending) migrations.delete(rule.chatId); }
}

/** Keeps the existing conversation switch and prepares its original transcript on enable. */
export async function inputMenu(event: ToolPkg.InputMenuToggleHookEvent): Promise<ToolPkg.InputMenuToggleObjectResult | void> {
  const { action, chatId, toggleId } = event.eventPayload;
  const isToggle = action === "toggle" && toggleId === TOGGLE_ID;
  if (!chatId || (action !== "create" && !isToggle)) return;
  const rule = await currentRule(chatId);
  if (!rule) return;
  if (action === "create") {
    const chinese = getLang().toLowerCase().startsWith("zh");
    return { toggles: [{ id: TOGGLE_ID, title: chinese ? "AI 分段对话" : "AI Segmented Chat",
      icon: "AutoAwesome", isChecked: rule.enabled,
      description: chinese
        ? "由 AI 选择展示内容和消息分段，原始记录保留在子对话中。当前会话开启 Waifu 时优先使用 Waifu。"
        : "Let AI select message content and segments; keep the full transcript in a child chat. Waifu takes priority when active.",
    }] };
  }
  await prepareRule({ ...rule, enabled: !rule.enabled });
}

/** Upgrades previously enabled conversation switches without changing their identity. */
export async function inputHook(event: ToolPkg.ChatInputHookEvent): Promise<ToolPkg.ChatInputHookObjectResult | void> {
  const payload = event.eventPayload;
  if (event.event !== "submit_requested" || !payload.chatId || payload.source === "Sequence" || payload.source === "Tool") return;
  const rule = await currentRule(payload.chatId);
  if (!rule?.enabled || await waifuIsActive(payload.chatId)) return;
  if (payload.isProcessing) return { action: "Block", message: "请等待原有回复完成后再发送" };
  if (!rule.originalChatId || !rule.historyCopied) await prepareRule(rule);
  return routeInput(event);
}

export function registerToolPkg(): boolean {
  ToolPkg.registerToolboxUiModule({ id: "ai_chat_presentation_settings", runtime: "compose_dsl", screen,
    params: {}, title: { zh: "AI 分段对话", en: "AI Segmented Chat" } });
  ToolPkg.registerInputMenuTogglePlugin({ id: "ai_reply_menu", function: inputMenu });
  ToolPkg.registerChatInputHook({ id: "ai_reply_input", function: inputHook });
  ToolPkg.registerSystemPromptComposeHook({ id: "ai_reply_prompt", function: composePrompt });
  return true;
}
