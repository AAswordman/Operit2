// Generated from operit-plugin-sdk Rust declarations.

import type { CatalogReadToolSourcesResultData } from "./results";

/**
 * Reads tool-source identities from their canonical registries.
 */
export namespace Catalog {
  /**
   * Identifies one actual configured tool source or registered built-in tool.
   */
  export interface ToolSource {
    name: string;
    displayName: string;
    description: string;
  }

  /**
   * Separates the actual canonical source registries rather than classifying names heuristically.
   */
  export interface ToolSources {
    builtinTools: ToolSource[];
    packages: ToolSource[];
    skills: ToolSource[];
    mcpServers: ToolSource[];
  }

  /**
   * Returns actual builtin, package, skill and MCP source identities.
   */
  function readToolSources(): Promise<CatalogReadToolSourcesResultData>;
}
