import type { ChatSidebarContext, SidebarChatSummary } from "../../../../src/ui-sidebar";
import { escapeHtml, icon, iconButton } from "../../../shared/ui/html";

/** Searches visible text only; it does not infer identity, ownership, scope, or command syntax from substrings. */
export function matchesSidebarChat(chat: SidebarChatSummary, query: string): boolean {
  const search = query.trim().toLocaleLowerCase();
  return search === "" || chat.title.toLocaleLowerCase().indexOf(search) !== -1 || chat.id.toLocaleLowerCase().indexOf(search) !== -1;
}
/** Renders an actual neutral host chat with precise active, lock, pin, and streaming states. */
export function sidebarChatRow(chat: SidebarChatSummary, context: ChatSidebarContext): string {
  const active = context.currentChatId === chat.id, streaming = context.activeStreamingChatIds.indexOf(chat.id) !== -1;
  const metadata = (chat.pinned ? icon("pin") : "") + (chat.locked ? icon("lock") : "");
  const status = streaming ? '<span class="sidebar-streaming" role="status" aria-label="正在生成"></span>' : "";
  const move = iconButton("sidebar-move-chat", "移动对话", "move", 'data-chat-id="' + escapeHtml(chat.id) + '"');
  return '<li class="sidebar-chat-row' + (active ? " selected" : "") + '" data-chat-id="' + escapeHtml(chat.id) + '"><button type="button" class="sidebar-chat-main" data-action="sidebar-activate-chat" data-chat-id="' + escapeHtml(chat.id) + '" aria-current="' + (active ? "true" : "false") + '">' + icon("chat") + '<span class="sidebar-chat-text"><span class="sidebar-chat-title">' + escapeHtml(chat.title) + '</span><span class="sidebar-chat-updated">' + escapeHtml(chat.updatedAt) + '</span></span><span class="sidebar-chat-status">' + metadata + status + '</span></button>' + move + '</li>';
}
/** Paints a complete requested range from real host summaries without inventing missing member records. */
export function sidebarChatList(chats: readonly SidebarChatSummary[], context: ChatSidebarContext): string {
  let html = '<ul class="sidebar-chat-list">';
  for (const chat of chats) html += sidebarChatRow(chat, context);
  return html + "</ul>";
}
