import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { host } from "../../bridge/transport";
import { getGroup } from "./data";
export function createGroupsActionsFeature(context: EditorContext) {
  const { snapshot, state, request, renderMain, toast } = context;
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "activate-group": state.snapshot = await request({ action: "activate", type: "group", id: dataValue(element, "id") }); renderMain(); toast("已切换群组"); return;
      case "create-group": await host().openEntity("create", "group", ""); return;
      case "edit-group": await host().openEntity("edit", "group", getGroup(snapshot(), dataValue(element, "id")).id); return;
      default: throw new Error(`Unknown groups action: ${action}`);
    }
  }
  return { names: ["activate-group", "create-group", "edit-group"], handleAction };
}
