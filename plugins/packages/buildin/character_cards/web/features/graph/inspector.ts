import type { GraphDialog } from "../../bridge/contracts";
import { button, escapeHtml, iconButton, notice } from "../../shared/ui/html";
import { graphData, graphItems, selectedMemory } from "./data";
/** Builds the graph inspector from loaded memory records and typed relationship endpoints. */
export function graphInspector(dialog: GraphDialog): string {
  let list = "";
  const items = graphItems(dialog);
  for (const item of items) list += `<button type="button" class="memory-item ${dialog.selected === item.uuid ? "selected" : ""}" data-action="select-memory" data-id="${escapeHtml(item.uuid)}"><strong>${escapeHtml(item.title)}</strong><small>${escapeHtml(item.content.slice(0, 70))}</small></button>`;
  const selected = selectedMemory(dialog);
  let detail = "";
  if (selected !== null) {
    let relationships = "";
    for (const edge of graphData(dialog).graph.edges) {
      if (edge.sourceId !== selected.uuid && edge.targetId !== selected.uuid) continue;
      relationships += `<div class="row"><span class="grow muted">${escapeHtml(edge.label === null ? "" : edge.label)} · ${edge.weight.toFixed(2)}</span>${iconButton("delete-link", "删除关系", "trash", `data-link="${escapeHtml(edge.id)}"`)}</div>`;
    }
    detail = `<div class="graph-detail"><h3>${escapeHtml(selected.title)}</h3><div class="memory-metadata"><span>类型 ${escapeHtml(selected.contentType)}</span><span>来源 ${escapeHtml(selected.source)}</span><span>可信度 ${selected.credibility.toFixed(2)}</span><span>重要性 ${selected.importance.toFixed(2)}</span></div><pre>${escapeHtml(selected.content)}</pre><div class="actions">${button("edit-memory", "编辑", "edit", "", "small")}${button("delete-memory", "删除", "trash", "", "small danger")}</div>${relationships}</div>`;
  }
  if (dialog.selectedEdge !== null) {
    const edge = graphData(dialog).graph.edges.find(
      /** Resolves the selected relationship by its exact persisted link id. */
      item => item.id === dialog.selectedEdge,
    );
    if (edge !== undefined) {
      const completeRecord = edge.label !== null && typeof edge.metadata.description === "string";
      const stage = completeRecord ? "" : notice("图谱关系已读取；完整关系记录尚未接入，当前不能保存关系编辑。");
      detail = `<div class="graph-detail"><h3>记忆关系</h3><p>${escapeHtml(edge.label === null ? "" : edge.label)}</p><p>强度 ${edge.weight.toFixed(2)}</p>${stage}${button("edit-link", "编辑关系", "edit", `data-link="${escapeHtml(edge.id)}"`, "small")}${button("delete-link", "删除关系", "trash", `data-link="${escapeHtml(edge.id)}"`, "danger")}</div>`;
    }
  }
  return `<h3>记忆 · ${items.length}</h3>${list === "" ? '<div class="empty">没有匹配的记忆</div>' : list}${detail}`;
}
