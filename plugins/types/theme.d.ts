// Generated from operit-plugin-sdk Rust declarations.

import type { ThemeApplyConfigResultData, ThemeListConfigsResultData } from "./results";

/**
 * Reads and applies named themes through the canonical appearance manager.
 */
export namespace Theme {
  /**
   * Preserves one ordinary named appearance configuration from the canonical user-preference ledger.
   */
  export interface Config {
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
   * Applies an exact named snapshot and active ID in the canonical transaction.
   */
  function applyConfig(id: string): Promise<ThemeApplyConfigResultData>;
  /**
   * Lists independent named appearance configurations.
   */
  function listConfigs(): Promise<ThemeListConfigsResultData>;
}
