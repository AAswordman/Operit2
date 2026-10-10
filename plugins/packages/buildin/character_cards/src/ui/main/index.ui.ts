import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { renderScreen } from "../../host";
import { definition } from "../../definition";

/** Renders the management editor through a standalone default-exported Compose entry. */
export default function Screen(ctx: ComposeDslContext): ComposeNode { return renderScreen(ctx, definition); }
