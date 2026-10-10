import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { reconcileSidebarCatalog, parseCurrentSidebar, type CurrentSidebar, type SidebarCatalog, type ConversationSidebarGroup, type ConversationSidebarScope } from "./ui-sidebar";
import { callMainDomain, encodeSelection } from "./ui-contributions";
import { createChatInput } from "./chat-lifecycle";
import { imageSource } from "./image-source";
import { nativeSidebarRows } from "./ui-sidebar-native-layout";
import { deleteConversationGroupMembers, type SidebarDeletionProgress } from "./sidebar-group-delete";

/** Native local state, with no browser controller, resource, or DOM lifecycle. */
interface SidebarState {
  data: SidebarCatalog | null; defaultAvatar: string | null; context: string; loading: boolean; busy: boolean; error: string;
  query: string; search: boolean; collapsed: string[]; expandedGroups: string[];
  dialog: { kind: "create" | "rename" | "delete" | "renameChat" | "deleteChat" | "chatActions" | "renameUngrouped" | "deleteUngrouped"; id: string; name: string; progress: SidebarDeletionProgress | null } | null;
}

/** Renders host chat metadata through native rows and resolves business data only through main-runtime IPC. */
export function renderSidebarScreen(ctx: ComposeDslContext): ComposeNode {
  const [input] = ctx.useState<CurrentSidebar["input"] | null>("input", null);
  const [chatSidebar] = ctx.useState<CurrentSidebar["chatSidebar"] | null>("chatSidebar", null);
  const supplied = { input, chatSidebar };
  const current = ctx.useRef<CurrentSidebar>("native-sidebar-current", parseCurrentSidebar(supplied));
  current.current = parseCurrentSidebar(supplied);
  const catalogKey = (value: CurrentSidebar): string => JSON.stringify({ input: value.input, chats: value.chatSidebar.chats });
  const identity = catalogKey(current.current), colors = ctx.MaterialTheme.colorScheme;
  const [state, setState] = ctx.useState<SidebarState>("native-sidebar-state", {
    data: null, defaultAvatar: null, context: "", loading: true, busy: false, error: "", query: "", search: false,
    collapsed: [], expandedGroups: [], dialog: null,
  });
  const latest = ctx.useRef("native-sidebar-state-ref", state); latest.current = state;
  const pending = ctx.useRef<{ key: string; promise: Promise<void> } | null>("native-sidebar-pending", null);
  /** Publishes only local state, preserving any changes made while an asynchronous read was pending. */
  function update(change: Partial<SidebarState>): void { latest.current = { ...latest.current, ...change }; setState(latest.current); }
  /** Rejects stale callbacks before changing any chat or group in a different supplied host context. */
  function assertCurrent(): void { if (catalogKey(current.current) !== identity) throw new Error("The sidebar context changed; refresh before using this action"); }
  /** Loads the exact native context and discards superseded responses, never publishing a guessed empty catalog. */
  async function refresh(): Promise<void> {
    const request = current.current, key = catalogKey(request);
    const active = pending.current;
    if (active?.key === key && typeof active.promise?.then === "function") return active.promise;
    const promise = load();
    pending.current = { key, promise };
    try { await promise; } finally { if (pending.current?.promise === promise) pending.current = null; }
    async function load(): Promise<void> {
    update({ loading: latest.current.data === null || latest.current.data.view !== request.input.view, error: "" });
    try {
      const data = await ToolPkg.ipc.call<CurrentSidebar, SidebarCatalog>("character-sidebar.catalog", request, { targetRuntime: "main" });
      if (catalogKey(current.current) === key) {
        if (data.view !== request.input.view) throw new Error("The sidebar catalog belongs to another view");
        let defaultAvatar = latest.current.defaultAvatar;
        if ((data.view === "characters" ? data.sections : data.scopes).some(section => section.kind === "card" && section.avatarUri === null) && defaultAvatar === null) {
          defaultAvatar = await imageSource(await ToolPkg.readResource("character_default_avatar", "operit-avatar.png"));
          if (defaultAvatar.trim() === "") throw new Error("Declared default avatar resource is missing");
        }
        const sections = data.view === "characters" ? data.sections : data.scopes;
        for (let offset = 0; offset < sections.length; offset += 8) {
          await Promise.all(sections.slice(offset, offset + 8).map(async section => {
            if (section.avatarUri !== null && section.avatarUri !== undefined)
              section.avatarUri = await imageSource(section.avatarUri);
          }));
        }
        if (catalogKey(current.current) === key) update({ data, defaultAvatar, context: key, loading: false });
      }
    } catch (failure) {
      if (catalogKey(current.current) === key) update({ loading: false, context: key, error: String(failure) });
    }
    }
  }
  /** Selection/streaming updates need a rerender, not another authoritative directory read. */
  async function refreshInputs(): Promise<void> {
    if (latest.current.data !== null && latest.current.error === "" && latest.current.context === catalogKey(current.current)) return;
    await refresh();
  }
  /** Serializes real mutations and keeps backend failures visible without selecting a replacement owner. */
  async function run<T>(action: () => Promise<T>): Promise<T | null> {
    assertCurrent(); if (latest.current.busy) throw new Error("A sidebar operation is already running");
    update({ busy: true, error: "" });
    try { return await action(); } catch (failure) { update({ error: String(failure) }); return null; }
    finally { update({ busy: false }); }
  }
  /** Returns the existing generic action; native navigation remains entirely in the host. */
  function activate(chatId: string): { type: string; chatId: string } {
    assertCurrent();
    if (!current.current.chatSidebar.chats.some(chat => chat.id === chatId)) throw new Error("The conversation is absent from the supplied sidebar history");
    return { type: "toolpkg.chat.activate", chatId };
  }
  /** Resolves one exact scope from the loaded plugin projection, not from a title or native business field. */
  function scopeFor(id: string): ConversationSidebarScope {
    const data = visibleCatalog();
    if (data === null) throw new Error("Conversation groups are not loaded");
    const scopes = data.view !== "characters" ? data.scopes : data.sections.map(section => ({ id: section.id, title: section.title,
      ownerSelection: section.selection, groups: section.conversationGroups ?? [], ungrouped: section.ungroupedChats ?? section.chats }));
    const scope = scopes.find(owner => owner.id === id || owner.groups.some(group => group.id === id));
    if (scope === undefined) throw new Error("The conversation-group scope no longer exists");
    return scope;
  }
  function readGroup(id: string): ConversationSidebarGroup {
    const group = scopeFor(id).groups.find(group => group.id === id);
    if (group === undefined) throw new Error("The native folder no longer exists");
    return group;
  }
  /** Creates through the native lifecycle and persists native folder membership before activation. */
  async function createChat(selection: string, groupId: string | null): Promise<{ type: string; chatId: string } | null> {
    assertCurrent(); if (latest.current.busy) return null;
    update({ busy: true, error: "" });
    try {
      const created = await Tools.Chat.createNew({ setAsCurrentChat: false, input: selection === "" ? null : createChatInput(selection) });
      await Tools.Chat.updateGroup([created.chatId], groupId === null ? null : readGroup(groupId).name);
      return { type: "toolpkg.chat.activate", chatId: created.chatId };
    } catch (failure) { update({ error: String(failure) }); return null; }
    finally { update({ busy: false }); }
  }
  /** Opens only staged native dialog state; it never mutates a group while the dialog is shown. */
  function dialog(kind: NonNullable<SidebarState["dialog"]>["kind"], id: string, name = ""): void {
    assertCurrent(); update({ dialog: { kind, id, name, progress: null } });
  }
  /** Applies one explicit confirmation, retaining original partial deletion evidence rather than pretending rollback. */
  async function submit(): Promise<{ type: string; chatId: string } | null> {
    const staged = latest.current.dialog;
    if (staged === null) throw new Error("No sidebar dialog is open");
    switch (staged.kind) {
      case "chatActions": throw new Error("Choose a conversation action");
      case "renameChat": await Tools.Chat.updateTitle(staged.id, staged.name); break;
      case "deleteChat": await Tools.Chat.deleteChat(staged.id); break;
      case "create": {
        const name = staged.name.trim(); if (name === "") throw new Error("分组名称不能为空");
        const selection = scopeFor(staged.id).ownerSelection;
        const created = await Tools.Chat.createNew({ setAsCurrentChat: false, input: selection === null ? null : createChatInput(selection) });
        await Tools.Chat.updateGroup([created.chatId], name);
        update({ dialog: null }); await refresh();
        return { type: "toolpkg.chat.activate", chatId: created.chatId };
      }
      case "renameUngrouped": {
        await Tools.Chat.updateGroup(scopeFor(staged.id).ungrouped.map(chat => chat.id), staged.name.trim());
        break;
      }
      case "deleteUngrouped": {
        const progress: SidebarDeletionProgress = {deletedChatIds: [], failedChatIds: [], };
        for (const chat of scopeFor(staged.id).ungrouped) {
          try { await Tools.Chat.deleteChat(chat.id); progress.deletedChatIds.push(chat.id); }
          catch (error) { progress.failedChatIds.push({chatId: chat.id, error: String(error)}); }
        }
        update({dialog: {...staged, progress}}); await refresh(); return null;
      }
      case "rename": await Tools.Chat.updateGroup(readGroup(staged.id).chats.map(chat => chat.id), staged.name.trim()); break;
      case "delete": {
        if (staged.progress !== null) throw new Error("This confirmed deletion has already been attempted");
        const group = readGroup(staged.id);
        const progress = await deleteConversationGroupMembers(group.chats.map(chat => chat.id), id => Tools.Chat.deleteChat(id));
        update({ dialog: { ...staged, progress } }); await refresh(); return null;
      }
    }
    update({ dialog: null }); await refresh();
    return null;
  }
  /** Uses native anchored popup menus, rather than turning a three-dot click into a modal page. */
  function chatFor(id: string) {
    const chat = current.current.chatSidebar.chats.find(value => value.id === id);
    if (chat === undefined) throw new Error("The conversation no longer exists");
    return chat;
  }
  function renameChat(id: string): void { dialog("renameChat", id, chatFor(id).title); }
  function deleteChat(id: string): void { dialog("deleteChat", id, chatFor(id).title); }
  function longPress(id: string): void { dialog("chatActions", id, chatFor(id).title); }
  async function changeFlag(id: string, pinned: boolean): Promise<void> {
    await run(async () => {
      const chat = chatFor(id);
      if (pinned) await Tools.Chat.updatePinned(id, !chat.pinned);
      else await Tools.Chat.updateLocked(id, !chat.locked);
      update({ dialog: null }); await refresh();
    });
  }
  async function reorder(chatId: string, targetId: string): Promise<void> {
    const ids = current.current.chatSidebar.chats.map(chat => chat.id);
    const from = ids.indexOf(chatId), to = ids.indexOf(targetId);
    if (from < 0 || to < 0) throw new Error("The reordered conversation no longer exists");
    if (from === to) return;
    ids.splice(to, 0, ids.splice(from, 1)[0]!);
    await Tools.Chat.reorder(ids);
  }
  function orderNeighbours(id: string): string[] {
    const data = visibleCatalog();
    if (data === null) return [];
    const scopes = data.view !== "characters" ? data.scopes : data.sections.map(section => ({ groups: section.conversationGroups ?? [], ungrouped: section.ungroupedChats ?? section.chats }));
    for (const scope of scopes) {
      const values = scope.groups.find(group => group.chats.some(chat => chat.id === id))?.chats ?? scope.ungrouped;
      if (values.some(chat => chat.id === id)) return values.map(chat => chat.id);
    }
    return [];
  }
  async function moveRelative(id: string, delta: number): Promise<void> {
    const ids = orderNeighbours(id), index = ids.indexOf(id), target = ids[index + delta];
    if (index < 0 || target === undefined) return;
    await run(async () => { await reorder(id, target); update({ dialog: null }); await refresh(); });
  }
  function menu(kind: "group" | "chat" | "ungrouped", id: string): ComposeNode {
    if (kind === "chat") {
      const chat = chatFor(id);
      return ctx.UI.PopupMenu({ key: "sidebar-chat-menu-" + id, width: 22, height: 22, tooltip: "对话操作", enabled: !state.busy,
        items: [ { label: "编辑名称", icon: "Edit" },
          { label: chat.pinned ? "取消置顶" : "置顶", icon: "PushPin" },
          { label: chat.locked ? "解锁" : "锁定", icon: chat.locked ? "LockOpen" : "Lock" },
          { label: "删除", icon: "DeleteOutline", danger: true } ],
        onSelected: async (index: number) => {
          if (index === 0) renameChat(id);
          else if (index === 1) await changeFlag(id, true);
          else if (index === 2) await changeFlag(id, false);
          else if (index === 3) deleteChat(id);
        },
      }, ctx.UI.Icon({ name: "MoreHoriz", size: 16, tint: colors.onSurfaceVariant.copy({ alpha: 0.78 }) }));
    }
    if (kind === "ungrouped") {
      const scope = scopeFor(id);
      const pinned = scope.ungrouped.length > 0 && scope.ungrouped.every(chat => chat.pinned);
      return ctx.UI.PopupMenu({ key: "sidebar-ungrouped-menu-" + id, width: 22, height: 22,
        menuMaxWidth: 138, tooltip: "分组操作", enabled: !state.busy && state.data !== null,
        items: [{label:"新建对话",icon:"AddCommentOutlined"}, {label:"编辑名称",icon:"EditOutlined"},
          {label:pinned ? "取消置顶" : "置顶",icon:pinned ? "PushPinOutlined" : "PushPinRounded"},
          {label:"删除",icon:"DeleteOutlineRounded",danger:true,dividerBefore:true}],
        onSelected: async (index: number) => {
          if (index === 0) return createChat(scope.ownerSelection ?? "", null);
          if (index === 1) dialog("renameUngrouped", id, "未分组");
          if (index === 2) await run(async () => {
            for (const chat of scope.ungrouped) await Tools.Chat.updatePinned(chat.id, !pinned);
            await refresh();
          });
          if (index === 3) dialog("deleteUngrouped", id, "未分组");
          return null;
        },
      }, ctx.UI.Icon({name:"MoreHorizRounded",size:16,tint:colors.onSurfaceVariant.copy({alpha:.78})}));
    }
    const group = scopeFor(id).groups.find(value => value.id === id)!;
    const items = [
      { label: "新建对话", icon: "AddCommentOutlined" }, { label: "编辑名称", icon: "EditOutlined" },
      { label: group.pinned ? "取消置顶" : "置顶", icon: group.pinned ? "PushPinOutlined" : "PushPinRounded" },
      { label: "删除", icon: "DeleteOutlineRounded", danger: true, dividerBefore: true },
    ];
    return ctx.UI.PopupMenu({ key: "sidebar-group-menu-" + id, width: 22, height: 22,
      menuMaxWidth: 138,
      tooltip: "分组操作", items, enabled: !state.busy,
      onSelected: async (index: number) => {
        if (index === 0) {
          const owner = scopeFor(id).ownerSelection;
          return createChat(owner ?? "", id);
        } else if (index === 1) dialog("rename", id, group.name);
        else if (index === 2) await run(async () => {
          for (const chat of group.chats) await Tools.Chat.updatePinned(chat.id, !group.pinned); await refresh();
        });
        else if (index === 3) dialog("delete", id, group.name);
        return null;
      },
    }, ctx.UI.Icon({ name: "MoreHorizRounded", size: 16, tint: colors.onSurfaceVariant.copy({ alpha: 0.78 }) }));
  }
  /** Native drag payloads are IDs; only this plugin reads and commits membership. */
  async function drop(chatId: string, scopeId: string, groupId: string | null, targetId?: string): Promise<void> {
    assertCurrent();
    if (latest.current.busy) throw new Error("A sidebar operation is already running");
    const scope = scopeFor(scopeId);
    if (!scope.ungrouped.some(chat => chat.id === chatId) && !scope.groups.some(group => group.chats.some(chat => chat.id === chatId)))
      throw new Error("The conversation no longer belongs to the target scope");
    if (groupId !== null && !scope.groups.some(group => group.id === groupId))
      throw new Error("The target conversation group no longer exists");
    const existing = scope.groups.find(group => group.chats.some(chat => chat.id === chatId))?.id ?? null;
    if (existing === groupId && targetId === undefined) return;
    update({ busy: true, error: "" });
    try {
      if (existing !== groupId) await Tools.Chat.updateGroup([chatId], groupId === null ? null : readGroup(groupId).name);
      if (targetId !== undefined) await reorder(chatId, targetId);
      await refresh();
    } catch (failure) { update({ error: String(failure) }); throw failure; }
    finally { update({ busy: false }); }
  }
  function visibleCatalog(): SidebarCatalog | null {
    const data = latest.current.data;
    return data === null || data.view !== current.current.input.view ? null
      : latest.current.context === catalogKey(current.current) ? data : reconcileSidebarCatalog(data, current.current);
  }
  const data = visibleCatalog();
  const toggleSection = (id: string): void => update({ collapsed: state.collapsed.includes(id)
    ? state.collapsed.filter(key => key !== id) : [...state.collapsed, id] });
  const toggleGroup = (id: string): void => update({ expandedGroups: state.expandedGroups.includes(id)
    ? state.expandedGroups.filter(key => key !== id) : [...state.expandedGroups, id] });
  const rows: ComposeNode[] = [ctx.UI.Row({ key: "sidebar-toolbar", fillMaxWidth: true,
    verticalAlignment: "center", spacing: 8, paddingStart: 14, paddingEnd: 12, paddingBottom: 8 }, [
    ctx.UI.Surface({ key: "sidebar-create-bar", weight: 1, height: 34,
      containerColor: colors.primaryContainer, contentColor: colors.onPrimaryContainer, shape: { type: "pill" } },
      ctx.UI.Row({ fillMaxSize: true, verticalAlignment: "center" }, [
      ctx.UI.Row({ key: "sidebar-new-chat", weight: 1, height: 34, verticalAlignment: "center", horizontalArrangement: "center",
        onClick: async () => {
          const active = await callMainDomain("activePrompt.get", {});
          return createChat(encodeSelection(active), null);
        }, spacing: 6 }, [ctx.UI.Icon({ name: "AddRounded", size: 17, tint: colors.onPrimaryContainer }),
        ctx.UI.Text({ text: "新建对话", fontSize: 13, fontWeight: "600", color: colors.onPrimaryContainer, ...{ letterSpacing: -0.1 } })]),
      ctx.UI.Box({ width: 1, height: 16, background: colors.onPrimaryContainer.copy({ alpha: 0.16 }) }),
      ctx.UI.Row({ key: "sidebar-new-group", width: 38, height: 34, verticalAlignment: "center", horizontalArrangement: "center",
        onClick: async () => {
          const active = await callMainDomain("activePrompt.get", {});
          const selection = encodeSelection(active);
          const catalog = visibleCatalog();
          if (catalog === null) throw new Error("Sidebar has not loaded");
          const scope = catalog.view === "characters" ? catalog.sections.find(section => section.selection === selection)
            : catalog.scopes.find(scope => scope.ownerSelection === selection);
          if (scope === undefined) throw new Error("The active owner is absent from the actual sidebar catalog");
          dialog("create", scope.id);
        } }, ctx.UI.Icon({ name: "CreateNewFolderOutlined", size: 16, tint: colors.onPrimaryContainer, contentDescription: "新建分组" })),
    ])),
    ctx.UI.Row({ key: "sidebar-search-toggle", width: 34, height: 34, verticalAlignment: "center", horizontalArrangement: "center",
      onClick: () => update({ search: !state.search }), modifier: ctx.Modifier.clip({ cornerRadius: 6 }),
      background: state.search ? colors.secondaryContainer.copy({ alpha: 0.35 }) : colors.surface.copy({ alpha: 0 }) },
      ctx.UI.Icon({ name: state.search ? "SearchOff" : "Search", size: 17, tint: colors.onSurfaceVariant.copy({ alpha: 0.78 }), contentDescription: state.search ? "收起搜索" : "搜索对话" })),
  ])];
  if (state.search) rows.push(ctx.UI.TextField({ key: "sidebar-search", value: state.query,
    onValueChange: query => update({ query }), placeholder: "搜索对话", singleLine: true,
    fillMaxWidth: true, paddingHorizontal: 12, paddingBottom: 12, ...{ shape: { cornerRadius: 14 } } }));
  if (state.error !== "") rows.push(ctx.UI.Text({ key: "sidebar-error", text: state.error, fontSize: 11, color: colors.error, paddingHorizontal: 20, paddingVertical: 6 }));
  if (data !== null) rows.push(...nativeSidebarRows(ctx, current.current, { data, query: state.query,
    collapsed: state.collapsed, expandedGroups: state.expandedGroups }, {
      activate, toggleSection, toggleGroup,
      menu, longPress, rename: renameChat, deleteChat, drop,
    }, state.defaultAvatar));
  else if (state.loading) rows.push(ctx.UI.Box({ fillMaxWidth: true, height: 80, contentAlignment: "center" },
    ctx.UI.LoadingIndicator({ size: 24 })));
  const content: ComposeNode[] = [ctx.UI.Column({ key: "native-sidebar-history", fillMaxWidth: true, paddingBottom: 16, spacing: 0 }, rows)];
  if (state.dialog !== null) {
    const staged = state.dialog, body: ComposeNode[] = [ctx.UI.Text({ text: { create: "新建分组", rename: "重命名分组", delete: "删除分组及其对话", renameUngrouped: "重命名分组", deleteUngrouped: "删除分组及其对话", renameChat: "编辑对话名称", deleteChat: "删除对话", chatActions: "聊天记录" }[staged.kind], fontSize: 16, fontWeight: "600" })];
    if (staged.kind === "create" || staged.kind === "rename" || staged.kind === "renameUngrouped" || staged.kind === "renameChat") body.push(ctx.UI.TextField({ key: "sidebar-group-name", value: staged.name, label: staged.kind === "renameChat" ? "对话名称" : "分组名称", onValueChange: name => update({ dialog: { ...staged, name } }), enabled: !state.busy }));
    if (staged.kind === "delete" || staged.kind === "deleteUngrouped" || staged.kind === "deleteChat") body.push(ctx.UI.Text({ text: staged.kind === "deleteChat" ? `确认删除对话“${staged.name}”？此操作不可撤销。` : `确认删除“${staged.name}”及其中的全部对话？此操作不可撤销。`, fontSize: 12 }));
    if (staged.kind === "chatActions") {
      const chat = chatFor(staged.id), ids = orderNeighbours(staged.id), index = ids.indexOf(staged.id);
      body.push(ctx.UI.Text({ text: chat.title, fillMaxWidth: true, maxLines: 2, style: "titleMedium", color: colors.onSurfaceVariant }));
      const tile = (label: string, icon: string, callback: () => void | Promise<void>, enabled = true, danger = false): ComposeNode =>
        ctx.UI.Surface({ fillMaxWidth: true, onClick: enabled ? callback : undefined, shape: { cornerRadius: 12 }, color: colors.surface },
          ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 16, paddingVertical: 10, verticalAlignment: "center", spacing: 12 }, [
            ctx.UI.Icon({ name: icon, size: 20, tint: danger ? colors.error : colors.onSurfaceVariant }),
            ctx.UI.Text({ text: label, weight: 1, fontSize: 14, color: danger ? colors.error : colors.onSurface }),
          ]));
      body.push(tile("编辑名称", "Edit", () => renameChat(chat.id)),
        tile("上移", "KeyboardArrowUp", () => moveRelative(chat.id, -1), index > 0),
        tile("下移", "KeyboardArrowDown", () => moveRelative(chat.id, 1), index >= 0 && index < ids.length - 1),
        tile(chat.pinned ? "取消置顶" : "置顶", "PushPin", () => changeFlag(chat.id, true)),
        tile(chat.locked ? "解锁" : "锁定", chat.locked ? "LockOpen" : "Lock", () => changeFlag(chat.id, false)),
        tile("删除", "DeleteOutline", () => deleteChat(chat.id), true, true));
    }
    if (staged.progress !== null) body.push(ctx.UI.Text({ key: "sidebar-delete-result", text: `已删除 ${staged.progress.deletedChatIds.length} 条对话\n` + staged.progress.failedChatIds.map(value => value.chatId + ": " + value.error).join("\n"), fontSize: 11, color: staged.progress.failedChatIds.length === 0 ? colors.onSurface : colors.error }));
    if (state.error !== "") body.push(ctx.UI.Text({ text: state.error, color: colors.error, fontSize: 11 }));
    body.push(ctx.UI.Row({ spacing: 8 }, [ctx.UI.TextButton({ text: "取消", enabled: !state.busy, onClick: () => update({ dialog: null }) }),
      ...(staged.progress === null && staged.kind !== "chatActions" ? [ctx.UI.Button({ text: "确认", enabled: !state.busy, onClick: () => run(submit) })] : [])]));
    content.push(ctx.UI.Dialog({ key: "sidebar-edit-dialog", closeOnDismissRequest: !state.busy, onDismissRequest: () => { if (!state.busy) update({ dialog: null }); } }, ctx.UI.Column({ padding: 20, spacing: 12, width: 320 }, body)));
  }
  return ctx.UI.Column({ key: "native-character-sidebar", fillMaxWidth: true, onLoad: refresh, onInputsChanged: refreshInputs }, content);
}
