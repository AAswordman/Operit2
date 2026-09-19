import React, { useState } from "react";
import { createRoot } from "react-dom/client";
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
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  Drawer,
  FormControlLabel,
  IconButton,
  MenuItem,
  Stack,
  Switch,
  TextField,
  ThemeProvider,
  Toolbar,
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
import CloseIcon from "@mui/icons-material/Close";
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
import "./style.css";
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
} from "../src/model";
import { validateGraph, parseNode } from "../src/validation";
import { templates } from "../src/templates";
import type { Request } from "../src/service";

declare global {
  interface Window {
    WorkflowHost: {
      request(request: Request): Promise<Snapshot>;
      exportFile(path: string, content: string): Promise<void>;
    };
  }
}
type GraphNode = Node<{ node: WorkflowNode; result?: string }>;
const statuses: Record<string, string> = {
  RUNNING: "运行中",
  SUCCESS: "成功",
  FAILED: "失败",
  CANCELLED: "已取消",
  pending: "等待",
  running: "运行中",
  success: "成功",
  failed: "失败",
  skipped: "跳过",
};

/** Renders a compact node with standard graph handles and execution state. */
function WorkflowCard({ data, selected }: NodeProps<GraphNode>) {
  const style = STYLES[data.node.type];
  return (
    <div
      className={"graph-node" + (selected ? " selected" : "")}
      style={{ borderColor: style.color }}
    >
      {data.node.type !== "trigger" && (
        <Handle type="target" position={Position.Left} />
      )}
      <div className="node-kind" style={{ color: style.color }}>
        {style.label}
        {data.result && " · " + statuses[data.result]}
      </div>
      <strong>{data.node.name}</strong>
      <div className="node-description">
        {data.node.description || "双击配置节点"}
      </div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}
const nodeTypes = {
  workflow: React.memo(
    WorkflowCard,
    (previous, next) =>
      previous.data === next.data && previous.selected === next.selected,
  ),
};
const fitOptions = { maxZoom: 1, padding: 0.2 };
const flowOptions = { hideAttribution: true };

/** Keeps pointer-frequency node updates inside the canvas instead of the application shell. */
function WorkflowCanvas({
  nodes: incomingNodes,
  ...props
}: ReactFlowProps<GraphNode> & { nodes: GraphNode[] }) {
  const [nodes, setNodes] = useState(incomingNodes);
  const [source, setSource] = useState(incomingNodes);
  // Synchronize committed edits during render so an effect cannot overwrite an ongoing gesture.
  if (source !== incomingNodes) {
    setSource(incomingNodes);
    setNodes(incomingNodes);
  }
  /** Applies positions, selection and measured dimensions only within this canvas. */
  const changeNodes = React.useCallback((changes: NodeChange<GraphNode>[]) => {
    setNodes((current) => applyNodeChanges(changes, current));
  }, []);
  return <ReactFlow {...props} nodes={nodes} onNodesChange={changeNodes} />;
}
const nodeIcons: Record<WorkflowNode["type"], React.ReactNode> = {
  trigger: <BoltOutlinedIcon />,
  execute: <BuildOutlinedIcon />,
  condition: <CallSplitOutlinedIcon />,
  logic: <AccountTreeOutlinedIcon />,
  extract: <FunctionsOutlinedIcon />,
};

/** Edits literals and upstream references using the persisted parameter contract. */
function Parameter({
  label,
  value,
  nodes,
  change,
}: {
  label: string;
  value: Value;
  nodes: WorkflowNode[];
  change(value: Value): void;
}) {
  return (
    <Stack spacing={1}>
      <Typography variant="body2">{label}</Typography>
      <TextField
        select
        size="small"
        label="值来源"
        value={"value" in value ? "literal" : "reference"}
        onChange={(event) => {
          if (event.target.value === "literal") change({ value: "" });
          else if (nodes.length) change({ nodeId: nodes[0].id });
        }}
      >
        <MenuItem value="literal">固定值</MenuItem>
        <MenuItem value="reference" disabled={!nodes.length}>
          节点输出
        </MenuItem>
      </TextField>
      {"value" in value ? (
        <TextField
          size="small"
          multiline
          label={label}
          value={value.value}
          onChange={(event) => change({ value: event.target.value })}
        />
      ) : (
        <TextField
          select
          size="small"
          label="来源节点"
          value={value.nodeId}
          onChange={(event) => change({ nodeId: event.target.value })}
        >
          {nodes.map((node) => (
            <MenuItem key={node.id} value={node.id}>
              {node.name}
            </MenuItem>
          ))}
        </TextField>
      )}
    </Stack>
  );
}

/** Edits every node kind without changing the execution schema. */
function NodeForm({
  node,
  workflow,
  change,
}: {
  node: WorkflowNode;
  workflow: Workflow;
  change(node: WorkflowNode): void;
}) {
  const [key, setKey] = useState("");
  const sources = workflow.nodes.filter((item) => item.id !== node.id);
  /** Updates a typed field on the current node draft. */
  function set(field: string, value: unknown) {
    change({ ...node, [field]: value } as WorkflowNode);
  }
  /** Builds a controlled text or integer input for one draft field. */
  function field(
    label: string,
    name: string,
    value: string | number,
    numeric = false,
  ) {
    return (
      <TextField
        key={name}
        size="small"
        label={label}
        type={numeric ? "number" : "text"}
        value={value}
        onChange={(event) =>
          set(name, numeric ? Number(event.target.value) : event.target.value)
        }
      />
    );
  }
  /** Builds an exact-choice selector for a draft field. */
  function select(
    label: string,
    name: string,
    value: string,
    options: string[],
  ) {
    return (
      <TextField
        select
        size="small"
        label={label}
        value={value}
        onChange={(event) => set(name, event.target.value)}
      >
        {options.map((item) => (
          <MenuItem key={item} value={item}>
            {item}
          </MenuItem>
        ))}
      </TextField>
    );
  }
  /** Changes one schedule field while retaining the remaining configuration. */
  function config(name: string, value: string) {
    if (node.type === "trigger")
      change({
        ...node,
        triggerConfig: { ...node.triggerConfig, [name]: value },
      });
  }
  return (
    <Stack spacing={2} sx={{ pt: 1 }}>
      {field("节点名称", "name", node.name)}
      {field("说明", "description", node.description)}
      {node.type === "trigger" && (
        <>
          <TextField
            select
            size="small"
            label="触发方式"
            value={node.triggerType}
            onChange={(event) => {
              const kind = event.target.value as typeof node.triggerType;
              change({
                ...node,
                triggerType: kind,
                triggerConfig:
                  kind === "schedule"
                    ? {
                        schedule_type: "interval",
                        interval_ms: "900000",
                        enabled: "true",
                        repeat: "true",
                      }
                    : kind === "event"
                      ? { topic: "app.lifecycle.resumed" }
                      : {},
              });
            }}
          >
            {Object.entries({
              manual: "手动",
              schedule: "定时",
              app_open: "应用启动",
              event: "宿主事件",
            }).map(([value, label]) => (
              <MenuItem key={value} value={value}>
                {label}
              </MenuItem>
            ))}
          </TextField>
          {node.triggerType === "event" && (
            <TextField
              label="事件主题"
              size="small"
              value={node.triggerConfig.topic}
              onChange={(event) => config("topic", event.target.value)}
            />
          )}
          {node.triggerType === "schedule" && (
            <>
              <TextField
                select
                size="small"
                label="定时方式"
                value={node.triggerConfig.schedule_type}
                onChange={(event) =>
                  change({
                    ...node,
                    triggerConfig: {
                      enabled: "true",
                      repeat: "true",
                      schedule_type: event.target.value,
                      ...{
                        interval: { interval_ms: "900000" },
                        specific_time: { specific_time: "2026-12-31 12:00:00" },
                        cron: { cron_expression: "0 9 * * *" },
                      }[event.target.value],
                    },
                  })
                }
              >
                {["interval", "specific_time", "cron"].map((value) => (
                  <MenuItem key={value} value={value}>
                    {value}
                  </MenuItem>
                ))}
              </TextField>
              {Object.entries(node.triggerConfig)
                .filter(
                  ([name]) =>
                    !["enabled", "repeat", "schedule_type"].includes(name),
                )
                .map(([name, value]) => (
                  <TextField
                    key={name}
                    size="small"
                    label={name}
                    value={value}
                    onChange={(event) => config(name, event.target.value)}
                  />
                ))}
              {["enabled", "repeat"].map((name) => (
                <FormControlLabel
                  key={name}
                  label={name === "enabled" ? "启用定时" : "重复"}
                  control={
                    <Switch
                      checked={node.triggerConfig[name] === "true"}
                      onChange={(_, checked) => config(name, String(checked))}
                    />
                  }
                />
              ))}
            </>
          )}
        </>
      )}
      {node.type === "execute" && (
        <>
          {field("工具名称（包名:工具名）", "actionType", node.actionType)}
          {Object.entries(node.actionConfig).map(([name, value]) => (
            <Stack key={name} spacing={1}>
              <Parameter
                label={name}
                value={value}
                nodes={sources}
                change={(next) =>
                  set("actionConfig", { ...node.actionConfig, [name]: next })
                }
              />
              <Button
                color="error"
                onClick={() => {
                  const next = { ...node.actionConfig };
                  delete next[name];
                  set("actionConfig", next);
                }}
              >
                移除参数
              </Button>
            </Stack>
          ))}
          <Stack direction="row" spacing={1}>
            <TextField
              label="参数名称"
              size="small"
              value={key}
              onChange={(event) => setKey(event.target.value)}
            />
            <Button
              disabled={
                !key.trim() || Object.hasOwn(node.actionConfig, key.trim())
              }
              onClick={() => {
                set("actionConfig", {
                  ...node.actionConfig,
                  [key.trim()]: { value: "" },
                });
                setKey("");
              }}
            >
              添加参数
            </Button>
          </Stack>
          <FormControlLabel
            label="JavaScript 执行"
            control={
              <Switch
                checked={node.jsCode !== null}
                onChange={(_, checked) =>
                  set("jsCode", checked ? "return inputs;" : null)
                }
              />
            }
          />
          {node.jsCode !== null && (
            <TextField
              multiline
              minRows={6}
              label="脚本（inputs、trigger、Tools、toolCall）"
              value={node.jsCode}
              onChange={(event) => set("jsCode", event.target.value)}
            />
          )}
        </>
      )}
      {node.type === "condition" && (
        <>
          <Parameter
            label="左值"
            value={node.left}
            nodes={sources}
            change={(value) => set("left", value)}
          />
          {select("比较方式", "operator", node.operator, [
            "EQ",
            "NE",
            "GT",
            "GTE",
            "LT",
            "LTE",
            "CONTAINS",
            "NOT_CONTAINS",
            "IN",
            "NOT_IN",
          ])}
          <Parameter
            label="右值"
            value={node.right}
            nodes={sources}
            change={(value) => set("right", value)}
          />
        </>
      )}
      {node.type === "logic" &&
        select("逻辑运算", "operator", node.operator, ["AND", "OR"])}
      {node.type === "extract" && (
        <>
          {select("运算方式", "mode", node.mode, [
            "REGEX",
            "JSON",
            "SUB",
            "CONCAT",
            "RANDOM_INT",
            "RANDOM_STRING",
          ])}
          <Parameter
            label="输入值"
            value={node.source}
            nodes={sources}
            change={(value) => set("source", value)}
          />
          {["REGEX", "JSON"].includes(node.mode) &&
            field("表达式 / JSON 路径", "expression", node.expression)}
          {node.mode === "REGEX" && field("捕获组", "group", node.group, true)}
          {node.mode === "SUB" && (
            <>
              {field("起始位置", "startIndex", node.startIndex, true)}
              {field("长度（-1 到结尾）", "length", node.length, true)}
            </>
          )}
          {node.mode === "CONCAT" && (
            <>
              {node.others.map((value, index) => (
                <Stack key={index}>
                  <Parameter
                    label={"拼接值 " + (index + 1)}
                    value={value}
                    nodes={sources}
                    change={(next) =>
                      set(
                        "others",
                        node.others.map((item, i) =>
                          i === index ? next : item,
                        ),
                      )
                    }
                  />
                  <Button
                    onClick={() =>
                      set(
                        "others",
                        node.others.filter((_, i) => i !== index),
                      )
                    }
                  >
                    移除
                  </Button>
                </Stack>
              ))}
              <Button
                onClick={() => set("others", [...node.others, { value: "" }])}
              >
                添加拼接值
              </Button>
            </>
          )}
          {["RANDOM_INT", "RANDOM_STRING"].includes(node.mode) && (
            <>
              <FormControlLabel
                label="使用固定值"
                control={
                  <Switch
                    checked={node.useFixed}
                    onChange={(_, checked) => set("useFixed", checked)}
                  />
                }
              />
              {node.useFixed ? (
                field("固定值", "fixedValue", node.fixedValue)
              ) : node.mode === "RANDOM_INT" ? (
                <>
                  {field("最小值", "randomMin", node.randomMin, true)}
                  {field("最大值", "randomMax", node.randomMax, true)}
                </>
              ) : (
                <>
                  {field(
                    "长度",
                    "randomStringLength",
                    node.randomStringLength,
                    true,
                  )}
                  {field(
                    "字符集",
                    "randomStringCharset",
                    node.randomStringCharset,
                  )}
                </>
              )}
            </>
          )}
        </>
      )}
    </Stack>
  );
}

/** Runs the Material list, graph editor and transactional dialogs inside the WebView. */
function App() {
  const dark = useMediaQuery("(prefers-color-scheme: dark)");
  const compact = useMediaQuery("(max-width:650px)");
  const theme = React.useMemo(
    () =>
      createTheme({
        palette: {
          mode: dark ? "dark" : "light",
          primary: { main: dark ? "#a0cde4" : "#286783" },
          background: { default: dark ? "#151c20" : "#f7f9fc" },
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
    [dark],
  );
  const [snapshot, setSnapshot] = useState<Snapshot>({
    workflows: [],
    runs: [],
  });
  const [workflow, setWorkflow] = useState<Workflow | null>(null);
  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [ready, setReady] = useState(false),
    [busy, setBusy] = useState(false),
    [dirty, setDirty] = useState(false);
  const [error, setError] = useState(""),
    [dialog, setDialog] = useState("");
  const [draft, setDraft] = useState<WorkflowNode | null>(null),
    [edgeId, setEdgeId] = useState("");
  const [name, setName] = useState(""),
    [description, setDescription] = useState(""),
    [text, setText] = useState("");
  const [run, setRun] = useState<Run | null>(null);
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
    return result;
  }
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
      await request({ action: "list" });
      setReady(true);
    });
  }, []);
  /** Opens a saved graph at a readable initial scale. */
  function open(value: Workflow) {
    setWorkflow(copy(value));
    setDirty(false);
    setRun(null);
    setSelected(null);
    setNodes(
      value.nodes.map((node) => ({
        id: node.id,
        type: "workflow",
        position: node.position,
        data: { node },
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
  const edges = React.useMemo(
    () =>
      workflow?.connections.map((edge) => ({
        id: edge.id,
        source: edge.sourceNodeId,
        target: edge.targetNodeId,
        label: edge.condition === null ? "" : edge.condition,
        type: "smoothstep",
        animated: busy,
      })) ?? [],
    [workflow?.connections, busy],
  );
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
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
                disabled={busy}
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
                <Button
                  className="toolbar-secondary"
                  aria-label="设置"
                  startIcon={<SettingsOutlinedIcon />}
                  disabled={busy}
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
                  disabled={busy || !dirty}
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
                  disabled={busy || !workflow.enabled}
                  onClick={() =>
                    operation(async () => {
                      const saved = dirty ? await save() : workflow;
                      const result = await request({
                        action: "run",
                        id: saved.id,
                        triggerId: null,
                        extras: {},
                      });
                      const latest = result.runs
                        .filter((item) => item.workflowId === saved.id)
                        .sort((a, b) => b.startedAt - a.startedAt)[0];
                      setRun(latest);
                      setDialog("logs");
                      setNodes((current) =>
                        current.map((node) => ({
                          ...node,
                          data: {
                            ...node.data,
                            result: latest.nodes[node.id]?.status,
                          },
                        })),
                      );
                    })
                  }
                >
                  {busy ? "执行中…" : "运行"}
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
              <Card key={item.id} variant="outlined">
                <CardContent
                  onClick={() => open(item)}
                  sx={{ cursor: "pointer" }}
                >
                  <Typography variant="h6" noWrap>
                    {item.name}
                  </Typography>
                  <Typography color="text.secondary" className="description">
                    {item.description || "暂无说明"}
                  </Typography>
                  <Stack direction="row" spacing={1}>
                    <Chip size="small" label={item.nodes.length + " 个节点"} />
                    <Chip
                      size="small"
                      label={item.enabled ? "已启用" : "已停用"}
                    />
                    {item.lastExecutionStatus && (
                      <Chip
                        size="small"
                        label={statuses[item.lastExecutionStatus]}
                      />
                    )}
                  </Stack>
                </CardContent>
                <CardActions>
                  <Button onClick={() => open(item)}>打开工作流</Button>
                  <Button
                    onClick={() =>
                      perform(async () => {
                        await request({ action: "copy", id: item.id });
                      })
                    }
                  >
                    复制
                  </Button>
                  <FormControlLabel
                    sx={{ ml: "auto" }}
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
            {!compact && (
              <Stack className="palette" direction="row" spacing={1}>
                {Object.entries(STYLES).map(([kind, style]) => (
                  <Button
                    key={kind}
                    disabled={busy}
                    variant="outlined"
                    startIcon={nodeIcons[kind as WorkflowNode["type"]]}
                    onClick={() => addNode(kind as WorkflowNode["type"])}
                  >
                    {style.label}
                  </Button>
                ))}
                <Typography className="hint" variant="caption">
                  双击配置 · 拖动端口连线 · 滚轮缩放
                </Typography>
              </Stack>
            )}
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
                nodesDraggable={!busy}
                nodesConnectable={!busy}
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
                  if (!busy) setDraft(copy(node.data.node));
                }}
                onEdgeClick={(_, edge) => {
                  if (!busy) {
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
            disabled={busy}
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
                disabled={busy}
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
              disabled={busy}
              onClick={() => setNodePicker(true)}
            >
              添加节点
            </Button>
          </Box>
        )}
        <Drawer
          anchor="bottom"
          open={nodePicker}
          onClose={() => setNodePicker(false)}
          slotProps={{ paper: { className: "node-picker-sheet" } }}
        >
          <Box className="node-picker-header">
            <Box>
              <Typography variant="h6">添加节点</Typography>
              <Typography variant="body2" color="text.secondary">
                选择要放入画布的节点类型
              </Typography>
            </Box>
            <IconButton aria-label="关闭" onClick={() => setNodePicker(false)}>
              <CloseIcon />
            </IconButton>
          </Box>
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
        </Drawer>
        <Dialog
          open={draft !== null}
          onClose={() => setDraft(null)}
          maxWidth="sm"
          fullWidth
          fullScreen={compact}
        >
          <DialogTitle>配置节点</DialogTitle>
          <DialogContent>
            {draft && workflow && (
              <NodeForm
                key={draft.id}
                node={draft}
                workflow={workflow}
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
          open={dialog !== ""}
          onClose={() => {
            if (!busy) setDialog("");
          }}
          maxWidth="sm"
          fullWidth
          fullScreen={compact}
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
                {templates().map((template) => (
                  <Button
                    key={template.name}
                    onClick={() =>
                      perform(async () => {
                        const result = await request({
                          action: "import",
                          json: JSON.stringify(template),
                        });
                        open(result.workflows[result.workflows.length - 1]);
                        setDialog("");
                      })
                    }
                  >
                    {template.name}
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
createRoot(document.getElementById("root")!).render(<App />);
