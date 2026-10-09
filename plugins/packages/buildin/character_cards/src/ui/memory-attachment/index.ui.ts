import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { renderScreen } from "../../host";
import { definition } from "../../definition";

/** Requires the explicit attachment presentation on its independently registered route. */
export default function Screen(ctx: ComposeDslContext): ComposeNode { return renderScreen(ctx, definition, "memory-attachment"); }
