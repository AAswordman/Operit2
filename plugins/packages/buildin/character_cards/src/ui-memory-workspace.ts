import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { Memory, MemoryGraphResult, MemoryLink } from "./model";
import { nativeUi, readUiJson, uiNumber, uiRequest, uiState } from "./ui-controls";
import { renderMemoryGraph } from "./ui-memory-graph";
import { renderMemorySettings } from "./ui-memory-settings";

export interface MemoryTarget { ownerKey: string; name: string; initial?: "browse" | "profile" | "settings" }
interface MemoryDraft { originalTitle: string | null; title: string; content: string; contentType: string; source: string; credibility: string; importance: string; folderPath: string; tags: string }
interface LinkDraft { id: string | null; sourceTitle: string; targetTitle: string; linkType: string; weight: string; description: string }
type MemoryPanel = { type: "memory"; draft: MemoryDraft } | { type: "link"; draft: LinkDraft }
  | { type: "profile"; content: string; loaded: boolean }
  | { type: "import"; strategy: "SKIP" | "UPDATE" | "CREATE_NEW"; content: string }
  | { type: "export"; content: string; name: string }
  | { type: "delete-memory"; title: string } | { type: "delete-link"; id: string }  | { type: "settings" };
interface WorkspaceState { data: MemoryGraphResult | null; panel: MemoryPanel | null; busy: boolean; error: string; notice: string; query: string; appliedQuery: string; searchIds: string[] | null; folder: string; inspectorExpanded: boolean; selected: string | null; selectedEdge: string | null; linkSource: string | null; linking: boolean; panelSequence: number }

/** Keeps the exact-folder inspector list and the original search-neighbor context on the canvas. */
export function filteredMemoryGraph(data: MemoryGraphResult, folder: string, searchIds: readonly string[] | null): MemoryGraphResult {
  const items = data.items.filter(item => (folder === "*" || (item.folderPath ?? "") === folder) && (searchIds === null || searchIds.includes(item.uuid)));
  const ids = new Set(items.map(item => item.uuid));
  if (searchIds !== null) {
    const matches = new Set(ids);
    for (const edge of data.graph.edges) {
      if (matches.has(edge.sourceId)) ids.add(edge.targetId);
      if (matches.has(edge.targetId)) ids.add(edge.sourceId);
    }
  }
  return { items, graph: { nodes: data.graph.nodes.filter(node => ids.has(node.id)), edges: data.graph.edges.filter(edge => ids.has(edge.sourceId) && ids.has(edge.targetId)) } };
}

/** Provides native memory browsing, editing and tools with an optional read-only web canvas. */
export function renderMemoryWorkspace(ctx: ComposeDslContext, target: MemoryTarget, close: () => void, instance: string): ComposeNode {
  const key = instance + ":" + target.ownerKey;
  const { state, live, update } = uiState<WorkspaceState>(ctx, key, { data: null, panel: target.initial === "settings" ? { type: "settings" } : target.initial === "profile" ? { type: "profile", content: "", loaded: false } : null, busy: false, error: "", notice: "", query: "", appliedQuery: "", searchIds: null, folder: "*", inspectorExpanded: false, selected: null, selectedEdge: null, linkSource: null, linking: false, panelSequence: 0 });
  const ui = nativeUi(ctx, !state.busy), ownerKey = target.ownerKey;
  const alive = ctx.useRef(key + ":alive", true);
  ctx.useEffect(key + ":lifetime", () => { alive.current = true; return () => { alive.current = false; }; }, []);
  /** Serializes operations and ignores completion after this owner has left the composition. */
  async function act(operation: () => Promise<void>): Promise<void> {
    if (live.current.busy || !alive.current) return;
    update({ busy: true, error: "", notice: "" });
    try { await operation(); } catch (error) { if (alive.current) update({ error: String(error) }); }
    finally { if (alive.current) update({ busy: false }); }
  }
  /** Opens a fresh native panel and removes the graph WebView from the tree. */
  function open(panel: MemoryPanel): void { update({ panel, panelSequence: live.current.panelSequence + 1, error: "" }); }
  /** Returns to the existing browse filters without reconstructing service state. */
  function back(): void { if (live.current.busy) return; if (target.initial !== undefined && target.initial !== "browse" && live.current.data === null) close(); else update({ panel: null, error: "" }); }
  /** Refreshes the complete graph, clearing any obsolete selection or search projection. */
  async function refresh(): Promise<void> {
    const data = await uiRequest({ action: "graph", ownerKey });
    if (alive.current) update({ data, searchIds: null, query: "", appliedQuery: "", selected: null, selectedEdge: null, linkSource: null });
  }
  /** Loads profile content before it can be written, never saving an unread empty draft. */
  async function profile(): Promise<void> {
    open({ type: "profile", content: "", loaded: false });
    await act(async () => { const result = await uiRequest({ action: "readUser", ownerKey }); if (alive.current) update({ panel: { type: "profile", content: result.content, loaded: true } }); });
  }
  /** Loads the canonical owner once when entering this workspace. */
  async function load(): Promise<void> { if (target.initial === "profile") { await profile(); return; } if (target.initial === "settings" || live.current.data !== null) return; await act(refresh); }
  /** Builds a memory draft without changing the original record until explicit save. */
  function editMemory(item: Memory | null): void {
    open({ type: "memory", draft: item === null ? { originalTitle: null, title: "", content: "", contentType: "text/plain", source: "manual", credibility: "0.5", importance: "0.5", folderPath: live.current.folder === "*" ? "" : live.current.folder, tags: "" } : { originalTitle: item.title, title: item.title, content: item.content, contentType: item.contentType, source: item.source, credibility: String(item.credibility), importance: String(item.importance), folderPath: item.folderPath ?? "", tags: item.tags.map(tag => tag.name).join(",") } });
  }
  /** Stages a new relationship between explicitly selected memories. */
  function newLink(sourceTitle = "", targetTitle = ""): void { open({ type: "link", draft: { id: null, sourceTitle, targetTitle, linkType: "related", weight: "1", description: "" } }); }
  /** Resolves an existing relationship through its full domain record. */
  async function editLink(id: string): Promise<void> {
    await act(async () => {
      const link: MemoryLink = await uiRequest({ action: "readMemoryLink", ownerKey, linkId: id });
      if (!alive.current) return;
      const source = live.current.data?.items.find(item => item.id === link.sourceMemoryId), destination = live.current.data?.items.find(item => item.id === link.targetMemoryId);
      if (source === undefined || destination === undefined) throw new Error("记忆关系端点不存在");
      open({ type: "link", draft: { id, sourceTitle: source.title, targetTitle: destination.title, linkType: link.type_, weight: String(link.weight), description: link.description } });
    });
  }
  /** Handles graph or list selections locally; two-node linking opens a native form. */
  function select(kind: "memory" | "edge", id: string): void {
    if (live.current.busy || live.current.panel !== null) return;
    if (kind === "edge") { update({ selected: null, selectedEdge: id }); return; }
    if (!live.current.linking) { update({ selected: id, selectedEdge: null }); return; }
    const first = live.current.linkSource;
    if (first === null) { update({ linkSource: id }); return; }
    if (first === id) return;
    const source = live.current.data?.items.find(item => item.uuid === first), destination = live.current.data?.items.find(item => item.uuid === id);
    if (source === undefined || destination === undefined) throw new Error("记忆节点不存在");
    newLink(source.title, destination.title);
  }
  /** Updates a memory draft from its current state rather than a stale field closure. */
  function memoryChange(patch: Partial<MemoryDraft>): void { const panel = live.current.panel; if (!live.current.busy && panel?.type === "memory") update({ panel: { ...panel, draft: { ...panel.draft, ...patch } } }); }
  /** Updates a relationship draft without changing its fixed persisted endpoints. */
  function linkChange(patch: Partial<LinkDraft>): void { const panel = live.current.panel; if (!live.current.busy && panel?.type === "link") update({ panel: { ...panel, draft: { ...panel.draft, ...patch } } }); }
  /** Commits one native draft through the same validated service used by tools. */
  async function saveMemory(): Promise<void> {
    await act(async () => { const panel = live.current.panel; if (panel?.type !== "memory") return; const draft = panel.draft;
      const data = await uiRequest({ action: "saveMemory", ownerKey, ...draft, credibility: uiNumber(draft.credibility, "可信度", 0, 1), importance: uiNumber(draft.importance, "重要性", 0, 1) });
      if (alive.current) update({ data, panel: null, searchIds: null, query: "", appliedQuery: "", selected: null, notice: "记忆已保存" });
    });
  }
  /** Commits a complete relationship and preserves service-assigned identities. */
  async function saveLink(): Promise<void> {
    await act(async () => { const panel = live.current.panel; if (panel?.type !== "link") return; const draft = panel.draft, weight = uiNumber(draft.weight, "关系强度", 0, 1);
      if (draft.linkType.trim() === "" || draft.sourceTitle === "" || draft.targetTitle === "" || draft.sourceTitle === draft.targetTitle) throw new Error("请选择两条不同记忆并填写关系类型");
      const data = await uiRequest(draft.id === null ? { action: "createLink", ownerKey, sourceTitle: draft.sourceTitle, targetTitle: draft.targetTitle, linkType: draft.linkType, weight, description: draft.description } : { action: "updateLink", ownerKey, linkId: draft.id, linkType: draft.linkType, weight, description: draft.description });
      if (alive.current) update({ data, panel: null, selectedEdge: null, linkSource: null, linking: false, notice: "关系已保存" });
    });
  }
  /** Renders all mutation and transfer forms using native fields. */
  function panelBody(panel: Exclude<MemoryPanel, { type: "settings" }>): ComposeNode[] {
    switch (panel.type) {
      case "memory": {
        const draft = panel.draft;
        return [ui.text(draft.originalTitle === null ? "创建记忆" : "编辑记忆", true), ui.field("memory-title", "标题 *", draft.title, title => memoryChange({ title })), ui.field("memory-content", "内容", draft.content, content => memoryChange({ content }), 8),
          ui.field("memory-content-type", "内容类型", draft.contentType, contentType => memoryChange({ contentType })), ui.field("memory-source", "来源", draft.source, source => memoryChange({ source })), ui.score("memory-credibility", "可信度", draft.credibility, credibility => memoryChange({ credibility })), ui.score("memory-importance", "重要性", draft.importance, importance => memoryChange({ importance })), ui.field("memory-folder", "文件夹", draft.folderPath, folderPath => memoryChange({ folderPath })), ui.field("memory-tags", "标签（逗号分隔）", draft.tags, tags => memoryChange({ tags })), ui.primary("memory-save", "保存", saveMemory)];
      }
      case "link": {
        const draft = panel.draft, options = (state.data?.items ?? []).map(item => ({ id: item.title, label: item.title }));
        return [ui.text(draft.id === null ? "创建记忆关系" : "编辑记忆关系", true), ...(draft.id === null ? [ui.choose("link-source", "源记忆", draft.sourceTitle, options, sourceTitle => linkChange({ sourceTitle })), ui.choose("link-target", "目标记忆", draft.targetTitle, options, targetTitle => linkChange({ targetTitle }))] : [ui.text(draft.sourceTitle + " → " + draft.targetTitle)]), ui.field("link-type", "关系类型", draft.linkType, linkType => linkChange({ linkType })), ui.field("link-weight", "强度（0–1）", draft.weight, weight => linkChange({ weight })), ui.field("link-description", "描述", draft.description, description => linkChange({ description }), 3), ui.primary("link-save", "保存", saveLink)];
      }
      case "profile": return [ui.text("用户资料 · USER.md", true), ...(panel.loaded ? [ui.field("memory-profile", "用户资料", panel.content, content => update({ panel: { ...panel, content } }), 12), ui.primary("memory-profile-save", "保存", () => act(async () => { const current = live.current.panel; if (current?.type !== "profile" || !current.loaded) throw new Error("用户资料尚未读取成功"); await uiRequest({ action: "writeUser", ownerKey, content: current.content }); if (alive.current) { if (live.current.data === null) close(); else update({ panel: null, notice: "用户资料已保存" }); } }))] : [ui.text(state.busy ? "正在读取用户资料…" : "用户资料读取失败"), ui.button("memory-profile-retry", "重试", profile)])];
      case "import": return [ui.text("导入记忆 JSON", true), ui.choose("memory-import-strategy", "相同 UUID 记忆处理", panel.strategy, [{ id: "SKIP", label: "跳过" }, { id: "UPDATE", label: "更新" }, { id: "CREATE_NEW", label: "创建新记忆" }], strategy => update({ panel: { ...panel, strategy: strategy as "SKIP" | "UPDATE" | "CREATE_NEW" } })), ui.text("选择记忆库导出的 JSON 备份文件，导入完整记忆、关系、文档分块及 USER.md。导入前建议先导出备份；只修改当前记忆库。"), ui.primary("memory-import-submit", "选择 JSON 文件并导入", () => act(async () => { const current = live.current.panel; if (current?.type !== "import") return; const content = await readUiJson(ctx); if (content === null) return; const result = await uiRequest({ action: "importMemory", ownerKey, content, strategy: current.strategy }); await refresh(); if (alive.current) update({ panel: null, notice: `新增 ${result.result.newMemories} · 更新 ${result.result.updatedMemories} · 跳过 ${result.result.skippedMemories} · 关系 ${result.result.newLinks}` }); }))];
      case "export": return [ui.text("导出 JSON", true), ctx.UI.TextField({ key: "memory-export-content", label: "JSON 内容", value: panel.content, readOnly: true, onValueChange: () => {}, fillMaxWidth: true, minLines: 16, maxLines: 24 }), ui.text("包含当前记忆库的完整记录与 USER.md。点击保存文件选择保存位置。"), ui.primary("memory-export-save", "保存文件", () => act(async () => { const current = live.current.panel; if (current?.type !== "export") return; const result = await ctx.saveFile({ suggestedName: current.name, content: current.content, mimeType: "application/json", extensions: ["json"] }); if (!result.cancelled && alive.current) update({ panel: null, notice: "文件已导出" }); }))];
      case "delete-memory": return [ui.text("确定删除记忆「" + panel.title + "」及关联关系？"), ui.primary("memory-delete-confirm", "确认删除", () => act(async () => { const data = await uiRequest({ action: "deleteMemory", ownerKey, title: panel.title }); if (alive.current) update({ data, panel: null, selected: null, selectedEdge: null, searchIds: null, notice: "记忆已删除" }); }))];
      case "delete-link": return [ui.text("确定删除这条记忆关系？"), ui.primary("link-delete-confirm", "确认删除", () => act(async () => { const data = await uiRequest({ action: "deleteLink", ownerKey, linkId: panel.id }); if (alive.current) update({ data, panel: null, selectedEdge: null, notice: "关系已删除" }); }))];

    }
  }
  /** Executes search with native controls and the production retrieval service. */
  async function search(): Promise<void> {
    await act(async () => { const query = live.current.query.trim(); const hits = query === "" ? null : await uiRequest({ action: "searchMemory", ownerKey, query }); if (alive.current) update({ searchIds: hits?.map(item => item.uuid) ?? null, appliedQuery: query, selected: null, selectedEdge: null }); });
  }
  const messages: ComposeNode[] = [];
  if (state.error !== "") messages.push(ctx.UI.Text({ key: "memory-workspace-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error }));
  if (state.notice !== "") messages.push(ui.text(state.notice));
  const layers: ComposeNode[] = [];
  const direct = target.initial !== undefined && target.initial !== "browse" && state.data === null;
  if (!direct) {
    const data = state.data, narrow = ui.size.width <= 840;
    const projection = data === null ? null : filteredMemoryGraph(data, state.folder, state.searchIds);
    const selected = data?.items.find(item => item.uuid === state.selected), edge = data?.graph.edges.find(item => item.id === state.selectedEdge);
    const header = ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 12, paddingVertical: 8, spacing: 8, verticalAlignment: "center" }, [
      ui.iconButton("memory-workspace-back", "关闭", "ArrowBack", close), ctx.UI.Column({ weight: 1 }, [ctx.UI.Text({ text: "记忆图谱", fontSize: 18, fontWeight: "600" }), ctx.UI.Text({ text: target.name, fontSize: 12, color: ctx.MaterialTheme.colorScheme.onSurfaceVariant, maxLines: 1, overflow: "ellipsis" })]),
      ui.menu("memory-more", "更多操作", "MoreVert", [
        { key: "memory-settings-open", label: "记忆设置", name: "Tune", action: () => open({ type: "settings" }) },
        { key: "memory-refresh", label: "刷新", name: "Refresh", enabled: data !== null, action: () => act(refresh) },
        { key: "memory-import", label: "导入记忆 JSON", name: "Upload", enabled: data !== null, action: () => open({ type: "import", strategy: "SKIP", content: "" }) },
        { key: "memory-export", label: "导出记忆 JSON", name: "Download", enabled: data !== null, action: () => act(async () => { const result = await uiRequest({ action: "exportMemory", ownerKey }); if (alive.current) open({ type: "export", name: "memory-backup.json", content: result.content }); }) },
      ], true),
    ]);
    let body: ComposeNode;
    if (data === null || projection === null) body = ui.list("memory-loading", [...messages, ui.text(state.busy ? "正在读取记忆图谱…" : "记忆图谱未加载成功，未开放写入。"), ui.button("memory-load-retry", "重试", () => act(refresh))]);
    else {
      const folders = [...new Set(["", ...data.items.map(item => item.folderPath ?? "")])].sort();
      const filters = ctx.UI.Column({ fillMaxWidth: true, paddingHorizontal: 12, paddingBottom: 10, spacing: 8 }, [
        ctx.UI.Row({ fillMaxWidth: true, height: 40, verticalAlignment: "center", spacing: 4, modifier: ctx.Modifier.clip({ cornerRadius: 12 }).background(ctx.MaterialTheme.colorScheme.surfaceContainerHigh) }, [ui.iconButton("memory-search", "搜索", "Search", search), ctx.UI.TextField({ key: "memory-search-query", placeholder: "搜索记忆…", decoration: { borderless: true, isDense: true, contentPadding: { horizontal: 4, vertical: 8 } }, style: { fontSize: 14 }, value: state.query, readOnly: state.busy, singleLine: true, weight: 1, onValueChange: query => update({ query }) }), ui.iconButton("memory-search-clear", "清除搜索", "Close", () => update({ query: "", appliedQuery: "", searchIds: null }))]),
        ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 12 }, [ui.choose("memory-folder-filter", "文件夹", state.folder, [{ id: "*", label: "全部文件夹" }, ...folders.map(folder => ({ id: folder, label: folder === "" ? "根目录" : folder }))], folder => update({ folder, selected: null, selectedEdge: null }), true), ctx.UI.FilterChip({ key: "memory-graph-link-mode", selected: state.linking, enabled: !state.busy && data.items.length > 1, label: ctx.UI.Text({ text: "链接" }), onClick: () => update({ linking: !live.current.linking, linkSource: null }) })]),
        ...(state.linking ? [ui.text(state.linkSource === null ? "选择两条记忆以建立链接 · 请选择源记忆" : "请选择目标记忆")] : []),
        ...(state.panel === null ? messages : []),
      ]);
      const details: ComposeNode[] = [];
      if (selected !== undefined) {
        details.push(ui.text("记忆详情"), ui.text(selected.title, true), ui.text(`${selected.contentType === "text/plain" ? "文本" : selected.contentType} · ${selected.source === "manual" ? "手动记录" : selected.source} · 可信度 ${selected.credibility.toFixed(2)} · 重要性 ${selected.importance.toFixed(2)}`), ui.text(selected.content), ui.actions([ui.button("memory-edit-selected", "编辑", () => editMemory(selected)), ui.button("memory-delete-selected", "删除", () => open({ type: "delete-memory", title: selected.title }))]));
        for (const relation of data.graph.edges.filter(item => item.sourceId === selected.uuid || item.targetId === selected.uuid)) details.push(ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center" }, [ctx.UI.Text({ text: `${relation.label ?? ""} · ${relation.weight.toFixed(2)}`, weight: 1 }), ui.iconButton("link-delete:" + relation.id, "删除关系", "DeleteOutline", () => open({ type: "delete-link", id: relation.id }))]));
      } else if (edge !== undefined) details.push(ui.text("关系详情"), ui.text("记忆关系", true), ui.text(edge.label ?? ""), ui.text("强度 " + edge.weight.toFixed(2)), ui.text(String(edge.metadata.description ?? "")), ui.actions([ui.button("link-edit-selected", "编辑关系", () => editLink(edge.id)), ui.button("link-delete-selected", "删除关系", () => open({ type: "delete-link", id: edge.id }))]));
      const items = projection.items.map(item => ctx.UI.Row({ key: "memory-select:" + item.uuid, fillMaxWidth: true, verticalAlignment: "center", spacing: 10, padding: 10, onClick: () => select("memory", item.uuid) }, [ctx.UI.Icon({ name: selected?.uuid === item.uuid ? "Check" : "PsychologyOutlined", size: 22 }), ctx.UI.Column({ weight: 1 }, [ctx.UI.Text({ text: item.title, fontSize: 13, fontWeight: "500" }), ctx.UI.Text({ text: item.content.slice(0, 70), fontSize: 12, color: ctx.MaterialTheme.colorScheme.onSurfaceVariant, maxLines: 2, overflow: "ellipsis" })])]));
      const inspector = ctx.UI.Column({ key: "memory-inspector", ...(narrow ? { weight: state.inspectorExpanded ? 1 : 2 } : { width: 304, fillMaxHeight: true }), modifier: ctx.Modifier.background(ctx.MaterialTheme.colorScheme.surfaceContainer) }, [ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 16, paddingVertical: 8, verticalAlignment: "center" }, [ctx.UI.Icon({ name: "PsychologyOutlined", size: 18 }), ctx.UI.Text({ text: `记忆库 ${projection.items.length}`, fontSize: 14, fontWeight: "600", weight: 1 }), ...(narrow ? [ui.button("memory-inspector-toggle", state.inspectorExpanded ? "收起" : "展开", () => update({ inspectorExpanded: !live.current.inspectorExpanded }))] : [])]), ui.list("memory-inspector-list", [...details, ...items, ...(items.length === 0 ? [ui.text("没有匹配的记忆")] : [])])]);
      const canvas = ctx.UI.Box({ key: "memory-canvas", weight: narrow ? 3 : 1, fillMaxWidth: narrow, fillMaxHeight: !narrow }, [
        ...(state.panel === null ? [renderMemoryGraph(ctx, key + ":canvas", { data: projection, selected: state.linkSource ?? state.selected, selectedEdge: state.selectedEdge, theme: ctx.Theme.getCurrent() }, select, error => update({ error }))] : []),
        ctx.UI.Box({ modifier: ctx.Modifier.align("bottomEnd"), padding: 16 }, ctx.UI.FloatingActionButton({ key: "memory-create", icon: "Add", text: "创建记忆", onClick: () => editMemory(null) })),
      ]);
      body = ctx.UI.Column({ fillMaxSize: true }, [filters, ctx.UI.Box({ weight: 1, fillMaxWidth: true }, [
        (narrow ? ctx.UI.Column : ctx.UI.Row)({ fillMaxSize: true }, [...(narrow && state.inspectorExpanded ? [] : [canvas]), inspector]),

      ])]);
    }
    layers.push(ui.dialog("memory-graph-dialog", "记忆图谱", body, [], close, "graph", undefined, header));
  }
  if (state.panel?.type === "settings") layers.push(renderMemorySettings(ctx, target, back, key + ":settings:" + state.panelSequence));
  else if (state.panel !== null) {
    const panel = state.panel, fields = panelBody(panel), titleNode = fields.shift();
    const submit = fields.pop()!;
    const title = panel.type === "memory" ? panel.draft.originalTitle === null ? "创建记忆" : "编辑记忆" : panel.type === "link" ? panel.draft.id === null ? "创建记忆关系" : "编辑记忆关系" : panel.type === "profile" ? "用户资料 · USER.md" : panel.type === "import" ? "导入记忆 JSON" : panel.type === "export" ? "导出 JSON" : panel.type === "delete-memory" ? "删除记忆" : "删除关系";
    const confirmation = panel.type === "delete-memory" || panel.type === "delete-link";
    layers.push(ui.dialog("memory-form-dialog", title, ui.list("memory-native-form", [...messages, ...(confirmation && titleNode ? [titleNode] : []), ...fields]), [ui.button("memory-form-cancel", "取消", back), submit], back, panel.type === "memory" || panel.type === "profile" || panel.type === "export" ? "large" : confirmation || panel.type === "import" ? "compact" : "normal", undefined, undefined, panel.type === "import" ? 360 : undefined));
  }
  return ctx.UI.Box({ key: "memory-workspace-root", fillMaxSize: true, onLoad: load }, layers);
}
