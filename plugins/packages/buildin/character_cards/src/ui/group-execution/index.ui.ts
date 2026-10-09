import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { renderGroupExecutionScreen } from "../../group-execution/control";

/** Renders package-owned group controls through their independent Compose entry. */
export default function Screen(ctx: ComposeDslContext): ComposeNode { return renderGroupExecutionScreen(ctx); }
