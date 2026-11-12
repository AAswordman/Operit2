import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { Definition } from "./model";
import { renderManagement } from "./ui-management";
import { renderMemoryAttachment } from "./ui-memory-attachment";

/** Routes native management and attachment screens without constructing a WebView. */
export function renderScreen(ctx: ComposeDslContext, _definition: Definition, requiredMode: "memory-attachment" | null = null, managementView: "characters" | "memory" = "characters"): ComposeNode {
  return requiredMode === "memory-attachment" ? renderMemoryAttachment(ctx) : renderManagement(ctx, managementView);
}
