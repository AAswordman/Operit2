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
