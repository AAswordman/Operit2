import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/contributions/ContributionPresentationResult.dart';
import 'package:operit2/ui/common/contributions/ToolPkgChatUiCatalog.dart';
import 'package:operit2/ui/features/chat/components/style/input/agent/AgentInputMenuPopup.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatSelectionTransition.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';

/// Covers the real shared menu, registered DSL actions and exact chat refresh ownership.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(ClientLogger.initialize);

  test('both actual composer styles mount the shared guarded input menu', () {
    for (final (style, name) in [('agent', 'Agent'), ('classic', 'Classic')]) {
      final source = File(
        'lib/ui/features/chat/components/style/input/$style/'
        '${name}ChatInputSection.dart',
      ).readAsStringSync();
      expect(source, matches(RegExp(r'child: AgentInputMenuPopup\(')));
      expect(
        source,
        matches(RegExp(r'isChatContextCurrent: isChatContextCurrent,')),
      );
      expect(
        source,
        matches(RegExp(r'final generation = \+\+_inputMenuContextGeneration;')),
      );
      expect(source, matches(RegExp(r'widget.currentChatId == chatId')));
    }
  });

  test(
    'catalog forwards null new-session state to every actual owner',
    () async {
      final bridge = _ContextBridge(currentChatId: null, publishIdentity: true);
      addTearDown(bridge.close);
      final catalog = ToolPkgChatUiCatalog(
        clients: GeneratedCoreProxyClients(bridge),
      );
      final context = await catalog.loadContext(chatId: null);
      expect(context.selectors, hasLength(2));
      expect(
        context.selectors.map((item) => item.action.route.containerPackageName),
        ['example.selector', 'example.other'],
      );
      final invocations = bridge.calls.where(
        (call) => call.methodName == 'invokeToolPkgPublicApi',
      );
      for (final call in invocations) {
        expect((call.args as Map)['payload'], {'chatId': null});
      }
    },
  );

  test(
    'broken existing namespace is not replaced by active selection',
    () async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: false,
      )..contextError = StateError('Invalid persisted namespace association');
      addTearDown(bridge.close);
      await expectLater(
        ToolPkgChatUiCatalog(
          clients: GeneratedCoreProxyClients(bridge),
        ).loadContext(chatId: 'chat-a'),
        throwsA(isA<StateError>()),
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'chatConfiguration'),
        isEmpty,
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'getActivePrompt'),
        isEmpty,
      );
    },
  );

  testWidgets(
    'unconfigured existing chat keeps selectors without an identity',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: false,
      );
      addTearDown(bridge.close);
      await tester.pumpWidget(_app(bridge));
      await tester.pumpAndSettle();
      expect(find.text('Choose example.selector'), findsOneWidget);
      expect(find.text('Choose example.other'), findsOneWidget);
      expect(find.text('Published identity'), findsNothing);
      expect(bridge.configurationCalls, isEmpty);
      await _removeHost(tester);
    },
  );

  testWidgets(
    'statistics failure does not remove unconfigured chat selectors',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: false,
      )..statisticsError = StateError('No configured model for statistics');
      addTearDown(bridge.close);
      await tester.pumpWidget(_app(bridge));
      await tester.pumpAndSettle();
      expect(find.text('Choose example.selector'), findsOneWidget);
      expect(find.textContaining('统计加载失败：'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await _removeHost(tester);
    },
  );

  testWidgets(
    'opaque route input and real chatId reach separate host state fields',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: true,
      );
      addTearDown(bridge.close);
      await _openSelector(tester, bridge);
      expect(bridge.renderStates.single['chatId'], 'chat-a');
      final presentation = bridge.renderStates.single['presentation'] as Map;
      expect(presentation['input'], _opaqueInput('example.selector'));
      expect(presentation['requestId'], isA<String>());
      expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
      await tester.tap(find.text('Cancel choice'));
      await tester.pumpAndSettle();
      expect(bridge.configurationCalls, isEmpty);
      await _removeHost(tester);
    },
  );

  testWidgets(
    'one click waits for durable commit then refreshes exact owning Core before broadcast',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: true,
      )..commitGate = Completer<void>();
      addTearDown(bridge.close);
      final subscription = ToolPkgCatalogChangeBus.listen(
        () => bridge.events.add('catalog'),
      );
      addTearDown(subscription.cancel);
      await _openSelector(tester, bridge);
      expect(find.byType(AgentInputMenuPopup), findsNothing);
      await tester.tap(find.text('Commit choice'));
      await tester.pump();
      expect(bridge.configurationCalls, isEmpty);
      expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
      bridge.commitGate!.complete();
      await tester.pumpAndSettle();
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
      expect(bridge.configurationCalls, hasLength(1));
      final call = bridge.configurationCalls.single;
      expect(call.target, 'core/chatRuntimeHolderMain');
      expect(call.args, {'chatId': 'chat-a'});
      expect(
        bridge.events,
        containsAllInOrder([
          'durable.commit',
          'configuration:chat-a',
          'catalog',
        ]),
      );
      await _removeHost(tester);
    },
  );

  testWidgets('explicit cancel does not resolve or publish any configuration', (
    tester,
  ) async {
    final bridge = _ContextBridge(
      currentChatId: 'chat-a',
      publishIdentity: true,
    );
    addTearDown(bridge.close);
    var notifications = 0;
    final subscription = ToolPkgCatalogChangeBus.listen(() => notifications++);
    addTearDown(subscription.cancel);
    await _openSelector(tester, bridge);
    await tester.tap(find.text('Cancel choice'));
    await tester.pumpAndSettle();
    expect(bridge.durableCommitted, isFalse);
    expect(bridge.configurationCalls, isEmpty);
    expect(notifications, 0);
    await _removeHost(tester);
  });

  testWidgets(
    'ordinary results and another request cannot complete this dialog',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: true,
      );
      addTearDown(bridge.close);
      await _openSelector(tester, bridge);
      for (final action in ['Ordinary preview', 'Wrong request']) {
        await tester.tap(find.text(action));
        await tester.pumpAndSettle();
        expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
        expect(bridge.configurationCalls, isEmpty);
      }
      await tester.tap(find.text('Cancel choice'));
      await tester.pumpAndSettle();
      await _removeHost(tester);
    },
  );

  testWidgets(
    'pending selection expires completion even when the committed chat ID stays equal',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: true,
      );
      addTearDown(bridge.close);
      await _openSelector(tester, bridge);
      ChatSelectionTransition.begin('chat-b');
      ChatSelectionTransition.complete('chat-b');
      await tester.tap(find.text('Commit choice'));
      await tester.pumpAndSettle();
      expect(bridge.currentChatId, 'chat-a');
      expect(bridge.configurationCalls, isEmpty);
      await _removeHost(tester);
    },
  );

  testWidgets('committed A to B to A cannot revive a stale presentation', (
    tester,
  ) async {
    final bridge = _ContextBridge(
      currentChatId: 'chat-a',
      publishIdentity: true,
    );
    addTearDown(bridge.close);
    await _openSelector(tester, bridge);
    bridge.select('chat-b');
    bridge.select('chat-a');
    await tester.pump();
    await tester.tap(find.text('Commit choice'));
    await tester.pumpAndSettle();
    expect(bridge.currentChatId, 'chat-a');
    expect(bridge.configurationCalls, isEmpty);
    await _removeHost(tester);
  });

  testWidgets(
    'disposed composer cannot publish after its already dismissed menu completes',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: true,
      );
      addTearDown(bridge.close);
      await _openSelector(tester, bridge);
      await tester.pumpWidget(_app(bridge, mountOwner: false));
      await tester.pumpAndSettle();
      expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
      await tester.tap(find.text('Commit choice'));
      await tester.pumpAndSettle();
      expect(bridge.configurationCalls, isEmpty);
      await _removeHost(tester);
    },
  );

  testWidgets(
    'chat switch during profile refresh never targets the newly selected chat',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: true,
      )..configurationGate = Completer<void>();
      addTearDown(bridge.close);
      var notifications = 0;
      final subscription = ToolPkgCatalogChangeBus.listen(
        () => notifications++,
      );
      addTearDown(subscription.cancel);
      await _openSelector(tester, bridge);
      await tester.tap(find.text('Commit choice'));
      await tester.pumpAndSettle();
      expect(bridge.configurationCalls.single.args, {'chatId': 'chat-a'});
      bridge.select('chat-b');
      bridge.configurationGate!.complete();
      await tester.pumpAndSettle();
      expect(bridge.configurationCalls, hasLength(1));
      expect(notifications, 0);
      await _removeHost(tester);
    },
  );

  testWidgets(
    'configuration failure is explicit and does not broadcast success',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: true,
      )..configurationError = StateError('Saved configuration is invalid');
      addTearDown(bridge.close);
      var notifications = 0;
      final subscription = ToolPkgCatalogChangeBus.listen(
        () => notifications++,
      );
      addTearDown(subscription.cancel);
      await _openSelector(tester, bridge);
      await tester.tap(find.text('Commit choice'));
      await tester.pumpAndSettle();
      expect(find.textContaining('插件操作失败：'), findsOneWidget);
      expect(notifications, 0);
      await _removeHost(tester);
    },
  );

  testWidgets(
    'null new-session completion publishes active change without fabricating a chat profile',
    (tester) async {
      final bridge = _ContextBridge(currentChatId: null, publishIdentity: true);
      addTearDown(bridge.close);
      var notifications = 0;
      final subscription = ToolPkgCatalogChangeBus.listen(
        () => notifications++,
      );
      addTearDown(subscription.cancel);
      await _openSelector(tester, bridge);
      expect(bridge.renderStates.single['chatId'], isNull);
      await tester.tap(find.text('Commit choice'));
      await tester.pumpAndSettle();
      expect(bridge.durableCommitted, isTrue);
      expect(bridge.configurationCalls, isEmpty);
      expect(notifications, 1);
      await _removeHost(tester);
    },
  );

  testWidgets(
    'disabled registered route is rejected before mounting its selector',
    (tester) async {
      final bridge = _ContextBridge(
        currentChatId: 'chat-a',
        publishIdentity: true,
      );
      addTearDown(bridge.close);
      await tester.pumpWidget(_app(bridge));
      await tester.pumpAndSettle();
      bridge.routes.clear();
      await tester.tap(find.text('Choose example.selector'));
      await tester.pumpAndSettle();
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
      expect(find.textContaining('插件操作失败：'), findsOneWidget);
      expect(bridge.configurationCalls, isEmpty);
      await _removeHost(tester);
    },
  );
}

/// Mounts a real shared menu under a root navigator without loading a plugin WebView.
Widget _app(_ContextBridge bridge, {bool mountOwner = true}) => MaterialApp(
  locale: const Locale('zh'),
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(
    body: mountOwner ? _MenuOwner(bridge: bridge) : const SizedBox.shrink(),
  ),
);

/// Removes the host and its menu timers without running native assets bootstrap.
Future<void> _removeHost(WidgetTester tester) async {
  await tester.pumpWidget(const SizedBox.shrink());
  await tester.pumpAndSettle();
}

/// Opens an actual catalog selector through the production shared menu renderer.
Future<void> _openSelector(WidgetTester tester, _ContextBridge bridge) async {
  await tester.pumpWidget(_app(bridge));
  await tester.pumpAndSettle();
  await tester.tap(find.text('Choose example.selector'));
  await tester.pumpAndSettle();
  expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
  expect(find.byType(Dialog), findsOneWidget);
  expect(find.byType(AlertDialog), findsNothing);
}

/// Keeps the dialog owner alive after removal of the menu overlay.
class _MenuOwner extends StatefulWidget {
  /// Captures the same owning runtime used by both production composers.
  const _MenuOwner({required this.bridge});

  final _ContextBridge bridge;

  /// Creates a lifetime distinct from the dismissed input popup.
  @override
  State<_MenuOwner> createState() => _MenuOwnerState();
}

class _MenuOwnerState extends State<_MenuOwner> {
  late final ChatViewModel _viewModel;
  bool _menuVisible = true;

  /// Binds the actual generated chat proxy for this menu instance.
  @override
  void initState() {
    super.initState();
    _viewModel = ChatViewModel(bridge: widget.bridge);
  }

  /// Preserves the composer owner while dismissing its actual popup widget.
  @override
  Widget build(BuildContext context) => Align(
    alignment: Alignment.bottomCenter,
    child: _menuVisible
        ? AgentInputMenuPopup(
            viewModel: _viewModel,
            currentChatId: widget.bridge.currentChatId,
            isChatContextCurrent: () => mounted,
            onDismiss: () => setState(() => _menuVisible = false),
          )
        : const SizedBox.shrink(),
  );
}

/// Preserves deliberately opaque plugin input without host selection fields.
Map<String, Object?> _opaqueInput(String owner) => {
  'plugin-note': owner,
  'nested': [
    null,
    {'unchanged': 'plugin-owned'},
  ],
};

/// Describes a registered pure DSL route owned by an arbitrary test package.
core_proxy.ToolPkgUiRoute _route(String owner) => core_proxy.ToolPkgUiRoute(
  containerPackageName: owner,
  toolPkgId: owner,
  routeId: 'selector-route',
  uiModuleId: 'selector-route',
  runtime: 'compose_dsl',
  screen: 'ui/selector.js',
  title: 'Selector for $owner',
  description: '',
  moduleSpec: const {},
  keepAlive: false,
);

/// Supplies only generic display fields from a registered public API owner.
Map<String, Object?> _contextActions(
  String owner, {
  required bool publishIdentity,
}) => {
  'selectors': [
    {
      'id': 'selector',
      'title': 'Choose $owner',
      'icon': null,
      'routeId': 'selector-route',
      'input': _opaqueInput(owner),
    },
  ],
  'identity': publishIdentity && owner == 'example.selector'
      ? {
          'title': 'Published identity',
          'avatarUri': null,
          'action': {'routeId': 'selector-route', 'input': _opaqueInput(owner)},
        }
      : null,
  'backgroundUri': null,
};

/// Implements real proxy codecs and catalog/DSL events, never alternate production providers.
class _ContextBridge extends OperitRuntimeBridge {
  /// Requires explicit independent chat selection and plugin display state.
  _ContextBridge({required this.currentChatId, required this.publishIdentity})
    : routes = [_route('example.selector'), _route('example.other')];

  String? currentChatId;
  final bool publishIdentity;
  final List<core_proxy.ToolPkgUiRoute> routes;
  final List<CoreCallRequest> calls = [];
  final List<Map<String, Object?>> renderStates = [];
  final List<String> events = [];
  final StreamController<String?> _selections =
      StreamController<String?>.broadcast();
  Completer<void>? commitGate;
  Completer<void>? configurationGate;
  Object? contextError;
  Object? configurationError;
  Object? statisticsError;
  bool durableCommitted = false;

  /// Retains exact authoritative profile-refresh requests for ownership assertions.
  List<CoreCallRequest> get configurationCalls =>
      calls.where((call) => call.methodName == 'chatConfiguration').toList();

  /// Emits real committed selection changes, including round trips to the same ID.
  void select(String? chatId) {
    currentChatId = chatId;
    _selections.add(chatId);
  }

  /// Releases the fixture's long-lived selected-chat stream after host disposal.
  Future<void> close() => _selections.close();

  /// Handles only actual catalog, native menu, render and configuration methods.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map<String, Object?>;
    Object? value;
    switch (request.methodName) {
      case 'chatInputMenuSettings':
        value = const core_proxy.ChatInputMenuSettings(
          enableMemoryAutoUpdate: false,
          permissionMode: core_proxy.AiPermissionMode.full,
          disableStreamOutput: false,
          disableUserPreferenceDescription: false,
          pluginChangeVersion: 0,
          pluginToggles: [],
        ).toJson();
      case 'chatInputMenuSummary':
        if (statisticsError case final Object error) throw error;
        value = const core_proxy.ChatInputMenuSummary(
          currentWindowSize: 0,
          inputTokenCount: 0,
          outputTokenCount: 0,
          maxContextLength: 32,
        ).toJson();
      case 'getToolPkgPublicApiOwners':
        if (args['apiName'] != 'chat.context.actions') {
          throw StateError('Unexpected API discovery');
        }
        value = [
          for (final owner in ['example.selector', 'example.other'])
            core_proxy.ToolPkgPublicApiOwner(
              containerPackageName: owner,
              apiName: 'chat.context.actions',
            ).toJson(),
        ];
      case 'invokeToolPkgPublicApi':
        if (args['methodName'] != 'chat.context.actions') {
          throw StateError('Unexpected API invocation');
        }
        if (contextError case final Object error) throw error;
        value = _contextActions(
          args['packageName'] as String,
          publishIdentity: publishIdentity,
        );
      case 'getToolPkgNavigationEntries':
        value = const [];
      case 'getToolPkgUiRoutes':
        value = routes.map((route) => route.toJson()).toList();
      case 'getToolPkgContainerRuntime':
        value = _runtime(args['containerPackageName'] as String).toJson();
      case 'acquireToolPkgExecutionEngine':
      case 'releaseToolPkgExecutionEngine':
        value = null;
      case 'readToolPkgTextResource':
        value = 'export default function render() {}';
      case 'executeToolPkgComposeDslScript':
        final state =
            (args['runtimeOptions'] as Map)['state'] as Map<String, Object?>;
        renderStates.add(Map<String, Object?>.from(state));
        value = _render(state);
      case 'chatConfiguration':
        if (!durableCommitted) {
          throw StateError('Configuration read before durable commit');
        }
        if (configurationError case final Object error) throw error;
        events.add('configuration:${args['chatId']}');
        await configurationGate?.future;
        value = core_proxy.ChatConfigurationResult(
          contextKey: 'opaque-runtime-context',
          profile: _profile(),
          participants: [_profile()],
        ).toJson();
      default:
        throw StateError('Unexpected method: ${request.methodName}');
    }
    return encodeCoreLink([0, value]);
  }

  /// Rejects uploads because input selection is not an attachment workflow.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw StateError('Unexpected push');

  /// Rejects snapshot-based alternate bindings explicitly.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw StateError('Unexpected snapshot');

  /// Replays selected-chat state and routes real registered DSL action events.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName == 'currentChatIdFlow') {
      return Stream<CoreEvent>.multi((controller) {
        final subscription = _selections.stream.listen(
          (chatId) => controller.add(_event(request, chatId)),
          onError: controller.addError,
          onDone: controller.close,
        );
        controller.onCancel = subscription.cancel;
        controller.add(_event(request, currentChatId));
      });
    }
    if (request.propertyName == 'dispatchToolPkgComposeDslActionEvents') {
      return _dispatchAction(request);
    }
    throw StateError('Unexpected watch: ${request.propertyName}');
  }

  /// Emits completion only after the simulated plugin durable commit has finished.
  Stream<CoreEvent> _dispatchAction(CoreWatchRequest request) async* {
    final args = request.args as Map<String, Object?>;
    final state =
        (args['runtimeOptions'] as Map)['state'] as Map<String, Object?>;
    final requestId = (state['presentation'] as Map)['requestId'];
    Object? result;
    switch (args['actionId']) {
      case 'commit':
        await commitGate?.future;
        durableCommitted = true;
        events.add('durable.commit');
        result = {
          'type': ContributionPresentationResult.completeType,
          'requestId': requestId,
          'value': true,
        };
      case 'cancel':
        result = {
          'type': ContributionPresentationResult.cancelType,
          'requestId': requestId,
        };
      case 'wrong':
        result = {
          'type': ContributionPresentationResult.completeType,
          'requestId': 'unowned-request',
          'value': true,
        };
      case 'ordinary':
        result = {'preview': true};
      default:
        throw StateError('Unexpected action: ${args['actionId']}');
    }
    yield _event(request, {
      'phase': 'final',
      'result': _render(state, result: result),
    });
    yield _event(request, {'phase': 'complete'});
  }
}

/// Supplies the full runtime descriptor consumed by the real generated decoder.
core_proxy.ChatParticipantProfile _profile() =>
    const core_proxy.ChatParticipantProfile(
      id: 'saved-participant',
      name: 'Persisted participant',
      avatarUri: null,
      introPrompt: '',
      userPreferencesText: '',
      openingStatement: '',
      modelBinding: core_proxy.ProviderFunctionModelBinding(
        providerId: 'provider',
        modelId: 'model',
      ),
      ttsConfigId: 'saved-voice',
      toolAccess: core_proxy.ChatToolAccess(
        enabled: false,
        allowedBuiltinTools: [],
        allowedPackages: [],
        allowedSkills: [],
        allowedMcpServers: [],
      ),
      resources: [],
    );

/// Renders an actual embedded DSL dialog with explicit V1 cancellation and actions.
String _render(Map<String, Object?> state, {Object? result}) => jsonEncode({
  'success': true,
  'tree': {
    'type': 'Dialog',
    'props': {
      'closeOnDismissRequest': false,
      'properties': {'dismissOnBackPress': true, 'dismissOnClickOutside': true},
      'onDismissRequest': {'__actionId': 'cancel'},
    },
    'children': [
      {
        'type': 'Column',
        'props': {'width': 360, 'height': 320},
        'children': [
          for (final entry in <String, String>{
            'commit': 'Commit choice',
            'cancel': 'Cancel choice',
            'wrong': 'Wrong request',
            'ordinary': 'Ordinary preview',
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
    ],
    'slots': <String, Object?>{},
  },
  'state': state,
  'memo': <String, Object?>{},
  'actionResult': result,
});

/// Encodes a real selected-chat snapshot or serialized DSL action event.
CoreEvent _event(CoreWatchRequest request, Object? value) => CoreEvent.raw(
  requestId: request.requestId,
  target: request.target,
  propertyName: request.propertyName,
  kind: 'Changed',
  decodeValue: decodeCoreLink<Object?>,
  valueBytes: encodeCoreLink(
    request.propertyName == 'currentChatIdFlow' ? value : jsonEncode(value),
  ),
);

/// Creates complete registered container metadata for the existing embedded launcher.
core_proxy.ToolPkgContainerRuntime _runtime(String owner) =>
    core_proxy.ToolPkgContainerRuntime(
      packageName: owner,
      displayName: const core_proxy.LocalizedText(
        values: {'default': 'Generic test package'},
      ),
      description: const core_proxy.LocalizedText(
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
      sourceType: core_proxy.ToolPkgSourceType.externalValue,
      sourcePath: 'test',
      subpackages: const [],
      resources: const [],
      wasmModules: const [],
      workflowTemplates: const [],
      workspaceTemplates: const [],
      uiModules: const [],
      uiRoutes: const [
        core_proxy.ToolPkgUiRouteRuntime(
          id: 'selector-route',
          routeId: 'selector-route',
          runtime: 'compose_dsl',
          screen: 'ui/selector.js',
          title: core_proxy.LocalizedText(values: {'default': 'Selection'}),
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
