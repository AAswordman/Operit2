import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { renderScreen } from "../../host";
import { definition } from "../../definition";

/** Independent memory route: no role route alias or in-page navigation. */
export default function Screen(ctx: ComposeDslContext): ComposeNode { return renderScreen(ctx, definition, null, "memory"); }
