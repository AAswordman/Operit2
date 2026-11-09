import type { EditorContext } from "../../bridge/context";
import { host } from "../../bridge/transport";
/** Binds characters transfers operations to one editor context. */
export function createCharactersTransfersFeature(context: EditorContext) {
  const { state, topDialog, pushDialog, popDialog, request, renderMain, renderDialogs, toast, owningGraph } = context;
  /** Executes only this feature's registered actions without choosing another transport. */
  async function handleAction(action: string, _element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "save-export": {
        const dialog = topDialog("export");
        const result = await host().exportFile(dialog.name, dialog.content);
        if (result.cancelled) return;
        popDialog(); toast("文件已导出"); return;
      }

      case "import-card":
      case "import-group": pushDialog({ type: "import", target: action === "import-card" ? "card" : "group", ownerKey: null, format: "operit", strategy: "SKIP", content: "", stage: "" }); return;

      case "read-import": {
        const dialog = topDialog("import");
        if (dialog.target === "card") state.snapshot = await request({ action: "importCharacter", format: dialog.format, content: dialog.content });
        else if (dialog.target === "group") state.snapshot = await request({ action: "importGroup", content: dialog.content });
        else {
          if (dialog.ownerKey === null) throw new Error("缺少导入记忆库标识");
          dialog.busy = true;
          try {
            dialog.stage = "正在选择并读取 JSON 文件…"; renderDialogs();
            const file = await host().readImportFile();
            if (file.cancelled) return;
            dialog.stage = "正在导入记忆；备份启用云端向量时，需要等待向量生成…"; renderDialogs();
            const imported = await request({ action: "importMemory", ownerKey: dialog.ownerKey, content: file.content, strategy: dialog.strategy }, { background: true });
            dialog.stage = "记忆已导入，正在刷新图谱…"; renderDialogs();
            const graph = owningGraph(dialog.ownerKey);
            graph.data = await request({ action: "graph", ownerKey: dialog.ownerKey }, { background: true });
            graph.searchIds = null; graph.query = ""; graph.appliedQuery = ""; graph.selected = null; graph.selectedEdge = null;
            popDialog(); toast(`记忆导入完成：新增 ${imported.result.newMemories}，更新 ${imported.result.updatedMemories}，跳过 ${imported.result.skippedMemories}，关系 ${imported.result.newLinks}`); return;
          } finally { dialog.busy = false; dialog.stage = ""; renderDialogs(); }
        }
        popDialog(); renderMain(); toast("导入完成"); return;
      }
      default: throw new Error(`Unknown characters/transfers action: ${action}`);
    }
  }
  return { names: ["save-export", "import-card", "import-group", "read-import"], handleAction };
}
