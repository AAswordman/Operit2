import 'dart:async';
import 'dart:collection';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/contributions/ChatUiContributionModels.dart';
import 'package:operit2/ui/common/contributions/ToolPkgChatUiCatalog.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceHomeContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceOverviewContributionCatalog.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceOverviewModels.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Covers generic workspace projection, the real catalog, request races, and UI states.
void main() {
  test(
    'workspace membership uses only exact current chat and workspace IDs',
    () {
      final histories = [
        _chat('a', 'workspace-a'),
        _chat('b', 'workspace-a'),
        _chat('c', 'workspace-b'),
        _chat('d', 'workspace-a '),
        _chat('e', null),
      ];
      final scope = WorkspaceOverviewChatScope.fromHistories(
        histories: histories,
        currentChatId: 'a',
        hasBoundWorkspace: true,
      );
      expect(scope.workspaceName, 'Same display name');
      expect(scope.chats.map((chat) => chat.id), ['a', 'b']);
      histories.clear();
      expect(scope.chats.map((chat) => chat.id), ['a', 'b']);
      expect(() => scope.chats.clear(), throwsUnsupportedError);
    },
  );

  test(
    'null selection or explicitly unbound workspace has no associated histories',
    () {
      final histories = [_chat('a', 'workspace-a'), _chat('unbound', null)];
      for (final currentChatId in <String?>[null, 'unbound']) {
        final scope = WorkspaceOverviewChatScope.fromHistories(
          histories: histories,
          currentChatId: currentChatId,
          hasBoundWorkspace: true,
        );
        expect(scope.chats, isEmpty);
      }
    },
  );

  test(
    'an actual unbind never reuses the previous workspace history membership',
    () {
      final scope = WorkspaceOverviewChatScope.fromHistories(
        histories: [_chat('a', 'workspace-a'), _chat('b', 'workspace-a')],
        currentChatId: 'a',
        hasBoundWorkspace: false,
      );
      expect(scope.workspaceName, isNull);
      expect(scope.chats, isEmpty);
    },
  );

  test(
    'missing or duplicate selected records are explicit bound-workspace errors',
    () {
      for (final histories in [
        [_chat('other', 'workspace-a')],
        [_chat('selected', 'workspace-a'), _chat('selected', 'workspace-b')],
      ]) {
        expect(
          () => WorkspaceOverviewChatScope.fromHistories(
            histories: histories,
            currentChatId: 'selected',
            hasBoundWorkspace: true,
          ),
          throwsStateError,
        );
      }
    },
  );

  test('explicitly unbound state needs no selected history record', () {
    final scope = WorkspaceOverviewChatScope.fromHistories(
      histories: [_chat('other', 'workspace-a')],
      currentChatId: 'missing',
      hasBoundWorkspace: false,
    );
    expect(scope, same(WorkspaceOverviewChatScope.empty));
  });

  test('duplicate or blank authoritative identities fail explicitly', () {
    for (final histories in [
      [_chat('a', 'workspace-a'), _chat('a', 'workspace-a')],
      [
        _chat('a', 'workspace-a'),
        _chat('b', 'workspace-a'),
        _chat('b', 'workspace-a'),
      ],
    ]) {
      expect(
        () => WorkspaceOverviewChatScope.fromHistories(
          histories: histories,
          currentChatId: 'a',
          hasBoundWorkspace: true,
        ),
        throwsStateError,
      );
    }
    expect(
      () => WorkspaceOverviewChatScope.fromHistories(
        histories: [_chat('a', ' ')],
        currentChatId: 'a',
        hasBoundWorkspace: true,
      ),
      throwsFormatException,
    );
  });

  test(
    'used contribution counts retain owner, opaque ID, action, and stable order',
    () {
      final first = _section('owner::first', 'opaque', ['a']);
      final second = _section('owner', 'first::opaque', ['a']);
      final popular = _section('owner.third', 'opaque', ['a', 'b']);
      final usages = workspaceContributionUsages([
        first,
        second,
        popular,
        _section('owner.unused', 'opaque', []),
      ]);
      expect(usages.map((usage) => usage.conversationCount), [2, 1, 1]);
      expect(usages[1].identity, ('owner::first', 'opaque'));
      expect(usages[2].identity, ('owner', 'first::opaque'));
      expect(usages[1].identity, isNot(usages[2].identity));
      expect(usages[1].title, first.title);
      expect(usages[1].avatarUri, first.avatarUri);
      expect(usages[1].preview, same(first.preview));
      expect(() => usages.clear(), throwsUnsupportedError);
    },
  );

  test(
    'real catalog receives only the current workspace generic summaries',
    () async {
      final scope = WorkspaceOverviewChatScope.fromHistories(
        histories: [
          _chat('a', 'workspace-a'),
          _chat('b', 'workspace-a'),
          _chat('c', 'workspace-b'),
        ],
        currentChatId: 'a',
        hasBoundWorkspace: true,
      );
      final first = _completedSections([
        _sectionData('opaque', ['a', 'b']),
      ]);
      final second = _completedSections([
        _sectionData('opaque', ['b']),
      ]);
      final bridge = _WorkspaceCatalogBridge(
        owners: ['owner.first', 'owner.second'],
        routes: [_route('owner.first'), _route('owner.second')],
        requests: [first, second],
      );
      final controller = WorkspaceOverviewContributionCatalog();
      addTearDown(controller.dispose);
      await controller.refresh(catalog: _catalog(bridge), chats: scope.chats);
      final usages = _ready(controller).usages;
      expect(usages.map((usage) => usage.identity), [
        ('owner.first', 'opaque'),
        ('owner.second', 'opaque'),
      ]);
      expect(usages.map((usage) => usage.conversationCount), [2, 1]);
      expect(usages.first.preview.route.containerPackageName, 'owner.first');
      expect(usages.last.preview.route.containerPackageName, 'owner.second');
      expect(usages.first.preview.input, _opaqueInput);
      for (final payload in bridge.payloads) {
        expect(payload.keys, ['chats']);
        expect(
          payload['chats'],
          scope.chats.map(chatUiHistorySummary).toList(),
        );
      }
      expect(scope.chats.length, 2);
    },
  );

  test(
    'a successful empty registration produces an empty catalog without invocation',
    () async {
      final bridge = _WorkspaceCatalogBridge(
        owners: [],
        routes: [],
        requests: [],
      );
      final controller = WorkspaceOverviewContributionCatalog();
      addTearDown(controller.dispose);
      await controller.refresh(
        catalog: _catalog(bridge),
        chats: [_chat('a', 'workspace-a')],
      );
      expect(_ready(controller).usages, isEmpty);
      expect(bridge.payloads, isEmpty);
      expect(bridge.calls.map((call) => call.methodName), [
        'getToolPkgPublicApiOwners',
        'getToolPkgUiRoutes',
      ]);
    },
  );

  for (final stage in ['discovery', 'routes', 'invocation']) {
    test(
      '$stage errors are retained as explicit failures, not empty catalogs',
      () async {
        final bridge = _WorkspaceCatalogBridge(
          owners: ['owner.first'],
          routes: [_route('owner.first')],
          requests: [_completedSections([])],
        );
        final error = StateError('$stage unavailable');
        switch (stage) {
          case 'discovery':
            bridge.discoveryError = error;
          case 'routes':
            bridge.routesError = error;
          case 'invocation':
            bridge.invocationError = error;
        }
        final controller = WorkspaceOverviewContributionCatalog();
        addTearDown(controller.dispose);
        await controller.refresh(
          catalog: _catalog(bridge),
          chats: [_chat('a', 'workspace-a')],
        );
        final state = controller.state;
        expect(state, isA<WorkspaceContributionsFailed>());
        expect((state as WorkspaceContributionsFailed).error, same(error));
      },
    );
  }

  for (final chatIds in [
    <String>['outside'],
    <String>['a', 'a'],
  ]) {
    test('catalog rejects invalid contributed chat IDs $chatIds', () async {
      final bridge = _WorkspaceCatalogBridge(
        owners: ['owner.first'],
        routes: [_route('owner.first')],
        requests: [
          _completedSections([_sectionData('opaque', chatIds)]),
        ],
      );
      final controller = WorkspaceOverviewContributionCatalog();
      addTearDown(controller.dispose);
      await controller.refresh(
        catalog: _catalog(bridge),
        chats: [_chat('a', 'workspace-a')],
      );
      expect(
        (controller.state as WorkspaceContributionsFailed).error,
        isA<FormatException>(),
      );
    });
  }

  test(
    'malformed section contracts are errors rather than fabricated display items',
    () async {
      final malformed = _sectionData('opaque', ['a'])..remove('preview');
      final bridge = _WorkspaceCatalogBridge(
        owners: ['owner.first'],
        routes: [_route('owner.first')],
        requests: [
          _completedSections([malformed]),
        ],
      );
      final controller = WorkspaceOverviewContributionCatalog();
      addTearDown(controller.dispose);
      await controller.refresh(
        catalog: _catalog(bridge),
        chats: [_chat('a', 'workspace-a')],
      );
      expect(
        (controller.state as WorkspaceContributionsFailed).error,
        isA<FormatException>(),
      );
    },
  );

  for (final routes in [
    [_route('owner.other')],
    [_route('owner.first'), _route('owner.first')],
  ]) {
    test(
      'section preview requires exactly one route of its actual owner $routes',
      () async {
        final bridge = _WorkspaceCatalogBridge(
          owners: ['owner.first'],
          routes: routes,
          requests: [
            _completedSections([
              _sectionData('opaque', ['a']),
            ]),
          ],
        );
        final controller = WorkspaceOverviewContributionCatalog();
        addTearDown(controller.dispose);
        await controller.refresh(
          catalog: _catalog(bridge),
          chats: [_chat('a', 'workspace-a')],
        );
        expect(
          (controller.state as WorkspaceContributionsFailed).error,
          isA<StateError>(),
        );
      },
    );
  }

  test(
    'disabling a registered plugin removes its usages on the next real refresh',
    () async {
      final bridge = _WorkspaceCatalogBridge(
        owners: ['owner.first'],
        routes: [_route('owner.first')],
        requests: [
          _completedSections([
            _sectionData('opaque', ['a']),
          ]),
        ],
      );
      final controller = WorkspaceOverviewContributionCatalog();
      addTearDown(controller.dispose);
      await controller.refresh(
        catalog: _catalog(bridge),
        chats: [_chat('a', 'workspace-a')],
      );
      expect(_ready(controller).usages, hasLength(1));
      bridge.owners.clear();
      bridge.routes.clear();
      await controller.refresh(
        catalog: _catalog(bridge),
        chats: [_chat('a', 'workspace-a')],
      );
      expect(_ready(controller).usages, isEmpty);
      expect(bridge.payloads, hasLength(1));
    },
  );

  for (final oldFails in [false, true]) {
    test(
      'older ${oldFails ? 'failure' : 'success'} cannot replace a newer workspace result',
      () async {
        final old = _SectionsRequest();
        final current = _SectionsRequest();
        final bridge = _WorkspaceCatalogBridge(
          owners: ['owner.first'],
          routes: [_route('owner.first')],
          requests: [old, current],
        );
        final controller = WorkspaceOverviewContributionCatalog();
        addTearDown(controller.dispose);
        final oldRefresh = controller.refresh(
          catalog: _catalog(bridge),
          chats: [_chat('a', 'workspace-a')],
        );
        await old.started.future;
        final currentRefresh = controller.refresh(
          catalog: _catalog(bridge),
          chats: [_chat('b', 'workspace-b')],
        );
        await current.started.future;
        current.data.complete({
          'sections': [
            _sectionData('current', ['b']),
          ],
        });
        await currentRefresh;
        final state = controller.state;
        if (oldFails) {
          old.data.completeError(StateError('Old request failed'));
        } else {
          old.data.complete({
            'sections': [
              _sectionData('old', ['a']),
            ],
          });
        }
        await oldRefresh;
        expect(controller.state, same(state));
        expect(_ready(controller).usages.single.sectionId, 'current');
      },
    );
  }

  test(
    'a newer failure cannot be replaced by an older successful catalog',
    () async {
      final old = _SectionsRequest();
      final current = _SectionsRequest();
      final bridge = _WorkspaceCatalogBridge(
        owners: ['owner.first'],
        routes: [_route('owner.first')],
        requests: [old, current],
      );
      final controller = WorkspaceOverviewContributionCatalog();
      addTearDown(controller.dispose);
      final oldRefresh = controller.refresh(
        catalog: _catalog(bridge),
        chats: [_chat('a', 'workspace-a')],
      );
      await old.started.future;
      final currentRefresh = controller.refresh(
        catalog: _catalog(bridge),
        chats: [_chat('b', 'workspace-b')],
      );
      await current.started.future;
      final error = StateError('Current request failed');
      current.data.completeError(error);
      await currentRefresh;
      final state = controller.state;
      old.data.complete({
        'sections': [
          _sectionData('old', ['a']),
        ],
      });
      await oldRefresh;
      expect(controller.state, same(state));
      expect((state as WorkspaceContributionsFailed).error, same(error));
    },
  );

  for (final fails in [false, true]) {
    test(
      'dispose rejects a pending ${fails ? 'failure' : 'success'} without notifying',
      () async {
        final request = _SectionsRequest();
        final bridge = _WorkspaceCatalogBridge(
          owners: ['owner.first'],
          routes: [_route('owner.first')],
          requests: [request],
        );
        final controller = WorkspaceOverviewContributionCatalog();
        var notifications = 0;
        controller.addListener(() => notifications++);
        final refresh = controller.refresh(
          catalog: _catalog(bridge),
          chats: [_chat('a', 'workspace-a')],
        );
        await request.started.future;
        controller.dispose();
        if (fails) {
          request.data.completeError(StateError('Disposed request failed'));
        } else {
          request.data.complete({
            'sections': [
              _sectionData('opaque', ['a']),
            ],
          });
        }
        await refresh;
        expect(notifications, 1);
        expect(() => controller.awaitHistories(), throwsStateError);
      },
    );
  }

  test(
    'history errors invalidate pending catalog work and stay visible',
    () async {
      final request = _SectionsRequest();
      final bridge = _WorkspaceCatalogBridge(
        owners: ['owner.first'],
        routes: [_route('owner.first')],
        requests: [request],
      );
      final controller = WorkspaceOverviewContributionCatalog();
      addTearDown(controller.dispose);
      final refresh = controller.refresh(
        catalog: _catalog(bridge),
        chats: [_chat('a', 'workspace-a')],
      );
      await request.started.future;
      final error = StateError('History stream failed');
      final stackTrace = StackTrace.current;
      controller.reportError(error, stackTrace);
      request.data.complete({
        'sections': [
          _sectionData('old', ['a']),
        ],
      });
      await refresh;
      final state = controller.state as WorkspaceContributionsFailed;
      expect(state.error, same(error));
      expect(state.stackTrace, same(stackTrace));
      controller.awaitHistories();
      expect(controller.state, isA<WorkspaceContributionsLoading>());
    },
  );

  test(
    'catalog change bus reloads the actual catalog until its subscription is cancelled',
    () async {
      final request = _SectionsRequest();
      final bridge = _WorkspaceCatalogBridge(
        owners: ['owner.first'],
        routes: [_route('owner.first')],
        requests: [request],
      );
      final controller = WorkspaceOverviewContributionCatalog();
      addTearDown(controller.dispose);
      final finished = Completer<void>();
      final subscription = ToolPkgCatalogChangeBus.listen(() {
        unawaited(
          controller
              .refresh(
                catalog: _catalog(bridge),
                chats: [_chat('a', 'workspace-a')],
              )
              .then((_) => finished.complete()),
        );
      });
      addTearDown(subscription.cancel);
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await request.started.future;
      request.data.complete({
        'sections': [
          _sectionData('opaque', ['a']),
        ],
      });
      await finished.future;
      expect(_ready(controller).usages.single.sectionId, 'opaque');
      await subscription.cancel();
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await Future<void>.delayed(Duration.zero);
      expect(bridge.payloads, hasLength(1));
    },
  );

  testWidgets(
    'home retains host counts and shows contribution errors explicitly',
    (tester) async {
      await _mountHome(
        tester,
        WorkspaceContributionsFailed(
          StateError('Exact catalog failure'),
          StackTrace.current,
        ),
      );
      expect(find.text('Same display name'), findsOneWidget);
      expect(find.text('2 个对话 · 贡献项加载失败 · 1 个文件夹'), findsOneWidget);
      expect(
        find.text('工作区贡献项加载失败：Bad state: Exact catalog failure'),
        findsOneWidget,
      );
      expect(find.text('3 个终端'), findsOneWidget);
      expect(find.text('2 个浏览器'), findsOneWidget);
    },
  );

  testWidgets(
    'home preserves compact clickable avatars with owner-scoped identities',
    (tester) async {
      final usages = workspaceContributionUsages([
        _section('owner::first', 'opaque', ['a']),
        _section('owner', 'first::opaque', ['a']),
      ]);
      await _mountHome(tester, WorkspaceContributionsReady(usages));
      expect(find.text('2 个对话 · 2 个贡献项 · 1 个文件夹'), findsOneWidget);
      for (final usage in usages) {
        final avatar = find.byKey(ValueKey(usage.identity));
        expect(avatar, findsOneWidget);
        expect(
          find.descendant(of: avatar, matching: find.byType(InkWell)),
          findsOneWidget,
        );
      }
      expect(find.byIcon(Icons.extension_outlined), findsNWidgets(2));
    },
  );

  testWidgets(
    'home distinguishes pending contributions from a successful empty catalog',
    (tester) async {
      await _mountHome(tester, const WorkspaceContributionsLoading());
      expect(find.text('正在读取工作区贡献项'), findsOneWidget);
      expect(find.text('2 个对话 · 贡献项读取中 · 1 个文件夹'), findsOneWidget);
      await _mountHome(tester, const WorkspaceContributionsReady([]));
      expect(find.text('正在读取工作区贡献项'), findsNothing);
      expect(find.text('2 个对话 · 0 个贡献项 · 1 个文件夹'), findsOneWidget);
    },
  );

  testWidgets('preview action revalidates disabled and foreign-owned routes', (
    tester,
  ) async {
    final bridge = _WorkspaceCatalogBridge(
      owners: ['owner.first'],
      routes: [_route('owner.first')],
      requests: [
        _completedSections([
          _sectionData('opaque', ['a']),
        ]),
      ],
    );
    final sections = await _catalog(
      bridge,
    ).loadSections(chats: [_chat('a', 'workspace-a')]);
    final usage = WorkspaceContributionUsage.fromSection(sections.single);
    late BuildContext presentationContext;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) {
            presentationContext = context;
            return const SizedBox.shrink();
          },
        ),
      ),
    );
    bridge.owners.clear();
    bridge.routes.clear();
    await expectLater(
      usage.preview.present(
        context: presentationContext,
        clients: GeneratedCoreProxyClients(bridge),
      ),
      throwsStateError,
    );
    bridge.routes.add(_route('owner.other'));
    await expectLater(
      usage.preview.present(
        context: presentationContext,
        clients: GeneratedCoreProxyClients(bridge),
      ),
      throwsStateError,
    );
  });
}

const _opaqueInput = <String, Object?>{
  'opaque': <Object?>[
    'owner-controlled',
    7,
    <String, Object?>{'token': 'a::b/c'},
  ],
};

/// Creates a generic history summary without constructing domain records.
core.ChatHistoryListItem _chat(String id, String? workspaceId) =>
    core.ChatHistoryListItem.fromJson(<String, Object?>{
      'id': id,
      'title': 'Chat $id',
      'updatedAt': '2026-10-08T12:00:00Z',
      'displayOrder': 0,
      'workspaceId': workspaceId,
      'workspaceName': workspaceId == null ? null : 'Same display name',
      'locked': false,
      'pinned': false,
    });

/// Registers a preview route with an intentionally shared route ID across owners.
core.ToolPkgUiRoute _route(String owner) => core.ToolPkgUiRoute(
  containerPackageName: owner,
  toolPkgId: 'display',
  routeId: 'shared-preview',
  uiModuleId: 'preview',
  runtime: 'compose_dsl',
  screen: 'opaque-preview',
  title: 'Published preview',
  description: '',
  moduleSpec: const {},
  keepAlive: false,
);

/// Supplies the exact public section fields and leaves selection and preview opaque.
Map<String, Object?> _sectionData(String id, List<String> chatIds) => {
  'id': id,
  'title': 'Same contribution title',
  'avatarUri': null,
  'chatIds': chatIds,
  'selection': _opaqueInput,
  'preview': {'routeId': 'shared-preview', 'input': _opaqueInput},
};

/// Creates an already validated section for display-only aggregation assertions.
ChatUiSection _section(String owner, String id, List<String> chatIds) =>
    ChatUiSection(
      ownerPackageName: owner,
      id: id,
      title: 'Same contribution title',
      avatarUri: null,
      chatIds: chatIds,
      selection: _opaqueInput,
      preview: ChatUiAction(route: _route(owner), input: _opaqueInput),
    );

/// Prepares a deterministic public API result while preserving async invocation.
_SectionsRequest _completedSections(List<Map<String, Object?>> sections) {
  final request = _SectionsRequest();
  request.data.complete({'sections': sections});
  return request;
}

/// Instantiates the production catalog on a codec-aware runtime fixture.
ToolPkgChatUiCatalog _catalog(_WorkspaceCatalogBridge bridge) =>
    ToolPkgChatUiCatalog(clients: GeneratedCoreProxyClients(bridge));

/// Requires the controller to have a successful state for the current request.
WorkspaceContributionsReady _ready(
  WorkspaceOverviewContributionCatalog controller,
) => controller.state as WorkspaceContributionsReady;

/// Mounts the production overview without starting unrelated native host interactions.
Future<void> _mountHome(
  WidgetTester tester,
  WorkspaceContributionsState state,
) async {
  final terminals = ValueNotifier<int>(3);
  final browsers = ValueNotifier<int>(2);
  addTearDown(terminals.dispose);
  addTearDown(browsers.dispose);
  await tester.pumpWidget(
    OperitTheme(
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
          body: WorkspaceHomeContent(
            workspacePath: '/workspace/project',
            workspaceUsage: WorkspaceOverviewUsage(
              workspaceName: 'Same display name',
              conversationCount: 2,
              contributions: state,
              mountedFolders: const [
                WorkspaceMountedFolder(
                  name: 'Mounted folder',
                  path: '/mnt/data',
                  relativePath: 'data',
                ),
              ],
              mountedFoldersLoading: false,
              mountedFoldersError: null,
            ),
            terminalSessionCountListenable: terminals,
            browserSessionCountListenable: browsers,
            onOpenFolder: _ignoreFolder,
            onAddFolder: _noop,
            onCreateWorkspace: _noop,
            onChooseExistingWorkspace: _noop,
            onUnbindWorkspace: _noop,
            onOpenTerminal: _noop,
            onOpenTerminalSessions: _noop,
            onOpenBrowserSessions: _noop,
            onOpenBrowser: _noop,
          ),
        ),
      ),
    ),
  );
  await tester.pump();
}

/// Leaves unrelated workspace actions inert in overview presentation tests.
void _noop() {}

/// Leaves mounted-folder navigation inert in overview presentation tests.
void _ignoreFolder(WorkspaceMountedFolder folder) {}

/// Holds explicit invocation and completion gates for stale-request tests.
class _SectionsRequest {
  /// Creates independent gates for one actual catalog invocation.
  _SectionsRequest();

  final started = Completer<void>();
  final data = Completer<Object?>();
}

/// Implements only the canonical public catalog methods used by this slice.
class _WorkspaceCatalogBridge extends OperitRuntimeBridge {
  /// Configures exact registered owners, routes, and ordered API results.
  _WorkspaceCatalogBridge({
    required this.owners,
    required this.routes,
    required List<_SectionsRequest> requests,
  }) : _requests = Queue<_SectionsRequest>.of(requests);

  final List<String> owners;
  final List<core.ToolPkgUiRoute> routes;
  final Queue<_SectionsRequest> _requests;
  final List<CoreCallRequest> calls = [];
  final List<Map<String, Object?>> payloads = [];
  Object? discoveryError;
  Object? routesError;
  Object? invocationError;

  /// Sends actual generated-proxy payloads through the production Link codec.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map<String, Object?>;
    Object? value;
    switch (request.methodName) {
      case 'getToolPkgPublicApiOwners':
        if (discoveryError case final Object error) throw error;
        if (args['apiName'] != 'chat.list.sections') {
          throw StateError('Unexpected public API discovery.');
        }
        value = [
          for (final owner in owners)
            core.ToolPkgPublicApiOwner(
              containerPackageName: owner,
              apiName: 'chat.list.sections',
            ).toJson(),
        ];
      case 'getToolPkgUiRoutes':
        if (routesError case final Object error) throw error;
        if (args['runtime'] != 'compose_dsl') {
          throw StateError('Unexpected presentation runtime.');
        }
        value = routes.map((route) => route.toJson()).toList();
      case 'invokeToolPkgPublicApi':
        if (invocationError case final Object error) throw error;
        if (args['methodName'] != 'chat.list.sections' ||
            !owners.any((owner) => owner == args['packageName'])) {
          throw StateError(
            'Invocation must target an exact enabled API owner.',
          );
        }
        payloads.add(Map<String, Object?>.from(args['payload'] as Map));
        final invocation = _requests.removeFirst();
        invocation.started.complete();
        value = await invocation.data.future;
      default:
        throw StateError(
          'Unexpected workspace catalog method: ${request.methodName}',
        );
    }
    return encodeCoreLink([0, value]);
  }

  /// Rejects push operations because an overview cannot upload plugin records.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw StateError('Unexpected workspace catalog push.');

  /// Rejects snapshot operations because this fixture covers ordinary catalog calls.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw StateError('Unexpected workspace catalog snapshot.');

  /// Rejects unrelated stream operations rather than fabricating runtime state.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw StateError('Unexpected workspace catalog stream.');
}
