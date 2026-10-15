import type { EditorContext } from "../../bridge/context";
import { selectedMemory } from "../graph/data";
/** Binds memory item-actions operations to one editor context. */
export function createMemoryItemActionsFeature(context: EditorContext) {
  const { topDialog, pushDialog, popDialog, request, toast, owningGraph } = context;


  /** Executes only this feature's registered actions without choosing another transport. */
  async function handleAction(action: string, _element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "create-memory": { const graph = topDialog("graph"); pushDialog({ type: "memory-edit", ownerKey: graph.ownerKey, originalTitle: null, draft: { title: "", content: "", contentType: "text/plain", source: "manual", credibility: 0.5, importance: 0.5, folderPath: graph.folder === "*" ? "" : graph.folder, tags: "" } }); return; }

      case "edit-memory": {
        const graph = topDialog("graph"), item = selectedMemory(graph);
        if (item === null) throw new Error("请先选择一条记忆");
        let tags = ""; for (const tag of item.tags) tags += `${tags === "" ? "" : ","}${tag.name}`;
        pushDialog({ type: "memory-edit", ownerKey: graph.ownerKey, originalTitle: item.title, draft: { title: item.title, content: item.content, contentType: item.contentType, source: item.source, credibility: item.credibility, importance: item.importance, folderPath: item.folderPath === null ? "" : item.folderPath, tags } }); return;
      }

      case "save-memory": {
        const dialog = topDialog("memory-edit");
        const data = await request({ action: "saveMemory", ownerKey: dialog.ownerKey, originalTitle: dialog.originalTitle, ...dialog.draft });
        owningGraph(dialog.ownerKey).data = data; popDialog(); toast("记忆已保存"); return;
      }

      case "delete-memory": {
        const graph = topDialog("graph"), item = selectedMemory(graph);
        if (item === null) throw new Error("请先选择一条记忆");
        pushDialog({ type: "confirm", title: "删除记忆", message: `确定删除“${item.title}”？`, operation: { action: "deleteMemory", ownerKey: graph.ownerKey, title: item.title }, closeParent: false }); return;
      }
      default: throw new Error(`Unknown memory/item-actions action: ${action}`);
    }
  }
  return { names: ["create-memory", "edit-memory", "save-memory", "delete-memory"], handleAction };
}
