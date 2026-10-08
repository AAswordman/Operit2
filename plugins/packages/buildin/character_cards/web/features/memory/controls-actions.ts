import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
/** Opens owner-scoped workflows without accessing a Core-specific plugin bridge. */
export function createMemoryControlsFeature(context: EditorContext) {
  const { openUser, pushDialog } = context;
  /** Routes an explicit owner to this plugin's existing workflow contract. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "owner-user": await openUser(dataValue(element, "owner"), dataValue(element, "name")); return;

      case "owner-settings": pushDialog({ type: "owner-settings", ownerKey: dataValue(element, "owner"), name: dataValue(element, "name"), tab: 0 }); return;
      default: throw new Error(`Unknown memory controls action: ${action}`);
    }
  }
  return { names: ["owner-user", "owner-settings"], handleAction };
}
