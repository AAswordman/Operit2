import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/RuntimeBootstrapScreen.dart';

void main() {
  testWidgets('Japanese locale maps native runtime states to Japanese', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('ja'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const RuntimeBootstrapScreen(
          state: 'preparingAssets',
          errorText: '本地运行时启动失败',
        ),
      ),
    );

    expect(find.text('端末内の実行ファイルを準備しています'), findsOneWidget);
    expect(find.text('本地运行时启动失败'), findsNothing);
  });
}
