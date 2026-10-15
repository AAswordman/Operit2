import type { Theme } from "../../../../types/theme";
import type { Tts } from "../../../../types/tts";

/** Constrains controlled boundary records to the sole officially generated independent configuration schema. */
export type SelectionSettingsInput = {
  themes: Awaited<ReturnType<typeof Theme.listConfigs>>;
  ttsConfigs: Awaited<ReturnType<typeof Tts.listConfigs>>;
};

/** Creates a labelled test record from the ordinary host's complete snapshot fixture, never a plugin-owned role theme. */
export function themeSettingsInput(snapshot: Theme.Config["snapshot"]): Theme.Config {
  return { id: "independent-theme-two", name: "Controlled independent theme", snapshot, createdAt: 1, updatedAt: 2 };
}

/** Creates a second complete controlled SDK speech record so tests can verify a real requested ID change. */
export function ttsSettingsInput(original: Tts.Config): Tts.Config {
  return { ...original, id: "tts-two", name: "Controlled independent speech", createdAt: 1, updatedAt: 2 };
}
