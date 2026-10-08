import type { SoftwareSettings, SoftwareThemeConfig, SoftwareTtsConfig } from "../../../../types/software_settings";

/** Constrains controlled boundary records to the sole officially generated independent configuration schema. */
export type SelectionSettingsInput = {
  themes: Awaited<ReturnType<typeof SoftwareSettings.listThemeConfigs>>;
  ttsConfigs: Awaited<ReturnType<typeof SoftwareSettings.listTtsConfigs>>;
};

/** Creates a labelled test record from the ordinary host's complete snapshot fixture, never a plugin-owned role theme. */
export function themeSettingsInput(snapshot: SoftwareThemeConfig["snapshot"]): SoftwareThemeConfig {
  return { id: "independent-theme-two", name: "Controlled independent theme", snapshot, createdAt: 1, updatedAt: 2 };
}

/** Creates a second complete controlled SDK speech record so tests can verify a real requested ID change. */
export function ttsSettingsInput(original: SoftwareTtsConfig): SoftwareTtsConfig {
  return { ...original, id: "tts-two", name: "Controlled independent speech", createdAt: 1, updatedAt: 2 };
}
