import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { renderChatInputMenu } from "../../ui-input-menu";

/** Renders the original current-character menu row directly through the shared DSL host. */
export default function Screen(ctx: ComposeDslContext): ComposeNode { return renderChatInputMenu(ctx); }
