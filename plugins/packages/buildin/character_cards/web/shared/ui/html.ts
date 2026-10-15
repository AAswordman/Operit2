import { icon } from "./icons";
import type { IconName } from "./icons";
export { icon } from "./icons";

/** Escapes persisted text before inserting it into HTML or SVG markup. */
export function escapeHtml(value: unknown): string {
  return String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}

/** Builds an accessible Material action with explicit metadata. */
export function button(action: string, label: string, iconName: IconName | "", attrs = "", style = ""): string {
  const image = iconName === "" ? "" : icon(iconName);
  return `<button type="button" class="button ${style}" data-action="${action}" ${attrs}>${image}${escapeHtml(label)}</button>`;
}

/** Builds a compact icon action with the original Flutter tooltip label. */
export function iconButton(action: string, label: string, name: IconName, attrs = ""): string {
  return `<button type="button" class="icon-button" data-action="${action}" title="${escapeHtml(label)}" aria-label="${escapeHtml(label)}" ${attrs}>${icon(name)}</button>`;
}

/** Builds an outlined field with the same eight-pixel corners and floating label. */
export function field(label: string, key: string, value: string | number | null, rows = 0, disabled = false, caption = "", inputOptions: { type?: "number" | "password" | "date"; min?: number; max?: number; step?: number | "any"; required?: boolean } = {}): string {
  const attrs = `data-field="${key}" aria-label="${escapeHtml(label)}" ${disabled ? "disabled" : ""}`;
  const text = value === null ? "" : escapeHtml(value);
  const inputAttrs = `${inputOptions.type === undefined ? "" : `type="${inputOptions.type}"`} ${inputOptions.min === undefined ? "" : `min="${inputOptions.min}"`} ${inputOptions.max === undefined ? "" : `max="${inputOptions.max}"`} ${inputOptions.step === undefined ? "" : `step="${inputOptions.step}"`} ${inputOptions.required ? "required" : ""}`;
  const control = rows === 0 ? `<input ${attrs} ${inputAttrs} value="${text}">` : `<textarea ${attrs} rows="${rows}">${text}</textarea>`;
  const expand = rows > 0 && !disabled ? `<span class="field-action">${iconButton("expand-field", "展开编辑", "expand", `data-key="${key}"`)}</span>` : "";
  return `<label class="field ${rows > 0 ? "expandable" : ""}"><span class="field-label">${escapeHtml(label)}</span>${control}${expand}${caption === "" ? "" : `<div class="field-caption">${escapeHtml(caption)}</div>`}</label>`;
}

/** Keeps native selection and keyboard semantics inside a consistently sized outlined field. */
export function selectField(label: string, key: string, selected: string | null, options: { id: string; label: string }[], disabled: boolean): string {
  let html = `<label class="field select-field"><span class="field-label">${escapeHtml(label)}</span><select data-field="${key}" aria-label="${escapeHtml(label)}" ${disabled ? "disabled" : ""}><option value="">请选择</option>`;
  for (const item of options) html += `<option value="${escapeHtml(item.id)}" ${item.id === selected ? "selected" : ""}>${escapeHtml(item.label)}</option>`;
  return html + `</select><span class="select-arrow" aria-hidden="true">${icon("down")}</span></label>`;
}

/** Renders a real setting state or an explicitly unknown disabled state. */
export function switchRow(title: string, description: string, key: string, checked: boolean, disabled: boolean): string {
  return `<div class="switch-row"><div class="grow"><div class="switch-label">${escapeHtml(title)}</div><div class="switch-description">${escapeHtml(description)}</div></div><input class="switch" type="checkbox" role="switch" aria-label="${escapeHtml(title)}" data-field="${key}" ${checked === true ? "checked" : ""} ${disabled ? "disabled" : ""}></div>`;
}

/** Renders a compact source-preserving informational notice. */
export function notice(text: string): string { return `<div class="hint">${icon("info")}<span>${escapeHtml(text)}</span></div>`; }

/** Copies JSON data so editor drafts never mutate a loaded Core snapshot. */
export function copy<T>(value: T): T { return JSON.parse(JSON.stringify(value)); }
