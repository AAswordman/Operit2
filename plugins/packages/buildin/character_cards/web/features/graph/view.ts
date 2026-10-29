import type { DialogSpec, GraphDialog } from "../../bridge/contracts";
import { button, escapeHtml, icon, iconButton, notice } from "../../shared/ui/html";
import { graphSvg } from "./canvas";
import { graphData } from "./data";
import { graphInspector } from "./inspector";
/** Presents a Material app bar with secondary operations in an anchored overflow surface. */
export function graphHeader(dialog: GraphDialog): string {
  const disabled = dialog.loading ? "disabled" : "";
  return `${iconButton("close-dialog", "关闭", "back", dialog.busy ? "disabled" : "")}<div class="graph-heading"><h2>记忆图谱</h2><span title="${escapeHtml(dialog.name)}">${escapeHtml(dialog.name)}</span></div><details class="graph-overflow"><summary class="icon-button" role="button" aria-label="更多操作" title="更多操作">${icon("more")}</summary><div class="graph-menu" role="group" aria-label="记忆图谱操作">${button("owner-settings", "记忆设置", "tune", `data-owner="${escapeHtml(dialog.ownerKey)}" data-name="${escapeHtml(dialog.name)}"`)}${button("refresh-graph", "刷新", "refresh", disabled)}<hr>${button("import-graph", "导入记忆 JSON", "upload", disabled)}${button("export-graph", "导出记忆 JSON", "download", disabled)}</div></details>`;
}

/** Defines the graph surface and its inspector as one nested Material view. */
export function graphSpec(dialog: GraphDialog): DialogSpec {
  if (dialog.data === null) return { title: `${dialog.name} 的记忆图谱`, style: "graph-dialog", tabs: [], body: dialog.loading ? '<div class="loading">正在读取记忆图谱…</div>' : notice("记忆图谱未加载成功，未开放写入。"), footer: "" };
  const folders = new Set([""]);
  for (const item of graphData(dialog).items) folders.add(item.folderPath === null ? "" : item.folderPath);
  let folderOptions = `<option value="*" ${dialog.folder === "*" ? "selected" : ""}>全部文件夹</option>`;
  for (const folder of folders) folderOptions += `<option value="${escapeHtml(folder)}" ${dialog.folder === folder ? "selected" : ""}>${escapeHtml(folder === "" ? "根目录" : folder)}</option>`;
  const linkAttributes = `aria-pressed="${dialog.linkMode}" ${graphData(dialog).items.length < 2 ? "disabled" : ""}`;
  return {
    title: `${dialog.name} 的记忆图谱`, style: "graph-dialog", tabs: [],
    body: `<div class="graph-toolbar">
      <div class="graph-search" role="search">${iconButton("search-graph", "搜索", "search")}<input type="search" aria-label="搜索记忆" data-graph-search value="${escapeHtml(dialog.query)}" placeholder="搜索记忆…">${iconButton("clear-search", "清除搜索", "close")}</div>
      <div class="graph-filters"><label class="graph-folder">${icon("folder")}<select data-graph-folder aria-label="文件夹">${folderOptions}</select>${icon("down")}</label>${button("create-link", "链接", "link", linkAttributes, "graph-link-chip")}</div>
      ${dialog.linkMode ? '<div class="graph-link-hint" role="status">选择两条记忆以建立链接</div>' : ""}
    </div><div class="graph-layout"><div class="graph-canvas"><svg viewBox="-400 -260 800 520" data-graph-surface aria-label="记忆图谱画布">${graphSvg(dialog)}</svg>${graphData(dialog).items.length === 0 ? '<div class="graph-empty"><span>暂无记忆</span><span class="muted">创建记忆后，可在这里查看和连接节点。</span></div>' : ""}<div class="graph-tools" role="group" aria-label="画布缩放">${iconButton("zoom-in", "放大", "add")}${iconButton("zoom-out", "缩小", "minus")}${iconButton("fit-graph", "适应画布", "expand")}</div>${button("create-memory", "创建记忆", "add", "", "graph-fab")}</div><aside class="graph-sidebar" data-expanded="${dialog.inspectorExpanded}" aria-label="记忆列表与详情">${graphInspector(dialog)}</aside></div>`,
    footer: "",
  };
}
