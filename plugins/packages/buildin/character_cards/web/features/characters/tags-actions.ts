import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { copy } from "../../shared/ui/html";
import { tagById } from "../characters/tags-view";
/** Binds characters tags-actions operations to one editor context. */
export function createCharactersTagsActionsFeature(context: EditorContext) {
  const { topDialog, pushDialog, popDialog, renderDialogs } = context;


  /** Executes only this feature's registered actions without choosing another transport. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "toggle-tag": {
        const id = dataValue(element, "id");
        const draft = topDialog("character").card;
        const index = draft.attachedTagIds.indexOf(id);
        if (index < 0) draft.attachedTagIds.push(id); else draft.attachedTagIds.splice(index, 1);
        renderDialogs(); return;
      }

      case "show-tags": pushDialog({ type: "tags", parent: topDialog("character") }); return;

      case "create-tag": pushDialog({ type: "tag-edit", parent: topDialog("tags").parent, create: true, draft: { id: "", name: "", description: "", promptContent: "", tagType: "CUSTOM" } }); return;

      case "edit-tag": {
        const parent = topDialog("tags").parent, tag = tagById(parent, dataValue(element, "id"));
        pushDialog({ type: "tag-edit", parent, create: false, draft: copy(tag) }); return;
      }

      case "save-tag": {
        const dialog = topDialog("tag-edit"), parent = dialog.parent, tag = dialog.draft;
        if (tag.name.trim() === "") throw new Error("标签名称不能为空");
        tag.name = tag.name.trim(); tag.description = tag.description.trim();
        if (dialog.create) {
          tag.id = `draft_prompt_tag_${++parent.nextTagId}`;
          parent.tags.push(copy(tag)); parent.card.attachedTagIds.push(tag.id);
          parent.tagChanges.created.push({ draftId: tag.id, values: copy(tag) });
        } else {
          const index = parent.tags.indexOf(tagById(parent, tag.id)); parent.tags[index] = copy(tag);
          const created = parent.tagChanges.created.find(
            /** Finds the staged creation belonging to this exact tag id. */
            item => item.draftId === tag.id,
          );
          if (created !== undefined) created.values = copy(tag);
          else {
            parent.tagChanges.updated = parent.tagChanges.updated.filter(
              /** Removes the previous staged edit of this same persisted tag. */
              item => item.id !== tag.id,
            );
            parent.tagChanges.updated.push(copy(tag));
          }
        }
        popDialog(); return;
      }

      case "delete-tag": {
        const parent = topDialog("tags").parent, tag = tagById(parent, dataValue(element, "id"));
        pushDialog({ type: "tag-delete", parent, tagId: dataValue(element, "id"), name: tag.name }); return;
      }

      case "confirm-tag-delete": {
        const dialog = topDialog("tag-delete"), parent = dialog.parent, tag = tagById(parent, dialog.tagId);
        parent.tags.splice(parent.tags.indexOf(tag), 1);
        parent.card.attachedTagIds = parent.card.attachedTagIds.filter(
          /** Unselects the deleted tag from this character draft. */
          id => id !== tag.id,
        );
        const index = parent.tagChanges.created.findIndex(
          /** Checks whether deletion cancels a tag that has not been created in Core. */
          item => item.draftId === tag.id,
        );
        if (index >= 0) parent.tagChanges.created.splice(index, 1);
        else {
          parent.tagChanges.updated = parent.tagChanges.updated.filter(
            /** Discards an edit superseded by this persisted tag deletion. */
            item => item.id !== tag.id,
          );
          parent.tagChanges.deleted.push(tag.id);
        }
        popDialog(); return;
      }
      default: throw new Error(`Unknown characters/tags-actions action: ${action}`);
    }
  }
  return { names: ["toggle-tag", "show-tags", "create-tag", "edit-tag", "save-tag", "delete-tag", "confirm-tag-delete"], handleAction };
}
