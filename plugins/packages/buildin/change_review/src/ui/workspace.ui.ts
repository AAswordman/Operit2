import type { ComposeDslContext, ComposeNode, ComposeColor, TextProps } from '../../../../../types/compose-dsl';
import type { FileEntry } from '../../../../../types/results';
import { fileIdentity, reviewForMessage, type ReviewFile, type ReviewMessage } from '../model';
import { parseDiff, diffItems, type DiffLine } from '../diff';
import { resolvePath, resolveResource, openPath, loadWorkspaceChanges, loadFileDiff, type GitFile } from '../git';
import { fileIcon } from '../icons';
import { buildChangeTree, changeTreeRows, fileKey as key, type ChangeTreeRow } from '../change_tree';

const svgVsCode = "\r\n<svg role=\"img\" viewBox=\"0 0 24 24\" fill=\"#007ACC\" xmlns=\"http://www.w3.org/2000/svg\">\r\n<path d=\"M23.15 2.587L18.21.21a1.494 1.494 0 0 0-1.705.29l-9.46 8.63-4.12-3.128a.999.999 0 0 0-1.276.057L.327 7.261A1 1 0 0 0 .326 8.74L3.899 12 .326 15.26a1 1 0 0 0 .001 1.479L1.65 17.94a.999.999 0 0 0 1.276.057l4.12-3.128 9.46 8.63a1.492 1.492 0 0 0 1.704.29l4.942-2.377A1.5 1.5 0 0 0 24 20.06V3.939a1.5 1.5 0 0 0-.85-1.352zm-5.146 14.861L10.826 12l7.178-5.448v10.896z\"/>\r\n</svg>\r\n";
const svgTerminal = "\r\n<svg role=\"img\" viewBox=\"0 0 24 24\" fill=\"#5391FE\" xmlns=\"http://www.w3.org/2000/svg\">\r\n<path d=\"M24 18.064c0 1.34-1.077 2.42-2.408 2.42H2.408C1.077 20.484 0 19.404 0 18.064V5.936C0 4.596 1.077 3.516 2.408 3.516h19.184C22.923 3.516 24 4.596 24 5.936v12.128zm-11.455-6.07l-6.86-5.474a.81.81 0 0 0-1.12.115.815.815 0 0 0 .114 1.127l5.966 4.76-5.966 4.761a.813.813 0 0 0-.114 1.127.81.81 0 0 0 1.12.115l6.86-5.474a.816.816 0 0 0 0-1.057zm7.42 4.417h-6.286a.81.81 0 1 0 0 1.621h6.286a.81.81 0 1 0 0-1.621z\"/>\r\n</svg>\r\n";
const svgTreeHierarchy = "\r\n<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" xmlns=\"http://www.w3.org/2000/svg\">\r\n<rect x=\"9\" y=\"3\" width=\"6\" height=\"5\" rx=\"1\"/>\r\n<rect x=\"3\" y=\"15\" width=\"6\" height=\"5\" rx=\"1\"/>\r\n<rect x=\"15\" y=\"15\" width=\"6\" height=\"5\" rx=\"1\"/>\r\n<path d=\"M12 8v4M6 15v-3h12v3\"/>\r\n</svg>\r\n";

type Source = 'git' | 'branch' | 'lastTurn';
type Pane = 'changes' | 'tree' | 'closed';
interface Input { files?: ReviewFile[]; messages?: ReviewMessage[]; chatId?: string }
interface TreeEntry extends FileEntry { path: string; depth: number }

/** Joins semantic workspace paths without guessing native platform separators. */
function join(root: string, child: string): string {
  if (child.startsWith('/')) return child;
  return `${root.replace(/\/$/, '')}/${child}`;
}

/** Renders the review state, comparison queries and interactions entirely inside the plugin. */
export default function Workspace(ctx: ComposeDslContext): ComposeNode {
  const { UI, Modifier: M } = ctx, c = ctx.MaterialTheme.colorScheme;
  /** Keeps native DefaultTextStyle metrics unless an explicit theme role is requested. */
  const text = (props: TextProps): ComposeNode => UI.Text({ inheritStyle: props.style === undefined, ...props });
  const [input] = ctx.useState<Input>('input', {});
  const [messages] = ctx.useState<ReviewMessage[]>('messages', []);
  const [workspace] = ctx.useState<string | null>('workspacePath', null);
  const [locale] = ctx.useState('locale', 'zh');
  const dark = ctx.Theme.getCurrent().brightness === 'dark';
  const lastTurn = ctx.useMemo('turn-files', () => {
    if (input.files !== undefined) return input.files;
    const conversation = input.messages === undefined ? messages : input.messages;
    for (let index = conversation.length - 1; index >= 0; index--) {
      const message = conversation[index];
      if (message.sender !== 'ai') continue;
      const files = reviewForMessage(message);
      if (files.length > 0) return files;
    }
    return [];
  }, [input, messages]);
  const [source, setSource] = ctx.useState<Source>('source', lastTurn.length > 0 ? 'lastTurn' : 'git');
  const [pane, setPane] = ctx.useState<Pane>('pane', 'changes');
  const previousPane = ctx.useRef<Pane>('previous-pane', 'changes');
  const [split, setSplit] = ctx.useState('split', false);
  const [wrap, setWrap] = ctx.useState('wrap', false);
  const [full, setFull] = ctx.useState('full', false);
  const [query, setQuery] = ctx.useState('query', '');
  const [base] = ctx.useState<string | null>('branch-base', null);
  const [listWidth, setListWidth] = ctx.useState('list-width', 220);
  const listDrag = ctx.useRef('list-drag-width', listWidth);
  const [collapsedChanges, setCollapsedChanges] = ctx.useState<string[]>('collapsed-changes', []);
  const [splitRatio, setSplitRatio] = ctx.useState('split-ratio', 0.5);
  const splitDrag = ctx.useRef('split-drag-ratio', splitRatio);
  const [size, setSize] = ctx.useState('size', { width: 800, height: 600 });
  const [catalog, setCatalog] = ctx.useState<GitFile[]>('catalog', []);
  const [loading, setLoading] = ctx.useState('loading', false);
  const [error, setError] = ctx.useState('error', '');
  const [selectedPath, setSelectedPath] = ctx.useState('selected', '');
  const [loadingDiff, setLoadingDiff] = ctx.useState('loading-diff', false);
  const [fileLines, setFileLines] = ctx.useState<string[] | null>('file-lines', null);
  const [gaps, setGaps] = ctx.useState<number[]>('gaps', []);
  const [tree, setTree] = ctx.useState<Record<string, TreeEntry[]>>('tree', {});
  const treeCache = ctx.useRef<Record<string, TreeEntry[]>>('tree-cache', {});
  const [expanded, setExpanded] = ctx.useState<string[]>('expanded', []);
  const [preview, setPreview] = ctx.useState<{ path: string; content: string } | null>('preview', null);
  const [previewLoading, setPreviewLoading] = ctx.useState('preview-loading', false);
  const [previewError, setPreviewError] = ctx.useState('preview-error', '');
  const [treeLoading, setTreeLoading] = ctx.useState('tree-loading', false);
  const [treeError, setTreeError] = ctx.useState('tree-error', '');
  const treeRequest = ctx.useRef('tree-request', 0);
  const previewRequest = ctx.useRef('preview-request', 0);
  const [opener, setOpener] = ctx.useState('opener', 1);
  const [refresh, setRefresh] = ctx.useState('refresh', 0);
  const sourceNames = ['Git 变更', '分支变更', '上一轮变更'];
  const sources: Source[] = ['git', 'branch', 'lastTurn'];
  const green = '#FF3FB950', red = '#FFF85149';
  const divider = dark ? '#FF262626' : c.outlineVariant.copy({ alpha: 0.35 });
  const all = source === 'lastTurn' ? lastTurn : catalog;
  const rows = ctx.useMemo('filtered-changes', () => all.filter(file => query.trim() === '' || file.path.toLowerCase().includes(query.trim().toLowerCase())), [all, query]);
  const currentIndex = Math.max(0, rows.findIndex(file => key(file) === selectedPath));
  const current = rows[currentIndex] ?? null;
  const effective = current;
  const totals = all.reduce((sum, file) => ({ additions: sum.additions + file.additions, deletions: sum.deletions + file.deletions }), { additions: 0, deletions: 0 });
  const availableListWidth = Math.max(0, Math.min(500, size.width - 161));
  const minimumListWidth = Math.min(120, availableListWidth);
  const sidebarWidth = Math.min(Math.max(minimumListWidth, listWidth), availableListWidth);
  const mainWidth = Math.max(1, size.width - (pane === 'closed' ? 0 : sidebarWidth + 1));
  const changeTree = ctx.useMemo('change-tree', () => buildChangeTree(rows), [rows]);
  const visibleChanges = changeTreeRows(changeTree, new Set(query.trim() === '' ? collapsedChanges : []));

  /** Follows new host review input while keeping user-owned layout and filter choices. */
  ctx.useEffect('review-input', () => { setSource(lastTurn.length > 0 ? 'lastTurn' : 'git'); return () => {}; }, [lastTurn]);

  /** Gives each source request a composition-owned cancellation boundary. */
  ctx.useEffect('catalog-load', () => {
    let active = true;
    ++previewRequest.current;
    setError(''); setCatalog([]); setSelectedPath(''); setPreview(null);
    if (source === 'lastTurn') { setLoading(false); return () => { active = false; }; }
    if (workspace === null || workspace === '') { setLoading(false); return () => { active = false; }; }
    setLoading(true);
    loadWorkspaceChanges(workspace, source, base).then(files => { if (active) { setCatalog(files); setLoading(false); } }, failure => {
      if (active) { setError(String(failure)); setLoading(false); }
    });
    return () => { active = false; };
  }, [source, workspace, base, refresh]);

  /** Resolves Git's native root through the runtime mapper instead of removing VFS segments. */
  async function filePath(file: ReviewFile): Promise<string> {
    if (source === 'lastTurn') return (await resolveResource(workspace ?? '', file.path)).vfsPath;
    if (workspace === null || workspace === '') throw new Error('当前对话未绑定工作区');
    return (await resolvePath(join((file as GitFile).root, file.path), true)).vfsPath;
  }

  /** Loads only the selected file, without letting a late request overwrite a later selection. */
  ctx.useEffect('selected-file', () => {
    let active = true;
    setError('');
    setFileLines(null); setGaps([]); setLoadingDiff(false);
    if (current === null || source === 'lastTurn' || workspace === null) return () => { active = false; };
    if ((current as GitFile).loaded) return () => { active = false; };
    setLoadingDiff(true);
    loadFileDiff(workspace, current as GitFile, async () => (await Tools.Files.read(await filePath(current))).content).then(file => {
      if (active) { setCatalog(catalog.map(entry => key(entry) === key(file) ? file : entry)); setLoadingDiff(false); }
    }, failure => { if (active) { setError(String(failure)); setLoadingDiff(false); } });
    return () => { active = false; };
  }, [source, workspace, current === null ? null : key(current), refresh]);

  /** Resolves current text only for full-file display; compact review does not read every file. */
  ctx.useEffect('full-file', () => {
    let active = true;
    if (!full || current === null) { setFileLines(null); setGaps([]); return () => { active = false; }; }
    filePath(current).then(path => Tools.Files.read(path)).then(result => {
      if (active) setFileLines(result.content.split(/\r?\n/));
    }, failure => { if (active) setError(String(failure)); });
    return () => { active = false; };
  }, [full, current === null ? null : key(current), source, workspace]);

  /** Fetches tree children through VFS, keeping directory state independent of the change catalog. */
  async function loadDirectory(path: string, depth: number, generation: number): Promise<void> {
    const listing = await Tools.Files.list(path);
    const children = listing.entries.filter(entry => !entry.name.startsWith('.')).sort((a, b) => {
      if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
      return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
    }).map(entry => ({ ...entry, path: join(path, entry.name), depth }));
    if (treeRequest.current !== generation) return;
    treeCache.current = { ...treeCache.current, [path]: children };
    setTree(treeCache.current);
  }

  /** Loads the root only when the tree surface is requested. */
  ctx.useEffect('tree-root', () => {
    let active = true;
    const generation = ++treeRequest.current;
    if (!workspace) return () => { active = false; };
    setTreeError(''); setTreeLoading(true); setExpanded([]);
    Tools.Files.list(workspace).then(listing => {
      if (active && treeRequest.current === generation) {
        treeCache.current = { [workspace]: listing.entries.filter(entry => !entry.name.startsWith('.')).sort((a, b) =>
        a.isDirectory !== b.isDirectory ? a.isDirectory ? -1 : 1 : a.name.toLowerCase().localeCompare(b.name.toLowerCase()))
        .map(entry => ({ ...entry, path: join(workspace, entry.name), depth: 0 })) };
        setTree(treeCache.current); setTreeLoading(false);
      }
    }, failure => { if (active) { setTreeError(String(failure)); setTreeLoading(false); } });
    return () => { active = false; ++treeRequest.current; ++previewRequest.current; };
  }, [workspace, refresh]);

  /** Keeps toolbar icons at the original compact dimensions and native hover tooltips. */
  function icon(name: string, tooltip: string, onClick: () => void | Promise<void>, active = false, enabled = true): ComposeNode {
    return UI.Tooltip({ message: tooltip }, UI.InkWell({ onClick: enabled ? onClick : undefined, shape: { cornerRadius: 4 } },
      UI.Box({ width: 22, height: 22, contentAlignment: 'center', modifier: active ? M.background(c.primary.copy({ alpha: 0.18 }), { cornerRadius: 4 }) : undefined },
        UI.Icon({ name, size: 14, tint: enabled ? active ? c.primary : c.onSurfaceVariant : c.onSurfaceVariant.copy({ alpha: 0.38 }) }))));
  }

  /** Renders the same monospace colored counters used in the native review. */
  function stat(value: number, addition: boolean, fontSize = 11): ComposeNode {
    return text({ text: `${addition ? '+' : '-'}${value}`, color: addition ? green : red, fontSize, fontFamily: 'monospace', fontWeight: fontSize === 11 ? '700' : undefined, maxLines: 1 });
  }

  /** Provides a shared separator, without introducing another page-level card. */
  function line(color: ComposeColor = c.outlineVariant): ComposeNode { return UI.Divider({ height: 1, thickness: 1, color }); }

  /** Keeps all menu labels and selection state owned by the plugin. */
  function menu(anchor: ComposeNode, labels: string[], selected: number, onSelect: (index: number) => void, tooltip: string): ComposeNode {
    return UI.MenuAnchor({ anchor, tooltip, itemHeight: 32, onSelect }, labels.map((label, index) => UI.Row({ verticalAlignment: 'center' }, [
      text({ text: label, fontSize: 12, weight: 1 }), ...(index === selected ? [UI.Icon({ name: 'check', size: 14 })] : []),
    ])));
  }

  /** Preserves the original source selector, native SVG tree control and sidebar counters. */
  function sidebarHeader(): ComposeNode {
    const sourceAnchor = UI.Box({ modifier: M.height(24).background(c.surface.copy({ alpha: 0.8 }), { cornerRadius: 5 }).border(1, c.outlineVariant.copy({ alpha: 0.3 }), { cornerRadius: 5 }).padding(8, 1), contentAlignment: 'center' },
      UI.Row({ verticalAlignment: 'center' }, [text({ text: sourceNames[sources.indexOf(source)], style: 'labelSmall', fontSize: 11.5, letterSpacing: 0.5, fontWeight: '600' }), UI.Spacer({ width: 2 }), UI.Icon({ name: 'arrow_drop_down', size: 15, tint: c.onSurfaceVariant })]));
    return UI.Box({ height: 32, modifier: M.padding(8, 0), contentAlignment: 'start' }, UI.ScrollArea({ axis: 'horizontal', height: 32 }, UI.Row({ modifier: M.widthIn(sidebarWidth - 16), verticalAlignment: 'center', horizontalArrangement: 'spaceBetween' }, [UI.Row({ verticalAlignment: 'center' }, [
      UI.Tooltip({ message: pane === 'tree' ? '切换至变更文件列表' : '切换至项目工作树' }, UI.InkWell({ shape: { cornerRadius: 5 }, onClick: () => setPane(pane === 'tree' ? 'changes' : 'tree') },
        UI.Box({ modifier: M.width(25).height(25).background(pane === 'tree' ? c.primary.copy({ alpha: 0.18 }) : c.surface.copy({ alpha: 0.8 }), { cornerRadius: 5 }).border(1, pane === 'tree' ? c.primary.copy({ alpha: 0.5 }) : c.outlineVariant.copy({ alpha: 0.3 }), { cornerRadius: 5 }).padding(5.5) }, UI.Svg({ source: svgTreeHierarchy, tint: pane === 'tree' ? c.primary : c.onSurfaceVariant })))), UI.Spacer({ width: 6 }),
      menu(sourceAnchor, sourceNames, sources.indexOf(source), index => setSource(sources[index]), '切换变更对比源')]), UI.Spacer({ width: 6 }), UI.Row({ verticalAlignment: 'center' }, [stat(totals.additions, true), UI.Spacer({ width: 5 }), stat(totals.deletions, false)])
    ])));
  }

  /** Switches file selection while preserving the selected comparison source. */
  function select(path: string): void {
    ++previewRequest.current; setSelectedPath(path); setPreview(null);
    const row = changeTreeRows(changeTree, new Set()).find(entry => entry.node.id === path);
    if (row !== undefined) setCollapsedChanges(collapsedChanges.filter(id => !row.parents.includes(id)));
  }

  /** Shows shared folders once and keeps filenames free of repeated parent paths. */
  function changeRow({ node, depth }: ChangeTreeRow): ComposeNode {
    const folder = node.kind === 'directory', selected = !folder && current !== null && node.id === key(current);
    const opened = query.trim() !== '' || !collapsedChanges.includes(node.id);
    return UI.Tooltip({ message: node.path }, UI.InkWell({ key: node.id, onClick: () => {
      if (node.kind === 'directory') setCollapsedChanges(opened ? [...collapsedChanges, node.id] : collapsedChanges.filter(id => id !== node.id));
      else select(node.id);
    } }, UI.Row({ height: 26, modifier: M.background(selected ? c.primary.copy({ alpha: 0.15 }) : 'transparent').borderSides(selected ? { start: { width: 2.5, color: c.primary } } : {}).padding(8 + depth * 14 - (selected ? 2.5 : 0), 0, 8, 0), verticalAlignment: 'center' }, [
      folder ? UI.Icon({ name: opened ? 'keyboard_arrow_down' : 'keyboard_arrow_right', size: 13, tint: c.outline }) : UI.Spacer({ width: 13 }), UI.Spacer({ width: 3 }),
      folder ? UI.Icon({ name: opened ? 'folder_open' : 'folder', size: 14, tint: '#FFEBCB8B' }) : UI.Svg({ source: fileIcon(node.name), width: 14, height: 14 }), UI.Spacer({ width: 6 }),
      text({ text: node.name, weight: 1, maxLines: 1, overflow: 'ellipsis', color: selected ? c.primary : c.onSurface, fontWeight: selected ? '700' : '500', fontSize: 11.5 }),
      ...(node.kind === 'file' && node.file.additions > 0 ? [stat(node.file.additions, true, 10.5), UI.Spacer({ width: 3 })] : []),
      ...(node.kind === 'file' && node.file.deletions > 0 ? [stat(node.file.deletions, false, 10.5)] : []),
    ])));
  }

  /** Resizes from the current rendered width and accumulates rapid pointer events without stale state. */
  function dragSidebar(deltaX: number): void {
    listDrag.current = Math.min(availableListWidth, Math.max(minimumListWidth, listDrag.current + deltaX));
    setListWidth(listDrag.current);
  }

  /** Expands directories or loads an ordinary file preview through the same VFS capability. */
  async function selectTree(entry: TreeEntry): Promise<void> {
    if (entry.isDirectory) {
      if (expanded.includes(entry.path)) setExpanded(expanded.filter(path => path !== entry.path));
      else {
        const generation = treeRequest.current;
        try {
          if (treeCache.current[entry.path] === undefined) await loadDirectory(entry.path, entry.depth + 1, generation);
          if (treeRequest.current === generation) setExpanded([...expanded, entry.path]);
        } catch (failure) { if (treeRequest.current === generation) setTreeError(String(failure)); }
      }
    } else {
      const generation = ++previewRequest.current;
      setPreview({ path: entry.path, content: '' }); setPreviewLoading(true); setPreviewError('');
      try {
        const result = await Tools.Files.read(entry.path);
        if (previewRequest.current === generation) { setPreview({ path: entry.path, content: result.content }); setPreviewLoading(false); }
      } catch (failure) {
        if (previewRequest.current === generation) { setPreviewError(String(failure)); setPreviewLoading(false); }
      }
    }
  }

  /** Traverses only expanded directories; hidden subtrees do not create composed rows. */
  function treeRows(path: string): ComposeNode[] {
    return (tree[path] ?? []).flatMap(entry => {
      const opened = expanded.includes(entry.path), selected = preview?.path === entry.path;
      return [UI.InkWell({ key: entry.path, onClick: () => selectTree(entry) }, UI.Row({ height: 24, modifier: M.background(selected ? c.primary.copy({ alpha: 0.12 }) : 'transparent').padding(6 + entry.depth * 14, 0, 6, 0), verticalAlignment: 'center' }, [
        entry.isDirectory ? UI.Icon({ name: opened ? 'keyboard_arrow_down' : 'keyboard_arrow_right', size: 13, tint: c.outline }) : UI.Spacer({ width: 13 }), UI.Spacer({ width: 3 }),
        entry.isDirectory ? UI.Icon({ name: opened ? 'folder_open' : 'folder', size: 14, tint: selected ? c.primary : '#FFEBCB8B' }) : UI.Svg({ source: fileIcon(entry.name), width: 14, height: 14 }),
        UI.Spacer({ width: 6 }), text({ text: entry.name, fontSize: 11.5, color: selected ? c.primary : undefined, fontWeight: selected ? '700' : undefined, weight: 1, maxLines: 1, overflow: 'ellipsis' }),
      ])), ...(opened ? treeRows(entry.path) : [])];
    });
  }

  /** Handles external actions through capabilities rather than platform checks in Dart. */
  async function openTarget(index: number): Promise<void> {
    const path = preview?.path ?? (current === null ? workspace : await filePath(current));
    if (!path) return;
    const resolved = await resolvePath(path);
    if (index === 3) { await Tools.System.setClipboardText(resolved.nativePath); ctx.showToast(`已复制路径: ${resolved.nativePath}`); }
    else if (index === 0) await openPath(path, 'reveal');
    else if (index === 1) await openPath(path, 'program', ['code', '-g']);
    else if (index === 2) await openPath(path, 'terminal');
    else throw new Error(`Unknown external opener: ${index}`);
  }

  /** Builds the native compact toolbar with plugin-owned opener and layout state. */
  function toolbar(): ComposeNode {
    /** Mirrors the native constrained IconButton hit target and icon geometry. */
    function navigation(name: string, tooltip: string, onClick: () => void, enabled = true, tint?: ComposeColor): ComposeNode {
      return UI.IconButton({ tooltip, width: 40, height: 30, enabled, onClick }, UI.Icon({ name, size: 14, tint }));
    }
    /** Draws one of the original bordered compact view switches. */
    function toggle(name: string, tooltip: string, onClick: () => void, active: boolean, enabled = true): ComposeNode {
      return UI.Tooltip({ message: tooltip }, UI.InkWell({ onClick: enabled ? onClick : undefined }, UI.Box({ modifier: M.height(22).background(active ? c.primary.copy({ alpha: 0.15 }) : c.surface.copy({ alpha: 0.6 }), { cornerRadius: 4 }).border(1, active ? c.primary.copy({ alpha: 0.5 }) : c.outlineVariant.copy({ alpha: 0.25 }), { cornerRadius: 4 }).padding(6, 1), contentAlignment: 'center' }, UI.Icon({ name, size: 14, tint: !enabled ? c.outlineVariant : active ? c.primary : c.outline }))));
    }
    /** Keeps the selected external opener icon independent from its menu labels. */
    function openerIcon(index: number, size = 14): ComposeNode {
      return index === 0 ? UI.Icon({ name: 'folder', size, tint: '#FFFFA000' }) : index === 3 ? UI.Icon({ name: 'copy', size }) : UI.Svg({ source: index === 1 ? svgVsCode : svgTerminal, width: size, height: size });
    }
    const labels = ['文件资源管理器', 'Visual Studio Code', '终端 (Terminal)', '复制路径'];
    const openerMenu = UI.MenuAnchor({ tooltip: '选择打开方式', itemHeight: 32,
      items: [{ enabled: false, height: 26 }, {}, {}, {}, { divider: true, height: 6 }, { height: 30 }],
      onSelect: index => { const chosen = index === 5 ? 3 : index - 1; if (chosen !== 3) setOpener(chosen); openTarget(chosen).catch(ctx.reportError); },
      anchor: UI.Box({ modifier: M.height(22).background(c.surface.copy({ alpha: 0.6 }), { cornerRadius: 4 }).border(1, c.outlineVariant.copy({ alpha: 0.3 }), { cornerRadius: 4 }).padding(7, 1), contentAlignment: 'center' }, UI.Row({ verticalAlignment: 'center' }, [openerIcon(opener), UI.Spacer({ width: 4 }), UI.Icon({ name: 'arrow_drop_down', size: 14, tint: c.onSurfaceVariant })])),
    }, [text({ text: '打开方式', fontSize: 10.5, fontWeight: '700', color: '#FF9E9E9E' }), ...[0, 1, 2].map(index => UI.Row({ verticalAlignment: 'center' }, [openerIcon(index, 15), UI.Spacer({ width: 8 }), text({ text: labels[index], fontSize: 12 }), ...(opener === index ? [UI.Box({ weight: 1 }), UI.Icon({ name: 'check', size: 14 })] : [])])), UI.Spacer({ height: 0 }), UI.Row({ verticalAlignment: 'center' }, [openerIcon(3), UI.Spacer({ width: 8 }), text({ text: labels[3], fontSize: 12 })])]);
    const layout = UI.Box({ modifier: M.height(22).background(c.surface.copy({ alpha: 0.6 }), { cornerRadius: 4 }).border(1, c.outlineVariant.copy({ alpha: 0.25 }), { cornerRadius: 4 }).padding(2.5), contentAlignment: 'center' }, UI.Row({ verticalAlignment: 'center' }, [false, true].map(value => UI.Tooltip({ message: value ? '分栏视图 (Split)' : '统一视图 (Unified)' }, UI.InkWell({ onClick: () => setSplit(value) }, UI.Box({ modifier: M.background(split === value ? c.primary.copy({ alpha: 0.15 }) : 'transparent', { cornerRadius: 3 }).padding(5, 0), contentAlignment: 'center' }, UI.Icon({ name: value ? 'vertical_split_outlined' : 'table_rows_outlined', size: 14, tint: split === value ? c.primary : c.outline })))))));
    return UI.Box({ height: 30, modifier: M.background(c.surfaceContainerHighest.copy({ alpha: 0.25 })).padding(6, 0) },
      UI.ScrollArea({ axis: 'horizontal' }, UI.Row({ height: 30, width: Math.max(410, mainWidth - 12), verticalAlignment: 'center', horizontalArrangement: 'spaceBetween' }, [UI.Row({ verticalAlignment: 'center' }, [
        navigation('view_sidebar_outlined', pane === 'closed' ? '展开文件区' : '收起文件区', () => { if (pane === 'closed') setPane(previousPane.current); else { previousPane.current = pane; setPane('closed'); } }, true, pane === 'closed' ? c.primary : c.onSurfaceVariant),
        UI.VerticalDivider({ width: 8, height: 14 }),
        navigation('arrow_back', '上一个改动 (P)', () => select(key(rows[Math.max(0, currentIndex - 1)])), rows.length > 0),
        navigation('arrow_forward', '下一个改动 (N)', () => select(key(rows[Math.min(rows.length - 1, currentIndex + 1)])), rows.length > 0),
        UI.VerticalDivider({ width: 8, height: 14 }), openerMenu]), UI.Spacer({ width: 8 }), layout, UI.Spacer({ width: 6 }),
        toggle(full ? 'unfold_more_outlined' : 'compress_outlined', current === null ? '请选择文件后再切换完整视图' : full ? '仅显示改动附近的行 (收起未更改区域)' : '完整展示整个文件 (展开未更改区域)', () => { setError(''); setFull(!full); }, full, current !== null), UI.Spacer({ width: 6 }),
        toggle('wrap_text_outlined', wrap ? '关闭换行 (不换行)' : '开启换行 (自动换行)', () => setWrap(!wrap), wrap),
      ])));
  }

  /** Renders an empty/error/loading state without hiding execution failures. */
  function empty(message: string): ComposeNode {
    return UI.Box({ fillMaxSize: true, contentAlignment: 'center' }, UI.Column({ horizontalAlignment: 'center' }, [
      UI.Box({ width: 32, height: 32, contentAlignment: 'center', modifier: M.border(1.2, c.outline.copy({ alpha: dark ? 0.35 : 0.5 }), { cornerRadius: 6 }) }, UI.Icon({ name: 'add', size: 16, tint: c.outline.copy({ alpha: dark ? 0.65 : 0.8 }) })),
      UI.Spacer({ height: 12 }), text({ text: '尚无文件变更', fontSize: 13, fontWeight: '600', color: c.onSurface.copy({ alpha: dark ? 0.88 : 0.92 }) }),
      UI.Spacer({ height: 4 }), text({ text: message, fontSize: 11.5, color: c.outline.copy({ alpha: dark ? 0.65 : 0.8 }) }),
    ]));
  }

  /** Preserves selected-file ribbon geometry and per-file count placement. */
  function ribbon(file: ReviewFile): ComposeNode {
    const identity = fileIdentity(file.path);
    const status = 'status' in file ? (file as GitFile).status : 'M';
    const statusColor = status === 'U' || status === 'A' ? green : status === 'D' ? red : status === 'R' ? '#FFD29922' : '#FF58A6FF';
    return UI.Row({ height: 32, modifier: M.background(c.surface.copy({ alpha: 0.35 })).padding(10, 0), verticalAlignment: 'center' }, [
      UI.Box({ modifier: M.background({ value: statusColor, alpha: 0.15 }, { cornerRadius: 3 }).padding(4, 1) }, text({ text: status, color: statusColor, fontSize: 10.5, fontWeight: '700' })), UI.Spacer({ width: 7 }),
      UI.Svg({ source: fileIcon(identity.name), width: 14, height: 14 }), UI.Spacer({ width: 7 }),
      UI.RichText({ weight: 1, maxLines: 1, overflow: 'ellipsis', spans: [
        { text: identity.name, fontWeight: '700', fontSize: 12, color: c.onSurface },
        ...(identity.directory === '' ? [] : [{ text: '  ' }, { text: identity.directory, fontSize: 11.5, color: c.outline.copy({ alpha: 0.7 }) }]),
      ] }), UI.Spacer({ width: 8 }), stat(file.additions, true), UI.Spacer({ width: 4 }), stat(file.deletions, false),
    ]);
  }

  /** Retains exact intraline emphasis ranges while leaving source text unmodified. */
  function lineText(row: DiffLine): ComposeNode {
    const accent = row.kind === 'add' ? dark ? '#FF3FB950' : '#FF1A7F37' : dark ? '#FFF85149' : '#FFCF222E';
    return UI.CodeText({ text: row.text, filePath: current?.path ?? '',
      emphasis: row.segments.map(segment => ({ ...segment, color: { value: accent, alpha: dark ? 0.4 : 0.32 }, fontWeight: '700' })),
      fontSize: 12, fontFamily: 'Consolas', lineHeight: 1, softWrap: wrap, maxLines: wrap ? undefined : 1,
      color: row.kind === 'context' ? c.onSurface.copy({ alpha: 0.82 }) : c.onSurface });
  }

  /** Mirrors native wrapped and fixed rows, including split-side coordinates and missing cells. */
  function diffLine(row: DiffLine | null, width: number, added?: boolean): ComposeNode {
    if (row === null) return UI.Box({ width, height: wrap ? undefined : 22, modifier: M.heightIn(22).background({ ...c.surfaceContainerHighest.copy({ alpha: dark ? 0.2 : 0.12 }), over: c.surface }) });
    const kind = row.kind, changed = kind !== 'context';
    const accent: ComposeColor = kind === 'add' ? dark ? '#FF3FB950' : '#FF1A7F37' : kind === 'remove' ? dark ? '#FFF85149' : '#FFCF222E' : 'transparent';
    const background: ComposeColor = { value: accent as string, alpha: dark ? 0.20 : 0.12, over: c.surface };
    const number = added === false ? row.oldLineNumber : added === true ? row.newLineNumber : kind === 'remove' ? row.oldLineNumber : row.newLineNumber;
    const gutter = UI.Box({ contentAlignment: 'end', modifier: (wrap ? M.width(42) : M.width(42).height(22)).background(changed ? { value: accent as string, alpha: dark ? 0.10 : 0.07, over: c.surface } : { ...c.onSurface.copy({ alpha: dark ? 0.055 : 0.045 }), over: c.surface }).padding(0, 0, 8, 0) },
      text({ textAlign: 'right', text: String(number ?? ''), fontSize: 11, lineHeight: 1, fontFamily: 'Consolas', fontFamilyFallback: ['Cascadia Mono', 'monospace'], color: changed ? { value: accent, alpha: dark ? 0.85 : 0.90 } : c.outline.copy({ alpha: 0.65 }) }));
    const content = UI.Row({ verticalAlignment: wrap ? 'topStart' : 'center', fillMaxWidth: true }, [
      ...(!wrap ? [UI.Box({ width: 3, height: 22, modifier: M.background(accent) })] : []), gutter,
      UI.Spacer({ width: added === undefined ? 8 : 6 }), UI.Box({ weight: 1, modifier: wrap ? M.padding(0, 3) : undefined }, lineText(row)),
    ]);
    if (wrap) return UI.Box({ modifier: M.width(width).heightIn(22).background(changed ? background : 'transparent').borderSides({ start: { width: 3, color: accent } }).padding(3, 0, 0, 0) }, content);
    return UI.Box({ width, height: 22 }, [
      ...(changed ? [UI.Canvas({ width, height: 22, commands: [{ type: 'Rect', x: 0, y: 0, width, height: 23, color: background }] })] : []), content,
    ]);
  }

  /** Expands only a region already covered by the caller-supplied full-file text. */
  function expandGap(start: number): void { setGaps([...gaps, start]); }

  /** Displays the same unified and paired diff models, with a lazy viewport rather than a full text widget. */
  function diffBody(file: ReviewFile): ComposeNode {
    const document = ctx.useMemo('document', () => parseDiff(file.diffContent, source === 'lastTurn'), [file.diffContent, source]);
    const items = ctx.useMemo('items', () => diffItems(document, split, fileLines, full ? null : new Set(gaps)), [document, split, fileLines, full, gaps]);
    if (items.length === 0) return empty('没有可显示的行级变更');
    const textWidth = document.maxLineLength * 7.8 + 48;
    const width = wrap ? mainWidth : Math.max(mainWidth, split ? 2 * (51 + textWidth) : 53 + textWidth);
    const usableWidth = width - 1, minimumRatio = Math.min(96 / usableWidth, 0.5);
    const ratio = Math.min(1 - minimumRatio, Math.max(minimumRatio, splitRatio));
    const leftWidth = usableWidth * ratio, rightWidth = usableWidth - leftWidth;
    /** Generates only the current host viewport from the plugin's immutable diff model. */
    function item(index: number): ComposeNode {
      const entry = items[index];
      if (entry.type === 'line') return diffLine(entry.row, width);
      if (entry.type === 'pair') return UI.Row({ width, verticalAlignment: wrap ? 'topStart' : 'center' }, [diffLine(entry.left, leftWidth, false), UI.Box({ width: 1, height: wrap ? 0 : 22, modifier: M.background(c.outlineVariant.copy({ alpha: 0.32 })) }), diffLine(entry.right, rightWidth, true)]);
      if (entry.type === 'hunk') return UI.Box({ modifier: M.width(width).height(22).background({ ...c.onSurface.copy({ alpha: dark ? 0.055 : 0.045 }), over: c.surface }).padding(53, 0, 0, 0), contentAlignment: 'start' },
        text({ text: entry.header, fontSize: 11, lineHeight: 1, color: c.outline.copy({ alpha: 0.9 }), fontFamily: 'Consolas', fontFamilyFallback: ['Cascadia Mono', 'monospace'], maxLines: 1 }));
      return UI.InkWell({ onClick: entry.expandable ? () => expandGap(entry.newStartLine) : undefined }, UI.Row({ width, modifier: M.padding(0, 6), verticalAlignment: 'center' }, [
        UI.Box({ weight: 1 }, line(c.outlineVariant.copy({ alpha: 0.4 }))), UI.Spacer({ width: 10 }),
        UI.Box({ modifier: M.background(c.surfaceContainerHighest.copy({ alpha: 0.7 }), { cornerRadius: 20 }).border(1, c.outlineVariant.copy({ alpha: 0.4 }), { cornerRadius: 20 }).padding(11, 4) },
          UI.Row({ verticalAlignment: 'center' }, [UI.Icon({ name: entry.expandable ? 'unfold_more' : 'unfold_less', size: 12, tint: entry.expandable ? c.primary : c.outline }), UI.Spacer({ width: 5 }), text({ text: `${entry.count} 行未更改`, fontSize: 10.5, color: entry.expandable ? c.primary : c.outline })])),
        UI.Spacer({ width: 10 }), UI.Box({ weight: 1 }, line(c.outlineVariant.copy({ alpha: 0.4 }))),
      ]));
    }
    const lazy = ctx.lazyItems({ key: `diff:${source}:${file.path}:${split}:${wrap}`, itemCount: items.length, itemExtent: 22, initialItemCount: 48, variableExtent: true }, item);
    const list = UI.LazyColumn({ width, height: Math.max(1, size.height - 64), contentPadding: { top: 4, bottom: 24 }, ...lazy });
    const content = split ? UI.Box({ width, height: Math.max(1, size.height - 64), contentAlignment: 'topStart' }, [list,
      UI.Box({ key: 'diff-splitter', width: 12, fillMaxHeight: true, modifier: M.offset(leftWidth - 5.5, 0).cursor('resizeColumn').dragGestures({
        onDragStart: () => { splitDrag.current = ratio; },
        onDrag: event => { splitDrag.current = Math.min(1 - minimumRatio, Math.max(minimumRatio, splitDrag.current + event.deltaX / usableWidth)); setSplitRatio(splitDrag.current); },
      }) }),
    ]) : list;
    return width > mainWidth ? UI.ScrollArea({ axis: 'horizontal', thumbVisibility: true, trackVisibility: true }, content) : content;
  }

  /** Keeps the native file preview header, selectable body and local loading/error states. */
  function filePreview(): ComposeNode {
    if (preview === null) throw new Error('A preview file must be selected');
    const relative = workspace === null ? preview.path : preview.path.slice(workspace.replace(/\/$/, '').length + 1);
    return UI.Column({ fillMaxSize: true }, [
      UI.Column({ fillMaxWidth: true, horizontalAlignment: 'start', modifier: M.padding(12, 8) }, [
        UI.Row({ verticalAlignment: 'center', fillMaxWidth: true }, [UI.Icon({ name: 'description_outlined', size: 16, tint: c.primary }), UI.Spacer({ width: 7 }), text({ text: fileIdentity(preview.path).name, style: 'titleSmall', weight: 1, maxLines: 1, overflow: 'ellipsis' })]),
        UI.Spacer({ height: 2 }), text({ text: relative, style: 'bodySmall', color: c.outline, maxLines: 1, overflow: 'ellipsis' }),
      ]), line(), UI.Box({ weight: 1, fillMaxWidth: true }, previewLoading ? UI.Box({ fillMaxSize: true, contentAlignment: 'center' }, UI.CircularProgressIndicator({ width: 18, height: 18, strokeWidth: 2 }))
        : previewError !== '' ? UI.Box({ modifier: M.padding(12) }, text({ text: previewError }))
        : UI.ScrollArea({}, UI.Box({ modifier: M.padding(12) }, UI.SelectionContainer({}, UI.CodeText({ text: preview.content, filePath: preview.path, style: 'bodySmall', fontFamily: 'monospace' }))))),
    ]);
  }

  const noChanges = error || (!workspace ? '项目变更将显示在此处' : source === 'git' ? '当前分支工作区无未提交的 Git 改动'
    : source === 'branch' ? '当前分支与基准分支无差异或未检测到上游基准' : '项目变更将显示在此处');
  const body = preview !== null ? filePreview() : effective === null ? empty(noChanges)
    : loadingDiff ? UI.Box({ fillMaxSize: true, contentAlignment: 'center' }, UI.CircularProgressIndicator({ width: 22, height: 22, strokeWidth: 2 }))
    : error !== '' ? empty(error) : diffBody(effective);
  const sidebar = pane === 'tree' ? treeError !== '' ? UI.Box({ modifier: M.padding(12) }, text({ text: treeError }))
    : treeLoading ? UI.Box({ fillMaxSize: true, contentAlignment: 'center' }, UI.CircularProgressIndicator({ width: 18, height: 18, strokeWidth: 2 }))
    : workspace === null || (tree[workspace] ?? []).length === 0 ? empty('工作区暂无目录或文件')
    : UI.LazyColumn({ contentPadding: { vertical: 4 } }, treeRows(workspace))
    : rows.length === 0 ? empty(error || ('没有匹配的改动文件')) : UI.ScrollArea({ axis: 'horizontal', thumbVisibility: true }, UI.LazyColumn({
      width: visibleChanges.reduce((width, { node, depth }) => Math.max(width, 70 + depth * 14 + node.name.length * 12 + (node.kind === 'file' ? 24 + String(node.file.additions).length * 7 + String(node.file.deletions).length * 7 : 0)), sidebarWidth),
      height: Math.max(1, size.height - 80), contentPadding: { vertical: 2 },
    }, visibleChanges.map(changeRow)));
  return UI.Box({ modifier: M.fillMaxSize().onSizeChanged(next => { if (next.width !== size.width || next.height !== size.height) setSize(next); }), contentAlignment: 'topStart' }, [UI.Row({ fillMaxSize: true }, [
    ...(pane === 'closed' ? [] : [UI.Column({ width: sidebarWidth, fillMaxHeight: true, modifier: M.background(c.surfaceContainerHighest.copy({ alpha: dark ? 0.12 : 0.04 })) }, [
      sidebarHeader(), line(),
      UI.Box({ modifier: M.padding(8, 6) }, UI.Box({ modifier: M.height(27).background(c.surfaceContainerHighest.copy({ alpha: 0.35 }), { cornerRadius: 4 }).border(1, c.outlineVariant.copy({ alpha: 0.28 }), { cornerRadius: 4 }).padding(1) }, UI.Row({ verticalAlignment: 'center' }, [
        UI.Box({ modifier: M.padding(7, 0, 5, 0) }, UI.Icon({ name: 'search', size: 13, tint: c.outline.copy({ alpha: 0.7 }) })),
        UI.TextField({ weight: 1, value: query, onValueChange: setQuery, singleLine: true, cursorWidth: 1.2, cursorHeight: 13, placeholder: '筛选文件...', style: { inheritStyle: true, fontSize: 11.5, color: c.onSurface }, decoration: { isDense: true, borderless: true, contentPadding: { vertical: 5 }, hintStyle: { inheritStyle: true, fontSize: 11.5, color: c.outline.copy({ alpha: 0.5 }) } } }),
        ...(query === '' ? [] : [UI.InkWell({ onClick: () => setQuery(''), shape: { cornerRadius: 3 } }, UI.Box({ modifier: M.padding(6, 4) }, UI.Icon({ name: 'close', size: 12, tint: c.outline.copy({ alpha: 0.7 }) })))]),
      ]))), line(),
      UI.Box({ weight: 1, fillMaxWidth: true }, sidebar),
    ]), UI.Box({ width: 1, fillMaxHeight: true, modifier: M.background(divider) })]),
    UI.Column({ weight: 1, fillMaxHeight: true }, [toolbar(), line(), ...(effective === null || preview !== null ? [] : [ribbon(effective), line(c.outlineVariant.copy({ alpha: 0.25 }))]), UI.Box({ weight: 1, fillMaxWidth: true }, body)]),
  ]), ...(pane === 'closed' ? [] : [UI.Box({ key: 'sidebar-splitter', width: 12, fillMaxHeight: true, modifier: M.offset(sidebarWidth - 5.5, 0).cursor('resizeColumn').dragGestures({
    onDragStart: () => { listDrag.current = sidebarWidth; }, onDrag: event => dragSidebar(event.deltaX),
  }) })]),
  ]);
}
