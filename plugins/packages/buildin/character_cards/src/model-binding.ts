import type { Card, ModelSummary } from "./model";

/** Interprets the documented legacy fixed-mode name without changing unknown stored values. */
export function characterModelBindingMode(mode: string): string {
  return mode === "FIXED_CONFIG" ? "FIXED_MODEL" : mode;
}

/** Compares only the model selection so unrelated edits preserve a binding that needs user repair. */
export function sameCharacterModelBinding(left: Card, right: Card): boolean {
  return left.chatModelBindingMode === right.chatModelBindingMode
    && left.chatProviderId === right.chatProviderId && left.chatModelId === right.chatModelId;
}

/** Describes the stored binding without changing its mode or selecting a replacement model. */
export function characterModelBindingStatus(card: Card, models: readonly ModelSummary[]): { needsRebind: boolean; label: string } {
  switch (characterModelBindingMode(card.chatModelBindingMode)) {
    case "FOLLOW_GLOBAL": return { needsRebind: false, label: "跟随全局模型" };
    case "FIXED_MODEL": {
      const matches = models.filter(
        /** Matches the exact provider and model identities selected by the user. */
        model => model.providerId === card.chatProviderId && model.modelId === card.chatModelId,
      );
      if (matches.length !== 1) return { needsRebind: true, label: "固定模型绑定无效，请重新绑定" };
      return { needsRebind: false, label: matches[0].providerName + " · " + matches[0].modelId };
    }
    default: return { needsRebind: true, label: "模型绑定模式无效，请重新绑定" };
  }
}
