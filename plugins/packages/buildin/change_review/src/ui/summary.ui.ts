import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import { fileIdentity, reviewForMessage, type ReviewFile, type ReviewMessage } from "../model";
import { fileIcon } from "../icons";

const ROUTE = "toolpkg:com.operit.change_review:ui:workspace";
const GREEN = "#FF3FB950";
const RED = "#FFF85149";

/** Reproduces the existing native message summary using plugin-owned declarative UI. */
export default function Summary(ctx: ComposeDslContext): ComposeNode {
  const [message] = ctx.useState<ReviewMessage | null>("message", null);
  const [chatId] = ctx.useState("chatId", "");
  const [locale] = ctx.useState("locale", "zh");
  const [streaming] = ctx.useState("isStreaming", false);
  const [expanded, setExpanded] = ctx.useState("files-expanded", true);
  const { UI, Modifier: M } = ctx;
  const c = ctx.MaterialTheme.colorScheme;
  const en = locale.startsWith("en");
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

  /** Keeps both count capsules at the original dimensions, colors and typography. */
  function capsule(value: number, added: boolean): ComposeNode {
    const color = added ? GREEN : RED;
    const alpha = { value: color, alpha: 0.12 };
    const border = { value: color, alpha: 0.28 };
    return UI.Box({ modifier: M.background(alpha, { cornerRadius: 4 }).border(0.8, border, { cornerRadius: 4 }).padding(5.8, 1.8) },
      UI.Text({ inheritStyle: true, text: `${added ? "+" : "-"}${value}`, color, fontWeight: "600", fontSize: 10.5, fontFamily: "monospace", lineHeight: 1.1 }));
  }

  /** Shares exact original inline count styling between filenames and the header. */
  function counts(file: ReviewFile): ComposeNode {
    return UI.RichText({ spans: [
      { text: `+${file.additions}`, color: GREEN, fontWeight: "600", fontSize: 11, fontFamily: "monospace" },
      { text: "  " },
      { text: `-${file.deletions}`, color: RED, fontWeight: "600", fontSize: 11, fontFamily: "monospace" },
    ] });
  }

  /** Mirrors filename emphasis, parent-directory dimming, hover delay and row action. */
  function fileRow(file: ReviewFile): ComposeNode {
    const { name, directory } = fileIdentity(file.path);
    return UI.InkWell({ shape: { cornerRadius: 6 }, onClick: () => open(file) },
      UI.Tooltip({ message: en ? `View diff for ${name}` : `查看 ${name} 的变更`, waitMillis: 400 },
        UI.Row({ modifier: M.padding(8, 5), verticalAlignment: "center" }, [
          UI.Svg({ source: fileIcon(name), width: 14, height: 14 }), UI.Spacer({ width: 8 }),
          UI.RichText({ weight: 1, maxLines: 1, overflow: "ellipsis", spans: [
            { text: name, fontSize: 12, fontWeight: "600", color: c.onSurface },
            ...(directory.length === 0 ? [] : [
              { text: "  " },
              { text: directory, fontSize: 11, fontFamily: "monospace", color: c.onSurfaceVariant.copy({ alpha: 0.65 }) },
            ]),
          ] }),
          UI.Spacer({ width: 8 }), counts(file), UI.Spacer({ width: 2 }),
          UI.Icon({ name: "chevron_right_rounded", size: 13, tint: c.onSurfaceVariant.copy({ alpha: 0.35 }) }),
        ])));
  }

  const header = UI.InkWell({
    shape: { topLeft: 10, topRight: 10, bottomLeft: expanded ? 0 : 10, bottomRight: expanded ? 0 : 10 },
    onClick: () => setExpanded(!expanded),
  }, UI.Row({ modifier: M.padding(10, 7), verticalAlignment: "center" }, [
    UI.Box({ modifier: M.background(c.primary.copy({ alpha: 0.12 }), { cornerRadius: 6 }).padding(4.5) },
      UI.Icon({ name: "difference_outlined", size: 14, tint: c.primary })),
    UI.Spacer({ width: 8 }),
    UI.Text({ text: en ? `Edited ${files.length} ${files.length === 1 ? "file" : "files"}` : `已编辑 ${files.length} 个文件`, style: "labelMedium", color: c.onSurface, fontWeight: "600", letterSpacing: 0.1 }),
    UI.Spacer({ width: 8 }),
    UI.Row({ verticalAlignment: "center" }, [capsule(totals.additions, true), UI.Spacer({ width: 4 }), capsule(totals.deletions, false)]),
    UI.Box({ weight: 1 }),
    UI.TextButton({ compact: true, contentColor: c.onSurfaceVariant.copy({ alpha: 0.8 }), contentPadding: { horizontal: 8, vertical: 4 },
      onClick: () => ({ type: "toolpkg.message.rollback" }) }, [
        UI.Icon({ name: "undo_rounded", size: 14 }), UI.Spacer({ width: 8 }),
        UI.Text({ inheritStyle: true, text: en ? "Undo" : "撤销", fontSize: 11.5 }),
      ]),
    UI.Spacer({ width: 4 }),
    UI.InkWell({ shape: { cornerRadius: 16 }, onClick: () => open() },
      UI.Box({ modifier: M.background(c.primary.copy({ alpha: 0.12 }), { cornerRadius: 16 }).border(0.8, c.primary.copy({ alpha: 0.35 }), { cornerRadius: 16 }).padding(9.8, 4.3) },
        UI.Row({ verticalAlignment: "center" }, [
          UI.Text({ inheritStyle: true, text: en ? "View changes" : "查看变更", fontSize: 11, fontWeight: "500", color: c.primary }),
          UI.Spacer({ width: 2 }), UI.Icon({ name: "arrow_forward_rounded", size: 11, tint: c.primary }),
        ]))),
    UI.Spacer({ width: 4 }),
    UI.Box({ modifier: M.padding(2) }, UI.Icon({ name: "keyboard_arrow_down", size: 16, tint: c.onSurfaceVariant.copy({ alpha: 0.75 }), turns: expanded ? 0.5 : 0, durationMillis: 200 })),
  ]));
  const body = UI.Box({ animation: { animateContentSize: true, durationMillis: 220, curve: "easeOutCubic", alignment: "topCenter" } },
    expanded ? UI.Column({ fillMaxWidth: true }, [
      UI.Divider({ height: 1, thickness: 0.8, color: c.outline.copy({ alpha: 0.15 }) }),
      UI.Box({ modifier: M.background(c.surfaceContainerHighest.copy({ alpha: 0.18 })).heightIn(0, 240) },
        UI.ScrollArea({ thumbVisibility: files.length > 5 },
          UI.Column({ modifier: M.padding(4), fillMaxWidth: true }, files.map(fileRow)))),
    ]) : UI.Spacer({ height: 0 }));
  return UI.Box({ modifier: M.fillMaxWidth().padding(12, 6, 12, 4) },
    UI.Card({ themeSurface: "card", material: false, containerColor: c.surfaceContainerLowest,
      shape: { cornerRadius: 10 }, border: { color: c.outline, alpha: 0.25, width: 0.8 } },
      UI.Column({ fillMaxWidth: true }, [header, body])));
}
