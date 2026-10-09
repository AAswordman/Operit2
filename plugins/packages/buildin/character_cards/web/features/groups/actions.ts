import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { copy } from "../../shared/ui/html";
/** Binds groups actions operations to one editor context. */
export function createGroupsActionsFeature(context: EditorContext) {
  const { snapshot, state, topDialog, pushDialog, popDialog, request, renderMain, toast } = context;

  /** Executes only this feature's registered actions without choosing another transport. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    const id = element.dataset.id;
    switch (action) {
      case "activate-group": state.snapshot = await request({ action: "activate", type: "group", id: dataValue(element, "id") }); renderMain(); toast("已切换群组"); return;

      case "create-group": pushDialog({ type: "group", group: { id: "", name: "", description: "", members: [], themeConfigId: null }, create: true }); return;

      case "edit-group": {
        for (const group of snapshot().groups) if (group.id === id) { pushDialog({ type: "group", group: copy(group), create: false }); return; }
        throw new Error("群组不存在");
      }

      case "save-group": {
        const dialog = topDialog("group"), selected = new Set<string>();
        for (const member of dialog.group.members) selected.add(member.characterCardId);
        dialog.group.members = [];
        for (const card of snapshot().cards) if (selected.has(card.id)) dialog.group.members.push({ characterCardId: card.id, orderIndex: dialog.group.members.length });
        state.snapshot = await request({ action: "saveGroup", group: dialog.group, create: dialog.create });
        await context.entityChanged("group", dialog.group.id, "saved");
        popDialog(); renderMain(); toast("群组已保存"); return;
      }

      case "delete-group": pushDialog({ type: "confirm", title: "删除群组", message: `确定删除“${topDialog("group").group.name}”？`, operation: { action: "deleteGroup", id: topDialog("group").group.id }, closeParent: true }); return;

      default: throw new Error(`Unknown groups/actions action: ${action}`);
    }
  }
  return { names: ["activate-group", "create-group", "edit-group", "save-group", "delete-group"], handleAction };
}
