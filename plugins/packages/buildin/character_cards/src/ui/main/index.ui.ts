import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { renderScreen } from "../../ui-editor";
import { renderEntityScreen } from "../../ui-entity-native";
import type { UiPresentation } from "../../ui-contributions";
import { definition } from "../../definition";

/** Renders the management editor through a standalone default-exported Compose entry. */
export default function Screen(ctx: ComposeDslContext): ComposeNode {
  const [presentation] = ctx.useState<UiPresentation | null>("presentation", null);
  if (presentation !== null && (presentation.input.mode === "preview" || presentation.input.mode === "edit")) return renderEntityScreen(ctx, presentation);
  return renderScreen(ctx, definition);
}
