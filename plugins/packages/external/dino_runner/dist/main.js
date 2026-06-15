"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
const index_ui_js_1 = __importDefault(require("./ui/dino_runner/index.ui.js"));
const DINO_ROUTE = "toolpkg:com.operit.dino_runner:ui:dino_runner";
function registerToolPkg() {
    ToolPkg.registerUiRoute({
        id: "dino_runner",
        route: DINO_ROUTE,
        runtime: "compose_dsl",
        screen: index_ui_js_1.default,
        params: {},
        keepAlive: false,
        title: {
            zh: "小恐龙快跑",
            en: "Dino Runner",
        },
    });
    ToolPkg.registerNavigationEntry({
        id: "dino_runner_sidebar",
        route: DINO_ROUTE,
        surface: "main_sidebar_plugins",
        title: {
            zh: "小恐龙快跑",
            en: "Dino Runner",
        },
        icon: "SportsEsports",
        order: 145,
    });
    return true;
}
