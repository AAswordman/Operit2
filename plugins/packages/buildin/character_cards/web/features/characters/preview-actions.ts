import type { ActionFeature, EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { host } from "../../bridge/transport";
import { getCard } from "./data";
import { getGroup } from "../groups/data";
/** Previews use the same native surface as avatar/sidebar entrypoints. */
export function createPreviewFeature(context: EditorContext): ActionFeature {
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    const id = dataValue(element, "id");
    if (action === "preview-card") { await host().openEntity("preview", "card", getCard(context.snapshot(), id).id); return; }
    if (action === "preview-group") { await host().openEntity("preview", "group", getGroup(context.snapshot(), id).id); return; }
    throw new Error(`Unknown preview action: ${action}`);
  }
  return { names: ["preview-card", "preview-group"], handleAction };
}
