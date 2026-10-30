import type { ComposeDslContext, ComposeNode } from '../../../../../types/compose-dsl';

/** Presents the original workspace entry card under the review plugin's ownership. */
export default function Home(ctx: ComposeDslContext): ComposeNode {
  const { UI, Modifier: M } = ctx, c = ctx.MaterialTheme.colorScheme;
  /** Requests the registered review page without encoding host navigation or Git behavior. */
  function open(): ToolPkg.JsonValue {
    return { type: 'toolpkg.ui.open', placement: 'workspace', routeId: 'toolpkg:com.operit.change_review:ui:workspace', instanceKey: 'change-review', input: {} };
  }
  return UI.OutlinedCard({ containerColor: c.surfaceContainerLow, elevation: 0, shape: { cornerRadius: 10 }, border: { width: 1, color: c.outlineVariant } },
    UI.Row({ modifier: M.padding(14, 12), verticalAlignment: 'center' }, [
      UI.Box({ width: 36, height: 36, contentAlignment: 'center', modifier: M.background(c.primary.copy({ alpha: 0.15 }), { cornerRadius: 8 }).border(1, c.primary.copy({ alpha: 0.3 }), { cornerRadius: 8 }) }, UI.Icon({ name: 'difference_outlined', size: 20, tint: c.primary })),
      UI.Spacer({ width: 12 }), UI.Column({ weight: 1, horizontalAlignment: 'start' }, [
        UI.Text({ text: '代码变更审查', style: 'bodyMedium', fontWeight: '600', color: c.onSurface }), UI.Spacer({ height: 2 }),
        UI.Text({ text: '查看工作区代码改动、行级对比与文件审查', style: 'bodySmall', fontSize: 11.5, color: c.onSurfaceVariant.copy({ alpha: 0.8 }) }),
      ]), UI.Spacer({ width: 8 }),
      UI.FilledTonalButton({ onClick: open, containerColor: c.surfaceContainerHigh, contentColor: c.onSurface, contentPadding: { horizontal: 12, vertical: 8 }, shape: { cornerRadius: 7 }, border: { width: 1, color: c.outlineVariant.copy({ alpha: 0.4 }) } }, UI.Row({ verticalAlignment: 'center' }, [
        UI.Text({ text: '查看变更', inheritStyle: true, fontWeight: '600', fontSize: 12, color: c.onSurface }), UI.Spacer({ width: 4 }), UI.Icon({ name: 'arrow_forward_rounded', size: 14, tint: c.onSurfaceVariant }),
      ])),
    ]));
}
