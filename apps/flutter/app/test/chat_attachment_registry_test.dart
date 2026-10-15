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
import 'package:operit2/ui/common/contributions/ChatAttachmentResult.dart';
import 'package:operit2/ui/common/contributions/ContributionPresentationResult.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatAttachmentMenuPopup.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

import 'support/compose_session_fixture.dart';

/// Verifies registry-backed plus-menu presentation and generic attachment results.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(ClientLogger.initialize);

  testWidgets('renders arbitrary owners and preserves every native action', (
    tester,
  ) async {
    final bridge = _AttachmentRegistryBridge();
    var nativeActions = 0;
    await tester.pumpWidget(
      _menu(bridge, onNativeAction: () => nativeActions++),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('example.first::picker')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('example.second::picker')),
      findsOneWidget,
    );
    expect(find.text('Toolbox only'), findsNothing);
    for (final icon in <IconData>[
      Icons.image,
      Icons.photo_camera,
      Icons.description,
      Icons.screenshot_monitor,
      Icons.notifications,
      Icons.location_on,
      Icons.auto_awesome,
    ]) {
      await tester.tap(find.byIcon(icon));
      await tester.pump();
    }
    expect(nativeActions, 7);
    expect(
      bridge.calls.every((call) => call.methodName != 'invokeToolPkgPublicApi'),
      isTrue,
    );
  });

  testWidgets(
    'empty registry retains native actions without an attach confirmation',
    (tester) async {
      final bridge = _AttachmentRegistryBridge(entries: const []);
      await tester.pumpWidget(_menu(bridge));
      await tester.pumpAndSettle();
      expect(find.byType(LinearProgressIndicator), findsNothing);
      expect(find.byKey(const ValueKey('example.first::picker')), findsNothing);
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
      expect(find.byIcon(Icons.description), findsOneWidget);
      expect(find.byIcon(Icons.location_on), findsOneWidget);
      expect(
        bridge.calls.where((call) => call.methodName == 'attachUploadedFile'),
        isEmpty,
      );
    },
  );

  testWidgets(
    'forwards opaque params separately from chatId and waits for explicit completion',
    (tester) async {
      final bridge = _AttachmentRegistryBridge();
      final values = <ChatAttachmentResult>[];
      final chats = <String?>[];
      await tester.pumpWidget(
        _menu(
          bridge,
          onResult: (value, chatId) async {
            values.add(ChatAttachmentResult.fromPresentationValue(value));
            chats.add(chatId);
          },
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('example.first::picker')));
      await tester.pumpAndSettle();
      final launcher = tester.widget<ToolPkgUiLauncherScreen>(
        find.byType(ToolPkgUiLauncherScreen),
      );
      final presentation = launcher.initialState['presentation'] as Map;
      expect(launcher.plugin.packageName, 'example.first');
      expect(launcher.initialRouteId, 'picker-route');
      expect(launcher.initialState['chatId'], 'chat-a');
      expect(presentation.keys.toSet(), {'requestId', 'input'});
      expect(presentation['input'], _opaqueInput('example.first'));
      expect(values, isEmpty);
      await tester.tap(find.text('Preview only'));
      await tester.pumpAndSettle();
      expect(values, isEmpty);
      expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
      await tester.tap(find.text('Wrong request'));
      await tester.pumpAndSettle();
      expect(values, isEmpty);
      expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
      await tester.tap(find.text('Confirm text'));
      await tester.pumpAndSettle();
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
      expect(chats, ['chat-a']);
      expect(
        (values.single as ChatUiTextAttachment).content,
        'Confirmed selection',
      );
    },
  );

  testWidgets(
    'second registered owner returns files through the existing attachment API',
    (tester) async {
      final bridge = _AttachmentRegistryBridge();
      final clients = GeneratedCoreProxyClients(bridge);
      await tester.pumpWidget(
        _menu(
          bridge,
          onResult: (value, chatId) async {
            final result =
                ChatAttachmentResult.fromPresentationValue(value)
                    as ChatUiFileAttachment;
            await clients.chatRuntimeHolderMain.attachUploadedFile(
              attachment: result.attachment,
              expectedChatId: chatId,
            );
          },
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('example.second::picker')));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<ToolPkgUiLauncherScreen>(
              find.byType(ToolPkgUiLauncherScreen),
            )
            .plugin
            .packageName,
        'example.second',
      );
      await tester.tap(find.text('Confirm file'));
      await tester.pumpAndSettle();
      final call = bridge.calls.singleWhere(
        (call) => call.methodName == 'attachUploadedFile',
      );
      final args = call.args as Map;
      expect(args['expectedChatId'], 'chat-a');
      expect(args['attachment'], _fileValue()['attachment']);
    },
  );

  testWidgets(
    'explicit cancel returns no attachment and cannot reuse a request ID',
    (tester) async {
      final bridge = _AttachmentRegistryBridge();
      var completions = 0;
      await tester.pumpWidget(
        _menu(bridge, onResult: (_, _) async => completions++),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('example.first::picker')));
      await tester.pumpAndSettle();
      final first = _requestId(tester);
      await tester.tap(find.text('Cancel selection'));
      await tester.pumpAndSettle();
      expect(completions, 0);
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
      await tester.tap(find.byKey(const ValueKey('example.first::picker')));
      await tester.pumpAndSettle();
      expect(_requestId(tester), isNot(first));
      await tester.tap(find.text('Cancel selection'));
      await tester.pumpAndSettle();
      expect(
        bridge.calls.where((call) => call.methodName == 'attachUploadedFile'),
        isEmpty,
      );
    },
  );

  testWidgets(
    'rejects returning to a chat after its presentation selection expired',
    (tester) async {
      final bridge = _AttachmentRegistryBridge();
      const request = ChatAttachmentRequest(
        chatId: 'chat-a',
        chatSelectionGeneration: 1,
      );
      var generation = 1;
      var currentChatId = 'chat-a';
      var accepted = 0;
      await tester.pumpWidget(
        _menu(
          bridge,
          onResult: (_, chatId) async {
            if (chatId != request.chatId ||
                !request.isCurrent(
                  currentChatId: currentChatId,
                  requestedChatId: currentChatId,
                  currentSelectionGeneration: generation,
                  switching: false,
                )) {
              throw StateError(
                'The attachment presentation belongs to an expired chat selection.',
              );
            }
            accepted++;
          },
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('example.first::picker')));
      await tester.pumpAndSettle();
      currentChatId = 'chat-b';
      generation++;
      currentChatId = 'chat-a';
      generation++;
      await tester.tap(find.text('Confirm text'));
      await tester.pumpAndSettle();
      expect(accepted, 0);
      expect(find.textContaining('expired chat selection'), findsOneWidget);
      expect(
        bridge.calls.where((call) => call.methodName == 'attachUploadedFile'),
        isEmpty,
      );
    },
  );

  testWidgets('unregistered owning route is an explicit catalog error', (
    tester,
  ) async {
    final bridge = _AttachmentRegistryBridge(
      routes: [_route('different.owner')],
    );
    await tester.pumpWidget(_menu(bridge));
    await tester.pumpAndSettle();
    expect(
      find.textContaining('exactly one registered UI route'),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('example.first::picker')), findsNothing);
    expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
  });

  testWidgets(
    'duplicate owning routes are rejected rather than choosing a route',
    (tester) async {
      final bridge = _AttachmentRegistryBridge(
        routes: [_route('example.first'), _route('example.first')],
      );
      await tester.pumpWidget(_menu(bridge));
      await tester.pumpAndSettle();
      expect(
        find.textContaining('exactly one registered UI route'),
        findsOneWidget,
      );
    },
  );

  testWidgets('malformed explicit completion is an error and never attaches', (
    tester,
  ) async {
    final bridge = _AttachmentRegistryBridge();
    var accepted = 0;
    await tester.pumpWidget(
      _menu(
        bridge,
        onResult: (value, _) async {
          ChatAttachmentResult.fromPresentationValue(value);
          accepted++;
        },
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('example.first::picker')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Malformed completion'));
    await tester.pumpAndSettle();
    expect(accepted, 0);
    expect(find.textContaining('invalid fields'), findsOneWidget);
  });

  testWidgets('completion survives disposal of the original plus-menu popup', (
    tester,
  ) async {
    final bridge = _AttachmentRegistryBridge();
    var visible = true;
    Object? received;
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: StatefulBuilder(
          builder: (context, update) => Scaffold(
            body: visible
                ? ChatAttachmentMenuPopup(
                    clients: GeneratedCoreProxyClients(bridge),
                    chatId: 'chat-a',
                    onDismiss: () => update(() => visible = false),
                    onAttachmentResult: (value, _) async {
                      received = value;
                    },
                    onAttachImage: null,
                    onTakePhoto: null,
                    onAttachFile: null,
                    onAttachScreenContent: null,
                    onAttachNotifications: null,
                    onAttachLocation: null,
                    onAttachPackage: () {},
                  )
                : const SizedBox.shrink(),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('example.first::picker')));
    await tester.pumpAndSettle();
    expect(find.byType(ChatAttachmentMenuPopup), findsNothing);
    expect(find.byType(ToolPkgUiLauncherScreen), findsOneWidget);
    await tester.tap(find.text('Confirm text'));
    await tester.pumpAndSettle();
    expect(received, _textValue());
    expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
  });

  test('an abandoned pending selection cannot revive an existing request', () {
    const request = ChatAttachmentRequest(
      chatId: 'chat-a',
      chatSelectionGeneration: 4,
    );
    expect(
      request.isCurrent(
        currentChatId: 'chat-a',
        requestedChatId: 'chat-a',
        currentSelectionGeneration: 5,
        switching: false,
      ),
      isFalse,
    );
  });

  test('text and file results preserve their exact generic payloads', () {
    final text =
        ChatAttachmentResult.fromPresentationValue(_textValue())
            as ChatUiTextAttachment;
    expect(text.name, 'selection.txt');
    expect(text.content, 'Confirmed selection');
    expect(text.mediaType, 'text/plain');
    final file =
        ChatAttachmentResult.fromPresentationValue(_fileValue())
            as ChatUiFileAttachment;
    expect(file.attachment.toJson(), _fileValue()['attachment']);
  });

  test('unknown types and invalid metadata are rejected without coercion', () {
    final invalid = <Object?>[
      null,
      {'type': 'custom', 'value': 'data'},
      {..._textValue(), 'extra': true},
      {..._textValue(), 'mediaType': 'text/html'},
      {..._textValue(), 'name': '../private.txt'},
      {..._textValue(), 'content': 1},
      {
        'type': 'file',
        'attachment': {'filePath': '/tmp/incomplete'},
      },
      {
        'type': 'file',
        'attachment': {..._fileValue()['attachment'] as Map, 'fileSize': -1},
      },
      {
        'type': 'file',
        'attachment': {
          ..._fileValue()['attachment'] as Map,
          'content': 'uncommitted',
        },
      },
    ];
    for (final value in invalid) {
      expect(
        () => ChatAttachmentResult.fromPresentationValue(value),
        throwsFormatException,
      );
    }
  });

  test(
    'request guards switching, requested identity, and generation independently',
    () {
      const request = ChatAttachmentRequest(
        chatId: 'chat-a',
        chatSelectionGeneration: 4,
      );
      expect(
        request.isCurrent(
          currentChatId: 'chat-a',
          requestedChatId: 'chat-a',
          currentSelectionGeneration: 4,
          switching: false,
        ),
        isTrue,
      );
      expect(
        request.isCurrent(
          currentChatId: 'chat-a',
          requestedChatId: 'chat-b',
          currentSelectionGeneration: 4,
          switching: false,
        ),
        isFalse,
      );
      expect(
        request.isCurrent(
          currentChatId: 'chat-a',
          requestedChatId: 'chat-a',
          currentSelectionGeneration: 5,
          switching: false,
        ),
        isFalse,
      );
      expect(
        request.isCurrent(
          currentChatId: 'chat-a',
          requestedChatId: 'chat-a',
          currentSelectionGeneration: 4,
          switching: true,
        ),
        isFalse,
      );
    },
  );

  test(
    'only the exact presentation type and owned request may settle a dialog',
    () {
      const request = ContributionPresentationContext(
        requestId: 'owned',
        input: {'opaque': true},
      );
      expect(
        ContributionPresentationResult.fromActionResult({
          'requestId': 'other',
          'type': ContributionPresentationResult.completeType,
          'value': _textValue(),
        }, request: request),
        isNull,
      );
      expect(
        ContributionPresentationResult.fromActionResult({
          'requestId': 'owned',
          'type': 'preview.complete',
          'value': _textValue(),
        }, request: request),
        isNull,
      );
      expect(
        ContributionPresentationResult.fromActionResult({
          'requestId': 'owned',
          'type': ContributionPresentationResult.cancelType,
        }, request: request)!.status,
        ContributionPresentationStatus.cancelled,
      );
      expect(
        () => ContributionPresentationResult.fromActionResult({
          'requestId': 'owned',
          'type': ContributionPresentationResult.completeType,
        }, request: request),
        throwsFormatException,
      );
    },
  );
}

/// Mounts the exact popup shared by the Agent and Classic production composers.
Widget _menu(
  _AttachmentRegistryBridge bridge, {
  ChatAttachmentCompletion? onResult,
  VoidCallback? onNativeAction,
}) => MaterialApp(
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(
    body: SizedBox(
      width: 360,
      child: ChatAttachmentMenuPopup(
        clients: GeneratedCoreProxyClients(bridge),
        chatId: 'chat-a',
        onDismiss: () {},
        onAttachmentResult: onResult,
        onAttachImage: onNativeAction,
        onTakePhoto: onNativeAction,
        onAttachFile: onNativeAction,
        onAttachScreenContent: onNativeAction,
        onAttachNotifications: onNativeAction,
        onAttachLocation: onNativeAction,
        onAttachPackage: onNativeAction ?? () {},
      ),
    ),
  ),
);

/// Reads the host-owned ID only after the actual embedded route is mounted.
String _requestId(WidgetTester tester) =>
    ((tester
                .widget<ToolPkgUiLauncherScreen>(
                  find.byType(ToolPkgUiLauncherScreen),
                )
                .initialState['presentation']
            as Map)['requestId'])
        as String;

/// Provides unrelated nested data to prove the host does not interpret route input.
Map<String, Object?> _opaqueInput(String owner) => {
  'custom': [
    owner,
    42,
    {'chatId': 'plugin-input'},
  ],
};

/// Uses the locked generic text completion without any plugin domain fields.
Map<String, Object?> _textValue() => {
  'type': 'text',
  'name': 'selection.txt',
  'content': 'Confirmed selection',
  'mediaType': 'text/plain',
};

/// Uses only existing AttachmentInfo metadata for an already committed upload.
Map<String, Object?> _fileValue() => {
  'type': 'file',
  'attachment': {
    'filePath': '/uploads/received.bin',
    'nodeId': 'test-node',
    'fileName': 'received.bin',
    'mimeType': 'application/octet-stream',
    'fileSize': 3,
    'content': '',
  },
};

/// Creates real catalog models with matching route IDs but different package owners.
core_proxy.ToolPkgNavigationEntry _entry(
  String owner, {
  String surface = 'chat_attachments',
}) => core_proxy.ToolPkgNavigationEntry(
  containerPackageName: owner,
  toolPkgId: owner,
  entryId: 'picker',
  routeId: 'picker-route',
  params: _opaqueInput(owner),
  surface: surface,
  title: surface == 'toolbox' ? 'Toolbox only' : 'Pick from $owner',
  description: '',
  action: null,
  icon: 'attach_file',
  order: 0,
);

/// Returns an exact owning registered route for the existing Compose host.
core_proxy.ToolPkgUiRoute _route(String owner) => core_proxy.ToolPkgUiRoute(
  containerPackageName: owner,
  toolPkgId: owner,
  routeId: 'picker-route',
  uiModuleId: 'picker-route',
  runtime: 'compose_dsl',
  screen: 'ui/picker.js',
  title: 'Picker for $owner',
  description: '',
  moduleSpec: const {},
  keepAlive: false,
);

/// Models real generated catalog calls and serialized Compose action events.
class _AttachmentRegistryBridge extends OperitRuntimeBridge {
  /// Supplies explicit test registrations without default production providers.
  _AttachmentRegistryBridge({
    List<core_proxy.ToolPkgNavigationEntry>? entries,
    List<core_proxy.ToolPkgUiRoute>? routes,
  }) : entries =
           entries ??
           [
             _entry('example.first'),
             _entry('example.second'),
             _entry('example.toolbox', surface: 'toolbox'),
           ],
       routes = routes ?? [_route('example.first'), _route('example.second')];

  final List<core_proxy.ToolPkgNavigationEntry> entries;
  final List<core_proxy.ToolPkgUiRoute> routes;
  final List<CoreCallRequest> calls = [];

  late final compose = ComposeSessionFixture(
    render: (_) => _attachmentTree(),
    action: _action,
  );

  /// Encodes actual registry and render responses through the generated proxy codec.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map<String, Object?>;
    Object? value;
    switch (request.methodName) {
      case 'getToolPkgNavigationEntries':
        value = entries.map((entry) => entry.toJson()).toList();
      case 'getToolPkgUiRoutes':
        value = routes.map((route) => route.toJson()).toList();
      case 'getToolPkgContainerRuntime':
        value = _runtime(args['containerPackageName'] as String).toJson();
      case 'acquireToolPkgExecutionEngine':
      case 'releaseToolPkgExecutionEngine':
      case 'attachUploadedFile':
        value = null;
      case 'readToolPkgTextResource':
        value = 'export default function render() {}';
      case 'openComposeDslSession':
        value = compose.open();
      default:
        throw StateError('Unexpected registry call: ${request.methodName}');
    }
    return encodeCoreLink([0, value]);
  }

  /// Submits actions through the production retained Compose session contract.
  @override
  Future<CorePushSink> push(CorePushRequest request) => compose.submit(request);

  /// Rejects snapshot watches because Compose actions use their own event streams.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw StateError('Unexpected snapshot');

  /// Observes typed updates from the exact retained Compose session.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName != 'updates') {
      throw StateError('Unexpected watch: ${request.propertyName}');
    }
    return compose.updates(request);
  }

  /// Returns presentation results through the production action-result callback.
  Future<Object?> _action(Map<String, Object?> state, String action) async {
    final presentation = state['presentation'] as Map;
    final requestId = presentation['requestId'];
    Object? result;
    switch (action) {
      case 'preview':
        result = {'preview': true};
      case 'wrong':
        result = {
          'type': ContributionPresentationResult.completeType,
          'requestId': 'another-request',
          'value': _textValue(),
        };
      case 'text':
        result = {
          'type': ContributionPresentationResult.completeType,
          'requestId': requestId,
          'value': _textValue(),
        };
      case 'file':
        result = {
          'type': ContributionPresentationResult.completeType,
          'requestId': requestId,
          'value': _fileValue(),
        };
      case 'cancel':
        result = {
          'type': ContributionPresentationResult.cancelType,
          'requestId': requestId,
        };
      case 'malformed':
        result = {
          'type': ContributionPresentationResult.completeType,
          'requestId': requestId,
          'value': {'type': 'text'},
        };
      default:
        throw StateError('Unexpected action: $action');
    }
    return result;
  }
}

/// Supplies registered attachment actions as retained Compose tree nodes.
Map<String, Object?> _attachmentTree() => {
  'type': 'Column',
  'props': <String, Object?>{},
  'children': [
    for (final entry in <String, String>{
      'preview': 'Preview only',
      'wrong': 'Wrong request',
      'text': 'Confirm text',
      'file': 'Confirm file',
      'cancel': 'Cancel selection',
      'malformed': 'Malformed completion',
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
};

/// Provides complete runtime metadata required by the existing embedded launcher.
core_proxy.ToolPkgContainerRuntime _runtime(String owner) =>
    core_proxy.ToolPkgContainerRuntime(
      chatLifecycleHooks: const [],
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
          id: 'picker-route',
          routeId: 'picker-route',
          runtime: 'compose_dsl',
          screenExport: null,
          screen: 'ui/picker.js',
          title: core_proxy.LocalizedText(values: {'default': 'Picker'}),
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
