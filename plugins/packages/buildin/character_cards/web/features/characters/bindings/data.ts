import type { Snapshot, Store, TtsConfig } from "../../../../src/model";
import type { CharacterDialog } from "../../../bridge/contracts";

/** Requires the exact saved shared library rather than displaying another identity as its name. */
export function sharedStore(snapshot: Snapshot, id: string): Store {
  for (const store of snapshot.stores) if (store.id === id) return store;
  throw new Error(`共享记忆库不存在：${id}`);
}

/** Resolves a selected TTS configuration from the actual host directory. */
export function ttsConfig(snapshot: Snapshot, id: string): TtsConfig {
  const matches = snapshot.ttsConfigs.filter(
    /** Requires exactly one authoritative independent config instead of taking the first duplicate identity. */
    config => config.id === id,
  );
  if (matches.length !== 1) throw new Error(`TTS 配置必须存在且标识唯一：${id}`);
  return matches[0];
}

/** Validates explicit binding selections before committing the complete character draft. */
export function validateBindings(dialog: CharacterDialog, snapshot: Snapshot): void {
  const card = dialog.card;
  if (card.chatModelBindingMode === "FIXED_MODEL") {
    if (card.chatModelId === null || card.chatModelId === "") throw new Error("请选择固定聊天模型");
    if (!snapshot.models.some(
      /** Requires the exact selected model identity in the real catalogue. */
      model => model.modelId === card.chatModelId,
    )) throw new Error("选择的聊天模型不存在");
  }
  if (dialog.ttsBindingEnabled) {
    if (card.ttsConfigId === null) throw new Error("请选择 TTS 配置");
    ttsConfig(snapshot, card.ttsConfigId);
  } else if (card.ttsConfigId !== null) throw new Error("跟随全局 TTS 时不能保留角色 TTS 绑定");
  if (card.memoryBindingMode === "SHARED") {
    if (card.sharedMemoryId === null || card.sharedMemoryId === "") throw new Error("请选择共享记忆库");
    sharedStore(snapshot, card.sharedMemoryId);
  }
}
