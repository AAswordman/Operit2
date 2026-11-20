import type { ComposeCanvasBrush, ComposeDslContext, ComposeNode, ComposeShape } from "../../../../types/compose-dsl";

const FRAME: ComposeCanvasBrush = { type: "VerticalGradient", colors: ["#67F5E2", "#6C9CFF", "#A791FF"] };
const GLASS: ComposeCanvasBrush = { type: "VerticalGradient", colors: ["#263854", "#101526"] };
const CIRCLE: ComposeShape = { type: "circle" };
const OUTLINE: ComposeShape = { type: "rounded", cornerRadius: 19 };

/** Paints the full outline and its non-interactive animated status orb. */
export function indicatorScreen({ UI, Modifier }: ComposeDslContext): ComposeNode {
  return UI.Box({ fillMaxSize: true }, [
    UI.Box({
      fillMaxSize: true,
      modifier: Modifier.padding(2).border(2.5, FRAME, OUTLINE),
    }),
    UI.Box({
      width: 58, height: 58, contentAlignment: "center",
      modifier: Modifier.align("topEnd").offset(-18, 30)
        .background(GLASS, CIRCLE).border(1.5, FRAME, CIRCLE),
    }, [
      UI.CircularProgressIndicator({ width: 43, height: 43, color: "#6BF6DF", strokeWidth: 2 }),
      UI.Icon({ name: "TouchApp", size: 22, tint: "#F2FFFD" }),
    ]),
  ]);
}

/** Shows the real window scope and requests permission only on user action. */
export function statusScreen(ctx: ComposeDslContext): ComposeNode {
  const { UI } = ctx;
  const zh = getLang().split("-")[0] === "zh";
  const [caps, setCaps] = ctx.useState<ToolPkg.FloatingWindowCapabilities | null>("scope", null);

  /** Reads the current host state after permission changes. */
  async function refresh(): Promise<void> {
    setCaps(await ToolPkg.ui.floatingWindows.capabilities());
  }

  return UI.Column({ fillMaxWidth: true, padding: 20, spacing: 14, onLoad: refresh }, [
    UI.Text({ text: zh ? "运行提示" : "Activity indicator", style: "headlineSmall" }),
    UI.Text({
      text: caps === null ? (zh ? "正在查询显示范围…" : "Checking display scope…") :
        caps.scope === "system" ? (zh ? "离开 Operit 后，在主屏显示状态球和描边。" : "Shows the orb and outline on the primary display while Operit is in the background.") :
        caps.scope === "application" ? (zh ? "在 Operit 视图内显示状态球和描边。" : "Shows the orb and outline within the Operit view.") :
        (zh ? "当前宿主不提供悬浮窗。" : "This host does not provide floating windows."),
    }),
    ...(caps?.scope === "system" && !caps.permissionGranted ? [UI.Button({
      text: zh ? "授予悬浮窗权限" : "Grant overlay permission",
      onClick: async () => { await ToolPkg.ui.floatingWindows.requestPermission(); await refresh(); },
    })] : []),
    UI.TextButton({ text: zh ? "刷新显示能力" : "Refresh display capabilities", onClick: refresh }),
  ]);
}
