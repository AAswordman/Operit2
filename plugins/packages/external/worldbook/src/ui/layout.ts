import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";

/** Adapts worldbook panes and card rows to the measured plugin viewport. */
export function useWorldBookLayout(ctx: ComposeDslContext) {
  const [width, setWidth] = ctx.useState("worldbook-layout-width", 0);
  const measure = ctx.Modifier.onSizeChanged(size => {
    if (size.width !== width) setWidth(size.width);
  });
  const wide = width >= 900;

  /** Keeps editor panes stacked on phones and aligned at the top on wide screens. */
  function panes(key: string, children: ComposeNode[]): ComposeNode {
    return wide
      ? ctx.UI.Row({ key, fillMaxWidth: true, spacing: 16, verticalAlignment: "start" }, children.map((child, index) =>
        ctx.UI.Column({ key: `${key}-${index}`, weight: 1, spacing: 12 }, [child])))
      : ctx.UI.Column({ key, fillMaxWidth: true, spacing: 12 }, children);
  }

  /** Emits separately scrollable list rows without stretching the last card across empty columns. */
  function cardRows(key: string, children: ComposeNode[]): ComposeNode[] {
    const count = width >= 1320 ? 3 : wide ? 2 : 1;
    const rows: ComposeNode[] = [];
    for (let start = 0; start < children.length; start += count) {
      rows.push(ctx.UI.Row({ key: `${key}-${start}`, fillMaxWidth: true, spacing: 12, verticalAlignment: "start" },
        Array.from({ length: count }, (_, index) => ctx.UI.Column({ weight: 1 }, children.slice(start + index, start + index + 1)))));
    }
    return rows;
  }

  return { measure, wide, panes, cardRows };
}
