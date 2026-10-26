import type { Settings } from "./api.js";
import { bundledEmojis } from "./stickers.js";

export interface Config { profiles: Record<string, Settings> }

/** Authors the initial waifu profile using the original typing and prompt defaults. */
export function defaultSettings(): Settings {
  return { charDelayMs: 250, removePunctuation: false, enableEmoticons: false, enableSelfie: false,
    customPrompt: "你必须遵守：禁止使用动作表情，禁止描述动作表情，只允许使用纯文本进行对话。",
    selfiePrompt: "kipfel vrchat, long hair, Matcha color hair, purple eyes, sweater vest, black skirt, black necktie, collared shirt, long sleeves, black headwear, beanie, pleated skirt, hair bun, white shirt, hair ribbon, hairclip, hair between eyes, black footwear, blush, hair ornament, cat hat, very long hair, sweater, animal ear headwear, bag, bandaid on leg, socks", emojis: Object.fromEntries(Object.entries(bundledEmojis).map(([category, keys]) => [category, keys.map(key => `resource:${key}`)])) };
}

/** Checks complete settings before persistence or use by an active stream. */
export function validateSettings(value: Settings): Settings {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("settings must be an object");
  if (!Number.isSafeInteger(value.charDelayMs) || value.charDelayMs < 0 || value.charDelayMs > 3000) throw new Error("charDelayMs must be between 0 and 3000");
  for (const key of ["removePunctuation", "enableEmoticons", "enableSelfie"] as const) if (typeof value[key] !== "boolean") throw new Error(`${key} must be boolean`);
  for (const key of ["customPrompt", "selfiePrompt"] as const) if (typeof value[key] !== "string" || value[key].length > 16000) throw new Error(`${key} must be a string within 16000 characters`);
  if (value.enableSelfie && value.selfiePrompt.trim() === "") throw new Error("Selfie appearance prompt must be configured before enabling selfies");
  if (typeof value.emojis !== "object" || value.emojis === null || Array.isArray(value.emojis)) throw new Error("emojis must map categories to image paths or URLs");
  if (value.enableEmoticons && Object.keys(value.emojis).length === 0) throw new Error("Emoticons require at least one configured category");
  for (const [category, images] of Object.entries(value.emojis)) {
    if (!/^[a-z][a-z0-9_]{0,63}$/.test(category) || !Array.isArray(images) || images.length === 0 || images.length > 100) throw new Error(`Invalid emoji category: ${category}`);
    for (const image of images) if (typeof image !== "string" || image.trim() === "" || image.length > 4096) throw new Error(`Invalid emoji image in ${category}`);
  }
  return JSON.parse(JSON.stringify(value)) as Settings;
}

/** Validates identifiers at plugin protocol boundaries. */
export function identifier(value: string, label: string): string {
  if (typeof value !== "string" || value.trim() === "" || value.length > 200) throw new Error(`${label} must be a nonblank string within 200 characters`);
  return value;
}

let config: Promise<Config> | null = null;

/** Loads the single authoritative set of named profiles in the provider runtime. */
export function loadConfig(): Promise<Config> {
  if (config === null) config = PluginConfig.use("waifu", { profiles: { default: defaultSettings() } });
  return config;
}

/** Reads an explicitly selected persisted profile without silently choosing another one. */
export async function readSettings(profile: string): Promise<Settings> {
  identifier(profile, "profile");
  const value = (await loadConfig()).profiles[profile];
  if (value === undefined) throw new Error(`Waifu profile does not exist: ${profile}`);
  return validateSettings(value);
}

/** Durably replaces a validated profile and leaves active stream snapshots unchanged. */
export async function writeSettings(profile: string, settings: Settings): Promise<Settings> {
  identifier(profile, "profile");
  const checked = validateSettings(settings), store = await loadConfig();
  store.profiles = { ...store.profiles, [profile]: checked };
  await PluginConfig.flush(store);
  return checked;
}
