import type { EditorContext } from "../../../bridge/context";
import { changeFixedModel } from "../events";

/** Applies only declared character-binding switches, keeping unselected catalogue fields genuinely unselected. */
export function changeCharacterBinding(context: EditorContext, element: HTMLInputElement): boolean {
  switch (element.dataset.field) {
    case "fixedModel": changeFixedModel(context, element); return true;
    case "fixedTts": {
      const dialog = context.topDialog("character");
      dialog.ttsBindingEnabled = element.checked;
      if (!element.checked) dialog.card.ttsConfigId = null;
      context.renderDialogs(); return true;
    }
    case "sharedMemory": {
      const card = context.topDialog("character").card;
      card.memoryBindingMode = element.checked ? "SHARED" : "CHARACTER";
      if (!element.checked) card.sharedMemoryId = null;
      context.renderDialogs(); return true;
    }
    case "toolAccess":
      context.topDialog("character").card.toolAccessConfig.enabled = element.checked;
      context.renderDialogs(); return true;
    default: return false;
  }
}
