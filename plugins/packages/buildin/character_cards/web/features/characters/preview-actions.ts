import type { ActionFeature, EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { copy } from "../../shared/ui/html";
import { getGroup } from "../groups/data";
import { getCard } from "./data";

/** Opens saved entity previews and nests the original editors without cloning business operations. */
export function createPreviewFeature(context: EditorContext): ActionFeature {
  const { snapshot, pushDialog, topDialog } = context;
  /** Routes one exact entity preview operation. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "preview-card": {
        const card = getCard(snapshot(), dataValue(element, "id"));
        pushDialog({ type: "preview", entity: "card", id: card.id, tab: 0 }); return;
      }
      case "preview-group": {
        const group = getGroup(snapshot(), dataValue(element, "id"));
        pushDialog({ type: "preview", entity: "group", id: group.id, tab: 0 }); return;
      }
      case "preview-edit": {
        const dialog = topDialog("preview");
        switch (dialog.entity) {
          case "card": pushDialog({ type: "character", card: copy(getCard(snapshot(), dialog.id)), create: false, tab: 0 }); return;
          case "group": pushDialog({ type: "group", group: copy(getGroup(snapshot(), dialog.id)), create: false }); return;
        }
      }
      default: throw new Error(`Unknown entity preview action: ${action}`);
    }
  }
  return { names: ["preview-card", "preview-group", "preview-edit"], handleAction };
}
