import { WORLDBOOK_I18N_EN_US } from "./en-US.js";
import { WORLDBOOK_I18N_ZH_CN } from "./zh-CN.js";
import type { WorldBookI18n } from "./types";

const DEFAULT_LOCALE = "zh-CN";

const WORLDBOOK_I18N_MAP: Record<string, WorldBookI18n> = {
  "zh-CN": WORLDBOOK_I18N_ZH_CN,
  "en-US": WORLDBOOK_I18N_EN_US
};

function normalizeLocale(locale?: string): string {
  const value = (locale || "").trim();
  if (!value) {
    return DEFAULT_LOCALE;
  }
  const lower = value.toLowerCase();
  if (lower.startsWith("zh")) {
    return "zh-CN";
  }
  if (lower.startsWith("en")) {
    return "en-US";
  }
  return value;
}

export function resolveWorldBookI18n(locale?: string): WorldBookI18n {
  const normalized = normalizeLocale(locale);
  return WORLDBOOK_I18N_MAP[normalized] || WORLDBOOK_I18N_MAP[DEFAULT_LOCALE];
}

export type { WorldBookI18n } from "./types";

/** Resolves the current host language at the time an editor or runtime operation runs. */
export function currentWorldBookI18n(): WorldBookI18n {
  return resolveWorldBookI18n(getLang());
}
