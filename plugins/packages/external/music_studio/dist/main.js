"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ROUTE = void 0;
exports.request = request;
exports.registerToolPkg = registerToolPkg;
const screen_1 = require("./ui/screen");
const host_service_1 = require("./host-service");
exports.ROUTE = "toolpkg:com.operit.music_studio:ui:studio";
ToolPkg.ipc.on("music.service", host_service_1.receive);
/** Same guarded, revision-checked service for dependent plugins. */
function request(event) { return (0, host_service_1.receive)(event.payload); }
function registerToolPkg() {
    ToolPkg.registerApi({ name: "request", function: request });
    ToolPkg.registerUiRoute({ id: "studio", route: exports.ROUTE, screen: screen_1.default, runtime: "compose_dsl", keepAlive: true, title: { zh: "音乐工作台", en: "Music Studio" } });
    for (const surface of ["main_sidebar_plugins", "toolbox"])
        ToolPkg.registerNavigationEntry({ id: `music_${surface}`, route: exports.ROUTE, surface, title: { zh: "音乐工作台", en: "Music Studio" }, icon: "MusicNote", order: 146 });
    return true;
}
