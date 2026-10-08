import type { ActivePrompt, Card, Group, Snapshot } from "../../../src/model";
import type { SelectScreen, SelectionKind, EntityKind, ScreenFeatureContext } from "../../bridge/screens";
import { getGroup } from "../groups/data";
import { getCard, isActive } from "./data";
import type { ActionFeature } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { button, escapeHtml, icon, iconButton } from "../../shared/ui/html";
import { groupSelectionTile } from "../groups/selection";

export interface SelectionState {
  mode: "select";
  input: SelectScreen;
  query: string;
  category: SelectionKind;
  selected: ActivePrompt | null;
}

/** Creates the selector from the caller's explicit category and selected prompt. */
export function createSelectionState(input: SelectScreen): SelectionState {
  return { mode: "select", input, query: "", category: input.kind, selected: input.selected };
}

/** Matches user-entered search text, not structural route or entity discriminators. */
export function matchesSelectionQuery(query: string, fields: readonly string[]): boolean {
  const text = query.trim().toLocaleLowerCase();
  if (text === "") return true;
  for (const field of fields) if (field.toLocaleLowerCase().indexOf(text) >= 0) return true;
  return false;
}

/** Resolves the complete group member list in its canonical order. */
export function groupMemberNames(group: Group, snapshot: Snapshot): string[] {
  const members = [...group.members].sort(
    /** Orders canonical members without mutating the loaded aggregate. */
    (left, right) => left.orderIndex - right.orderIndex,
  );
  const names: string[] = [];
  for (const member of members) names.push(getCard(snapshot, member.characterCardId).name);
  return names;
}

/** Searches character names, descriptions and assigned tags from the real snapshot. */
export function cardMatchesQuery(card: Card, query: string, snapshot: Snapshot): boolean {
  const fields = [card.name, card.description];
  for (const id of card.attachedTagIds) for (const tag of snapshot.tags) if (tag.id === id) fields.push(tag.name);
  return matchesSelectionQuery(query, fields);
}

/** Compares exact externally tagged prompt identities. */
export function selectedEntity(selection: ActivePrompt | null, kind: EntityKind, id: string): boolean {
  if (selection === null) return false;
  switch (kind) {
    case "card": return "CharacterCard" in selection && selection.CharacterCard.id === id;
    case "group": return "CharacterGroup" in selection && selection.CharacterGroup.id === id;
  }
}

/** Validates that the selected prompt is present and permitted by this selector's category. */
export function validateSelection(selection: ActivePrompt | null, kind: SelectionKind, snapshot: Snapshot): void {
  if (selection === null) return;
  if ("CharacterCard" in selection) {
    if (kind === "group") throw new Error("群组选择器不能选择角色卡");
    getCard(snapshot, selection.CharacterCard.id);
  } else {
    if (kind === "card") throw new Error("角色选择器不能选择群组");
    getGroup(snapshot, selection.CharacterGroup.id);
  }
}

/** Owns selector events; confirmation returns a prompt identity and never activates it implicitly. */
export function createSelectionFeature(context: ScreenFeatureContext): ActionFeature & {
  /** Updates the user's search text while preserving field focus. */
  onInput(element: HTMLInputElement): void;
} {
  const { editor, screen, render, complete } = context;
  /** Selects only a permitted canonical entity or returns its typed result. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    const state = screen("select");
    switch (action) {
      case "select-character": {
        if (state.input.kind === "group") throw new Error("当前视图只允许选择群组");
        const card = getCard(editor.snapshot(), dataValue(element, "id"));
        state.selected = { CharacterCard: { id: card.id } }; render(); return;
      }
      case "select-group": {
        if (state.input.kind === "card") throw new Error("当前视图只允许选择角色卡");
        const group = getGroup(editor.snapshot(), dataValue(element, "id"));
        state.selected = { CharacterGroup: { id: group.id } }; render(); return;
      }
      case "selection-category": {
        if (state.input.kind !== "all") throw new Error("当前视图不能切换选择类型");
        const category = dataValue(element, "category");
        if (category !== "all" && category !== "card" && category !== "group") throw new Error("选择类型无效");
        state.category = category; render(); return;
      }
      case "confirm-selection":
        if (state.selected === null) throw new Error("请选择角色或群组");
        validateSelection(state.selected, state.input.kind, editor.snapshot());
        await complete({ mode: "select", selection: state.selected }); return;
      default: throw new Error(`Unknown selector action: ${action}`);
    }
  }
  /** Repaints search results and restores the actual input's caret after DOM replacement. */
  function onInput(element: HTMLInputElement): void {
    const state = screen("select"), start = element.selectionStart, end = element.selectionEnd;
    state.query = element.value; render();
    const input = editor.app.querySelector<HTMLInputElement>("[data-selection-query]");
    if (input === null) throw new Error("角色选择搜索框不存在");
    input.focus(); input.setSelectionRange(start, end);
  }
  return { names: ["select-character", "select-group", "selection-category", "confirm-selection"], handleAction, onInput };
}

/** Renders one actual card as a selector option without changing the active prompt. */
function selectionTile(card: Card, state: SelectionState, snapshot: Snapshot): string {
  const selected = selectedEntity(state.selected, "card", card.id);
  return `<article class="entity ${selected ? "selected" : ""}" data-selection-entity="card"><button type="button" class="entity-main" role="radio" aria-checked="${selected}" aria-label="选择 ${escapeHtml(card.name)}" data-action="select-character" data-id="${escapeHtml(card.id)}"><span class="avatar">${icon("person")}</span><span class="entity-copy"><span class="entity-title">${escapeHtml(card.name)}</span><span class="entity-subtitle">${escapeHtml(card.description)}</span>${isActive(snapshot.active, "card", card.id) ? '<span class="badges"><span class="badge">当前使用</span></span>' : ""}</span><span class="selection-check">${selected ? icon("check") : ""}</span></button><div class="entity-actions">${iconButton("preview-card", `预览 ${card.name}`, "info", `data-id="${escapeHtml(card.id)}"`)}</div></article>`;
}

/** Paints the searchable character/group selector using the existing Material tile language. */
export function selectionPage(state: SelectionState, snapshot: Snapshot, error: string, disabled: boolean): string {
  let cards = "", groups = "";
  if (state.category !== "group") for (const card of snapshot.cards) if (cardMatchesQuery(card, state.query, snapshot)) cards += selectionTile(card, state, snapshot);
  if (state.category !== "card") for (const group of snapshot.groups) {
    if (matchesSelectionQuery(state.query, [group.name, group.description, ...groupMemberNames(group, snapshot)])) groups += groupSelectionTile(group, state.selected, snapshot);
  }
  const filter = state.input.kind === "all" ? `<div class="chips selection-categories" aria-label="选择类型">${categoryButton("all", "全部", state)}${categoryButton("card", "角色卡", state)}${categoryButton("group", "群组", state)}</div>` : "";
  const list = cards + groups;
  return `<div class="page presentation-page"><header class="appbar">${icon(state.input.kind === "group" ? "groups" : "badge")}<h1>${state.input.kind === "group" ? "选择群组" : "选择角色"}</h1></header><div class="selection-toolbar"><label class="field"><span class="field-label">搜索角色或群组</span><input data-selection-query aria-label="搜索角色或群组" value="${escapeHtml(state.query)}"></label>${filter}</div><div class="scroll">${error === "" ? "" : `<div class="error-banner" role="alert">${escapeHtml(error)}</div>`}<div role="radiogroup" aria-label="角色与群组">${list === "" ? '<div class="empty">没有符合条件的角色或群组。</div>' : list}</div></div><footer class="presentation-footer">${button("screen-cancel", "取消", "", disabled ? "disabled" : "")}${button("confirm-selection", "确定", "check", disabled || state.selected === null ? "disabled" : "", "filled")}</footer></div>`;
}

/** Renders an explicit category control with a stable finite discriminator. */
function categoryButton(category: SelectionState["category"], label: string, state: SelectionState): string {
  return `<button type="button" class="chip ${state.category === category ? "selected" : ""}" data-action="selection-category" data-category="${category}" aria-pressed="${state.category === category}">${escapeHtml(label)}</button>`;
}
