import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/application/PluginHotReload.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/contributions/ChatSidebarTabHost.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/features/chat/components/NewChatIntro.dart';
import 'package:operit2/ui/main/components/CollapsedDrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerContent.dart';
import 'package:operit2/ui/main/components/NavigationDrawerAppearance.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';

const _tabPreference = 'chat_sidebar_tab_v1';
const _preferenceFile = 'user_preferences.preferences.json';
const _workspaceKey = ValueKey('native-workspace-content');

/// Covers real sidebar registrations, embedded routes, and validated host activation.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(ClientLogger.initialize);
  tearDown(() => newChatIntroArmed.value = false);

  test('tab identity namespaces owners without delimiter collisions', () {
    expect(
      chatSidebarTabIdentity('owner.first', 'history'),
      isNot(chatSidebarTabIdentity('owner.second', 'history')),
    );
    expect(
      chatSidebarTabIdentity('a::b', 'c'),
      isNot(chatSidebarTabIdentity('a', 'b::c')),
    );
    expect(
      chatSidebarTabIdentity('owner.first', 'history'),
      jsonEncode(['owner.first', 'history']),
    );
  });

  test('summary exposes native folder metadata but no plugin role fields', () {
    final summary = chatSidebarSummary(_chat('chat-a'));
    expect(summary, {
      'id': 'chat-a',
      'title': 'Title chat-a',
      'updatedAt': '2026-10-08T12:00:00Z',
      'displayOrder': 3,
      'workspaceId': 'workspace-a',
      'workspaceName': 'Workspace A',
      'locked': true,
      'pinned': true,
      'group': 'legacy-native-folder',
    });
    expect(summary.keys.toSet(), {
      'id',
      'title',
      'updatedAt',
      'displayOrder',
      'workspaceId',
      'workspaceName',
      'locked',
      'pinned',
      'group',
    });
  });

  test('only exact well-shaped chat activation results are accepted', () {
    expect(chatSidebarActivation(null), isNull);
    expect(chatSidebarActivation({'preview': true}), isNull);
    expect(
      chatSidebarActivation({'type': 'toolpkg.presentation.complete'}),
      isNull,
    );
    expect(
      chatSidebarActivation({
        'type': 'toolpkg.chat.activate',
        'chatId': 'chat-a',
      }),
      'chat-a',
    );
    for (final malformed in <Object?>[
      {'type': 'toolpkg.chat.activate'},
      {'type': 'toolpkg.chat.activate', 'chatId': ''},
      {'type': 'toolpkg.chat.activate', 'chatId': ' '},
      {'type': 'toolpkg.chat.activate', 'chatId': 42},
      {'type': 'toolpkg.chat.activate', 'chatId': 'chat-a', 'selection': {}},
    ]) {
      expect(() => chatSidebarActivation(malformed), throwsFormatException);
    }
  });

  test('new tab preference does not read or migrate legacy grouping', () async {
    final bridge = _registry();
    bridge.preferences['chat_history_grouping_mode'] = 'character';
    final preferences = UserPreferencesManager(
      clients: GeneratedCoreProxyClients(bridge),
    );
    expect(
      await preferences.loadChatSidebarTab(),
      UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB,
    );
    expect(
      await preferences.chatSidebarTabFlow().first,
      UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB,
    );
    final tabId = chatSidebarTabIdentity('owner.second', 'history');
    await preferences.saveChatSidebarTab(tabId);
    expect(await preferences.loadChatSidebarTab(), tabId);
    expect(bridge.preferences[_tabPreference], tabId);
    expect(bridge.preferences['chat_history_grouping_mode'], 'character');
    expect(
      bridge.calls
          .where((call) => call.methodName == 'getPreferences')
          .map((call) => (call.args as Map)['keys']),
      everyElement([_tabPreference]),
    );
  });

  test('corrupt or blank tab preferences fail explicitly', () async {
    final bridge = _registry();
    bridge.preferences[_tabPreference] = ' ';
    final preferences = UserPreferencesManager(
      clients: GeneratedCoreProxyClients(bridge),
    );
    await expectLater(preferences.loadChatSidebarTab(), throwsFormatException);
    await expectLater(
      preferences.chatSidebarTabFlow().first,
      throwsFormatException,
    );
    await expectLater(preferences.saveChatSidebarTab(''), throwsArgumentError);
    expect(
      bridge.calls.where((call) => call.methodName == 'setPreferences'),
      isEmpty,
    );
  });

  testWidgets(
    'host input updates preserve pending action ownership and deliver its completed activation',
    (tester) async {
      final bridge = _registry(), activated = <String>[];
      final clients = GeneratedCoreProxyClients(bridge);
      await tester.pumpWidget(
        _host(bridge, clients: clients, onActivate: activated.add),
      );
      await tester.pumpAndSettle();
      await _openFirstTab(tester);
      final gate = Completer<void>();
      bridge.nextActionRead = gate.future;
      await tester.tap(find.text('Activate current'));
      await tester.pump();
      await tester.pumpWidget(
        _host(
          bridge,
          clients: clients,
          currentChatId: null,
          onActivate: activated.add,
        ),
      );
      await tester.pumpAndSettle();
      expect(activated, isEmpty);
      gate.complete();
      await tester.pumpAndSettle();
      expect(activated, ['chat-a']);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('workspace and arbitrary owners share no hardcoded role tab', (
    tester,
  ) async {
    final bridge = _registry();
    await tester.pumpWidget(_host(bridge));
    await tester.pumpAndSettle();
    expect(find.byKey(_workspaceKey), findsOneWidget);
    expect(
      find.byKey(const ValueKey('chat-sidebar-workspace-tab')),
      findsOneWidget,
    );
    for (final owner in ['owner.first', 'owner.second']) {
      expect(find.byKey(_tabKey(owner)), findsOneWidget);
    }
    expect(find.text('Toolbox entry'), findsNothing);
    expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
    expect(bridge.preferences.containsKey(_tabPreference), isFalse);
  });

  testWidgets(
    'native workspace remains usable while the plugin catalog is pending',
    (tester) async {
      final bridge = _registry();
      final pending = Completer<void>();
      bridge.nextCatalogRead = pending.future;
      await tester.pumpWidget(_host(bridge));
      await tester.pumpAndSettle();
      expect(find.byKey(_workspaceKey), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      expect(bridge.catalogReads, 1);
      pending.complete();
      await tester.pumpAndSettle();
      expect(find.byKey(_tabKey('owner.first')), findsOneWidget);
    },
  );

  testWidgets(
    'a broken plugin catalog cannot replace the native workspace with an error',
    (tester) async {
      final bridge = _registry();
      bridge.routes.clear();
      await tester.pumpWidget(_host(bridge));
      await tester.pumpAndSettle();
      expect(find.byKey(_workspaceKey), findsOneWidget);
      expect(
        find.byKey(const ValueKey('chat-sidebar-tab-error')),
        findsNothing,
      );
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
    },
  );

  testWidgets(
    'the built-in workspace can scroll its own brand and capsule header',
    (tester) async {
      final bridge = _registry();
      bridge.routes.clear();
      await tester.pumpWidget(
        _host(
          bridge,
          workspaceWithTabsBuilder: (context, tabs) => CustomScrollView(
            key: _workspaceKey,
            slivers: [
              SliverToBoxAdapter(child: tabs),
              const SliverToBoxAdapter(child: Text('Built-in history')),
            ],
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('Built-in history'), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(_workspaceKey),
          matching: find.byKey(const ValueKey('chat-sidebar-segmented-switch')),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('chat-sidebar-tab-error')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'tab embeds its owning route and forwards opaque input untouched',
    (tester) async {
      final bridge = _registry();
      await tester.pumpWidget(_host(bridge));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.byKey(_tabKey('owner.second')));
      await tester.tap(find.byKey(_tabKey('owner.second')));
      await tester.pumpAndSettle();
      final launcher = _launcher(tester);
      expect(launcher.plugin.packageName, 'owner.second');
      expect(launcher.initialRouteId, 'history-route');
      expect(launcher.embeddedScreenPath, 'ui/history.js');
      expect(launcher.showLauncherChrome, isFalse);
      expect(launcher.initialState, {
        'input': _input('owner.second'),
        'chatSidebar': {
          'chats': [chatSidebarSummary(_chat('chat-a'))],
          'currentChatId': 'chat-a',
          'activeStreamingChatIds': ['chat-a'],
        },
      });
      expect(find.byType(Dialog), findsNothing);
      expect(find.byKey(_workspaceKey), findsNothing);
      expect(
        bridge.calls
            .where(
              (call) => call.methodName == 'executeToolPkgComposeDslScript',
            )
            .last
            .args,
        containsPair('containerPackageName', 'owner.second'),
      );
    },
  );

  testWidgets(
    'switching visited tabs preserves execution leases and loaded trees',
    (tester) async {
      final bridge = _registry();
      await tester.pumpWidget(_host(bridge));
      await tester.pumpAndSettle();
      await _openFirstTab(tester);
      final mounted = tester.state(find.byType(ToolPkgUiLauncherScreen));
      final initialRenders = bridge.calls
          .where((call) => call.methodName == 'executeToolPkgComposeDslScript')
          .length;
      final acquisitions = bridge.calls
          .where((call) => call.methodName == 'acquireToolPkgExecutionEngine')
          .length;
      for (var i = 0; i < 3; i++) {
        await tester.tap(
          find.byKey(const ValueKey('chat-sidebar-workspace-tab')),
        );
        await tester.pumpAndSettle();
        expect(find.byKey(_workspaceKey), findsOneWidget);
        await _openFirstTab(tester);
        expect(
          identical(
            tester.state(find.byType(ToolPkgUiLauncherScreen)),
            mounted,
          ),
          isTrue,
        );
      }
      expect(
        bridge.calls
            .where(
              (call) => call.methodName == 'executeToolPkgComposeDslScript',
            )
            .length,
        initialRenders,
      );
      expect(
        bridge.calls
            .where((call) => call.methodName == 'acquireToolPkgExecutionEngine')
            .length,
        acquisitions,
      );
      expect(
        bridge.calls.where(
          (call) => call.methodName == 'releaseToolPkgExecutionEngine',
        ),
        isEmpty,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('ordinary Compose results do not activate a chat', (
    tester,
  ) async {
    final bridge = _registry();
    final activated = <String>[];
    await tester.pumpWidget(_host(bridge, onActivate: activated.add));
    await tester.pumpAndSettle();
    await _openFirstTab(tester);
    await tester.tap(find.text('Preview only'));
    await tester.pumpAndSettle();
    expect(activated, isEmpty);
    expect(bridge.historyReads, 0);
    expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
  });

  testWidgets(
    'activation validates a newly created chat using a fresh Core watch',
    (tester) async {
      final bridge = _registry();
      final activated = <String>[];
      await tester.pumpWidget(_host(bridge, onActivate: activated.add));
      await tester.pumpAndSettle();
      await _openFirstTab(tester);
      bridge.chats.add(_chat('chat-created'));
      await tester.tap(find.text('Activate newly created'));
      await tester.pumpAndSettle();
      expect(activated, ['chat-created']);
      expect(bridge.historyReads, 1);
      expect(bridge.catalogReads, 2);
    },
  );

  testWidgets('missing chats and malformed activations are explicit errors', (
    tester,
  ) async {
    final bridge = _registry();
    final activated = <String>[];
    await tester.pumpWidget(_host(bridge, onActivate: activated.add));
    await tester.pumpAndSettle();
    await _openFirstTab(tester);
    await tester.tap(find.text('Activate missing'));
    await tester.pumpAndSettle();
    expect(activated, isEmpty);
    expect(find.textContaining('conversation does not exist'), findsOneWidget);
    await tester.tap(find.text('Malformed activation'));
    await tester.pumpAndSettle();
    expect(activated, isEmpty);
    expect(
      find.textContaining('exactly type and a nonblank chatId'),
      findsOneWidget,
    );
  });

  testWidgets(
    'ordinary and duplicate results cannot release an in-flight activation',
    (tester) async {
      final bridge = _registry();
      final started = <String>[];
      final release = Completer<void>();
      await tester.pumpWidget(
        _host(
          bridge,
          onActivateAsync: (chatId) async {
            started.add(chatId);
            await release.future;
          },
        ),
      );
      await tester.pumpAndSettle();
      await _openFirstTab(tester);
      final callback = _launcher(tester).onActionResult!;
      callback({'type': 'toolpkg.chat.activate', 'chatId': 'chat-a'});
      await tester.pumpAndSettle();
      expect(started, ['chat-a']);
      callback({'preview': true});
      callback({'type': 'toolpkg.chat.activate', 'chatId': 'chat-a'});
      await tester.pumpAndSettle();
      callback({'type': 'toolpkg.chat.activate', 'chatId': 'chat-a'});
      await tester.pumpAndSettle();
      expect(started, ['chat-a']);
      expect(
        find.textContaining('activation is already running'),
        findsOneWidget,
      );
      release.complete();
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'disabled selected tab stays selected and never shows workspace',
    (tester) async {
      final bridge = _registry();
      await tester.pumpWidget(_host(bridge));
      await tester.pumpAndSettle();
      await _openFirstTab(tester);
      final savedId = bridge.preferences[_tabPreference];
      bridge.entries.removeWhere(
        (entry) => entry.containerPackageName == 'owner.first',
      );
      bridge.routes.removeWhere(
        (route) => route.containerPackageName == 'owner.first',
      );
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await tester.pumpAndSettle();
      expect(bridge.preferences[_tabPreference], savedId);
      expect(find.textContaining('not enabled or registered'), findsOneWidget);
      expect(find.byKey(_workspaceKey), findsNothing);
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
    },
  );

  testWidgets(
    'disabled runtime discovered during activation invalidates cached content',
    (tester) async {
      final bridge = _registry();
      final activated = <String>[];
      await tester.pumpWidget(_host(bridge, onActivate: activated.add));
      await tester.pumpAndSettle();
      await _openFirstTab(tester);
      bridge.disabledOwners.add('owner.first');
      _launcher(tester).onActionResult!({
        'type': 'toolpkg.chat.activate',
        'chatId': 'chat-a',
      });
      await tester.pumpAndSettle();
      expect(activated, isEmpty);
      expect(find.textContaining('owner is disabled'), findsWidgets);
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
      expect(find.byKey(_workspaceKey), findsNothing);
      expect(
        bridge.preferences[_tabPreference],
        chatSidebarTabIdentity('owner.first', 'history'),
      );
    },
  );

  testWidgets(
    'hot reload rejects callbacks belonging to the previous catalog generation',
    (tester) async {
      final bridge = _registry();
      final activated = <String>[];
      await tester.pumpWidget(_host(bridge, onActivate: activated.add));
      await tester.pumpAndSettle();
      await _openFirstTab(tester);
      final staleCallback = _launcher(tester).onActionResult!;
      PluginHotReload.revision.value += 1;
      await tester.pumpAndSettle();
      staleCallback({'type': 'toolpkg.chat.activate', 'chatId': 'chat-a'});
      await tester.pumpAndSettle();
      expect(activated, isEmpty);
      expect(find.textContaining('no longer current'), findsOneWidget);
      expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
    },
  );

  testWidgets(
    'selection changes during authoritative chat validation reject activation',
    (tester) async {
      final bridge = _registry();
      final activated = <String>[];
      await tester.pumpWidget(_host(bridge, onActivate: activated.add));
      await tester.pumpAndSettle();
      await _openFirstTab(tester);
      final gate = Completer<void>();
      bridge.nextHistoryRead = gate.future;
      _launcher(tester).onActionResult!({
        'type': 'toolpkg.chat.activate',
        'chatId': 'chat-a',
      });
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey('chat-sidebar-workspace-tab')),
      );
      await tester.pumpAndSettle();
      gate.complete();
      await tester.pumpAndSettle();
      expect(activated, isEmpty);
      expect(find.byKey(_workspaceKey), findsOneWidget);
      expect(find.textContaining('selection changed'), findsOneWidget);
    },
  );

  testWidgets(
    'live generic state updates preserve the embedded launcher instance',
    (tester) async {
      final bridge = _registry();
      final clients = GeneratedCoreProxyClients(bridge);
      await tester.pumpWidget(_host(bridge, clients: clients));
      await tester.pumpAndSettle();
      await _openFirstTab(tester);
      final oldElement = tester.element(find.byType(ToolPkgUiLauncherScreen));
      final chat = _chat('chat-b');
      await tester.pumpWidget(
        _host(
          bridge,
          clients: clients,
          chats: [chat],
          currentChatId: 'chat-b',
          streamingChatIds: {'chat-b'},
        ),
      );
      await tester.pumpAndSettle();
      expect(
        tester.element(find.byType(ToolPkgUiLauncherScreen)),
        same(oldElement),
      );
      expect(_launcher(tester).initialState['input'], _input('owner.first'));
      expect(_launcher(tester).initialState['chatSidebar'], {
        'chats': [chatSidebarSummary(chat)],
        'currentChatId': 'chat-b',
        'activeStreamingChatIds': ['chat-b'],
      });
      expect(
        bridge.renderedStates.last['chatSidebar'],
        _launcher(tester).initialState['chatSidebar'],
      );
    },
  );

  testWidgets(
    'peer preference changes select registered routes without rebuilding the catalog',
    (tester) async {
      final bridge = _registry();
      await tester.pumpWidget(_host(bridge));
      await tester.pumpAndSettle();
      final initialReads = bridge.catalogReads;
      bridge.commitPreferences({
        _tabPreference: chatSidebarTabIdentity('owner.second', 'history'),
      });
      await tester.pumpAndSettle();
      expect(_launcher(tester).plugin.packageName, 'owner.second');
      expect(bridge.catalogReads, initialReads);
    },
  );

  testWidgets(
    'cross-owner and duplicate routes are not rendered as valid tabs',
    (tester) async {
      final bridge = _registry();
      bridge.preferences[_tabPreference] = chatSidebarTabIdentity(
        'owner.first',
        'history',
      );
      bridge.routes.removeWhere(
        (route) => route.containerPackageName == 'owner.first',
      );
      await tester.pumpWidget(_host(bridge));
      await tester.pumpAndSettle();
      expect(
        find.textContaining('one exact registered Compose route'),
        findsOneWidget,
      );
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
      bridge.routes.addAll([_route('owner.first'), _route('owner.first')]);
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await tester.pumpAndSettle();
      expect(
        find.textContaining('one exact registered Compose route'),
        findsOneWidget,
      );
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
    },
  );
  for (final expanded in <bool>[true, false]) {
    for (final source in <String?>['chat-a', null]) {
      testWidgets(
        '${expanded ? 'expanded' : 'collapsed'} native create forwards exact nullable source $source',
        (tester) async {
          final bridge = _registry();
          var activations = 0;
          await tester.pumpWidget(
            _nativeDrawer(
              bridge,
              expanded: expanded,
              currentChatId: source,
              onActivated: () => activations++,
            ),
          );
          await tester.pumpAndSettle();
          await tester.tap(
            expanded
                ? find.byKey(const ValueKey('workspace-create-chat'))
                : find.byIcon(Icons.add_comment_outlined),
          );
          await tester.pumpAndSettle();
          final request = bridge.calls.singleWhere(
            (call) => call.methodName == 'createNewChat',
          );
          expect(request.args, {
            'setAsCurrentChat': true,
            'sourceChatId': source,
            'input': null,
          });
          expect(activations, 1);
          expect(newChatIntroArmed.value, isTrue);
          expect(
            bridge.calls.where(
              (call) => call.methodName == 'chatConfiguration',
            ),
            isEmpty,
          );
        },
      );
    }
    testWidgets(
      '${expanded ? 'expanded' : 'collapsed'} creation error neither activates nor arms intro',
      (tester) async {
        final bridge = _registry();
        bridge.createError = StateError('Authoritative create failure');
        var activations = 0;
        await tester.pumpWidget(
          _nativeDrawer(
            bridge,
            expanded: expanded,
            currentChatId: 'chat-a',
            onActivated: () => activations++,
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(
          expanded
              ? find.byKey(const ValueKey('workspace-create-chat'))
              : find.byIcon(Icons.add_comment_outlined),
        );
        await tester.pumpAndSettle();
        expect(activations, 0);
        expect(newChatIntroArmed.value, isFalse);
        expect(
          find.textContaining('Authoritative create failure'),
          findsWidgets,
        );
      },
    );
  }

  testWidgets(
    'native row reordering uses only the approved generic canonical order arguments',
    (tester) async {
      final bridge = _registry();
      final first = _chat('chat-a');
      final second = _chat('chat-b');
      await tester.pumpWidget(
        _nativeDrawer(
          bridge,
          expanded: true,
          currentChatId: null,
          histories: [first, second],
          onActivated: () {},
        ),
      );
      await tester.pumpAndSettle();
      final target = tester
          .widgetList<ConversationDrawerItem>(
            find.byType(ConversationDrawerItem),
          )
          .singleWhere((row) => row.history.id == second.id);
      target.onMoveTo(first);
      await tester.pumpAndSettle();
      final request = bridge.calls.singleWhere(
        (call) => call.methodName == 'updateChatOrder',
      );
      final args = request.args as Map;
      expect(args.keys.toSet(), {'reorderedHistories', 'movedItem'});
      expect(
        (args['reorderedHistories'] as List).map((item) => (item as Map)['id']),
        ['chat-b', 'chat-a'],
      );
      expect((args['movedItem'] as Map)['id'], 'chat-a');
    },
  );
}

/// Mounts the real expanded or collapsed native sidebar without a production host bootstrap.
Widget _nativeDrawer(
  _SidebarRegistryBridge bridge, {
  required bool expanded,
  required String? currentChatId,
  required VoidCallback onActivated,
  List<core.ChatHistoryListItem> histories = const [],
}) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: Scaffold(
    body: SizedBox(
      width: 420,
      child: expanded
          ? DrawerContent(
              navigationEntries: const [],
              pluginEntries: const [],
              selectedRouteId: 'native-chat',
              appearance: _appearance,
              histories: histories,
              activeStreamingChatIds: const {},
              currentChatId: currentChatId,
              errorMessage: null,
              loading: false,
              bridge: bridge,
              onNavigationEntrySelected: (_) {},
              onConversationActivated: onActivated,
            )
          : CollapsedDrawerContent(
              navigationEntries: const [
                NavigationEntrySpec(
                  entryId: 'main.ai_chat',
                  routeId: 'native-chat',
                  surface: NavigationSurface.mainSidebarAi,
                  title: 'Chat',
                  icon: Icons.chat,
                ),
                NavigationEntrySpec(
                  entryId: 'main.package_manager',
                  routeId: 'native-packages',
                  surface: NavigationSurface.mainSidebarAi,
                  title: 'Packages',
                  icon: Icons.extension,
                ),
                NavigationEntrySpec(
                  entryId: 'main.settings',
                  routeId: 'native-settings',
                  surface: NavigationSurface.mainSidebarAi,
                  title: 'Settings',
                  icon: Icons.settings,
                ),
              ],
              pluginEntries: const [],
              selectedRouteId: 'native-chat',
              appearance: _appearance,
              currentChatId: currentChatId,
              bridge: bridge,
              onNavigationEntrySelected: (_) {},
              onConversationActivated: onActivated,
            ),
    ),
  ),
);

const _appearance = NavigationDrawerAppearance(
  containerColor: Colors.white,
  titleColor: Colors.black,
  statusAvailableColor: Colors.blue,
  itemColor: Colors.black87,
  buttonContainerColor: Colors.white70,
  selectedContainerColor: Colors.blue,
  selectedContentColor: Colors.white,
  dividerColor: Colors.black12,
  transparentSurfaceEnabled: false,
);

/// Mounts the production tab host with authoritative generated clients and no modal.
Widget _host(
  _SidebarRegistryBridge bridge, {
  GeneratedCoreProxyClients? clients,
  List<core.ChatHistoryListItem>? chats,
  String? currentChatId = 'chat-a',
  Set<String> streamingChatIds = const {'chat-a'},
  void Function(String)? onActivate,
  Future<void> Function(String)? onActivateAsync,
  Widget Function(BuildContext, Widget)? workspaceWithTabsBuilder,
}) => MaterialApp(
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(
    body: SizedBox(
      width: 520,
      child: ChatSidebarTabHost(
        clients: clients ?? GeneratedCoreProxyClients(bridge),
        chats: chats ?? [_chat('chat-a')],
        currentChatId: currentChatId,
        activeStreamingChatIds: streamingChatIds,
        workspaceWithTabsBuilder: workspaceWithTabsBuilder,
        workspaceBuilder: (_) =>
            const Text('Native workspace', key: _workspaceKey),
        onActivateChat: (chatId) async {
          onActivate?.call(chatId);
          if (onActivateAsync != null) {
            await onActivateAsync(chatId);
          }
        },
      ),
    ),
  ),
);

/// Selects a real registered chip before dispatching any Compose action.
Future<void> _openFirstTab(WidgetTester tester) async {
  await tester.tap(find.byKey(_tabKey('owner.first')));
  await tester.pumpAndSettle();
}

/// Reads the actual embedded launcher after registry and preference resolution.
ToolPkgUiLauncherScreen _launcher(WidgetTester tester) => tester
    .widget<ToolPkgUiLauncherScreen>(find.byType(ToolPkgUiLauncherScreen));

/// Matches the collision-safe identity of the actual registered navigation item.
ValueKey<String> _tabKey(String owner) =>
    ValueKey('chat-sidebar-tab:${chatSidebarTabIdentity(owner, 'history')}');

/// Creates explicit registry fixtures for two unrelated packages and a toolbox entry.
_SidebarRegistryBridge _registry() => _SidebarRegistryBridge(
  entries: [
    _entry('owner.first'),
    _entry('owner.second'),
    _entry('owner.toolbox', surface: 'toolbox'),
  ],
  routes: [_route('owner.first'), _route('owner.second')],
  chats: [_chat('chat-a')],
);

/// Supplies nested plugin input that the host must never decode into native modes.
Object _input(String owner) => {
  'view': 'plugin-owned-view',
  'selection': [
    owner,
    null,
    {'mode': 'opaque', 'card': 'not-a-host-field'},
  ],
};

/// Builds real NavigationEntry metadata using the shared surface registration model.
core.ToolPkgNavigationEntry _entry(
  String owner, {
  String surface = 'chat_sidebar_tabs',
}) => core.ToolPkgNavigationEntry(
  containerPackageName: owner,
  toolPkgId: owner,
  entryId: 'history',
  routeId: 'history-route',
  params: _input(owner),
  surface: surface,
  title: surface == 'toolbox' ? 'Toolbox entry' : 'History $owner',
  description: '',
  action: null,
  icon: 'chat',
  order: owner == 'owner.first' ? 1 : 2,
);

/// Resolves the same route ID independently in each owning package namespace.
core.ToolPkgUiRoute _route(String owner) => core.ToolPkgUiRoute(
  containerPackageName: owner,
  toolPkgId: owner,
  routeId: 'history-route',
  uiModuleId: 'history-route',
  runtime: 'compose_dsl',
  screen: 'ui/history.js',
  title: 'History $owner',
  description: '',
  moduleSpec: const {},
  keepAlive: false,
);

/// Decodes the generated canonical DTO while proving legacy metadata never enters tab state.
core.ChatHistoryListItem _chat(String id) => core.ChatHistoryListItem.fromJson({
  'id': id,
  'title': 'Title $id',
  'updatedAt': '2026-10-08T12:00:00Z',
  'displayOrder': 3,
  'workspaceId': 'workspace-a',
  'workspaceName': 'Workspace A',
  'locked': true,
  'pinned': true,
  'group': 'legacy-native-folder',
  'characterCardName': 'legacy-card-not-forwarded',
  'characterGroupId': 'legacy-subject-not-forwarded',
});

/// Exercises generated catalog, preference, watch, and Compose render codecs.
class _SidebarRegistryBridge extends OperitRuntimeBridge {
  /// Requires an explicit registry and canonical chat snapshot for each test.
  _SidebarRegistryBridge({
    required this.entries,
    required this.routes,
    required this.chats,
  });

  final List<core.ToolPkgNavigationEntry> entries;
  final List<core.ToolPkgUiRoute> routes;
  final List<core.ChatHistoryListItem> chats;
  final Map<String, String> preferences = {};
  final List<CoreCallRequest> calls = [];
  final List<Map<String, Object?>> renderedStates = [];
  final Set<String> disabledOwners = {};
  final Map<CoreWatchRequest, StreamController<CoreEvent>> _preferenceWatches =
      {};
  Future<void>? nextHistoryRead;
  Future<void>? nextActionRead;
  Future<void>? nextCatalogRead;
  Object? createError;
  int catalogReads = 0;
  int historyReads = 0;

  /// Publishes a committed canonical preference snapshot through actual watch events.
  void commitPreferences(Map<String, String> values) {
    preferences
      ..clear()
      ..addAll(values);
    for (final entry in _preferenceWatches.entries) {
      entry.value.add(_event(entry.key, Map<String, String>.of(preferences)));
    }
  }

  /// Implements only calls made by the production embedded sidebar and preferences.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map<String, Object?>;
    Object? value;
    switch (request.methodName) {
      case 'getPreferences':
        expect(args['fileName'], _preferenceFile);
        value = <String, String>{
          for (final key in (args['keys'] as List).cast<String>())
            if (preferences[key] case final String stored) key: stored,
        };
      case 'setPreferences':
        expect(args['fileName'], _preferenceFile);
        commitPreferences({
          ...preferences,
          ...(args['values'] as Map).cast<String, String>(),
        });
        value = null;
      case 'getToolPkgNavigationEntries':
        catalogReads += 1;
        if (nextCatalogRead case final Future<void> pending) {
          nextCatalogRead = null;
          await pending;
        }
        value = entries.map((entry) => entry.toJson()).toList();
      case 'getToolPkgUiRoutes':
        expect(args['runtime'], 'compose_dsl');
        value = routes.map((route) => route.toJson()).toList();
      case 'getToolPkgContainerRuntime':
        final owner = args['containerPackageName'] as String;
        value = disabledOwners.contains(owner)
            ? null
            : _runtime(owner).toJson();
      case 'createNewChat':
        if (createError case final Object error) throw error;
        value = 'new-chat-id';
      case 'updateChatOrder':
        value = null;
      case 'acquireToolPkgExecutionEngine':
      case 'releaseToolPkgExecutionEngine':
        value = null;
      case 'readToolPkgTextResource':
        expect(args['resourcePath'], 'ui/history.js');
        value = 'export default function render() {}';
      case 'executeToolPkgComposeDslScript':
        final state =
            (args['runtimeOptions'] as Map)['state'] as Map<String, Object?>;
        renderedStates.add(state);
        value = _render(state);
      default:
        throw StateError('Unexpected sidebar call: ${request.methodName}');
    }
    return encodeCoreLink([0, value]);
  }

  /// Provides authoritative history snapshots and serialized real Compose action events.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName == 'preferencesFlow') {
      expect((request.args as Map)['fileName'], _preferenceFile);
      late final StreamController<CoreEvent> controller;
      controller = StreamController<CoreEvent>(
        onListen: () => controller.add(
          _event(request, Map<String, String>.of(preferences)),
        ),
        onCancel: () => _preferenceWatches.remove(request),
      );
      _preferenceWatches[request] = controller;
      return controller.stream;
    }
    if (request.propertyName == 'chatHistoryListItemsFlow') {
      historyReads += 1;
      final gate = nextHistoryRead;
      nextHistoryRead = null;
      return _historyEvents(request, gate);
    }
    if (request.propertyName == 'dispatchToolPkgComposeDslActionEvents') {
      return _actionEvents(request);
    }
    throw StateError('Unexpected sidebar watch: ${request.propertyName}');
  }

  /// Releases deliberately blocked snapshots without substituting stale host chat state.
  Stream<CoreEvent> _historyEvents(
    CoreWatchRequest request,
    Future<void>? gate,
  ) async* {
    await gate;
    yield _event(request, chats.map((chat) => chat.toJson()).toList());
  }

  /// Delivers action results through the actual launcher dispatch/render callback path.
  Stream<CoreEvent> _actionEvents(CoreWatchRequest request) async* {
    final args = request.args as Map<String, Object?>;
    final gate = nextActionRead;
    nextActionRead = null;
    await gate;
    final state =
        (args['runtimeOptions'] as Map)['state'] as Map<String, Object?>;
    final Object result = switch (args['actionId']) {
      'preview' => {'preview': true},
      'activate' => {'type': 'toolpkg.chat.activate', 'chatId': 'chat-a'},
      'created' => {'type': 'toolpkg.chat.activate', 'chatId': 'chat-created'},
      'missing' => {'type': 'toolpkg.chat.activate', 'chatId': 'missing-chat'},
      'malformed' => {'type': 'toolpkg.chat.activate', 'chatId': ''},
      _ => throw StateError('Unexpected sidebar action: ${args['actionId']}'),
    };
    yield _event(
      request,
      jsonEncode({'phase': 'final', 'result': _render(state, result: result)}),
    );
    yield _event(request, jsonEncode({'phase': 'complete'}));
  }

  /// Rejects push operations outside the sidebar fixture's real contract.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw StateError('Unexpected sidebar push');

  /// Rejects snapshot API calls because production consumers use generated watch streams.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw StateError('Unexpected sidebar snapshot');
}

/// Encodes a generated-client-compatible committed Core watch value.
CoreEvent _event(CoreWatchRequest request, Object? value) => CoreEvent.raw(
  requestId: request.requestId,
  target: request.target,
  propertyName: request.propertyName,
  kind: 'Changed',
  valueBytes: encodeCoreLink(value),
  decodeValue: decodeCoreLink<Object?>,
);

/// Renders interactive DSL nodes without bypassing production action dispatch.
String _render(Map<String, Object?> state, {Object? result}) => jsonEncode({
  'tree': {
    'type': 'Column',
    'props': {},
    'children': [
      for (final entry in const {
        'preview': 'Preview only',
        'activate': 'Activate current',
        'created': 'Activate newly created',
        'missing': 'Activate missing',
        'malformed': 'Malformed activation',
      }.entries)
        {
          'type': 'Button',
          'props': {
            'text': entry.value,
            'onClick': {'__actionId': entry.key},
          },
          'children': <Object?>[],
          'slots': <String, Object?>{},
        },
    ],
    'slots': <String, Object?>{},
  },
  'state': state,
  'memo': <String, Object?>{},
  'actionResult': result,
});

/// Supplies complete real runtime metadata for the registered embedded Compose route.
core.ToolPkgContainerRuntime _runtime(String owner) =>
    core.ToolPkgContainerRuntime(
      chatLifecycleHooks: const [],
      packageName: owner,
      displayName: const core.LocalizedText(
        values: {'default': 'Sidebar test package'},
      ),
      description: const core.LocalizedText(
        values: {'default': 'Registry fixture'},
      ),
      version: '1.0.0',
      apiVersion: '2.0.0',
      publicApi: null,
      publicApis: const [],
      requires: const [],
      dependencyIssues: const [],
      manifestExtensions: const {},
      author: const ['Operit'],
      mainEntry: 'dist/main.js',
      sourceType: core.ToolPkgSourceType.externalValue,
      sourcePath: 'test',
      subpackages: const [],
      resources: const [],
      wasmModules: const [],
      workflowTemplates: const [],
      workspaceTemplates: const [],
      uiModules: const [],
      uiRoutes: const [
        core.ToolPkgUiRouteRuntime(
          id: 'history-route',
          routeId: 'history-route',
          runtime: 'compose_dsl',
          screen: 'ui/history.js',
          title: core.LocalizedText(values: {'default': 'History'}),
          keepAlive: false,
        ),
      ],
      chatComposerSlots: const [],
      navigationEntries: const [],
      desktopWidgets: const [],
      appLifecycleHooks: const [],
      messageProcessingPlugins: const [],
      xmlRenderPlugins: const [],
      inputMenuTogglePlugins: const [],
      chatInputHooks: const [],
      chatViewHooks: const [],
      chatMessageHooks: const [],
      chatMessageMenuItems: const [],
      chatRuntimeHooks: const [],
      hostEventHooks: const [],
      toolLifecycleHooks: const [],
      promptInputHooks: const [],
      promptHistoryHooks: const [],
      promptEstimateHistoryHooks: const [],
      systemPromptComposeHooks: const [],
      toolPromptComposeHooks: const [],
      promptFinalizeHooks: const [],
      promptEstimateFinalizeHooks: const [],
      summaryGenerateHooks: const [],
      coreCommands: const [],
      aiProviders: const [],
      manifestExtensionHandlers: const [],
      logoResource: null,
      marketOrigin: null,
    );
