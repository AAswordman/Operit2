export interface Route {
  id: string;
  label: string;
  kind: 'none' | 'navigate' | 'command';
  target?: string;
}

export interface EventBinding {
  field: 'action' | 'longAction';
  event: 'click' | 'long_press';
  label: string;
}

/** Shared capability list for the editor, validation and Agents. Every route must have a matching implementation in operit_mini_ui.c. */
export const routes: Route[] = [
  {id: '', label: '无动作', kind: 'none'},
  {id: 'home', label: '跳转 · 首页', kind: 'navigate', target: 'Home'},
  {id: 'apps', label: '跳转 · 应用网格', kind: 'navigate', target: 'Apps'},
  {id: 'page:theme', label: '跳转 · 主题设置', kind: 'navigate', target: 'Theme'},
  {id: 'page:settings', label: '跳转 · 设备设置', kind: 'navigate', target: 'Settings'},
  {id: 'page:network', label: '跳转 · 网络状态', kind: 'navigate', target: 'Network'},
  {id: 'page:face', label: '跳转 · 表情页面', kind: 'navigate', target: 'Face'},
  {id: 'page:terminal', label: '跳转 · 终端页面', kind: 'navigate', target: 'Terminal'},
  {id: 'theme_next', label: '主题 · 切换配色', kind: 'command'},
  {id: 'shape_toggle', label: '主题 · 切换图标形状', kind: 'command'},
  {id: 'face_online', label: '设备动作 · 在线表情', kind: 'command'},
  {id: 'run_node', label: '设备动作 · 运行节点', kind: 'command'},
  {id: 'edge_search', label: 'Edge · 搜索附近 Space', kind: 'command'},
  {id: 'edge_pair', label: 'Edge · 发起配对', kind: 'command'},
  {id: 'edge_chat', label: 'Edge · 打开 Space 聊天', kind: 'command'},
];

export const actions: string[] = routes.map((route) => route.id);

export const eventBindings: EventBinding[] = [
  {field: 'action', event: 'click', label: '点击'},
  {field: 'longAction', event: 'long_press', label: '长按'},
];

/** Looks up a registered route label; unknown ids are returned unchanged. */
export function routeLabel(id: string): string {
  const route = routes.find((item) => item.id === id);
  if (!route) return id;
  return route.label;
}
