import workbench from './ui/workbench.ui.js';

const ROUTE = 'toolpkg:com.operit.image_workbench:ui:workbench';

/** Registers one workbench entry for all image providers and the preserved video capability. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: 'workbench', route: ROUTE, runtime: 'compose_dsl', screen: workbench, keepAlive: true, title: { zh: '生图工作台', en: 'Image Workbench' } });
  ToolPkg.registerNavigationEntry({ id: 'workbench', route: ROUTE, surface: 'main_sidebar_plugins', title: { zh: '生图工作台', en: 'Image Workbench' }, icon: 'palette', order: 40 });
  ToolPkg.registerNavigationEntry({ id: 'workbench_toolbox', route: ROUTE, surface: 'toolbox', title: { zh: '生图工作台', en: 'Image Workbench' }, icon: 'palette', order: 40 });
  return true;
}
