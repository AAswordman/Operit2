import type { IndependentConfigurationAccess, ThemeChoice } from "./selection-application";
import type { SoftwareSettings, SoftwareThemeConfig, SoftwareTtsConfig } from "../../../../types/software_settings";
import { assertObject, assertString, requireId } from "./validation";

let configured: IndependentConfigurationAccess | null = null;

/** Connects only actual independent owners; registration stores callbacks without reading preferences or applying configs. */
export function connectIndependentConfigurations(access: IndependentConfigurationAccess): void {
  if (configured !== null) throw new Error("Independent configuration access has already been connected");
  configured = access;
}

/** Rejects an absent dependency instead of declaring a referenced independent config valid or successfully applied. */
function requiredAccess(): IndependentConfigurationAccess {
  if (configured === null) throw new Error("Independent Theme/TTS configuration capabilities have not been connected");
  return configured;
}

/** Delegates to the one actually connected owner; explicit null plans never request these capabilities. */
export const independentConfigurations: IndependentConfigurationAccess = {
  /** Requires an actual theme directory reader; a missing source cannot become an empty successful chooser. */
  readThemeChoices: () => requiredAccess().readThemeChoices(),
  /** Requires a real theme existence validator before any selection write. */
  validateTheme: id => requiredAccess().validateTheme(id),
  /** Requires a real speech existence validator before any selection write. */
  validateTts: id => requiredAccess().validateTts(id),
  /** Awaits the actual theme owner's acknowledgement without synthesizing a receipt. */
  applyTheme: id => requiredAccess().applyTheme(id),
  /** Awaits the actual speech owner's acknowledgement without applying a different config on rejection. */
  applyTts: id => requiredAccess().applyTts(id),
};

/** Uses only the authoritative generic Settings methods; this plugin does not declare another wire schema. */
type ConfigurationSettings = Pick<typeof SoftwareSettings, "listThemeConfigs" | "applyThemeConfig" | "listTtsConfigs" | "getCurrentTtsConfigId" | "setCurrentTtsConfigId">;

/** Validates identity uniqueness in a real owner directory without inspecting or copying its preference snapshot. */
function configurationRecords<T extends { id: string }>(values: T[], label: string): T[] {
  if (!Array.isArray(values)) throw new Error(label + "目录必须是数组");
  const ids = new Set<string>();
  for (const value of values) {
    assertObject(value, label + "记录"); const id = requireId(value.id, label + " ID");
    if (ids.has(id)) throw new Error(label + " ID 重复：" + id);
    ids.add(id);
  }
  return values;
}

/** Requires one exact independent reference; absence is an error, never a request to use another config. */
function requireConfiguration<T extends { id: string }>(values: T[], id: string, label: string): T {
  requireId(id, label + "引用 ID"); const records = configurationRecords(values, label);
  const matches = records.filter(
    /** Resolves an exact ID, not a role-prefixed target, configured global or inferred participant. */
    config => config.id === id,
  );
  if (matches.length !== 1) throw new Error(label + "不存在：" + id);
  return matches[0];
}

/** Reads and applies true independent configuration owners while keeping all role/domain meaning in this plugin. */
export function createSettingsConfigurationAccess(readSettings: () => ConfigurationSettings): IndependentConfigurationAccess {
  return {
    /** Projects actual named Theme configs into the plugin's one private picker row contract. */
    async readThemeChoices(): Promise<ThemeChoice[]> {
      const configs = configurationRecords<SoftwareThemeConfig>(await readSettings().listThemeConfigs(), "主题配置");
      return configs.map(
        /** Preserves the actual independent ID and display name without storing or rewriting its snapshot. */
        config => {
          assertString(config.name, "主题配置名称");
          if (config.name.trim() === "") throw new Error("主题配置名称不能为空：" + config.id);
          return { id: config.id, label: config.name };
        },
      );
    },
    /** Verifies a genuine independent Theme ID before any active selection or chat namespace write. */
    async validateTheme(id: string): Promise<void> {
      requireConfiguration<SoftwareThemeConfig>(await readSettings().listThemeConfigs(), id, "主题配置");
    },
    /** Verifies a genuine independent TTS ID before any active selection or chat namespace write. */
    async validateTts(id: string): Promise<void> {
      requireConfiguration<SoftwareTtsConfig>(await readSettings().listTtsConfigs(), id, "TTS 配置");
    },
    /** Awaits the canonical Theme transaction and validates its acknowledgement of this exact independent config. */
    async applyTheme(id: string): Promise<void> {
      requireId(id, "主题配置引用 ID");
      const applied = await readSettings().applyThemeConfig(id); assertObject(applied, "已应用主题配置");
      if (applied.id !== id) throw new Error("主题配置应用返回的 ID 与请求不一致：" + id);
    },
    /** Awaits the canonical speech write and actual current-ID read; failures remain visible partial application errors. */
    async applyTts(id: string): Promise<void> {
      requireId(id, "TTS 配置引用 ID");
      const applied = await readSettings().setCurrentTtsConfigId(id);
      if (applied !== id) throw new Error("TTS 配置提交返回的 ID 与请求不一致：" + id);
      const current = await readSettings().getCurrentTtsConfigId();
      if (current !== id) throw new Error("TTS 当前配置读取结果未确认请求的 ID：" + id);
    },
  };
}

/** Connects the real Settings adapter exactly once during bootstrap without reading directories, opening files or applying prefs. */
export function registerSelectionSettingsAccess(): void {
  connectIndependentConfigurations(createSettingsConfigurationAccess(
    /** Resolves the canonical Settings capability only when a real directory read or application is requested. */
    () => Tools.SoftwareSettings,
  ));
}
