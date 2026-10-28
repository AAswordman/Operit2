import {applyOperations} from '../src/layout/layout-model.mjs';
import {pageEdges, pageOf, pagesOf} from '../src/layout/project-model.mjs';
import {errorMessage, query} from './types.js';
import type {LayoutDocument, LayoutPage} from './types.js';
import type {PageSetupOptions} from './types.js';

type PageLinkField = 'swipeLeft' | 'swipeRight';

interface GraphPoint {
  x: number;
  y: number;
}

const pageLinkFields: Array<[string, PageLinkField]> = [
  ['page-swipe-left', 'swipeLeft'],
  ['page-swipe-right', 'swipeRight'],
];

/** Installs page management, navigation graph and page-level settings. */
export function setupPages({snapshot, commit, selectPage}: PageSetupOptions): {render: () => void} {
  const dialog = query<HTMLDialogElement>('#pages-dialog');
  const picker = query<HTMLSelectElement>('#page-picker');
  const pageFeedback = query<HTMLElement>('#pages-feedback');
  const pageNameInput = query<HTMLInputElement>('#page-name');
  const entryPageSelect = query<HTMLSelectElement>('#entry-page');
  const newPageNameInput = query<HTMLInputElement>('#new-page-name');
  const removePageButton = query<HTMLButtonElement>('#remove-page');
  const graph = query<HTMLElement>('#page-graph');
  const connectionList = query<HTMLElement>('#page-connections');

  /** Applies page operations to the current editor document. */
  function apply(operations: unknown[]): boolean {
    try {
      const state = snapshot();
      return commit(applyOperations(state.document, operations));
    } catch (error) {
      pageFeedback.textContent = errorMessage(error);
      return false;
    }
  }

  /** Returns the page selected in the current document. */
  function activePage(layout: LayoutDocument, id: string): LayoutPage {
    const page = pagesOf(layout).find((item) => item.id === id);
    if (!page) throw new Error('页面不存在：' + id);
    return page;
  }

  /** Returns the page options used by all page selectors. */
  function pageOptions(layout: LayoutDocument): HTMLOptionElement[] {
    return pagesOf(layout).map(
      (page) => new Option(`${page.name}${page.id === layout.entryPage ? ' · 启动' : ''}`, page.id),
    );
  }

  /** Creates the SVG page graph and its navigation edges. */
  function renderGraph(layout: LayoutDocument, pageId: string): void {
    const pages = pagesOf(layout);
    const namespace = 'http://www.w3.org/2000/svg';
    const rows = Math.ceil(pages.length / 3);
    const width = 850;
    const height = rows * 150 + 30;
    const svg = document.createElementNS(namespace, 'svg');
    svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
    svg.style.width = `${width}px`;
    svg.style.height = `${height}px`;
    svg.setAttribute('aria-label', '页面跳转关系图');

    const positions = new Map<string, GraphPoint>(
      pages.map((page, index) => [
        page.id,
        {x: 30 + (index % 3) * 280, y: 25 + Math.floor(index / 3) * 150},
      ]),
    );
    const positionOf = (id: string): GraphPoint => {
      const position = positions.get(id);
      if (!position) throw new Error('页面关系图包含未知页面：' + id);
      return position;
    };

    const defs = document.createElementNS(namespace, 'defs');
    defs.innerHTML =
      '<marker id="route-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto">' +
      '<path d="M0,0 L8,4 L0,8" fill="#85ddc9"/></marker>';
    svg.append(defs);

    const drawn = new Set<string>();
    for (const edge of pageEdges(layout)) {
      const key = `${edge.from}>${edge.to}`;
      if (edge.from === edge.to || drawn.has(key)) continue;
      const from = positionOf(edge.from);
      const to = positionOf(edge.to);
      drawn.add(key);
      const path = document.createElementNS(namespace, 'path');
      const x1 = from.x + 200;
      const y1 = from.y + 36;
      const x2 = to.x;
      const y2 = to.y + 36;
      path.setAttribute('d', `M ${x1} ${y1} C ${x1 + 70} ${y1 + 60}, ${x2 - 60} ${y2 + 60}, ${x2} ${y2}`);
      path.setAttribute('fill', 'none');
      path.setAttribute('stroke', '#85ddc960');
      path.setAttribute('stroke-width', '2');
      path.setAttribute('marker-end', 'url(#route-arrow)');
      const title = document.createElementNS(namespace, 'title');
      title.textContent = `${edge.from} → ${edge.to}`;
      path.append(title);
      svg.append(path);
    }

    for (const page of pages) {
      const {x, y} = positionOf(page.id);
      const group = document.createElementNS(namespace, 'g');
      group.setAttribute('transform', `translate(${x},${y})`);
      group.setAttribute('role', 'button');
      group.setAttribute('tabindex', '0');
      group.setAttribute('aria-label', '编辑页面 ' + page.name);
      group.classList.add('graph-page');

      const rectangle = document.createElementNS(namespace, 'rect');
      rectangle.setAttribute('width', '200');
      rectangle.setAttribute('height', '78');
      rectangle.setAttribute('rx', '10');
      rectangle.setAttribute('fill', page.id === pageId ? '#23443e' : '#202a38');
      rectangle.setAttribute('stroke', page.id === pageId ? '#85ddc9' : '#526175');
      group.append(rectangle);

      const title = document.createElementNS(namespace, 'text');
      title.setAttribute('x', '14');
      title.setAttribute('y', '29');
      title.setAttribute('fill', '#eef6ff');
      title.setAttribute('font-size', '14');
      title.textContent = `${page.name}${page.id === layout.entryPage ? ' · 启动' : ''}`;
      group.append(title);

      const metadata = document.createElementNS(namespace, 'text');
      metadata.setAttribute('x', '14');
      metadata.setAttribute('y', '54');
      metadata.setAttribute('fill', '#9cafc3');
      metadata.setAttribute('font-size', '11');
      metadata.textContent = `${page.id} · ${page.nodes.length} 组件`;
      group.append(metadata);

      group.addEventListener('click', () => selectPage(page.id));
      group.addEventListener('keydown', (event: Event) => {
        if (!(event instanceof KeyboardEvent) || !['Enter', ' '].includes(event.key)) return;
        event.preventDefault();
        selectPage(page.id);
      });
      svg.append(group);
    }
    graph.append(svg);
  }

  /** Renders the page selectors, graph and current page connections. */
  function render(): void {
    const state = snapshot();
    const layout = state.document;
    const pages = pagesOf(layout);
    const active = activePage(layout, state.pageId);
    const options = pageOptions(layout);

    picker.replaceChildren(...options.map((option) => option.cloneNode(true) as HTMLOptionElement));
    picker.value = state.pageId;
    entryPageSelect.replaceChildren(...options.map((option) => option.cloneNode(true) as HTMLOptionElement));
    entryPageSelect.value = layout.entryPage ?? 'home';
    pageNameInput.value = state.pageId === 'home' ? '首页' : active.name;
    pageNameInput.disabled = state.pageId === 'home';
    removePageButton.disabled = state.pageId === 'home' || state.pageId === layout.entryPage;
    for (const [id, field] of pageLinkFields) {
      const select = query<HTMLSelectElement>('#' + id);
      select.replaceChildren(
        new Option('无连接', ''),
        ...options.map((option) => option.cloneNode(true) as HTMLOptionElement),
      );
      select.value = active[field] ?? '';
    }
    query<HTMLElement>('#page-count').textContent = `${pages.length} / 12 页`;
    graph.replaceChildren();
    renderGraph(layout, state.pageId);

    const edges = pageEdges(layout);
    const currentEdges = edges.filter((edge) => edge.from === state.pageId);
    connectionList.replaceChildren();
    if (currentEdges.length === 0) {
      connectionList.textContent = '此页面暂无出口。可设置左滑/右滑目标，或为组件添加点击跳转。';
    }
    for (const edge of currentEdges) {
      const row = window.document.createElement('button');
      row.textContent = `${edge.label} → ${pages.find((page) => page.id === edge.to)?.name ?? edge.to}`;
      row.addEventListener('click', () => selectPage(edge.to));
      connectionList.append(row);
    }
  }

  /** Selects the active page from the toolbar. */
  function selectFromPicker(): void {
    selectPage(picker.value);
  }

  /** Creates a new empty page in the project. */
  function addPage(): void {
    const id = 'screen_' + Date.now().toString(36).slice(-7);
    const name = newPageNameInput.value.trim() || '新页面';
    if (!apply([{op: 'addPage', page: {id, name, background: '#091420', nodes: []}}])) return;
    selectPage(id);
    newPageNameInput.value = '';
    pageFeedback.textContent = '已新建页面。配置跳转按钮或滑动连接，让用户能进入此页。';
  }

  /** Removes the current non-home page after explicit confirmation. */
  function removePage(): void {
    const state = snapshot();
    if (state.pageId === 'home') return;
    if (!window.confirm('删除当前页面及其组件？指向此页的跳转也会移除，可以撤销。')) return;
    if (apply([{op: 'removePage', id: state.pageId}])) selectPage('home');
  }

  /** Updates the project entry page. */
  function updateEntryPage(): void {
    apply([{op: 'document', changes: {entryPage: entryPageSelect.value}}]);
  }

  /** Updates the current page name. */
  function updatePageName(): void {
    apply([{op: 'updatePage', id: snapshot().pageId, changes: {name: pageNameInput.value}}]);
  }

  /** Updates one of the current page swipe targets. */
  function updatePageLink(field: PageLinkField, value: string): void {
    apply([{op: 'updatePage', id: snapshot().pageId, changes: {[field]: value}}]);
  }

  picker.addEventListener('change', selectFromPicker);
  query<HTMLButtonElement>('#manage-pages').addEventListener('click', () => {
    render();
    dialog.showModal();
  });
  query<HTMLButtonElement>('#pages-close').addEventListener('click', () => dialog.close());
  query<HTMLButtonElement>('#add-page').addEventListener('click', addPage);
  removePageButton.addEventListener('click', removePage);
  entryPageSelect.addEventListener('change', updateEntryPage);
  pageNameInput.addEventListener('change', updatePageName);
  for (const [id, field] of pageLinkFields) {
    query<HTMLSelectElement>('#' + id).addEventListener('change', (event: Event) => {
      const select = event.currentTarget;
      if (!(select instanceof HTMLSelectElement)) throw new Error('页面连接控件类型错误');
      updatePageLink(field, select.value);
    });
  }

  return {render};
}
