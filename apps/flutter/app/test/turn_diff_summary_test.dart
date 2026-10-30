import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceInlineDiff.dart';

/// Keeps coverage for the inline diff renderer still used by tool results.
void main() {
  test('computes intra-line segments for paired removed/added rows', () {
    final document = WorkspaceDiffDocument.parse(
      '@@ -10,3 +10,3 @@\n'
      ' context\n'
      '-final count = 1;\n'
      '+final count = 42;\n',
    );
    final removed = document.hunks.single.rows[1];
    final added = document.hunks.single.rows[2];
    expect(removed.kind, InlineDiffKind.remove);
    expect(added.kind, InlineDiffKind.add);
    expect(
      removed.text.substring(
        removed.segments.single.start,
        removed.segments.single.end,
      ),
      '1',
    );
    expect(
      added.text.substring(
        added.segments.single.start,
        added.segments.single.end,
      ),
      '42',
    );
  });

  testWidgets('folds unchanged regions between hunks', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 800,
            height: 400,
            child: WorkspaceInlineDiff(
              diff:
                  '@@ -1,2 +1,2 @@\n'
                  ' a\n'
                  '-x\n'
                  '+y\n'
                  '@@ -30,2 +30,2 @@\n'
                  ' b\n'
                  '-p\n'
                  '+q\n',
            ),
          ),
        ),
      ),
    );
    expect(find.text('27 行未更改'), findsOneWidget);
    expect(find.text('x'), findsOneWidget);
    expect(find.text('q'), findsOneWidget);
  });
}
