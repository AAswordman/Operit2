export interface LayoutNode {
  [key: string]: unknown;
  id: string;
  type: string;
  parent: string | null;
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
  color: string;
  radius: number;
  value: number;
  action: string;
  longAction?: string;
  binding?: string;
  fontSize?: number;
}

export interface LayoutPage {
  [key: string]: unknown;
  id: string;
  name: string;
  background: string;
  nodes: LayoutNode[];
  swipeLeft?: string;
  swipeRight?: string;
}

export interface LayoutDocument {
  [key: string]: unknown;
  version: number;
  width: number;
  height: number;
  enabled: boolean;
  background: string;
  nodes: LayoutNode[];
  pages?: LayoutPage[];
  entryPage?: string;
  swipeLeft?: string;
  swipeRight?: string;
}

export interface FoundNode {
  node: LayoutNode;
  pageId: string;
}

export interface ProjectRoute {
  id: string;
  label: string;
  kind: 'navigate';
  target: string;
}

export interface PageEdge {
  from: string;
  to: string;
  label: string;
  field: string;
  nodeId?: string;
}

/** Returns true when value is a plain object. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Converts an unknown thrown value into a displayable error message. */
export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Reads a numeric `status` assigned on an Error. */
export function errorStatus(error: unknown): number | undefined {
  if (error instanceof Error && 'status' in error && typeof error.status === 'number') {
    return error.status;
  }
  return undefined;
}

/** Reads a Node `code` string assigned on an Error. */
export function errorCode(error: unknown): string | undefined {
  if (error instanceof Error && 'code' in error && typeof error.code === 'string') {
    return error.code;
  }
  return undefined;
}

/** Creates an Error carrying an HTTP status code. */
export function withStatus(status: number, message: string): Error & {status: number} {
  const error = new Error(message) as Error & {status: number};
  error.status = status;
  return error;
}

/** The root nodes are the home page (compatible with version 1). Other pages share the same schema; only the active page occupies self-drawn UI runtime memory. */
export function pagesOf(doc: LayoutDocument): LayoutPage[] {
  return [{id: 'home', name: '首页', ...doc}, ...(doc.pages || [])];
}

/** Returns the home document or a named extra page. */
export function pageOf(doc: LayoutDocument, id = 'home'): LayoutDocument | LayoutPage | undefined {
  return id === 'home' ? doc : doc.pages?.find((page) => page.id === id);
}

/** Finds a node by id across the home document and extra pages. */
export function findNode(doc: LayoutDocument, id: string): FoundNode | null {
  for (const page of pagesOf(doc)) {
    const node = page.nodes?.find((item) => item.id === id);
    if (node) return {node, pageId: page.id};
  }
  return null;
}

/** Builds a JSON pointer to a node in the on-disk document. */
export function nodePointer(doc: LayoutDocument, id: string): string | null {
  const found = findNode(doc, id);
  if (!found) return null;
  const page = pageOf(doc, found.pageId)!;
  const index = page.nodes.findIndex((item) => item.id === id);
  if (found.pageId === 'home') return `/nodes/${index}`;
  return `/pages/${doc.pages!.findIndex((item) => item.id === found.pageId)}/nodes/${index}`;
}

/** Lists navigate routes generated from the project's own pages. */
export function projectRoutes(doc: LayoutDocument): ProjectRoute[] {
  return pagesOf(doc).map((page) => ({
    id: 'go:' + page.id,
    label: '页面 · ' + page.name,
    kind: 'navigate',
    target: page.id,
  }));
}

/** Extracts a page id from go:/page:/legacy home-apps actions. */
export function routeTarget(action: unknown): string | null {
  if (typeof action !== 'string') return null;
  if (action.startsWith('go:')) return action.slice(3);
  if (action === 'home' || action === 'apps') return action;
  if (action.startsWith('page:')) return action.slice(5);
  return null;
}

/** Collects swipe and button navigation edges for the page graph. */
export function pageEdges(doc: LayoutDocument): PageEdge[] {
  const edges: PageEdge[] = [];
  for (const page of pagesOf(doc)) {
    for (const field of ['swipeLeft', 'swipeRight'] as const) {
      const to = page[field];
      if (to) edges.push({from: page.id, to, label: field === 'swipeLeft' ? '左滑' : '右滑', field});
    }
    for (const node of page.nodes) {
      for (const field of ['action', 'longAction'] as const) {
        const to = routeTarget(node[field]);
        if (to) {
          edges.push({
            from: page.id,
            to,
            nodeId: node.id,
            field,
            label: `${node.text || node.id} · ${field === 'action' ? '点击' : '长按'}`,
          });
        }
      }
    }
  }
  return edges;
}

/** Expands a button matrix into a panel plus independently editable child buttons. */
export function expandMatrix(node: LayoutNode): LayoutNode[] {
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
    ...cells.map(([x, y, cellW, cellH], i) => ({
      ...node,
      id: node.id.slice(0, 33) + '_cell' + i,
      type: 'button',
      parent: node.id,
      x,
      y,
      w: cellW,
      h: cellH,
      text: labels[i] || `Button ${i + 1}`,
      radius: Math.min(node.radius, 8),
    })),
  ];
}
