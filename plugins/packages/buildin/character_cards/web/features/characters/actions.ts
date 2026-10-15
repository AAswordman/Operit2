import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { host } from "../../bridge/transport";
import { getCard, ownerFor } from "./data";
/** Entity editors live exclusively in the plugin's native DSL renderer. */
export function createCharactersActionsFeature(context: EditorContext) {
  const { snapshot, state, request, renderMain, toast, openUser, openGraph } = context;
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "create-card": await host().openEntity("create", "card", ""); return;
      case "edit-card": await host().openEntity("edit", "card", getCard(snapshot(), dataValue(element, "id")).id); return;
      case "activate-card": state.snapshot = await request({ action: "activate", type: "card", id: dataValue(element, "id") }); renderMain(); toast("已切换角色卡"); return;
      case "card-user": { const card = getCard(snapshot(), dataValue(element, "id")); await openUser(ownerFor(card), card.name); return; }
      case "card-graph": { const card = getCard(snapshot(), dataValue(element, "id")); await openGraph(ownerFor(card), card.name); return; }
      default: throw new Error(`Unknown characters action: ${action}`);
    }
  }
  return { names: ["create-card", "edit-card", "activate-card", "card-user", "card-graph"], handleAction };
}
