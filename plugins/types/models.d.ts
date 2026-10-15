// Generated from operit-plugin-sdk Rust declarations.

import type { ModelsListSummariesResultData } from "./results";

/**
 * Reads configured model summaries from the canonical host manager.
 */
export namespace Models {
  /**
   * Describes the actual configured model capability flags.
   */
  export interface Capabilities {
    directImage: boolean;
    directAudio: boolean;
    directVideo: boolean;
    toolCall: boolean;
  }

  /**
   * Selects the configured billing unit without changing its wire spelling.
   */
  export type BillingMode = "TOKEN" | "COUNT";

  /**
   * Selects the configured pricing currency.
   */
  export type PricingCurrency = "CNY" | "USD";

  /**
   * Preserves all configured model pricing fields, including explicit nullable prices.
   */
  export interface Pricing {
    billingMode: BillingMode;
    inputPricePerMillion: number;
    cachedInputPricePerMillion: number | null;
    cacheWritePricePerMillion: number | null;
    outputPricePerMillion: number;
    pricePerRequest: number;
    currency: PricingCurrency;
  }

  /**
   * Preserves a complete real provider/model summary returned by the host configuration manager.
   */
  export interface Summary {
    providerId: string;
    providerName: string;
    providerTypeId: string;
    endpoint: string;
    modelId: string;
    capabilities: Capabilities;
    pricing: Pricing | null;
  }

  /**
   * Returns complete provider/model summaries, capabilities and pricing.
   */
  function listSummaries(): Promise<ModelsListSummariesResultData>;
}
