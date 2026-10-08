import type { Card } from "../../../src/model";
import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { copy } from "../../shared/ui/html";
import { validateBindings } from "./bindings/data";
import { newCard, ownerFor, getCard as readCard } from "../characters/data";
/** Binds characters actions operations to one editor context. */
export function createCharactersActionsFeature(context: EditorContext) {
  const { snapshot, state, topDialog, pushDialog, popDialog, request, renderMain, renderDialogs, toast, openUser, openGraph } = context;
  /** Resolves one loaded character by exact canonical identity. */
  const getCard = (id: string): Card => readCard(snapshot(), id);


  /** Executes only this feature's registered actions without choosing another transport. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "create-card": pushDialog({ type: "character", card: newCard(), create: true, tab: 0 }); return;

      case "edit-card": pushDialog({ type: "character", card: copy(getCard(dataValue(element, "id"))), create: false, tab: 0 }); return;

      case "activate-card": state.snapshot = await request({ action: "activate", type: "card", id: dataValue(element, "id") }); renderMain(); toast("已切换角色卡"); return;

      case "card-user": { const card = getCard(dataValue(element, "id")); await openUser(ownerFor(card), card.name); return; }

      case "card-graph": { const card = getCard(dataValue(element, "id")); await openGraph(ownerFor(card), card.name); return; }

      case "save-card": {
        const dialog = topDialog("character");
        if (dialog.card.name.trim() === "") throw new Error("角色名称不能为空");
        validateBindings(dialog, snapshot());
        state.snapshot = await request({ action: "saveCharacter", card: dialog.card, create: dialog.create, tagChanges: dialog.tagChanges });
        await context.entityChanged("card", dialog.card.id, "saved");
        popDialog(); renderMain(); toast("角色卡已保存"); return;
      }

      case "clear-avatar": topDialog("character").card.avatarUri = null; renderDialogs(); return;

      case "delete-card": pushDialog({ type: "confirm", title: "删除角色卡", message: `确定删除“${topDialog("character").card.name}”？`, operation: { action: "deleteCharacter", id: topDialog("character").card.id }, closeParent: true }); return;

      case "show-tool-access": { const parent = topDialog("character"); pushDialog({ type: "tool-access", parent, draft: copy(parent.card.toolAccessConfig) }); return; }
      default: throw new Error(`Unknown characters/actions action: ${action}`);
    }
  }
  return { names: ["create-card", "edit-card", "activate-card", "card-user", "card-graph", "save-card", "clear-avatar", "delete-card", "show-tool-access"], handleAction };
}
