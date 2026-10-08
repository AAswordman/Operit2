import type { Snapshot } from "../../../../src/model";
import type { CharacterDialog } from "../../../bridge/contracts";
import { button, escapeHtml, selectField, switchRow } from "../../../shared/ui/html";
import { themeReferenceSection } from "./theme";

/** Renders the original editable binding sections from real catalogues, without choosing default entries. */
export function bindingsBody(dialog: CharacterDialog, snapshot: Snapshot): string {
  const card = dialog.card, models = [], stores = [];
  for (const model of snapshot.models) models.push({ id: model.modelId, label: `${model.providerName} · ${model.modelId}` });
  for (const store of snapshot.stores) stores.push({ id: store.id, label: store.name });
  const toolCount = card.toolAccessConfig.allowedBuiltinTools.length + card.toolAccessConfig.allowedPackages.length + card.toolAccessConfig.allowedSkills.length + card.toolAccessConfig.allowedMcpServers.length;
  const ttsMatches = snapshot.ttsConfigs.filter(
    /** Matches the explicit saved speech reference without assigning any other config when it was deleted. */
    config => config.id === card.ttsConfigId,
  );
  if (ttsMatches.length > 1) throw new Error("TTS 配置 ID 重复：" + card.ttsConfigId);
  const tts = card.ttsConfigId === null ? "请选择 TTS 配置" : ttsMatches.length === 1 ? ttsMatches[0].name : "已绑定 TTS 配置不存在：" + card.ttsConfigId;
  const ttsMissing = card.ttsConfigId !== null && ttsMatches.length === 0;
  const modelSection = `<section class="binding">${switchRow("聊天模型", card.chatModelBindingMode === "FIXED_MODEL" ? "使用固定模型配置" : "跟随全局模型配置", "fixedModel", card.chatModelBindingMode === "FIXED_MODEL", false)}${card.chatModelBindingMode === "FIXED_MODEL" ? selectField("聊天模型配置", "chatModelId", card.chatModelId, models, false) : ""}</section>`;
  const ttsSection = `<section class="binding">${switchRow("TTS 配置", dialog.ttsBindingEnabled ? "使用角色卡 TTS 配置" : "不要求改变当前独立 TTS 配置", "fixedTts", dialog.ttsBindingEnabled, false)}${snapshot.ttsConfigs.length === 0 ? '<p class="muted">还没有 TTS 配置</p>' : ""}${dialog.ttsBindingEnabled ? `<div class="row"><span class="grow" ${ttsMissing ? 'role="alert"' : ""}>${escapeHtml(tts)}</span>${button("select-tts", "选择 TTS 配置", "tune", "", "small")}</div>` : ""}</section>`;
  const memorySection = `<section class="binding">${switchRow("记忆绑定", card.memoryBindingMode === "SHARED" ? "使用共享记忆" : "使用角色记忆", "sharedMemory", card.memoryBindingMode === "SHARED", snapshot.stores.length === 0)}${snapshot.stores.length === 0 ? '<p class="muted">还没有共享记忆库</p>' : card.memoryBindingMode === "SHARED" ? selectField("共享记忆库", "sharedMemoryId", card.sharedMemoryId, stores, false) : ""}</section>`;
  const toolSection = `<section class="binding">${switchRow("工具访问", card.toolAccessConfig.enabled ? "自定义允许使用的工具" : "跟随全局工具配置", "toolAccess", card.toolAccessConfig.enabled, false)}${card.toolAccessConfig.enabled ? `<div class="row"><span class="grow muted">已配置 ${toolCount} 个工具来源</span>${button("show-tool-access", "配置允许使用的工具", "tools", "", "small")}</div>` : ""}</section>`;
  return modelSection + themeReferenceSection(card.themeConfigId) + ttsSection + memorySection + toolSection;
}
