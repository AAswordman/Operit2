import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceHomeContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceOverviewModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/terminal/WorkspaceTerminalSessions.dart';

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
          await tester.pumpWidget(fixture.app(brightness: brightness, scale: scale));
          await tester.pumpAndSettle();

          expect(find.text('查看变更'), findsOneWidget);
          expect(find.byType(Card), findsNWidgets(5));
          expect(find.text('代码变更审查'), findsOneWidget);
          expect(find.text('就绪'), findsNothing);
          expect(find.text('0 个终端'), findsNothing);
          expect(find.text('0 个浏览器'), findsNothing);
          expect(find.text('挂载新目录'), findsNothing);
          expect(find.text('新建终端会话'), findsOneWidget);
          expect(find.text('新建浏览器页面'), findsOneWidget);
          final terminalIcon = tester.widget<Icon>(find.byIcon(Icons.terminal_rounded).first);
          final context = tester.element(find.byType(WorkspaceHomeContent));
          expect(terminalIcon.color, Theme.of(context).colorScheme.onSurfaceVariant);
          expect(tester.takeException(), isNull);
        });
      }
    }
  }

  testWidgets('compact review and session creation preserve their callbacks', (
    tester,
  ) async {
    final fixture = _HomeFixture();
    addTearDown(fixture.dispose);
    await tester.pumpWidget(fixture.app());
    await tester.tap(find.text('查看变更'));
    await tester.tap(find.text('新建终端会话'));
    await tester.tap(find.text('新建浏览器页面'));
    expect(fixture.calls, <String>['review', 'terminal', 'browser']);
  });

  testWidgets('folder actions use the workspace menu', (tester) async {
    final fixture = _HomeFixture();
    addTearDown(fixture.dispose);
    await tester.pumpWidget(fixture.app());
    for (final (label, call) in <(String, String)>[
      ('项目目录', 'folder'),
      ('挂载新文件夹', 'mount'),
      ('切换工作区', 'choose'),
      ('解除绑定', 'unbind'),
    ]) {
      await tester.tap(find.text(_HomeFixture.name));
      await tester.pumpAndSettle();
      await tester.tap(find.text(label));
      await tester.pumpAndSettle();
      expect(fixture.calls.last, call);
    }
    expect(tester.takeException(), isNull);
  });

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
    expect(find.text('4 个运行中'), findsOneWidget);
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
  final terminals = ValueNotifier<int>(0);
  final browsers = ValueNotifier<int>(0);
  final sessions = ValueNotifier<List<WorkspaceTerminalSessionInfo>>(
    const <WorkspaceTerminalSessionInfo>[],
  );
  final calls = <String>[];

  /// Mounts the actual overview with a Material theme and no runtime requests.
  Widget app({Brightness brightness = Brightness.light, double scale = 1}) {
    return MaterialApp(
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xff386a54),
          brightness: brightness,
        ),
      ),
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(context).copyWith(textScaler: TextScaler.linear(scale)),
        child: child!,
      ),
      home: Scaffold(
        body: WorkspaceHomeContent(
          workspacePath: '/app/workspaces/project/with/a/long/directory/path',
          workspaceUsage: const WorkspaceOverviewUsage(
            workspaceName: name,
            conversationCount: 2,
            contributions: WorkspaceContributionsReady([]),
            mountedFolders: <WorkspaceMountedFolder>[
              WorkspaceMountedFolder(
                name: '项目目录',
                path: '/app/workspaces/project',
                relativePath: '.',
              ),
            ],
            mountedFoldersLoading: false,
            mountedFoldersError: null,
          ),
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
          onSelectTerminalSession: (session) => calls.add('select:${session.sessionId}'),
          onCloseTerminalSession: (session) => calls.add('close:${session.sessionId}'),
          onOpenDiff: () => calls.add('review'),
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
