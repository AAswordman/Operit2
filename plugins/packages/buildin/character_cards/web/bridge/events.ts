import { changeCharacterBinding } from "../features/characters/bindings/events";
import { changeToolSource } from "../features/characters/bindings/tool-policy";
import { updateTtsQuery } from "../features/characters/bindings/tts";
import { updateThemeQuery } from "../features/characters/bindings/theme";
import { changeGroupMember } from "../features/groups/events";
import { updateScoreOutput } from "../features/memory/item-events";
import { dataValue } from "../shared/dom";
import { editableDialog, setEditableValue } from "../shared/edits";
import type { ActionFeature, EditorContext } from "./context";
/** Routes user events to an exact registered feature; unregistered actions are explicit errors. */
export function createEditorEvents(context: EditorContext, features: readonly ActionFeature[]) {
  const { state, topDialog, renderDialogs, renderMain, dismissDialog, updateGraphSurface } = context;
  const registry = new Map<string, ActionFeature>();
  for (const feature of features) for (const name of feature.names) {
    if (registry.has(name)) throw new Error(`Duplicate UI action owner: ${name}`);
    registry.set(name, feature);
  }
  /** Looks up only an exact finite action name in this editor's feature registry. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    const feature = registry.get(action);
    if (feature === undefined) throw new Error(`Unknown UI action: ${action}`);
    await feature.handleAction(action, element);
  }
  /** Serializes interactive operations and preserves drafts when the host reports an error. */
  async function onClick(event: MouseEvent | KeyboardEvent): Promise<void> {
    if (!(event.target instanceof Element)) return;
    if (event.type === "click" && event.detail !== 0 && state.gestureMoved && event.target.closest<SVGSVGElement>("[data-graph-surface]") !== null) {
      event.preventDefault(); return;
    }
    const element = event.target.closest<HTMLElement | SVGElement>("[data-action]");
    if (element === null || (element instanceof HTMLButtonElement && element.disabled) || state.busy) return;
    event.preventDefault();
    state.busy = true;
    const owner = state.dialogs.length === 0 ? null : topDialog();
    if (owner !== null) owner.error = "";
    try { await handleAction(dataValue(element, "action"), element); }
    catch (error) {
      if (state.dialogs.length !== 0) { topDialog().error = String(error); renderDialogs(); }
      else { state.error = String(error); renderMain(); }
    } finally { state.busy = false; renderDialogs(); if (state.dialogs.length === 0) renderMain(); }
  }
  /** Updates only the active draft without rebuilding fields while the user types. */
  function onInput(event: Event): void {
    if (!(event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement || event.target instanceof HTMLSelectElement)) return;
    const element = event.target;
    if (element.disabled) return;
    if (element instanceof HTMLInputElement && context.onScreenInput(element)) return;
    if (element instanceof HTMLInputElement && element.hasAttribute("data-theme-query")) { updateThemeQuery(context, element); return; }
    if (element instanceof HTMLInputElement && element.hasAttribute("data-tts-query")) { updateTtsQuery(context, element); return; }
    if (element instanceof HTMLInputElement && element.type === "checkbox") return;
    if (element.hasAttribute("data-graph-search")) { topDialog("graph").query = element.value; return; }
    if (!element.hasAttribute("data-field") || element.disabled) return;
    const dialog = topDialog(), key = dataValue(element, "field");
    if (key === "fixedModel") return;
    if (dialog.type === "text") { if (key !== "value") throw new Error("展开编辑字段无效"); dialog.value = element.value; return; }
    setEditableValue(editableDialog(dialog), key, element.value);
    if (dialog.type === "memory-edit" && element instanceof HTMLInputElement && element.type === "range") {
      updateScoreOutput(element);
    }
  }
  /** Handles typed switches, member selection and graph folder filtering. */
  function onChange(event: Event): void {
    if (!(event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement)) return;
    const element = event.target;
    if (element.disabled) return;
    if (element.hasAttribute("data-graph-folder")) { topDialog("graph").folder = element.value; updateGraphSurface(topDialog("graph")); return; }
    if (element instanceof HTMLInputElement && changeCharacterBinding(context, element)) return;
    if (element instanceof HTMLInputElement && element.hasAttribute("data-tool-category")) { changeToolSource(context, element); return; }
    if (element.hasAttribute("data-member") && element instanceof HTMLInputElement) changeGroupMember(context, element);
  }
  /** Keeps Escape scoped to the top nested dialog and blocks cancellation during an operation. */
  async function onCancel(event: Event): Promise<void> {
    event.preventDefault();
    if (state.busy || state.dialogs.length === 0) return;
    state.busy = true;
    try { await dismissDialog(); }
    catch (error) { if (state.dialogs.length !== 0) topDialog().error = String(error); else state.error = String(error); }
    finally { state.busy = false; renderDialogs(); if (state.dialogs.length === 0) renderMain(); }
  }
  return { onClick, onInput, onChange, onCancel };
}
