import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceHomeContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceOverviewModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/terminal/WorkspaceTerminalSessions.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';

final _reviewFixture =
    jsonDecode(File('test/fixtures/change_review_home.json').readAsStringSync())
        as Map<String, dynamic>;

/// Exercises the native overview layout and its existing workspace callbacks.
void main() {
  for (final brightness in Brightness.values) {
    for (final width in <double>[280, 400, 900]) {
      for (final scale in <double>[1, 1.5]) {
        testWidgets('overview fits $width / $brightness / text scale $scale', (
          tester,
        ) async {
          final fixture = _HomeFixture();
          addTearDown(fixture.dispose);
          tester.view.physicalSize = Size(width, 1000);
          tester.view.devicePixelRatio = 1;
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetDevicePixelRatio);
          await tester.pumpWidget(
            fixture.app(brightness: brightness, scale: scale),
          );
          await tester.pumpAndSettle();

          expect(find.text('查看变更'), findsOneWidget);
          expect(find.byType(Card), findsNWidgets(5));
          expect(find.text('代码变更审查'), findsOneWidget);
          expect(find.text('就绪'), findsNothing);
          expect(find.text('0 个终端'), findsNothing);
          expect(find.text('0 个浏览器'), findsNothing);
          expect(find.text('挂载新目录'), findsNothing);
          expect(find.text('新建终端会话'), findsNothing);
          expect(find.text('新建浏览器页面'), findsNothing);
          expect(find.byTooltip('新建终端'), findsOneWidget);
          expect(find.byTooltip('新建浏览器页面'), findsOneWidget);
          expect(find.byIcon(Icons.add_rounded), findsNWidgets(3));
          expect(find.text('项目目录'), findsOneWidget);
          expect(find.byTooltip('挂载文件夹'), findsOneWidget);
          expect(find.text('终端工件'), findsOneWidget);
          expect(find.text('浏览器工件'), findsOneWidget);
          expect(find.text('终端工件 (PTY)'), findsNothing);
          expect(find.text('浏览器工件 (WebView)'), findsNothing);
          final terminalIcon = tester.widget<Icon>(
            find.byIcon(Icons.terminal_rounded).first,
          );
          final context = tester.element(find.byType(WorkspaceHomeContent));
          expect(
            terminalIcon.color,
            Theme.of(context).colorScheme.onSurfaceVariant,
          );
          expect(tester.takeException(), isNull);
        });
      }
    }
  }

  for (final brightness in Brightness.values) {
    for (final size in <Size>[
      Size(280, 640),
      Size(400, 800),
      Size(900, 700),
      Size(640, 240),
    ]) {
      for (final scale in <double>[1, 1.5, 2]) {
        testWidgets('unbound home fits $size / $brightness / scale $scale', (
          tester,
        ) async {
          final fixture = _HomeFixture();
          addTearDown(fixture.dispose);
          tester.view.physicalSize = size;
          tester.view.devicePixelRatio = 1;
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetDevicePixelRatio);
          await tester.pumpWidget(
            fixture.app(brightness: brightness, scale: scale, bound: false),
          );
          await tester.pumpAndSettle();

          expect(find.text('为当前对话添加工作区'), findsOneWidget);
          expect(find.byType(Card), findsNothing);
          expect(find.byType(MenuAnchor), findsNothing);
          expect(find.text('查看变更'), findsNothing);
          expect(find.text('终端工件'), findsNothing);
          expect(find.text('浏览器工件'), findsNothing);
          expect(find.text('当前对话未绑定工作区'), findsNothing);
          for (final (label, call) in <(String, String)>[
            ('创建工作区', 'create'),
            ('选择工作区', 'choose'),
          ]) {
            final action = find.ancestor(
              of: find.text(label),
              matching: find.byWidgetPredicate(
                (widget) => widget is ButtonStyleButton,
              ),
            );
            await tester.ensureVisible(action);
            await tester.pumpAndSettle();
            expect(tester.getSize(action).height, greaterThanOrEqualTo(48));
            await tester.tap(action);
            expect(fixture.calls.last, call);
          }
          expect(tester.takeException(), isNull);
        });
      }
    }
  }

  testWidgets('binding swaps the setup screen for the workspace overview', (
    tester,
  ) async {
    final fixture = _HomeFixture();
    addTearDown(fixture.dispose);
    await tester.pumpWidget(fixture.app(bound: false));
    expect(find.text('创建工作区'), findsOneWidget);
    await tester.pumpWidget(fixture.app());
    expect(find.text('创建工作区'), findsNothing);
    expect(find.text(_HomeFixture.name), findsOneWidget);
    expect(find.text('终端工件'), findsOneWidget);
    await tester.pumpWidget(fixture.app(bound: false));
    expect(find.text('创建工作区'), findsOneWidget);
    expect(find.text(_HomeFixture.name), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('compact review and session creation preserve their callbacks', (
    tester,
  ) async {
    final fixture = _HomeFixture();
    addTearDown(fixture.dispose);
    await tester.pumpWidget(fixture.app());
    await tester.tap(find.text('查看变更'));
    await tester.tap(find.byTooltip('新建终端'));
    await tester.tap(find.byTooltip('新建浏览器页面'));
    expect(fixture.calls, <String>['review', 'terminal', 'browser']);
  });

  testWidgets('folders and their add action live only in the lower card', (
    tester,
  ) async {
    final fixture = _HomeFixture();
    addTearDown(fixture.dispose);
    await tester.pumpWidget(fixture.app());
    final folder = find.text('项目目录');
    final addFolder = find.byTooltip('挂载文件夹');
    expect(folder, findsOneWidget);
    expect(find.text('/app/workspaces/project'), findsOneWidget);
    expect(find.text('切换工作区'), findsNothing);
    expect(find.text('挂载新文件夹'), findsNothing);
    expect(
      tester.getCenter(addFolder).dx,
      greaterThan(tester.getCenter(find.text('文件夹')).dx),
    );
    expect(
      tester.getCenter(addFolder).dy,
      greaterThan(tester.getCenter(find.text('浏览器工件')).dy),
    );
    await tester.ensureVisible(folder);
    await tester.tap(folder);
    await tester.ensureVisible(addFolder);
    await tester.tap(addFolder);
    expect(fixture.calls, <String>['folder', 'mount']);
    expect(tester.takeException(), isNull);
  });

  testWidgets('workspace menu contains only a native unbind item', (
    tester,
  ) async {
    final fixture = _HomeFixture();
    addTearDown(fixture.dispose);
    await tester.pumpWidget(fixture.app());
    await tester.tap(find.text(_HomeFixture.name));
    await tester.pumpAndSettle();
    expect(find.byType(MenuItemButton), findsOneWidget);
    expect(find.text('切换工作区'), findsNothing);
    expect(find.text('挂载新文件夹'), findsNothing);
    expect(find.text('项目目录'), findsOneWidget);
    final unbind = find.widgetWithText(MenuItemButton, '解除绑定');
    expect(unbind, findsOneWidget);
    expect(
      tester.getCenter(find.byIcon(Icons.link_off_rounded)).dx,
      lessThan(tester.getCenter(find.text('解除绑定')).dx),
    );
    await tester.tap(unbind);
    await tester.pumpAndSettle();
    expect(fixture.calls, <String>['unbind']);
    expect(find.byType(MenuItemButton), findsNothing);
    expect(tester.takeException(), isNull);
  });

  for (final state in <String>['loading', 'error', 'empty']) {
    testWidgets('folder $state state keeps mounting available', (tester) async {
      final fixture = _HomeFixture();
      addTearDown(fixture.dispose);
      await tester.pumpWidget(
        fixture.app(
          folders: const <WorkspaceMountedFolder>[],
          foldersLoading: state == 'loading',
          foldersError: state == 'error' ? 'Folder request failed' : null,
        ),
      );
      expect(
        find.text(switch (state) {
          'loading' => '正在加载文件夹…',
          'error' => 'Folder request failed',
          'empty' => '暂无挂载的文件夹',
          _ => throw StateError('Unknown folder test state'),
        }),
        findsOneWidget,
      );
      final addFolder = find.byTooltip('挂载文件夹');
      await tester.ensureVisible(addFolder);
      await tester.tap(addFolder);
      expect(fixture.calls, <String>['mount']);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('live terminal rows retain select, close and view-all actions', (
    tester,
  ) async {
    final fixture = _HomeFixture();
    addTearDown(fixture.dispose);
    await tester.pumpWidget(fixture.app());
    fixture.terminals.value = 4;
    fixture.sessions.value = <WorkspaceTerminalSessionInfo>[
      for (var index = 0; index < 4; index++)
        WorkspaceTerminalSessionInfo(
          sessionId: '$index',
          sessionName: 'session-$index',
          platform: 'host',
          terminal: 'shell',
          terminalType: 'shell',
          sessionKind: 'pty',
          workingDir: '/app/workspaces/project',
          commandRunning: false,
        ),
    ];
    await tester.pumpAndSettle();
    expect(find.text('4 个运行中'), findsNothing);
    expect(find.byTooltip('新建终端'), findsOneWidget);
    expect(find.text('session-2'), findsOneWidget);
    expect(find.text('session-3'), findsNothing);
    await tester.tap(find.text('切换').first);
    await tester.tap(find.byIcon(Icons.close_rounded).first);
    expect(find.text('4 个终端'), findsNothing);
    expect(find.text('管理全部'), findsOneWidget);
    await tester.tap(find.text('管理全部'));
    expect(fixture.calls, <String>['select:0', 'close:0', 'terminals']);

    fixture.terminals.value = 0;
    fixture.sessions.value = const <WorkspaceTerminalSessionInfo>[];
    await tester.pumpAndSettle();
    expect(find.text('暂无运行中的终端会话'), findsOneWidget);
    expect(find.text('管理全部'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('session management stays reachable before row details arrive', (
    tester,
  ) async {
    final fixture = _HomeFixture();
    addTearDown(fixture.dispose);
    await tester.pumpWidget(fixture.app());
    fixture.terminals.value = 2;
    fixture.browsers.value = 1;
    await tester.pumpAndSettle();
    expect(find.text('2 个运行中'), findsNothing);
    expect(find.text('1 个打开'), findsNothing);
    expect(find.byIcon(Icons.add_rounded), findsNWidgets(3));
    expect(find.text('暂无运行中的终端会话'), findsNothing);
    expect(find.text('暂无打开的浏览器页面'), findsNothing);
    expect(find.text('管理全部'), findsNWidgets(2));
    await tester.tap(find.text('管理全部').first);
    await tester.tap(find.text('管理全部').last);
    expect(fixture.calls, <String>['terminals', 'browsers']);
    expect(tester.takeException(), isNull);
  });
}

class _HomeFixture {
  static const name = 'Operit2Bug / a deliberately long workspace title';
  static const folder = WorkspaceMountedFolder(
    name: '项目目录',
    path: '/app/workspaces/project',
    relativePath: '.',
  );
  final terminals = ValueNotifier<int>(0);
  final browsers = ValueNotifier<int>(0);
  final sessions = ValueNotifier<List<WorkspaceTerminalSessionInfo>>(
    const <WorkspaceTerminalSessionInfo>[],
  );
  final calls = <String>[];

  /// Mounts the actual overview with a Material theme and no runtime requests.
  Widget app({
    Brightness brightness = Brightness.light,
    double scale = 1,
    bool bound = true,
    List<WorkspaceMountedFolder> folders = const <WorkspaceMountedFolder>[
      folder,
    ],
    bool foldersLoading = false,
    String? foldersError,
  }) {
    return MaterialApp(
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xff386a54),
          brightness: brightness,
        ),
      ),
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(
          context,
        ).copyWith(textScaler: TextScaler.linear(scale)),
        child: child!,
      ),
      home: Scaffold(
        body: WorkspaceHomeContent(
          workspacePath: bound
              ? '/app/workspaces/project/with/a/long/directory/path'
              : null,
          workspaceUsage: bound
              ? WorkspaceOverviewUsage(
                  workspaceName: name,
                  conversationCount: 2,
                  contributions: const WorkspaceContributionsReady([]),
                  mountedFolders: folders,
                  mountedFoldersLoading: foldersLoading,
                  mountedFoldersError: foldersError,
                )
              : WorkspaceOverviewUsage.empty,
          terminalSessionCountListenable: terminals,
          browserSessionCountListenable: browsers,
          terminalSessionsListenable: sessions,
          onOpenFolder: (_) => calls.add('folder'),
          onAddFolder: () => calls.add('mount'),
          onCreateWorkspace: () => calls.add('create'),
          onChooseExistingWorkspace: () => calls.add('choose'),
          onUnbindWorkspace: () => calls.add('unbind'),
          onOpenTerminal: () => calls.add('terminal'),
          onOpenTerminalSessions: () => calls.add('terminals'),
          onOpenBrowserSessions: () => calls.add('browsers'),
          onOpenBrowser: () => calls.add('browser'),
          onSelectTerminalSession: (session) =>
              calls.add('select:${session.sessionId}'),
          onCloseTerminalSession: (session) =>
              calls.add('close:${session.sessionId}'),
          pluginContent: Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: buildComposeDslLayoutForTest(
              node: _reviewFixture,
              hostContext: ComposeDslWebViewHostContext(
                packageName: 'com.operit.change_review',
                routeInstanceId: 'home-test',
                executionContextKey: 'home-test',
                dispatchAction: (_, [payload]) async {
                  calls.add('review');
                  return null;
                },
                runtimeOptionsProvider: () => {},
              ),
              splitMarkdownContent: (_) async => [],
              onTextInput: (_, _) async => null,
            ),
          ),
        ),
      ),
    );
  }

  /// Releases only the listenables owned by this fixture.
  void dispose() {
    terminals.dispose();
    browsers.dispose();
    sessions.dispose();
  }
}
