import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

/// Verifies focus traversal across retained and startup MaterialApp roots.
void main() {
  testWidgets('independent theme roots have a valid outer traversal order', (
    tester,
  ) async {
    await tester.pumpWidget(
      FocusTraversalGroup(
        policy: ReadingOrderTraversalPolicy(),
        child: ThemeRenderStack(
          children: <Widget>[
            _application('Retained application'),
            _application('Startup status'),
          ],
        ),
      ),
    );
    await tester.pumpAndSettle();
    final context = tester.element(find.text('Startup status'));
    final policy = FocusTraversalGroup.of(context);
    final applicationGroup = FocusManager.instance.rootScope.descendants
        .firstWhere(
          (node) =>
              node.debugLabel == 'FocusTraversalGroup' &&
              node.context!.findAncestorWidgetOfExactType<MaterialApp>() !=
                  null &&
              node.context!.getInheritedWidgetOfExactType<Directionality>() ==
                  null,
        );
    final first = policy.findFirstFocus(
      applicationGroup,
      ignoreCurrentFocus: true,
    );
    expect(first, isNotNull);
    expect(first!.context, isNotNull);
    for (var index = 0; index < 6; index++) {
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();
      expect(tester.takeException(), isNull);
    }
  });
}

/// Builds a localized application whose default traversal group is above Directionality.
Widget _application(String label) {
  return MaterialApp(
    home: Scaffold(
      body: Center(
        child: TextButton(onPressed: () {}, child: Text(label)),
      ),
    ),
  );
}
