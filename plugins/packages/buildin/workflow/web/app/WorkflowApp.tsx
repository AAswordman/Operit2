import React, { useState } from "react";
import {
  Alert,
  AppBar,
  Box,
  Button,
  Card,
  CardActions,
  CardContent,
  Chip,
  CssBaseline,
  GlobalStyles,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  FormControlLabel,
  IconButton,
  MenuItem,
  Stack,
  Switch,
  TextField,
  ThemeProvider,
  Toolbar,
  Tooltip,
  Typography,
  createTheme,
  useMediaQuery,
} from "@mui/material";
import ArrowBackIcon from "@mui/icons-material/ArrowBack";
import SettingsOutlinedIcon from "@mui/icons-material/SettingsOutlined";
import HistoryOutlinedIcon from "@mui/icons-material/HistoryOutlined";
import SaveOutlinedIcon from "@mui/icons-material/SaveOutlined";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import AddIcon from "@mui/icons-material/Add";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import FileUploadOutlinedIcon from "@mui/icons-material/FileUploadOutlined";
import AutoAwesomeOutlinedIcon from "@mui/icons-material/AutoAwesomeOutlined";
import BoltOutlinedIcon from "@mui/icons-material/BoltOutlined";
import BuildOutlinedIcon from "@mui/icons-material/BuildOutlined";
import CallSplitOutlinedIcon from "@mui/icons-material/CallSplitOutlined";
import AccountTreeOutlinedIcon from "@mui/icons-material/AccountTreeOutlined";
import FunctionsOutlinedIcon from "@mui/icons-material/FunctionsOutlined";
import ContentCopyOutlinedIcon from "@mui/icons-material/ContentCopyOutlined";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  Handle,
  Position,
  applyNodeChanges,
  type Node,
  type NodeProps,
  type ReactFlowProps,
  type NodeChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import "../styles/style.css";
import {
  copy,
  id,
  newNode,
  STYLES,
  values,
  type Workflow,
  type WorkflowNode,
  type Snapshot,
  type Value,
  type Run,
  type ToolDefinition,
  type ManifestWorkflowTemplate,
} from "../../src/model";
import { validateGraph, parseNode } from "../../src/validation";
import { templates } from "../../src/templates";
import type { Request } from "../../src/service";

declare global {
  interface Window {
    WorkflowHost: {
      request(request: Request): Promise<Snapshot>;
      exportFile(path: string, content: string): Promise<void>;
      currentTheme(): Promise<import("../../../../../types/compose-dsl").ComposeThemeSnapshot>;
    };
    receiveWorkflowProgress?: (run: Run) => void;
  }
}

interface TemplateOption {
  key: string;
  name: string;
  description: string;
  builtin?: Workflow;
  manifest?: ManifestWorkflowTemplate;
}

/** Combines the built-in and manifest-provided workflow template catalogs. */
function templateOptions(snapshot: Snapshot): TemplateOption[] {
  return [
    ...templates().map((template) => ({ key: `builtin:${template.id}`, name: template.name, description: template.description, builtin: template })),
    ...snapshot.manifestTemplates.map((template) => ({ key: `${template.sourceToolPkgId}:${template.templateId}`, name: template.displayName, description: template.description, manifest: template })),
  ];
}
import {
  fitOptions,
  flowOptions,
  nodeIcons,
  nodeTypes,
  statuses,
  type GraphNode,
  WorkflowCanvas,
} from "../components/WorkflowCanvas";
import { NodeForm } from "../components/NodeForm";
import { useHostTheme } from "./HostTheme";

/** Retains dialog content until its closing transition has finished. */
function useDialogContent<T>(empty: T) {
  const [state, setState] = useState({ value: empty, open: false });
  /** Changes content when opening and only changes visibility when closing. */
  function setValue(value: T) {
    setState((current) =>
      Object.is(value, empty)
        ? { ...current, open: false }
        : { value, open: true },
    );
  }
  /** Clears closed content without disturbing a newly opened dialog. */
  function onExited() {
    setState((current) =>
      current.open ? current : { value: empty, open: false },
    );
  }
  return { value: state.value, open: state.open, setValue, onExited };
}

/** Runs the Material list, graph editor and transactional dialogs inside the WebView. */
function App() {
  const hostTheme = useHostTheme();
  const dark = hostTheme?.brightness === "dark";
  const compact = useMediaQuery("(max-width:650px)");
  const theme = React.useMemo(
    () =>
      hostTheme && createTheme({
        palette: {
          mode: hostTheme.brightness === "dark" ? "dark" : "light",
          primary: { main: hostTheme.colors.primary, contrastText: hostTheme.colors.onPrimary },
          secondary: { main: hostTheme.colors.secondary, contrastText: hostTheme.colors.onSecondary },
          error: { main: hostTheme.colors.error, contrastText: hostTheme.colors.onError },
          background: { default: hostTheme.colors.background, paper: hostTheme.colors.surfaceContainer },
          text: { primary: hostTheme.colors.onSurface, secondary: hostTheme.colors.onSurfaceVariant },
          divider: hostTheme.colors.outlineVariant,
        },
        shape: { borderRadius: 16 },
        typography: {
          fontFamily:
            '"Segoe UI", "Microsoft YaHei", "PingFang SC", "Noto Sans CJK SC", sans-serif',
          button: { textTransform: "none" },
        },
        components: {
          MuiButton: { defaultProps: { disableElevation: true } },
          MuiDialog: { styleOverrides: { paper: { borderRadius: 24 } } },
        },
      }),
    [hostTheme],
  );
  const [snapshot, setSnapshot] = useState<Snapshot>({
    workflows: [],
    runs: [],
    manifestTemplates: [],
  });
  const [tools, setTools] = useState<ToolDefinition[]>([]);
  const [workflow, setWorkflow] = useState<Workflow | null>(null);
  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [ready, setReady] = useState(false),
    [busy, setBusy] = useState(false),
    [dirty, setDirty] = useState(false);
  const [error, setError] = useState("");
  const dialogState = useDialogContent("");
  const { value: dialog, setValue: setDialog } = dialogState;
  const draftState = useDialogContent<WorkflowNode | null>(null);
  const { value: draft, setValue: setDraft } = draftState;
  const [edgeId, setEdgeId] = useState("");
  const [name, setName] = useState(""),
    [description, setDescription] = useState(""),
    [text, setText] = useState("");
  const [run, setRun] = useState<Run | null>(null);
  const [liveRuns, setLiveRuns] = useState<Record<string, Run>>({});
  const [selected, setSelected] = useState<string | null>(null);
  const [exportPath, setExportPath] = useState("");
  const [nodePicker, setNodePicker] = useState(false);
  /** Displays operation failures while retaining the active draft. */
  async function perform(action: () => Promise<void> | void) {
    setError("");
    try {
      await action();
    } catch (failure) {
      setError(String(failure));
    }
  }
  /** Sends a durable operation through the existing main-runtime service. */
  async function request(message: Request) {
    const result = await window.WorkflowHost.request(message);
    setSnapshot(result);
    if (result.tools !== undefined) setTools(result.tools);
    setLiveRuns((current) => {
      const next = { ...current };
      for (const item of result.runs) {
        const known = next[item.workflowId];
        if (known === undefined || known.startedAt <= item.startedAt)
          next[item.workflowId] = item;
      }
      return next;
    });
    return result;
  }
  const workflowRef = React.useRef<Workflow | null>(null);
  React.useEffect(() => {
    workflowRef.current = workflow;
  }, [workflow]);
  /** Applies main-runtime node transitions to the open graph without polling. */
  React.useEffect(() => {
    window.receiveWorkflowProgress = (progress) => {
      setLiveRuns((current) => ({ ...current, [progress.workflowId]: progress }));
      setSnapshot((current) => ({
        workflows: current.workflows.map((item) =>
          item.id === progress.workflowId
            ? { ...item, lastExecutionStatus: progress.status }
            : item,
        ),
        runs: [
          progress,
          ...current.runs.filter((item) => item.id !== progress.id),
        ],
        manifestTemplates: current.manifestTemplates,
      }));
      if (workflowRef.current?.id !== progress.workflowId) return;
      setRun(progress);
      setNodes((current) =>
        current.map((item) => ({
          ...item,
          data: { ...item.data, result: progress.nodes[item.id]?.status },
        })),
      );
    };
    return () => {
      delete window.receiveWorkflowProgress;
    };
  }, []);
  React.useEffect(() => {
    void perform(async () => {
      /** Waits for the host bridge to finish installing page interfaces. */
      async function waitForWorkflowHost(): Promise<void> {
        if (typeof window.WorkflowHost?.request === "function") return;
        await new Promise<void>((resolve) => {
          window.addEventListener(
            "operitComposeDslInterfacesReady",
            () => resolve(),
            { once: true },
          );
        });
        if (typeof window.WorkflowHost?.request !== "function")
          throw new Error("工作流宿主接口未就绪");
      }
      await waitForWorkflowHost();
      window.applyWorkflowTheme(await window.WorkflowHost.currentTheme());
      await request({ action: "tool_catalog" });
      setReady(true);
    });
  }, []);
  /** Opens a saved graph at a readable initial scale. */
  function open(value: Workflow) {
    setWorkflow(copy(value));
    setDirty(false);
    const latest = liveRuns[value.id] ?? null;
    setRun(latest);
    setSelected(null);
    setNodes(
      value.nodes.map((node) => ({
        id: node.id,
        type: "workflow",
        position: node.position,
        data: { node, result: latest?.nodes[node.id]?.status },
      })),
    );
  }
  /** Applies a local transaction without sending pointer events over the bridge. */
  function edit(next: Workflow) {
    setWorkflow(next);
    setDirty(true);
    setNodes(
      next.nodes.map((node) => ({
        id: node.id,
        type: "workflow",
        position: node.position,
        data: { node },
      })),
    );
  }
  /** Validates and persists the entire graph as one revision. */
  async function save() {
    if (!workflow) throw new Error("没有打开工作流");
    validateGraph(workflow, false);
    const result = await request({ action: "save", workflow });
    const saved = result.workflows.find((item) => item.id === workflow.id);
    if (!saved) throw new Error("保存结果缺少工作流");
    setWorkflow(saved);
    setDirty(false);
    return saved;
  }
  /** Handles save and run operations without allowing duplicate submissions. */
  async function operation(action: () => Promise<void>) {
    setBusy(true);
    await perform(action);
    setBusy(false);
  }
  /** Removes a node only after checking all parameter references. */
  function removeNode() {
    if (!workflow || !draft) return;
    const dependents = workflow.nodes.filter(
      (node) =>
        node.id !== draft.id &&
        values(node).some(
          (value) => "nodeId" in value && value.nodeId === draft.id,
        ),
    );
    if (dependents.length)
      throw new Error(
        "请先修改参数引用：" + dependents.map((node) => node.name).join("、"),
      );
    edit({
      ...workflow,
      nodes: workflow.nodes.filter((node) => node.id !== draft.id),
      connections: workflow.connections.filter(
        (edge) =>
          edge.sourceNodeId !== draft.id && edge.targetNodeId !== draft.id,
      ),
    });
    setDraft(null);
  }
  /** Returns to the list, asking explicitly about unsaved graph edits. */
  function back() {
    if (dirty) setDialog("leave");
    else setWorkflow(null);
  }
  /** Opens a new node draft at the next uncluttered grid position. */
  function addNode(kind: WorkflowNode["type"]): void {
    if (!workflow) return;
    setDraft(
      newNode(
        kind,
        80 + (workflow.nodes.length % 4) * 280,
        80 + Math.floor(workflow.nodes.length / 4) * 160,
      ),
    );
    setNodePicker(false);
  }
  const workflowRunning = run?.status === "RUNNING";
  const editorLocked = busy || workflowRunning;
  const edges = React.useMemo(
    () =>
      workflow?.connections.map((edge) => ({
        id: edge.id,
        source: edge.sourceNodeId,
        target: edge.targetNodeId,
        label: edge.condition === null ? "" : edge.condition,
        type: "smoothstep",
        animated: run?.nodes[edge.targetNodeId]?.status === "running",
        className: "workflow-edge edge-" + (run?.nodes[edge.targetNodeId]?.status ?? "idle"),
      })) ?? [],
    [workflow?.connections, run],
  );
  if (!theme || !hostTheme) return null;
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <GlobalStyles styles={{ ":root": Object.fromEntries(
        Object.entries(hostTheme.colors).map(([role, color]) => [`--operit-${role}`, color]),
      ) }} />
      <Box className="app">
        <AppBar position="static" color="transparent" elevation={0}>
          <Toolbar
            className={`toolbar ${workflow ? "editor-toolbar" : "list-toolbar"}`}
          >
            {workflow && (
              <Button
                className="toolbar-back"
                aria-label="返回列表"
                startIcon={<ArrowBackIcon />}
                onClick={back}
              >
                <span className="action-label">返回列表</span>
              </Button>
            )}
            <Box className="title-block">
              <Typography variant="h6" noWrap>
                {workflow ? workflow.name : "我的工作流"}
              </Typography>
              <Typography variant="caption" color="text.secondary">
                {workflow
                  ? dirty
                    ? "有未保存的修改"
                    : "已保存"
                  : "编排节点，让重复的工作自动完成"}
              </Typography>
            </Box>
            {workflow ? (
              <Stack className="toolbar-actions" direction="row" spacing={0.5}>
                {!compact && (
                  <Button
                  className="toolbar-secondary"
                  aria-label="添加节点"
                  startIcon={<AddIcon />}
                    disabled={editorLocked}
                    onClick={() => setNodePicker(true)}
                  >
                    <span className="action-label">添加节点</span>
                  </Button>
                )}
                <Button
                  className="toolbar-secondary"
                  aria-label="设置"
                  startIcon={<SettingsOutlinedIcon />}
                  disabled={editorLocked}
                  onClick={() => {
                    setName(workflow.name);
                    setDescription(workflow.description);
                    setDialog("meta");
                  }}
                >
                  <span className="action-label">设置</span>
                </Button>
                <Button
                  className="toolbar-secondary"
                  aria-label="记录"
                  startIcon={<HistoryOutlinedIcon />}
                  disabled={busy}
                  onClick={() => {
                    setRun(null);
                    setDialog("logs");
                  }}
                >
                  <span className="action-label">记录</span>
                </Button>
                <Button
                  className="toolbar-secondary"
                  aria-label="保存"
                  startIcon={<SaveOutlinedIcon />}
                  disabled={editorLocked || !dirty}
                  onClick={() =>
                    operation(async () => {
                      await save();
                    })
                  }
                >
                  <span className="action-label">保存</span>
                </Button>
                <Button
                  className="run-action"
                  variant="contained"
                  startIcon={<PlayArrowIcon />}
                  disabled={editorLocked || !workflow.enabled}
                  onClick={() =>
                    operation(async () => {
                      const saved = dirty ? await save() : workflow;
                      setRun({
                        id: "",
                        workflowId: saved.id,
                        workflowName: saved.name,
                        triggerId: null,
                        status: "RUNNING",
                        startedAt: Date.now(),
                        finishedAt: null,
                        nodes: {},
                        logs: [],
                      });
                      setNodes((current) =>
                        current.map((item) => ({
                          ...item,
                          data: { ...item.data, result: "pending" },
                        })),
                      );
                      await request({
                        action: "start",
                        id: saved.id,
                        triggerId: null,
                        extras: {},
                      });
                    })
                  }
                >
                  {workflowRunning ? "执行中…" : "运行"}
                </Button>
              </Stack>
            ) : (
              <Stack className="toolbar-actions" direction="row" spacing={0.5}>
                <Button
                  className="toolbar-secondary"
                  aria-label="导入"
                  startIcon={<FileUploadOutlinedIcon />}
                  onClick={() => {
                    setText("");
                    setDialog("import");
                  }}
                >
                  <span className="action-label">导入</span>
                </Button>
                <Button
                  className="toolbar-secondary"
                  aria-label="模板"
                  startIcon={<AutoAwesomeOutlinedIcon />}
                  onClick={() => setDialog("templates")}
                >
                  <span className="action-label">模板</span>
                </Button>
                <Button
                  className="new-action"
                  variant="contained"
                  startIcon={<AddIcon />}
                  disabled={!ready}
                  onClick={() => {
                    setName("");
                    setDescription("");
                    setDialog("create");
                  }}
                >
                  新建
                </Button>
              </Stack>
            )}
          </Toolbar>
        </AppBar>
        <Divider />
        {error && (
          <Alert severity="error" onClose={() => setError("")}>
            {error}
          </Alert>
        )}
        {!ready ? (
          <Box sx={{ p: 4 }}>正在加载…</Box>
        ) : !workflow ? (
          <Box className="list">
            {!snapshot.workflows.length && (
              <Box className="empty">
                <Typography variant="h5">创建你的第一个工作流</Typography>
                <Typography color="text.secondary">
                  添加触发节点，然后连接工具与条件。
                </Typography>
                <Button
                  variant="contained"
                  onClick={() => {
                    setName("");
                    setDescription("");
                    setDialog("create");
                  }}
                >
                  新建工作流
                </Button>
              </Box>
            )}
            {snapshot.workflows.map((item) => (
              <Card key={item.id} className="workflow-card" variant="outlined">
                <CardContent
                  className="workflow-card-content"
                  onClick={() => open(item)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      open(item);
                    }
                  }}
                  role="button"
                  tabIndex={0}
                >
                  <Box className="workflow-card-heading">
                    <Box className="workflow-card-icon">
                      <AccountTreeOutlinedIcon />
                    </Box>
                    <Typography className="workflow-card-name" variant="h6" noWrap>
                      {item.name}
                    </Typography>
                    <Typography
                      className={
                        "workflow-card-status " +
                        (item.enabled ? "is-enabled" : "is-disabled")
                      }
                      variant="caption"
                    >
                      {item.enabled ? "已启用" : "已停用"}
                    </Typography>
                  </Box>
                  <Typography color="text.secondary" className="workflow-card-description">
                    {item.description || "暂无说明"}
                  </Typography>
                  <Stack className="workflow-card-meta" direction="row" spacing={1}>
                    <Chip size="small" label={`${item.nodes.length} 个节点`} />
                    {item.lastExecutionStatus && (
                      <Chip size="small" label={`最近 ${statuses[item.lastExecutionStatus]}`} />
                    )}
                  </Stack>
                </CardContent>
                <CardActions className="workflow-card-actions">
                  <Button size="small" variant="contained" onClick={() => open(item)}>
                    打开工作流
                  </Button>
                  <Tooltip title="复制工作流">
                    <IconButton
                      aria-label="复制工作流"
                      onClick={() =>
                        perform(async () => {
                          await request({ action: "copy", id: item.id });
                        })
                      }
                    >
                      <ContentCopyOutlinedIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                  <FormControlLabel
                    className="workflow-card-toggle"
                    label="启用"
                    control={
                      <Switch
                        size="small"
                        checked={item.enabled}
                        onChange={(_, checked) =>
                          perform(async () => {
                            await request({
                              action: "save",
                              workflow: { ...item, enabled: checked },
                            });
                          })
                        }
                      />
                    }
                  />
                </CardActions>
              </Card>
            ))}
          </Box>
        ) : (
          <Box className="editor">
            <Box className="canvas">
              <WorkflowCanvas
                key={workflow.id}
                nodes={nodes}
                edges={edges}
                nodeTypes={nodeTypes}
                colorMode={dark ? "dark" : "light"}
                fitView
                fitViewOptions={fitOptions}
                minZoom={0.2}
                maxZoom={1.6}
                proOptions={flowOptions}
                nodesDraggable={!editorLocked}
                nodesConnectable={!editorLocked}
                deleteKeyCode={null}
                onNodeDragStop={(_, moved) =>
                  edit({
                    ...workflow,
                    nodes: workflow.nodes.map((node) =>
                      node.id === moved.id
                        ? { ...node, position: moved.position }
                        : node,
                    ),
                  })
                }
                onNodeClick={(_, node) => setSelected(node.id)}
                onPaneClick={() => setSelected(null)}
                onNodeDoubleClick={(_, node) => {
                  if (!editorLocked) setDraft(copy(node.data.node));
                }}
                onEdgeClick={(_, edge) => {
                  if (!editorLocked) {
                    setEdgeId(edge.id);
                    setText(
                      workflow.connections.find((item) => item.id === edge.id)!
                        .condition ?? "",
                    );
                    setDialog("edge");
                  }
                }}
                onConnect={(connection) =>
                  perform(() => {
                    if (editorLocked) return;
                    const next = {
                      ...workflow,
                      connections: [
                        ...workflow.connections,
                        {
                          id: id("edge"),
                          sourceNodeId: connection.source,
                          targetNodeId: connection.target,
                          condition: null,
                        },
                      ],
                    };
                    validateGraph(next, false);
                    edit(next);
                  })
                }
              >
                <Background gap={24} size={1} />
                <Controls
                  showInteractive={false}
                  orientation={compact ? "horizontal" : "vertical"}
                  fitViewOptions={fitOptions}
                />
                {!compact && <MiniMap pannable zoomable />}
              </WorkflowCanvas>
              {!workflow.nodes.length && (
                <Box className="canvas-empty">
                  <Typography variant="h6">从触发节点开始</Typography>
                  <Typography color="text.secondary">
                    点击“添加节点”，从触发节点开始
                  </Typography>
                </Box>
              )}
            </Box>
          </Box>
        )}
        {workflow && selected && !compact && (
          <Button
            className="edit-selected"
            variant="contained"
            disabled={editorLocked}
            onClick={() => {
              const node = workflow.nodes.find((item) => item.id === selected);
              if (node) setDraft(copy(node));
            }}
          >
            编辑选中节点
          </Button>
        )}
        {workflow && compact && (
          <Box className="mobile-node-bar">
            {selected && (
              <Button
                variant="outlined"
                startIcon={<EditOutlinedIcon />}
                disabled={editorLocked}
                onClick={() => {
                  const node = workflow.nodes.find(
                    (item) => item.id === selected,
                  );
                  if (node) setDraft(copy(node));
                }}
              >
                编辑节点
              </Button>
            )}
            <Button
              variant="contained"
              startIcon={<AddIcon />}
              disabled={editorLocked}
              onClick={() => setNodePicker(true)}
            >
              添加节点
            </Button>
          </Box>
        )}
        <Dialog
          open={nodePicker}
          onClose={() => setNodePicker(false)}
          maxWidth="sm"
          fullWidth
        >
          <DialogTitle>添加节点</DialogTitle>
          <DialogContent>
            <Typography color="text.secondary" sx={{ mb: 2 }}>
              选择要放入画布的节点类型
            </Typography>
            <Box className="node-picker-grid">
              {Object.entries(STYLES).map(([kind, style]) => (
                <Button
                  key={kind}
                  className="node-picker-item"
                  variant="outlined"
                  startIcon={nodeIcons[kind as WorkflowNode["type"]]}
                  onClick={() => addNode(kind as WorkflowNode["type"])}
                >
                  {style.label}
                </Button>
              ))}
            </Box>
          </DialogContent>
          <DialogActions>
            <Button onClick={() => setNodePicker(false)}>关闭</Button>
          </DialogActions>
        </Dialog>
        <Dialog
          open={draftState.open}
          slotProps={{ transition: { onExited: draftState.onExited } }}
          onClose={() => setDraft(null)}
          maxWidth="sm"
          fullWidth
        >
          <DialogTitle>配置节点</DialogTitle>
          <DialogContent>
            {draft && workflow && (
              <NodeForm
                key={draft.id}
                node={draft}
                workflow={workflow}
                tools={tools}
                change={setDraft}
              />
            )}
            {error && <Alert severity="error">{error}</Alert>}
          </DialogContent>
          <DialogActions>
            <Button color="error" onClick={() => perform(removeNode)}>
              删除节点
            </Button>
            <Button onClick={() => setDraft(null)}>取消</Button>
            <Button
              variant="contained"
              onClick={() =>
                perform(() => {
                  if (!draft || !workflow) return;
                  const node = parseNode(draft);
                  const exists = workflow.nodes.some(
                    (item) => item.id === node.id,
                  );
                  edit({
                    ...workflow,
                    nodes: exists
                      ? workflow.nodes.map((item) =>
                          item.id === node.id ? node : item,
                        )
                      : [...workflow.nodes, node],
                  });
                  setDraft(null);
                })
              }
            >
              应用
            </Button>
          </DialogActions>
        </Dialog>
        <Dialog
          open={dialogState.open}
          slotProps={{ transition: { onExited: dialogState.onExited } }}
          onClose={() => {
            if (!busy) setDialog("");
          }}
          maxWidth="sm"
          fullWidth
        >
          <DialogTitle>
            {
              {
                create: "新建工作流",
                meta: "工作流设置",
                import: "导入工作流",
                export: "导出工作流",
                templates: "从模板创建",
                logs: "执行记录",
                edge: "连线条件",
                leave: "保存修改？",
                delete: "删除工作流？",
              }[dialog]
            }
          </DialogTitle>
          <DialogContent>
            {["create", "meta"].includes(dialog) && (
              <Stack spacing={2} sx={{ pt: 1 }}>
                <TextField
                  label="名称"
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                />
                <TextField
                  label="说明"
                  multiline
                  value={description}
                  onChange={(event) => setDescription(event.target.value)}
                />
                {dialog === "meta" && workflow && (
                  <>
                    <FormControlLabel
                      label="启用工作流"
                      control={
                        <Switch
                          checked={workflow.enabled}
                          onChange={(_, enabled) =>
                            edit({ ...workflow, enabled })
                          }
                        />
                      }
                    />
                    <Button
                      onClick={() => {
                        setText(JSON.stringify(workflow, null, 2));
                        setDialog("export");
                      }}
                    >
                      导出 JSON
                    </Button>
                    <Button color="error" onClick={() => setDialog("delete")}>
                      删除工作流
                    </Button>
                  </>
                )}
              </Stack>
            )}
            {["import", "export"].includes(dialog) && (
              <Stack spacing={2} sx={{ pt: 1 }}>
                <TextField
                  label="Workflow JSON"
                  multiline
                  minRows={8}
                  value={text}
                  slotProps={{ input: { readOnly: dialog === "export" } }}
                  onChange={(event) => setText(event.target.value)}
                />
                {dialog === "import" && (
                  <Button component="label">
                    选择 JSON 文件
                    <input
                      hidden
                      type="file"
                      accept="application/json,.json"
                      onChange={(event) => {
                        const file = event.target.files?.[0];
                        if (file)
                          void perform(async () => setText(await file.text()));
                      }}
                    />
                  </Button>
                )}
              </Stack>
            )}
            {dialog === "templates" && (
              <Stack spacing={1}>
                {templateOptions(snapshot).map((template) => (
                  <Button
                    key={template.key}
                    className="template-option"
                    variant="outlined"
                    onClick={() =>
                      perform(async () => {
                        const result = template.builtin !== undefined
                          ? await request({ action: "import", json: JSON.stringify(template.builtin) })
                          : await request({ action: "import_manifest_template", sourceToolPkgId: template.manifest!.sourceToolPkgId, templateId: template.manifest!.templateId });
                        open(result.workflows[result.workflows.length - 1]);
                        setDialog("");
                      })
                    }
                  >
                    <Stack spacing={0.5} alignItems="flex-start">
                      <Typography variant="subtitle2">{template.name}</Typography>
                      <Typography variant="caption" color="text.secondary">
                        {template.description}
                      </Typography>
                    </Stack>
                  </Button>
                ))}
              </Stack>
            )}
            {dialog === "logs" && (
              <Stack spacing={2}>
                {(run
                  ? [run]
                  : snapshot.runs
                      .filter((item) => item.workflowId === workflow?.id)
                      .sort((a, b) => b.startedAt - a.startedAt)
                ).map((item) => (
                  <Box key={item.id}>
                    <Chip label={statuses[item.status]} />
                    <Typography variant="caption">
                      {" "}
                      {new Date(item.startedAt).toLocaleString()}
                    </Typography>
                    {item.logs.map((log, index) => (
                      <Typography
                        key={index}
                        component="pre"
                        className="log"
                        color={log.level === "error" ? "error" : "text.primary"}
                      >
                        {log.message}
                      </Typography>
                    ))}
                  </Box>
                ))}
              </Stack>
            )}
            {dialog === "edge" && (
              <TextField
                fullWidth
                sx={{ mt: 1 }}
                label="条件（空：默认分支；false：否分支）"
                value={text}
                onChange={(event) => setText(event.target.value)}
              />
            )}
            {dialog === "leave" && (
              <Typography>工作流有未保存的修改。</Typography>
            )}
            {dialog === "delete" && (
              <Typography>将删除此工作流及其执行记录。</Typography>
            )}
            {dialog === "export" && (
              <Stack spacing={2} sx={{ mt: 2 }}>
                <TextField
                  label="保存路径"
                  value={exportPath}
                  onChange={(event) => setExportPath(event.target.value)}
                />
                <Button
                  disabled={!exportPath.trim() || busy}
                  onClick={() =>
                    operation(async () => {
                      await window.WorkflowHost.exportFile(exportPath, text);
                      setDialog("");
                    })
                  }
                >
                  保存 JSON 文件
                </Button>
              </Stack>
            )}
            {error && (
              <Alert severity="error" sx={{ mt: 2 }}>
                {error}
              </Alert>
            )}
          </DialogContent>
          <DialogActions>
            {dialog === "leave" && (
              <Button
                onClick={() => {
                  setWorkflow(null);
                  setDirty(false);
                  setDialog("");
                }}
              >
                放弃修改
              </Button>
            )}
            {dialog === "edge" && (
              <Button
                color="error"
                onClick={() => {
                  if (workflow)
                    edit({
                      ...workflow,
                      connections: workflow.connections.filter(
                        (edge) => edge.id !== edgeId,
                      ),
                    });
                  setDialog("");
                }}
              >
                删除连线
              </Button>
            )}
            <Button disabled={busy} onClick={() => setDialog("")}>
              关闭
            </Button>
            {!["logs", "templates", "export"].includes(dialog) && (
              <Button
                variant="contained"
                disabled={busy}
                onClick={() =>
                  operation(async () => {
                    if (dialog === "create") {
                      const result = await request({
                        action: "create",
                        name,
                        description,
                      });
                      open(result.workflows[result.workflows.length - 1]);
                    }
                    if (dialog === "meta" && workflow) {
                      if (!name.trim()) throw new Error("名称不能为空");
                      edit({ ...workflow, name, description });
                    }
                    if (dialog === "import") {
                      const result = await request({
                        action: "import",
                        json: text,
                      });
                      open(result.workflows[result.workflows.length - 1]);
                    }
                    if (dialog === "edge" && workflow)
                      edit({
                        ...workflow,
                        connections: workflow.connections.map((edge) =>
                          edge.id === edgeId
                            ? { ...edge, condition: text === "" ? null : text }
                            : edge,
                        ),
                      });
                    if (dialog === "leave") {
                      await save();
                      setWorkflow(null);
                    }
                    if (dialog === "delete" && workflow) {
                      await request({ action: "delete", ids: [workflow.id] });
                      setWorkflow(null);
                    }
                    setDialog("");
                  })
                }
              >
                {dialog === "leave" ? "保存并返回" : "确定"}
              </Button>
            )}
          </DialogActions>
        </Dialog>
      </Box>
    </ThemeProvider>
  );
}
export { App as WorkflowApp };
