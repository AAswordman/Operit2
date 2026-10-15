import {
  CHAT_VIEW_HOOK_ID,
  MENU_HOOK_ID,
  PLANASK_XML_RENDER_HOOK_ID,
  PLANASK_XML_TAG,
  PROMPT_ESTIMATE_FINALIZE_HOOK_ID,
  PROMPT_FINALIZE_HOOK_ID,
  PROMPT_HOOK_ID,
  TOOL_PROMPT_HOOK_ID,
  TOGGLE_ID,
  TOGGLE_ICON,
  XML_RENDER_HOOK_ID,
  XML_TAG,
} from "../shared/plan_mode_constants.js";
import { resolvePlanModeI18n, type PlanModeI18n } from "../shared/plan_mode_i18n.js";
import {
  PlanModeShared,
  registerSharedMethods,
} from "../shared/plan_mode_runtime_ipc.js";
import { appendPrompt, buildExistingPlanPrompt, buildPlanningModePrompt } from "../shared/plan_mode_prompt.js";
import { type PlanModeRuntime } from "../shared/plan_mode_state.js";
import {
  PLAN_MODE_IS_PLAN_STARTED_IPC_CHANNEL,
  PLAN_MODE_START_IMPLEMENTATION_IPC_CHANNEL,
  type StartPlanImplementationResult,
} from "../shared/plan_mode_execution.js";
import {
  PLAN_MODE_SUBMIT_ANSWERS_IPC_CHANNEL,
  type SubmitPlanaskAnswersRequest,
  type SubmitPlanaskAnswersResult,
} from "../shared/plan_mode_ask_execution.js";
import { type PlanImplementationRequest } from "../shared/plan_mode_execution.js";
import {
  logPlanModeDebug,
} from "../shared/plan_mode_workspace.js";
import { onPlanaskXmlRender } from "./planask-xml-render-plugin.js";
import { onPlantodoXmlRender } from "./plantodo-xml-render-plugin.js";

const PLAN_MODE_BLOCKED_TOOL_NAMES = new Set([
  "apply_file",
  "create_file",
  "edit_file",
  "delete_file",
]);

const PLAN_MODE_COMMAND_ID = "plan_mode_command";
const PLAN_MODE_COMMAND_NAME = "plan";

let planModeIpcRegistered = false;

/** Tests whether a prompt hook invocation belongs to chat composition. */
function usesChatPrompt(
  payload: Pick<ToolPkg.PromptHookEventPayload, "promptFunctionType" | "functionType">
): boolean {
  const promptFunctionType = payload.promptFunctionType;
  if (promptFunctionType !== undefined && promptFunctionType !== "") {
    return promptFunctionType === "CHAT";
  }
  const functionType = payload.functionType;
  return functionType === undefined || functionType === "" || functionType === "CHAT";
}

/** Tests whether a runtime can host plan-mode chat views. */
function isPlanModeRuntime(value: string | undefined): value is PlanModeRuntime {
  return value === "main" || value === "floating";
}

/** Tests whether a value is a non-empty string. */
function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim() !== "";
}

/** Executes the /plan command against the active chat view. */
export async function onPlanModeCommand(
  event: ToolPkg.CoreCommandHookEvent
): Promise<ToolPkg.CoreCommandResult> {
  const text = resolvePlanModeI18n();
  const args = event.eventPayload.args.map((arg) => arg.trim()).filter((arg) => arg !== "");
  const action = args.length === 1 ? args[0].toLowerCase() : "";
  if (action === "status") {
    const activeView = await PlanModeShared.getSingleActiveChatView();
    if (!activeView) {
      return {
        stderr: text.commandChatViewMissing,
        json: { ok: false, error: "chat_view_missing" },
      };
    }
    const enabled = await PlanModeShared.isEnabled(activeView.chatId);
    return {
      stdout: enabled ? text.commandStatusEnabled : text.commandStatusDisabled,
      json: {
        ok: true,
        action,
        enabled,
        chatId: activeView.chatId,
        workspacePath: activeView.workspacePath,
      },
    };
  }
  if (action === "off") {
    const activeView = await PlanModeShared.getSingleActiveChatView();
    if (!activeView) {
      return {
        stderr: text.commandChatViewMissing,
        json: { ok: false, error: "chat_view_missing" },
      };
    }
    await PlanModeShared.disable(activeView.chatId);
    return {
      stdout: text.commandDisabled,
      json: { ok: true, action, enabled: false, chatId: activeView.chatId },
    };
  }
  const message = args.length === 1 && action !== "on" ? args[0] : args.join(" ");
  if (args.length === 1 && action === "on") {
    return await enablePlanModeFromCommand(text);
  }
  const activeView = await PlanModeShared.getSingleActiveChatView();
  if (!activeView) {
    return {
      stderr: text.commandChatViewMissing,
      json: { ok: false, error: "chat_view_missing" },
    };
  }

  const workspace = await PlanModeShared.resolveWorkspace(activeView.chatId, activeView.runtime);
  if (!workspace) {
    return {
      stderr: text.toastWorkspaceRequired,
      json: { ok: false, action: message ? "message" : "on", error: "workspace_required" },
    };
  }
  await PlanModeShared.enable(activeView.chatId);
  if (message) {
    void Tools.Chat.sendMessage({ kind: "submit", chatId: activeView.chatId, runtime: activeView.runtime, input: { text: message, attachments: [], replyToMessageTimestamp: null }, turn: { kind: "execute" }, notifyReply: false }).catch((error) => {
      const errorText = error instanceof Error ? error.message || "error" : String(error);
      void Tools.System.toast(`${text.toastPlanSendFailedPrefix}${errorText}`);
    });
  }
  return {
    stdout: text.commandEnabled,
    json: {
      ok: true,
      action: message ? "message" : "on",
      enabled: true,
      chatId: activeView.chatId,
      workspacePath: workspace.workspacePath,
      message: message || undefined,
    },
  };
}

/** Enables plan mode from the explicit /plan on command. */
async function enablePlanModeFromCommand(text: PlanModeI18n): Promise<ToolPkg.CoreCommandResult> {
  const activeView = await PlanModeShared.getSingleActiveChatView();
  if (!activeView) {
    return {
      stderr: text.commandChatViewMissing,
      json: { ok: false, error: "chat_view_missing" },
    };
  }
  const workspace = await PlanModeShared.resolveWorkspace(activeView.chatId, activeView.runtime);
  if (!workspace) {
    return {
      stderr: text.toastWorkspaceRequired,
      json: { ok: false, action: "on", error: "workspace_required" },
    };
  }
  await PlanModeShared.enable(activeView.chatId);
  return {
    stdout: text.commandEnabled,
    json: { ok: true, action: "on", enabled: true, chatId: activeView.chatId, workspacePath: workspace.workspacePath },
  };
}

/** Sends plan clarification answers through the chat view that rendered the UI. */
async function handleSubmitPlanaskAnswersIpc(
  request: SubmitPlanaskAnswersRequest
): Promise<SubmitPlanaskAnswersResult> {
  const text = resolvePlanModeI18n();
  try {
    const trackedView = await PlanModeShared.getTrackedChatViewByChatId(request.chatId);
    if (!trackedView) {
      await Tools.System.toast(text.toastChatViewMissing);
      return {
        success: false,
        error: text.toastChatViewMissing,
      };
    }
    const workspace = await PlanModeShared.resolveWorkspace(trackedView.chatId, trackedView.runtime);
    if (!workspace) {
      await Tools.System.toast(text.toastWorkspaceRequired);
      return {
        success: false,
        error: text.toastWorkspaceRequired,
      };
    }

    void Tools.Chat.sendMessage({ kind: "submit", chatId: trackedView.chatId, runtime: trackedView.runtime, input: { text: request.message, attachments: [], replyToMessageTimestamp: null }, turn: { kind: "execute" }, notifyReply: false }).catch((error) => {
      const errorText = error instanceof Error
        ? error.message || "error"
        : (typeof error === "string" || error == null ? error || "error" : "error");
      const toastMessage = `${text.askToastAnswerSendFailedPrefix}${errorText}`;
      void Tools.System.toast(toastMessage);
    });
    await Tools.System.toast(text.askToastAnswerSent);
    return { success: true };
  } catch (error) {
    const errorText = error instanceof Error
      ? error.message || "error"
      : (typeof error === "string" || error == null ? error || "error" : "error");
    const messageText = `${text.askToastAnswerSendFailedPrefix}${errorText}`;
    await Tools.System.toast(messageText);
    return {
      success: false,
      error: messageText,
    };
  }
}

/** Persists a confirmed plan and triggers implementation in its rendered chat. */
async function handleStartImplementationIpc(
  request: PlanImplementationRequest
): Promise<StartPlanImplementationResult> {
  const text = resolvePlanModeI18n();
  const normalizedPlanContent = request.planContent.trim();
  if (!normalizedPlanContent) {
    const messageText = text.toastPlanEmpty;
    await Tools.System.toast(messageText);
    return { success: false, error: messageText };
  }

  try {
    const trackedView = await PlanModeShared.getTrackedChatViewByChatId(request.chatId);
    if (!trackedView) {
      await Tools.System.toast(text.toastChatViewMissing);
      return { success: false, error: text.toastChatViewMissing };
    }
    const workspace = await PlanModeShared.resolveWorkspace(trackedView.chatId, trackedView.runtime);
    if (!workspace) {
      await Tools.System.toast(text.toastWorkspaceRequired);
      return { success: false, error: text.toastWorkspaceRequired };
    }
    if (!(await PlanModeShared.claimPlanStart(normalizedPlanContent, trackedView.chatId))) {
      await Tools.System.toast(text.toastPlanAlreadyStarted);
      return { success: true, alreadyStarted: true };
    }
    try {
      const written = await PlanModeShared.writePlanFile(workspace.chatId, normalizedPlanContent);
      await PlanModeShared.disable(written.chatId);
      void Tools.Chat.sendMessage({ kind: "submit", chatId: written.chatId, runtime: workspace.runtime ?? trackedView.runtime, input: { text: text.implementationMessage, attachments: [], replyToMessageTimestamp: null }, turn: { kind: "execute" }, notifyReply: false }).catch((error) => {
        const errorText = error instanceof Error
          ? error.message || "error"
          : (typeof error === "string" || error == null ? error || "error" : "error");
        const messageText = `${text.toastPlanSendFailedPrefix}${errorText}`;
        void Tools.System.toast(messageText);
      });
      void Tools.System.toast(text.toastPlanStarted);
      return { success: true };
    } catch (error) {
      await PlanModeShared.forgetPlanStart(normalizedPlanContent, trackedView.chatId);
      throw error;
    }
  } catch (error) {
    const errorText = error instanceof Error
      ? error.message || "error"
      : (typeof error === "string" || error == null ? error || "error" : "error");
    const messageText = `${text.toastPlanWriteFailedPrefix}${errorText}`;
    await Tools.System.toast(messageText);
    return { success: false, error: messageText };
  }
}

/** Reads whether a plan was already handed off for the rendered chat. */
async function handleIsPlanStartedIpc(request: PlanImplementationRequest): Promise<boolean> {
  try {
    return await PlanModeShared.isPlanStarted(request.planContent, request.chatId);
  } catch (error) {
    logPlanModeDebug("handleIsPlanStartedIpc.error", {
      message: error instanceof Error ? error.message : "error",
    });
    return false;
  }
}

/** Registers every IPC endpoint required by plan mode once per runtime. */
function registerPlanModeIpc(): void {
  if (planModeIpcRegistered) {
    return;
  }
  planModeIpcRegistered = true;
  registerSharedMethods(PlanModeShared);
  ToolPkg.ipc.on<SubmitPlanaskAnswersRequest, SubmitPlanaskAnswersResult>(
    PLAN_MODE_SUBMIT_ANSWERS_IPC_CHANNEL,
    handleSubmitPlanaskAnswersIpc
  );
  ToolPkg.ipc.on<PlanImplementationRequest, StartPlanImplementationResult>(
    PLAN_MODE_START_IMPLEMENTATION_IPC_CHANNEL,
    handleStartImplementationIpc
  );
  ToolPkg.ipc.on<PlanImplementationRequest, boolean>(
    PLAN_MODE_IS_PLAN_STARTED_IPC_CHANNEL,
    handleIsPlanStartedIpc
  );
}

/** Removes file mutation tools while the chat is in planning mode. */
function filterPlanModeTools(
  availableTools: ToolPkg.ToolPromptItem[]
): ToolPkg.ToolPromptItem[] {
  return availableTools.filter((tool) => !PLAN_MODE_BLOCKED_TOOL_NAMES.has(tool.name));
}

/** Injects a plan prompt into the system turn of prepared history. */
function appendPlanPromptToPreparedHistory(
  preparedHistory: ToolPkg.PromptTurn[],
  extraPrompt: string
): ToolPkg.PromptTurn[] {
  const nextHistory = preparedHistory.slice();
  const systemIndex = nextHistory.findIndex((turn) => turn.kind === "SYSTEM");
  if (systemIndex < 0) {
    return [
      {
        kind: "SYSTEM",
        content: extraPrompt,
      },
      ...nextHistory,
    ];
  }
  const systemTurn = nextHistory[systemIndex];
  nextHistory[systemIndex] = {
    ...systemTurn,
    content: appendPrompt(systemTurn.content, extraPrompt),
  };
  return nextHistory;
}

/** Creates and handles the chat input plan-mode toggle. */
export async function onInputMenuToggle(
  event: ToolPkg.InputMenuToggleHookEvent
): Promise<ToolPkg.InputMenuToggleObjectResult | null> {
  try {
    const payload = event.eventPayload;
    const action = payload.action;
    const runtime = payload.runtime;
    const chatId = payload.chatId;
    if ((action !== "create" && action !== "toggle") || !isPlanModeRuntime(runtime) || chatId === undefined) {
      return null;
    }
    const text = resolvePlanModeI18n();
    const enabled = await PlanModeShared.isEnabled(chatId);
    const workspace = await PlanModeShared.resolveWorkspace(chatId, runtime);
    logPlanModeDebug("onInputMenuToggle", {
      action,
      runtime,
      chatId,
      enabled,
      hasWorkspace: !!workspace,
      workspacePath: workspace?.workspacePath,
    });

    if (action === "create") {
      return {
        toggles: [
          {
            id: TOGGLE_ID,
            title: text.menuTitle,
            description: workspace
              ? enabled
                ? text.menuDescriptionEnabled
                : text.menuDescriptionDisabled
              : text.menuDescriptionWorkspaceMissing,
            icon: TOGGLE_ICON,
            isChecked: enabled,
          },
        ],
      };
    }

    if (action === "toggle" && payload.toggleId === TOGGLE_ID) {
      if (enabled) {
        logPlanModeDebug("toggle.disable", { chatId, runtime });
        await PlanModeShared.disable(chatId);
        return null;
      }
      if (!workspace) {
        logPlanModeDebug("toggle.enable_denied_workspace_missing", { chatId, runtime });
        await Tools.System.toast(text.toastWorkspaceRequired);
        return null;
      }
      logPlanModeDebug("toggle.enable", {
        chatId,
        runtime,
        workspacePath: workspace.workspacePath,
        workspaceEnv: workspace.workspaceEnv,
      });
      await PlanModeShared.enable(workspace.chatId);
    }

    return null;
  } catch (error) {
    logPlanModeDebug("onInputMenuToggle.error", {
      message: error instanceof Error ? error.message : "error",
    });
    return null;
  }
}

/** Tracks active chat views and their workspace bindings. */
export async function onChatViewEvent(
  event: ToolPkg.ChatViewHookEvent
): Promise<void> {
  try {
    const payload = event.eventPayload;
    const eventName = event.eventName;
    const runtime = payload.runtime;
    const viewId = payload.viewId;
    const chatId = payload.chatId;
    const workspacePath = payload.workspacePath;
    const workspaceEnv = payload.workspaceEnv;
    const title = payload.title;
    if (
      !isPlanModeRuntime(runtime) ||
      !isNonEmptyString(viewId) ||
      !isNonEmptyString(chatId)
    ) {
      logPlanModeDebug("onChatViewEvent.ignore_invalid_workspace", {
        eventName,
        runtime,
        viewId: typeof viewId === "string" ? viewId : undefined,
        chatId: typeof chatId === "string" ? chatId : undefined,
        workspacePath:
          typeof workspacePath === "string" ? workspacePath : workspacePath === null ? "null" : undefined,
        workspaceEnv:
          typeof workspaceEnv === "string" ? workspaceEnv : workspaceEnv === null ? "null" : undefined,
      });
      return;
    }

    // Closing or unbinding a view is valid even when its workspace path is empty.
    if (eventName === "view_closed") {
      await PlanModeShared.removeTrackedChatView(runtime, viewId);
      return;
    }
    if (!isNonEmptyString(workspacePath)) {
      await PlanModeShared.removeTrackedChatView(runtime, viewId);
      await PlanModeShared.disable(chatId);
      logPlanModeDebug("onChatViewEvent.workspace_unbound", { eventName, runtime, viewId, chatId });
      return;
    }
    if (typeof title !== "string") {
      logPlanModeDebug("onChatViewEvent.ignore_invalid_title", { eventName, runtime, viewId, chatId });
      return;
    }

    logPlanModeDebug("onChatViewEvent", {
      eventName,
      runtime,
      viewId,
      chatId,
      workspacePath,
      workspaceEnv,
      title,
    });

    await PlanModeShared.upsertTrackedChatView({
      viewId,
      runtime,
      chatId,
      workspacePath,
      workspaceEnv: isNonEmptyString(workspaceEnv) ? workspaceEnv : undefined,
      title,
      updatedAt: Date.now(),
    });
  } catch (error) {
    logPlanModeDebug("onChatViewEvent.error", {
      message: error instanceof Error ? error.message : "error",
    });
  }
}

/** Adds the relevant plan-mode instruction during system prompt composition. */
export async function onSystemPromptCompose(
  event: ToolPkg.SystemPromptComposeHookEvent
): Promise<ToolPkg.PromptHookObjectResult | null> {
  try {
    const payload = event.eventPayload;
    const stage = payload.stage === undefined ? event.eventName : payload.stage;
    if (stage !== "after_compose_system_prompt" || !usesChatPrompt(payload)) {
      return null;
    }

    const useEnglish = payload.useEnglish === true;
    const chatId = payload.chatId;
    const currentPrompt = payload.systemPrompt;
    if (chatId === undefined || currentPrompt === undefined) {
      return null;
    }
    if (await PlanModeShared.isEnabled(chatId)) {
      return {
        systemPrompt: appendPrompt(currentPrompt, buildPlanningModePrompt(useEnglish)),
      };
    }

    const workspace = await PlanModeShared.resolveWorkspace(chatId);
    if (!workspace || !(await PlanModeShared.hasPlanFile(workspace.chatId))) {
      return null;
    }

    return {
      systemPrompt: appendPrompt(currentPrompt, buildExistingPlanPrompt(useEnglish)),
    };
  } catch (error) {
    logPlanModeDebug("onSystemPromptCompose.error", {
      message: error instanceof Error ? error.message : "error",
    });
    return null;
  }
}

/** Filters side-effecting tools while a chat is planning. */
export async function onToolPromptCompose(
  event: ToolPkg.ToolPromptComposeHookEvent
): Promise<ToolPkg.PromptHookObjectResult | null> {
  try {
    const payload = event.eventPayload;
    const stage = payload.stage === undefined ? event.eventName : payload.stage;
    if (stage !== "filter_tool_prompt_items" && stage !== "filter_tool_call_tools") {
      return null;
    }

    const chatId = payload.chatId;
    const availableTools = payload.availableTools;
    if (chatId === undefined || availableTools === undefined || !usesChatPrompt(payload)) {
      return null;
    }
    if (!(await PlanModeShared.isEnabled(chatId))) {
      return null;
    }

    const filteredTools = filterPlanModeTools(availableTools);
    logPlanModeDebug("onToolPromptCompose", {
      chatId,
      stage,
      toolCountBefore: availableTools.length,
      toolCountAfter: filteredTools.length,
    });
    return {
      availableTools: filteredTools,
    };
  } catch (error) {
    logPlanModeDebug("onToolPromptCompose.error", {
      message: error instanceof Error ? error.message : "error",
    });
    return null;
  }
}

/** Builds the final history update for a planning or implementation chat. */
async function buildPromptFinalizeResult(
  payload: ToolPkg.PromptHookEventPayload
): Promise<ToolPkg.PromptHookObjectResult | null> {
  const chatId = payload.chatId;
  const preparedHistory = payload.preparedHistory;
  if (chatId === undefined || preparedHistory === undefined || !usesChatPrompt(payload)) {
    return null;
  }
  const useEnglish = payload.useEnglish === true;
  if (await PlanModeShared.isEnabled(chatId)) {
    return {
      preparedHistory: appendPlanPromptToPreparedHistory(
        preparedHistory,
        buildPlanningModePrompt(useEnglish)
      ),
    };
  }

  const workspace = await PlanModeShared.resolveWorkspace(chatId);
  if (!workspace || !(await PlanModeShared.hasPlanFile(workspace.chatId))) {
    return null;
  }

  return {
    preparedHistory: appendPlanPromptToPreparedHistory(
      preparedHistory,
      buildExistingPlanPrompt(useEnglish)
    ),
  };
}

/** Adds plan instructions immediately before a model request. */
export async function onPromptFinalize(
  event: ToolPkg.PromptFinalizeHookEvent
): Promise<ToolPkg.PromptHookObjectResult | null> {
  try {
    const payload = event.eventPayload;
    const stage = payload.stage === undefined ? event.eventName : payload.stage;
    if (stage !== "before_send_to_model") {
      return null;
    }
    return await buildPromptFinalizeResult(payload);
  } catch (error) {
    logPlanModeDebug("onPromptFinalize.error", {
      message: error instanceof Error ? error.message : "error",
    });
    return null;
  }
}

/** Adds plan instructions while estimating a model request. */
export async function onPromptEstimateFinalize(
  event: ToolPkg.PromptEstimateFinalizeHookEvent
): Promise<ToolPkg.PromptHookObjectResult | null> {
  try {
    const payload = event.eventPayload;
    const stage = payload.stage === undefined ? event.eventName : payload.stage;
    if (stage !== "before_send_to_model") {
      return null;
    }
    return await buildPromptFinalizeResult(payload);
  } catch (error) {
    logPlanModeDebug("onPromptEstimateFinalize.error", {
      message: error instanceof Error ? error.message : "error",
    });
    return null;
  }
}

registerPlanModeIpc();

/** Registers the plan-mode hooks and XML renderers with ToolPkg. */
export function registerToolPkg(): boolean {
  ToolPkg.registerCoreCommand({
    id: PLAN_MODE_COMMAND_ID,
    name: PLAN_MODE_COMMAND_NAME,
    title: { zh: "计划模式", en: "Plan Mode" },
    description: { zh: "查询或切换当前聊天的计划模式。", en: "Show or toggle plan mode for the active chat." },
    usage: "/plan [message]",
    function: onPlanModeCommand,
  });
  ToolPkg.registerInputMenuTogglePlugin({
    id: MENU_HOOK_ID,
    function: onInputMenuToggle,
  });
  ToolPkg.registerChatViewHook({
    id: CHAT_VIEW_HOOK_ID,
    function: onChatViewEvent,
  });
  ToolPkg.registerSystemPromptComposeHook({
    id: PROMPT_HOOK_ID,
    function: onSystemPromptCompose,
  });
  ToolPkg.registerToolPromptComposeHook({
    id: TOOL_PROMPT_HOOK_ID,
    function: onToolPromptCompose,
  });
  ToolPkg.registerPromptFinalizeHook({
    id: PROMPT_FINALIZE_HOOK_ID,
    function: onPromptFinalize,
  });
  ToolPkg.registerPromptEstimateFinalizeHook({
    id: PROMPT_ESTIMATE_FINALIZE_HOOK_ID,
    function: onPromptEstimateFinalize,
  });
  ToolPkg.registerXmlRenderPlugin({
    id: XML_RENDER_HOOK_ID,
    tag: XML_TAG,
    function: onPlantodoXmlRender,
  });
  ToolPkg.registerXmlRenderPlugin({
    id: PLANASK_XML_RENDER_HOOK_ID,
    tag: PLANASK_XML_TAG,
    function: onPlanaskXmlRender,
  });
  return true;
}
