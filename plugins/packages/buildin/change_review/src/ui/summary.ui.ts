import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import { fileIdentity, reviewForMessage, type ReviewFile, type ReviewMessage } from "../model";

const ROUTE = "toolpkg:com.operit.change_review:ui:workspace";

/** Keeps file paths and line changes readable without decorative color blocks. */
export default function Summary(ctx: ComposeDslContext): ComposeNode {
  const [message] = ctx.useState<ReviewMessage | null>("message", null);
  const [chatId] = ctx.useState("chatId", "");
  const [locale] = ctx.useState("locale", "zh");
  const [streaming] = ctx.useState("isStreaming", false);
  const [expanded, setExpanded] = ctx.useState("files-expanded", true);
  const { UI, Modifier: M } = ctx;
  const c = ctx.MaterialTheme.colorScheme;
  const en = locale.startsWith("en");
  const dark = ctx.Theme.getCurrent().brightness === "dark";
  const addedColor = dark ? "#FF8DD3A8" : "#FF267344";
  const removedColor = dark ? "#FFF0A5A0" : "#FFB23D38";
  const files = ctx.useMemo("review", () => message === null || message.sender !== 'ai' || streaming ? [] : reviewForMessage(message), [message, streaming]);
  if (message === null || message.sender !== "ai" || streaming || files.length === 0) return UI.Spacer({ width: 0, height: 0 });
  const totals = files.reduce((sum, file) => ({ additions: sum.additions + file.additions, deletions: sum.deletions + file.deletions }), { additions: 0, deletions: 0 });

  /** Opens the same plugin's full review with the selected file ordered first. */
  function open(file?: ReviewFile): ToolPkg.JsonValue {
    return {
      type: "toolpkg.ui.open", placement: "workspace", routeId: ROUTE,
      instanceKey: "change-review", input: {
        chatId, messageTimestamp: message!.timestamp, variantIndex: message!.variantIndex,
        files: file === undefined ? files : [file, ...files.filter(item => item.path !== file.path)],
      },
    };
  }

  /** Preserves both counts, including zero, with color confined to the numbers. */
  function count(value: number, added: boolean): ComposeNode {
    return UI.Text({ inheritStyle: true, text: `${added ? "+" : "−"}${value}`,
      color: value === 0 ? c.onSurfaceVariant : added ? addedColor : removedColor,
      fontWeight: "600", fontSize: 12, fontFamily: "monospace" });
  }

  /** Every row keeps its filename, parent path and individual change counts visible. */
  function fileRow(file: ReviewFile): ComposeNode {
    const { name, directory } = fileIdentity(file.path);
    return UI.InkWell({ shape: { cornerRadius: 6 }, onClick: () => open(file) },
      UI.Tooltip({ message: `${en ? "View changes" : "查看变更"} · ${file.path}`, waitMillis: 400 },
        UI.Row({ modifier: M.padding(12, 10), verticalAlignment: "center" }, [
          UI.Icon({ name: "insert_drive_file_outlined", size: 16, tint: c.onSurfaceVariant }),
          UI.Spacer({ width: 10 }),
          UI.RichText({ weight: 1, maxLines: 1, overflow: "ellipsis", spans: [
            { text: name, fontSize: 12.5, fontWeight: "600", color: c.onSurface },
            ...(directory.length === 0 ? [] : [
              { text: "  " },
              { text: directory, fontSize: 11.5, color: c.onSurfaceVariant },
            ]),
          ] }),
          UI.Spacer({ width: 10 }),
          count(file.additions, true), UI.Spacer({ width: 10 }), count(file.deletions, false),
          UI.Spacer({ width: 8 }),
          UI.Icon({ name: "chevron_right_rounded", size: 14, tint: c.onSurfaceVariant }),
        ])));
  }

  const header = UI.InkWell({
    shape: { topLeft: 10, topRight: 10, bottomLeft: expanded ? 0 : 10, bottomRight: expanded ? 0 : 10 },
    onClick: () => setExpanded(!expanded),
  }, UI.Row({ modifier: M.padding(12, 10), verticalAlignment: "center" }, [
    UI.Icon({ name: "difference_outlined", size: 15, tint: c.onSurfaceVariant }),
    UI.Spacer({ width: 8 }),
    UI.FlowRow({ weight: 1, spacing: 8, runSpacing: 4, itemVerticalAlignment: "center" }, [
      UI.Text({ maxLines: 1, overflow: "ellipsis", text: en ? `Edited ${files.length} ${files.length === 1 ? "file" : "files"}` : `已编辑 ${files.length} 个文件`, style: "labelMedium", color: c.onSurface, fontWeight: "600" }),
      count(totals.additions, true),
      count(totals.deletions, false),
    ]),
    UI.Spacer({ width: 8 }),
    UI.Tooltip({ message: en ? "Undo" : "撤销", waitMillis: 400 },
      UI.TextButton({ compact: true, contentColor: c.onSurfaceVariant, contentPadding: { horizontal: 6, vertical: 5 },
        onClick: () => ({ type: "toolpkg.message.rollback" }) }, [
        UI.Icon({ name: "undo_rounded", size: 15 }),
        UI.Spacer({ width: 6 }),
        UI.Text({ inheritStyle: true, text: en ? "Undo" : "撤销", fontSize: 11.5 }),
      ])),
    UI.Spacer({ width: 4 }),
    UI.InkWell({ shape: { cornerRadius: 6 }, onClick: () => open() },
      UI.Box({ modifier: M.padding(6, 5) },
        UI.Text({ inheritStyle: true, text: en ? "Review" : "查看变更", fontSize: 12, fontWeight: "500", color: c.onSurface }))),
    UI.Spacer({ width: 6 }),
    UI.Icon({ name: "keyboard_arrow_down", size: 16, tint: c.onSurfaceVariant, turns: expanded ? 0.5 : 0, durationMillis: 200 }),
  ]));
  const body = UI.Box({ animation: { animateContentSize: true, durationMillis: 220, curve: "easeOutCubic", alignment: "topCenter" } },
    expanded ? UI.Column({ fillMaxWidth: true }, [
      UI.Divider({ height: 1, thickness: 0.8, color: c.outlineVariant.copy({ alpha: 0.55 }) }),
      UI.Box({ modifier: M.heightIn(0, 240) },
        UI.ScrollArea({ thumbVisibility: files.length > 5 },
          UI.Column({ modifier: M.padding(4, 2), fillMaxWidth: true }, files.map(fileRow)))),
    ]) : UI.Spacer({ height: 0 }));
  return UI.Box({ modifier: M.fillMaxWidth().padding(12, 6, 12, 4) },
    UI.Card({ themeSurface: "card", material: false, containerColor: c.surfaceContainer,
      shape: { cornerRadius: 10 }, border: { color: c.outlineVariant, alpha: 0.65, width: 0.8 } },
      UI.Column({ fillMaxWidth: true }, [header, body])));
}
