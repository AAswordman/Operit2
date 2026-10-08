import type { SidebarViewState } from "./contracts";
import { escapeHtml, iconButton } from "../../shared/ui/html";
import { charactersSidebar } from "./characters/view";
import { groupsSidebar } from "./groups/view";
import { sidebarGroupDialog } from "./groups/dialogs";

/** Paints an explicit embedded sidebar view without management controls or modal presentation completion. */
export function sidebarView(state: SidebarViewState): string {
  const title = state.current.input.view === "characters" ? "角色分类" : "会话群组";
  let content: string;
  if (state.data === null) content = '<p role="status" class="sidebar-loading">正在读取插件侧边栏…</p>';
  else {
    if (state.data.view !== state.current.input.view) throw new Error("Sidebar projection does not match its explicit route input");
    content = state.data.view === "characters" ? charactersSidebar(state.data.sections, state) : groupsSidebar(state.data.scopes, state);
  }
  return '<div class="sidebar-view' + (state.busy ? " busy" : "") + '"><header class="sidebar-toolbar"><h1>' + title + '</h1>' + iconButton("sidebar-refresh", "刷新侧边栏", "refresh") + '</header><label class="sidebar-search"><span>搜索对话或分组</span><input type="search" aria-label="搜索对话或分组" data-sidebar-query value="' + escapeHtml(state.query) + '"></label>' + (state.error === "" ? "" : '<div class="sidebar-error" role="alert">' + escapeHtml(state.error) + '</div>') + '<div class="sidebar-scroll">' + content + "</div></div>" + sidebarGroupDialog(state);
}
