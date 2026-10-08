import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {layoutSourceReference} from './source-reference.mts';
import type {SourceReference} from './source-reference.mts';
import {withStatus} from '../layout/project-model.mts';

const root = new URL('../../../../', import.meta.url);

/** SHA-256 hex digest of a source file. */
function hash(text: string): string {
  return createHash('sha256').update(text).digest('hex');
}

const anchors: Array<[string, string, string]> = [
  ['apps/esp32/ui_port/operit_mini_ui.c', 'activate', 'static void activate('],
  ['apps/esp32/ui_port/operit_mini_ui.c', 'operit_ui_layout_bind', 'void operit_ui_layout_bind('],
  ['apps/esp32/ui_port/operit_mini_ui.c', 'scene', 'static void scene(void) {'],
  ['apps/esp32/src/main.rs', 'device action dispatch', '"run_node" | "edge_search" | "edge_pair" =>'],
  ['tools/esp32-editor/src/layout/routes.mts', 'routes', 'export const routes'],
];

export interface ImplementationAnchor {
  path: string;
  symbol: string;
  line: number | null;
  revision: string;
}

export interface ComponentSource {
  revision: string;
  location: SourceReference | null;
  implementation: ImplementationAnchor[];
}

/** Locates a component in layout.json and the C/Rust/editor symbols that implement its behavior. */
export async function componentSource(componentId: string | null | undefined, revision?: string | null): Promise<ComponentSource> {
  if (typeof componentId !== 'string' || !/^[a-zA-Z][a-zA-Z0-9_-]{0,39}$/.test(componentId)) {
    throw new Error('组件 ID 无效');
  }
  const raw = await readFile(new URL('apps/esp32/ui/layout.json', root), 'utf8');
  const currentRevision = hash(raw);
  if (revision && revision !== currentRevision) {
    throw withStatus(409, '布局版本已变化，请关闭面板并同步最新布局后重新引用');
  }
  const uniquePaths = [...new Set(anchors.map((anchor) => anchor[0]))];
  const files = new Map(
    await Promise.all(uniquePaths.map(async (path) => [path, await readFile(new URL(path, root), 'utf8')] as const)),
  );
  const implementation = anchors.map(([path, symbol, anchor]) => {
    const text = files.get(path)!;
    const offset = text.indexOf(anchor);
    return {
      path,
      symbol,
      line: offset < 0 ? null : text.slice(0, offset).split('\n').length,
      revision: hash(text),
    };
  });
  return {revision: currentRevision, location: layoutSourceReference(raw, componentId, currentRevision), implementation};
}
