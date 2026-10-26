import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/theme/ThemeAppearanceErrorScreen.dart';

/// Verifies that appearance recovery requires a user action and exposes operation errors.
void main() {
  testWidgets(
    'error screen offers explicit retry and reset without mutating appearance',
    (tester) async {
      var retries = 0;
      var resets = 0;
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          locale: const Locale('en'),
          home: ThemeAppearanceErrorScreen(
            error: StateError('Font asset is missing'),
            onRetry: () async {
              retries++;
            },
            onReset: () async {
              resets++;
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.textContaining('Font asset is missing'), findsOneWidget);
      expect(retries, 0);
      expect(resets, 0);
      await tester.tap(find.byKey(const ValueKey('appearance-retry')));
      await tester.pumpAndSettle();
      expect(retries, 1);
      expect(resets, 0);
      await tester.tap(find.byKey(const ValueKey('appearance-reset')));
      await tester.pumpAndSettle();
      expect(resets, 1);
    },
  );
  testWidgets('failed repair is visible and actions remain available', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        locale: const Locale('en'),
        home: ThemeAppearanceErrorScreen(
          error: StateError('Invalid palette'),
          onRetry: () async {},
          onReset: () async {
            throw StateError('Storage rejected reset');
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('appearance-reset')));
    await tester.pumpAndSettle();
    expect(find.textContaining('Storage rejected reset'), findsOneWidget);
    expect(
      tester
          .widget<OutlinedButton>(
            find.byKey(const ValueKey('appearance-retry')),
          )
          .onPressed,
      isNotNull,
    );
    expect(tester.takeException(), isNull);
  });
}
