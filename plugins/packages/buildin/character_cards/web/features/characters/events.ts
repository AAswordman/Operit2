import type { EditorContext } from "../../bridge/context";

/** Switches the existing model binding draft without inventing a selected model. */
export function changeFixedModel(context: EditorContext, element: HTMLInputElement): void {
  const card = context.topDialog("character").card;
  card.chatModelBindingMode = element.checked ? "FIXED_MODEL" : "FOLLOW_GLOBAL";
  if (!element.checked) card.chatModelId = null;
  context.renderDialogs();
}
