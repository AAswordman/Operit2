import type { ReviewFile } from './model';

export interface ChangeDirectory { kind: 'directory'; id: string; name: string; path: string; children: ChangeNode[] }
export interface ChangeFile { kind: 'file'; id: string; name: string; path: string; file: ReviewFile }
export type ChangeNode = ChangeDirectory | ChangeFile;
export interface ChangeTreeRow { node: ChangeNode; depth: number; parents: string[] }

/** Keeps equal relative filenames in different repositories independently selectable. */
export function fileKey(file: ReviewFile): string {
  return typeof file.root === 'string' ? JSON.stringify([file.root, file.path]) : file.path;
}

/** Groups only changed files under shared directories, without reading the workspace tree. */
export function buildChangeTree(files: ReviewFile[]): ChangeNode[] {
  const roots = new Map<string, ChangeDirectory>();
  const directories = new Map<string, ChangeDirectory>();
  for (const file of files) {
    const root = typeof file.root === 'string' ? file.root : '';
    let parent = roots.get(root);
    if (parent === undefined) {
      parent = { kind: 'directory', id: `change-root:${JSON.stringify(root)}`, name: root.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? '.', path: root, children: [] };
      roots.set(root, parent);
    }
    const segments = file.path.replace(/\\/g, '/').split('/').filter(Boolean);
    for (let index = 0; index < segments.length - 1; index++) {
      const path = segments.slice(0, index + 1).join('/');
      const id = `change-dir:${JSON.stringify([root, path])}`;
      let directory = directories.get(id);
      if (directory === undefined) {
        directory = { kind: 'directory', id, name: segments[index], path, children: [] };
        directories.set(id, directory);
        parent.children.push(directory);
      }
      parent = directory;
    }
    parent.children.push({ kind: 'file', id: fileKey(file), name: segments[segments.length - 1], path: file.path, file });
  }
  /** Places folders before files while retaining deterministic sibling order. */
  function sort(nodes: ChangeNode[]): ChangeNode[] {
    nodes.sort((a, b) => Number(b.kind === 'directory') - Number(a.kind === 'directory') || a.name.localeCompare(b.name));
    for (const node of nodes) if (node.kind === 'directory') sort(node.children);
    return nodes;
  }
  return sort(roots.size === 1 ? [...roots.values()][0].children : [...roots.values()]);
}

/** Materializes expanded rows and their ancestor identities for selection reveal. */
export function changeTreeRows(nodes: ChangeNode[], collapsed: ReadonlySet<string>, depth = 0, parents: string[] = []): ChangeTreeRow[] {
  return nodes.flatMap(node => [
    { node, depth, parents },
    ...(node.kind === 'directory' && !collapsed.has(node.id) ? changeTreeRows(node.children, collapsed, depth + 1, [...parents, node.id]) : []),
  ]);
}
