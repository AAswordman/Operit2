import toolboxUI from "./ui/index.ui.js";
import {
  appendExtraInfoToMessage,
  getExtraInfoInjectionEnabled,
  loadSettings,
  resolveExtraInfoI18n,
  saveSettings,
  setExtraInfoInjectionEnabled,
} from "./shared";

RuntimeContext.register({
  loadSettings,
  saveSettings,
});

/** Passes the prompt execution identity to extra information injection. */
async function appendExtraInfoWithStatus(
  processedInput: string,
  chatId?: string,
  executionContext?: ToolPkg.ExecutionContext
) {
  return appendExtraInfoToMessage(
    processedInput,
    chatId || undefined,
    executionContext
  );
}

/** Reads the exact participant selected by the generic prompt host. */
function resolveHookExecutionContext(
  input: ToolPkg.PromptInputHookEvent | ToolPkg.PromptFinalizeHookEvent
): ToolPkg.ExecutionContext | undefined {
  return input.eventPayload.metadata?.executionContext;
}

/** Registers the settings screen and prompt injection hooks. */
export function registerToolPkg(): boolean {
  ToolPkg.registerToolboxUiModule({
    id: "message_insert_settings",
    runtime: "compose_dsl",
    screen: toolboxUI,
    params: {},
    title: {
      zh: "额外信息注入",
      en: "Extra Info Injection",
    },
  });

  ToolPkg.registerPromptInputHook({
    id: "message_insert_prompt_input",
    function: onPromptInput,
  });

  ToolPkg.registerPromptFinalizeHook({
    id: "message_insert_prompt_finalize",
    function: onPromptFinalize,
  });

  ToolPkg.registerInputMenuTogglePlugin({
    id: "message_insert_input_menu_toggle",
    function: onInputMenuToggle,
  });

  return true;
}

/** Injects persisted attachments before input processing when enabled. */
export async function onPromptInput(
  input: ToolPkg.PromptInputHookEvent
) {
  const stage = String(input.eventPayload.stage ?? input.eventName ?? "");
  if (stage !== "before_process") {
    return null;
  }

  const settings = await loadSettings();
  if (!settings.persistInjectedContent) {
    return null;
  }

  const processedInput = String(
    input.eventPayload.processedInput ?? input.eventPayload.rawInput ?? ""
  );
  if (!processedInput.trim()) {
    return null;
  }

  const chatId = String(input.eventPayload.chatId ?? getChatId() ?? "").trim();
  const executionContext = resolveHookExecutionContext(input);
  return appendExtraInfoWithStatus(
    processedInput,
    chatId || undefined,
    executionContext
  );
}

/** Injects transient attachments into the final model request when enabled. */
export async function onPromptFinalize(
  input: ToolPkg.PromptFinalizeHookEvent
) {
  const stage = String(input.eventPayload.stage ?? input.eventName ?? "");
  if (stage !== "before_send_to_model") {
    return null;
  }

  const settings = await loadSettings();
  if (settings.persistInjectedContent) {
    return null;
  }

  const processedInput = String(
    input.eventPayload.processedInput ?? input.eventPayload.rawInput ?? ""
  );
  if (!processedInput.trim()) {
    return null;
  }

  const chatId = String(input.eventPayload.chatId ?? getChatId() ?? "").trim();
  const executionContext = resolveHookExecutionContext(input);
  return appendExtraInfoWithStatus(
    processedInput,
    chatId || undefined,
    executionContext
  );
}

/** Reads or toggles the extra information injection menu state. */
export async function onInputMenuToggle(
  input: ToolPkg.InputMenuToggleHookEvent
): Promise<ToolPkg.InputMenuToggleDefinitionResult[]> {
  const action = String(input.eventPayload.action ?? "").toLowerCase();

  if (action === "toggle") {
    await setExtraInfoInjectionEnabled(!(await getExtraInfoInjectionEnabled()));
    return [];
  }

  if (action !== "create") {
    return [];
  }

  const text = resolveExtraInfoI18n();
  return [
    {
      id: "message_extra_info_injection",
      icon: "post_add",
      title: text.menuTitle,
      description: text.menuDescription,
      isChecked: await getExtraInfoInjectionEnabled(),
    },
  ];
}
