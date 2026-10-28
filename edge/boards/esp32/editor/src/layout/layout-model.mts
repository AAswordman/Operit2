import {pagesOf, pageOf, findNode, routeTarget, isRecord} from './project-model.mts';
import type {LayoutDocument, LayoutNode, LayoutPage} from './project-model.mts';
import {actions} from './routes.mts';

export {actions, routes, eventBindings} from './routes.mts';

export interface CatalogItem {
  type: string;
  label: string;
  group: string;
  w: number;
  h: number;
  code: number;
  editable: boolean;
  reason: string | null;
}

type CatalogRow = [string, string, string, number, number] | [string, string, string, number, number, string];

const catalogRows: CatalogRow[] = [
  ['panel', '容器', '基础', 100, 70],
  ['label', '文本', '基础', 120, 24],
  ['button', '按钮', '基础', 100, 40],
  ['icon', '图标', '基础', 48, 48],
  ['arc', '圆弧', '数值', 72, 72],
  ['bar', '进度条', '数值', 140, 20],
  ['slider', '滑块', '输入', 140, 28],
  ['switch', '开关', '输入', 54, 28],
  ['checkbox', '复选框', '输入', 140, 28],
  ['dropdown', '下拉框', '输入', 130, 36],
  ['roller', '滚轮', '输入', 100, 80],
  ['textarea', '文本输入', '输入', 180, 42],
  ['spinbox', '数值输入', '输入', 100, 40],
  ['led', '指示灯', '数值', 24, 24],
  ['spinner', '加载动画', '数值', 48, 48],
  ['line', '线条', '基础', 100, 20],
  ['chart', '图表', '数值', 180, 95],
  ['table', '表格', '容器', 180, 80],
  ['buttonmatrix', '按钮矩阵', '输入', 170, 70],
  ['list', '列表', '容器', 150, 95],
  ['calendar', '日历', '容器', 230, 180],
  ['keyboard', '键盘', '输入', 280, 110],
  ['tabview', '选项卡', '容器', 230, 150],
  ['tileview', '滑动页', '容器', 230, 150],
  ['scale', '刻度', '数值', 150, 60],
  ['span', '富文本', '基础', 160, 40],
  ['menu', '菜单', '容器', 180, 130],
  ['msgbox', '消息框', '容器', 220, 130],
  ['win', '窗口', '容器', 220, 150],
  ['image', '图片', '资源', 80, 80, '需要嵌入图片资源，暂不支持拖入'],
  ['animimg', '序列帧', '资源', 80, 80, '需要序列帧资源，暂不支持拖入'],
  ['imagebutton', '图片按钮', '资源', 80, 48, '需要各状态图片，暂不支持拖入'],
  ['canvas', '像素画布', '资源', 100, 80, '需要像素缓冲区，暂不支持拖入'],
];

export const catalog: CatalogItem[] = catalogRows.map(([type, label, group, w, h, reason], index) => ({
  type,
  label,
  group,
  w,
  h,
  code: index,
  editable: !reason,
  reason: reason || null,
}));

const color = /^#[0-9a-fA-F]{6}$/;
const id = /^[a-zA-Z][a-zA-Z0-9_-]{0,39}$/;
const nodeKeys = [
  'id',
  'type',
  'parent',
  'x',
  'y',
  'w',
  'h',
  'text',
  'color',
  'radius',
  'value',
  'action',
  'longAction',
  'binding',
  'fontSize',
];
const pageDocKeys = ['version', 'width', 'height', 'enabled', 'background', 'nodes'];
const projectKeys = [
  'version',
  'width',
  'height',
  'enabled',
  'background',
  'nodes',
  'pages',
  'entryPage',
  'swipeLeft',
  'swipeRight',
];
const pageKeys = ['id', 'name', 'background', 'nodes', 'swipeLeft', 'swipeRight'];
const integerFields = ['x', 'y', 'w', 'h', 'radius', 'value'] as const;
const heavyTypes = ['calendar', 'keyboard', 'menu', 'msgbox', 'win', 'tabview', 'tileview'];
const iconNames = ['face', 'wifi', 'settings', 'home', 'play', 'folder', 'plugins', 'theme', 'terminal'];
const bindings = ['', 'clock', 'connection', 'expression', 'theme', 'pairing', 'space', 'chat'];
const fontSizes = [14, 48];
const documentFields = ['background', 'enabled', 'entryPage', 'swipeLeft', 'swipeRight'];
const pageChangeFields = ['name', 'background', 'swipeLeft', 'swipeRight'];

/** Validates a single page document (v1 shape or one resolved v2 page). */
function validatePage(doc: unknown, allowedActions: readonly string[] = actions): string[] {
  const errors: string[] = [];

  /** Adds one validation error to the current page result. */
  const add = (message: string): void => {
    errors.push(message);
  };

  if (!isRecord(doc)) return ['布局必须是对象'];
  for (const key of Object.keys(doc)) {
    if (!pageDocKeys.includes(key)) add('未知文档字段 ' + key);
  }
  if (doc.version !== 1) add('version 必须为 1');
  if (doc.width !== 320 || doc.height !== 240) add('当前板型仅支持 320×240');
  if (typeof doc.enabled !== 'boolean') add('enabled 必须为布尔值');
  if (typeof doc.background !== 'string' || !color.test(doc.background)) add('background 必须为 #RRGGBB');
  if (!Array.isArray(doc.nodes) || doc.nodes.length > 24) {
    return [...errors, 'nodes 必须为数组，最多 24 个组件'];
  }

  const seen = new Map<string, Record<string, unknown>>();
  let cost = 0;
  for (const [i, raw] of doc.nodes.entries()) {
    const prefix = `nodes[${i}]`;
    if (!isRecord(raw)) {
      add(prefix + ' 无效');
      continue;
    }
    for (const key of Object.keys(raw)) {
      if (!nodeKeys.includes(key)) add(prefix + ' 未知字段 ' + key);
    }
    const item = catalog.find((entry) => entry.type === raw.type);
    if (!item?.editable) add(prefix + ' 组件尚不支持: ' + String(raw.type));
    cost += heavyTypes.includes(String(raw.type)) ? 5 : 1;
    if (typeof raw.id !== 'string' || !id.test(raw.id) || seen.has(raw.id)) {
      add(prefix + ' ID 无效或重复');
    }
    const parent = typeof raw.parent === 'string' && raw.parent ? seen.get(raw.parent) : undefined;
    if (raw.parent && (!parent || parent.type !== 'panel')) {
      add(prefix + ' parent 必须引用前面声明的 panel');
    }
    for (const field of integerFields) {
      if (!Number.isInteger(raw[field])) add(prefix + '.' + field + ' 必须是整数');
    }
    const parentW = typeof parent?.w === 'number' ? parent.w : 320;
    const parentH = typeof parent?.h === 'number' ? parent.h : 240;
    if (
      typeof raw.x !== 'number' ||
      typeof raw.y !== 'number' ||
      typeof raw.w !== 'number' ||
      typeof raw.h !== 'number' ||
      raw.x < 0 ||
      raw.y < 0 ||
      raw.w < 8 ||
      raw.h < 8 ||
      raw.x + raw.w > parentW ||
      raw.y + raw.h > parentH
    ) {
      add(prefix + ' 组件超出父容器');
    }
    if (
      typeof raw.radius !== 'number' ||
      typeof raw.value !== 'number' ||
      raw.radius < 0 ||
      raw.radius > 120 ||
      raw.value < 0 ||
      raw.value > 100
    ) {
      add(prefix + ' radius/value 超限');
    }
    if (
      typeof raw.text !== 'string' ||
      new TextEncoder().encode(raw.text).length > 160 ||
      raw.text.includes('\0')
    ) {
      add(prefix + ' text 最多 160 字节，不能含 NUL');
    }
    // This firmware currently embeds Montserrat only; do not silently render missing glyphs.
    if (typeof raw.text === 'string' && /[^\x20-\x7e\n]/.test(raw.text)) {
      add(prefix + ' 当前固件字体仅支持 ASCII 文本');
    }
    if (typeof raw.color !== 'string' || !color.test(raw.color)) add(prefix + ' color 无效');
    if (typeof raw.action !== 'string' || !allowedActions.includes(raw.action)) {
      add(prefix + ' action 不支持');
    }
    if (
      raw.longAction !== undefined &&
      (typeof raw.longAction !== 'string' || !allowedActions.includes(raw.longAction))
    ) {
      add(prefix + ' longAction 不支持');
    }
    if (raw.type === 'icon' && (typeof raw.text !== 'string' || !iconNames.includes(raw.text))) {
      add(prefix + ' icon text 请选择 face/wifi/settings/home/play/folder');
    }
    if (
      raw.binding !== undefined &&
      (typeof raw.binding !== 'string' || !bindings.includes(raw.binding))
    ) {
      add(prefix + ' binding 不支持');
    }
    if (raw.binding && raw.type !== 'label') add(prefix + ' 动态绑定仅用于文本');
    if (
      raw.fontSize !== undefined &&
      (typeof raw.fontSize !== 'number' || !fontSizes.includes(raw.fontSize))
    ) {
      add(prefix + ' fontSize 仅支持 14/48');
    }
    if (typeof raw.id === 'string') seen.set(raw.id, raw);
  }
  if (cost > 40) add('组件复杂度超出布局文档的编辑器预算 (40)');
  return errors;
}

/** Validates a candidate v1 page or v2 project document. */
export function validate(doc: unknown): string[] {
  if (!isRecord(doc) || doc.version !== 2) return validatePage(doc);
  const errors: string[] = [];
  for (const key of Object.keys(doc as Record<string, unknown>)) {
    if (!projectKeys.includes(key)) errors.push('未知文档字段 ' + key);
  }
  if (!Array.isArray(doc.pages) || doc.pages.length > 11) return ['最多 12 个页面（含首页）'];
  const pages = pagesOf(doc as LayoutDocument);
  const ids = new Set(['home']);
  const nodeIds = new Set<string>();
  for (const raw of doc.pages) {
    if (!isRecord(raw)) return ['页面无效'];
    if (typeof raw.id !== 'string' || !/^[a-zA-Z][a-zA-Z0-9_-]{0,17}$/.test(raw.id) || ids.has(raw.id)) {
      errors.push('页面 ID 无效或重复');
    }
    if (typeof raw.id === 'string') ids.add(raw.id);
    if (typeof raw.name !== 'string' || raw.name.length > 40) errors.push('页面名称最多 40 字符');
    for (const key of Object.keys(raw)) {
      if (!pageKeys.includes(key)) errors.push('未知页面字段 ' + key);
    }
  }
  if (typeof doc.entryPage !== 'string' || !ids.has(doc.entryPage)) errors.push('启动页面不存在');
  const allowed = [...actions, ...pages.map((page) => 'go:' + page.id)];
  for (const page of pages) {
    for (const error of validatePage({
      version: 1,
      width: doc.width,
      height: doc.height,
      enabled: doc.enabled,
      background: page.background,
      nodes: page.nodes,
    }, allowed)) {
      errors.push(page.id + ': ' + error);
    }
    for (const field of ['swipeLeft', 'swipeRight'] as const) {
      if (page[field] && !ids.has(page[field])) errors.push(page.id + ' 滑动目标不存在');
    }
    for (const node of Array.isArray(page.nodes) ? page.nodes : []) {
      if (!node) continue;
      if (nodeIds.has(node.id)) errors.push('跨页面组件 ID 重复 ' + node.id);
      nodeIds.add(node.id);
      for (const field of ['action', 'longAction'] as const) {
        const target = routeTarget(node[field]);
        if (target && !ids.has(target)) errors.push('跳转页面不存在 ' + target);
      }
    }
  }
  return errors;
}

/** Applies a bounded list of layout operations and re-validates the result. */
export function applyOperations(doc: LayoutDocument, operations: unknown): LayoutDocument {
  if (!Array.isArray(operations) || operations.length > 64) throw new Error('operations 最多 64 项');
  const next = structuredClone(doc);
  for (const raw of operations) {
    if (!isRecord(raw)) throw new Error('操作必须是对象');
    const op = raw;
    if (op.op === 'add') {
      const page = pageOf(next, typeof op.pageId === 'string' ? op.pageId : 'home');
      if (!page) throw new Error('页面不存在');
      page.nodes.push(op.node as LayoutNode);
    } else if (op.op === 'update') {
      if (typeof op.id !== 'string') throw new Error('未知组件 ' + String(op.id));
      const found = findNode(next, op.id);
      if (!found) throw new Error('未知组件 ' + op.id);
      const n = found.node;
      const changes = op.changes;
      if (isRecord(changes) && typeof changes.id === 'string' && changes.id !== op.id) throw new Error('不能修改 ID');
      Object.assign(n, changes);
    } else if (op.op === 'remove') {
      if (typeof op.id !== 'string') throw new Error('未知组件 ' + String(op.id));
      for (const page of [next, ...(next.pages || [])]) {
        const remove = new Set([op.id]);
        for (const n of page.nodes) {
          if (n.parent && remove.has(n.parent)) remove.add(n.id);
        }
        page.nodes = page.nodes.filter((n) => !remove.has(n.id));
      }
    } else if (op.op === 'document') {
      const changes = op.changes;
      if (!isRecord(changes)) throw new Error('不可修改文档字段 ');
      for (const k of Object.keys(changes)) {
        if (!documentFields.includes(k)) throw new Error('不可修改文档字段 ' + k);
        (next as Record<string, unknown>)[k] = changes[k];
      }
    } else if (op.op === 'addPage') {
      if (next.version !== 2) throw new Error('需要 v2 项目');
      next.pages!.push(op.page as LayoutPage);
    } else if (op.op === 'updatePage') {
      if (typeof op.id !== 'string') throw new Error('页面不存在');
      const page = pageOf(next, op.id);
      if (!page) throw new Error('页面不存在');
      const changes = op.changes;
      if (!isRecord(changes)) throw new Error('不可修改页面字段 ');
      for (const k of Object.keys(changes)) {
        if (!pageChangeFields.includes(k) || (op.id === 'home' && k === 'name')) throw new Error('不可修改页面字段 ' + k);
        (page as Record<string, unknown>)[k] = changes[k];
      }
    } else if (op.op === 'removePage') {
      if (typeof op.id !== 'string') throw new Error('未知操作 ' + String(op.op));
      if (op.id === 'home') throw new Error('不能删除首页');
      if (next.entryPage === op.id) throw new Error('先更换启动页面');
      next.pages = next.pages!.filter((p) => p.id !== op.id);
      for (const page of [next, ...next.pages]) {
        for (const field of ['swipeLeft', 'swipeRight'] as const) {
          if (page[field] === op.id) page[field] = '';
        }
        for (const n of page.nodes) {
          for (const field of ['action', 'longAction'] as const) {
            if (routeTarget(n[field]) === op.id) n[field] = '';
          }
        }
      }
    } else {
      throw new Error('未知操作 ' + String(op.op));
    }
  }
  const errors = validate(next);
  if (errors.length) throw new Error(errors.join('; '));
  return next;
}
