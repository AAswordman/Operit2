import summary from './ui/summary.ui.js';
import workspace from './ui/workspace.ui.js';
import home from './ui/home.ui.js';

export const SUMMARY_ROUTE = 'toolpkg:com.operit.change_review:ui:summary';
export const WORKSPACE_ROUTE = 'toolpkg:com.operit.change_review:ui:workspace';
export const HOME_ROUTE = 'toolpkg:com.operit.change_review:ui:home';

/** Registers three Compose surfaces under one independently disableable plugin owner. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: 'home', route: HOME_ROUTE, runtime: 'compose_dsl', screen: home, title: { zh: '代码变更审查', en: 'Code change review' } });
  ToolPkg.registerUiRoute({ id: 'summary', route: SUMMARY_ROUTE, runtime: 'compose_dsl', screen: summary, title: { zh: '文件变更', en: 'File changes' } });
  ToolPkg.registerUiRoute({ id: 'workspace', route: WORKSPACE_ROUTE, runtime: 'compose_dsl', screen: workspace, keepAlive: true, title: { zh: '变更', en: 'Changes' } });
  ToolPkg.registerNavigationEntry({ id: 'message_changes', route: SUMMARY_ROUTE, surface: 'chat_message_after', title: { zh: '文件变更', en: 'File changes' }, order: 100 });
  ToolPkg.registerNavigationEntry({ id: 'workspace_changes', route: WORKSPACE_ROUTE, surface: 'workspace', title: { zh: '变更', en: 'Changes' }, icon: 'difference_outlined', order: 100 });
  ToolPkg.registerNavigationEntry({ id: 'home_changes', route: HOME_ROUTE, surface: 'workspace_home', title: { zh: '代码变更审查', en: 'Code change review' }, order: 100 });
  return true;
}
