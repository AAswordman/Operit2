import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/common/components/OperitDialog.dart';
import 'package:operit2/ui/features/announcement/RemoteAnnouncementHost.dart';
import 'package:operit2/ui/features/chat/components/workspace/terminal/WorkspaceTerminalLaunch.dart';
import 'package:operit_ui/operit_ui.dart';

ThemeData _theme() => buildOperitThemeData(
  ColorScheme.fromSeed(seedColor: Colors.blue, brightness: Brightness.dark),
  ThemePreferenceDefaults.snapshot,
  fontFamily: 'sans-serif',
  fontFamilyFallback: const <String>[],
);

final _longText = List.filled(
  20,
  'Cannot resolve terminal working directory for arm64/shell at '
  '/operitapp/workspaces/new: No such file or directory (POSIX error 2).',
).join('\n\n');

Finder _surface() => find
    .descendant(of: find.byType(Dialog), matching: find.byType(Material))
    .first;

void _expectCompactSurface(WidgetTester tester, Size viewport) {
  final surface = tester.getRect(_surface());
  expect(surface.width, lessThanOrEqualTo(560));
  expect(surface.left, greaterThanOrEqualTo(24));
  expect(surface.right, lessThanOrEqualTo(viewport.width - 24));
  expect(surface.top, greaterThanOrEqualTo(24));
  expect(surface.bottom, lessThanOrEqualTo(viewport.height - 24));
  expect(surface.center.dx, closeTo(viewport.width / 2, 0.1));
  expect(tester.takeException(), isNull);
}

void main() {
  for (final viewport in <Size>[
    const Size(1440, 900),
    const Size(390, 640),
    const Size(800, 300),
  ]) {
    testWidgets('text alerts stay compact at $viewport', (tester) async {
      await tester.binding.setSurfaceSize(viewport);
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          theme: _theme(),
          home: Scaffold(
            body: AlertDialog(
              title: const Text('Notice'),
              scrollable: true,
              content: SelectableText(_longText),
              actions: <Widget>[
                TextButton(onPressed: () {}, child: const Text('Close')),
              ],
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      _expectCompactSurface(tester, viewport);
      expect(find.text('Close').hitTestable(), findsOneWidget);
      await tester.drag(
        find.byType(SingleChildScrollView),
        const Offset(0, -300),
      );
      await tester.pumpAndSettle();
      final scrollable = tester.state<ScrollableState>(
        find.byType(Scrollable).first,
      );
      expect(scrollable.position.pixels, greaterThan(0));
      expect(find.text('Close').hitTestable(), findsOneWidget);
      expect(tester.takeException(), isNull);
    });

    testWidgets('announcements scroll with confirmation visible at $viewport', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(viewport);
      addTearDown(() => tester.binding.setSurfaceSize(null));
      var acknowledged = false;
      await tester.pumpWidget(
        MaterialApp(
          theme: _theme(),
          home: Scaffold(
            body: Builder(
              builder: (context) => TextButton(
                onPressed: () => showDialog<void>(
                  context: context,
                  barrierDismissible: false,
                  builder: (context) => RemoteAnnouncementDialog(
                    announcement: RemoteAnnouncementDisplay(
                      id: 'layout-test',
                      version: 1,
                      title: 'Announcement',
                      body: _longText,
                      acknowledgeText: 'Confirm',
                      countdownSec: 2,
                    ),
                    onAcknowledge: () async => acknowledged = true,
                  ),
                ),
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Open'));
      await tester.pumpAndSettle();

      _expectCompactSurface(tester, viewport);
      final pending = find.widgetWithText(TextButton, 'Confirm (2s)');
      expect(tester.widget<TextButton>(pending).onPressed, isNull);
      await tester.tapAt(const Offset(4, 4));
      await tester.pumpAndSettle();
      expect(find.byType(RemoteAnnouncementDialog), findsOneWidget);
      await tester.drag(
        find.byType(SingleChildScrollView),
        const Offset(0, -400),
      );
      await tester.pump(const Duration(seconds: 2));
      await tester.pumpAndSettle();
      expect(find.text('Confirm').hitTestable(), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.tap(find.text('Confirm'));
      await tester.pumpAndSettle();
      expect(acknowledged, isTrue);
      expect(find.byType(RemoteAnnouncementDialog), findsNothing);
    });
  }

  testWidgets(
    'terminal errors fit a phone with large text and keyboard insets',
    (tester) async {
      const viewport = Size(390, 844);
      await tester.binding.setSurfaceSize(viewport);
      addTearDown(() => tester.binding.setSurfaceSize(null));
      addTearDown(tester.view.resetViewInsets);
      tester.view.viewInsets = FakeViewPadding(
        bottom: 280 * tester.view.devicePixelRatio,
      );
      await tester.pumpWidget(
        MaterialApp(
          theme: _theme(),
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: TextScaler.linear(2)),
            child: child!,
          ),
          home: Scaffold(
            body: Builder(
              builder: (context) => TextButton(
                onPressed: () => launchWorkspaceTerminal(
                  context: context,
                  terminal: 'arm64',
                  terminalType: 'shell',
                  launch: () async => throw StateError(_longText),
                ),
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Open'));
      await tester.pumpAndSettle();

      _expectCompactSurface(tester, viewport);
      expect(
        tester.getRect(_surface()).bottom,
        lessThanOrEqualTo(844 - 280 - 24),
      );
      expect(find.text('关闭').hitTestable(), findsOneWidget);
      await tester.drag(
        find.byType(SingleChildScrollView),
        const Offset(0, -400),
      );
      await tester.pumpAndSettle();
      expect(find.text('关闭').hitTestable(), findsOneWidget);
      await tester.tap(find.text('关闭'));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('custom dialog widths can exceed the shared text dialog limit', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1440, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      MaterialApp(
        theme: _theme(),
        home: const Scaffold(
          body: OperitDialogScaffold(
            title: 'Editor',
            maxWidth: 800,
            child: SizedBox(height: 200),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(tester.getSize(_surface()).width, 800);
    expect(tester.takeException(), isNull);
  });
}
