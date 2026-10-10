import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { copy } from "../../shared/ui/html";
/** Binds memory library-actions operations to one editor context. */
export function createMemoryLibraryActionsFeature(context: EditorContext) {
  const { snapshot, state, topDialog, pushDialog, popDialog, request, renderMain, toast } = context;


  /** Executes only this feature's registered actions without choosing another transport. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    const id = element.dataset.id;
    switch (action) {

      case "create-store": pushDialog({ type: "store", create: true, store: { id: "", name: "" } }); return;

      case "edit-store": {
        for (const store of snapshot().stores) if (store.id === id) { pushDialog({ type: "store", create: false, store: copy(store) }); return; }
        throw new Error("共享记忆库不存在");
      }

      case "save-store": {
        const dialog = topDialog("store");
        state.snapshot = await request({ action: "saveStore", store: dialog.store, create: dialog.create });
        popDialog(); renderMain(); toast("共享记忆库已保存"); return;
      }

      case "delete-store": pushDialog({ type: "confirm", title: "删除共享记忆库", message: "确定删除这个共享记忆库？此操作会影响关联角色。", operation: { action: "deleteStore", id: dataValue(element, "id") }, closeParent: false }); return;
      default: throw new Error(`Unknown memory/library-actions action: ${action}`);
    }
  }
  return { names: ["create-store", "edit-store", "save-store", "delete-store"], handleAction };
}
