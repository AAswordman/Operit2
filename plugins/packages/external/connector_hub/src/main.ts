import { PACKAGE, type Request } from "./api.js";
import { IPC, consoleRequest, dispatch } from "./service.js";
import consoleScreen from "./console.ui.js";

const id = "connector_console";
const route = `toolpkg:${PACKAGE}:ui:${id}`;
const title = { zh: "外部连接器", en: "External Connectors" };

ToolPkg.ipc.on(IPC, consoleRequest);
export async function connectors(event: ToolPkg.PublicApiEvent<Request>): Promise<unknown> {
  return dispatch(event.payload, event.callerPackage);
}
export function registerToolPkg(): boolean {
  ToolPkg.registerApi({ name: "connectors.v1", function: connectors });
  ToolPkg.registerUiRoute({ id, route, title, runtime: "compose_dsl", screen: consoleScreen });
  for (const surface of ["main_sidebar_plugins", "toolbox"] as const) {
    ToolPkg.registerNavigationEntry({ id: `${id}_${surface}`, route, surface, title, icon: "Hub", order: 144 });
  }
  return true;
}
