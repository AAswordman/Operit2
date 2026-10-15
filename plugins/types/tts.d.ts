// Generated from operit-plugin-sdk Rust declarations.

import type { TtsGetCurrentConfigIdResultData, TtsListConfigsResultData, TtsSetCurrentConfigIdResultData, TtsSynthesizeResultData } from "./results";

/**
 * Reads and selects speech configurations through the canonical host manager.
 */
export namespace Tts {
  /**
   * Preserves one configured speech request or response-pipeline header.
   */
  export interface Header {
    name: string;
    value: string;
  }

  /**
   * Preserves every configured speech response-pipeline field.
   */
  export interface ResponseStep {
    stepType: string;
    path: string;
    headers: Header[];
  }

  /**
   * Preserves the complete speech configuration returned by the host manager.
   */
  export interface Config {
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
    headers: Header[];
    responsePipeline: ResponseStep[];
    createdAt: number;
    updatedAt: number;
  }

  /**
   * Synthesizes audio on the plugin's host without starting host playback.
   */
  export interface SynthesizeRequest {
    text: string;
    /**
     * Uses the selected host configuration when omitted.
     */
    configId?: string;
  }

  /**
   * Reads the current application-wide speech configuration ID.
   */
  function getCurrentConfigId(): Promise<TtsGetCurrentConfigIdResultData>;
  /**
   * Returns complete configured speech records.
   */
  function listConfigs(): Promise<TtsListConfigsResultData>;
  /**
   * Selects an exact existing speech configuration ID.
   */
  function setCurrentConfigId(id: string): Promise<TtsSetCurrentConfigIdResultData>;
  function synthesize(request: SynthesizeRequest): Promise<TtsSynthesizeResultData>;
}
