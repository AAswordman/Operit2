import type { Card } from "../../../src/model";
import type { EditorContext } from "../../bridge/context";
import type { NativeRecord } from "../../bridge/contracts";
import { host } from "../../bridge/transport";
import { getCard as readCard } from "../characters/data";
/** Binds characters transfers operations to one editor context. */
export function createCharactersTransfersFeature(context: EditorContext) {
  const { snapshot, state, topDialog, pushDialog, popDialog, request, renderMain, toast } = context;
  /** Resolves one loaded character by exact canonical identity. */
  const getCard = (id: string): Card => readCard(snapshot(), id);

  /** Serializes native character/group records without a plugin-only envelope. */
  function openExport(type: "character" | "group", value: NativeRecord): void {
    pushDialog({ type: "export", path: "", content: JSON.stringify(value, null, 2), typeName: type });
  }
  /** Executes only this feature's registered actions without choosing another transport. */
  async function handleAction(action: string, _element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "export-card": openExport("character", getCard(topDialog("character").card.id)); return;

      case "export-group": {
        const id = topDialog("group").group.id;
        for (const group of snapshot().groups) if (group.id === id) { openExport("group", group); return; }
        throw new Error("群组不存在");
      }

      case "save-export": await host().exportFile(topDialog("export").path, topDialog("export").content); popDialog(); toast("文件已导出"); return;

      case "import-card":
      case "import-group": pushDialog({ type: "import", target: action === "import-card" ? "card" : "group", content: "" }); return;

      case "read-import": {
        const dialog = topDialog("import");
        if (dialog.target === "card") state.snapshot = await request({ action: "importCharacter", format: "operit", content: dialog.content });
        else state.snapshot = await request({ action: "importGroup", content: dialog.content });
        popDialog(); renderMain(); toast("导入完成"); return;
      }
      default: throw new Error(`Unknown characters/transfers action: ${action}`);
    }
  }
  return { names: ["export-card", "export-group", "save-export", "import-card", "import-group", "read-import"], handleAction, openExport };
}
