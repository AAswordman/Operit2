// Generated from operit-plugin-sdk Rust declarations.

import type { EnvironmentVariableReadResultData, EnvironmentVariableWriteResultData } from "./results";

/**
 * Describes the actual configured model capability flags.
 */
export interface SoftwareModelCapabilities {
  directImage: boolean;
  directAudio: boolean;
  directVideo: boolean;
  toolCall: boolean;
}

/**
 * Selects the configured billing unit without changing its wire spelling.
 */
export type SoftwareModelBillingMode = "TOKEN" | "COUNT";

/**
 * Selects the configured pricing currency.
 */
export type SoftwareModelPricingCurrency = "CNY" | "USD";

/**
 * Preserves all configured model pricing fields, including explicit nullable prices.
 */
export interface SoftwareModelPricing {
  billingMode: SoftwareModelBillingMode;
  inputPricePerMillion: number;
  cachedInputPricePerMillion: number | null;
  cacheWritePricePerMillion: number | null;
  outputPricePerMillion: number;
  pricePerRequest: number;
  currency: SoftwareModelPricingCurrency;
}

/**
 * Preserves a complete real provider/model summary returned by the host configuration manager.
 */
export interface SoftwareModelSummary {
  providerId: string;
  providerName: string;
  providerTypeId: string;
  endpoint: string;
  modelId: string;
  capabilities: SoftwareModelCapabilities;
  pricing: SoftwareModelPricing | null;
}

/**
 * Preserves one configured speech request or response-pipeline header.
 */
export interface SoftwareTtsHeader {
  name: string;
  value: string;
}

/**
 * Preserves every configured speech response-pipeline field.
 */
export interface SoftwareTtsResponseStep {
  stepType: string;
  path: string;
  headers: SoftwareTtsHeader[];
}

/**
 * Preserves the complete speech configuration returned by the host manager.
 */
export interface SoftwareTtsConfig {
  id: string;
  name: string;
  providerType: string;
  endpoint: string;
  apiKey: string;
  model: string;
  voice: string;
  responseFormat: string;
  speed: number;
  httpMethod: string;
  requestBody: string;
  contentType: string;
  headers: SoftwareTtsHeader[];
  responsePipeline: SoftwareTtsResponseStep[];
  createdAt: number;
  updatedAt: number;
}

/**
 * Preserves one ordinary named appearance configuration from the canonical user-preference ledger.
 */
export interface SoftwareThemeConfig {
  id: string;
  name: string;
  /**
   * Contains every key and exact value validated by the ordinary host appearance snapshot contract.
   */
  snapshot: Record<string, unknown>;
  createdAt: number;
  updatedAt: number;
}

/**
 * Identifies one actual configured tool source or registered built-in tool.
 */
export interface SoftwareToolSource {
  name: string;
  displayName: string;
  description: string;
}

/**
 * Separates the actual canonical source registries rather than classifying names heuristically.
 */
export interface SoftwareToolSourceCatalog {
  builtinTools: SoftwareToolSource[];
  packages: SoftwareToolSource[];
  skills: SoftwareToolSource[];
  mcpServers: SoftwareToolSource[];
}

/**
 * Reads host configuration directories, manages environment variables and executes core commands.
 */
export namespace SoftwareSettings {
  /**
   * Applies the complete existing named snapshot and active ID in the canonical preference transaction.
   */
  function applyThemeConfig(id: string): Promise<SoftwareThemeConfig>;
  /**
   * Execute a core command with CLI-style arguments.
   * @param args - Command arguments, for example ['plugin', 'list']
   */
  function exec(args: string[]): Promise<string>;
  /**
   * Reads the actual current canonical speech configuration without a plugin-usage lookup.
   */
  function getCurrentTtsConfigId(): Promise<string>;
  /**
   * Returns complete configured model summaries directly from the canonical host manager.
   */
  function listModelSummaries(): Promise<SoftwareModelSummary[]>;
  /**
   * Lists independent named themes from the same canonical ledger as ordinary appearance settings.
   */
  function listThemeConfigs(): Promise<SoftwareThemeConfig[]>;
  /**
   * Returns complete configured speech records directly from the canonical host manager.
   */
  function listTtsConfigs(): Promise<SoftwareTtsConfig[]>;
  /**
   * Read current value of an environment variable.
   * @param key - Environment variable key
   */
  function readEnvironmentVariable(key: string): Promise<EnvironmentVariableReadResultData>;
  /**
   * Returns actual builtin, package, skill and MCP source identities from their own registries.
   */
  function readToolSourceCatalog(): Promise<SoftwareToolSourceCatalog>;
  /**
   * Selects an existing exact speech ID; invalid IDs reject without changing the current configuration.
   */
  function setCurrentTtsConfigId(id: string): Promise<string>;
  /**
   * Write an environment variable; empty value clears the variable.
   * @param key - Environment variable key
   * @param value - Variable value (empty string clears)
   */
  function writeEnvironmentVariable(key: string, value?: string): Promise<EnvironmentVariableWriteResultData>;
}
