import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceFileBrowserContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspacePathBar.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies path editing keeps the full text height in narrow and scaled layouts.
void main() {
  const root = '/operitapp/outputs/2026-11-12_22-34-28-987';
  for (final width in <double>[360, 800]) {
    for (final scale in <double>[1, 1.5, 2]) {
      testWidgets('path editor fits text at width $width and scale $scale', (
        tester,
      ) async {
        await tester.binding.setSurfaceSize(Size(width, 700));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        final listed = <String>[];
        final snapshot = UserPreferencesManager.defaultThemePreferenceSnapshot;
        await tester.pumpWidget(
          OperitTheme(
            initialThemePreferenceSnapshot: snapshot,
            initialThemeIsReady: false,
            unconfiguredChildEnabled: true,
            hostInteractionHostsEnabled: false,
            child: MaterialApp(
              locale: const Locale('zh'),
              supportedLocales: AppLocalizations.supportedLocales,
              localizationsDelegates: AppLocalizations.localizationsDelegates,
              theme: buildOperitThemeData(
                ColorScheme.fromSeed(seedColor: Colors.blue),
                snapshot,
                fontFamily: 'Ahem',
                fontFamilyFallback: const <String>[],
              ),
              builder: (context, child) => MediaQuery(
                data: MediaQuery.of(
                  context,
                ).copyWith(textScaler: TextScaler.linear(scale)),
                child: child!,
              ),
              home: Scaffold(
                body: WorkspaceFileBrowserContent(
                  rootLabel: root,
                  rootRelativePath: '',
                  onListWorkspaceFiles: (path) async {
                    listed.add(path);
                    return const [];
                  },
                  onOpenFile: (_) async {},
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byTooltip('编辑'));
        await tester.pumpAndSettle();

        final editable = tester
            .state<EditableTextState>(find.byType(EditableText))
            .renderEditable;
        expect(
          editable.size.height,
          greaterThanOrEqualTo(editable.preferredLineHeight),
          reason: 'The input viewport must fit a complete line of path text.',
        );
        final inputRect = tester.getRect(find.byType(TextField));
        final textRect = editable.localToGlobal(Offset.zero) & editable.size;
        expect(textRect.top, greaterThanOrEqualTo(inputRect.top));
        expect(textRect.bottom, lessThanOrEqualTo(inputRect.bottom));

        final controller = tester
            .widget<TextField>(find.byType(TextField))
            .controller!;
        expect(controller.text, root);
        final caret = editable.getLocalRectForCaret(
          TextPosition(offset: root.length),
        );
        expect(caret.left, greaterThanOrEqualTo(0));
        expect(caret.right, lessThanOrEqualTo(editable.size.width));

        await tester.enterText(find.byType(TextField), '$root/nested');
        await tester.tap(find.byTooltip('确定'));
        await tester.pumpAndSettle();
        expect(listed, ['', 'nested']);
        expect(
          tester.widget<WorkspacePathBar>(find.byType(WorkspacePathBar)).path,
          '$root/nested',
        );
        expect(tester.takeException(), isNull);
      });
    }
  }
}
