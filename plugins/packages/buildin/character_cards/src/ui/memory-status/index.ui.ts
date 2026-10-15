import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { record } from "../../domain";
import { requireId, assertString } from "../../validation";

/** Reproduces the original memory-status AlertDialog after the generic host dismisses its root menu overlay. */
export default function Screen(ctx: ComposeDslContext): ComposeNode {
  const [presentation] = ctx.useState<unknown>("presentation", null);
  const value = record(presentation, "memory status presentation");
  const requestId = requireId(value.requestId, "memory status request id");
  const input = record(value.input, "memory status input"); assertString(input.description, "memory status description");
  const original = ctx.useRef("memory-status-request", requestId);
  if (original.current !== requestId) throw new Error("Memory status presentation changed");
  function close(): ToolPkg.JsonValue { return { type: "toolpkg.presentation.cancel", requestId }; }
  return ctx.UI.AlertDialog({ key: "memory-status", title: "记忆状态", text: input.description,
    confirmText: "关闭", onConfirm: close, onDismissRequest: close });
}
