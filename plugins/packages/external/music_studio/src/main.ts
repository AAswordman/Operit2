import screen from "./ui/screen";
import { receive } from "./host-service";
import type { Request } from "./shared/model";
export const ROUTE = "toolpkg:com.operit.music_studio:ui:studio";
ToolPkg.ipc.on<Request, unknown>("music.service", receive);
/** Same guarded, revision-checked service for dependent plugins. */
export function request(event: ToolPkg.PublicApiEvent<Request>): Promise<unknown> { return receive(event.payload); }
export function registerToolPkg(): boolean {
  ToolPkg.registerApi({ name: "request", function: request });
  ToolPkg.registerUiRoute({ id: "studio", route: ROUTE, screen, runtime: "compose_dsl", keepAlive: true, title: { zh: "音乐工作台", en: "Music Studio" } });
  for (const surface of ["main_sidebar_plugins", "toolbox"] as const) ToolPkg.registerNavigationEntry({ id: `music_${surface}`, route: ROUTE, surface, title: { zh: "音乐工作台", en: "Music Studio" }, icon: "MusicNote", order: 146 });
  return true;
}
