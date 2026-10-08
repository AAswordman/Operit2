import type { CharacterSidebarSection } from "../../../../src/ui-sidebar";
import type { SidebarViewState } from "../contracts";
import { escapeHtml, icon, iconButton } from "../../../shared/ui/html";
import { matchesSidebarChat, sidebarChatList } from "../chat-list/view";

/** Paints an actual persisted character or role-group section, including an explicitly empty category. */
function characterSection(section: CharacterSidebarSection, state: SidebarViewState): string {
  const collapsed = state.collapsedSections.has(section.id), chats = section.chats.filter(
    /** Filters only the visible generic chat title and identity. */
    chat => matchesSidebarChat(chat, state.query),
  );
  const nameMatches = section.title.toLocaleLowerCase().indexOf(state.query.trim().toLocaleLowerCase()) !== -1;
  if (state.query.trim() !== "" && !nameMatches && chats.length === 0) return "";
  const create = section.kind === "unbound" ? "" : iconButton("sidebar-create-chat", "新建对话", "add", 'data-section-id="' + escapeHtml(section.id) + '" data-group-id=""');
  let html = '<section class="sidebar-section" data-section-id="' + escapeHtml(section.id) + '"><div class="sidebar-section-header"><button type="button" class="sidebar-section-toggle" data-action="sidebar-toggle-section" data-section-id="' + escapeHtml(section.id) + '" aria-expanded="' + (collapsed ? "false" : "true") + '">' + icon(section.kind === "group" ? "groups" : "badge") + '<span>' + escapeHtml(section.title) + '</span><span class="sidebar-count">' + section.chats.length + '</span>' + icon(collapsed ? "down" : "up") + '</button>' + create + "</div>";
  if (!collapsed) html += chats.length === 0 ? '<p class="sidebar-empty-section">暂无对话</p>' : sidebarChatList(chats, state.current.chatSidebar);
  return html + "</section>";
}
/** Renders the independent character-classification tab without introducing manager-page tab buttons. */
export function charactersSidebar(sections: readonly CharacterSidebarSection[], state: SidebarViewState): string {
  let html = "";
  for (const section of sections) html += characterSection(section, state);
  return html === "" ? '<p class="sidebar-empty-section">没有匹配的角色或对话</p>' : html;
}
