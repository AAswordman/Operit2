import type {LayoutDocument, LayoutNode, LayoutPage} from './project-model.mts';

/** Builds a layout node with board defaults. */
function node(
  id: string,
  type: string,
  x: number,
  y: number,
  w: number,
  h: number,
  text = '',
  extra: Partial<LayoutNode> = {},
): LayoutNode {
  return {
    id,
    type,
    parent: null,
    x,
    y,
    w,
    h,
    text,
    color: type === 'label' || type === 'icon' ? '#f4f8ff' : '#172a3d',
    radius: 12,
    value: 0,
    action: '',
    ...extra,
  };
}

/** Builds a label node. */
function label(
  id: string,
  x: number,
  y: number,
  w: number,
  h: number,
  text: string,
  extra: Partial<LayoutNode> = {},
): LayoutNode {
  return node(id, 'label', x, y, w, h, text, {radius: 0, ...extra});
}

/** Builds a button node with a route action. */
function button(
  id: string,
  x: number,
  y: number,
  w: number,
  h: number,
  text: string,
  action: string,
  extra: Partial<LayoutNode> = {},
): LayoutNode {
  return node(id, 'button', x, y, w, h, text, {color: '#216c73', action, ...extra});
}

/** Builds a sub-page with a back button and title. */
function screen(id: string, name: string, nodes: LayoutNode[]): LayoutPage {
  return {
    id,
    name,
    background: '#091420',
    nodes: [
      button(id + '_back', 12, 10, 36, 32, '<', 'go:apps'),
      label(id + '_title', 58, 16, 246, 24, name.toUpperCase()),
      ...nodes,
    ],
  };
}

/** Editable counterparts of the original firmware pages. No raster assets. */
export function defaultProject(): LayoutDocument {
  const home = [
    label('home_brand', 18, 16, 190, 24, 'OPERIT / EDGE', {color: '#53dfc5'}),
    label('home_theme', 242, 16, 74, 24, 'Aurora'),
    node('home_card', 'panel', 18, 48, 284, 111, '', {radius: 22}),
    label('home_clock', 31, 56, 262, 58, '00:00', {binding: 'clock', fontSize: 48}),
    label('home_caption', 73, 119, 230, 24, 'DEVICE TIME'),
    label('home_connection', 24, 177, 272, 24, 'WIFI STARTING', {binding: 'connection', color: '#53dfc5'}),
    button('home_apps', 180, 205, 124, 28, 'Apps >', 'go:apps'),
  ];
  const apps = screen('apps', 'Apps', []);
  apps.nodes[0].action = 'go:home';
  apps.swipeRight = 'home';
  const tiles: Array<[string, string]> = [
    ['face', 'Face'],
    ['plugins', 'Plugins'],
    ['theme', 'Theme'],
    ['settings', 'Settings'],
    ['terminal', 'Terminal'],
    ['network', 'Network'],
  ];
  for (const [i, [id, text]] of tiles.entries()) {
    const x = 24 + (i % 3) * 100;
    const y = 48 + Math.floor(i / 3) * 88;
    apps.nodes.push(
      button('apps_' + id, x, y, 56, 56, text, 'go:' + id),
      label('apps_' + id + '_caption', x - 2, y + 58, 86, 22, text),
    );
  }
  const theme = screen('theme', 'Theme', [
    label('theme_caption', 20, 58, 280, 24, 'COLOR PALETTE'),
    button('theme_palette', 20, 88, 280, 42, 'Next palette', 'theme_next'),
    button('theme_shape', 20, 172, 280, 42, 'Icon shape', 'shape_toggle'),
  ]);
  const settings = screen('settings', 'Settings', [
    label('settings_wifi', 20, 66, 280, 26, 'Wi-Fi status', {binding: 'connection'}),
    label('settings_display', 20, 114, 280, 24, 'Display 320 x 240'),
    button('settings_appearance', 20, 180, 280, 40, 'Appearance', 'go:theme'),
  ]);
  const network = screen('network', 'Network', [
    label('network_wifi', 20, 66, 280, 26, 'Wi-Fi status', {binding: 'connection'}),
    label('network_help', 20, 120, 280, 22, 'Edge Link: waiting for Space', {binding: 'space'}),
    label('network_pairing', 20, 146, 280, 22, 'Pairing code: waiting', {binding: 'pairing', color: '#53dfc5'}),
    button('network_search', 20, 180, 88, 40, 'Search', 'edge_search'),
    button('network_pair', 116, 180, 88, 40, 'Pair', 'edge_pair'),
    button('network_chat', 212, 180, 88, 40, 'Chat', 'edge_chat'),
  ]);
  const face = screen('face', 'Face', [
    label('face_eyes', 50, 65, 220, 58, 'o   o', {fontSize: 48, color: '#53dfc5'}),
    label('face_expression', 80, 136, 160, 24, 'neutral', {binding: 'expression'}),
    button('face_online_button', 90, 180, 140, 40, 'Online', 'face_online'),
  ]);
  const terminal = screen('terminal', 'Terminal', [
    label('terminal_ready', 20, 70, 280, 24, '> Operit Edge ready', {color: '#53dfc5'}),
    label('terminal_info', 20, 106, 280, 24, 'Local display + Wi-Fi'),
    button('terminal_run', 20, 180, 280, 40, 'Run node', 'run_node'),
  ]);
  const plugins = screen('plugins', 'Plugins', [
    label('plugins_empty', 20, 92, 280, 32, 'No plugins installed'),
  ]);
  return {
    version: 2,
    width: 320,
    height: 240,
    enabled: true,
    entryPage: 'home',
    background: '#091420',
    swipeLeft: 'apps',
    nodes: home,
    pages: [apps, theme, settings, network, face, terminal, plugins],
  };
}
