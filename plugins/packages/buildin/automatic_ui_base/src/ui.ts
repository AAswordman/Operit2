import type { ComposeCanvasBrush, ComposeDslContext, ComposeNode, ComposeShape } from "../../../../types/compose-dsl";

const FRAME: ComposeCanvasBrush = { type: "VerticalGradient", colors: ["#67F5E2", "#6C9CFF", "#A791FF"] };
const GLASS: ComposeCanvasBrush = { type: "VerticalGradient", colors: ["#263854", "#101526"] };
const CIRCLE: ComposeShape = { type: "circle" };

/** Host-driven aurora, breathing bloom and perimeter glints; the center stays clear. */
export function indicatorScreen({ UI, Modifier }: ComposeDslContext): ComposeNode {
  return UI.Box({ key: "automation-indicator", fillMaxSize: true }, [
    UI.GlowBorder({
      key: "automation-glow",
      fillMaxSize: true,
      colors: ["#67F5E2", "#62ABFF", "#A791FF", "#F094D8"],
      cornerRadius: 0, strokeWidth: 3, glowWidth: 56,
      durationMillis: 6000,
    }),
    UI.Box({
      width: 58, height: 58, contentAlignment: "center",
      modifier: Modifier.align("topEnd").safeDrawingPadding().offset(-18, 30)
        .background(GLASS, CIRCLE).border(1.5, FRAME, CIRCLE),
    }, [
      UI.CircularProgressIndicator({ width: 43, height: 43, color: "#6BF6DF", strokeWidth: 2 }),
      UI.Icon({ name: "TouchApp", size: 22, tint: "#F2FFFD" }),
    ]),
  ]);
}
