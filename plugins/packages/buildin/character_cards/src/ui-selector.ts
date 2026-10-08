import type { ActivePrompt } from "./model";
import type { PresentationCancel, PresentationComplete, SelectScreen } from "./presentation";
import { createUiScreenSession, chatIdentity, type UiPresentation, type UiScreenSession, callMainDomain, encodeSelection } from "./ui-contributions";
import { decodeChatMarker } from "./chat-markers";
import { requireChatSelection } from "./chat-bindings";
import type { ToolPkg as ToolPkgTypes } from "../../../../types/toolpkg";
import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";

/** Distinguishes an explicitly global choice from a real existing chat, never a missing target. */
export interface SelectorInput extends SelectScreen { readonly chatId: string | null }
/** Projects a persisted record into one native row without changing its identity, description or avatar. */
export interface SelectorOption {
  key: string; kind: "card" | "group"; id: string; title: string; description: string;
  selection: ActivePrompt; token: string; avatar: { type: "uri"; uri: string } | { type: "resource"; path: string } | { type: "group" };
}
/** Returns real list records and the freshly read authoritative target selection, not the caller's stale check mark. */
export interface SelectorData { options: SelectorOption[]; selected: string | null }
/** Tracks loading and exactly one in-flight switch while retaining visible failures on the same modal. */
export interface SelectorState { data: SelectorData | null; loading: boolean; switchingKey: string | null; error: string; finished: boolean }
/** Owns native row actions and their genuine generic presentation result. */
export interface SelectorController {
  /** Reads the current local state for rendering without substituting unloaded records. */
  current(): SelectorState;
  /** Loads actual records once for this request, propagating an original source failure. */
  load(): Promise<void>;
  /** Commits one existing record through the shared session before completing this presentation. */
  select(key: string): Promise<PresentationComplete>;
  /** Cancels the actual presentation without touching global active state or a chat extension. */
  cancel(): PresentationCancel;
}

/** Requires a real selector request and an explicitly nullable chat target; management cannot enter this route. */
export function selectorSession(presentation: UiPresentation | null): { input: SelectorInput; session: UiScreenSession } {
  if (presentation === null || presentation.input.mode !== "select") throw new Error("The selection route requires an explicit select presentation");
  const { chatId } = presentation.input;
  if (chatId !== null) chatIdentity(chatId, "selector input.chatId");
  const session = createUiScreenSession(presentation), current = session.currentScreen();
  if (current.requestId === null || current.input.mode !== "select") throw new Error("The selection route requires a real presentation request");
  return { input: { ...current.input, chatId }, session };
}

/** Reads the genuine main-runtime records and the explicit global or per-chat target without creating another repository. */
export async function readSelectorData(input: SelectorInput): Promise<SelectorData> {
  const [cards, groups] = await Promise.all([callMainDomain("character.list", {}), callMainDomain("group.list", {})]);
  let selected: string | null;
  if (input.chatId === null) selected = encodeSelection(await callMainDomain("activePrompt.get", {}));
  else {
    const extension = await Tools.Chat.readExtension({ kind: "chat", chatId: input.chatId });
    selected = extension === null ? null : decodeChatMarker(extension).selection;
  }
  if (selected !== null) requireChatSelection(selected, cards, groups);
  const options: SelectorOption[] = [];
  let defaultAvatar: string | null = null;
  if (input.kind === "card" || input.kind === "all") {
    if (cards.some(
      /** Requests the declared static avatar only for genuinely avatar-free cards, not after a failed image read. */
      card => card.avatarUri === null,
    )) defaultAvatar = await ToolPkg.readResource("character_default_avatar", "operit-avatar.png");
    for (const card of cards) {
      let avatar: SelectorOption["avatar"];
      if (card.avatarUri !== null) avatar = { type: "uri", uri: card.avatarUri };
      else {
        if (defaultAvatar === null || defaultAvatar.trim() === "") throw new Error("Declared character default-avatar resource did not resolve");
        avatar = { type: "resource", path: defaultAvatar };
      }
      options.push({ key: "card:" + card.id, token: "card:" + card.id, kind: "card", id: card.id, title: card.name, description: card.description,
        selection: { CharacterCard: { id: card.id } }, avatar });
    }
  }
  if (input.kind === "group" || input.kind === "all") for (const group of groups) options.push({
    key: "group:" + group.id, token: "group:" + group.id, kind: "group", id: group.id, title: group.name, description: group.description,
    selection: { CharacterGroup: { id: group.id } }, avatar: { type: "group" },
  });
  const keys = new Set<string>();
  for (const option of options) { if (keys.has(option.key)) throw new Error("Duplicate selector row identity: " + option.key); keys.add(option.key); }
  return { options, selected };
}

/** Owns one pure-DSL modal request and uses the existing single-write presentation session for every real switch. */
export function createSelectorController(input: SelectorInput, session: UiScreenSession, publish: (state: SelectorState) => void, assertOwner: () => void): SelectorController {
  let state: SelectorState = { data: null, loading: true, switchingKey: null, error: "", finished: false }, loading: Promise<void> | null = null;
  /** Publishes a detached state object before exposing it to an asynchronous callback or host rerender. */
  function update(changes: Partial<SelectorState>): void { state = { ...state, ...changes }; publish(state); }
  /** Rejects duplicate or concurrent result delivery without issuing another domain write. */
  function idle(): void {
    assertOwner();
    if (state.finished) throw new Error("This selector presentation has already finished");
    if (state.switchingKey !== null) throw new Error("A selector binding commit is already running");
  }
  return {
    /** Supplies the authoritative current local state to the renderer. */
    current(): SelectorState { return state; },
    /** Retains the first real load result or rejection; node rerenders do not initiate implicit source retries. */
    load(): Promise<void> {
      assertOwner();
      if (loading === null) loading = readSelectorData(input).then(
        /** Installs genuine persisted records only while this request remains open. */
        data => { assertOwner(); if (!state.finished) update({ data, loading: false, error: "" }); },
        /** Keeps a real source failure visible and propagates the same original error to the host. */
        failure => { update({ loading: false, error: String(failure) }); throw failure; },
      );
      return loading;
    },
    /** Matches the original popup: one row click awaits the real switch, then returns an explicit completion. */
    async select(key: string): Promise<PresentationComplete> {
      idle();
      if (state.loading || state.data === null) throw new Error("Selector records have not been loaded successfully");
      const options = state.data.options.filter(
        /** Resolves exactly one genuine allowed row rather than a caller-supplied role identity. */
        option => option.key === key,
      );
      if (options.length !== 1) throw new Error("Selector row is not one actual available choice: " + key);
      update({ switchingKey: key, error: "" });
      try {
        const complete = await session.completeScreen({ mode: "select", selection: options[0].selection });
        assertOwner(); update({ finished: true }); return complete;
      } catch (failure) { update({ error: String(failure) }); throw failure; }
      finally { update({ switchingKey: null }); }
    },
    /** Closes only this request and never commits a staged choice or modifies another selector. */
    cancel(): PresentationCancel { idle(); const cancel = session.cancelScreen(); update({ finished: true }); return cancel; },
  };
}

/** Encodes only explicit completion or cancellation as a plain generic JSON action result, without casts or a new receiver. */
export function selectorResultJson(result: PresentationComplete | PresentationCancel): ToolPkgTypes.JsonValue {
  switch (result.type) {
    case "toolpkg.presentation.complete": return { type: result.type, requestId: result.requestId, value: result.value };
    case "toolpkg.presentation.cancel": return { type: result.type, requestId: result.requestId };
  }
}

/** Renders an actual registered URI or resource image; absent group avatars have their explicit group glyph. */
export function selectorAvatar(ctx: ComposeDslContext, option: SelectorOption): ComposeNode {
  const props = { width: 28, height: 28, contentScale: "crop" as const, contentDescription: option.title, modifier: ctx.Modifier.clip({ type: "circle" }) };
  switch (option.avatar.type) {
    case "uri": return ctx.UI.Image({ ...props, uri: option.avatar.uri });
    case "resource": return ctx.UI.Image({ ...props, path: option.avatar.path });
    case "group": return ctx.UI.Box({ width: 28, height: 28, contentAlignment: "center" }, ctx.UI.Icon({ name: "Groups", size: 22, tint: "onSurfaceVariant", contentDescription: option.title }));
  }
}

/** Reproduces the compact original avatar/name/description/current-check row and disables every row while switching. */
export function selectorRow(ctx: ComposeDslContext, option: SelectorOption, state: SelectorState, controller: SelectorController): ComposeNode {
  if (state.data === null) throw new Error("A selector row requires its actual loaded catalog");
  const active = option.token === state.data.selected, switching = state.switchingKey === option.key;
  const enabled = state.switchingKey === null && !state.finished, colors = ctx.MaterialTheme.colorScheme;
  const titleColor = active ? colors.primary : enabled ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.65 });
  const descriptionColor = enabled ? colors.onSurfaceVariant : colors.onSurfaceVariant.copy({ alpha: 0.5 });
  const props = { key: option.key, fillMaxWidth: true, paddingStart: 10, paddingTop: 7, paddingEnd: 8, paddingBottom: 7, verticalAlignment: "center" as const, spacing: 8 };
  const contents = [
    selectorAvatar(ctx, option),
    ctx.UI.Column({ weight: 1, horizontalAlignment: "start" }, [
      ctx.UI.Text({ text: option.title, style: "bodySmall", maxLines: 1, overflow: "ellipsis", color: titleColor, fontWeight: active ? "700" : "600" }),
      ...(option.description === "" ? [] : [ctx.UI.Text({ text: option.description, style: "labelSmall", maxLines: 1, overflow: "ellipsis", color: descriptionColor })]),
    ]),
    ctx.UI.Box({ width: 20, height: 20, contentAlignment: "center" }, switching
      ? ctx.UI.CircularProgressIndicator({ width: 16, height: 16, strokeWidth: 2 })
      : ctx.UI.Icon({ name: active ? "Check" : "CircleOutlined", size: active ? 18 : 16, tint: active ? colors.primary : colors.onSurfaceVariant.copy({ alpha: 0.45 }), contentDescription: active ? "当前选中" : "未选中" })),
  ];
  if (!enabled) return ctx.UI.Row(props, contents);
  return ctx.UI.Row({ ...props,
    /** Returns the real commit result to the existing generic Compose action channel without a synthetic receiver. */
    onClick: async () => selectorResultJson(await controller.select(option.key)),
  }, contents);
}

/** Reproduces the original fixed-height, width-bounded popup while keeping loading, errors and close inside this modal. */
export function selectorView(ctx: ComposeDslContext, input: SelectorInput, state: SelectorState, controller: SelectorController): ComposeNode {
  const title = input.kind === "card" ? "切换角色卡" : input.kind === "group" ? "切换群组" : "切换角色卡或群组";
  const colors = ctx.MaterialTheme.colorScheme;
  const header = ctx.UI.Row({ fillMaxWidth: true, paddingStart: 16, paddingTop: 8, paddingEnd: 8, paddingBottom: 4, verticalAlignment: "center" }, [
    ctx.UI.Text({ text: title, style: "titleSmall", fontWeight: "700", weight: 1 }),
    ...(state.data === null ? [] : [ctx.UI.Text({ text: state.data.options.length + " 个", style: "labelSmall", color: colors.onSurfaceVariant }), ctx.UI.Spacer({ width: 2 })]),
    ctx.UI.IconButton({ key: "selector-close", enabled: state.switchingKey === null && !state.finished, width: 32, height: 32,
      /** Emits the explicit cancellation discriminator, not a route pop that bypasses the plugin session. */
      onClick: () => selectorResultJson(controller.cancel()),
    }, ctx.UI.Icon({ name: "Close", size: 18, contentDescription: "关闭" })),
  ]);
  let body: ComposeNode;
  if (state.loading) body = ctx.UI.Box({ weight: 1, fillMaxWidth: true, contentAlignment: "center" }, ctx.UI.CircularProgressIndicator({ width: 20, height: 20, strokeWidth: 2 }));
  else if (state.data === null) body = ctx.UI.Text({ key: "selector-load-error", text: state.error, color: colors.error, paddingHorizontal: 16, paddingVertical: 18, weight: 1 });
  else if (state.data.options.length === 0) body = ctx.UI.Text({ text: input.kind === "group" ? "暂无群组" : "暂无角色卡", style: "bodySmall", color: colors.onSurfaceVariant, paddingStart: 16, paddingTop: 18, paddingEnd: 16, paddingBottom: 20, weight: 1 });
  else {
    const rows: ComposeNode[] = [];
    for (const option of state.data.options) {
      if (rows.length !== 0) rows.push(ctx.UI.HorizontalDivider({ thickness: 1, color: colors.outlineVariant.copy({ alpha: 0.45 }) }));
      rows.push(selectorRow(ctx, option, state, controller));
    }
    body = ctx.UI.LazyColumn({ key: "selector-records", weight: 1, fillMaxWidth: true, paddingStart: 8, paddingTop: 4, paddingEnd: 8, paddingBottom: 8 }, rows);
  }
  return ctx.UI.Dialog({ key: "character-selection", containerColor: "surfaceContainerHigh", shape: { cornerRadius: 28 }, closeOnDismissRequest: false,
    properties: { usePlatformDefaultWidth: false, dismissOnBackPress: state.switchingKey === null, dismissOnClickOutside: state.switchingKey === null },
    /** Loads once through the main domain IPC; a rerender cannot initialize another store or retry a failed read. */
    onLoad: () => controller.load(),
    /** Returns explicit V1 cancellation for host dismissal rather than changing a global active selection. */
    onDismissRequest: () => selectorResultJson(controller.cancel()),
  }, ctx.UI.Column({ fillMaxWidth: true, height: 420, modifier: ctx.Modifier.widthIn({ maxWidth: 360 }).heightIn({ minHeight: 420, maxHeight: 420 }) }, [
    header, ctx.UI.HorizontalDivider({ thickness: 1 }), body,
    ...(state.error === "" || state.data === null ? [] : [ctx.UI.Text({ key: "selector-commit-error", text: state.error, color: colors.error, style: "bodySmall", paddingHorizontal: 16, paddingVertical: 8 })]),
  ]));
}

/** Renders only the real native selector route, never an editor, management mode or WebView substitute. */
export function renderSelectionScreen(ctx: ComposeDslContext): ComposeNode {
  const [presentation] = ctx.useState<UiPresentation | null>("presentation", null);
  const [state, setState] = ctx.useState<SelectorState>("native-selector-state", { data: null, loading: true, switchingKey: null, error: "", finished: false });
  const stored = ctx.useRef<SelectorController | null>("native-selector-controller", null);
  const identity = ctx.useRef("native-selector-request", JSON.stringify(presentation));
  /** Checks the live SDK request before every action as well as render, so a retained stale callback cannot mutate another presentation. */
  function assertOwner(): void {
    const [current] = ctx.useState<UiPresentation | null>("presentation", null);
    if (identity.current !== JSON.stringify(current)) throw new Error("A native selector cannot change its owning presentation request");
  }
  assertOwner();
  const parsed = selectorSession(presentation);
  if (stored.current === null) stored.current = createSelectorController(parsed.input, parsed.session, setState, assertOwner);
  return selectorView(ctx, parsed.input, state, stored.current);
}
