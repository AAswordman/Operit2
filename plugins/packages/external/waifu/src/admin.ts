import type { Settings } from "./api.js";
import type { Mode, ModeContext } from "./mode.js";
import { modeContext, saveMode } from "./mode.js";
import { readSettings, writeSettings } from "./settings.js";

export type AdminRequest = { action: "get"; profile: string } | { action: "set"; profile: string; settings: Settings }
  | { action: "mode.get" } | { action: "mode.set"; selection: string; mode: Mode }
  | { action: "emoji.previews"; emojis: Record<string, string[]> }
  | { action: "emoji.import"; files: { path: string; name: string }[] };
type Result<R extends AdminRequest> = R extends { action: "mode.get" } ? ModeContext
  : R extends { action: "mode.set" } ? Mode
  : R extends { action: "emoji.previews" } ? Record<string, string[]>
  : R extends { action: "emoji.import" } ? string[] : Settings;
let nextImage = 0;

/** Routes native authoring commands through the single provider runtime. */
export async function admin(request: AdminRequest): Promise<Settings | ModeContext | Mode | Record<string, string[]> | string[]> {
  switch (request.action) {
    case "get": return readSettings(request.profile);
    case "set": return writeSettings(request.profile, request.settings);
    case "mode.get": return modeContext();
    case "mode.set": return saveMode(request.selection, request.mode);
    case "emoji.previews": {
      const result: Record<string, string[]> = {};
      for (const [category, images] of Object.entries(request.emojis)) {
        result[category] = [];
        for (const source of images) {
          const resource = /^resource:([a-z0-9_]+)$/.exec(source);
          result[category].push(resource === null ? source : await ToolPkg.readResource(resource[1]));
        }
      }
      return result;
    }
    case "emoji.import": {
      const directory = `${ToolPkg.getSpaceDataDir()}/emojis`;
      await Tools.Files.mkdir(directory, true);
      const paths: string[] = [];
      for (const file of request.files) {
        const extension = /\.(png|jpe?g|gif|webp|avif)$/i.exec(file.name);
        if (extension === null) throw new Error(`Unsupported emoji image: ${file.name}`);
        const destination = `${directory}/${Date.now()}-${++nextImage}.${extension[1].toLowerCase()}`;
        await Tools.Files.copy(file.path, destination);
        paths.push(destination);
      }
      return paths;
    }
  }
}

/** Calls the private main-runtime settings channel from tools and Compose screens. */
export function callAdmin<R extends AdminRequest>(request: R): Promise<Result<R>> {
  return ToolPkg.ipc.call<R, Result<R>>("waifu.settings", request, { targetRuntime: "main" });
}
