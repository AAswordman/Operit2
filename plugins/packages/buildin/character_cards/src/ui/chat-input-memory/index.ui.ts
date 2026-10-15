import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { renderMemoryMenu } from "../../ui-memory-menu";

/** Supplies the original memory group at the registered generic menu-section surface. */
export default function Screen(ctx: ComposeDslContext): ComposeNode { return renderMemoryMenu(ctx); }
