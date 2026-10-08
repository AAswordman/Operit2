import type { SidebarGroupScopeView, SidebarGroupView, SidebarViewState } from "../contracts";
import { escapeHtml, icon, iconButton } from "../../../shared/ui/html";
import { matchesSidebarChat, sidebarChatList } from "../chat-list/view";

/** Paints a persisted conversation-group projection with its full count and independent preview expansion. */
function groupView(group: SidebarGroupView, scope: SidebarGroupScopeView, state: SidebarViewState): string {
  const expanded = state.expandedGroups.has(group.id), query = state.query.trim().toLocaleLowerCase();
  const chats = group.chats.filter(
    /** Filters generic visible metadata, not group ownership or legacy field names. */
    chat => matchesSidebarChat(chat, query),
  );
  if (query !== "" && group.name.toLocaleLowerCase().indexOf(query) === -1 && chats.length === 0) return "";
  const attrs = 'data-group-id="' + escapeHtml(group.id) + '"';
  let html = '<section class="sidebar-conversation-group" data-group-id="' + escapeHtml(group.id) + '"><div class="sidebar-group-header"><strong>' + escapeHtml(group.name) + '</strong><span class="sidebar-count">' + group.chats.length + '</span>' + (group.pinned ? icon("pin") : "") + '<details class="sidebar-group-menu"><summary aria-label="分组操作">' + icon("more") + '</summary><div class="sidebar-menu-items">' + iconButton("sidebar-rename-group", "重命名分组", "edit", attrs) + iconButton("sidebar-pin-group", group.pinned ? "取消置顶分组" : "置顶分组", "pin", attrs) + iconButton("sidebar-group-up", "上移分组", "up", attrs) + iconButton("sidebar-group-down", "下移分组", "down", attrs) + iconButton("sidebar-delete-group", "删除分组及全部成员对话", "trash", attrs) + '</div></details>' + iconButton("sidebar-create-chat", "新建分组对话", "add", 'data-section-id="' + escapeHtml(scope.id) + '" ' + attrs) + "</div>";
  html += chats.length === 0 ? '<p class="sidebar-empty-section">暂无对话</p>' : sidebarChatList(expanded || query !== "" ? chats : chats.slice(0, 3), state.current.chatSidebar);
  if (chats.length > 3 && query === "") html += '<button type="button" class="sidebar-show-more" data-action="sidebar-toggle-group" ' + attrs + ' aria-expanded="' + (expanded ? "true" : "false") + '">' + (expanded ? "收起" : "显示其余 " + (chats.length - 3) + " 条对话") + "</button>";
  return html + "</section>";
}
/** Paints an owner-scoped grouping section so legal same-name groups never collapse into one category. */
function scopeView(scope: SidebarGroupScopeView, state: SidebarViewState): string {
  const collapsed = state.collapsedSections.has(scope.id);
  let html = '<section class="sidebar-section" data-scope-id="' + escapeHtml(scope.id) + '"><div class="sidebar-section-header"><button type="button" class="sidebar-section-toggle" data-action="sidebar-toggle-section" data-section-id="' + escapeHtml(scope.id) + '" aria-expanded="' + (collapsed ? "false" : "true") + '">' + icon("folder") + '<span>' + escapeHtml(scope.title) + '</span>' + icon(collapsed ? "down" : "up") + '</button>' + iconButton("sidebar-create-group", "新建分组", "add", 'data-scope-id="' + escapeHtml(scope.id) + '"') + "</div>";
  if (!collapsed) {
    for (const group of scope.groups) html += groupView(group, scope, state);
    const chats = scope.ungrouped.filter(
      /** Searches only the explicitly ungrouped native summaries in this exact scope. */
      chat => matchesSidebarChat(chat, state.query),
    );
    if (chats.length !== 0) html += '<section class="sidebar-conversation-group"><div class="sidebar-group-header"><strong>未分组</strong><span class="sidebar-count">' + chats.length + '</span></div>' + sidebarChatList(chats, state.current.chatSidebar) + "</section>";
  }
  return html + "</section>";
}
/** Renders complete scoped groups supplied by the real projection, including persisted empty groups. */
export function groupsSidebar(scopes: readonly SidebarGroupScopeView[], state: SidebarViewState): string {
  let html = "";
  for (const scope of scopes) html += scopeView(scope, state);
  return html === "" ? '<p class="sidebar-empty-section">暂无会话分组</p>' : html;
}
