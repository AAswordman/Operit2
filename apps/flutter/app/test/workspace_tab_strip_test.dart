import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabStrip.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

Widget _app(Widget child) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: MaterialApp(
    locale: const Locale('zh'),
    supportedLocales: AppLocalizations.supportedLocales,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    home: Scaffold(
      body: Align(alignment: Alignment.topLeft, child: child),
    ),
  ),
);

List<WorkspaceTab> _tabs() => [
  const WorkspaceTab(
    kind: WorkspaceTabKind.home,
    title: 'Home',
    icon: Icons.home,
    closable: false,
  ),
  const WorkspaceTab(
    kind: WorkspaceTabKind.filePreview,
    title: 'a.txt',
    icon: Icons.description,
  ),
  const WorkspaceTab(
    kind: WorkspaceTabKind.terminal,
    title: 'bash',
    icon: Icons.terminal,
  ),
  const WorkspaceTab(
    kind: WorkspaceTabKind.browser,
    title: 'Web',
    icon: Icons.public,
  ),
];

Future<void> _rightClick(WidgetTester tester, String title) async {
  await tester.tap(
    find.text(title),
    buttons: kSecondaryMouseButton,
    kind: PointerDeviceKind.mouse,
  );
  await tester.pumpAndSettle();
}

void main() {
  for (final width in [390.0, 1024.0]) {
    for (final entry in <String, List<int>>{
      '关闭': [2],
      '关闭其他': [1, 3],
      '关闭右侧': [3],
      '关闭全部': [1, 2, 3],
    }.entries) {
      testWidgets('tab context menu ${entry.key} at width $width', (
        tester,
      ) async {
        tester.view.devicePixelRatio = 1;
        tester.view.physicalSize = Size(width, 600);
        addTearDown(tester.view.resetDevicePixelRatio);
        addTearDown(tester.view.resetPhysicalSize);
        final tabs = _tabs();
        List<WorkspaceTab>? targets;
        int? selected;
        await tester.pumpWidget(
          _app(
            WorkspaceTabStrip(
              tabs: tabs,
              selectedIndex: 1,
              onSelected: (index) => selected = index,
              onClosed: (_) => fail('Menu bypassed the batch close handler'),
              onCloseTabs: (value) async => targets = value,
            ),
          ),
        );
        await tester.pumpAndSettle();
        await _rightClick(tester, 'bash');
        expect(find.byType(PopupMenuItem<int>), findsNWidgets(4));
        expect(selected, isNull);
        await tester.tap(find.text(entry.key));
        await tester.pumpAndSettle();
        expect(targets, entry.value.map((index) => tabs[index]).toList());
        expect(targets, isNot(contains(tabs.first)));
        expect(tester.takeException(), isNull);
      });
    }
  }

  testWidgets('home and empty close groups have disabled menu actions', (
    tester,
  ) async {
    await tester.pumpWidget(
      _app(
        WorkspaceTabStrip(
          tabs: [_tabs().first],
          selectedIndex: 0,
          onSelected: (_) {},
          onClosed: (_) => fail('Home cannot be closed'),
          onCloseTabs: (_) async => fail('No closable tabs'),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await _rightClick(tester, 'Home');
    expect(
      tester
          .widgetList<PopupMenuItem<int>>(find.byType(PopupMenuItem<int>))
          .every((item) => !item.enabled),
      isTrue,
    );
  });

  testWidgets('menu keeps tab identities when tabs change while it is open', (
    tester,
  ) async {
    final tabs = _tabs();
    final expected = [tabs[1], tabs[3]];
    List<WorkspaceTab>? targets;
    await tester.pumpWidget(
      _app(
        WorkspaceTabStrip(
          tabs: tabs,
          selectedIndex: 1,
          onSelected: (_) {},
          onClosed: (_) {},
          onCloseTabs: (value) async => targets = value,
        ),
      ),
    );
    await tester.pumpAndSettle();
    await _rightClick(tester, 'bash');
    tabs.removeAt(1);
    tabs.add(
      const WorkspaceTab(
        kind: WorkspaceTabKind.filePreview,
        title: 'new.txt',
        icon: Icons.description,
      ),
    );
    await tester.tap(find.text('关闭其他'));
    await tester.pumpAndSettle();
    expect(targets, expected);
  });

  for (final width in [390.0, 1024.0]) {
    testWidgets('long file and terminal titles stay bounded at $width', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = Size(width, 600);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final tabs = [
        _tabs().first,
        for (final kind in [
          WorkspaceTabKind.filePreview,
          WorkspaceTabKind.terminal,
        ])
          WorkspaceTab(
            kind: kind,
            title: '${kind.name}_${'very_long_name_' * 40}',
            icon: Icons.description,
          ),
      ];
      await tester.pumpWidget(
        _app(
          SizedBox(
            width: 300,
            child: WorkspaceTabStrip(
              tabs: tabs,
              selectedIndex: 0,
              onSelected: (_) {},
              onClosed: (_) {},
              onCloseTabs: (_) async {},
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      for (final tab in tabs.skip(1)) {
        final tooltip = find.byTooltip(tab.title);
        expect(tooltip, findsOneWidget);
        expect(tester.getSize(tooltip).width, lessThanOrEqualTo(180));
        final text = tester.widget<Text>(find.text(tab.title));
        expect(text.overflow, TextOverflow.ellipsis);
        expect(text.maxLines, 1);
      }
      final scrollable = tester.state<ScrollableState>(
        find.descendant(
          of: find.byType(WorkspaceTabStrip),
          matching: find.byType(Scrollable),
        ),
      );
      expect(scrollable.position.maxScrollExtent, lessThan(260));
      expect(tester.takeException(), isNull);
    });
  }
}
