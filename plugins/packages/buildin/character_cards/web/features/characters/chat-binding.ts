import type { BindableChat, Snapshot } from "../../../src/model";
import type { ActionFeature, EditorContext } from "../../bridge/context";
import type { ChatBindingDialog, DialogSpec } from "../../bridge/contracts";
import { dataValue } from "../../shared/dom";
import { button, escapeHtml, icon, selectField } from "../../shared/ui/html";
import { getCard } from "./data";

/** Resolves a stored binding for display without substituting the active global role. */
export function bindingLabel(selection: string | null, snapshot: Snapshot): string {
  if (selection === null) return "未绑定角色";
  if (selection === "model:chat") return "仅使用模型";
  for (const card of snapshot.cards) if (selection === "card:" + card.id) return card.name;
  for (const group of snapshot.groups) if (selection === "group:" + group.id) return "群组 · " + group.name;
  return "原角色已失效";
}

/** Searches titles and current role names; filtering never discards hidden selections. */
export function filteredBindingChats(dialog: ChatBindingDialog, snapshot: Snapshot): BindableChat[] {
  const query = dialog.query.trim().toLocaleLowerCase();
  return dialog.chats.filter(chat => (dialog.filter !== "unbound" || chat.selection === null)
    && (query === "" || [chat.title, bindingLabel(chat.selection, snapshot)].some(value => value.toLocaleLowerCase().includes(query))));
}

/** Excludes already-bound rows from checkbox selection and batch submission. */
export function selectableBindingChats(dialog: ChatBindingDialog, snapshot: Snapshot): BindableChat[] {
  return filteredBindingChats(dialog, snapshot).filter(chat => chat.selection !== "card:" + dialog.cardId);
}

function chatDate(value: string): string {
  const date = new Date(/^\d+$/.test(value) ? Number(value) : value);
  return Number.isNaN(date.getTime()) ? "" : date.toLocaleDateString("zh-CN", { month: "numeric", day: "numeric" });
}

/** Renders a bounded picker with fixed controls and a separately scrolling history list. */
export function chatBindingSpec(dialog: ChatBindingDialog, snapshot: Snapshot): DialogSpec {
  const visible = filteredBindingChats(dialog, snapshot), selectable = selectableBindingChats(dialog, snapshot);
  const allSelected = selectable.length > 0 && selectable.every(chat => dialog.selectedIds.includes(chat.id));
  let rows = "";
  for (const chat of visible) {
    const bound = chat.selection === "card:" + dialog.cardId, selected = dialog.selectedIds.includes(chat.id);
    const failure = dialog.failures[chat.id];
    rows += `<label class="binding-chat ${selected ? "selected" : ""} ${bound ? "already-bound" : ""}"><input type="checkbox" data-binding-chat="${escapeHtml(chat.id)}" aria-label="选择对话 ${escapeHtml(chat.title)}" ${selected ? "checked" : ""} ${bound || dialog.busy || dialog.cardId === "" ? "disabled" : ""}><span class="binding-chat-copy"><span class="binding-chat-title">${escapeHtml(chat.title || "未命名对话")}${chat.isCurrent ? '<span class="binding-current">当前对话</span>' : ""}</span><span class="binding-chat-meta"><span class="binding-owner ${bound ? "bound" : ""}">${bound ? icon("check") : icon("person")}${escapeHtml(bound ? "已绑定" : bindingLabel(chat.selection, snapshot))}</span><span>${chat.messageCount} 条消息${chatDate(chat.updatedAt) === "" ? "" : " · " + escapeHtml(chatDate(chat.updatedAt))}</span></span>${failure === undefined ? "" : `<span class="binding-chat-error">${escapeHtml(failure)}</span>`}</span></label>`;
  }
  const target = selectField("绑定到角色卡", "bindingCard", dialog.cardId, snapshot.cards.map(card => ({ id: card.id, label: card.name })), dialog.loading || dialog.busy);
  const filter = (value: "all" | "unbound", label: string) => `<button type="button" class="chip ${dialog.filter === value ? "selected" : ""}" data-action="binding-filter" data-filter="${value}" aria-pressed="${dialog.filter === value}" ${dialog.busy || dialog.loading ? "disabled" : ""}>${label}</button>`;
  const body = `<div class="binding-intro"><span class="binding-intro-icon">${icon("chat")}</span><div><strong>为已有对话指定角色</strong><p>绑定后，对话将使用所选角色的设定继续交流。已有角色绑定会被替换。</p></div></div><div class="binding-controls">${target}<label class="field binding-search"><span class="field-label">搜索对话或当前角色</span><input data-binding-query aria-label="搜索对话或当前角色" value="${escapeHtml(dialog.query)}" ${dialog.busy || dialog.loading ? "disabled" : ""}></label><div class="binding-list-toolbar"><div class="chips">${filter("all", "全部对话")}${filter("unbound", "未绑定")}</div>${button("binding-select-visible", allSelected ? "取消全选" : "全选结果", "", selectable.length === 0 || dialog.loading || dialog.busy || dialog.cardId === "" ? "disabled" : "", "small")}</div></div><div class="binding-chat-list" aria-label="选择要绑定的对话" aria-busy="${dialog.loading}">${dialog.loading ? '<div class="binding-empty" role="status">正在读取对话…</div>' : visible.length === 0 ? `<div class="binding-empty">${icon("chat")}<strong>${dialog.chats.length === 0 ? "还没有对话" : "没有符合条件的对话"}</strong><span>${dialog.chats.length === 0 ? "开始一段对话后，即可在这里绑定角色。" : "试试其他关键词，或切换到全部对话。"}</span></div>` : rows}</div>`;
  const status = dialog.busy ? `正在绑定 ${dialog.completed} / ${dialog.total}` : `已选择 ${dialog.selectedIds.length} 条${dialog.query !== "" || dialog.filter !== "all" ? ` · 筛选出 ${visible.length} 条` : ` · 共 ${dialog.chats.length} 条`}`;
  return { title: "批量绑定对话", style: "binding-dialog", tabs: [], body,
    footer: `<span class="binding-selection-count" role="status" aria-live="polite">${status}</span><div class="binding-footer-actions">${button("close-dialog", "取消", "", dialog.busy ? "disabled" : "")}${button("binding-submit", dialog.busy ? "绑定中…" : `绑定${dialog.selectedIds.length > 0 ? " " + dialog.selectedIds.length + " 条对话" : "对话"}`, "chat", dialog.loading || dialog.busy || dialog.selectedIds.length === 0 || dialog.cardId === "" ? "disabled" : "", "filled")}</div>` };
}

/** Owns draft selection and commits each binding through the existing settings-aware service. */
export function createChatBindingFeature(context: EditorContext): ActionFeature {
  const { snapshot, pushDialog, topDialog, request, renderDialogs, popDialog, toast } = context;
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    if (action === "bind-chats" || action === "bind-card-chats") {
      const cardId = action === "bind-card-chats" ? getCard(snapshot(), dataValue(element, "id")).id : "";
      const dialog = pushDialog({ type: "chat-binding", cardId, query: "", filter: "all", loading: true, chats: [], selectedIds: [], failures: {}, completed: 0, total: 0 });
      try { dialog.chats = await request({ action: "listBindableChats" }, { background: true }); }
      finally { dialog.loading = false; renderDialogs(); }
      return;
    }
    const dialog = topDialog("chat-binding");
    if (dialog.busy || dialog.loading) return;
    switch (action) {
      case "binding-filter": {
        const filter = dataValue(element, "filter");
        if (filter !== "all" && filter !== "unbound") throw new Error("无效的对话筛选条件");
        dialog.filter = filter; return;
      }
      case "binding-select-visible": {
        if (dialog.cardId === "") return;
        const ids = selectableBindingChats(dialog, snapshot()).map(chat => chat.id);
        const selected = new Set(dialog.selectedIds), allSelected = ids.length > 0 && ids.every(id => selected.has(id));
        for (const id of ids) { if (allSelected) selected.delete(id); else selected.add(id); }
        dialog.selectedIds = [...selected]; return;
      }
      case "binding-submit": {
        getCard(snapshot(), dialog.cardId);
        const selection = "card:" + dialog.cardId;
        // Freeze the explicit draft before starting; filter changes never alter this submission.
        const ids = dialog.selectedIds.filter(id => dialog.chats.some(chat => chat.id === id && chat.selection !== selection));
        if (ids.length === 0) return;
        dialog.busy = true; dialog.total = ids.length; dialog.completed = 0; dialog.failures = {};
        renderDialogs();
        let succeeded = 0;
        try {
          for (const chatId of ids) {
            try {
              const binding = await request({ action: "writeChatBinding", chatId, selection }, { background: true });
              if (binding.chatId !== chatId || binding.selection !== selection) throw new Error("绑定结果不一致，请刷新后重试");
              const chat = dialog.chats.find(value => value.id === chatId)!;
              chat.selection = selection;
              dialog.selectedIds = dialog.selectedIds.filter(id => id !== chatId);
              succeeded++;
            } catch (error) { dialog.failures[chatId] = String(error); }
            dialog.completed++; renderDialogs();
          }
        } finally { dialog.busy = false; }
        if (succeeded === ids.length) { popDialog(); toast(`已将 ${succeeded} 条对话绑定到「${getCard(snapshot(), dialog.cardId).name}」`); }
        else {
          dialog.error = `已绑定 ${succeeded} 条，${ids.length - succeeded} 条未完成。保留了未完成的选择，可重试。`;
          renderDialogs();
        }
        return;
      }
      default: throw new Error("Unknown chat binding action: " + action);
    }
  }
  return { names: ["bind-chats", "bind-card-chats", "binding-filter", "binding-select-visible", "binding-submit"], handleAction };
}

/** Keeps search caret and keyboard focus stable while updating the filtered list. */
export function inputChatBinding(context: EditorContext, element: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement): boolean {
  if (!element.hasAttribute("data-binding-query") && element.dataset.field !== "bindingCard") return false;
  const dialog = context.topDialog("chat-binding");
  if (context.state.busy || dialog.busy || dialog.loading) return true;
  if (element.hasAttribute("data-binding-query") && element instanceof HTMLInputElement) {
    const start = element.selectionStart, end = element.selectionEnd;
    dialog.query = element.value; context.renderDialogs();
    const next = context.dialogsRoot.querySelector<HTMLInputElement>("[data-binding-query]");
    if (next !== null) { next.focus(); next.setSelectionRange(start, end); }
  } else {
    dialog.cardId = element.value;
    dialog.selectedIds = dialog.selectedIds.filter(id => dialog.chats.some(chat => chat.id === id && chat.selection !== "card:" + dialog.cardId));
    dialog.failures = {}; dialog.error = ""; context.renderDialogs();
    context.dialogsRoot.querySelector<HTMLSelectElement>('[data-field="bindingCard"]')?.focus();
  }
  return true;
}

/** Native checkboxes provide full-row pointer and keyboard selection without click interception. */
export function changeChatBinding(context: EditorContext, element: HTMLInputElement | HTMLSelectElement): boolean {
  if (!(element instanceof HTMLInputElement) || !element.hasAttribute("data-binding-chat")) return false;
  const dialog = context.topDialog("chat-binding"), id = dataValue(element, "bindingChat");
  if (context.state.busy || dialog.busy || dialog.loading || dialog.cardId === "") return true;
  if (!dialog.chats.some(chat => chat.id === id && chat.selection !== "card:" + dialog.cardId)) return true;
  dialog.selectedIds = element.checked ? [...new Set([...dialog.selectedIds, id])] : dialog.selectedIds.filter(value => value !== id);
  context.renderDialogs();
  context.dialogsRoot.querySelectorAll<HTMLInputElement>("[data-binding-chat]").forEach(input => { if (input.dataset.bindingChat === id) input.focus(); });
  return true;
}
