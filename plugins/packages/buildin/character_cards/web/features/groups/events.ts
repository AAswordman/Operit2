import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { renumberMembers } from "./data";

/** Updates explicit group member selection while preserving canonical card identities. */
export function changeGroupMember(context: EditorContext, element: HTMLInputElement): void {
  const group = context.topDialog("group").group;
  const id = dataValue(element, "member");
  if (element.checked) group.members.push({ characterCardId: id, orderIndex: group.members.length });
  else {
    const index = group.members.findIndex(
      /** Matches only the exact selected card identity. */
      member => member.characterCardId === id,
    );
    if (index < 0) throw new Error("群组成员选择状态不一致");
    group.members.splice(index, 1);
  }
  renumberMembers(group);
  context.renderDialogs();
}
