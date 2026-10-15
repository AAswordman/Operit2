import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { ToolPkg as ToolPkgTypes } from "../../../../types/toolpkg";
import type { Card, Group, Snapshot, Request, RequestOutput, ToolAccessConfig, TagEdit, TagChanges } from "./model";
import type { ThemeChoice } from "./selection-application";
import type { UiPresentation, UiScreenSession } from "./ui-contributions";
import { createUiScreenSession } from "./ui-contributions";
import { createCharacterDraft, createGroupDraft } from "./drafts";
import { readAvatarImport, pickedImagePath } from "./image-source";
import { renderEntityPreview } from "./ui-entity-preview";

interface ExpandedField { key: string; label: string; value: string; apply: (value: string) => void }
interface NativePicker { title: string; key: string; value: string | null; choices: readonly { id: string; label: string; subtitle?: string }[]; apply: (id: string | null) => void; query: string }
interface EntityState {
  snapshot: Snapshot | null; card: Card | null; group: Group | null;
  editing: boolean; tab: number; previewConfigurationExpanded: boolean; loading: boolean; busy: boolean; error: string;
  fullscreen: boolean; deleting: boolean; finished: boolean;
  tags: TagEdit[]; tagChanges: TagChanges; tagDraft: TagEdit | null; tagDeleting: boolean; nextTag: number;
  expanded: ExpandedField | null; picker: NativePicker | null; tagManaging: boolean; toolDraft: ToolAccessConfig | null; toolTab: number; toolQuery: string; exporting: boolean; exportContent: string; exportName: string;
  avatar: string | null; themes: ThemeChoice[] | null; themeError: string;
}
/** Copies complete persisted records without introducing another storage contract. */
function copy<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
/** Returns the generic host JSON contract, not a browser completion acknowledgement. */
function resultJson(value: ReturnType<UiScreenSession["cancelScreen"]> | Awaited<ReturnType<UiScreenSession["completeScreen"]>>): ToolPkgTypes.JsonValue { return JSON.parse(JSON.stringify(value)) as ToolPkgTypes.JsonValue; }
/** Dispatches the same finite request handler used by the management editor. */
function request<R extends Request>(input: R): Promise<RequestOutput<R>> { return ToolPkg.ipc.call<R, RequestOutput<R>>("character-memory.request", input, { targetRuntime: "main" }); }

export type NativeEntityInput = { mode: "preview" | "edit"; entity: "card" | "group"; id: string } | { mode: "create"; entity: "card" | "group"; id: "" };
export interface ManagedEntityRequest { requestId: string; input: NativeEntityInput }
interface EntityLifecycle {
  assertOwner(): void;
  complete(operation?: "saved" | "deleted", id?: string): Promise<ToolPkgTypes.JsonValue>;
  cancel(): ToolPkgTypes.JsonValue;
}
/** External presentations and management share this one plugin-owned renderer. */
export function renderEntityScreen(ctx: ComposeDslContext, presentation: UiPresentation): ComposeNode {
  const owner = ctx.useRef("entity-presentation-owner", JSON.stringify(presentation));
  const session = ctx.useRef<UiScreenSession | null>("entity-presentation-session", null);
  if (session.current === null) session.current = createUiScreenSession(presentation);
  const current = session.current.currentScreen(), screenSession = session.current;
  if (current.input.mode !== "preview" && current.input.mode !== "edit") throw new Error("Native entity screen requires a preview or edit presentation");
  const input = current.input;
  return renderNativeEntity(ctx, input, {
    assertOwner() {
      const [actual] = ctx.useState<UiPresentation | null>("presentation", null);
      if (owner.current !== JSON.stringify(actual)) throw new Error("Native entity presentation cannot change its owning request");
    },
    async complete(operation) {
      return resultJson(input.mode === "preview"
        ? await screenSession.completeScreen({ mode: "preview", entity: input.entity, id: input.id })
        : await screenSession.completeScreen({ mode: "edit", entity: input.entity, id: input.id, operation: operation! }));
    },
    cancel: () => resultJson(screenSession.cancelScreen()),
  });
}
/** Local management dialogs do not fabricate a host presentation request/result. */
export function renderManagedEntityScreen(ctx: ComposeDslContext, pending: ManagedEntityRequest, finish: (result: { operation: "saved" | "deleted"; id: string } | null) => void): ComposeNode {
  const assertOwner = (): void => {
    const [active] = ctx.useState<ManagedEntityRequest | null>("managed-entity", null);
    if (active?.requestId !== pending.requestId) throw new Error("Native management entity request is no longer active");
  };
  return renderNativeEntity(ctx, pending.input, {
    assertOwner,
    async complete(operation, id) { assertOwner(); finish(operation === undefined ? null : { operation, id: id ?? pending.input.id }); return null; },
    cancel() { assertOwner(); finish(null); return null; },
  }, "managed-entity:" + pending.requestId + ":");
}
/** No HTML resource, WebView, or browser-owned character dialog. */
function renderNativeEntity(ctx: ComposeDslContext, input: NativeEntityInput, lifecycle: EntityLifecycle, statePrefix = ""): ComposeNode {
  const [state, setState] = ctx.useState<EntityState>(statePrefix + "native-entity-state", {
    snapshot: null, card: null, group: null, editing: input.mode !== "preview", tab: 0,
    previewConfigurationExpanded: false, loading: true, busy: false, error: "", fullscreen: false, deleting: false, finished: false,
    tags: [], tagChanges: { created: [], updated: [], deleted: [] }, tagDraft: null, tagDeleting: false, nextTag: 0,
    expanded: null, picker: null, tagManaging: false, toolDraft: null, toolTab: 0, toolQuery: "", exporting: false, exportContent: "", exportName: "",
    avatar: null, themes: null, themeError: "",
  });
  const live = ctx.useRef(statePrefix + "native-entity-live", state); live.current = state;
  const assertOwner = lifecycle.assertOwner;
  assertOwner();
  function update(patch: Partial<EntityState>): void { live.current = { ...live.current, ...patch }; setState(live.current); }
  /** Serializes mutations and retains failures in the same native surface. */
  async function act(operation: () => Promise<ToolPkgTypes.JsonValue | void>): Promise<ToolPkgTypes.JsonValue> {
    assertOwner(); if (live.current.busy || live.current.finished) return null;
    update({ busy: true, error: "" });
    try { const value = await operation(); if (!live.current.finished) assertOwner(); return value ?? null; }
    catch (error) { assertOwner(); update({ error: String(error) }); }
    finally { update({ busy: false }); }
    return null;
  }
  function card(): Card { if (live.current.card === null) throw new Error("角色卡尚未加载"); return live.current.card; }
  function group(): Group { if (live.current.group === null) throw new Error("群组尚未加载"); return live.current.group; }
  function changeCard(patch: Partial<Card>): void { assertOwner(); if (!live.current.busy) update({ card: { ...card(), ...patch } }); }
  function changeGroup(patch: Partial<Group>): void { assertOwner(); if (!live.current.busy) update({ group: { ...group(), ...patch } }); }
  /** Reads independent theme names on demand without blocking cancellation. */
  async function loadThemes(): Promise<void> {
    try {
      const themes = await request({ action: "listThemeChoices" });
      if (live.current.finished) return;
      assertOwner();
      const picker = live.current.picker;
      update({ themes, themeError: "", ...(picker !== null && (picker.key === "theme" || picker.key === "entity-theme-picker") ? { picker: { ...picker, choices: themes } } : {}) });
    } catch (error) {
      if (live.current.finished) return;
      try { assertOwner(); } catch { return; }
      update({ themeError: String(error) });
    }
  }
  /** Updates the avatar after entity content is usable; failures retain its placeholder. */
  async function loadDefaultAvatar(): Promise<void> {
    try {
      const avatar = await ToolPkg.readResource("character_default_avatar", "operit-avatar.png");
      if (avatar.trim() === "" || live.current.finished) return;
      assertOwner(); update({ avatar });
    } catch { /* Image lookup must not prevent entity editing or cancellation. */ }
  }
  /** Resolves exactly the requested saved entity; never substitutes the active/default role. */
  async function load(): Promise<void> {
    await act(async () => {
      const snapshot = await request({ action: "snapshot" }); assertOwner();
      const record = input.mode === "create"
        ? input.entity === "card" ? createCharacterDraft() : createGroupDraft()
        : input.entity === "card" ? snapshot.cards.find(value => value.id === input.id) : snapshot.groups.find(value => value.id === input.id);
      if (record === undefined) throw new Error("预览实体不存在：" + input.id);
      assertOwner(); update({ snapshot, tags: copy(snapshot.tags), card: input.entity === "card" ? copy(record as Card) : null, group: input.entity === "group" ? copy(record as Group) : null, loading: false });
      if (input.entity === "group" || (record as Card).avatarUri === null) void loadDefaultAvatar();
      if (live.current.editing) void loadThemes();
    });
    if (live.current.loading) update({ loading: false });
  }
  /** Native close/back cancels a nested draft first, and terminates only its actual root request. */
  async function close(): Promise<ToolPkgTypes.JsonValue> {
    assertOwner(); if (live.current.busy || live.current.finished) return null;
    if (live.current.expanded !== null) { update({ expanded: null, fullscreen: false }); return null; }
    if (live.current.picker !== null) { update({ picker: null }); return null; }
    if (live.current.toolDraft !== null) { update({ toolDraft: null }); return null; }
    if (live.current.exporting) { update({ exporting: false, exportContent: "" }); return null; }
    if (live.current.tagManaging && live.current.tagDraft === null) { update({ tagManaging: false }); return null; }
    if (live.current.tagDraft !== null) { update({ tagDraft: null, tagDeleting: false }); return null; }
    if (live.current.deleting) { update({ deleting: false }); return null; }
    if (input.mode === "preview" && live.current.editing) {
      const snapshot = live.current.snapshot;
      if (snapshot === null) throw new Error("预览数据尚未加载");
      update({ editing: false, tab: 0, error: "", tags: copy(snapshot.tags), tagChanges: { created: [], updated: [], deleted: [] }, card: input.entity === "card" ? copy(snapshot.cards.find(value => value.id === input.id) ?? card()) : null,
        group: input.entity === "group" ? copy(snapshot.groups.find(value => value.id === input.id) ?? group()) : null }); return null;
    }
    update({ finished: true });
    return input.mode === "preview" ? lifecycle.complete() : lifecycle.cancel();
  }
  async function edit(): Promise<void> { assertOwner(); update({ editing: true, tab: 0 }); if (live.current.themes === null) void loadThemes(); }
  /** Expands readonly bindings without mutating records or blocking dismissal. */
  function togglePreviewConfiguration(): void {
    assertOwner();
    const expanded = !live.current.previewConfigurationExpanded;
    update({ previewConfigurationExpanded: expanded });
    if (expanded && (live.current.card ?? live.current.group)?.themeConfigId != null && live.current.themes === null) void loadThemes();
  }
  /** Writes the complete draft through the authoritative plugin service before completing. */
  async function save(): Promise<ToolPkgTypes.JsonValue> {
    return act(async () => {
      let snapshot: Snapshot;
      if (input.entity === "card") {
        const draft = card(); if (draft.name.trim() === "") throw new Error("角色名称不能为空");
        if (draft.chatModelBindingMode === "FIXED_MODEL" && !live.current.snapshot?.models.some(model => model.modelId === draft.chatModelId)) throw new Error("请选择有效的固定聊天模型");
        if (draft.memoryBindingMode === "SHARED" && !live.current.snapshot?.stores.some(store => store.id === draft.sharedMemoryId)) throw new Error("请选择有效的共享记忆库");
        snapshot = await request({ action: "saveCharacter", card: copy(draft), create: input.mode === "create", tagChanges: copy(live.current.tagChanges) });
      } else {
        const draft = group(); if (draft.name.trim() === "") throw new Error("群组名称不能为空");
        snapshot = await request({ action: "saveGroup", group: copy(draft), create: input.mode === "create" });
      }
      const previousSnapshot = live.current.snapshot!;
      assertOwner(); update({ snapshot, editing: false, tab: 0, tags: copy(snapshot.tags), tagChanges: { created: [], updated: [], deleted: [] },
        card: input.entity === "card" ? copy(snapshot.cards.find(value => value.id === input.id) ?? card()) : null,
        group: input.entity === "group" ? copy(snapshot.groups.find(value => value.id === input.id) ?? group()) : null });
      if (input.mode !== "preview") {
        const before = previousSnapshot;
        const saved = input.mode === "create"
          ? (input.entity === "card" ? snapshot.cards.find(value => !before.cards.some(old => old.id === value.id)) : snapshot.groups.find(value => !before.groups.some(old => old.id === value.id)))
          : input.entity === "card" ? snapshot.cards.find(value => value.id === input.id) : snapshot.groups.find(value => value.id === input.id);
        if (saved === undefined) throw new Error("保存结果缺少实体标识");
        update({ finished: true }); return lifecycle.complete("saved", saved.id);
      }
      return null;
    });
  }
  /** Stages label changes with the card transaction; cancelling never writes shared tags. */
  async function stageTag(): Promise<ToolPkgTypes.JsonValue> {
    return act(async () => {
      const draft = live.current.tagDraft; if (draft === null) throw new Error("没有标签草稿");
      const changes = copy(live.current.tagChanges);
      let tags = copy(live.current.tags), attachedTagIds = [...card().attachedTagIds];
      if (live.current.tagDeleting) {
        tags = tags.filter(tag => tag.id !== draft.id); attachedTagIds = attachedTagIds.filter(id => id !== draft.id);
        if (changes.created.some(tag => tag.draftId === draft.id)) changes.created = changes.created.filter(tag => tag.draftId !== draft.id);
        else { changes.updated = changes.updated.filter(tag => tag.id !== draft.id); changes.deleted.push(draft.id); }
      } else {
        if (draft.name.trim() === "") throw new Error("标签名称不能为空");
        const id = draft.id === "" ? "draft_prompt_tag_" + (live.current.nextTag + 1) : draft.id;
        const tag = { ...draft, id, name: draft.name.trim(), description: draft.description.trim() };
        const { id: _id, ...values } = tag;
        if (draft.id === "") { tags.push(tag); attachedTagIds.push(id); changes.created.push({ draftId: id, values }); }
        else {
          tags = tags.map(value => value.id === id ? tag : value);
          const created = changes.created.find(value => value.draftId === id);
          if (created !== undefined) created.values = values;
          else { changes.updated = changes.updated.filter(value => value.id !== id); changes.updated.push(tag); }
        }
      }
      update({ tags, tagChanges: changes, card: { ...card(), attachedTagIds }, tagDraft: null, tagDeleting: false, nextTag: live.current.nextTag + 1 });
    });
  }
  async function remove(): Promise<ToolPkgTypes.JsonValue> {
    return act(async () => {
      await request(input.entity === "card" ? { action: "deleteCharacter", id: input.id } : { action: "deleteGroup", id: input.id }); assertOwner();
      update({ finished: true });
      return lifecycle.complete(input.mode === "preview" ? undefined : "deleted");
    });
  }
  /** Uses the existing host picker and file service; the selected asset remains plugin-owned. */
  async function chooseAvatar(): Promise<void> {
    await act(async () => {
      const selected = await ctx.openFilePicker({ picker: "image", allowMultiple: false, mimeTypes: ["image/*"] }); assertOwner();
      if (selected.cancelled) return;
      if (selected.files.length !== 1) throw new Error("请选择一个头像文件");
      const platform = (await Tools.System.terminal.info()).platform, contentBase64 = await readAvatarImport(pickedImagePath(selected.files[0].path, platform)); assertOwner();
      const directory = ToolPkg.getConfigDir().replace(/\/$/, "") + "/avatars";
      const created = await Tools.Files.mkdir(directory, true); if (!created.successful) throw new Error(created.details);
      const uri = directory + "/" + Date.now() + "-" + Math.random().toString(36).slice(2) + ".image";
      const written = await Tools.Files.writeBinary(uri, contentBase64);
      if (!written.successful) throw new Error(written.details); assertOwner(); update({ card: { ...card(), avatarUri: uri } });
    });
  }
  // Geometry below follows main's OperitDialogScaffold / CharacterCardEditorDialog,
  // not a host-owned character widget: header 24/22/16/12, actions 24/0/24/20,
  // body 18/10/18/10, field bottom gap 12, radius 28, 760 x 820 bounds.
  const colors = ctx.MaterialTheme.colorScheme, enabled = !state.busy && !state.finished;
  const text = (value: string): ComposeNode => ctx.UI.Text({ text: value, fillMaxWidth: true, style: "bodyMedium" });
  const caption = (value: string): ComposeNode => ctx.UI.Text({ text: value, fillMaxWidth: true, style: "bodySmall", color: colors.onSurfaceVariant });
  const section = (title: string, value: string): ComposeNode => ctx.UI.Column({ key: "preview-" + title, fillMaxWidth: true, paddingBottom: 12, spacing: 6 }, [
    ctx.UI.Text({ text: title, style: "labelLarge", color: colors.onSurfaceVariant }),
    ctx.UI.SelectionContainer({}, text(value)),
  ]);
  /** A single native field with the same fullscreen suffix and line limits as main. */
  const field = (key: string, label: string, value: string, apply: (value: string) => void, maxLines = 1): ComposeNode =>
    ctx.UI.Box({ fillMaxWidth: true, paddingBottom: 12 }, ctx.UI.TextField({ key: "entity-field-" + key, label, value, fillMaxWidth: true, enabled,
      singleLine: maxLines === 1, minLines: maxLines, maxLines, style: {}, onValueChange: apply,
      ...(maxLines === 1 ? {} : { suffixIconWidth: 36, suffixIconHeight: 36, trailingIcon: ctx.UI.Box({ width: 36, height: 36, contentAlignment: "center" }, ctx.UI.IconButton({ key: "entity-expand-" + key, width: 28, height: 28, contentPadding: { horizontal: 0, vertical: 0 }, enabled,
        onClick: () => { assertOwner(); update({ expanded: { key, label, value, apply }, fullscreen: true }); } }, ctx.UI.Icon({ name: "Fullscreen", size: 18 }))) }),
    }));
  const check = (key: string, label: string, checked: boolean, apply: (checked: boolean) => void): ComposeNode =>
    ctx.UI.Row({ key, fillMaxWidth: true, verticalAlignment: "center", spacing: 8 }, [ctx.UI.Checkbox({ checked, enabled, onCheckedChange: apply }), ctx.UI.Text({ text: label, weight: 1 })]);
  const configure = (key: string, label: string, value: string, apply: () => void): ComposeNode => ctx.UI.Box({ fillMaxWidth: true, paddingBottom: 12 },
    ctx.UI.Surface({ key, fillMaxWidth: true, color: "transparent", onClick: enabled ? apply : undefined },
      ctx.UI.InputDecorator({ label, enabled }, ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 12 }, [ctx.UI.Text({ text: value, weight: 1, maxLines: 1, overflow: "ellipsis", style: "bodyMedium" }), ctx.UI.Icon({ name: "Tune", size: 24 })]))));
  const select = (key: string, label: string, value: string | null, choices: readonly { id: string; label: string; subtitle?: string }[], apply: (value: string | null) => void, empty = "不绑定"): ComposeNode =>
    configure(key, label, choices.find(option => option.id === value)?.label ?? (value === null ? empty : "已绑定配置不存在：" + value), () => {
      assertOwner(); update({ picker: { title: "选择" + label, key, value, choices, apply, query: "" } });
    });
  const binding = (key: string, title: string, subtitleOff: string, subtitleOn: string, checked: boolean, apply: (checked: boolean) => void, children: ComposeNode[], switchEnabled = true): ComposeNode =>
    ctx.UI.Box({ key, fillMaxWidth: true, paddingBottom: 7 }, ctx.UI.Box({ fillMaxWidth: true, modifier: ctx.Modifier.border(1, colors.outlineVariant.copy({ alpha: 0.24 }), { cornerRadius: 12 }), paddingHorizontal: 10, paddingVertical: 6 },
      ctx.UI.Column({ fillMaxWidth: true }, [
        ctx.UI.Row({ fillMaxWidth: true, modifier: ctx.Modifier.heightIn({ minHeight: 46 }), verticalAlignment: "center", spacing: 8 }, [
          ctx.UI.Column({ weight: 1, spacing: 1 }, [ctx.UI.Text({ text: title, fontSize: 14, fontWeight: "700", color: enabled && switchEnabled ? colors.onSurface : colors.onSurface.copy({ alpha: 0.46 }) }), ctx.UI.Text({ text: checked ? subtitleOn : subtitleOff, fontSize: 12, color: enabled && switchEnabled ? colors.onSurfaceVariant : colors.onSurfaceVariant.copy({ alpha: 0.46 }) })]),
          ctx.UI.Switch({ key: key + "-switch", checked, enabled: enabled && switchEnabled, modifier: ctx.Modifier.scale(0.82), onCheckedChange: apply }),
        ]),
        ...(checked || !switchEnabled ? [ctx.UI.Spacer({ height: 6 }), ...children] : []),
      ])));
  // Keep form padding inside the scroll clip, as main's tab body does.
  // An outlined field's floating label paints above its own layout bounds.
  const body = (key: string, children: ComposeNode[], padding = { paddingStart: 18, paddingTop: 10, paddingEnd: 18, paddingBottom: 10 }): ComposeNode =>
    ctx.UI.LazyColumn({ key, fillMaxWidth: true }, children.map((child, index) =>
      ctx.UI.Box({ key: key + "-item-" + String(child.props?.key ?? index), fillMaxWidth: true,
        paddingStart: padding.paddingStart, paddingEnd: padding.paddingEnd,
        paddingTop: index === 0 ? padding.paddingTop : 0,
        paddingBottom: index === children.length - 1 ? padding.paddingBottom : 0,
      }, child)));
  const tabs = (key: string, labels: string[], selected: number, apply: (tab: number) => void): ComposeNode => ctx.UI.PrimaryTabRow({ key, fillMaxWidth: true, selectedTabIndex: selected,
    tabs: labels.map((label, tab) => ctx.UI.Tab({ key: key + "-" + tab, selected: selected === tab, enabled, onClick: () => { assertOwner(); apply(tab); } }, ctx.UI.Text({ text: label, style: "titleSmall" }))),
  });
  const standardButton = (key: string, label: string, action: () => Promise<ToolPkgTypes.JsonValue> | Promise<void> | ToolPkgTypes.JsonValue | void, filled = false): ComposeNode =>
    filled ? ctx.UI.Button({ key, text: label, enabled, onClick: action }) : ctx.UI.TextButton({ key, text: label, enabled, onClick: action });
  /** Same shared shell geometry as main, assembled exclusively from generic DSL controls. */
  function shell(title: string, child: ComposeNode, actions: ComposeNode[], width = 760, height = 820, plainBody = false, reading = false): ComposeNode {
    return ctx.UI.Column({ key: "native-entity-panel", ...(state.fullscreen ? { fillMaxSize: true } : { width, height }), modifier: ctx.Modifier.clip({ cornerRadius: state.fullscreen ? 0 : 28 }) }, [
      ctx.UI.Row({ key: "entity-header", fillMaxWidth: true, paddingStart: 24, paddingTop: reading ? 18 : 22, paddingEnd: 16, paddingBottom: reading ? 8 : 12, verticalAlignment: "center" }, [
        ctx.UI.Text({ text: title, style: reading ? "labelLarge" : "headlineSmall", fontWeight: reading ? "600" : "700", ...(reading ? { color: colors.onSurfaceVariant, fontSize: 14 } : {}), maxLines: 2, overflow: "ellipsis", weight: 1 }),
        ctx.UI.IconButton({ key: "entity-close", icon: "Close", enabled, onClick: close }),
      ]),
      ...(state.busy ? [ctx.UI.LinearProgressIndicator({ fillMaxWidth: true })] : []),
      ...(state.error === "" ? [] : [ctx.UI.Text({ key: "entity-error", text: state.error, color: colors.error, fillMaxWidth: true, paddingHorizontal: 24, paddingBottom: 12 })]),
      ctx.UI.Box({ key: "entity-content", weight: 1, fillMaxWidth: true, ...(plainBody ? {} : { paddingStart: 24, paddingTop: 16, paddingEnd: 24, paddingBottom: 20 }) }, child),
      ctx.UI.FlowRow({ key: "entity-footer", fillMaxWidth: true, paddingStart: 24, paddingTop: 0, paddingEnd: 24, paddingBottom: 20, horizontalArrangement: "end", spacing: 8, runSpacing: 8 }, actions),
    ]);
  }
  let panel: ComposeNode;
  const cancel = standardButton("entity-cancel", "取消", close);
  if (state.expanded !== null) {
    const expanded = state.expanded;
    panel = shell(expanded.label, ctx.UI.TextField({ key: "entity-expanded-input", label: expanded.label, value: expanded.value, fillMaxSize: true, minLines: 16, maxLines: 40, enabled, onValueChange: value => { assertOwner(); if (live.current.expanded !== null) update({ expanded: { ...live.current.expanded, value } }); } }), [cancel,
      standardButton("entity-primary", "保存", () => { assertOwner(); const draft = live.current.expanded; if (draft !== null) { draft.apply(draft.value); update({ expanded: null, fullscreen: false }); } }, true),
    ], 760, 620);
  } else if (state.picker !== null) {
    const picker = state.picker;
    const themePending = (picker.key === "theme" || picker.key === "entity-theme-picker") && state.themes === null;
    const choose = (id: string | null): void => { assertOwner(); picker.apply(id); update({ picker: null }); };
    const choices = picker.choices.filter(option => (option.label + " " + (option.subtitle ?? "")).toLowerCase().includes(picker.query.trim().toLowerCase()));
    panel = shell(picker.title, ctx.UI.Column({ fillMaxSize: true }, [
      field("picker-query", "搜索", picker.query, query => { assertOwner(); if (live.current.picker !== null) update({ picker: { ...live.current.picker, query } }); }),
      ...(themePending ? [state.themeError === "" ? caption("正在读取主题配置…") : ctx.UI.Column({ fillMaxWidth: true }, [ctx.UI.Text({ text: state.themeError, color: colors.error }), standardButton("entity-picker-retry", "重新加载主题", loadThemes)])] : []),
      ctx.UI.LazyColumn({ key: "entity-picker-list", weight: 1, fillMaxWidth: true }, [
        ctx.UI.Row({ key: "entity-picker-clear", fillMaxWidth: true, paddingHorizontal: 8, paddingVertical: 9, verticalAlignment: "center", spacing: 10, onClick: () => choose(null) }, [ctx.UI.Icon({ name: picker.value === null ? "CheckCircle" : "CircleOutlined", size: 20, tint: picker.value === null ? colors.primary : colors.onSurfaceVariant }), text("跟随全局 / 不绑定")]),
        ...choices.map(option => ctx.UI.Row({ key: "entity-picker-" + option.id, fillMaxWidth: true, paddingHorizontal: 8, paddingVertical: 9, verticalAlignment: "center", spacing: 10, onClick: () => choose(option.id) }, [ctx.UI.Icon({ name: picker.value === option.id ? "CheckCircle" : "CircleOutlined", size: 20, tint: picker.value === option.id ? colors.primary : colors.onSurfaceVariant }), ctx.UI.Column({ weight: 1 }, [text(option.label), ...(option.subtitle === undefined ? [] : [caption(option.subtitle)])])])),
      ]),
    ]), [cancel], 580, 560);
  } else if (state.toolDraft !== null && state.snapshot !== null) {
    const draft = state.toolDraft, catalog = state.snapshot.toolCatalog;
    const names = ["内置工具", "插件", "技能", "MCP"], keys = ["allowedBuiltinTools", "allowedPackages", "allowedSkills", "allowedMcpServers"] as const;
    const key = keys[state.toolTab], options = [catalog.builtinTools, catalog.packages, catalog.skills, catalog.mcpServers][state.toolTab];
    const children = options.filter(option => (option.displayName + " " + option.description).toLowerCase().includes(state.toolQuery.toLowerCase())).map(option => check("entity-tool-" + key + "-" + option.name, option.displayName, draft[key].includes(option.name), checked => {
      assertOwner(); const policy = live.current.toolDraft; if (policy !== null) update({ toolDraft: { ...policy, [key]: checked ? [...policy[key], option.name] : policy[key].filter(name => name !== option.name) } });
    }));
    panel = shell("配置允许使用的工具", ctx.UI.Column({ fillMaxSize: true }, [tabs("entity-tool-tabs", names, state.toolTab, toolTab => update({ toolTab })),
      ctx.UI.Box({ fillMaxWidth: true, paddingHorizontal: 18, paddingTop: 10 }, field("tool-query", "搜索", state.toolQuery, toolQuery => update({ toolQuery }))),
      ctx.UI.Box({ weight: 1, fillMaxWidth: true }, body("entity-tool-list", children)),
    ]), [cancel, standardButton("entity-primary", "确定", () => { assertOwner(); if (live.current.toolDraft !== null) changeCard({ toolAccessConfig: copy(live.current.toolDraft) }); update({ toolDraft: null }); }, true)], 760, 620, true);
  } else if (state.exporting) {
    /** Serializes saved entity data and suggests a portable export file name. */
    const exportFile = async (format: "operit" | "tavern"): Promise<void> => { await act(async () => {
      const content = input.entity === "card" ? await request({ action: "exportCharacter", id: input.id, format }) : await request({ action: "exportGroup", id: input.id });
      assertOwner();
      const name = input.entity === "card" ? card().name : group().name;
      update({ exportContent: content, exportName: name.replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_") + "-" + format + ".json" });
    }); };
    /** Opens the host save selector and retains the export preview on cancellation or error. */
    const writeExport = async (): Promise<void> => { await act(async () => {
      const result = await ctx.saveFile({ suggestedName: live.current.exportName, content: live.current.exportContent, mimeType: "application/json", extensions: ["json"] });
      assertOwner();
      if (result.cancelled) return;
      update({ exporting: false, exportContent: "", exportName: "" });
    }); };
    panel = shell("导出", state.exportContent === "" ? ctx.UI.Column({ fillMaxWidth: true, spacing: 8 }, [standardButton("entity-export-json", "导出 JSON", () => exportFile("operit")), ...(input.entity === "card" ? [standardButton("entity-export-tavern", "导出酒馆 JSON", () => exportFile("tavern"))] : [])]) : body("entity-export-body", [caption("点击保存文件选择保存位置。"), section("JSON", state.exportContent)]), [cancel, ...(state.exportContent === "" ? [] : [standardButton("entity-export-save", "保存文件", writeExport, true)])], state.exportContent === "" ? 420 : 760, state.exportContent === "" ? 240 : 620);
  } else if (state.tagDraft !== null) {
    const draft = state.tagDraft;
    const children = state.tagDeleting ? [text(`确定删除标签“${draft.name}”？`)] : ([ ["name", "标签名称", 1], ["description", "描述", 1], ["promptContent", "提示词内容", 6] ] as const).map(([key, label, lines]) => field("tag-" + key, label, draft[key], value => { assertOwner(); if (live.current.tagDraft !== null) update({ tagDraft: { ...live.current.tagDraft, [key]: value } }); }, lines));
    panel = shell(state.tagDeleting ? "删除标签" : draft.id === "" ? "创建标签" : "编辑标签", body("entity-tag-body", children, { paddingStart: 0, paddingTop: 0, paddingEnd: 0, paddingBottom: 0 }), [
      ...(draft.id === "" || state.tagDeleting ? [] : [standardButton("entity-tag-delete", "删除", () => { assertOwner(); update({ tagDeleting: true }); })]), cancel, standardButton("entity-primary", state.tagDeleting ? "删除" : "保存", stageTag, true),
    ], 620, state.tagDeleting ? 240 : 520);
  } else if (state.tagManaging) {
    panel = shell("管理标签", body("entity-tag-manager", state.tags.map(tag => ctx.UI.Row({ key: "entity-tag-row-" + tag.id, fillMaxWidth: true, verticalAlignment: "center" }, [
      ctx.UI.Box({ weight: 1 }, check("entity-tag-" + tag.id, tag.name, state.card?.attachedTagIds.includes(tag.id) ?? false, checked => changeCard({ attachedTagIds: checked ? [...card().attachedTagIds, tag.id] : card().attachedTagIds.filter(id => id !== tag.id) }))),
      standardButton("entity-tag-edit-" + tag.id, "编辑", () => { assertOwner(); update({ tagDraft: copy(tag) }); }),
      standardButton("entity-tag-remove-" + tag.id, "删除", () => { assertOwner(); update({ tagDraft: copy(tag), tagDeleting: true }); }),
    ])), { paddingStart: 0, paddingTop: 0, paddingEnd: 0, paddingBottom: 0 }), [standardButton("entity-tag-create", "创建标签", () => { assertOwner(); update({ tagDraft: { id: "", name: "", description: "", promptContent: "", tagType: "CUSTOM" } }); }), standardButton("entity-cancel", "完成", close)], 620, 560);
  } else if (state.deleting) {
    panel = shell("确认删除", text(`确定删除“${state.card?.name ?? state.group?.name}”？此操作不可撤销。`), [cancel, standardButton("entity-primary", "删除", remove, true)], 560, 240);
  } else {
    const pages: ComposeNode[][] = [[], [], []];
    if (state.loading) pages[0].push(ctx.UI.Box({ fillMaxSize: true, contentAlignment: "center" }, ctx.UI.LoadingIndicator({ size: 32 })));
    else if (state.snapshot === null) pages[0].push(standardButton("entity-retry", "重新加载", load));
    else if (state.card !== null) {
      const draft = state.card, snapshot = state.snapshot;
      if (!state.editing) {
        pages[0].push(...renderEntityPreview(ctx, { card: draft, group: null, snapshot, tags: state.tags, avatar: state.avatar, themes: state.themes,
          themeError: state.themeError, configurationExpanded: state.previewConfigurationExpanded, onToggleConfiguration: togglePreviewConfiguration }));
      } else {
        pages[0].push(field("name", "角色名称", draft.name, name => changeCard({ name })), field("description", "描述", draft.description, description => changeCard({ description })),
          ctx.UI.Box({ fillMaxWidth: true, paddingBottom: 12 }, ctx.UI.InputDecorator({ label: "角色头像", enabled }, ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center" }, [
            ctx.UI.Image({ key: "entity-edit-avatar", ...(draft.avatarUri === null ? { path: state.avatar ?? undefined } : { uri: draft.avatarUri }), width: 44, height: 44, contentScale: "crop", contentDescription: "角色头像", modifier: ctx.Modifier.clip({ type: "circle" }) }), ctx.UI.Spacer({ width: 12 }), ctx.UI.Text({ text: draft.avatarUri ?? "未设置", style: "bodyMedium", weight: 1, maxLines: 1, overflow: "ellipsis" }), ctx.UI.Spacer({ width: 8 }), standardButton("entity-choose-avatar", "选择", chooseAvatar),
            ...(draft.avatarUri === null ? [] : [standardButton("entity-clear-avatar", "清除", async () => { await act(async () => { assertOwner(); update({ card: { ...card(), avatarUri: null } }); if (live.current.avatar === null) void loadDefaultAvatar(); }); })]),
          ]))),
          field("characterSetting", "角色设定", draft.characterSetting, characterSetting => changeCard({ characterSetting }), 6), field("openingStatement", "开场白", draft.openingStatement, openingStatement => changeCard({ openingStatement }), 3),
          ctx.UI.Column({ fillMaxWidth: true, paddingBottom: 12 }, [ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center" }, [ctx.UI.Text({ text: "标签", weight: 1, fontWeight: "700" }), ctx.UI.TextButton({ key: "entity-tags-manage", enabled, onClick: () => { assertOwner(); update({ tagManaging: true }); } }, ctx.UI.Row({ spacing: 8, verticalAlignment: "center" }, [ctx.UI.Icon({ name: "Tune", size: 18 }), ctx.UI.Text({ text: "管理标签" })]))]), ctx.UI.Spacer({ height: 6 }),
            state.tags.length === 0 ? caption("暂无提示词标签") : ctx.UI.FlowRow({ spacing: 8, runSpacing: 8 }, state.tags.map(tag => ctx.UI.FilterChip({ key: "entity-tag-" + tag.id, selected: draft.attachedTagIds.includes(tag.id), enabled, label: ctx.UI.Text({ text: tag.name, maxLines: 1, overflow: "ellipsis", modifier: ctx.Modifier.widthIn({ maxWidth: 160 }) }), onClick: () => { assertOwner(); changeCard({ attachedTagIds: card().attachedTagIds.includes(tag.id) ? card().attachedTagIds.filter(id => id !== tag.id) : [...card().attachedTagIds, tag.id] }); } }))),
          ]),
        );
        for (const [key, label, lines] of [["otherContentChat", "聊天附加内容", 4], ["otherContentVoice", "语音附加内容", 4], ["advancedCustomPrompt", "高级自定义 Prompt", 4], ["marks", "备注", 3]] as const) pages[1].push(field(key, label, draft[key], value => changeCard({ [key]: value }), lines));
        const tts = snapshot.ttsConfigs.find(config => config.id === draft.ttsConfigId);
        const policy = draft.toolAccessConfig, count = policy.allowedBuiltinTools.length + policy.allowedPackages.length + policy.allowedSkills.length + policy.allowedMcpServers.length;
        pages[2].push(
          binding("entity-model", "聊天模型", "跟随全局模型配置", "使用固定模型配置", draft.chatModelBindingMode === "FIXED_MODEL", checked => changeCard({ chatModelBindingMode: checked ? "FIXED_MODEL" : "FOLLOW_GLOBAL", chatModelId: checked ? card().chatModelId : null }), [select("entity-model-picker", "聊天模型配置", draft.chatModelId, snapshot.models.map(value => ({ id: value.modelId, label: value.modelId, subtitle: value.providerName })), chatModelId => changeCard({ chatModelId }), "请选择聊天模型")]),
          binding("entity-theme", "主题配置", "不改变当前独立主题", "使用角色卡主题配置", draft.themeConfigId !== null, checked => { if (!checked) changeCard({ themeConfigId: null }); else update({ picker: { title: "选择主题配置", key: "theme", value: draft.themeConfigId, choices: state.themes ?? [], apply: themeConfigId => changeCard({ themeConfigId }), query: "" } }); }, [select("entity-theme-picker", "主题配置", draft.themeConfigId, state.themes ?? [], themeConfigId => changeCard({ themeConfigId }))]),
          binding("entity-tts", "TTS 配置", "跟随全局 TTS 配置", "使用角色卡 TTS 配置", draft.ttsConfigId !== null, checked => { if (!checked) changeCard({ ttsConfigId: null }); else update({ picker: { title: "选择 TTS 配置", key: "tts", value: draft.ttsConfigId, choices: snapshot.ttsConfigs.map(config => ({ id: config.id, label: config.name, subtitle: config.providerType + " · " + config.voice })), apply: ttsConfigId => changeCard({ ttsConfigId }), query: "" } }); }, [select("entity-tts-picker", "TTS 配置", draft.ttsConfigId, snapshot.ttsConfigs.map(config => ({ id: config.id, label: config.name, subtitle: config.providerType + " · " + config.voice })), ttsConfigId => changeCard({ ttsConfigId }), tts?.name ?? "请选择 TTS 配置")]),
          binding("entity-memory", "记忆绑定", "使用角色记忆", "使用共享记忆", draft.memoryBindingMode === "SHARED", checked => changeCard({ memoryBindingMode: checked ? "SHARED" : "CHARACTER", sharedMemoryId: checked ? card().sharedMemoryId : null }), [snapshot.stores.length === 0 ? caption("还没有共享记忆库") : select("entity-store-picker", "共享记忆库", draft.sharedMemoryId, snapshot.stores.map(store => ({ id: store.id, label: store.name })), sharedMemoryId => changeCard({ sharedMemoryId }), "请选择共享记忆库")], snapshot.stores.length !== 0),
          binding("entity-tool", "工具访问", "跟随全局工具配置", "自定义允许使用的工具", policy.enabled, checked => changeCard({ toolAccessConfig: { ...card().toolAccessConfig, enabled: checked } }), [configure("entity-tool-configure", "配置允许使用的工具", "已配置 " + count + " 个工具来源", () => { assertOwner(); update({ toolDraft: copy(card().toolAccessConfig), toolTab: 0, toolQuery: "" }); })]),
        );
        // References introduced by the plugin stay visible, without replacing main's sections.
        if (draft.sharedMemoryMounts.length !== 0) pages[2].push(section("共享记忆挂载", JSON.stringify(draft.sharedMemoryMounts, null, 2)));
        if (state.themeError !== "") pages[2].push(ctx.UI.Text({ text: state.themeError, color: colors.error }), standardButton("entity-theme-retry", "重新加载主题", async () => { await act(loadThemes); }));
      }
    } else if (state.group !== null && state.snapshot !== null) {
      const draft = state.group, snapshot = state.snapshot;
      if (!state.editing) pages[0].push(...renderEntityPreview(ctx, { card: null, group: draft, snapshot, tags: state.tags, avatar: state.avatar, themes: state.themes,
        themeError: state.themeError, configurationExpanded: state.previewConfigurationExpanded, onToggleConfiguration: togglePreviewConfiguration }));
      else {
        pages[0].push(field("name", "群组名称", draft.name, name => changeGroup({ name })), field("description", "描述", draft.description, description => changeGroup({ description })), select("entity-theme-picker", "主题配置", draft.themeConfigId, state.themes ?? [], themeConfigId => changeGroup({ themeConfigId })), ctx.UI.Text({ text: "组内角色", style: "titleSmall", fontWeight: "700", paddingTop: 8, paddingBottom: 8 }));
        for (const option of snapshot.cards) {
          const selected = draft.members.some(member => member.characterCardId === option.id);
          pages[0].push(ctx.UI.Box({ fillMaxWidth: true, paddingBottom: 6 }, ctx.UI.Surface({ key: "entity-member-" + option.id, fillMaxWidth: true, color: selected ? colors.primaryContainer.copy({ alpha: 0.16 }) : colors.surfaceContainerHighest.copy({ alpha: 0.28 }), shape: { cornerRadius: 10 }, modifier: ctx.Modifier.border(1, selected ? colors.primary.copy({ alpha: 0.45 }) : colors.outlineVariant.copy({ alpha: 0.28 }), { cornerRadius: 10 }),
            onClick: () => { assertOwner(); const members = selected ? group().members.filter(member => member.characterCardId !== option.id) : [...group().members, { characterCardId: option.id, orderIndex: group().members.length }]; changeGroup({ members: members.map((member, orderIndex) => ({ ...member, orderIndex })) }); },
          }, ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 12, paddingVertical: 8, spacing: 10, verticalAlignment: "center" }, [ctx.UI.Icon({ name: selected ? "CheckCircle" : "CircleOutlined", size: 18, tint: selected ? colors.primary : colors.onSurfaceVariant }), ctx.UI.Image({ width: 28, height: 28, ...(option.avatarUri === null ? { path: state.avatar ?? undefined } : { uri: option.avatarUri }), modifier: ctx.Modifier.clip({ type: "circle" }), contentScale: "crop", contentDescription: option.name }), ctx.UI.Column({ weight: 1 }, [ctx.UI.Text({ text: option.name, style: "bodyMedium", fontWeight: "700", maxLines: 1, overflow: "ellipsis" }), ...(option.description.trim() === "" ? [] : [caption(option.description)])])]))));
        }
      }
    }
    const isCard = state.card !== null;
    const reading = !state.editing;
    const page = reading ? body("entity-preview-body", pages[0], { paddingStart: 24, paddingTop: 8, paddingEnd: 24, paddingBottom: 24 }) : isCard ? ctx.UI.Column({ fillMaxSize: true }, [tabs("entity-tab", ["基础", "内容", "绑定"], state.tab, tab => update({ tab })), ctx.UI.HorizontalPager({ key: "entity-pager", weight: 1, fillMaxWidth: true, currentPage: state.tab, onPageChanged: tab => { assertOwner(); if (tab >= 0 && tab < 3) update({ tab }); } }, pages.map((children, index) => body("entity-body-" + index, children)))]) : body("entity-body", pages[0], { paddingStart: 0, paddingTop: 0, paddingEnd: 0, paddingBottom: 0 });
    const title = state.editing ? input.mode === "create" ? input.entity === "card" ? "创建角色卡" : "创建群组" : input.entity === "card" ? "编辑角色卡" : "编辑群组" : input.entity === "card" ? "角色卡" : "角色群组";
    panel = shell(title, page, [
      ...(state.editing && input.mode !== "create" ? [
        ...(state.card?.isDefault ? [] : [standardButton("entity-delete", "删除", () => { assertOwner(); update({ deleting: true }); })]),
        standardButton("entity-export", "导出", () => { assertOwner(); update({ exporting: true }); }),
      ] : []),
      standardButton("entity-cancel", state.editing ? "取消" : "关闭", close),
      ...(state.snapshot === null || state.finished ? [] : [standardButton("entity-primary", state.editing ? "保存" : "编辑", state.editing ? save : edit, true)]),
    ], reading ? isCard ? 680 : 620 : isCard ? 760 : 620, reading ? 760 : 820, reading || isCard, reading);
  }
  return ctx.UI.Dialog({ key: "character-presentation", onLoad: load, closeOnDismissRequest: false, onDismissRequest: close,
    containerColor: colors.surfaceContainerHigh, shape: { cornerRadius: state.fullscreen ? 0 : 28 },
    properties: { fullscreen: state.fullscreen, usePlatformDefaultWidth: false, dismissOnClickOutside: false },
  }, panel);
}
