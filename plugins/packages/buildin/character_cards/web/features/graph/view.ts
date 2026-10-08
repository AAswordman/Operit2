import type { DialogSpec, GraphDialog } from "../../bridge/contracts";
import { button, escapeHtml, iconButton, notice } from "../../shared/ui/html";
import { graphSvg } from "./canvas";
import { graphData } from "./data";
import { graphInspector } from "./inspector";
/** Exposes the original graph header operations within the current character's dialog. */
export function graphHeader(dialog: GraphDialog): string {
  return `<div class="actions">${iconButton("owner-settings", "记忆设置", "tune", `data-owner="${escapeHtml(dialog.ownerKey)}" data-name="${escapeHtml(dialog.name)}"`)}${iconButton("export-graph", "导出记忆 JSON（备份接口待接入）", "download", "disabled")}${iconButton("refresh-graph", "刷新", "refresh", dialog.loading ? "disabled" : "")}</div>`;
}

/** Defines the graph surface and its inspector as one nested Material view. */
export function graphSpec(dialog: GraphDialog): DialogSpec {
  if (dialog.data === null) return { title: `${dialog.name} 的记忆图谱`, style: "graph-dialog", tabs: [], body: dialog.loading ? '<div class="loading">正在读取记忆图谱…</div>' : notice("记忆图谱未加载成功，未开放写入。"), footer: "" };
  const folders = new Set([""]);
  for (const item of graphData(dialog).items) folders.add(item.folderPath === null ? "" : item.folderPath);
  let folderOptions = `<option value="*" ${dialog.folder === "*" ? "selected" : ""}>全部文件夹</option>`;
  for (const folder of folders) folderOptions += `<option value="${escapeHtml(folder)}" ${dialog.folder === folder ? "selected" : ""}>${escapeHtml(folder === "" ? "根目录" : folder)}</option>`;
  return { title: `${dialog.name} 的记忆图谱`, style: "graph-dialog", tabs: [], body: `<div class="graph-toolbar"><input class="search" aria-label="搜索记忆" data-graph-search value="${escapeHtml(dialog.query)}" placeholder="搜索记忆…">${iconButton("search-graph", "搜索", "search")}${iconButton("clear-search", "清除搜索", "close")}<select class="search" style="flex:0;min-width:140px" data-graph-folder aria-label="文件夹">${folderOptions}</select>${button("create-link", "链接", "link", graphData(dialog).items.length < 2 ? "disabled" : "", dialog.linkMode ? "small tonal" : "small")}${button("create-memory", "创建记忆", "add", "", "small tonal")}</div><div class="graph-layout"><div class="graph-canvas"><svg viewBox="-400 -260 800 520" data-graph-surface aria-label="记忆图谱画布">${graphSvg(dialog)}</svg>${graphData(dialog).items.length === 0 ? '<div class="graph-empty"><span>暂无记忆</span><span class="muted">创建记忆后，可在这里查看和连接节点。</span></div>' : ""}<div class="graph-tools">${iconButton("zoom-in", "放大", "add")}${iconButton("zoom-out", "缩小", "minus")}${iconButton("fit-graph", "适应画布", "expand")}</div></div><aside class="graph-sidebar">${graphInspector(dialog)}</aside></div>`, footer: "" };
}
