import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/settings/about/AboutOperitScreen.dart';

/// Exercises the rendered About page and its license dialog.
void main() {
  testWidgets('about page displays project details and licenses', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: AboutOperitScreen())),
    );

    expect(find.text('Operit2'), findsWidgets);
    expect(find.text('版本 2.0.0+5'), findsOneWidget);
    expect(find.text('项目源码'), findsOneWidget);

    await tester.tap(find.text('开源许可证'));
    await tester.pumpAndSettle();

    expect(find.text('flutter_math_fork'), findsOneWidget);
    expect(find.text('AGPL-3.0'), findsWidgets);
  });
}
