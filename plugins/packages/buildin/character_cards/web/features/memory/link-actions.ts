import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { graphData } from "../graph/data";
/** Binds memory link-actions operations to one editor context. */
export function createMemoryLinkActionsFeature(context: EditorContext) {
  const { topDialog, pushDialog, popDialog, request, renderDialogs, toast, owningGraph } = context;


  /** Executes only this feature's registered actions without choosing another transport. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "create-link": {
        const graph = topDialog("graph");
        graph.linkMode = !graph.linkMode; graph.linkSource = null;
        renderDialogs(); toast(graph.linkMode ? "先选择源记忆，再选择目标记忆" : "已退出链接模式"); return;
      }

      case "save-link": {
        const dialog = topDialog("link"), weight = Number(dialog.draft.weight);
        dialog.draft.linkType = dialog.draft.linkType.trim(); dialog.draft.description = dialog.draft.description.trim();
        if (dialog.draft.linkType === "") throw new Error("请输入关系类型");
        if (dialog.draft.sourceTitle === "" || dialog.draft.targetTitle === "" || dialog.draft.sourceTitle === dialog.draft.targetTitle) throw new Error("请选择两条不同的记忆");
        if (!Number.isFinite(weight) || weight < 0 || weight > 1) throw new Error("关系强度必须在 0 到 1 之间");
        const graph = owningGraph(dialog.ownerKey); graph.data = await request({ action: "createLink", ownerKey: dialog.ownerKey, ...dialog.draft, weight }); graph.linkSource = null; popDialog(); toast("关系已创建"); return;
      }

      case "edit-link": {
        const graph = topDialog("graph"), edge = graphData(graph).graph.edges.find(
          /** Resolves the selected persisted relationship by its stable id. */
          item => item.id === dataValue(element, "link"),
        );
        if (edge === undefined) throw new Error("记忆关系不存在");
        const source = graphData(graph).items.find(
          /** Resolves the relationship source title from its UUID endpoint. */
          item => item.uuid === edge.sourceId,
        );
        const target = graphData(graph).items.find(
          /** Resolves the relationship target title from its UUID endpoint. */
          item => item.uuid === edge.targetId,
        );
        if (source === undefined || target === undefined) throw new Error("记忆关系端点不存在");
        if (edge.label === null || typeof edge.metadata.description !== "string") throw new Error("插件未返回完整关系记录，不能用图谱边覆盖关系描述");
        pushDialog({ type: "link-edit", ownerKey: graph.ownerKey, linkId: edge.id, draft: { sourceTitle: source.title, targetTitle: target.title, linkType: edge.label, weight: edge.weight, description: edge.metadata.description } }); return;
      }

      case "save-link-edit": {
        const dialog = topDialog("link-edit"), weight = Number(dialog.draft.weight);
        if (dialog.draft.linkType.trim() === "" || !Number.isFinite(weight) || weight < 0 || weight > 1) throw new Error("请输入关系类型和有效的关系强度");
        const graph = owningGraph(dialog.ownerKey);
        graph.data = await request({ action: "updateLink", ownerKey: dialog.ownerKey, linkId: dialog.linkId, linkType: dialog.draft.linkType.trim(), weight, description: dialog.draft.description.trim() });
        popDialog(); toast("关系已保存"); return;
      }

      case "delete-link": pushDialog({ type: "confirm", title: "删除关系", message: "确定删除这条记忆关系？", operation: { action: "deleteLink", ownerKey: topDialog("graph").ownerKey, linkId: dataValue(element, "link") }, closeParent: false }); return;
      default: throw new Error(`Unknown memory/link-actions action: ${action}`);
    }
  }
  return { names: ["create-link", "save-link", "edit-link", "save-link-edit", "delete-link"], handleAction };
}
