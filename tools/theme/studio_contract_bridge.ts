import type {
  ThemeBinaryExportInput,
  ThemeContractBridge,
  ThemePreviewAsset,
} from "./studio_message_port.ts";

interface WorkerClient {
  normalizeDraft(themeJson: Uint8Array): Promise<string>;
  validatePreview(themeJson: Uint8Array, assets: readonly ThemePreviewAsset[]): Promise<string>;
  exportPackage(input: ThemeBinaryExportInput): Promise<Uint8Array>;
  validatePackage(archiveBytes: Uint8Array, manifestPath: string): Promise<string>;
  close(): void;
}

/**
 * Studio's contract adapter. The worker URL must point at the same immutable
 * renderer release as the iframe; this class never implements ZIP or schema
 * semantics in TypeScript.
 */
export class StudioContractBridge implements ThemeContractBridge {
  private readonly client: Promise<WorkerClient>;

  constructor(workerUrl: string | URL, rendererReleaseId: string) {
    const worker = new URL(workerUrl);
    if (worker.origin !== window.location.origin || !worker.pathname.endsWith("/theme_contract_worker.js")) {
      throw new Error("contract worker must be the same-origin pinned renderer worker");
    }
    const clientUrl = new URL("theme_contract_client.js", worker);
    this.client = import(/* @vite-ignore */ clientUrl.href).then(module =>
      new module.ThemeContractWorker(worker.href, rendererReleaseId) as WorkerClient);
  }

  async normalizeDraft(themeJson: Uint8Array): Promise<string> {
    return (await this.client).normalizeDraft(themeJson);
  }

  async validatePreview(
    themeJson: Uint8Array,
    assets: readonly ThemePreviewAsset[],
  ): Promise<string> {
    return (await this.client).validatePreview(themeJson, assets);
  }

  async exportPackage(input: ThemeBinaryExportInput): Promise<Uint8Array> {
    return (await this.client).exportPackage(input);
  }

  /** Requires every template in the final merged archive to pass the shared Rust validator. */
  async validatePackage(archiveBytes: Uint8Array, manifestPath: string): Promise<void> {
    const response = JSON.parse(await (await this.client).validatePackage(archiveBytes, manifestPath));
    if (response.ok !== true) throw new Error(JSON.stringify(response.errors));
  }

  close(): void { void this.client.then(client => client.close()); }
}
