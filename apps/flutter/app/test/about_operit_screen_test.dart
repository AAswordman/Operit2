import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/settings/about/AboutOperitScreen.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Exercises the rendered About page and its license dialog.
void main() {
  testWidgets('about page displays project details and licenses', (
    tester,
  ) async {
    tester.binding.platformDispatcher.localesTestValue = const <Locale>[
      Locale('ja'),
    ];
    addTearDown(tester.binding.platformDispatcher.clearLocalesTestValue);
    await tester.pumpWidget(
      const OperitTheme(
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: Scaffold(body: AboutOperitScreen()),
      ),
    );

    expect(find.text('Operit2'), findsWidgets);
    expect(find.text('バージョン 2.0.0+5'), findsOneWidget);
    expect(find.text('ソースコード'), findsOneWidget);
    expect(find.text('版本 2.0.0+5'), findsNothing);
    expect(find.text('项目源码'), findsNothing);

    await tester.tap(find.text('オープンソースライセンス'));
    await tester.pumpAndSettle();

    expect(find.text('flutter_math_fork'), findsOneWidget);
    expect(find.text('AGPL-3.0'), findsWidgets);
  });
}
