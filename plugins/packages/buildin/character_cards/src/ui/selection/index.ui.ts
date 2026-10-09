import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { renderSelectionScreen } from "../../ui-selector";

/** Renders the compact native selector through its own serializable module path. */
export default function Screen(ctx: ComposeDslContext): ComposeNode { return renderSelectionScreen(ctx); }
