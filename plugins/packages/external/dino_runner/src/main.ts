import dinoRunnerScreen from "./ui/dino_runner/index.ui.js";

const DINO_ROUTE = "toolpkg:com.operit.dino_runner:ui:dino_runner";

export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({
    id: "dino_runner",
    route: DINO_ROUTE,
    runtime: "compose_dsl",
    screen: dinoRunnerScreen,
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
