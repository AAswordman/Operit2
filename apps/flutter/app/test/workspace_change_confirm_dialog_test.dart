import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/chat/components/WorkspaceChangeConfirmDialog.dart';

/// Opens the production dialog route with the requested file changes.
Future<void> _openDialog(
  WidgetTester tester, {
  required WorkspaceChangeConfirmMode mode,
  required List<WorkspaceFileChange> changes,
  required Future<void> Function() onConfirm,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) => TextButton(
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => WorkspaceChangeConfirmDialog(
                mode: mode,
                changes: changes,
                onConfirm: onConfirm,
              ),
            ),
            child: const Text('Open dialog'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('Open dialog'));
  await tester.pumpAndSettle();
}

/// Verifies intrinsic sizing, scrolling, semantics, and dialog actions.
void main() {
  for (final mode in WorkspaceChangeConfirmMode.values) {
    for (final size in <Size>[const Size(1200, 900), const Size(360, 640)]) {
      testWidgets('${mode.name} lays out and scrolls at $size', (tester) async {
        await tester.binding.setSurfaceSize(size);
        addTearDown(() => tester.binding.setSurfaceSize(null));
        final semantics = tester.ensureSemantics();
        addTearDown(semantics.dispose);
        final changes = List<WorkspaceFileChange>.generate(
          50,
          (index) => WorkspaceFileChange(
            path: 'lib/features/workspace/changed_file_$index.dart',
            changeType: ChangeType.modified,
            changedLines: index + 1,
          ),
        );
        var confirmations = 0;

        await _openDialog(
          tester,
          mode: mode,
          changes: changes,
          onConfirm: () async {
            confirmations++;
          },
        );

        expect(tester.takeException(), isNull);
        expect(find.text(changes.first.path), findsOneWidget);
        final listRect = tester.getRect(find.byType(ListView));
        expect(listRect.width, lessThanOrEqualTo(520));
        expect(listRect.height, lessThanOrEqualTo(420));
        expect(listRect.left, greaterThanOrEqualTo(0));
        expect(listRect.right, lessThanOrEqualTo(size.width));
        await tester.scrollUntilVisible(
          find.text(changes.last.path),
          300,
          scrollable: find.byType(Scrollable),
        );
        await tester.pumpAndSettle();
        expect(find.text(changes.last.path), findsOneWidget);
        expect(tester.takeException(), isNull);

        await tester.tap(find.byType(FilledButton));
        await tester.pumpAndSettle();
        expect(confirmations, 1);
        expect(find.byType(WorkspaceChangeConfirmDialog), findsNothing);
        expect(tester.takeException(), isNull);
      });
    }

    testWidgets('${mode.name} supports an empty change list and cancel', (
      tester,
    ) async {
      var confirmations = 0;
      await _openDialog(
        tester,
        mode: mode,
        changes: const <WorkspaceFileChange>[],
        onConfirm: () async {
          confirmations++;
        },
      );

      expect(tester.takeException(), isNull);
      expect(find.byType(ListView), findsNothing);
      await tester.tap(
        find.descendant(
          of: find.byType(WorkspaceChangeConfirmDialog),
          matching: find.byType(TextButton),
        ),
      );
      await tester.pumpAndSettle();
      expect(confirmations, 0);
      expect(find.byType(WorkspaceChangeConfirmDialog), findsNothing);
      expect(tester.takeException(), isNull);
    });
  }
}
