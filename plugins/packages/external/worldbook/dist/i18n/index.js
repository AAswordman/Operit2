"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.resolveWorldBookI18n = resolveWorldBookI18n;
exports.currentWorldBookI18n = currentWorldBookI18n;
const en_US_js_1 = require("./en-US.js");
const zh_CN_js_1 = require("./zh-CN.js");
const DEFAULT_LOCALE = "zh-CN";
const WORLDBOOK_I18N_MAP = {
    "zh-CN": zh_CN_js_1.WORLDBOOK_I18N_ZH_CN,
    "en-US": en_US_js_1.WORLDBOOK_I18N_EN_US
};
function normalizeLocale(locale) {
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
function resolveWorldBookI18n(locale) {
    const normalized = normalizeLocale(locale);
    return WORLDBOOK_I18N_MAP[normalized] || WORLDBOOK_I18N_MAP[DEFAULT_LOCALE];
}
/** Resolves the current host language at the time an editor or runtime operation runs. */
function currentWorldBookI18n() {
    return resolveWorldBookI18n(getLang());
}
