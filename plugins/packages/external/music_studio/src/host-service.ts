import { StudioService, type StudioStore } from "./service";
import type { Request } from "./shared/model";
let config: { database: StudioStore | null } | undefined;
const service = new StudioService({
  async read() { config = await PluginConfig.use("music-studio-v1", { database: null as StudioStore | null }); return config.database; },
  async write(value) { if (!config) throw new Error("Storage not initialized"); const before = config.database; config.database = value; try { await PluginConfig.flush(config); } catch (error) { config.database = before; try { await PluginConfig.flush(config); } catch { /* Preserve the original durable write error. */ } throw error; } },
});
export function receive(request: Request): Promise<unknown> { return service.request(request); }
