import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/components/SettingsAssetTile.dart';
import 'package:operit2/ui/features/settings/components/SettingsControlStyles.dart';

/// Verifies that bubble image actions stay inside their tile and accept taps.
void main() {
  for (final language in ['zh', 'en']) {
    for (final width in [320.0, 400.0, 1024.0]) {
      for (final textScale in [1.0, 1.3]) {
        testWidgets(
          '$language bubble actions remain tappable at width $width and scale $textScale',
          (tester) async {
            await tester.binding.setSurfaceSize(Size(width, 1000));
            addTearDown(() => tester.binding.setSurfaceSize(null));
            final l10n = lookupAppLocalizations(Locale(language));
            final cleared = <bool>[];
            await tester.pumpWidget(
              MaterialApp(
                home: Scaffold(
                  body: MediaQuery(
                    data: MediaQueryData(
                      size: Size(width, 1000),
                      textScaler: TextScaler.linear(textScale),
                    ),
                    child: ListView(
                      padding: const EdgeInsets.all(32),
                      children: [
                        for (final isUser in [true, false])
                          _bubbleTile(l10n, isUser, () => cleared.add(isUser)),
                      ],
                    ),
                  ),
                ),
              ),
            );
            await tester.pumpAndSettle();
            expect(tester.takeException(), isNull);
            for (final isUser in [true, false]) {
              final tile = find.byKey(ValueKey(isUser));
              final clear = find.byTooltip(
                isUser
                    ? l10n.settingsAppearanceClearUserBubbleImage
                    : l10n.settingsAppearanceClearAiBubbleImage,
              );
              await tester.ensureVisible(clear);
              await tester.pumpAndSettle();
              final tileRect = tester.getRect(tile);
              final clearRect = tester.getRect(clear);
              expect(tileRect.contains(clearRect.topLeft), isTrue);
              expect(tileRect.contains(clearRect.bottomRight), isTrue);
              expect(clear.hitTestable(), findsOneWidget);
              final title = find.text(
                isUser
                    ? l10n.settingsAppearanceUserBubbleImage
                    : l10n.settingsAppearanceAiBubbleImage,
              );
              expect(tester.getSize(title).width, greaterThan(0));
              await tester.tap(clear);
              await tester.pump();
              expect(cleared.last, isUser);
            }
            expect(cleared, [true, false]);
            expect(tester.takeException(), isNull);
          },
        );
      }
    }
  }
}

/// Builds the same localized image actions used by the appearance panel.
Widget _bubbleTile(AppLocalizations l10n, bool isUser, VoidCallback onClear) {
  return SettingsAssetTile(
    key: ValueKey(isUser),
    icon: isUser ? Icons.person_outline : Icons.smart_toy_outlined,
    title: isUser
        ? l10n.settingsAppearanceUserBubbleImage
        : l10n.settingsAppearanceAiBubbleImage,
    subtitle: isUser ? 'user-bubble.png' : 'ai-bubble.png',
    actions: [
      FilledButton.tonalIcon(
        style: SettingsControlStyles.sectionTextButton(),
        onPressed: () {},
        icon: const Icon(Icons.image_outlined, size: 16),
        label: Text(
          isUser
              ? l10n.settingsAppearanceChooseUserBubbleImage
              : l10n.settingsAppearanceChooseAiBubbleImage,
        ),
      ),
      OutlinedButton.icon(
        style: SettingsControlStyles.sectionTextButton(),
        onPressed: () {},
        icon: const Icon(Icons.tune_outlined, size: 16),
        label: Text(
          isUser
              ? l10n.settingsAppearanceBubbleImageAdjustUser
              : l10n.settingsAppearanceBubbleImageAdjustAi,
        ),
      ),
      SettingsEntityIconButton(
        tooltip: isUser
            ? l10n.settingsAppearanceClearUserBubbleImage
            : l10n.settingsAppearanceClearAiBubbleImage,
        icon: Icons.layers_clear_outlined,
        onPressed: onClear,
      ),
    ],
    bottom: const SizedBox(height: 32),
  );
}
