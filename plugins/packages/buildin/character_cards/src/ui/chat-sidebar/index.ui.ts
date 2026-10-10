import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { renderSidebarScreen } from "../../ui-sidebar";

/** Renders the embedded sidebar without importing or evaluating the main service entry. */
export default function Screen(ctx: ComposeDslContext): ComposeNode { return renderSidebarScreen(ctx); }
