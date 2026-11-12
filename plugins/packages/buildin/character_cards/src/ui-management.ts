import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { BindableChat, Card, Snapshot, StoreValues } from "./model";
import { renderManagedEntityScreen, type ManagedEntityRequest, type NativeEntityInput } from "./ui-entity-native";
import { initializationRecoveryUi } from "./ui-initialization-recovery";
import { nativeUi, uiRequest, uiState } from "./ui-controls";
import { renderMemoryWorkspace, type MemoryTarget } from "./ui-memory-workspace";

interface BindingDraft { cardId: string; query: string; unbound: boolean; chats: BindableChat[]; selected: string[]; failures: Record<string, string>; completed: number; total: number }
type ManagementPanel = { type: "store"; store: StoreValues; create: boolean }
  | { type: "delete-store"; id: string; name: string }
  | { type: "import"; entity: "card" | "group"; format: "operit" | "tavern"; content: string }
  | { type: "binding"; draft: BindingDraft };
interface ManagementState { snapshot: Snapshot | null; busy: boolean; error: string; notice: string; panel: ManagementPanel | null; memory: MemoryTarget | null; navigation: number; defaultAvatar: string | null }

/** Resolves the memory owner selected in the persisted character configuration. */
export function cardMemoryTarget(card: Card): MemoryTarget {
  if (card.memoryBindingMode === "SHARED") {
    if (card.sharedMemoryId === null) throw new Error("请先为角色绑定共享记忆库");
    return { ownerKey: "shared:" + card.sharedMemoryId, name: card.name };
  }
  return { ownerKey: "character:" + card.id, name: card.name };
}

/** Names a persisted chat binding without substituting the current global role. */
export function chatBindingLabel(selection: string | null, snapshot: Snapshot): string {
  if (selection === null) return "未绑定角色";
  if (selection === "model:chat") return "仅使用模型";
  for (const card of snapshot.cards) if (selection === "card:" + card.id) return card.name;
  for (const group of snapshot.groups) if (selection === "group:" + group.id) return "群组 · " + group.name;
  return "原角色已失效";
}

/** Owns both management routes entirely in DSL, reusing the existing entity editor. */
export function renderManagement(ctx: ComposeDslContext, view: "characters" | "memory"): ComposeNode {
  const { state, live, update } = uiState<ManagementState>(ctx, "management:" + view, { snapshot: null, busy: false, error: "", notice: "", panel: null, memory: null, navigation: 0, defaultAvatar: null });
  const [managed, setManaged] = ctx.useState<ManagedEntityRequest | null>("managed-entity", null);
  const sequence = ctx.useRef("management-entity-sequence", 0);
  const recovery = initializationRecoveryUi(ctx, () => { void load(); });
  const ui = nativeUi(ctx, !state.busy);
  /** Serializes foreground operations and keeps failures visible beside their draft. */
  async function act(operation: () => Promise<void>): Promise<void> {
    if (live.current.busy) return;
    update({ busy: true, error: "", notice: "" });
    try { await operation(); } catch (error) { update({ error: String(error) }); }
    finally { update({ busy: false }); }
  }
  /** Refreshes the canonical aggregate without discarding local navigation or drafts. */
  async function refresh(): Promise<void> { update({ snapshot: await uiRequest({ action: "snapshot" }) }); }
  /** Loads management data and the packaged avatar independently of network resources. */
  async function load(): Promise<void> {
    await act(async () => { try { await refresh(); } catch (error) { recovery.reportFailure(error); throw error; } });
    try { update({ defaultAvatar: await ToolPkg.readResource("character_default_avatar", "operit-avatar.png") }); }
    catch (error) { update({ error: String(error) }); }
  }
  /** Opens the one existing native editor with a fresh request-scoped draft. */
  function entity(input: NativeEntityInput): void {
    if (live.current.busy) return;
    setManaged({ requestId: "management-" + (++sequence.current), input });
  }
  /** Refreshes list records after an entity transaction completes. */
  function finishEntity(result: { operation: "saved" | "deleted"; id: string } | null): void {
    setManaged(null); if (result !== null) void act(refresh);
  }
  /** Retains the selected owner explicitly when moving into memory management. */
  function memory(target: MemoryTarget, initial: "browse" | "profile" | "settings" = "browse"): void { update({ memory: { ...target, initial }, navigation: live.current.navigation + 1, error: "" }); }
  /** Opens a batch picker only after the canonical conversation list is available. */
  async function bindings(cardId: string): Promise<void> {
    await act(async () => { update({ panel: { type: "binding", draft: { cardId, query: "", unbound: false, chats: await uiRequest({ action: "listBindableChats" }), selected: [], failures: {}, completed: 0, total: 0 } } }); });
  }
  /** Applies draft changes without capturing a previously rendered batch selection. */
  function bindingChange(patch: Partial<BindingDraft>): void {
    const panel = live.current.panel; if (panel?.type !== "binding" || live.current.busy) return;
    update({ panel: { ...panel, draft: { ...panel.draft, ...patch } } });
  }
  /** Commits each chosen chat, retaining only failed selections for explicit retry. */
  async function submitBindings(): Promise<void> {
    await act(async () => {
      const panel = live.current.panel; if (panel?.type !== "binding") return;
      const draft = panel.draft, selection = "card:" + draft.cardId;
      if (!live.current.snapshot?.cards.some(card => card.id === draft.cardId)) throw new Error("请选择角色卡");
      const ids = draft.selected.filter(id => draft.chats.some(chat => chat.id === id && chat.selection !== selection));
      if (ids.length === 0) throw new Error("请选择要绑定的对话");
      const next = { ...draft, chats: draft.chats.map(chat => ({ ...chat })), failures: {} as Record<string, string>, completed: 0, total: ids.length, selected: [...ids] };
      update({ panel: { type: "binding", draft: next } });
      for (const chatId of ids) {
        try {
          const result = await uiRequest({ action: "writeChatBinding", chatId, selection });
          if (result.chatId !== chatId || result.selection !== selection) throw new Error("绑定结果与请求不一致");
          next.chats.find(chat => chat.id === chatId)!.selection = selection;
          next.selected = next.selected.filter(id => id !== chatId);
        } catch (error) { next.failures[chatId] = String(error); }
        next.completed++; update({ panel: { type: "binding", draft: { ...next } } });
      }
      if (next.selected.length === 0) update({ panel: null, notice: "已绑定 " + ids.length + " 条对话" });
      else update({ error: "部分对话未完成绑定，已保留失败选择，可重试。" });
    });
  }
  /** Renders draft controls without mounting an HTML page behind them. */
  function panelBody(panel: ManagementPanel): ComposeNode[] {
    switch (panel.type) {
      case "store": return [ui.field("store-name", "名称", panel.store.name, name => update({ panel: { ...panel, store: { ...panel.store, name } } })), ui.primary("store-save", "保存", () => act(async () => { const current = live.current.panel; if (current?.type !== "store") return; update({ snapshot: await uiRequest({ action: "saveStore", store: current.store, create: current.create }), panel: null }); }))];
      case "delete-store": return [ui.text("删除共享记忆库「" + panel.name + "」？此操作会影响关联角色。"), ui.primary("store-delete-confirm", "确认删除", () => act(async () => update({ snapshot: await uiRequest({ action: "deleteStore", id: panel.id }), panel: null })))];
      case "import": return [...(panel.entity === "card" ? [ui.choose("import-format", "角色格式", panel.format, [{ id: "operit", label: "Operit JSON" }, { id: "tavern", label: "Tavern JSON" }], format => update({ panel: { ...panel, format: format as "operit" | "tavern" } }))] : []),
        ui.field("import-content", panel.entity === "card" && panel.format === "tavern" ? "粘贴 Tavern JSON" : "粘贴 Operit JSON", panel.content, content => update({ panel: { ...panel, content } }), 18), ui.text("角色卡支持 Operit 和 Tavern JSON；群组使用 Operit JSON。"),
        ui.primary("import-submit", "导入", () => act(async () => { const current = live.current.panel; if (current?.type !== "import") return; update({ snapshot: await uiRequest(current.entity === "card" ? { action: "importCharacter", format: current.format, content: current.content } : { action: "importGroup", content: current.content }), panel: null, notice: "导入完成" }); }))];
      case "binding": {
        const draft = panel.draft, snapshot = state.snapshot!;
        const query = draft.query.trim().toLocaleLowerCase();
        const visible = draft.chats.filter(chat => (!draft.unbound || chat.selection === null) && (query === "" || [chat.title, chatBindingLabel(chat.selection, snapshot)].some(text => text.toLocaleLowerCase().includes(query))));
        const selectable = visible.filter(chat => chat.selection !== "card:" + draft.cardId);
        const all = selectable.length > 0 && selectable.every(chat => draft.selected.includes(chat.id));
        return [ui.text("为已有对话指定角色", true), ui.text("绑定后，对话将使用所选角色的设定继续交流。已有角色绑定会被替换。"),
          ui.choose("binding-card", "绑定到角色卡", draft.cardId, snapshot.cards.map(card => ({ id: card.id, label: card.name })), cardId => bindingChange({ cardId, selected: draft.selected.filter(id => draft.chats.some(chat => chat.id === id && chat.selection !== "card:" + cardId)), failures: {} })),
          ui.field("binding-query", "搜索对话或当前角色", draft.query, query => bindingChange({ query })), ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 8 }, [ctx.UI.Box({ weight: 1 }, ui.choices("binding-filter", "", draft.unbound ? "unbound" : "all", [{ id: "all", label: "全部对话" }, { id: "unbound", label: "未绑定" }], value => bindingChange({ unbound: value === "unbound" }))),
          ui.button("binding-select-visible", all ? "取消全选结果" : "全选结果", () => { const selected = new Set(draft.selected); for (const chat of selectable) { if (all) selected.delete(chat.id); else selected.add(chat.id); } bindingChange({ selected: [...selected] }); }, draft.cardId !== "")]),
          ui.text(state.busy ? `正在绑定 ${draft.completed}/${draft.total}` : `已选择 ${draft.selected.length} 条 · 筛选出 ${visible.length} 条`, false, "binding-status"),
          ...visible.map(chat => ctx.UI.Row({ key: "binding-chat:" + chat.id, fillMaxWidth: true, verticalAlignment: "center", spacing: 8 }, [ctx.UI.Checkbox({ key: "binding-check:" + chat.id, checked: draft.selected.includes(chat.id), enabled: !state.busy && draft.cardId !== "" && chat.selection !== "card:" + draft.cardId, onCheckedChange: checked => bindingChange({ selected: checked ? [...new Set([...draft.selected, chat.id])] : draft.selected.filter(id => id !== chat.id) }) }), ctx.UI.Column({ weight: 1, spacing: 4 }, [ui.text(chat.title + (chat.isCurrent ? " · 当前对话" : ""), true), ui.text(chatBindingLabel(chat.selection, snapshot) + ` · ${chat.messageCount} 条消息`), ...(draft.failures[chat.id] ? [ui.text(draft.failures[chat.id])] : [])])])),
          ...(visible.length === 0 ? [ui.text("没有符合条件的对话")] : []), ui.primary("binding-submit", state.busy ? "绑定中…" : draft.selected.length > 0 ? `绑定 ${draft.selected.length} 条对话` : "绑定对话", submitBindings, draft.cardId !== "" && draft.selected.length > 0)];
      }
    }
  }
  if (recovery.panel !== null) return recovery.panel;

  const snapshot = state.snapshot;
  const children: ComposeNode[] = [];
  if (state.error !== "") children.push(ctx.UI.Text({ key: "management-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error }));
  if (state.notice !== "") children.push(ui.text(state.notice));
  if (snapshot !== null) {
    if (view === "characters") {
      const cards = snapshot.cards.map(card => {
        const active = "CharacterCard" in snapshot.active && snapshot.active.CharacterCard.id === card.id;
        return ui.tile("card:" + card.id, card.name, card.description, [card.chatModelBindingMode === "FIXED_MODEL" ? "固定模型配置" : "跟随全局模型", card.memoryBindingMode === "SHARED" ? "共享记忆" : "角色记忆", ...snapshot.tags.filter(tag => card.attachedTagIds.includes(tag.id)).map(tag => tag.name)], [
          ui.iconButton("card-bind:" + card.id, "绑定对话", "ChatBubbleOutline", () => bindings(card.id)),
          ui.iconButton("card-memory:" + card.id, "打开记忆图谱", "AccountTreeOutlined", () => act(async () => memory(cardMemoryTarget(card)))),
          ui.iconButton("card-user:" + card.id, "编辑用户资料", "AccountBoxOutlined", () => act(async () => memory(cardMemoryTarget(card), "profile"))),
          active ? ui.activePill() : ui.sectionAction("card-activate:" + card.id, "使用", "", () => act(async () => update({ snapshot: await uiRequest({ action: "activate", type: "card", id: card.id }) }))),
        ], active, () => entity({ mode: "edit", entity: "card", id: card.id }), card.avatarUri ?? state.defaultAvatar);
      });
      children.push(ui.section("character-section", "角色卡", "BadgeOutlined", [ui.sectionAction("bind-chats", "绑定对话", "ChatBubbleOutline", () => bindings(""), snapshot.cards.length > 0), ui.menu("add-card", "添加", "Add", [{ key: "create-card", label: "新建角色卡", name: "Add", action: () => entity({ mode: "create", entity: "card", id: "" }) }, { key: "import-card", label: "导入角色卡", name: "Upload", action: () => update({ panel: { type: "import", entity: "card", format: "operit", content: "" } }) }])], cards.length === 0 ? [ui.text("还没有角色卡，点击创建添加角色。")] : cards));
      const groups = snapshot.groups.map(group => {
        const active = "CharacterGroup" in snapshot.active && snapshot.active.CharacterGroup.id === group.id;
        return ui.tile("group:" + group.id, group.name, group.members.map(member => snapshot.cards.find(card => card.id === member.characterCardId)?.name ?? "已删除角色").join("、"), [`${group.members.length} 个角色`], [active ? ui.activePill() : ui.sectionAction("group-activate:" + group.id, "使用", "", () => act(async () => update({ snapshot: await uiRequest({ action: "activate", type: "group", id: group.id }) })))], active, () => entity({ mode: "edit", entity: "group", id: group.id }), null, "GroupsOutlined");
      });
      children.push(ui.section("group-section", "群组", "GroupsOutlined", [ui.sectionAction("import-group", "导入 JSON", "Upload", () => update({ panel: { type: "import", entity: "group", format: "operit", content: "" } })), ui.sectionAction("create-group", "创建", "Add", () => entity({ mode: "create", entity: "group", id: "" }))], groups.length === 0 ? [ui.text("还没有群组，创建后可编排多个角色参与对话。")] : groups));
    } else {
      const global: ComposeNode[] = [];
      for (const [key, label, description] of [["userDescriptionEnabled", "向模型提供用户资料", "聊天执行时提供用户资料；关闭不会删除 USER.md 或改写历史记录。"], ["autoUpdateEnabled", "自动更新记忆库", "允许自动候选入队和定时提取；关闭不会取消手动历史重建。"]] as const) global.push(ctx.UI.Row({ fillMaxWidth: true, spacing: 16, paddingVertical: 10, verticalAlignment: "center" }, [ctx.UI.Column({ weight: 1, spacing: 4 }, [ctx.UI.Text({ text: label, fontSize: 14 }), ctx.UI.Text({ text: description, fontSize: 12, color: ctx.MaterialTheme.colorScheme.onSurfaceVariant })]), ctx.UI.Switch({ key: "memory-preference:" + key, checked: snapshot.memoryPreferences[key], enabled: !state.busy, onCheckedChange: value => act(async () => { const memoryPreferences = await uiRequest({ action: "writeGlobalMemoryPreference", key, value }); update({ snapshot: { ...live.current.snapshot!, memoryPreferences } }); }) })]));
      children.push(ui.section("global-memory-section", "全局记忆设置", "Tune", [], global));
      /** Keeps graph, USER.md and settings as three separate original entries. */
      function ownerActions(target: MemoryTarget, scope = ""): ComposeNode[] {
        if (!snapshot!.memoryOwnerKeys.includes(target.ownerKey)) return [ui.sectionAction("library-create:" + target.ownerKey + scope, target.ownerKey.startsWith("character:") ? "新建角色卡记忆库" : "新建共享记忆库", "Add", () => act(async () => update({ snapshot: await uiRequest({ action: "createMemoryLibrary", ownerKey: target.ownerKey }) })))];
        return [ui.iconButton("library-open:" + target.ownerKey + scope, "记忆图谱", "AccountTreeOutlined", () => memory(target)), ui.iconButton("library-user:" + target.ownerKey + scope, "用户资料 · USER.md", "AccountBoxOutlined", () => memory(target, "profile")), ui.iconButton("library-settings:" + target.ownerKey + scope, "记忆设置", "Tune", () => memory(target, "settings"))];
      }
      const stores = snapshot.stores.map(store => ui.tile("store:" + store.id, store.name, "", [`${snapshot.cards.filter(card => card.memoryBindingMode === "SHARED" && card.sharedMemoryId === store.id).length} 个角色绑定`], [...ownerActions({ ownerKey: "shared:" + store.id, name: store.name }), ui.iconButton("store-edit:" + store.id, "编辑共享记忆库", "EditOutlined", () => update({ panel: { type: "store", create: false, store: { id: store.id, name: store.name } } })), ui.iconButton("store-delete:" + store.id, "删除共享记忆库", "DeleteOutline", () => update({ panel: { type: "delete-store", id: store.id, name: store.name } }))], false, undefined, null, "HubOutlined"));
      children.push(ui.section("shared-memory-section", "共享记忆库", "HubOutlined", [ui.sectionAction("create-store", "创建", "Add", () => update({ panel: { type: "store", create: true, store: { id: "", name: "" } } }))], stores.length === 0 ? [ui.text("当前还没有共享记忆库，创建后可被多个角色卡挂载共享。")] : stores));
      const cards = snapshot.cards.map(card => {
        const missing = card.memoryBindingMode === "SHARED" && !snapshot.stores.some(store => store.id === card.sharedMemoryId);
        const target = missing ? null : cardMemoryTarget(card);
        const active = "CharacterCard" in snapshot.active && snapshot.active.CharacterCard.id === card.id;
        return ui.tile("memory-card:" + card.id, card.name, "", [card.memoryBindingMode === "SHARED" ? "共享记忆" : "角色记忆", ...(missing ? ["需要重新绑定记忆库"] : target !== null && !snapshot.memoryOwnerKeys.includes(target.ownerKey) ? ["当前角色无记忆库"] : []), ...(active ? ["当前使用"] : [])], target === null ? [ui.sectionAction("memory-rebind:" + card.id, "重新绑定记忆库", "EditOutlined", () => entity({ mode: "edit", entity: "card", id: card.id }))] : ownerActions(target, card.memoryBindingMode === "SHARED" ? ":card:" + card.id : ""), active, undefined, card.avatarUri ?? state.defaultAvatar);
      });
      children.push(ui.section("character-memory-section", "角色记忆库", "BadgeOutlined", [], cards));
    }
  } else children.push(ui.text(state.busy ? "正在读取…" : "尚未读取成功，请重试"));
  const overlays: ComposeNode[] = [];
  if (state.panel !== null) {
    const panel = state.panel, fields = panelBody(panel), submit = fields.pop()!;
    const close = (): void => { if (!live.current.busy) update({ panel: null, error: "" }); };
    const title = panel.type === "binding" ? "批量绑定对话" : panel.type === "import" ? panel.entity === "card" ? "导入角色卡" : "导入群组" : panel.type === "delete-store" ? "删除共享记忆库" : panel.create ? "创建共享记忆库" : "编辑共享记忆库";
    const messages = state.error === "" ? [] : [ctx.UI.Text({ text: state.error, color: ctx.MaterialTheme.colorScheme.error })];
    const body = panel.type === "binding" ? ctx.UI.Column({ fillMaxSize: true }, [
      ctx.UI.Column({ fillMaxWidth: true, paddingHorizontal: 18, spacing: 6 }, fields.filter(node => !String(node.props?.key ?? "").startsWith("binding-chat:") && node.props?.key !== "binding-status")),
      ui.list("binding-chat-list", [...messages, ...fields.filter(node => String(node.props?.key ?? "").startsWith("binding-chat:"))]),
    ]) : ui.list("management-form", [...messages, ...fields]);
    const cancel = ui.button("management-back", "取消", close);
    const status = fields.find(node => node.props?.key === "binding-status");
    const footer = panel.type !== "binding" ? [cancel, submit] : [ui.size.width <= 600
      ? ctx.UI.Column({ fillMaxWidth: true, spacing: 8 }, [status!, ctx.UI.Row({ fillMaxWidth: true, horizontalArrangement: "end", verticalAlignment: "center", spacing: 8 }, [cancel, submit])])
      : ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 8 }, [ctx.UI.Box({ weight: 1 }, status!), cancel, submit])];
    overlays.push(ui.dialog("management-dialog", title, body, footer, close, panel.type === "binding" || panel.type === "import" ? "large" : "compact"));
  }
  if (state.memory !== null) overlays.push(renderMemoryWorkspace(ctx, state.memory, () => update({ memory: null }), "management-memory:" + state.navigation));
  if (managed !== null) overlays.push(renderManagedEntityScreen(ctx, managed, finishEntity));
  return ctx.UI.Box({ key: "management-root", fillMaxSize: true, modifier: ui.measure, onLoad: load }, [ctx.UI.Column({ fillMaxSize: true }, [ctx.UI.LazyColumn({ key: "management-list", weight: 1, fillMaxWidth: true, paddingHorizontal: ui.size.width <= 600 ? 12 : 16, paddingTop: 12, paddingBottom: 20 }, children)]), ...overlays]);
}
