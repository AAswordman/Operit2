import {applyOperations, catalog, validate} from '../src/layout/layout-model.mjs';
import {findNode, pageOf, pagesOf, projectRoutes} from '../src/layout/project-model.mjs';
import {routes} from '../src/layout/routes.mjs';
import {resizeFromCorner} from '../src/layout/geometry.mjs';
import {request} from './transport.js';
import {setupAI} from './ai.js';
import {setupDeploy} from './deploy.js';
import {setupInteractions} from './interactions.js';
import {setupPages} from './pages.js';
import {errorMessage, query} from './types.js';
import type {
  CatalogItem,
  EditorRuntime,
  EditorSnapshot,
  LayoutDocument,
  LayoutNode,
  LayoutResponse,
  NodePropertyKey,
  PageLike,
  PageSetupOptions,
  PropertyInput,
  Route,
} from './types.js';
import type {ProjectRoute} from '../src/layout/project-model.mts';

type ResizeCorner = 'nw' | 'ne' | 'sw' | 'se';
type NumericNodeProperty = 'x' | 'y' | 'w' | 'h' | 'radius' | 'value' | 'fontSize';
type NodeValue = string | number | null | undefined;

interface DragState {
  id: string;
  startX: number;
  startY: number;
  node: LayoutNode;
  before: LayoutDocument;
  moved: boolean;
  resize: ResizeCorner | null;
}

const propertyLabels: Record<NodePropertyKey, string> = {
  x: '横坐标 X',
  y: '纵坐标 Y',
  w: '宽度 W',
  h: '高度 H',
  radius: '圆角',
  value: '数值 0–100',
  color: '组件颜色',
  text: '显示内容',
  action: '点击动作',
  longAction: '长按动作',
  parent: '父容器',
  binding: '动态内容',
  fontSize: '字号',
};

const symbols: Record<string, string> = {
  panel: '▣',
  label: 'T',
  button: '▭',
  icon: '◈',
  arc: '◔',
  bar: '▰',
  slider: '⊶',
  switch: '⊷',
  checkbox: '☑',
  dropdown: '⌄',
  roller: '≡',
  textarea: '¶',
  spinbox: '±',
  led: '●',
  spinner: '◌',
  line: '╱',
  chart: '▥',
  table: '▦',
  buttonmatrix: '▦',
  list: '☷',
  calendar: '▦',
  keyboard: '⌨',
  tabview: '▤',
  tileview: '▣',
  scale: '┼',
  span: 'Tt',
  menu: '☰',
  msgbox: '▢',
  win: '▣',
  image: '▧',
  animimg: '▧',
  imagebutton: '▧',
  canvas: '▧',
};

const propertyKeys: NodePropertyKey[] = [
  'x',
  'y',
  'w',
  'h',
  'radius',
  'value',
  'color',
  'text',
  'action',
  'longAction',
  'parent',
  'binding',
  'fontSize',
];

const numericProperties = new Set<NumericNodeProperty>([
  'x',
  'y',
  'w',
  'h',
  'radius',
  'value',
  'fontSize',
]);

/** Installs the visual editor and connects it to the shared LVGL runtime. */
export async function setupEditor(ui: EditorRuntime, log: (message: string) => void): Promise<void> {
  const initial = await request<LayoutResponse>('/api/layout');
  const initialErrors = validate(initial.document);
  if (initialErrors.length) throw new Error(initialErrors.join('; '));

  let layout = initial.document;
  let revision = initial.revision;
  let selected: string | null = null;
  let activePageId = layout.entryPage ?? 'home';
  // Open the developer tool in device runtime mode; layout editing remains opt-in.
  let editing = false;
  let holdTimer: number | null = null;
  let holdPointer: {x: number; y: number} | null = null;
  let saved = JSON.stringify(layout);
  let history: LayoutDocument[] = [];
  let future: LayoutDocument[] = [];
  let drag: DragState | null = null;
  let saving = false;
  let remotePending = false;

  const editLayer = query<HTMLElement>('#edit-layer');
  const palette = query<HTMLElement>('#component-list');
  const properties = query<HTMLElement>('#properties');
  const layers = query<HTMLElement>('#layers');

  /** Returns the currently active page and rejects invalid editor state. */
  function currentPage(project: LayoutDocument = layout): PageLike {
    const page = pageOf(project, activePageId);
    if (!page) throw new Error('当前页面不存在：' + activePageId);
    return page;
  }

  /** Creates an independent document snapshot for history and draft editing. */
  function copyDocument(): LayoutDocument {
    return structuredClone(layout);
  }

  /** Reports whether the current document differs from the saved revision. */
  function isDirty(): boolean {
    return JSON.stringify(layout) !== saved;
  }

  /** Shows a status message in the editor header. */
  function notify(message: string): void {
    query<HTMLElement>('#editor-status').textContent = message;
  }

  /** Returns the complete state consumed by editor integrations. */
  function snapshot(): EditorSnapshot {
    return {
      document: copyDocument(),
      revision,
      dirty: isDirty(),
      pageId: activePageId,
      componentId: selected,
    };
  }

  /** Returns a catalog item for a known layout node type. */
  function catalogItem(type: string): CatalogItem {
    const item = catalog.find((candidate) => candidate.type === type);
    if (!item) throw new Error('未知组件类型：' + type);
    return item;
  }

  /** Returns a route descriptor for a known route id. */
  function routeItem(layout: LayoutDocument, id: string): Route | ProjectRoute {
    const item = [...projectRoutes(layout), ...routes].find((route) => route.id === id);
    if (!item) throw new Error('未知路由：' + id);
    return item;
  }

  /** Returns the selected component in a page snapshot. */
  function nodeInPage(project: LayoutDocument, id: string): LayoutNode {
    const node = currentPage(project).nodes.find((candidate) => candidate.id === id);
    if (!node) throw new Error('组件不存在：' + id);
    return node;
  }

  /** Returns one editable property using its declared field type. */
  function nodeValue(node: LayoutNode, key: NodePropertyKey): NodeValue {
    switch (key) {
      case 'x':
        return node.x;
      case 'y':
        return node.y;
      case 'w':
        return node.w;
      case 'h':
        return node.h;
      case 'radius':
        return node.radius;
      case 'value':
        return node.value;
      case 'color':
        return node.color;
      case 'text':
        return node.text;
      case 'action':
        return node.action;
      case 'longAction':
        return node.longAction;
      case 'parent':
        return node.parent;
      case 'binding':
        return node.binding;
      case 'fontSize':
        return node.fontSize;
    }
  }

  /** Writes one editable property using its declared field type. */
  function setNodeValue(node: LayoutNode, key: NodePropertyKey, value: string): void {
    switch (key) {
      case 'x':
        node.x = Number(value);
        break;
      case 'y':
        node.y = Number(value);
        break;
      case 'w':
        node.w = Number(value);
        break;
      case 'h':
        node.h = Number(value);
        break;
      case 'radius':
        node.radius = Number(value);
        break;
      case 'value':
        node.value = Number(value);
        break;
      case 'fontSize':
        node.fontSize = Number(value);
        break;
      case 'color':
        node.color = value;
        break;
      case 'text':
        node.text = value;
        break;
      case 'action':
        node.action = value;
        break;
      case 'longAction':
        node.longAction = value;
        break;
      case 'parent':
        node.parent = value || null;
        break;
      case 'binding':
        node.binding = value;
        break;
    }
  }

  /** Updates the enabled state and labels of editor actions. */
  function updateButtons(): void {
    const saveButton = query<HTMLButtonElement>('#save-layout');
    saveButton.disabled = saving || !isDirty();
    saveButton.textContent = saving ? '保存中…' : '保存项目';
    query<HTMLButtonElement>('#undo-layout').disabled = history.length === 0;
    query<HTMLButtonElement>('#redo-layout').disabled = future.length === 0;
    for (const id of ['delete-node', 'duplicate-node', 'inspect-node', 'component-logic']) {
      query<HTMLButtonElement>('#' + id).disabled = selected === null;
    }
  }

  /** Reports the current draft state after an editor mutation. */
  function updateChangeStatus(): void {
    updateButtons();
    notify(isDirty() ? '未保存 · 保存项目后可下发到设备，无需编译' : '已恢复到保存版本');
  }

  /** Stores one document snapshot and clears the redo stack. */
  function checkpoint(): void {
    history.push(copyDocument());
    if (history.length > 20) history.shift();
    future = [];
  }

  /** Computes a node position in screen coordinates through its parent chain. */
  function absolutePosition(node: LayoutNode): {x: number; y: number} {
    let x = node.x;
    let y = node.y;
    let parentId = node.parent;
    while (parentId) {
      const parent = currentPage().nodes.find((candidate) => candidate.id === parentId);
      if (!parent) throw new Error('组件父容器不存在：' + parentId);
      x += parent.x;
      y += parent.y;
      parentId = parent.parent;
    }
    return {x, y};
  }

  /** Renders the active page through the shared LVGL WebAssembly runtime. */
  function preview(): void {
    const page = currentPage();
    const errors = validate(layout);
    if (errors.length) {
      notify(errors.join('; '));
      return;
    }
    ui._operit_lvgl_layout_clear(parseInt(page.background.slice(1), 16));
    if (ui._operit_lvgl_layout_page_meta) {
      ui.ccall(
        'operit_lvgl_layout_page_meta',
        null,
        ['string', 'string', 'string'],
        [activePageId, page.swipeLeft ?? '', page.swipeRight ?? ''],
      );
    }
    page.nodes.forEach((node, index) => {
      const item = catalogItem(node.type);
      const parent = node.parent ? page.nodes.findIndex((candidate) => candidate.id === node.parent) : -1;
      if (node.parent && parent < 0) throw new Error('组件父容器不存在：' + node.parent);
      const result = ui.ccall(
        'operit_lvgl_layout_add',
        'number',
        ['number', 'number', 'number', 'number', 'number', 'number', 'number', 'number', 'number', 'string', 'string'],
        [
          item.code,
          parent,
          node.x,
          node.y,
          node.w,
          node.h,
          parseInt(node.color.slice(1), 16),
          node.radius,
          node.value,
          node.text,
          node.action,
        ],
      );
      if (result < 0) {
        notify('LVGL 组件创建失败：' + node.id);
      } else if (ui._operit_lvgl_layout_bind) {
        ui.ccall(
          'operit_lvgl_layout_bind',
          null,
          ['number', 'string', 'string'],
          [result, node.action, node.longAction ?? ''],
        );
        if (ui._operit_lvgl_layout_style) {
          ui.ccall(
            'operit_lvgl_layout_style',
            null,
            ['number', 'string', 'number'],
            [result, node.binding ?? '', node.fontSize ?? 14],
          );
        }
      } else if (node.longAction || node.action.startsWith('page:')) {
        notify('当前预览尚未包含新路由，请等待同源构建完成');
      }
      void index;
    });
  }

  /** Enters layout editing mode and reveals the interaction layer. */
  function enterEditMode(): void {
    editing = true;
    editLayer.hidden = false;
    query<HTMLInputElement>('#edit-mode').checked = true;
  }

  /** Commits and previews a validated document snapshot. */
  function commit(next: LayoutDocument, selection: string | null = selected): boolean {
    const errors = validate(next);
    if (errors.length) {
      notify(errors.join('; '));
      return false;
    }
    if (JSON.stringify(next) === JSON.stringify(layout)) return false;
    checkpoint();
    layout = next;
    selected = selection;
    enterEditMode();
    preview();
    render();
    updateChangeStatus();
    return true;
  }

  /** Cancels an active touch hold timer. */
  function cancelHold(): void {
    if (holdTimer !== null) window.clearTimeout(holdTimer);
    holdTimer = null;
    holdPointer = null;
  }

  /** Selects a page and switches between edit mode and runtime mode. */
  function selectPage(id: string, run = false): void {
    if (!pageOf(layout, id)) return;
    finishDrag();
    activePageId = id;
    selected = null;
    editing = !run;
    editLayer.hidden = run;
    query<HTMLInputElement>('#edit-mode').checked = !run;
    preview();
    render();
    notify(run ? '运行页面 · ' + id : '正在编辑页面 · ' + id);
  }

  /** Opens the component logic inspector for a selected component. */
  function openLogic(id: string): void {
    cancelHold();
    selected = id;
    drag = null;
    render();
    interactions.open(id);
  }

  /** Moves one selected component by a bounded pixel delta. */
  function nudge(dx: number, dy: number): void {
    if (!selected) return;
    const next = copyDocument();
    const node = nodeInPage(next, selected);
    const parent = node.parent ? currentPage(next).nodes.find((candidate) => candidate.id === node.parent) : undefined;
    const width = parent?.w ?? 320;
    const height = parent?.h ?? 240;
    node.x = Math.max(0, Math.min(width - node.w, node.x + dx));
    node.y = Math.max(0, Math.min(height - node.h, node.y + dy));
    commit(next);
  }

  /** Creates a property input with options matching the selected node field. */
  function propertyInput(node: LayoutNode, key: NodePropertyKey): PropertyInput {
    const isRoute = key === 'action' || key === 'longAction';
    const isIconText = key === 'text' && node.type === 'icon';
    const isSelect = isRoute || isIconText || key === 'parent' || key === 'binding' || key === 'fontSize';
    if (isSelect) {
      const input = document.createElement('select');
      let options: string[];
      if (isRoute) {
        options = [...projectRoutes(layout), ...routes].map((route) => route.id);
      } else if (isIconText) {
        options = ['face', 'wifi', 'settings', 'home', 'play', 'folder'];
      } else if (key === 'binding') {
        options = ['', 'clock', 'connection', 'expression'];
      } else if (key === 'fontSize') {
        options = ['14', '48'];
      } else {
        options = [
          '',
          ...currentPage().nodes
            .slice(0, currentPage().nodes.findIndex((candidate) => candidate.id === node.id))
            .filter((candidate) => candidate.type === 'panel')
            .map((candidate) => candidate.id),
        ];
      }
      for (const option of options) {
        const label = isRoute ? routeItem(layout, option).label : option || '无';
        input.add(new Option(label, option));
      }
      return input;
    }
    return document.createElement(key === 'text' ? 'textarea' : 'input');
  }

  /** Renders all editable properties for the selected component. */
  function renderProperties(): void {
    properties.replaceChildren();
    const node = selected ? currentPage().nodes.find((candidate) => candidate.id === selected) : undefined;
    if (!node) {
      const empty = document.createElement('div');
      empty.className = 'empty-state';
      empty.innerHTML = '<span class="empty-icon">⌖</span>选择画布上的组件<br>或从组件库添加一个';
      properties.append(empty);
      return;
    }

    const title = document.createElement('strong');
    title.textContent = catalogItem(node.type).label + ' / ' + node.id;
    properties.append(title);

    for (const key of propertyKeys) {
      const label = document.createElement('label');
      label.textContent = propertyLabels[key];
      if (['text', 'action', 'longAction', 'parent', 'color', 'binding'].includes(key)) {
        label.className = 'wide';
      }
      const input = propertyInput(node, key);
      if (input instanceof HTMLInputElement) {
        input.type = key === 'color' ? 'color' : numericProperties.has(key as NumericNodeProperty) ? 'number' : 'text';
        if (numericProperties.has(key as NumericNodeProperty)) {
          input.step = '1';
          input.min = ['w', 'h'].includes(key) ? '8' : '0';
        }
      }
      const value = nodeValue(node, key);
      input.value = value === null || value === undefined ? (key === 'fontSize' ? '14' : '') : String(value);
      input.dataset.property = key;
      input.addEventListener('change', () => {
        const next = copyDocument();
        const changed = nodeInPage(next, node.id);
        setNodeValue(changed, key, input.value);
        if (!commit(next)) renderProperties();
      });
      label.append(input);
      properties.append(label);
    }

    const nudgeControls = document.createElement('div');
    nudgeControls.className = 'nudge-controls';
    const nudges: Array<[string, number, number, string]> = [
      ['←', -1, 0, '左移 1 像素'],
      ['↑', 0, -1, '上移 1 像素'],
      ['↓', 0, 1, '下移 1 像素'],
      ['→', 1, 0, '右移 1 像素'],
    ];
    for (const [text, dx, dy, name] of nudges) {
      const button = document.createElement('button');
      button.textContent = text;
      button.title = name;
      button.setAttribute('aria-label', name);
      button.addEventListener('click', () => nudge(dx, dy));
      nudgeControls.append(button);
    }
    properties.append(nudgeControls);

    if (node.type === 'buttonmatrix') {
      const expand = document.createElement('button');
      expand.className = 'wide';
      expand.textContent = '展开为可编辑子按钮';
      expand.addEventListener('click', () => {
        try {
          const next = copyDocument();
          const page = currentPage(next);
          const index = page.nodes.findIndex((candidate) => candidate.id === node.id);
          if (index < 0) throw new Error('组件不存在：' + node.id);
          page.nodes.splice(index, 1, ...expandMatrix(node));
          commit(next);
        } catch (error) {
          notify(errorMessage(error));
        }
      });
      properties.append(expand);
    }
  }

  /** Renders the editable overlays and layer list for the active page. */
  function render(): void {
    if (!pageOf(layout, activePageId)) activePageId = layout.entryPage ?? 'home';
    const page = currentPage();
    if (!page.nodes.some((node) => node.id === selected)) selected = null;

    editLayer.replaceChildren();
    for (const node of page.nodes) {
      const element = document.createElement('div');
      const position = absolutePosition(node);
      element.className = 'editable-node' + (selected === node.id ? ' selected' : '');
      element.dataset.id = node.id;
      element.style.cssText =
        `left:${position.x / 320 * 100}%;top:${position.y / 240 * 100}%;` +
        `width:${node.w / 320 * 100}%;height:${node.h / 240 * 100}%`;
      element.title = node.id + ' · ' + node.type;
      if (selected === node.id) {
        const tag = document.createElement('span');
        tag.textContent = node.id;
        element.append(tag);
        for (const corner of ['nw', 'ne', 'sw', 'se'] as ResizeCorner[]) {
          const handle = document.createElement('i');
          handle.className = 'resize-handle handle-' + corner;
          handle.dataset.corner = corner;
          element.append(handle);
        }
      }
      editLayer.append(element);
    }

    renderProperties();
    layers.replaceChildren();
    for (const node of page.nodes) {
      const button = document.createElement('button');
      button.textContent = (node.parent ? '↳ ' : '') + node.id;
      const type = document.createElement('small');
      type.textContent = catalogItem(node.type).label;
      button.append(type);
      button.classList.toggle('active', selected === node.id);
      button.setAttribute('aria-pressed', String(selected === node.id));
      button.addEventListener('click', () => {
        selected = node.id;
        enterEditMode();
        preview();
        render();
      });
      layers.append(button);
    }
    query<HTMLElement>('#node-count').textContent = `${page.nodes.length} / 24`;
    query<HTMLElement>('#selection-name').textContent = selected ?? '选择组件开始编辑';
    query<HTMLInputElement>('#layout-enabled').checked = layout.enabled;
    query<HTMLInputElement>('#layout-bg').value = page.background;
    updateButtons();
    pages.render();
    ai.updateContext();
  }

  /** Adds a new catalog component at the requested logical position. */
  function add(type: string, x = 24, y = 48): void {
    const item = catalogItem(type);
    if (!item.editable) return;
    const node: LayoutNode = {
      id: `${type}_${Date.now().toString(36)}`,
      type,
      parent: null,
      x: Math.max(0, Math.min(320 - item.w, Math.round(x / 4) * 4)),
      y: Math.max(0, Math.min(240 - item.h, Math.round(y / 4) * 4)),
      w: item.w,
      h: item.h,
      text: type === 'icon' ? 'face' : ['dropdown', 'roller'].includes(type) ? 'One\nTwo\nThree' : type === 'panel' ? '' : '',
      color: ['label', 'icon', 'span'].includes(type) ? '#f4f8ff' : '#216c73',
      radius: 12,
      value: 50,
      action: '',
    };
    const next = copyDocument();
    currentPage(next).nodes.push(...(type === 'buttonmatrix' ? expandMatrix(node) : [node]));
    if (commit(next, node.id)) window.dispatchEvent(new Event('operit-component-added'));
  }

  /** Expands a button matrix into independently editable child nodes. */
  function expandMatrix(node: LayoutNode): LayoutNode[] {
    const {w, h} = node;
    const gap = 4;
    const half = Math.floor((w - gap) / 2);
    const row = Math.floor((h - gap) / 2);
    if (half < 8 || row < 8) throw new Error('矩阵至少需要 20 × 20 像素才能展开');
    const labels = node.text.includes('|') ? node.text.split('|') : ['One', 'Two', 'Three'];
    const cells: Array<[number, number, number, number]> = [
      [0, 0, half, row],
      [half + gap, 0, w - half - gap, row],
      [0, row + gap, w, h - row - gap],
    ];
    return [
      {...node, type: 'panel', text: '', action: '', longAction: ''},
      ...cells.map(([cellX, cellY, cellW, cellH], index) => ({
        ...node,
        id: node.id.slice(0, 33) + '_cell' + index,
        type: 'button',
        parent: node.id,
        x: cellX,
        y: cellY,
        w: cellW,
        h: cellH,
        text: labels[index] ?? `Button ${index + 1}`,
        radius: Math.min(node.radius, 8),
      })),
    ];
  }

  /** Opens the selected component inspector after a long press or context click. */
  function openSelectedLogic(): void {
    if (selected) openLogic(selected);
  }

  /** Starts a pointer drag or a touch-hold component action. */
  function beginPointer(event: PointerEvent): void {
    cancelHold();
    if (event.button !== 0) return;
    const element = event.target instanceof Element ? event.target.closest<HTMLElement>('.editable-node') : null;
    if (!element) {
      selected = null;
      render();
      return;
    }
    event.preventDefault();
    const id = element.dataset.id;
    if (!id) throw new Error('编辑组件缺少 ID');
    selected = id;
    const node = currentPage().nodes.find((candidate) => candidate.id === id);
    if (!node) throw new Error('组件不存在：' + id);
    drag = {
      id,
      startX: event.clientX,
      startY: event.clientY,
      node: {...node},
      before: copyDocument(),
      moved: false,
      resize: event.target instanceof HTMLElement ? (event.target.dataset.corner as ResizeCorner | undefined) ?? null : null,
    };
    editLayer.setPointerCapture(event.pointerId);
    render();
    if (event.pointerType !== 'mouse' && !drag.resize) {
      holdPointer = {x: event.clientX, y: event.clientY};
      holdTimer = window.setTimeout(() => {
        if (!drag || drag.moved) return;
        const logicId = drag.id;
        if (editLayer.hasPointerCapture(event.pointerId)) editLayer.releasePointerCapture(event.pointerId);
        openLogic(logicId);
      }, 550);
    }
  }

  /** Updates the dragged component geometry during a pointer move. */
  function movePointer(event: PointerEvent): void {
    if (!drag) return;
    if (holdPointer) {
      if (Math.hypot(event.clientX - holdPointer.x, event.clientY - holdPointer.y) <= 8) return;
      cancelHold();
    }
    const rectangle = editLayer.getBoundingClientRect();
    const dx = Math.round((event.clientX - drag.startX) * 320 / rectangle.width / 4) * 4;
    const dy = Math.round((event.clientY - drag.startY) * 240 / rectangle.height / 4) * 4;
    const node = nodeInPage(layout, drag.id);
    const parent = node.parent ? currentPage().nodes.find((candidate) => candidate.id === node.parent) : undefined;
    const width = parent?.w ?? 320;
    const height = parent?.h ?? 240;
    if (!drag.moved && !dx && !dy) return;
    if (!drag.moved) {
      checkpoint();
      drag.moved = true;
    }
    if (drag.resize) {
      Object.assign(
        node,
        resizeFromCorner(
          drag.node,
          drag.resize,
          dx,
          dy,
          width,
          height,
          currentPage().nodes.filter((candidate) => candidate.parent === node.id),
        ),
      );
    } else {
      node.x = Math.max(0, Math.min(width - node.w, drag.node.x + dx));
      node.y = Math.max(0, Math.min(height - node.h, drag.node.y + dy));
    }
    ui._operit_lvgl_layout_geometry(currentPage().nodes.indexOf(node), node.x, node.y, node.w, node.h);
    renderOverlays();
    updateChangeStatus();
  }

  /** Completes a pointer drag and refreshes the editor panels. */
  function finishDrag(): void {
    cancelHold();
    if (!drag) return;
    if (drag.moved && JSON.stringify(drag.before) === JSON.stringify(layout)) history.pop();
    drag = null;
    render();
  }

  /** Renders only the component overlays while a drag is active. */
  function renderOverlays(): void {
    const page = currentPage();
    editLayer.replaceChildren();
    for (const node of page.nodes) {
      const element = document.createElement('div');
      const position = absolutePosition(node);
      element.className = 'editable-node' + (selected === node.id ? ' selected' : '');
      element.dataset.id = node.id;
      element.style.cssText =
        `left:${position.x / 320 * 100}%;top:${position.y / 240 * 100}%;` +
        `width:${node.w / 320 * 100}%;height:${node.h / 240 * 100}%`;
      editLayer.append(element);
    }
  }

  /** Restores a previous document from one history stack into the other. */
  function travel(from: LayoutDocument[], to: LayoutDocument[]): void {
    if (!from.length) return;
    to.push(copyDocument());
    if (to.length > 20) to.shift();
    const next = from.pop();
    if (!next) throw new Error('历史记录状态错误');
    layout = next;
    enterEditMode();
    preview();
    render();
    updateChangeStatus();
  }

  /** Saves the current document with optimistic revision checking. */
  async function saveLayout(): Promise<void> {
    if (saving || !isDirty()) return;
    const submitted = copyDocument();
    const errors = validate(submitted);
    if (errors.length) {
      notify(errors.join('; '));
      return;
    }
    saving = true;
    updateButtons();
    try {
      const result = await request<LayoutResponse>('/api/layout', {
        method: 'PUT',
        body: {revision, document: submitted},
      });
      revision = result.revision;
      saved = JSON.stringify(submitted);
      notify(
        isDirty()
          ? '已保存提交版本；还有新修改未保存'
          : `已写入 apps/esp32/ui/layout.json · ${revision.slice(0, 8)} · 可部署到设备`,
      );
      log('layout saved ' + revision.slice(0, 12));
    } catch (error) {
      notify(errorMessage(error));
    } finally {
      saving = false;
      updateButtons();
    }
  }

  /** Reloads the saved layout after preserving the current draft decision. */
  async function reloadLayout(): Promise<void> {
    if (saving) return;
    if (isDirty() && !window.confirm('丢弃未保存的布局修改并重新读取？')) return;
    try {
      const before = JSON.stringify(layout);
      const result = await request<LayoutResponse>('/api/layout');
      if (JSON.stringify(layout) !== before || saving || drag) {
        notify('读取期间产生新修改，已保留草稿');
        return;
      }
      layout = result.document;
      revision = result.revision;
      saved = JSON.stringify(layout);
      history = [];
      future = [];
      selected = null;
      enterEditMode();
      preview();
      render();
      notify('已读取最新布局');
    } catch (error) {
      notify(errorMessage(error));
    }
  }

  /** Handles keyboard shortcuts for saving, history and component movement. */
  function handleKeydown(event: KeyboardEvent): void {
    const target = event.target;
    const typing =
      target instanceof Element && target.closest('input,textarea,select,[contenteditable=true]') !== null;
    const modifier = event.ctrlKey || event.metaKey;
    if (modifier && event.key.toLowerCase() === 's') {
      event.preventDefault();
      void saveLayout();
      return;
    }
    if (
      typing ||
      !editing ||
      query<HTMLDialogElement>('#component-dialog').open ||
      query<HTMLDialogElement>('#ai-dialog').open ||
      query<HTMLDialogElement>('#pages-dialog').open ||
      query<HTMLDialogElement>('#deploy-dialog').open
    ) {
      return;
    }
    if (selected && (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10'))) {
      event.preventDefault();
      openLogic(selected);
      return;
    }
    if (modifier && ['z', 'y', 'd'].includes(event.key.toLowerCase())) {
      event.preventDefault();
      const key = event.key.toLowerCase();
      if (key === 'd') query<HTMLButtonElement>('#duplicate-node').click();
      else if (key === 'y' || event.shiftKey) query<HTMLButtonElement>('#redo-layout').click();
      else query<HTMLButtonElement>('#undo-layout').click();
      return;
    }
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault();
      query<HTMLButtonElement>('#delete-node').click();
      return;
    }
    if (selected && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) {
      event.preventDefault();
      const step = event.shiftKey ? 8 : 1;
      nudge(
        event.key === 'ArrowLeft' ? -step : event.key === 'ArrowRight' ? step : 0,
        event.key === 'ArrowUp' ? -step : event.key === 'ArrowDown' ? step : 0,
      );
    }
  }

  /** Keeps the local draft warning active while unsaved changes exist. */
  function handleBeforeUnload(event: BeforeUnloadEvent): void {
    if (!isDirty()) return;
    event.preventDefault();
    event.returnValue = '';
  }

  /** Synchronizes clean external layout changes without overwriting a local draft. */
  async function pollExternalLayout(): Promise<void> {
    if (remotePending || bridge.isBusy() || documentHidden()) return;
    remotePending = true;
    try {
      const requestedRevision = revision;
      const current = await request<LayoutResponse>('/api/layout');
      if (saving || drag || revision !== requestedRevision) return;
      if (current.revision !== revision) {
        if (isDirty()) {
          notify('AI 或其他编辑器已修改布局 · 草稿已保留，请在图层面板重新读取；读取会询问是否丢弃草稿');
          return;
        }
        const errors = validate(current.document);
        if (errors.length) {
          notify('外部布局无效，保留当前页面');
          return;
        }
        layout = current.document;
        revision = current.revision;
        saved = JSON.stringify(layout);
        history = [];
        future = [];
        if (editing) preview();
        render();
        notify('已同步 AI / 外部编辑器的布局修改');
      }
    } catch {
      return;
    } finally {
      remotePending = false;
    }
  }

  /** Reports whether the editor is occupied by a modal or active pointer operation. */
  function isBusy(): boolean {
    return (
      saving ||
      drag !== null ||
      query<HTMLDialogElement>('#component-dialog').open ||
      query<HTMLDialogElement>('#ai-dialog').open ||
      query<HTMLDialogElement>('#pages-dialog').open ||
      query<HTMLDialogElement>('#deploy-dialog').open
    );
  }

  /** Reads document visibility without coupling editor logic to browser globals. */
  function documentHidden(): boolean {
    return documentVisibilityState() === 'hidden';
  }

  /** Returns the current browser document visibility state. */
  function documentVisibilityState(): DocumentVisibilityState {
    return window.document.visibilityState;
  }

  const interactions = setupInteractions({snapshot, commit, notify});
  const pages = setupPages({snapshot, commit, selectPage});
  const ai = setupAI({snapshot, commit, notify});
  setupDeploy({snapshot, notify});
  const bridge = {isDirty, isEditing: () => editing, isBusy};

  /** Handles navigation events emitted by the runtime preview. */
  function handlePageNavigation(event: Event): void {
    const detail = (event as CustomEvent<string>).detail;
    if (typeof detail !== 'string') throw new Error('页面导航事件缺少页面 ID');
    selectPage(detail, true);
  }

  /** Handles navigation from the built-in runtime controls. */
  function handleRuntimeNavigation(): void {
    editing = false;
    editLayer.hidden = true;
    query<HTMLInputElement>('#edit-mode').checked = false;
    notify('运行内置页面 · 开启编辑布局可返回草稿');
  }

  /** Handles a drag-and-drop component from the palette onto the canvas. */
  function dropComponent(event: DragEvent): void {
    event.preventDefault();
    if (!event.dataTransfer) throw new Error('拖拽数据不可用');
    const rectangle = editLayer.getBoundingClientRect();
    add(
      event.dataTransfer.getData('text/plain'),
      (event.clientX - rectangle.left) * 320 / rectangle.width,
      (event.clientY - rectangle.top) * 240 / rectangle.height,
    );
  }

  /** Creates the component palette and its search index. */
  function renderPalette(): void {
    for (const group of [...new Set(catalog.map((item) => item.group))]) {
      const section = document.createElement('section');
      section.className = 'component-group';
      const heading = document.createElement('h3');
      heading.textContent = group;
      section.append(heading);
      for (const item of catalog.filter((candidate) => candidate.group === group)) {
        const button = document.createElement('button');
        button.className = 'component-card';
        button.dataset.type = item.type;
        button.dataset.search = `${item.label} ${item.type}`.toLowerCase();
        const symbol = document.createElement('span');
        symbol.className = 'component-symbol';
        symbol.textContent = symbols[item.type] ?? '';
        symbol.setAttribute('aria-hidden', 'true');
        const name = document.createElement('span');
        name.textContent = item.label;
        const code = document.createElement('small');
        code.textContent = item.type;
        button.append(symbol, name, code);
        if (!item.editable) {
          const reason = document.createElement('small');
          reason.className = 'component-reason';
          reason.textContent = '需资源接入';
          button.append(reason);
        }
        button.draggable = item.editable;
        button.disabled = !item.editable;
        button.title = item.reason ?? '拖到屏幕或点击添加';
        button.addEventListener('dragstart', (event: DragEvent) => {
          if (!event.dataTransfer) throw new Error('拖拽数据不可用');
          event.dataTransfer.setData('text/plain', item.type);
        });
        button.addEventListener('click', () => add(item.type));
        section.append(button);
      }
      palette.append(section);
    }
  }

  /** Filters the component palette by its visible label and type. */
  function filterPalette(): void {
    const input = query<HTMLInputElement>('#component-search');
    const term = input.value.trim().toLowerCase();
    let visible = 0;
    for (const section of palette.querySelectorAll<HTMLElement>('.component-group')) {
      let count = 0;
      for (const button of section.querySelectorAll<HTMLButtonElement>('button')) {
        const match = (button.dataset.search ?? '').includes(term);
        button.hidden = !match;
        if (match) count += 1;
      }
      section.hidden = count === 0;
      visible += count;
    }
    query<HTMLElement>('#search-empty').hidden = visible !== 0;
  }

  renderPalette();
  query<HTMLInputElement>('#component-search').addEventListener('input', filterPalette);
  editLayer.addEventListener('dragover', (event: DragEvent) => event.preventDefault());
  editLayer.addEventListener('drop', dropComponent);
  editLayer.addEventListener('contextmenu', (event: MouseEvent) => {
    if (!editing) return;
    const element = event.target instanceof Element ? event.target.closest<HTMLElement>('.editable-node') : null;
    const id = element?.dataset.id;
    if (!id) return;
    event.preventDefault();
    openLogic(id);
  });
  query<HTMLButtonElement>('#component-logic').addEventListener('click', openSelectedLogic);
  editLayer.addEventListener('pointerdown', beginPointer);
  editLayer.addEventListener('pointermove', movePointer);
  editLayer.addEventListener('pointerup', finishDrag);
  editLayer.addEventListener('pointercancel', finishDrag);
  editLayer.addEventListener('lostpointercapture', finishDrag);
  query<HTMLButtonElement>('#delete-node').addEventListener('click', () => {
    if (!selected) return;
    const next = copyDocument();
    const ids = new Set([selected]);
    for (const node of currentPage(next).nodes) {
      if (node.parent && ids.has(node.parent)) ids.add(node.id);
    }
    currentPage(next).nodes = currentPage(next).nodes.filter((node) => !ids.has(node.id));
    commit(next, null);
  });
  query<HTMLButtonElement>('#duplicate-node').addEventListener('click', () => {
    if (!selected) return;
    const next = copyDocument();
    const ids = new Map<string, string>();
    const original = nodeInPage(layout, selected);
    const parent = original.parent ? currentPage().nodes.find((node) => node.id === original.parent) : undefined;
    for (const node of currentPage().nodes) {
      if (node.id !== selected && !ids.has(node.parent ?? '')) continue;
      const clone: LayoutNode = {...node, id: `${node.type}_${Date.now().toString(36)}_${ids.size}`};
      ids.set(node.id, clone.id);
      if (node.id === selected) {
        clone.x = Math.min((parent?.w ?? 320) - node.w, node.x + 8);
        clone.y = Math.min((parent?.h ?? 240) - node.h, node.y + 8);
      } else {
        const parentId = ids.get(node.parent ?? '');
        if (!parentId) throw new Error('复制组件父容器不存在：' + node.parent);
        clone.parent = parentId;
      }
      currentPage(next).nodes.push(clone);
    }
    commit(next, ids.get(selected) ?? null);
  });
  query<HTMLButtonElement>('#undo-layout').addEventListener('click', () => travel(history, future));
  query<HTMLButtonElement>('#redo-layout').addEventListener('click', () => travel(future, history));
  query<HTMLInputElement>('#edit-mode').addEventListener('change', (event: Event) => {
    const input = event.currentTarget;
    if (!(input instanceof HTMLInputElement)) throw new Error('编辑模式控件类型错误');
    editing = input.checked;
    editLayer.hidden = !editing;
    if (editing) preview(); else ui._operit_lvgl_navigate_home();
    notify(editing ? '编辑模式 · 拖动组件，四角调整尺寸' : '运行模式 · 点击或滑动体验 LVGL 控件');
  });
  query<HTMLInputElement>('#layout-enabled').addEventListener('change', (event: Event) => {
    const input = event.currentTarget;
    if (!(input instanceof HTMLInputElement)) throw new Error('布局开关控件类型错误');
    const next = copyDocument();
    next.enabled = input.checked;
    commit(next);
  });
  query<HTMLInputElement>('#layout-bg').addEventListener('change', (event: Event) => {
    const input = event.currentTarget;
    if (!(input instanceof HTMLInputElement)) throw new Error('背景颜色控件类型错误');
    const next = copyDocument();
    currentPage(next).background = input.value;
    commit(next);
  });
  query<HTMLButtonElement>('#save-layout').addEventListener('click', () => void saveLayout());
  query<HTMLButtonElement>('#reload-layout').addEventListener('click', () => void reloadLayout());
  window.addEventListener('operit-navigate-page', handlePageNavigation);
  window.addEventListener('operit-runtime-navigation', handleRuntimeNavigation);
  window.addEventListener('keydown', handleKeydown);
  window.addEventListener('beforeunload', handleBeforeUnload);
  window.operitEditor = bridge;
  window.setInterval(() => void pollExternalLayout(), 2000);

  editLayer.hidden = !editing;
  query<HTMLInputElement>('#edit-mode').checked = editing;
  ui._operit_lvgl_navigate_home();
  render();
  notify('运行实际首页 · 开启编辑布局可修改草稿');
}
