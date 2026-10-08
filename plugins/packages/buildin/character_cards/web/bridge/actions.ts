import { dataValue, requireElement } from "../shared/dom";
import { editableDialog, editableValue, setEditableValue } from "../shared/edits";
import type { EditorContext } from "./context";
/** Registers only stack, fullscreen-text and confirmation operations at the bridge boundary. */
export function createDialogActions(context: EditorContext) {
  const { state, topDialog, pushDialog, popDialog, request, renderMain, renderDialogs, toast, owningGraph } = context;
  /** Applies one exact non-business editor action. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "refresh": state.snapshot = await request({ action: "snapshot" }); state.error = ""; renderMain(); return;

      case "select-tab": {
        const dialog = topDialog();
        if (!("tab" in dialog)) throw new Error("此页面没有编辑页签");
        dialog.tab = Number(dataValue(element, "tab")); renderDialogs(); return;
      }

      case "close-dialog":
        if (topDialog().busy) return;
        await context.dismissDialog(); return;

      case "expand-field": {
        const parent = editableDialog(topDialog()), key = dataValue(element, "key");
        const field = requireElement(element.closest(".field"), HTMLElement);
        const labelElement = requireElement(field.querySelector(".field-label"), HTMLElement);
        if (labelElement.textContent === null) throw new Error("文字字段标题不存在");
        pushDialog({ type: "text", key, value: editableValue(parent, key), label: labelElement.textContent, parent }); return;
      }

      case "save-text": {
        const dialog = topDialog("text");
        setEditableValue(dialog.parent, dialog.key, dialog.value); popDialog(); return;
      }

      case "confirm-operation": {
        const dialog = topDialog("confirm"), operation = dialog.operation;
        if (operation.action === "deleteMemory" || operation.action === "deleteLink") owningGraph(operation.ownerKey).data = await request(operation);
        else { state.snapshot = await request(operation); renderMain(); }
        if (operation.action === "deleteCharacter") await context.entityChanged("card", operation.id, "deleted");
        if (operation.action === "deleteGroup") await context.entityChanged("group", operation.id, "deleted");
        state.dialogs.pop(); if (dialog.closeParent) state.dialogs.pop(); renderDialogs(); toast("已删除"); return;
      }
      default: throw new Error(`Unknown editor action: ${action}`);
    }
  }
  return { names: ["refresh", "select-tab", "close-dialog", "expand-field", "save-text", "confirm-operation"], handleAction };
}
