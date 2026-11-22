import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { CHANNEL, DEFAULTS, LABELS, MIN_WIDTH, MAX_WIDTH, petCaption, taskProgress, selectView, type Request, type Status, type Settings, type View } from "./model";

/** Sends UI operations to the main runtime instead of maintaining per-window task state. */
function call(request: Request): Promise<Status> { return ToolPkg.ipc.call<Request, Status>(CHANNEL, request, { targetRuntime: "main" }); }

/** Renders the animation with the actual selected task and execution round. */
export function petScreen(ctx: ComposeDslContext): ComposeNode {
  const [view] = ctx.useState<View>("view", selectView([], [], null, 0));
  const [sprite] = ctx.useState("sprite", "");
  const [error, setError] = ctx.useState("error", "");
  const [title, progress] = petCaption(view).split("\n");
  /** Opens current details without submitting any approval from a pet click. */
  async function open(): Promise<void> {
    try { await call({ kind: "details" }); setError(""); }
    catch (error) { setError(String(error)); }
  }
  return ctx.UI.Column({ fillMaxSize: true, horizontalAlignment: "center", spacing: 0 }, [
    ctx.UI.Image({ key: "task-cat", uri: sprite, animationKey: `${view.mood}:${view.sequence}`, contentScale: "fit", fillMaxWidth: true, weight: 1,
      contentDescription: `${LABELS[view.mood]}，点击查看任务，按住拖动`, modifier: ctx.Modifier.clickable(open) }),
    ctx.UI.Box({ height: 32, fillMaxWidth: true, contentAlignment: "center", modifier: ctx.Modifier.clickable(open) }, [
      ctx.UI.Column({ fillMaxWidth: true, horizontalAlignment: "center", spacing: 0,
        modifier: ctx.Modifier.background("#D91B3144", { type: "rounded", cornerRadius: 8 }).padding(3) }, [
        ctx.UI.Text({ text: error || title, fontSize: 10, maxLines: 1, color: "#EAF6FF" }),
        ...(!error ? [ctx.UI.Text({ text: progress, fontSize: 10, maxLines: 1, color: "#EAF6FF" })] : []),
      ]),
    ]),
  ]);
}

/** Shows live task details and explicit approval controls synchronized with the broker. */
export function detailsScreen(ctx: ComposeDslContext): ComposeNode {
  const { UI } = ctx;
  const colors = ctx.MaterialTheme.colorScheme;
  const [view] = ctx.useState<View>("view", selectView([], [], null, 0));
  const [busy, setBusy] = ctx.useState("busy", false);
  const submitting = ctx.useRef("submitting", false);
  const [error, setError] = ctx.useState("error", "");
  /** Prevents duplicate actions and displays stale-request errors without hiding them. */
  async function act(request: Request): Promise<void> {
    if (submitting.current) return;
    submitting.current = true; setBusy(true); setError("");
    try { await call(request); }
    catch (error) { setError(error instanceof Error ? error.message : String(error)); }
    finally { submitting.current = false; setBusy(false); }
  }
  const request = view.approvals[0];
  return UI.Card({ fillMaxSize: true, containerColor: colors.surfaceContainerHigh, shape: { type: "rounded", cornerRadius: 18 } }, [
    UI.Column({ fillMaxSize: true, padding: 16, spacing: 10 }, [
      UI.Row({ fillMaxWidth: true, verticalAlignment: "center" }, [
        UI.Text({ text: LABELS[view.mood], style: "titleLarge", weight: 1 }),
        UI.TextButton({ text: "收起", enabled: !busy, onClick: () => act({ kind: "closeDetails" }) }),
      ]),
      UI.ScrollArea({ weight: 1, fillMaxWidth: true }, [UI.Column({ fillMaxWidth: true, spacing: 10 }, [
        UI.Text({ text: view.message, style: "bodyMedium" }),
        ...(request ? [
          UI.Text({ text: `${view.approvals.length} 项待批准`, color: colors.primary }),
          UI.Text({ text: request.description, style: "titleMedium" }),
          UI.Text({ text: request.tool.name, style: "labelLarge" }),
          ...request.tool.parameters.map(parameter => UI.Column({ fillMaxWidth: true, spacing: 3 }, [
            UI.Text({ text: parameter.name, color: colors.primary }), UI.Text({ text: parameter.value, style: "bodySmall" }),
          ])),
        ] : view.tasks.map(task => UI.Column({ fillMaxWidth: true, spacing: 4 }, [
          UI.Text({ text: task.title || "当前任务", style: "titleSmall", maxLines: 2 }),
          UI.Text({ text: taskProgress(task), style: "bodyMedium" }),
          ...(task.state === "error" && task.message ? [UI.Text({ text: task.message, color: colors.error })] : []),
          UI.TextButton({ text: "切换到此对话", enabled: !busy, onClick: () => act({ kind: "switchChat", chatId: task.chatId }) }),
        ]))),
      ])]),
      ...(error ? [UI.Text({ text: error, color: colors.error, maxLines: 3 })] : []),
      ...(request ? [UI.Row({ fillMaxWidth: true, spacing: 8 }, [
        UI.Button({ text: "拒绝", weight: 1, enabled: !busy, onClick: () => act({ kind: "respond", chatId: request.chatId, requestId: request.requestId, decision: "deny" }) }),
        UI.Button({ text: "允许一次", weight: 1, enabled: !busy, onClick: () => act({ kind: "respond", chatId: request.chatId, requestId: request.requestId, decision: "allow" }) }),
      ])] : []),
      ...(view.chatId ? [UI.TextButton({ text: "切换到对应对话", enabled: !busy, onClick: () => act({ kind: "switchChat", chatId: view.chatId }) })] : []),
      UI.TextButton({ text: "隐藏任务猫", enabled: !busy, onClick: () => act({ kind: "hide" }) }),
    ]),
  ]);
}

/** Provides durable controls, explicit overlay authorization and truthful scope reporting. */
export function settingsScreen(ctx: ComposeDslContext): ComposeNode {
  const { UI } = ctx;
  const [settings, setSettings] = ctx.useState<Settings>("settings", { ...DEFAULTS });
  const [ready, setReady] = ctx.useState("ready", false);
  const [busy, setBusy] = ctx.useState("busy", false);
  const running = ctx.useRef("running", false);
  const [status, setStatus] = ctx.useState("status", "正在读取状态…");
  const [scope, setScope] = ctx.useState("scope", "");
  /** Applies a serialized command and refreshes controls from the main owner. */
  async function act(request: Request): Promise<void> {
    if (running.current) return;
    running.current = true; setBusy(true);
    try {
      const result = await call(request); setSettings(result.settings); setReady(true);
      setScope(result.scope === "system" ? "系统悬浮 · 可显示在其他应用上方" : result.scope === "application" ? "应用内悬浮 · 离开 Operit 后不可见" : "当前宿主不支持悬浮窗");
      setStatus(!result.permissionGranted ? "需要授予系统悬浮权限，授权返回后点击刷新。" : result.presentation === "shown" ? "任务猫已显示" : result.presentation === "unsupported" ? "当前宿主不支持悬浮窗" : "已就绪，将根据后台状态和设置显示");
    } catch (error) { setStatus(error instanceof Error ? error.message : String(error)); }
    finally { running.current = false; setBusy(false); }
  }
  /** Requests host permission only after an explicit click. */
  async function authorize(): Promise<void> {
    try { await ToolPkg.ui.floatingWindows.requestPermission(); await act({ kind: "status" }); }
    catch (error) { setStatus(String(error)); }
  }
  /** Saves each toggle as one validated settings object. */
  function toggle(key: "enabled" | "backgroundOnly" | "keepIdle", text: string): ComposeNode {
    return UI.Row({ fillMaxWidth: true, spacing: 12, verticalAlignment: "center" }, [UI.Text({ text, weight: 1 }),
      UI.Switch({ checked: settings[key], enabled: ready && !busy, onCheckedChange: value => act({ kind: "save", settings: { ...settings, [key]: value } }) })]);
  }
  return UI.LazyColumn({ fillMaxSize: true, padding: 20, spacing: 16, onLoad: () => act({ kind: "status" }) }, [
    UI.Text({ text: "任务状态猫", style: "headlineMedium" }),
    UI.Text({ text: "后台敲键盘，完成后休息；等待批准时抬爪，点击猫查看任务和审批。" }),
    UI.Text({ text: scope, style: "titleMedium" }), UI.Text({ text: status }),
    UI.FlowRow({ spacing: 8 }, [
      UI.Button({ text: "显示任务猫", enabled: ready && settings.enabled && !busy, onClick: () => act({ kind: "show" }) }),
      UI.TextButton({ text: "隐藏", enabled: ready && !busy, onClick: () => act({ kind: "hide" }) }),
      UI.TextButton({ text: "授予系统悬浮权限", enabled: !busy, onClick: authorize }),
      UI.TextButton({ text: "刷新", enabled: !busy, onClick: () => act({ kind: "status" }) }),
    ]),
    toggle("enabled", "启用任务猫"), toggle("backgroundOnly", "只在 Operit 后台时显示"), toggle("keepIdle", "完成后保留空闲状态"),
    UI.Text({ text: `大小 · ${settings.width} 逻辑像素` }),
    UI.Slider({ value: settings.width, min: MIN_WIDTH, max: MAX_WIDTH, divisions: (MAX_WIDTH - MIN_WIDTH) / 8,
      label: `${settings.width}`, enabled: ready && !busy,
      onValueChange: value => setSettings({ ...settings, width: Math.round(value) }),
      onValueChangeFinished: value => act({ kind: "save", settings: { ...settings, width: Math.round(value) } }) }),
    UI.Text({ text: "按住拖动；点击查看详情。多任务共用一只猫，待批准请求优先显示。单次完成动作 3 秒后转入空闲。", style: "bodySmall" }),
    UI.Text({ text: "Windows、macOS、Android、X11 使用系统悬浮能力；iOS、Web、OHOS、Wayland 当前只能在应用内显示。后台执行仍受宿主进程生命周期约束。", style: "bodySmall" }),
    UI.Text({ text: "只保存六个无损透明动画 WebP，不携带序列图集和预览副本。", style: "bodySmall" }),
  ]);
}
