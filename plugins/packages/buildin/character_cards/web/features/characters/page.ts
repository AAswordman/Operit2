import type { EditorContext } from "../../bridge/context";
import { button, escapeHtml, icon, iconButton } from "../../shared/ui/html";
import { groupTile } from "../groups/view";
import { cardTile } from "./tiles";
/** Composes the single role-card page from separate card and group tile renderers. */
export function createPageRenderer(context: EditorContext) {
  const { state, app, snapshot } = context;
  /** Renders the only top-level plugin page; memories stay inside character workflows. */
  function renderMain() {
    if (state.snapshot === null) return;
    let cards = "", groups = "";
    for (const card of snapshot().cards) cards += cardTile(card, snapshot());
    for (const group of snapshot().groups) groups += groupTile(group, snapshot());
    app.innerHTML = `<div class="page"><header class="appbar">${icon("badge")}<h1>角色卡</h1>${iconButton("memory-settings", "记忆设置", "tune")}${iconButton("refresh", "刷新", "refresh")}</header><div class="scroll">${state.error === "" ? "" : `<div class="error-banner" role="alert">${escapeHtml(state.error)}</div>`}<section class="section"><div class="section-head"><span class="section-icon">${icon("badge")}</span><h2>角色卡</h2><div class="actions">${button("import-card", "导入", "upload", "", "small")}${button("create-card", "创建", "add", "", "small")}</div></div>${cards === "" ? '<div class="empty">还没有角色卡，点击创建添加角色。</div>' : cards}</section><section class="section"><div class="section-head"><span class="section-icon">${icon("groups")}</span><h2>群组</h2><div class="actions">${button("import-group", "导入 JSON", "upload", "", "small")}${button("create-group", "创建", "add", "", "small")}</div></div>${groups === "" ? '<div class="empty">还没有群组，创建后可编排多个角色参与对话。</div>' : groups}</section></div></div>`;
  }
  return { renderMain };
}
