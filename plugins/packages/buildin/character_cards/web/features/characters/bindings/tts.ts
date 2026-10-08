import type { Snapshot } from "../../../../src/model";
import type { ActionFeature, EditorContext } from "../../../bridge/context";
import type { DialogSpec, TtsDialog } from "../../../bridge/contracts";
import { dataValue, requireElement } from "../../../shared/dom";
import { button, escapeHtml, icon } from "../../../shared/ui/html";
import { matchesSelectionQuery } from "../selection";
import { ttsConfig } from "./data";

/** Renders the original searchable TTS chooser from full directory records without exposing credentials. */
export function ttsSpec(dialog: TtsDialog, snapshot: Snapshot): DialogSpec {
  let choices = "";
  for (const config of snapshot.ttsConfigs) {
    if (!matchesSelectionQuery(dialog.query, [config.name, config.model, config.voice, config.providerType])) continue;
    const selected = config.id === dialog.parent.card.ttsConfigId;
    choices += `<button type="button" class="picker-row" data-action="choose-tts" data-id="${escapeHtml(config.id)}" aria-label="选择 TTS ${escapeHtml(config.name)}">${icon(selected ? "check" : "tune")}<span><strong>${escapeHtml(config.name)}</strong><br><span class="muted">${escapeHtml(config.model)} · ${escapeHtml(config.voice)} · ${escapeHtml(config.providerType)}</span></span></button>`;
  }
  const search = `<label class="field"><span class="field-label">搜索 TTS 配置</span><input data-tts-query aria-label="搜索 TTS 配置" value="${escapeHtml(dialog.query)}"></label>`;
  const id = dialog.parent.card.ttsConfigId;
  const missing = id !== null && !snapshot.ttsConfigs.some(
    /** Keeps a deleted saved reference visible until the user explicitly selects another ID or removes the binding. */
    config => config.id === id,
  ) ? '<p role="alert">已绑定 TTS 配置不存在：' + escapeHtml(id) + '</p>' : "";
  return { title: "选择 TTS 配置", style: "chooser", tabs: [], body: search + missing + (choices === "" ? '<div class="empty">没有符合条件的 TTS 配置。</div>' : choices), footer: button("close-dialog", "取消", "") };
}

/** Owns local TTS selection; it changes only the parent draft until the actual character save. */
export function createTtsFeature(context: EditorContext): ActionFeature {
  /** Selects exactly one real configuration or opens the existing directory chooser. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "select-tts": context.pushDialog({ type: "tts", parent: context.topDialog("character"), query: "" }); return;
      case "choose-tts": {
        const dialog = context.topDialog("tts"), config = ttsConfig(context.snapshot(), dataValue(element, "id"));
        dialog.parent.card.ttsConfigId = config.id; dialog.parent.ttsBindingEnabled = true;
        context.popDialog(); return;
      }
      default: throw new Error(`Unknown TTS action: ${action}`);
    }
  }
  return { names: ["select-tts", "choose-tts"], handleAction };
}

/** Refreshes TTS search without losing the user's actual text selection or field focus. */
export function updateTtsQuery(context: EditorContext, element: HTMLInputElement): void {
  const start = element.selectionStart, end = element.selectionEnd;
  context.topDialog("tts").query = element.value;
  context.renderDialogs();
  const input = requireElement(context.dialogsRoot.querySelector("[data-tts-query]"), HTMLInputElement);
  input.focus(); input.setSelectionRange(start, end);
}
