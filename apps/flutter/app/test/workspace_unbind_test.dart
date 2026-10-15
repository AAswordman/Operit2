import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceHomeContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceOverviewModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceUnbindDialog.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

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
    home: Scaffold(body: child),
  ),
);

void main() {
  Future<void> mountHome(
    WidgetTester tester, {
    String? path,
    String? name,
    VoidCallback? onUnbind,
  }) async {
    final count = ValueNotifier<int>(0);
    addTearDown(count.dispose);
    await tester.pumpWidget(
      _app(
        WorkspaceHomeContent(
          workspacePath: path,
          workspaceUsage: WorkspaceOverviewUsage(
            workspaceName: name,
            conversationCount: 1,
            contributions: const WorkspaceContributionsReady([]),
            mountedFolders: const [],
            mountedFoldersLoading: false,
            mountedFoldersError: null,
          ),
          terminalSessionCountListenable: count,
          browserSessionCountListenable: count,
          onOpenFolder: (_) {},
          onAddFolder: () {},
          onCreateWorkspace: () {},
          onChooseExistingWorkspace: () {},
          onUnbindWorkspace: onUnbind ?? () {},
          onOpenTerminal: () {},
          onOpenTerminalSessions: () {},
          onOpenBrowserSessions: () {},
          onOpenBrowser: () {},
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> openDialog(
    WidgetTester tester,
    Future<void> Function() onUnbind,
  ) async {
    await tester.pumpWidget(
      _app(
        Builder(
          builder: (context) => TextButton(
            onPressed: () => showDialog<void>(
              context: context,
              barrierDismissible: false,
              useRootNavigator: false,
              builder: (_) =>
                  WorkspaceUnbindDialog(onUnbindWorkspace: onUnbind),
            ),
            child: const Text('打开确认'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('打开确认'));
    await tester.pumpAndSettle();
  }

  testWidgets('bound workspace exposes a working unbind action', (
    tester,
  ) async {
    var taps = 0;
    await mountHome(tester, path: '/workspace/project', onUnbind: () => taps++);
    expect(find.text('解除绑定'), findsOneWidget);
    expect(find.text('创建工作区'), findsNothing);
    expect(find.text('选择工作区'), findsNothing);
    await tester.tap(find.text('解除绑定'));
    expect(taps, 1);
  });

  testWidgets('workspace known by name also exposes unbind', (tester) async {
    await mountHome(tester, name: '项目工作区');
    expect(find.text('解除绑定'), findsOneWidget);
  });

  testWidgets('unbound workspace offers create and bind, not unbind', (
    tester,
  ) async {
    await mountHome(tester, path: ' ', name: ' ');
    expect(find.text('解除绑定'), findsNothing);
    expect(find.text('创建工作区'), findsOneWidget);
    expect(find.text('选择工作区'), findsOneWidget);
  });

  testWidgets('cancel keeps binding and makes no runtime call', (tester) async {
    var calls = 0;
    await openDialog(tester, () async => calls++);
    expect(find.textContaining('工作区和文件都会保留，不影响其他会话'), findsOneWidget);
    await tester.tap(find.text('取消'));
    await tester.pumpAndSettle();
    expect(calls, 0);
    expect(find.byType(WorkspaceUnbindDialog), findsNothing);
  });

  testWidgets('confirm detaches only the explicit chat and closes dialog', (
    tester,
  ) async {
    final bridge = _UnbindBridge();
    final viewModel = ChatViewModel(bridge: bridge);
    await openDialog(
      tester,
      () => viewModel.unbindChatFromWorkspace('bound-chat'),
    );
    await tester.tap(find.widgetWithText(FilledButton, '解除绑定'));
    await tester.pumpAndSettle();
    expect(bridge.calls, hasLength(1));
    expect(bridge.calls.single.methodName, 'unbindChatFromWorkspace');
    expect((bridge.calls.single.args as Map)['chatId'], 'bound-chat');
    expect(find.byType(WorkspaceUnbindDialog), findsNothing);
  });

  testWidgets('pending unbind disables duplicate submission and dismissal', (
    tester,
  ) async {
    final pending = Completer<void>();
    var calls = 0;
    await openDialog(tester, () {
      calls++;
      return pending.future;
    });
    await tester.tap(find.widgetWithText(FilledButton, '解除绑定'));
    await tester.pump();
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNull,
    );
    expect(
      tester
          .widget<TextButton>(find.widgetWithText(TextButton, '取消'))
          .onPressed,
      isNull,
    );
    await tester.tap(find.byType(FilledButton));
    await tester.tap(find.text('取消'));
    await Navigator.of(
      tester.element(find.byType(WorkspaceUnbindDialog)),
    ).maybePop();
    await tester.pump();
    expect(calls, 1);
    expect(find.byType(WorkspaceUnbindDialog), findsOneWidget);
    pending.complete();
    await tester.pumpAndSettle();
    expect(find.byType(WorkspaceUnbindDialog), findsNothing);
  });

  testWidgets('failed unbind stays open with error and can be retried', (
    tester,
  ) async {
    var calls = 0;
    await openDialog(tester, () async {
      calls++;
      if (calls == 1) {
        throw StateError('runtime unavailable');
      }
    });
    await tester.tap(find.widgetWithText(FilledButton, '解除绑定'));
    await tester.pumpAndSettle();
    expect(find.byType(WorkspaceUnbindDialog), findsOneWidget);
    expect(find.textContaining('解除绑定失败，请重试。'), findsOneWidget);
    expect(find.textContaining('runtime unavailable'), findsOneWidget);
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNotNull,
    );
    await tester.tap(find.widgetWithText(FilledButton, '解除绑定'));
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(find.byType(WorkspaceUnbindDialog), findsNothing);
  });
}

class _UnbindBridge extends OperitRuntimeBridge {
  final List<CoreCallRequest> calls = [];

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    if (request.methodName != 'unbindChatFromWorkspace') {
      throw StateError('Unexpected runtime call: ${request.methodName}');
    }
    return encodeCoreLink([0, null]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}
