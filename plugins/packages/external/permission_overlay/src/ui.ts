import type { ComposeDslContext, ComposeNode, ComposeShape } from "../../../../types/compose-dsl";

const CARD: ComposeShape = { type: "rounded", cornerRadius: 24 };
const DETAILS: ComposeShape = { type: "rounded", cornerRadius: 16 };
const CONTROL: ComposeShape = { type: "rounded", cornerRadius: 12 };

/** Renders the original generation's centered shield card over a translucent full-scope scrim. */
export function approvalScreen(ctx: ComposeDslContext): ComposeNode {
  const { UI, Modifier } = ctx;
  const colors = ctx.MaterialTheme.colorScheme;
  const [request] = ctx.useState<ToolPkg.ToolPermissionRequest | null>("request", null);
  const [pendingCount] = ctx.useState("pendingCount", 0);
  const [size, setSize] = ctx.useState("size", { width: 0, height: 0 });
  const [response, setResponse] = ctx.useState("response", { requestId: "", busy: false, error: "" });
  const submitting = ctx.useRef("submitting", "");
  const activeRequest = ctx.useRef("activeRequest", "");
  activeRequest.current = request === null ? "" : request.requestId;
  const measure = Modifier.onSizeChanged(value => {
    if (value.width !== size.width || value.height !== size.height) setSize({ width: value.width, height: value.height });
  });
  const busy = request !== null && response.requestId === request.requestId && response.busy;
  const error = request !== null && response.requestId === request.requestId ? response.error : "";

  /** Submits exactly the displayed request; a replaced card never inherits its submission state. */
  async function respond(result: ToolPkg.ToolPermissionDecision): Promise<void> {
    if (request === null || submitting.current === request.requestId) return;
    const requestId = request.requestId;
    submitting.current = requestId;
    setResponse({ requestId, busy: true, error: "" });
    try {
      await ToolPkg.toolPermissions.respond(request.chatId, requestId, result);
    } catch (error) {
      console.error("Permission decision failed", error);
      if (activeRequest.current !== requestId) return;
      submitting.current = "";
      setResponse({ requestId, busy: false, error: error instanceof Error ? error.message : String(error) });
    }
  }

  /** Keeps the one-generation operation labels and readable full argument values. */
  function detail(label: string, value: string): ComposeNode {
    return UI.Column({ fillMaxWidth: true, spacing: 4 }, [
      UI.Text({ text: label, fontSize: 15, color: colors.onSurfaceVariant }),
      UI.Text({ text: value, fontSize: 17, fontWeight: "medium", color: colors.onSurface }),
    ]);
  }

  const card = request === null || size.width === 0 || size.height === 0 ? [] : [
    UI.Card({
      width: Math.min(560, size.width * 0.85),
      height: Math.min(size.height - 32, Math.max(340, Math.min(720, size.height * 0.65))),
      shape: CARD, elevation: 6, containerColor: colors.surfaceContainerHigh,
    }, [UI.Column({ fillMaxSize: true, padding: 20, spacing: 12, horizontalAlignment: "center" }, [
      UI.ScrollArea({ fillMaxWidth: true, weight: 1 }, [
        UI.Column({ fillMaxWidth: true, spacing: 8, horizontalAlignment: "center" }, [
          UI.Icon({ name: "Shield", size: 36, tint: colors.primary }),
          UI.Text({ text: "权限请求", fontSize: 20, fontWeight: "bold", color: colors.onSurface }),
          UI.Text({ text: "AI 助手请求执行以下操作", fontSize: 15, textAlign: "center", color: colors.onSurfaceVariant }),
          ...(pendingCount > 1 ? [UI.Text({ text: `还有 ${pendingCount - 1} 项待批准，将依次显示`, fontSize: 12, color: colors.primary })] : []),
          UI.Spacer({ height: 8 }),
          UI.Column({ fillMaxWidth: true, padding: 16, spacing: 16,
            modifier: Modifier.background(colors.surfaceContainer, DETAILS).clip(DETAILS) }, [
            detail("请求的操作", request.description),
            detail("使用的工具", request.tool.name),
            ...(request.tool.parameters.length === 0 ? [] : [
              UI.Text({ text: "参数详情", fontSize: 17, fontWeight: "medium", color: colors.onSurface }),
              UI.Column({ fillMaxWidth: true, padding: 10, spacing: 8,
                modifier: Modifier.background(colors.surfaceContainerLow, CONTROL).clip(CONTROL) },
              request.tool.parameters.map(parameter => UI.Column({ fillMaxWidth: true, spacing: 2 }, [
                UI.Text({ text: parameter.name, fontSize: 15, fontWeight: "medium", color: colors.primary }),
                UI.Text({ text: parameter.value, fontSize: 15, color: colors.onSurfaceVariant }),
              ]))),
            ]),
          ]),
        ]),
      ]),
      ...(error ? [UI.Text({ text: error, color: colors.error, fontSize: 12, maxLines: 3 })] : []),
      UI.Row({ fillMaxWidth: true, spacing: 12, verticalAlignment: "topStart" }, [
        UI.Button({ text: "拒绝", weight: 1, height: 44, shape: CONTROL, enabled: !busy,
          containerColor: colors.surfaceContainerHigh, contentColor: colors.primary,
          border: { width: 1, color: colors.outline }, onClick: () => respond("deny") }),
        UI.Column({ weight: 1, spacing: 4, horizontalAlignment: "center" }, [
          UI.Button({ text: busy ? "处理中…" : "允许", fillMaxWidth: true, height: 44, shape: CONTROL,
            enabled: !busy, onClick: () => respond("allow") }),
          UI.TextButton({ text: "本次运行始终允许", compact: true, enabled: !busy,
            onClick: () => respond("allow_session") }),
        ]),
      ]),
      UI.Text({ text: "持续授权仅在本次运行中生效，重启后清除。", fontSize: 12,
        textAlign: "center", color: colors.onSurfaceVariant }),
    ])]),
  ];
  return UI.Box({ fillMaxSize: true, backgroundColor: "#99000000", contentAlignment: "center",
    modifier: measure.safeDrawingPadding() }, card);
}

/** Shows the host's actual overlay scope and an explicit system permission action. */
export function settingsScreen(ctx: ComposeDslContext): ComposeNode {
  const { UI } = ctx;
  const [status, setStatus] = ctx.useState("status", "正在查询悬浮窗状态…");
  const [busy, setBusy] = ctx.useState("busy", false);
  const [canRequest, setCanRequest] = ctx.useState("canRequest", false);

  /** Reads capabilities again after returning from the host's permission settings. */
  async function refresh(): Promise<void> {
    setBusy(true);
    try {
      const caps = await ToolPkg.ui.floatingWindows.capabilities();
      setCanRequest(caps.supported && !caps.permissionGranted);
      setStatus(!caps.supported ? "当前宿主不支持悬浮窗。" : !caps.permissionGranted
        ? "尚未授予系统悬浮窗权限。授权并返回应用后，会自动显示待批准的请求。"
        : caps.scope === "system" ? "已就绪：需要批准时，会在其他应用上方显示权限弹窗。"
        : "已就绪：需要批准时，会在 Operit 应用内显示权限弹窗。");
    } catch (error) {
      console.error("Unable to read permission overlay capabilities", error);
      setStatus(error instanceof Error ? error.message : String(error));
    } finally { setBusy(false); }
  }

  /** Opens host authorization only from this explicit user action. */
  async function requestPermission(): Promise<void> {
    setBusy(true);
    try { await ToolPkg.ui.floatingWindows.requestPermission(); }
    catch (error) {
      console.error("Unable to request overlay permission", error);
      setStatus(error instanceof Error ? error.message : String(error));
    } finally { setBusy(false); }
  }

  ctx.useEffect("capabilities", () => { void refresh(); return () => {}; }, []);
  return UI.Column({ padding: 20, spacing: 16 }, [
    UI.Text({ text: "权限悬浮窗", style: "headlineSmall" }),
    UI.Text({ text: "工具需要批准时，显示一代风格的权限弹窗。聊天中的权限卡片同步可用，任意一处处理后会同时更新。" }),
    UI.Text({ text: status }),
    UI.Button({ text: "授予系统悬浮窗权限", enabled: canRequest && !busy, onClick: requestPermission }),
    UI.TextButton({ text: "刷新状态", enabled: !busy, onClick: refresh }),
    UI.Text({ text: "一次允许仅放行当前调用；持续授权会记住这个工具，作用于本次运行中的所有聊天，重启后清除。" }),
  ]);
}
