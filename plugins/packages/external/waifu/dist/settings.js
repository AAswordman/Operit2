"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.defaultSettings = defaultSettings;
exports.validateSettings = validateSettings;
exports.identifier = identifier;
exports.loadConfig = loadConfig;
exports.readSettings = readSettings;
exports.writeSettings = writeSettings;
const stickers_js_1 = require("./stickers.js");
/** Authors the initial waifu profile using the original typing and prompt defaults. */
function defaultSettings() {
    return { charDelayMs: 250, removePunctuation: false, enableEmoticons: false, enableSelfie: false,
        customPrompt: "你必须遵守：禁止使用动作表情，禁止描述动作表情，只允许使用纯文本进行对话。",
        selfiePrompt: "kipfel vrchat, long hair, Matcha color hair, purple eyes, sweater vest, black skirt, black necktie, collared shirt, long sleeves, black headwear, beanie, pleated skirt, hair bun, white shirt, hair ribbon, hairclip, hair between eyes, black footwear, blush, hair ornament, cat hat, very long hair, sweater, animal ear headwear, bag, bandaid on leg, socks", emojis: Object.fromEntries(Object.entries(stickers_js_1.bundledEmojis).map(([category, keys]) => [category, keys.map(key => `resource:${key}`)])) };
}
/** Checks complete settings before persistence or use by an active stream. */
function validateSettings(value) {
    if (typeof value !== "object" || value === null || Array.isArray(value))
        throw new Error("settings must be an object");
    if (!Number.isSafeInteger(value.charDelayMs) || value.charDelayMs < 0 || value.charDelayMs > 3000)
        throw new Error("charDelayMs must be between 0 and 3000");
    for (const key of ["removePunctuation", "enableEmoticons", "enableSelfie"])
        if (typeof value[key] !== "boolean")
            throw new Error(`${key} must be boolean`);
    for (const key of ["customPrompt", "selfiePrompt"])
        if (typeof value[key] !== "string" || value[key].length > 16000)
            throw new Error(`${key} must be a string within 16000 characters`);
    if (value.enableSelfie && value.selfiePrompt.trim() === "")
        throw new Error("Selfie appearance prompt must be configured before enabling selfies");
    if (typeof value.emojis !== "object" || value.emojis === null || Array.isArray(value.emojis))
        throw new Error("emojis must map categories to image paths or URLs");
    if (value.enableEmoticons && Object.keys(value.emojis).length === 0)
        throw new Error("Emoticons require at least one configured category");
    for (const [category, images] of Object.entries(value.emojis)) {
        if (!/^[a-z][a-z0-9_]{0,63}$/.test(category) || !Array.isArray(images) || images.length === 0 || images.length > 100)
            throw new Error(`Invalid emoji category: ${category}`);
        for (const image of images)
            if (typeof image !== "string" || image.trim() === "" || image.length > 4096)
                throw new Error(`Invalid emoji image in ${category}`);
    }
    return JSON.parse(JSON.stringify(value));
}
/** Validates identifiers at plugin protocol boundaries. */
function identifier(value, label) {
    if (typeof value !== "string" || value.trim() === "" || value.length > 200)
        throw new Error(`${label} must be a nonblank string within 200 characters`);
    return value;
}
let config = null;
/** Loads the single authoritative set of named profiles in the provider runtime. */
function loadConfig() {
    if (config === null)
        config = PluginConfig.use("waifu", { profiles: { default: defaultSettings() } });
    return config;
}
/** Reads an explicitly selected persisted profile without silently choosing another one. */
async function readSettings(profile) {
    identifier(profile, "profile");
    const value = (await loadConfig()).profiles[profile];
    if (value === undefined)
        throw new Error(`Waifu profile does not exist: ${profile}`);
    return validateSettings(value);
}
/** Durably replaces a validated profile and leaves active stream snapshots unchanged. */
async function writeSettings(profile, settings) {
    identifier(profile, "profile");
    const checked = validateSettings(settings), store = await loadConfig();
    store.profiles = { ...store.profiles, [profile]: checked };
    await PluginConfig.flush(store);
    return checked;
}
