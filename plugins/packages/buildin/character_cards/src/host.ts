import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { Definition, Request, RequestOutput } from "./model";
import type { DomainOperation, DomainOutput } from "./api";
import { dispatch, dispatchDomain, initializeService } from "./service-runtime";
import { record } from "./domain";
import type { DomainMessage } from "./ui-contributions";
import { parseDomainMessage } from "./ui-contributions";

/** Registers independent character and memory management entries using the same generic route. */
export function register(definition: Definition, screen: (ctx: ComposeDslContext) => ComposeNode, attachmentScreen: (ctx: ComposeDslContext) => ComposeNode, sidebarScreen: (ctx: ComposeDslContext) => ComposeNode, selectionScreen: (ctx: ComposeDslContext) => ComposeNode, groupExecutionScreen: (ctx: ComposeDslContext) => ComposeNode, memoryScreen: (ctx: ComposeDslContext) => ComposeNode, inputMenuScreen: (ctx: ComposeDslContext) => ComposeNode): boolean {
  const route = `toolpkg:${definition.id}:ui:main`;
  ToolPkg.registerUiRoute({ id: "main", route, screen, runtime: "compose_dsl", keepAlive: true, title: { zh: definition.title, en: "Characters" } });
  const memoryRoute = `toolpkg:${definition.id}:ui:memory`;
  const attachmentRoute = `toolpkg:${definition.id}:ui:memory-attachment`;
  ToolPkg.registerUiRoute({ id: "memory-attachment", route: attachmentRoute, screen: attachmentScreen, runtime: "compose_dsl", keepAlive: false, title: { zh: "记忆附件", en: "Memory attachment" } });
  const sidebarRoute = `toolpkg:${definition.id}:ui:chat-sidebar`;
  ToolPkg.registerUiRoute({ id: "chat-sidebar", route: sidebarRoute, screen: sidebarScreen, runtime: "compose_dsl", keepAlive: true, title: { zh: "会话侧边栏", en: "Chat sidebar" } });
  const inputMenuRoute = `toolpkg:${definition.id}:ui:chat-input-menu`;
  ToolPkg.registerUiRoute({ id: "chat-input-menu", route: inputMenuRoute, screen: inputMenuScreen, runtime: "compose_dsl", keepAlive: false, title: { zh: "当前角色卡", en: "Current character" } });
  ToolPkg.registerNavigationEntry({ id: "current-character", route: inputMenuRoute, surface: "chat_input_menu", title: { zh: "当前角色卡", en: "Current character" }, order: definition.order, params: {} });
  const selectionRoute = `toolpkg:${definition.id}:ui:selection`;
  ToolPkg.registerUiRoute({ id: "selection", route: selectionRoute, screen: selectionScreen, runtime: "compose_dsl", keepAlive: false, title: { zh: "切换角色卡", en: "Switch character" } });
  ToolPkg.registerUiRoute({ id: "group-execution", route: `toolpkg:${definition.id}:ui:group-execution`, screen: groupExecutionScreen, runtime: "compose_dsl", keepAlive: false, title: { zh: "群组执行", en: "Group execution" } });
  ToolPkg.registerUiRoute({ id: "memory", route: memoryRoute, screen: memoryScreen, runtime: "compose_dsl", keepAlive: true, title: { zh: "记忆", en: "Memory" } });
  ToolPkg.registerNavigationEntry({ id: "sidebar-characters", route: sidebarRoute, surface: "chat_sidebar_tabs", title: { zh: "角色卡", en: "Characters" }, icon: "Badge", order: definition.order, params: { view: "characters" } });
  ToolPkg.registerNavigationEntry({ id: "sidebar", route, surface: "main_sidebar_plugins", title: { zh: definition.title, en: "Characters" }, icon: definition.icon, order: definition.order });
  ToolPkg.registerNavigationEntry({ id: "memory-settings", route: memoryRoute, surface: "main_sidebar_plugins", title: { zh: "记忆", en: "Memory" }, icon: "Memory", order: definition.order + 1 });
  ToolPkg.registerNavigationEntry({ id: "toolbox", route, surface: "toolbox", title: { zh: definition.title, en: "Characters" }, icon: definition.icon, order: definition.order });
  ToolPkg.registerNavigationEntry({ id: "memory-attachment", route: attachmentRoute, surface: "chat_attachments", title: { zh: "记忆附件", en: "Memory attachment" }, icon: "Memory", order: definition.order, params: { mode: "memory-attachment", ownerKey: null, folderPath: null } });
  return true;
}

/** Validates the finite UI envelope before delegating to the shared plugin service. */
export async function receiveUiRequest(value: unknown): Promise<RequestOutput> {
  const request = record(value, "character-memory.request");
  if (typeof request.action !== "string" || request.action.trim() === "") throw new Error("character-memory.request.action must be a nonblank string");
  return dispatch(request as Request);
}

/** Validates memory-tool and UI domain IPC using the service's exact operation/input contract. */
export async function receiveDomainRequest(value: unknown): Promise<DomainOutput<DomainOperation>> {
  const message = parseDomainMessage(value);
  return dispatchDomain(message.operation, message.input);
}

/** Declares the UI operation channel without initializing the business store or migration. */
export function registerUiRequestChannel(): void {
  ToolPkg.ipc.on<Request, RequestOutput>("character-memory.request", receiveUiRequest);
  ToolPkg.ipc.on<DomainMessage, DomainOutput<DomainOperation>>("character-memory.domain", receiveDomainRequest);
}

/** Declares a lifecycle callback that initializes only when the host dispatches the event. */
export function registerServiceLifecycle(): void {
  ToolPkg.registerAppLifecycleHook({ id: "service-initialize", event: "application_on_create", function: onServiceInitialize });
}

/** Shares the retained first-call initializer with the actual application lifecycle. */
export async function onServiceInitialize(_event: ToolPkg.AppLifecycleHookEvent): Promise<void> {
  await initializeService();
}
