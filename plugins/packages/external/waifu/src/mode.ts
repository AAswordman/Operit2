import type { Settings } from "./api.js";
import { defaultSettings, identifier, validateSettings } from "./settings.js";

export interface Mode {
  enabled: boolean;
  mergeSend: boolean;
  mergeDelayMs: number;
  settings: Settings;
}
export interface Target { selection: string; name: string }
export interface ModeContext { target: Target; mode: Mode }
interface ModeStore { bindings: Record<string, Mode> }
type Active = { CharacterCard: { id: string } } | { CharacterGroup: { id: string } } | { Model: { id: "chat" } };
let store: Promise<ModeStore> | null = null;

/** Loads only the independent ordinary-chat mode authoring state. */
function loadModes(): Promise<ModeStore> {
  if (store === null) store = PluginConfig.use("mode", { bindings: {} });
  return store;
}

/** Calls the declared character dependency without interpreting its private storage. */
function characters<I, O>(method: string, input: I): Promise<O> {
  return ToolPkg.callDependency<I, O>("com.operit.character_cards", method, input);
}

/** Resolves the exact selection already persisted on an actual conversation. */
export async function chatTarget(chatId: string): Promise<string> {
  const binding = await characters<{ chatId: string }, { chatId: string; selection: string }>("chat.configuration.binding.read", { chatId: identifier(chatId, "chatId") });
  return identifier(binding.selection, "character selection");
}

/** Reads the same currently active card or group shown by the original Waifu settings page. */
export async function activeTarget(): Promise<Target> {
  const active = await characters<Record<string, never>, Active>("activePrompt.get", {});
  if ("CharacterCard" in active) {
    const card = await characters<{ id: string }, { name: string }>("character.get", active.CharacterCard);
    return { selection: `card:${active.CharacterCard.id}`, name: card.name };
  }
  if ("CharacterGroup" in active) {
    const group = await characters<{ id: string }, { name: string }>("group.get", active.CharacterGroup);
    return { selection: `group:${active.CharacterGroup.id}`, name: group.name };
  }
  if (active.Model.id !== "chat") throw new Error("Invalid active model selection");
  return { selection: "model:chat", name: "模型对话" };
}

/** Authors a new explicitly selected role binding with the original disabled defaults. */
export async function ensureMode(selection: string): Promise<Mode> {
  identifier(selection, "selection");
  const config = await loadModes();
  if (config.bindings[selection] === undefined) {
    config.bindings[selection] = { enabled: false, mergeSend: false, mergeDelayMs: 5000, settings: defaultSettings() };
    await PluginConfig.flush(config);
  }
  return JSON.parse(JSON.stringify(config.bindings[selection])) as Mode;
}

/** Reads an enabled role mode without creating a binding during ordinary model execution. */
export async function modeForChat(chatId: string): Promise<Mode | null> {
  const selection = await chatTarget(chatId), config = await loadModes();
  const mode = config.bindings[selection];
  if (mode === undefined || !mode.enabled) return null;
  return JSON.parse(JSON.stringify(mode)) as Mode;
}

/** Loads the authoring context used by native settings and CLI controls. */
export async function modeContext(): Promise<ModeContext> {
  const target = await activeTarget();
  return { target, mode: await ensureMode(target.selection) };
}

/** Saves one complete role-bound mode and validates timing before persistence. */
export async function saveMode(selection: string, mode: Mode): Promise<Mode> {
  identifier(selection, "selection");
  if (typeof mode.enabled !== "boolean" || typeof mode.mergeSend !== "boolean") throw new Error("Mode switches must be boolean");
  if (!Number.isSafeInteger(mode.mergeDelayMs) || mode.mergeDelayMs < 500 || mode.mergeDelayMs > 10000) throw new Error("Merge delay must be between 500 and 10000 ms");
  const checked: Mode = { enabled: mode.enabled, mergeSend: mode.mergeSend, mergeDelayMs: mode.mergeDelayMs, settings: validateSettings(mode.settings) };
  const config = await loadModes();
  config.bindings = { ...config.bindings, [selection]: checked };
  await PluginConfig.flush(config);
  return JSON.parse(JSON.stringify(checked)) as Mode;
}

/** Adds a native input-menu switch for the current conversation's bound role. */
export async function inputMenu(event: ToolPkg.InputMenuToggleHookEvent): Promise<ToolPkg.InputMenuToggleObjectResult | void> {
  const { action, chatId, toggleId } = event.eventPayload;
  if (chatId === undefined || (action !== "create" && action !== "toggle")) return;
  if (action === "toggle" && toggleId !== "waifu.mode") return;
  const selection = await chatTarget(chatId), mode = await ensureMode(selection);
  if (action === "toggle") await saveMode(selection, { ...mode, enabled: !mode.enabled });
  else return { toggles: [{ id: "waifu.mode", title: "Waifu 模式", description: "按句子发送气泡，模拟自然聊天节奏", icon: "Face", isChecked: mode.enabled }] };
}
