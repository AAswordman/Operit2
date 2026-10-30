import type { ComposeDslContext, ComposeNode } from '../../../../../types/compose-dsl';

/** Offers a single, quiet entry to workspace changes. */
export default function Home(ctx: ComposeDslContext): ComposeNode {
  const { UI, Modifier: M } = ctx, c = ctx.MaterialTheme.colorScheme;
  const [locale] = ctx.useState('locale', 'zh');
  const en = locale.startsWith('en');
  function open(): ToolPkg.JsonValue {
    return { type: 'toolpkg.ui.open', placement: 'workspace', routeId: 'toolpkg:com.operit.change_review:ui:workspace', instanceKey: 'change-review', input: {} };
  }
  return UI.InkWell({ onClick: open, shape: { cornerRadius: 8 } },
    UI.Row({ modifier: M.padding(12, 16), verticalAlignment: 'center' }, [
      UI.Icon({ name: 'difference_outlined', size: 18, tint: c.onSurfaceVariant }),
      UI.Spacer({ width: 10 }),
      UI.Column({ weight: 1, horizontalAlignment: 'start' }, [
        UI.Text({ text: en ? 'Code changes' : '代码变更', style: 'bodyMedium', fontWeight: '500', color: c.onSurface }),
        UI.Spacer({ height: 4 }),
        UI.Text({ text: en ? 'Review file changes' : '查看文件改动与对比', style: 'bodySmall', color: c.onSurfaceVariant }),
      ]),
      UI.Spacer({ width: 12 }),
      UI.Icon({ name: 'chevron_right_rounded', size: 16, tint: c.onSurfaceVariant }),
    ]));
}
