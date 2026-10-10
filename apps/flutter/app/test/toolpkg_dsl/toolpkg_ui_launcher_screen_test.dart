import '../support/compose_session_fixture.dart';
import 'dart:async';
import 'dart:convert';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/common/markdown/XmlRenderPluginRegistry.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleAiMessageComposable.dart';

import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:desktop_widgets/desktop_widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/features/chat/components/MessageContextMenu.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';

/// Verifies plugin rendering and message-menu integration through the Core bridge.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(ClientLogger.initialize);

  testWidgets('bubble rebuild retains the XML plugin engine and action state', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge();
    final clients = GeneratedCoreProxyClients(bridge);
    final plugin = _pluginRuntime();
    XmlRenderPluginRegistry.register(({
      required tagName,
      required xmlContent,
      required textColor,
      required isStreaming,
      required xmlStream,
    }) {
      if (tagName != 'lifecycle_test') return null;
      return ToolPkgUiLauncherScreen(
        clients: clients,
        plugin: plugin,
        showLauncherChrome: false,
      );
    });
    final rawMessage = _translationMessage('ai').toJson();
    rawMessage['parts'] = [
      core_proxy.MessagePart(
        partId: 'badge',
        sequence: 0,
        kind: core_proxy.MessagePartKind.markdown,
        content: '<lifecycle_test></lifecycle_test>',
        toolCallId: null,
        toolName: null,
        attributes: const {},
      ).toJson(),
    ];
    final message = core_proxy.ChatMessage.fromJson(rawMessage);

    /// Rebuilds the real AI bubble while changing only its available width.
    Widget screen(double width) => OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: width,
            child: BubbleAiMessageComposable(
              message: message,
              backgroundColor: Colors.white,
              textColor: Colors.black,
              splitMarkdownContent: (content) async => [
                for (final type in [
                  'markdownBlockStart',
                  'markdownBlockChunk',
                  'markdownBlockEnd',
                ])
                  core_proxy.MarkdownStreamEvent(
                    chatId: 'test',
                    eventType: type,
                    value: type == 'markdownBlockChunk' ? content : null,
                    id: null,
                    blockId: 1,
                    inlineId: null,
                    parentBlockId: null,
                    nodeType: 'XmlBlock',
                    headerLevel: null,
                    xml: null,
                  ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpWidget(screen(760));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Increment'));
    await tester.pumpAndSettle();
    expect(find.text('Counter: 1'), findsOneWidget);
    final state = tester.state(find.byType(ToolPkgUiLauncherScreen));
    await tester.pumpWidget(screen(560));
    await tester.pumpAndSettle();
    expect(tester.state(find.byType(ToolPkgUiLauncherScreen)), same(state));
    expect(find.text('Counter: 1'), findsOneWidget);
    expect(
      bridge.calls.where((call) => call.methodName == 'render'),
      hasLength(1),
    );
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('equal nested XML inputs do not execute the script again', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge();
    final clients = GeneratedCoreProxyClients(bridge);
    final plugin = _pluginRuntime();

    /// Recreates equal JSON values as an external rebuild does.
    Widget screen() => MaterialApp(
      home: ToolPkgUiLauncherScreen(
        clients: clients,
        plugin: plugin,
        initialState: {
          'xmlContent': '<test/>',
          'items': [
            {'value': 1},
          ],
        },
        initialMemo: {
          'nested': {
            'values': [1, 2],
          },
        },
      ),
    );
    await tester.pumpWidget(screen());
    await tester.pumpAndSettle();
    await tester.tap(find.text('Increment'));
    await tester.pumpAndSettle();
    await tester.pumpWidget(screen());
    await tester.pumpAndSettle();
    expect(find.text('Counter: 1'), findsOneWidget);
    expect(
      bridge.calls.where((call) => call.methodName == 'render'),
      hasLength(1),
    );
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'moodlet XML screen stays independent of moduleSpec without screen',
    (tester) async {
      final bridge = _ToolPkgDslTestBridge();
      const spec = {'id': 'silent_badge_fingerprint', 'runtime': 'compose_dsl'};
      await tester.pumpWidget(
        _screen(
          bridge,
          plugin: _moduleOnlyPluginRuntime(),
          initialRouteId: 'xml_render',
          embeddedScreenPath: 'dist/ui/silent_badge.ui.js',
          initialModuleSpec: spec,
        ),
      );
      await tester.pumpAndSettle();
      final read = bridge.calls.singleWhere(
        (call) => call.methodName == 'readToolPkgTextResource',
      );
      expect((read.args as Map)['resourcePath'], 'dist/ui/silent_badge.ui.js');
      expect(
        bridge.calls.where(
          (call) => call.methodName == 'getToolPkgComposeDslScript',
        ),
        isEmpty,
      );
      final execute = bridge.calls.singleWhere(
        (call) => call.methodName == 'render',
      );
      final args = execute.args as Map;
      final options = args['runtimeOptions'] as Map;
      expect(
        args['contextKey'],
        startsWith(
          'toolpkg_xml_render:module_only_toolpkg:dist/ui/silent_badge.ui.js:',
        ),
      );
      expect(options['__operit_script_screen'], 'dist/ui/silent_badge.ui.js');
      expect(options['uiModuleId'], 'xml_render');
      expect(options['moduleSpec'], spec);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('streaming inputs preserve the tree and execution context', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge();
    final clients = GeneratedCoreProxyClients(bridge);
    final plugin = _pluginRuntime();

    /// Updates the XML input on the same embedded screen instance.
    Widget screen(String xml) => MaterialApp(
      home: ToolPkgUiLauncherScreen(
        clients: clients,
        plugin: plugin,
        initialState: {'xmlContent': xml},
        initialMemo: const {'source': 'xml'},
      ),
    );
    await tester.pumpWidget(screen('<demo>'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Increment'));
    await tester.pumpAndSettle();
    expect(find.text('Counter: 1'), findsOneWidget);
    final hostState = tester.state(find.byType(ToolPkgUiLauncherScreen));
    final pending = Completer<String>();
    bridge.renderCompletion = pending;
    await tester.pumpWidget(screen('<demo>one'));
    await tester.pumpAndSettle();
    expect(find.text('Counter: 1'), findsOneWidget);
    expect(tester.state(find.byType(ToolPkgUiLauncherScreen)), same(hostState));
    final calls = bridge.calls.where((call) => call.methodName == 'render');
    final options = (calls.last.args as Map)['runtimeOptions'] as Map;
    expect(
      options['__operit_input_state'],
      containsPair('xmlContent', '<demo>one'),
    );
    expect(options.containsKey('state'), isFalse);
    expect(bridge._count, 1);
    expect(options.containsKey('memo'), isFalse);
    await tester.pumpWidget(screen('<demo>two</demo>'));
    await tester.pump();
    bridge.renderCompletion = null;
    pending.complete(_counterRenderResult(99));
    await tester.pumpAndSettle();
    expect(find.text('Counter: 99'), findsNothing);
    final latest = (calls.last.args as Map)['runtimeOptions'] as Map;
    expect(
      latest['__operit_input_state'],
      containsPair('xmlContent', '<demo>two</demo>'),
    );
    expect(
      bridge.calls.where(
        (call) => call.methodName == 'acquireToolPkgExecutionEngine',
      ),
      hasLength(1),
    );
    expect(
      bridge.calls.where(
        (call) => call.methodName == 'releaseToolPkgExecutionEngine',
      ),
      isEmpty,
    );
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
  });

  for (final sender in ['user', 'ai']) {
    testWidgets('opens and closes translation from the $sender message menu', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = sender == 'user'
          ? const Size(390, 844)
          : const Size(1280, 720);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final bridge = _ToolPkgDslTestBridge(
        renderResult: (_) => jsonEncode({
          'success': true,
          'tree': _node(
            'AlertDialog',
            props: {
              'title': 'Translation dialog',
              'confirmText': 'Translate',
              'dismissText': 'Close',
              'closeOnConfirm': false,
              'onConfirm': {'__actionId': 'translate'},
            },
            slots: {
              'text': [
                _node(
                  'LazyColumn',
                  props: {'width': 560, 'height': 200},
                  children: [
                    _node('Text', props: {'text': 'Selected message'}),
                  ],
                ),
              ],
            },
          ),
        }),
      );
      final clients = GeneratedCoreProxyClients(bridge);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: MessageContextMenu(
              message: _translationMessage(sender),
              chatId: 'translation-chat',
              messageIndex: 3,
              clients: clients,
              packageManager: clients.application.packageManager(),
              onToggleFavoriteMessage: (timestamp, isFavorite) async {},
              child: const Text('Message bubble'),
            ),
          ),
        ),
      );
      if (sender == 'user') {
        await tester.longPress(find.text('Message bubble'));
      } else {
        await tester.tap(
          find.text('Message bubble'),
          buttons: kSecondaryButton,
        );
      }
      await tester.pumpAndSettle();
      expect(find.byIcon(Icons.translate), findsOneWidget);
      await tester.tap(find.text('Translate message'));
      await tester.pumpAndSettle();
      final invocation =
          bridge.calls
                  .singleWhere(
                    (call) =>
                        call.methodName == 'invokeToolPkgChatMessageMenuItem',
                  )
                  .args
              as Map;
      expect(invocation['chatId'], 'translation-chat');
      expect(invocation['messageIndex'], 3);
      expect((invocation['message'] as Map)['content'], 'Selected message');
      expect((invocation['message'] as Map)['sender'], sender);
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text('Selected message'), findsOneWidget);
      final bounds = tester.getRect(find.byType(AlertDialog));
      expect(bounds.left, greaterThanOrEqualTo(0));
      expect(bounds.right, lessThanOrEqualTo(tester.view.physicalSize.width));
      await tester.tap(find.text('Translate'));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(bridge.calls.last.args, containsPair('actionId', 'translate'));
      if (sender == 'user') {
        await tester.tap(find.text('Close'));
      } else {
        await tester.tapAt(const Offset(4, 4));
      }
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(find.byType(ToolPkgUiLauncherScreen), findsNothing);
      expect(find.text('Message bubble'), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets(
    'publishes color-only theme changes without reloading the UI context',
    (tester) async {
      final bridge = _ToolPkgDslTestBridge();
      final clients = GeneratedCoreProxyClients(bridge);
      final plugin = _pluginRuntime();

      /// Rebuilds the same plugin route with a different application color scheme.
      Widget themed(Color seed) => MaterialApp(
        theme: ThemeData(colorScheme: ColorScheme.fromSeed(seedColor: seed)),
        home: ToolPkgUiLauncherScreen(clients: clients, plugin: plugin),
      );
      await tester.pumpWidget(themed(Colors.red));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Increment'));
      await tester.pumpAndSettle();
      await tester.pumpWidget(themed(Colors.green));
      await tester.pumpAndSettle();
      expect(find.text('Counter: 1'), findsOneWidget);
      expect(
        bridge.calls.where((call) => call.methodName == 'render'),
        hasLength(1),
      );
      final changes = bridge.calls.where(
        (call) =>
            call.methodName == 'action' &&
            (call.args as Map)['actionId'] == '__operit_theme_changed',
      );
      expect(changes, isNotEmpty);
      final snapshot = (changes.last.args as Map)['payload'] as Map;
      expect(snapshot['brightness'], 'light');
      final expected = ColorScheme.fromSeed(seedColor: Colors.green).primary;
      final rgb = (expected.toARGB32() & 0xffffff)
          .toRadixString(16)
          .padLeft(6, '0');
      expect((snapshot['colors'] as Map)['primary'], '#${rgb}ff');
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('one DSL frame builds both window content and published pixels', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge();
    final clients = GeneratedCoreProxyClients(bridge);
    const definition = core_proxy.ToolPkgDesktopWidget(
      containerPackageName: 'demo_toolpkg',
      toolPkgId: 'demo_toolpkg',
      widgetId: 'summary',
      routeId: 'old-route',
      renderRouteId: 'widget_summary',
      title: 'Old title',
      subtitle: '',
      description: '',
      icon: null,
      order: 0,
    );
    late BuildContext hostContext;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) {
            hostContext = context;
            return const Scaffold(body: Text('Host'));
          },
        ),
      ),
    );
    final frame = await ToolPkgDesktopWidgetFrame.load(
      clients: clients,
      definition: definition,
      instanceId: 'shared-frame',
      useEnglish: true,
    );
    String? opened;

    /// Records the common action destination for either presentation.
    Future<void> openRoute(String package, String route) async {
      opened = '$package/$route';
    }

    final pixels = DesktopWidgetRasterizer.render(
      context: hostContext,
      size: const Size(180, 120),
      child: frame.buildContent(
        clients: clients,
        instanceId: 'shared-frame',
        onOpenRoute: openRoute,
      ),
    );
    await tester.pump();
    await tester.runAsync(() async {
      expect((await pixels).take(4), [137, 80, 78, 71]);
    });
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: frame.buildContent(
            clients: clients,
            instanceId: 'shared-frame',
            onOpenRoute: openRoute,
          ),
        ),
      ),
    );
    await tester.tap(find.text('Widget loaded'));
    expect(opened, 'demo_toolpkg/main');
    expect(frame.definition.title, 'Summary');
    expect(
      bridge.calls.where(
        (call) => call.methodName == 'renderToolPkgDesktopWidget',
      ),
      hasLength(1),
    );
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'renders registered desktop widget and opens its application route',
    (tester) async {
      final bridge = _ToolPkgDslTestBridge();
      String? openedRoute;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ToolPkgDesktopWidgetView(
              clients: GeneratedCoreProxyClients(bridge),
              definition: const core_proxy.ToolPkgDesktopWidget(
                containerPackageName: 'demo_toolpkg',
                toolPkgId: 'demo_toolpkg',
                widgetId: 'summary',
                routeId: 'main',
                renderRouteId: 'widget_summary',
                title: 'Summary',
                subtitle: '',
                description: '',
                icon: null,
                order: 0,
              ),
              instanceId: 'desktop-instance-1',
              onOpenRoute: (package, route) async {
                openedRoute = '$package/$route';
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('Widget loaded'), findsOneWidget);
      final call = bridge.calls.singleWhere(
        (call) => call.methodName == 'renderToolPkgDesktopWidget',
      );
      expect(call.args, containsPair('widgetId', 'summary'));
      expect(call.args, containsPair('instanceId', 'desktop-instance-1'));
      await tester.tap(find.text('Widget loaded'));
      await tester.pumpAndSettle();
      expect(openedRoute, 'demo_toolpkg/main');
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('keeps alert dialog open according to close policy', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: (_) => jsonEncode({
        'success': true,
        'tree': _node(
          'alertdialog',
          props: {
            'title': 'Confirm operation',
            'text': 'Dialog body',
            'confirmText': 'Apply',
            'closeOnConfirm': false,
            'onConfirm': {'__actionId': 'confirm'},
          },
        ),
      }),
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();
    expect(find.text('Dialog body'), findsOneWidget);
    await tester.tap(find.text('Apply'));
    await tester.pumpAndSettle();
    expect(find.text('Dialog body'), findsOneWidget);
    expect(bridge.calls.last.args, containsPair('actionId', 'confirm'));
    await tester.tapAt(const Offset(4, 4));
    await tester.pumpAndSettle();
    expect(find.text('Dialog body'), findsNothing);
  });

  for (final size in [
    const Size(1280, 720),
    const Size(390, 844),
    const Size(740, 360),
  ]) {
    testWidgets('workflow dialog stays bounded at $size', (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = size;
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final bridge = _ToolPkgDslTestBridge(
        renderResult: (_) => jsonEncode({
          'success': true,
          'tree': _node(
            'Dialog',
            props: {
              'properties': {'usePlatformDefaultWidth': false},
            },
            children: [
              _node(
                'Column',
                props: {
                  'width': 560,
                  'padding': 24,
                  'spacing': 16,
                  'modifier': {
                    '__modifierOps': [
                      {
                        'name': 'heightIn',
                        'args': [0, 650],
                      },
                    ],
                  },
                },
                children: [
                  _node(
                    'Text',
                    props: {'text': 'Edit node', 'style': 'headlineSmall'},
                  ),
                  _node(
                    'LazyColumn',
                    props: {
                      'height': 440,
                      'weight': 1,
                      'weightFill': false,
                      'fillMaxWidth': true,
                      'spacing': 12,
                    },
                    children: [
                      for (var index = 0; index < 30; index++)
                        _node(
                          'Text',
                          props: {'text': 'Field $index', 'height': 48},
                        ),
                    ],
                  ),
                  _node(
                    'FlowRow',
                    props: {
                      'fillMaxWidth': true,
                      'horizontalArrangement': 'end',
                    },
                    children: [
                      _node(
                        'TextButton',
                        props: {
                          'text': 'Save',
                          'onClick': {'__actionId': 'save'},
                        },
                      ),
                    ],
                  ),
                ],
              ),
            ],
          ),
        }),
      );
      await tester.pumpWidget(_screen(bridge));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(
        tester.getSize(find.byType(Dialog)).width,
        lessThanOrEqualTo(size.width),
      );
      final save = tester.getRect(find.text('Save'));
      expect(save.bottom, lessThan(size.height));
      final list = tester.widget<ListView>(
        find.byWidgetPredicate((widget) => widget is ListView),
      );
      list.controller!.jumpTo(list.controller!.position.maxScrollExtent);
      await tester.pumpAndSettle();
      expect(find.text('Field 29'), findsOneWidget);
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      expect(bridge.calls.last.args, containsPair('actionId', 'save'));
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets(
    'combined motion supports dragging and pinch without competing recognizers',
    (tester) async {
      final bridge = _ToolPkgDslTestBridge(
        renderResult: (_) => jsonEncode({
          'success': true,
          'tree': _node(
            'Canvas',
            props: {
              'width': 400,
              'height': 300,
              'commands': [],
              'modifier': {
                '__modifierOps': [
                  {
                    'name': 'dragGestures',
                    'args': [
                      {
                        'onDragStart': {'__actionId': 'start'},
                        'onDrag': {'__actionId': 'drag'},
                        'onDragEnd': {'__actionId': 'end'},
                        'onDragCancel': {'__actionId': 'cancel'},
                      },
                    ],
                  },
                  {
                    'name': 'transformGestures',
                    'args': [
                      {
                        'onGesture': {'__actionId': 'transform'},
                      },
                    ],
                  },
                ],
              },
            },
          ),
        }),
      );
      await tester.pumpWidget(_screen(bridge));
      await tester.pumpAndSettle();
      final origin =
          tester.getTopLeft(find.byType(CustomPaint).last) +
          const Offset(70, 70);
      final first = await tester.startGesture(origin, pointer: 1);
      await first.moveBy(const Offset(35, 0));
      await tester.pump();
      await first.moveBy(const Offset(20, 0));
      await tester.pump();
      final second = await tester.startGesture(
        origin + const Offset(150, 0),
        pointer: 2,
      );
      await second.moveBy(const Offset(45, 0));
      await tester.pump();
      await second.moveBy(const Offset(30, 0));
      await tester.pump();
      await second.up();
      await first.up();
      await tester.pumpAndSettle();
      final actions = bridge.calls
          .where((call) => call.args is Map)
          .map((call) => (call.args as Map)['actionId'])
          .toList();
      expect(actions, containsAll(['start', 'drag', 'cancel', 'transform']));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('builds lazy list entries only near the viewport', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: (_) => jsonEncode({
        'success': true,
        'tree': _node(
          'LazyColumn',
          props: {'height': 200, 'spacing': 8},
          children: [
            for (var index = 0; index < 200; index++)
              _node(
                'Text',
                props: {'key': index, 'height': 48, 'text': 'Entry $index'},
              ),
          ],
        ),
      }),
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();
    expect(find.text('Entry 0'), findsOneWidget);
    expect(find.text('Entry 199'), findsNothing);
    final list = tester.widget<ListView>(
      find.byWidgetPredicate((widget) => widget is ListView),
    );
    list.controller!.jumpTo(list.controller!.position.maxScrollExtent);
    await tester.pumpAndSettle();
    expect(find.text('Entry 199'), findsOneWidget);
  });

  testWidgets('applies column and row spacing between children', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: (_) => jsonEncode({
        'success': true,
        'tree': _node(
          'Column',
          props: {'spacing': 10, 'padding': 14},
          children: [
            _node('Text', props: {'text': 'Section', 'height': 24}),
            _node(
              'Row',
              props: {'spacing': 12},
              children: [
                _node(
                  'Text',
                  props: {'text': 'Logo', 'width': 34, 'height': 20},
                ),
                _node(
                  'Text',
                  props: {'text': 'Title', 'width': 40, 'height': 20},
                ),
              ],
            ),
          ],
        ),
      }),
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final section = tester.getRect(find.text('Section'));
    final logo = tester.getRect(find.text('Logo'));
    final title = tester.getRect(find.text('Title'));
    expect(logo.top, section.bottom + 10);
    expect(title.left, logo.right + 12);
    expect(section.left, greaterThan(10));
  });

  testWidgets('wraps FlowRow children into additional lines', (tester) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: (_) => jsonEncode({
        'success': true,
        'tree': _node(
          'FlowRow',
          props: {'width': 118, 'spacing': 10, 'runSpacing': 12},
          children: [
            _node('Text', props: {'text': 'Alpha', 'width': 50, 'height': 20}),
            _node('Text', props: {'text': 'Beta', 'width': 50, 'height': 20}),
            _node('Text', props: {'text': 'Gamma', 'width': 50, 'height': 20}),
          ],
        ),
      }),
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final alpha = tester.getRect(find.text('Alpha'));
    final beta = tester.getRect(find.text('Beta'));
    final gamma = tester.getRect(find.text('Gamma'));
    expect(find.byType(Wrap), findsOneWidget);
    expect(beta.top, alpha.top);
    expect(gamma.top, alpha.bottom + 12);
  });

  testWidgets('dispatches direct Row onClick actions', (tester) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: (_) => jsonEncode({
        'success': true,
        'tree': _node(
          'Row',
          props: <String, Object?>{
            'onClick': <String, Object?>{'__actionId': 'expand'},
            'padding': <String, Object?>{'horizontal': 14, 'vertical': 12},
          },
          children: <Map<String, Object?>>[
            _node('Text', props: <String, Object?>{'text': 'Expand'}),
          ],
        ),
        'state': <String, Object?>{},
        'memo': <String, Object?>{},
      }),
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Expand'));
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    expect((actionCall.args as Map<String, Object?>)['actionId'], 'expand');
  });

  testWidgets('clickable site card navigates without an action return value', (
    tester,
  ) async {
    final navigated = <String>[];
    AppRouterGateway.install(
      handler: (routeId, args, source) {
        navigated.add(routeId);
        expect(source, RouteEntrySource.script);
        expect(args['id'], 'doubao');
      },
      reset: (_, _, _) {},
    );
    addTearDown(AppRouterGateway.clear);
    var clicked = false;
    final bridge = _ToolPkgDslTestBridge(
      onAction: (args) {
        expect(args['actionId'], 'open');
        clicked = true;
      },
      renderResult: (_) => jsonEncode({
        'success': true,
        'tree': _node(
          'Card',
          props: {
            'fillMaxWidth': true,
            'modifier': {
              '__modifierOps': [
                {'name': 'fillMaxWidth', 'args': []},
                {
                  'name': 'clickable',
                  'args': [
                    {'__actionId': 'open'},
                  ],
                },
              ],
            },
          },
          children: [
            _node('Text', props: {'text': 'Open site'}),
          ],
        ),
        'navigationCommands': [
          if (clicked)
            {
              'route':
                  'toolpkg:com.operit.sidebar_model_sites:ui:model_sites_viewer',
              'args': {'id': 'doubao'},
            },
        ],
      }),
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();
    expect(navigated, isEmpty);
    await tester.tap(find.text('Open site'));
    await tester.pumpAndSettle();
    expect(navigated, [
      'toolpkg:com.operit.sidebar_model_sites:ui:model_sites_viewer',
    ]);
  });

  testWidgets(
    'premature action stream closure reports failure without publishing success',
    (tester) async {
      final results = <Object?>[];
      final bridge = _ToolPkgDslTestBridge(closeBeforeActionComplete: true);
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: Scaffold(
            body: ToolPkgUiLauncherScreen(
              clients: GeneratedCoreProxyClients(bridge),
              plugin: _pluginRuntime(),
              onActionResult: results.add,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Increment'));
      await tester.pumpAndSettle();
      expect(
        find.textContaining('Compose session update stream closed'),
        findsOneWidget,
      );
      expect(results, isEmpty);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('serializes rapid input and ignores delayed echoes', (
    tester,
  ) async {
    var echo = '';
    final bridge = _ToolPkgDslTestBridge(
      holdActionCompletion: true,
      onAction: (args) {
        if (args['actionId'] == 'edit') echo = args['payload'] as String;
      },
      renderResult: (_) => jsonEncode({
        'success': true,
        'tree': _node(
          'TextField',
          props: {
            'value': echo,
            'onValueChange': {'__actionId': 'edit'},
          },
        ),
      }),
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'a');
    await tester.pump();
    await tester.enterText(find.byType(TextField), 'ab');
    await tester.pump();
    expect(echo, 'a');
    expect(
      tester.widget<TextField>(find.byType(TextField)).controller!.text,
      'ab',
    );
    bridge.actionCompletion!.complete();
    await tester.pump();
    await tester.pump();
    expect(echo, 'ab');
    expect(
      tester.widget<TextField>(find.byType(TextField)).controller!.text,
      'ab',
    );
    bridge.actionCompletion!.complete();
    await tester.pumpAndSettle();
  });

  testWidgets('dispatches long press from combined clickable', (tester) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: (_) => jsonEncode({
        'success': true,
        'tree': _node(
          'Text',
          props: {
            'text': 'Hold here',
            'modifier': {
              '__modifierOps': [
                {
                  'name': 'combinedClickable',
                  'args': [
                    {
                      'onClick': {'__actionId': 'click'},
                      'onLongClick': {'__actionId': 'hold'},
                    },
                  ],
                },
              ],
            },
          },
        ),
      }),
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();
    await tester.longPress(find.text('Hold here'));
    await tester.pumpAndSettle();
    expect(bridge.calls.last.args, containsPair('actionId', 'hold'));
  });

  testWidgets('renders compose dsl tree from toolpkg route', (tester) async {
    final bridge = _ToolPkgDslTestBridge();
    await tester.pumpWidget(_screen(bridge));

    await tester.pumpAndSettle();

    expect(find.text('Demo ToolPkg'), findsOneWidget);
    expect(find.text('Main route'), findsNothing);
    expect(find.text('Counter: 0'), findsOneWidget);
    expect(find.text('Increment'), findsOneWidget);
    expect(find.byType(Card), findsOneWidget);
    expect(find.byType(CustomPaint), findsAtLeastNWidgets(1));
    expect(tester.widget<AppBar>(find.byType(AppBar)).actions, isNull);

    final scriptCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'getToolPkgComposeDslScript',
    );
    final packageManagerId = GeneratedCoreProxyClients(
      bridge,
    ).application.packageManager().objectId;
    expect(scriptCall.target, packageManagerId);
    expect(scriptCall.args, isA<Map<String, Object?>>());
    final args = scriptCall.args as Map<String, Object?>;
    expect(args['containerPackageName'], 'demo_toolpkg');
    expect(args['uiModuleId'], 'main');

    final renderCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'render',
    );
    expect(
      renderCall.target,
      GeneratedCoreProxyClients(
        bridge,
      ).servicesComposeDslSessionService.objectId,
    );
    final renderArgs = renderCall.args as Map<String, Object?>;
    expect(
      renderArgs['contextKey'],
      startsWith(
        'toolpkg_compose_dsl:demo_toolpkg:main:screen:demo_toolpkg:main:',
      ),
    );
  });

  testWidgets('opens compose dsl module when plugin has no routes', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge();
    await tester.pumpWidget(
      _screen(bridge, plugin: _moduleOnlyPluginRuntime()),
    );
    await tester.pumpAndSettle();

    expect(find.text('Module ToolPkg'), findsOneWidget);
    expect(find.text('Toolbox module'), findsNothing);
    expect(tester.widget<AppBar>(find.byType(AppBar)).actions, isNull);

    final scriptCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'getToolPkgComposeDslScript',
    );
    final scriptArgs = scriptCall.args as Map<String, Object?>;
    expect(scriptArgs['containerPackageName'], 'module_only_toolpkg');
    expect(scriptArgs['uiModuleId'], 'toolbox');

    final renderCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'render',
    );
    final renderArgs = renderCall.args as Map<String, Object?>;
    final runtimeOptions = renderArgs['runtimeOptions'] as Map<String, Object?>;
    expect(
      runtimeOptions['routeInstanceId'],
      'legacy:module_only_toolpkg:toolbox',
    );
    expect(
      runtimeOptions['executionContextKey'],
      startsWith(
        'toolpkg_compose_dsl:module_only_toolpkg:toolbox:legacy:module_only_toolpkg:toolbox:',
      ),
    );
    final moduleSpec = runtimeOptions['moduleSpec'] as Map<String, Object?>;
    expect(moduleSpec['id'], 'toolbox');
    expect(moduleSpec['routeId'], 'toolbox');
    expect(moduleSpec['runtime'], 'compose_dsl');
    expect(moduleSpec['screen'], 'ui/toolbox.js');
  });

  testWidgets('renders an embedded XML screen by its explicit resource path', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge();
    await tester.pumpWidget(
      _screen(
        bridge,
        plugin: _moduleOnlyPluginRuntime(),
        initialRouteId: 'xml_render',
        embeddedScreenPath: 'ui/planask/index.ui.js',
        initialModuleSpec: <String, Object?>{
          'id': 'xml_render',
          'runtime': 'compose_dsl',
          'screen': 'ui/planask/index.ui.js',
          'toolPkgId': 'module_only_toolpkg',
        },
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Counter: 0'), findsOneWidget);
    final resourceCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'readToolPkgTextResource',
    );
    expect(
      (resourceCall.args as Map<String, Object?>)['resourcePath'],
      'ui/planask/index.ui.js',
    );
    expect(
      bridge.calls.where(
        (request) => request.methodName == 'getToolPkgComposeDslScript',
      ),
      isEmpty,
    );
    final renderCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'render',
    );
    final renderArgs = renderCall.args as Map<String, Object?>;
    expect(
      renderArgs['contextKey'],
      startsWith(
        'toolpkg_xml_render:module_only_toolpkg:ui/planask/index.ui.js:',
      ),
    );
    final runtimeOptions = renderArgs['runtimeOptions'] as Map<String, Object?>;
    expect(runtimeOptions['uiModuleId'], 'xml_render');
    expect(runtimeOptions['__operit_script_screen'], 'ui/planask/index.ui.js');
  });

  testWidgets('dispatches compose dsl action and renders returned tree', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge();
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Increment'));
    await tester.pumpAndSettle();

    expect(find.text('Counter: 1'), findsOneWidget);

    final actionCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'increment');
    expect(args['payload'], isNull);
    expect(
      actionCall.target,
      GeneratedCoreProxyClients(
        bridge,
      ).servicesComposeDslSessionService.objectId,
    );
    expect(
      args['contextKey'],
      startsWith(
        'toolpkg_compose_dsl:demo_toolpkg:main:screen:demo_toolpkg:main:',
      ),
    );

    final runtimeOptions = args['runtimeOptions'] as Map<String, Object?>;
    expect(runtimeOptions['routeInstanceId'], 'screen:demo_toolpkg:main');
    expect(runtimeOptions.containsKey('state'), isFalse);
    expect(
      runtimeOptions['moduleSpec'],
      containsPair('toolPkgId', 'demo_toolpkg'),
    );
  });

  testWidgets(
    'renders intermediate compose dsl action trees before completion',
    (tester) async {
      final bridge = _ToolPkgDslTestBridge(holdActionCompletion: true);
      await tester.pumpWidget(_screen(bridge));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Increment'));
      await tester.pump();
      await tester.pump();

      expect(find.text('Counter: 1'), findsOneWidget);
      expect(bridge.actionCompletion, isNotNull);

      bridge.actionCompletion!.complete();
      await tester.pumpAndSettle();
    },
  );

  testWidgets('applies a compose dsl switch action result', (tester) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: (_) => _toggleRenderResult(false),
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);

    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();

    expect(tester.widget<Switch>(find.byType(Switch)).value, isTrue);
    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'toggle');
    expect(args['payload'], isTrue);
  });

  testWidgets('renders row fillMaxWidth child with finite text constraints', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _rowFillMaxWidthRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('Wide row text'), findsOneWidget);
    expect(find.text('Tail'), findsOneWidget);
  });

  testWidgets('renders row surface text with finite constraints', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _rowSurfaceRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('开始/暂停'), findsOneWidget);
  });

  testWidgets('wraps direct long text in a bounded row', (tester) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _rowLongTextRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(
      find.text(
        '这里的开关和输入菜单里的“额外信息注入”是同一个状态；你可以分别控制注入项目、是否落盘保存，以及记忆检索是否允许重复命中。',
      ),
      findsOneWidget,
    );
  });

  testWidgets('weighted rows keep short workflow labels on one line', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _weightedWorkflowRowRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(tester.getSize(find.text('已禁用')).height, lessThan(30));
    expect(tester.getSize(find.text('节点')).height, lessThan(30));
    expect(tester.getSize(find.text('工作流标题')).width, greaterThan(100));
  });

  testWidgets('renders text field slots and preserves focused input state', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _textFieldRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Alias'), findsOneWidget);
    expect(find.text('Type alias'), findsOneWidget);
    expect(find.text('https://'), findsOneWidget);
    expect(find.text('.site'), findsOneWidget);
    expect(find.text('Visible to the model'), findsOneWidget);

    await tester.tap(find.byType(TextField));
    await tester.enterText(find.byType(TextField), 'operit');
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'alias_change');
    expect(args['payload'], 'operit');

    final field = tester.widget<TextField>(find.byType(TextField));
    expect(field.controller?.text, 'operit');
    expect(field.minLines, 2);
    expect(field.maxLines, 4);
    expect(field.style?.fontWeight, FontWeight.w600);
  });

  testWidgets(
    'renders text field input options and action gated enabled state',
    (tester) async {
      final bridge = _ToolPkgDslTestBridge(
        renderResult: _textFieldInputOptionsRenderResult,
      );
      await tester.pumpWidget(_screen(bridge));
      await tester.pumpAndSettle();

      final fields = tester
          .widgetList<TextField>(find.byType(TextField))
          .toList();
      expect(fields, hasLength(2));
      expect(fields.first.enabled, isTrue);
      expect(fields.first.keyboardType, TextInputType.emailAddress);
      expect(fields.first.textInputAction, TextInputAction.search);
      expect(fields.last.enabled, isFalse);

      await tester.enterText(find.byType(TextField).first, 'agent@example.com');
      await tester.pumpAndSettle();

      final actionCall = bridge.calls.lastWhere(
        (request) => request.methodName == 'action',
      );
      final args = actionCall.args as Map<String, Object?>;
      expect(args['actionId'], 'email_change');
      expect(args['payload'], 'agent@example.com');
    },
  );

  testWidgets('renders selection controls with actions and switch colors', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _selectionControlsRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('S'), findsOneWidget);
    final switchWidget = tester.widget<Switch>(find.byType(Switch));
    expect(switchWidget.onChanged, isNotNull);
    expect(
      switchWidget.thumbColor?.resolve(<WidgetState>{WidgetState.selected}),
      const Color(0xff4caf50),
    );
    expect(
      switchWidget.trackColor?.resolve(<WidgetState>{}),
      const Color(0xffffcc80),
    );

    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    var actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    var args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'switch_change');
    expect(args['payload'], false);

    await tester.tap(find.byType(Checkbox).first);
    await tester.pumpAndSettle();
    actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'checkbox_change');
    expect(args['payload'], true);

    final disabledCheckbox = tester.widget<Checkbox>(
      find.byType(Checkbox).last,
    );
    expect(disabledCheckbox.onChanged, isNull);

    await tester.tap(find.byWidgetPredicate((widget) => widget is Radio<bool>));
    await tester.pumpAndSettle();
    actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'radio_select');
    expect(args['payload'], isNull);
  });

  testWidgets('renders progress colors and icon image semantics', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _progressImageResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final linear = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator).first,
    );
    expect(linear.value, 1);
    expect(linear.color, const Color(0xff00695c));
    expect(linear.backgroundColor, const Color(0xffb2dfdb));

    final circular = tester.widget<CircularProgressIndicator>(
      find.byType(CircularProgressIndicator).first,
    );
    expect(circular.value, 0);
    expect(circular.strokeWidth, 6);
    expect(circular.color, const Color(0xff5e35b1));
    expect(circular.backgroundColor, const Color(0xffd1c4e9));

    final icon = tester.widget<Icon>(find.byIcon(Icons.settings));
    expect(icon.size, 32);
    expect(icon.color, const Color(0xffff00ff));
    expect(
      tester
          .widget<Opacity>(
            find
                .ancestor(
                  of: find.byIcon(Icons.settings),
                  matching: find.byType(Opacity),
                )
                .first,
          )
          .opacity,
      moreOrLessEquals(0.5),
    );
    expect(find.bySemanticsLabel('Settings image'), findsOneWidget);
  });

  testWidgets('renders badge list item and snackbar slots', (tester) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _badgeListSnackbarRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('7'), findsOneWidget);
    expect(find.text('Inbox'), findsOneWidget);
    expect(find.text('3'), findsOneWidget);
    expect(find.text('OVERLINE'), findsOneWidget);
    expect(find.text('Headline'), findsOneWidget);
    expect(find.text('Supporting copy'), findsOneWidget);
    expect(find.text('Lead'), findsOneWidget);
    expect(find.text('Trail'), findsOneWidget);
    expect(find.text('Snack content'), findsOneWidget);
    expect(find.text('Undo'), findsOneWidget);
    expect(find.text('Dismiss'), findsOneWidget);

    final badges = tester.widgetList<Badge>(find.byType(Badge)).toList();
    expect(badges.first.backgroundColor, const Color(0xffd32f2f));
    expect(badges.first.textColor, const Color(0xffffffff));

    await tester.tap(find.text('Undo'));
    await tester.pumpAndSettle();
    var actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    var args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'snackbar_undo');
    expect(args['payload'], isNull);

    await tester.tap(find.text('Dismiss'));
    await tester.pumpAndSettle();
    actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'snackbar_dismiss');
    expect(args['payload'], isNull);
  });

  testWidgets('renders expanded dropdown menu content and selection action', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _dropdownRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Mode'), findsOneWidget);
    expect(find.text('Option A'), findsOneWidget);
    expect(find.text('Option B'), findsOneWidget);

    await tester.tap(find.text('Option B'));
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'select_mode');
    expect(args['payload'], 1);
  });

  testWidgets('renders pull to refresh indicator slot while refreshing', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _refreshRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Refresh content'), findsOneWidget);
    expect(find.text('Refreshing now'), findsOneWidget);
  });

  testWidgets('reports canvas size changes through action payload', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _canvasSizeRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'canvas_size');

    final payload = args['payload'] as Map<String, Object?>;
    expect(payload['width'], 80);
    expect(payload['height'], 48);
  });

  testWidgets('renders provide text style as inherited text context', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _provideTextStyleRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Styled slot'), findsOneWidget);
    expect(
      find.byWidgetPredicate(
        (widget) =>
            widget is DefaultTextStyle &&
            widget.style.fontSize == 18 &&
            widget.style.fontWeight == FontWeight.w700,
      ),
      findsOneWidget,
    );
  });

  testWidgets('renders time picker dialog slots and dismiss request', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _timePickerDialogRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Pick time'), findsOneWidget);
    expect(find.text('Clock content'), findsOneWidget);
    expect(find.text('Keyboard'), findsOneWidget);
    expect(find.text('Cancel'), findsOneWidget);
    expect(find.text('OK'), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'dismiss_time_picker');
    expect(args['payload'], isNull);
  });

  testWidgets('renders scaffold slots and content color context', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _scaffoldRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Top tools'), findsOneWidget);
    expect(find.text('Scaffold body'), findsOneWidget);
    expect(find.text('Bottom tools'), findsOneWidget);
    expect(find.text('Snack zone'), findsOneWidget);
    expect(find.text('Add'), findsOneWidget);

    final bodyElement = tester.element(find.text('Scaffold body'));
    expect(
      DefaultTextStyle.of(bodyElement).style.color,
      const Color(0xffe91e63),
    );
  });

  testWidgets('renders card and surface colors borders and content slots', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _cardSurfaceRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final card = tester.widget<Card>(find.byType(Card).first);
    expect(card.color?.a, moreOrLessEquals(0.5));
    expect(card.color?.r, moreOrLessEquals(0x10 / 255));
    expect(card.color?.g, moreOrLessEquals(0x20 / 255));
    expect(card.color?.b, moreOrLessEquals(0x30 / 255));
    final cardShape = card.shape as RoundedRectangleBorder;
    expect(cardShape.side.width, 2);
    expect(cardShape.side.color.a, moreOrLessEquals(0.25));
    expect(cardShape.side.color.r, moreOrLessEquals(1));
    expect(find.text('Card content'), findsOneWidget);
    final cardTextStyle = tester.widget<DefaultTextStyle>(
      find
          .ancestor(
            of: find.text('Card content'),
            matching: find.byType(DefaultTextStyle),
          )
          .first,
    );
    expect(cardTextStyle.style.color, const Color(0xffffffff));

    final surfaceMaterial = tester.widget<Material>(
      find
          .ancestor(
            of: find.text('Surface content'),
            matching: find.byType(Material),
          )
          .first,
    );
    expect(surfaceMaterial.color?.a, moreOrLessEquals(0.75));
    expect(surfaceMaterial.color?.r, moreOrLessEquals(0xf5 / 255));
    expect(surfaceMaterial.elevation, 7);
    final surfaceTextStyle = tester.widget<DefaultTextStyle>(
      find
          .ancestor(
            of: find.text('Surface content'),
            matching: find.byType(DefaultTextStyle),
          )
          .first,
    );
    expect(surfaceTextStyle.style.color, const Color(0xff00695c));
    expect(
      find.ancestor(
        of: find.text('Surface content'),
        matching: find.byType(Opacity),
      ),
      findsNothing,
    );

    await tester.tap(find.text('Surface content'));
    await tester.pumpAndSettle();
    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'surface_click');
  });

  testWidgets('renders button colors slots disabled state and border', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _buttonStyleRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final filled = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Slot Button'),
    );
    expect(
      filled.style?.backgroundColor?.resolve(<WidgetState>{}),
      const Color(0xff00695c),
    );
    expect(
      filled.style?.foregroundColor?.resolve(<WidgetState>{}),
      const Color(0xffffffff),
    );

    final disabled = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Disabled Button'),
    );
    expect(disabled.onPressed, isNull);
    expect(
      disabled.style?.backgroundColor?.resolve(<WidgetState>{
        WidgetState.disabled,
      }),
      const Color(0xff78909c),
    );
    expect(
      disabled.style?.foregroundColor?.resolve(<WidgetState>{
        WidgetState.disabled,
      }),
      const Color(0xff263238),
    );

    final outlined = tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, 'Outline'),
    );
    final side = outlined.style?.side?.resolve(<WidgetState>{});
    expect(side?.width, 3);
    expect(side?.color, const Color(0xffe91e63));

    await tester.tap(find.text('Slot Button'));
    await tester.pumpAndSettle();
    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'slot_button');
  });

  testWidgets('renders floating action button size variants and colors', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _fabVariantsRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final fabFinder = find.byType(FloatingActionButton);
    expect(fabFinder, findsNWidgets(3));

    expect(tester.getSize(fabFinder.at(0)), const Size(48, 48));
    expect(tester.getSize(fabFinder.at(1)), const Size(56, 56));
    expect(tester.getSize(fabFinder.at(2)), const Size(96, 96));

    final smallWidget = tester.widget<FloatingActionButton>(fabFinder.at(0));
    expect(smallWidget.backgroundColor, const Color(0xff4caf50));
    expect(smallWidget.foregroundColor, const Color(0xffffffff));
    expect(smallWidget.shape, isA<RoundedRectangleBorder>());
  });

  testWidgets('renders icon toggle selected state and checked change action', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _iconToggleRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Selected toggle'), findsOneWidget);
    expect(find.text('Unchecked toggle'), findsNothing);

    final selectedIconButtonFinder = find.byWidgetPredicate(
      (widget) => widget is IconButton && widget.isSelected == true,
    );
    expect(selectedIconButtonFinder, findsOneWidget);

    final selectedIconButton = tester.widget<IconButton>(
      selectedIconButtonFinder,
    );
    expect(selectedIconButton.selectedIcon, isNotNull);
    expect(
      selectedIconButton.style?.shape?.resolve(<WidgetState>{}),
      isA<RoundedRectangleBorder>(),
    );

    await tester.tap(find.text('Selected toggle'));
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'toggle_icon');
    expect(args['payload'], false);
  });

  for (final contentLocation in ['children', 'slot']) {
    testWidgets('renders icon button custom content from $contentLocation', (
      tester,
    ) async {
      final icon = _node('Icon', props: {'name': 'delete', 'size': 16});
      final bridge = _ToolPkgDslTestBridge(
        renderResult: (_) => jsonEncode({
          'success': true,
          'tree': _node(
            'Row',
            children: [
              _node(
                'IconButton',
                props: {
                  'width': 32,
                  'height': 32,
                  'onClick': {'__actionId': 'delete'},
                },
                children: contentLocation == 'children' ? [icon] : [],
                slots: contentLocation == 'slot'
                    ? {
                        'content': [icon],
                      }
                    : {},
              ),
            ],
          ),
        }),
      );
      await tester.pumpWidget(_screen(bridge));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.byType(ErrorWidget), findsNothing);
      final iconFinder = find.byIcon(Icons.delete);
      final buttonFinder = find.byType(IconButton);
      expect(tester.widget<Icon>(iconFinder).size, 16);
      expect(tester.getSize(buttonFinder), const Size(32, 32));
      expect(tester.getCenter(iconFinder), tester.getCenter(buttonFinder));
      await tester.tap(buttonFinder);
      await tester.pumpAndSettle();
      final action = bridge.calls.lastWhere(
        (call) => call.methodName == 'action',
      );
      expect((action.args as Map<String, Object?>)['actionId'], 'delete');
    });
  }

  for (final invalidType in ['Icon', 'IconButton']) {
    testWidgets(
      'contains invalid plugin $invalidType without breaking sibling actions',
      (tester) async {
        final bridge = _ToolPkgDslTestBridge(
          renderResult: (count) => jsonEncode({
            'success': true,
            'tree': _node(
              'Row',
              children: [
                _node('Text', props: {'text': 'Healthy sibling $count'}),
                _node(
                  invalidType,
                  props: {
                    'name': 'not_a_material_icon',
                    'icon': 'not_a_material_icon',
                    'width': 32,
                    'height': 32,
                  },
                ),
                _node(
                  'IconButton',
                  props: {
                    'icon': 'add',
                    'onClick': {'__actionId': 'increment'},
                  },
                ),
              ],
            ),
          }),
        );
        await tester.pumpWidget(_screen(bridge));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        expect(find.byType(ErrorWidget), findsNothing);
        expect(find.text('Healthy sibling 0'), findsOneWidget);
        await tester.tap(find.byIcon(Icons.add));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        expect(find.byType(ErrorWidget), findsNothing);
        expect(find.text('Healthy sibling 1'), findsOneWidget);
      },
    );
  }

  testWidgets('applies explicit size to a plain icon button', (tester) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _plainIconButtonRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final finder = find.byType(IconButton);
    expect(finder, findsOneWidget);
    expect(tester.getSize(finder), const Size(32, 32));

    final button = tester.widget<IconButton>(finder);
    expect(button.style?.minimumSize?.resolve(<WidgetState>{}), Size.zero);
    expect(button.style?.padding?.resolve(<WidgetState>{}), EdgeInsets.zero);
    expect(button.style?.tapTargetSize, MaterialTapTargetSize.shrinkWrap);
    final iconFinder = find.descendant(
      of: finder,
      matching: find.byIcon(Icons.close),
    );
    expect(iconFinder, findsOneWidget);
    expect(tester.getCenter(iconFinder), tester.getCenter(finder));
  });

  testWidgets('renders navigation item slots and dispatches item actions', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _navigationRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Bar selected icon'), findsOneWidget);
    expect(find.text('Bar inactive icon'), findsOneWidget);
    expect(find.text('Bar badge'), findsOneWidget);
    expect(find.text('Rail header'), findsOneWidget);
    expect(find.text('Rail selected icon'), findsOneWidget);
    expect(find.text('Drawer icon'), findsOneWidget);
    expect(find.text('Drawer badge'), findsOneWidget);

    await tester.tap(find.text('Bar inactive'));
    await tester.pumpAndSettle();

    final barActionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final barArgs = barActionCall.args as Map<String, Object?>;
    expect(barArgs['actionId'], 'bar_inactive');
    expect(barArgs['payload'], isNull);

    await tester.tap(find.text('Drawer item'));
    await tester.pumpAndSettle();

    final drawerActionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final drawerArgs = drawerActionCall.args as Map<String, Object?>;
    expect(drawerArgs['actionId'], 'drawer_item');
    expect(drawerArgs['payload'], isNull);
  });

  testWidgets('renders tab row slots and tab action semantics', (tester) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _tabsRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Overview tab'), findsOneWidget);
    expect(find.text('Details tab'), findsOneWidget);
    expect(find.text('Leading icon'), findsOneWidget);
    expect(find.text('Leading text'), findsOneWidget);
    expect(find.text('Tab indicator'), findsOneWidget);
    expect(find.text('Tab divider'), findsOneWidget);

    final selectedTextElement = tester.element(find.text('Details tab'));
    expect(
      DefaultTextStyle.of(selectedTextElement).style.color,
      const Color(0xffe91e63),
    );

    await tester.tap(find.text('Details tab'));
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'details_tab');
    expect(args['payload'], isNull);
  });

  testWidgets('renders chip family slots and actions', (tester) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _chipsRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Assist leading'), findsOneWidget);
    expect(find.text('Assist label'), findsOneWidget);
    expect(find.text('Assist trailing'), findsOneWidget);
    expect(find.text('Filter leading'), findsOneWidget);
    expect(find.text('Filter label'), findsOneWidget);
    expect(find.text('Input avatar'), findsOneWidget);
    expect(find.text('Input label'), findsOneWidget);
    expect(find.text('Input dismiss'), findsOneWidget);
    expect(find.text('Suggestion icon'), findsOneWidget);
    expect(find.text('Suggestion label'), findsOneWidget);

    final assistChip = tester.widget<ActionChip>(find.byType(ActionChip).first);
    expect(assistChip.backgroundColor, const Color(0xffe3f2fd));
    expect(assistChip.labelStyle?.color, const Color(0xff0d47a1));
    expect(assistChip.shape, isA<RoundedRectangleBorder>());

    final filterChip = tester.widget<FilterChip>(find.byType(FilterChip));
    expect(filterChip.selected, isTrue);
    expect(filterChip.selectedColor, const Color(0xfffff59d));

    await tester.tap(find.byType(FilterChip));
    await tester.pumpAndSettle();

    final filterActionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final filterArgs = filterActionCall.args as Map<String, Object?>;
    expect(filterArgs['actionId'], 'filter_chip');
    expect(filterArgs['payload'], isNull);

    await tester.tap(
      find.byWidgetPredicate(
        (widget) =>
            widget is Icon && widget.icon == Icons.close && widget.size == 18,
      ),
    );
    await tester.pumpAndSettle();

    final dismissActionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'action',
    );
    final dismissArgs = dismissActionCall.args as Map<String, Object?>;
    expect(dismissArgs['actionId'], 'dismiss_input_chip');
    expect(dismissArgs['payload'], isNull);
  });
}

Widget _screen(
  _ToolPkgDslTestBridge bridge, {
  core_proxy.ToolPkgContainerRuntime? plugin,
  String? initialRouteId,
  String? embeddedScreenPath,
  Map<String, Object?>? initialModuleSpec,
}) {
  return MaterialApp(
    home: ToolPkgUiLauncherScreen(
      clients: GeneratedCoreProxyClients(bridge),
      plugin: plugin ?? _pluginRuntime(),
      initialRouteId: initialRouteId,
      embeddedScreenPath: embeddedScreenPath,
      initialModuleSpec: initialModuleSpec,
    ),
  );
}

core_proxy.ToolPkgContainerRuntime _pluginRuntime() {
  return const core_proxy.ToolPkgContainerRuntime(
    chatLifecycleHooks: [],
    packageName: 'demo_toolpkg',
    displayName: core_proxy.LocalizedText(
      values: <String, String>{'default': 'Demo ToolPkg'},
    ),
    description: core_proxy.LocalizedText(
      values: <String, String>{'default': 'DSL test package'},
    ),
    version: '1.0.0',
    apiVersion: '2.0.0',
    publicApi: null,
    publicApis: <core_proxy.ToolPkgRegisteredFunctionHook>[],
    requires: <core_proxy.ToolPkgManifestRequirement>[],
    dependencyIssues: <core_proxy.ToolPkgDependencyIssue>[],
    manifestExtensions: <String, Object?>{},
    author: <String>['Operit'],
    mainEntry: 'dist/main.js',
    sourceType: core_proxy.ToolPkgSourceType.externalValue,
    sourcePath: 'test',
    subpackages: <core_proxy.ToolPkgSubpackageRuntime>[],
    resources: <core_proxy.ToolPkgResourceRuntime>[],
    wasmModules: <core_proxy.ToolPkgWasmModuleRuntime>[],
    workflowTemplates: <core_proxy.ToolPkgWorkflowTemplateRuntime>[],
    workspaceTemplates: <core_proxy.ToolPkgWorkspaceTemplateRuntime>[],
    uiModules: <core_proxy.ToolPkgUiModuleRuntime>[
      core_proxy.ToolPkgUiModuleRuntime(
        id: 'main',
        runtime: 'compose_dsl',
        screenExport: null,
        screen: 'ui/main.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Main route'},
        ),
        keepAlive: true,
      ),
    ],
    uiRoutes: <core_proxy.ToolPkgUiRouteRuntime>[
      core_proxy.ToolPkgUiRouteRuntime(
        id: 'main',
        routeId: 'main',
        runtime: 'compose_dsl',
        screenExport: null,
        screen: 'ui/main.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Main route'},
        ),
        keepAlive: true,
      ),
    ],
    chatComposerSlots: <core_proxy.ToolPkgChatComposerSlotRuntime>[],
    navigationEntries: <core_proxy.ToolPkgNavigationEntryRuntime>[],
    desktopWidgets: <core_proxy.ToolPkgDesktopWidgetRuntime>[],
    appLifecycleHooks: <core_proxy.ToolPkgAppLifecycleHookRuntime>[],
    messageProcessingPlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    xmlRenderPlugins: <core_proxy.ToolPkgTagFunctionHookRuntime>[],
    inputMenuTogglePlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatViewHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatMessageHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatMessageMenuItems: <core_proxy.ToolPkgChatMessageMenuItemRuntime>[],
    chatRuntimeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    hostEventHooks: <core_proxy.ToolPkgHostEventHookRuntime>[],
    toolLifecycleHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    systemPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    toolPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    summaryGenerateHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    coreCommands: <core_proxy.ToolPkgCoreCommandRuntime>[],
    aiProviders: <core_proxy.ToolPkgAiProviderRuntime>[],
    manifestExtensionHandlers:
        <core_proxy.ToolPkgRegisteredManifestExtension>[],
    logoResource: null,
    marketOrigin: null,
  );
}

core_proxy.ToolPkgContainerRuntime _moduleOnlyPluginRuntime() {
  return const core_proxy.ToolPkgContainerRuntime(
    chatLifecycleHooks: [],
    packageName: 'module_only_toolpkg',
    displayName: core_proxy.LocalizedText(
      values: <String, String>{'default': 'Module ToolPkg'},
    ),
    description: core_proxy.LocalizedText(
      values: <String, String>{'default': 'Module only package'},
    ),
    version: '1.0.0',
    apiVersion: '2.0.0',
    publicApi: null,
    publicApis: <core_proxy.ToolPkgRegisteredFunctionHook>[],
    requires: <core_proxy.ToolPkgManifestRequirement>[],
    dependencyIssues: <core_proxy.ToolPkgDependencyIssue>[],
    manifestExtensions: <String, Object?>{},
    author: <String>['Operit'],
    mainEntry: 'dist/main.js',
    sourceType: core_proxy.ToolPkgSourceType.externalValue,
    sourcePath: 'test',
    subpackages: <core_proxy.ToolPkgSubpackageRuntime>[],
    resources: <core_proxy.ToolPkgResourceRuntime>[],
    wasmModules: <core_proxy.ToolPkgWasmModuleRuntime>[],
    workflowTemplates: <core_proxy.ToolPkgWorkflowTemplateRuntime>[],
    workspaceTemplates: <core_proxy.ToolPkgWorkspaceTemplateRuntime>[],
    uiModules: <core_proxy.ToolPkgUiModuleRuntime>[
      core_proxy.ToolPkgUiModuleRuntime(
        id: 'toolbox',
        runtime: 'compose_dsl',
        screenExport: null,
        screen: 'ui/toolbox.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Toolbox module'},
        ),
        keepAlive: true,
      ),
    ],
    uiRoutes: <core_proxy.ToolPkgUiRouteRuntime>[],
    chatComposerSlots: <core_proxy.ToolPkgChatComposerSlotRuntime>[],
    navigationEntries: <core_proxy.ToolPkgNavigationEntryRuntime>[],
    desktopWidgets: <core_proxy.ToolPkgDesktopWidgetRuntime>[],
    appLifecycleHooks: <core_proxy.ToolPkgAppLifecycleHookRuntime>[],
    messageProcessingPlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    xmlRenderPlugins: <core_proxy.ToolPkgTagFunctionHookRuntime>[],
    inputMenuTogglePlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatViewHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatMessageHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatMessageMenuItems: <core_proxy.ToolPkgChatMessageMenuItemRuntime>[],
    chatRuntimeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    hostEventHooks: <core_proxy.ToolPkgHostEventHookRuntime>[],
    toolLifecycleHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    systemPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    toolPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    summaryGenerateHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    coreCommands: <core_proxy.ToolPkgCoreCommandRuntime>[],
    aiProviders: <core_proxy.ToolPkgAiProviderRuntime>[],
    manifestExtensionHandlers:
        <core_proxy.ToolPkgRegisteredManifestExtension>[],
    logoResource: null,
    marketOrigin: null,
  );
}

/// Creates a selected message whose tool and thinking parts must stay out of translation.
core_proxy.ChatMessage _translationMessage(String sender) {
  return core_proxy.ChatMessage(
    pluginExtensions: const {},
    sender: sender,
    parts: [
      for (final entry in [
        (core_proxy.MessagePartKind.thinking, 'Private reasoning'),
        (core_proxy.MessagePartKind.toolResult, 'Tool output'),
        (core_proxy.MessagePartKind.markdown, 'Selected message'),
      ].indexed)
        core_proxy.MessagePart(
          partId: 'part-${entry.$1}',
          sequence: entry.$1,
          kind: entry.$2.$1,
          content: entry.$2.$2,
          toolCallId: null,
          toolName: null,
          attributes: const {},
        ),
    ],
    timestamp: 100,
    roleName: '',
    selectedVariantIndex: 0,
    variantCount: 1,
    provider: '',
    modelName: '',
    inputTokens: 0,
    outputTokens: 0,
    cachedInputTokens: 0,
    sentAt: 100,
    outputDurationMs: 0,
    waitDurationMs: 0,
    completedAt: 101,
    displayMode: core_proxy.ChatMessageDisplayMode.normal,
    isFavorite: false,
    contentStream: null,
  );
}

class _ToolPkgDslTestBridge extends OperitRuntimeBridge {
  _ToolPkgDslTestBridge({
    String Function(int count)? renderResult,
    this.holdActionCompletion = false,
    this.closeBeforeActionComplete = false,
    this.onAction,
  }) : _renderResult = renderResult ?? _counterRenderResult;

  final List<CoreCallRequest> calls = <CoreCallRequest>[];
  final String Function(int count) _renderResult;
  final bool holdActionCompletion;
  final bool closeBeforeActionComplete;
  final void Function(Map<String, Object?>)? onAction;
  Completer<void>? actionCompletion;
  Completer<String>? renderCompletion;
  var _count = 0;
  var _checked = false;
  final _sessions = <String, _DslFixtureSession>{};

  /// Encodes decoded test responses using the current native bridge envelope.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    return encodeCoreLink(<Object?>[0, await call(request)]);
  }

  @override
  Future<Object?> call(CoreCallRequest request) async {
    calls.add(request);
    switch (request.methodName) {
      case 'getToolPkgChatMessageMenuItems':
        return [
          const core_proxy.ToolPkgChatMessageMenuItem(
            containerPackageName: 'demo_toolpkg',
            itemId: 'translate_message',
            title: 'Translate message',
            icon: 'translate',
            order: 20,
            dialog: core_proxy.ToolPkgChatMessageMenuDialog(
              screen: 'ui/translate.js',
              title: 'Translation dialog',
            ),
          ).toJson(),
        ];
      case 'invokeToolPkgChatMessageMenuItem':
        return jsonEncode({
          'dialog': {'state': {}, 'moduleSpec': {}},
        });
      case 'getToolPkgContainerRuntime':
        return _pluginRuntime().toJson();
      case 'renderToolPkgDesktopWidget':
        return {
          'widget': const core_proxy.ToolPkgDesktopWidget(
            containerPackageName: 'demo_toolpkg',
            toolPkgId: 'demo_toolpkg',
            widgetId: 'summary',
            routeId: 'main',
            renderRouteId: 'widget_summary',
            title: 'Summary',
            subtitle: '',
            description: '',
            icon: null,
            order: 0,
          ).toJson(),
          'renderUpdate': composeFixtureUpdate(
            _node('Text', props: {'text': 'Widget loaded'}),
            1,
          ).toJson(),
        };
      case 'openComposeDslSession':
        final id = 'dsl-fixture-${_sessions.length}';
        _sessions[id] = _DslFixtureSession();
        return id;
      case 'close':
        final id = (request.args as Map)['sessionId'] as String;
        await _sessions[id]!.updates.close();
        return null;
      case 'acquireToolPkgExecutionEngine':
      case 'releaseToolPkgExecutionEngine':
        return null;
      case 'getToolPkgComposeDslScript':
        return 'export default function render() {}';
      case 'readToolPkgTextResource':
        return 'export default function render() {}';
      case 'getToolPkgComposeDslScreenPath':
        final args = request.args as Map<String, Object?>;
        return args['uiModuleId'] == 'toolbox' ? 'ui/toolbox.js' : 'ui/main.js';
    }
    throw StateError('unexpected core call: ${request.methodName}');
  }

  /// Attaches the current generated reverse command stream to its fixture session.
  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    if (request.methodName != 'submit')
      throw StateError('unexpected push: ${request.methodName}');
    return _DslCommandSink(
      this,
      _sessions[(request.args as Map)['sessionId']]!,
    );
  }

  /// Executes a typed command and publishes updates using the actual session protocol.
  Future<void> handle(_DslFixtureSession session, Object? raw) async {
    final command = core_proxy.ToolPkgComposeDslCommand.fromJson(
      Map<String, Object?>.from(raw as Map),
    );
    final options = command.runtimeOptions;
    final args = <String, Object?>{
      'contextKey': options['executionContextKey'],
      'containerPackageName': options['packageName'],
      'script': command.script,
      'actionId': command.actionId,
      'payload': command.payload,
      'runtimeOptions': options,
    };
    calls.add(
      CoreCallRequest(
        requestId: command.requestId,
        target: session.target,
        methodName: command.operation,
        args: args,
      ),
    );
    try {
      String encoded;
      if (command.operation == 'render') {
        final pending = renderCompletion;
        if (pending != null)
          encoded = await pending.future;
        else
          encoded = _renderResult(_count);
      } else if (command.operation == 'action') {
        onAction?.call(args);
        if (command.actionId == 'increment') _count++;
        if (command.actionId == 'toggle') _checked = command.payload as bool;
        encoded = command.actionId == 'toggle'
            ? _toggleRenderResult(_checked)
            : _renderResult(_count);
      } else {
        throw StateError('Unexpected operation: ${command.operation}');
      }
      final result = jsonDecode(encoded) as Map<String, Object?>;
      final tree = result['tree'] as Map<String, Object?>;

      /// Publishes a flat commit and routes side effects only on the final response.
      void emit(String phase) {
        session.emit(
          core_proxy.ToolPkgComposeDslEvent(
            requestId: command.requestId,
            phase: phase,
            update: composeFixtureUpdate(
              tree,
              ++session.revision,
              reset: session.revision == 1,
            ),
            actionResult: phase == 'final' ? result['actionResult'] : null,
            navigationCommands:
                phase == 'final' && result.containsKey('navigationCommands')
                ? (result['navigationCommands'] as List).cast<Object?>()
                : const [],
            error: null,
          ),
        );
      }

      if (command.operation == 'action') {
        emit('intermediate');
        if (holdActionCompletion) {
          actionCompletion = Completer<void>();
          await actionCompletion!.future;
        }
      }
      emit('final');
      if (command.operation == 'action' && closeBeforeActionComplete) {
        await session.updates.close();
        return;
      }
    } catch (error) {
      session.emit(
        core_proxy.ToolPkgComposeDslEvent(
          requestId: command.requestId,
          phase: 'error',
          update: null,
          actionResult: null,
          navigationCommands: const [],
          error: '$error',
        ),
      );
    }
    session.emit(
      core_proxy.ToolPkgComposeDslEvent(
        requestId: command.requestId,
        phase: 'complete',
        update: null,
        actionResult: null,
        navigationCommands: const [],
        error: null,
      ),
    );
  }

  /// Rejects embedded streams because this test models direct DSL watches.
  @override
  Stream<T> openEmbeddedCoreStream<T>(
    String streamId,
    String target,
    String propertyName,
    Object? args,
    T Function(CoreLinkValueReader reader) decode,
  ) {
    throw StateError('unexpected embedded core stream: $propertyName');
  }

  /// Returns the initial event for a direct DSL watch.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    return CoreEvent(
      requestId: request.requestId,
      target: request.target,
      propertyName: request.propertyName,
      kind: 'Snapshot',
      value: null,
    );
  }

  /// Exposes the sole typed downstream session stream through the existing Core watch protocol.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName != 'updates')
      throw StateError('unexpected watch: ${request.propertyName}');
    final session = _sessions[(request.args as Map)['sessionId']]!;
    session.target = request.target;
    return session.updates.stream.map(
      (event) => CoreEvent.raw(
        requestId: request.requestId,
        target: request.target,
        propertyName: request.propertyName,
        kind: 'Changed',
        decodeValue: decodeCoreLink<Object?>,
        valueBytes: encodeCoreLink(event.toJson()),
      ),
    );
  }
}

/// Holds typed stream ownership independently of a command's completion.
class _DslFixtureSession {
  final updates =
      StreamController<core_proxy.ToolPkgComposeDslEvent>.broadcast();
  String target = '';
  int revision = 0;

  /// Publishes only while the fixture session remains open.
  void emit(core_proxy.ToolPkgComposeDslEvent event) {
    if (!updates.isClosed) updates.add(event);
  }
}

/// Implements only the current reverse stream sink for launcher regression tests.
class _DslCommandSink implements CorePushSink {
  /// Binds one sink to its live fixture session.
  _DslCommandSink(this.bridge, this.session);
  final _ToolPkgDslTestBridge bridge;
  final _DslFixtureSession session;

  /// Admits a typed command without waiting for its asynchronous action to finish.
  @override
  Future<void> add(Object? value) async {
    unawaited(bridge.handle(session, value));
  }

  /// Ends updates when the launcher releases its reverse stream owner.
  @override
  Future<void> close() async {
    if (!session.updates.isClosed) await session.updates.close();
  }
}

String _counterRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'Card',
          children: <Map<String, Object?>>[
            _node('Text', props: <String, Object?>{'text': 'Counter: $count'}),
            _node(
              'Button',
              props: <String, Object?>{
                'text': 'Increment',
                'onClick': <String, Object?>{'__actionId': 'increment'},
              },
            ),
          ],
        ),
        _node(
          'Canvas',
          props: <String, Object?>{
            'width': 80,
            'height': 48,
            'commands': <Map<String, Object?>>[
              <String, Object?>{
                'type': 'drawRect',
                'x': 0,
                'y': 0,
                'width': 80,
                'height': 48,
                'color': '#2196f3',
              },
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

/// Builds a switch result used to verify action events update the widget tree.
String _toggleRenderResult(bool checked) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Switch',
      props: <String, Object?>{
        'checked': checked,
        'onCheckedChange': <String, Object?>{'__actionId': 'toggle'},
      },
    ),
    'state': <String, Object?>{'checked': checked},
    'memo': const <String, Object?>{},
  });
}

String _rowFillMaxWidthRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'Box',
          props: <String, Object?>{
            'fillMaxWidth': true,
            'padding': <String, Object?>{'horizontal': 12, 'vertical': 8},
          },
          children: <Map<String, Object?>>[
            _node('Text', props: <String, Object?>{'text': 'Wide row text'}),
          ],
        ),
        _node('Text', props: <String, Object?>{'text': 'Tail'}),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _rowSurfaceRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'Surface',
          props: <String, Object?>{
            'containerColor': '#5F6368EE',
            'contentColor': '#F7F7F7',
            'shape': <String, Object?>{'type': 'pill'},
            'padding': <String, Object?>{'horizontal': 22, 'vertical': 14},
          },
          children: <Map<String, Object?>>[
            _node(
              'Text',
              props: <String, Object?>{'text': '开始/暂停', 'style': 'labelLarge'},
            ),
          ],
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

/// Builds workflow labels beside an explicitly weighted title at card width.
String _weightedWorkflowRowRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Box',
      children: <Map<String, Object?>>[
        _node(
          'Row',
          props: <String, Object?>{'width': 320, 'spacing': 8},
          children: <Map<String, Object?>>[
            _node(
              'Text',
              props: <String, Object?>{
                'text': '工作流标题',
                'weight': 1,
                'maxLines': 1,
              },
            ),
            _node('Text', props: <String, Object?>{'text': '已禁用'}),
            _node('Text', props: <String, Object?>{'text': '节点'}),
            _node('Switch', props: <String, Object?>{'checked': false}),
          ],
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

/// Builds a bounded Row containing the long banner text used by message_insert.
String _rowLongTextRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Surface',
      props: <String, Object?>{'fillMaxWidth': true},
      children: <Map<String, Object?>>[
        _node(
          'Row',
          props: <String, Object?>{
            'padding': <String, Object?>{'horizontal': 14, 'vertical': 12},
          },
          children: <Map<String, Object?>>[
            _node('Icon', props: <String, Object?>{'name': 'info', 'size': 18}),
            _node('Spacer', props: <String, Object?>{'width': 8}),
            _node(
              'Text',
              props: <String, Object?>{
                'text':
                    '这里的开关和输入菜单里的“额外信息注入”是同一个状态；你可以分别控制注入项目、是否落盘保存，以及记忆检索是否允许重复命中。',
                'softWrap': true,
              },
            ),
          ],
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _textFieldRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'TextField',
      props: <String, Object?>{
        'key': 'alias-field',
        'value': 'initial',
        'minLines': 2,
        'maxLines': 4,
        'style': <String, Object?>{
          'fontSize': 18,
          'fontWeight': 'semibold',
          'color': '#1b5e20',
        },
        'onValueChange': <String, Object?>{'__actionId': 'alias_change'},
      },
      slots: <String, List<Map<String, Object?>>>{
        'label': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Alias'}),
        ],
        'placeholder': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Type alias'}),
        ],
        'prefix': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'https://'}),
        ],
        'suffix': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': '.site'}),
        ],
        'supportingText': <Map<String, Object?>>[
          _node(
            'Text',
            props: <String, Object?>{'text': 'Visible to the model'},
          ),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _textFieldInputOptionsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'TextField',
          props: <String, Object?>{
            'value': 'agent',
            'keyboardType': 'email',
            'keyboardAction': 'search',
            'onValueChange': <String, Object?>{'__actionId': 'email_change'},
          },
        ),
        _node(
          'TextField',
          props: <String, Object?>{'value': 'locked', 'keyboardType': 'number'},
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _selectionControlsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'Switch',
          props: <String, Object?>{
            'checked': true,
            'checkedThumbColor': '#4caf50',
            'checkedTrackColor': '#a5d6a7',
            'uncheckedThumbColor': '#ff9800',
            'uncheckedTrackColor': '#ffcc80',
            'onCheckedChange': <String, Object?>{'__actionId': 'switch_change'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'thumbContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'S'}),
            ],
          },
        ),
        _node(
          'Checkbox',
          props: <String, Object?>{
            'checked': false,
            'onCheckedChange': <String, Object?>{
              '__actionId': 'checkbox_change',
            },
          },
        ),
        _node('Checkbox', props: <String, Object?>{'checked': true}),
        _node(
          'RadioButton',
          props: <String, Object?>{
            'selected': false,
            'onClick': <String, Object?>{'__actionId': 'radio_select'},
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _progressImageResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'LinearProgressIndicator',
          props: <String, Object?>{
            'progress': 1.25,
            'color': '#00695c',
            'trackColor': '#b2dfdb',
          },
        ),
        _node(
          'CircularProgressIndicator',
          props: <String, Object?>{
            'progress': -0.25,
            'strokeWidth': 6,
            'color': '#5e35b1',
            'trackColor': '#d1c4e9',
          },
        ),
        _node(
          'Image',
          props: <String, Object?>{
            'name': 'settings',
            'tint': '#ff00ff',
            'size': 32,
            'alpha': 0.5,
            'contentDescription': 'Settings image',
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _badgeListSnackbarRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'Badge',
          props: <String, Object?>{
            'containerColor': '#d32f2f',
            'contentColor': '#ffffff',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': '7'}),
            ],
          },
        ),
        _node(
          'BadgedBox',
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Inbox'}),
            ],
            'badge': <Map<String, Object?>>[
              _node(
                'Badge',
                slots: <String, List<Map<String, Object?>>>{
                  'content': <Map<String, Object?>>[
                    _node('Text', props: <String, Object?>{'text': '3'}),
                  ],
                },
              ),
            ],
          },
        ),
        _node(
          'ListItem',
          props: <String, Object?>{'shadowElevation': 2},
          slots: <String, List<Map<String, Object?>>>{
            'overlineContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'OVERLINE'}),
            ],
            'headlineContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Headline'}),
            ],
            'supportingContent': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Supporting copy'},
              ),
            ],
            'leadingContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Lead'}),
            ],
            'trailingContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Trail'}),
            ],
          },
        ),
        _node(
          'Snackbar',
          props: <String, Object?>{
            'actionOnNewLine': true,
            'shape': 8,
            'containerColor': '#263238',
            'contentColor': '#ffffff',
            'actionContentColor': '#80deea',
            'dismissActionContentColor': '#ffab91',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Snack content'}),
            ],
            'action': <Map<String, Object?>>[
              _node(
                'TextButton',
                props: <String, Object?>{
                  'text': 'Undo',
                  'onClick': <String, Object?>{'__actionId': 'snackbar_undo'},
                },
              ),
            ],
            'dismissAction': <Map<String, Object?>>[
              _node(
                'TextButton',
                props: <String, Object?>{
                  'text': 'Dismiss',
                  'onClick': <String, Object?>{
                    '__actionId': 'snackbar_dismiss',
                  },
                },
              ),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _dropdownRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'DropdownMenu',
      props: <String, Object?>{
        'expanded': true,
        'label': 'Mode',
        'text': 'Choose',
        'onClick': <String, Object?>{'__actionId': 'select_mode'},
      },
      slots: <String, List<Map<String, Object?>>>{
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Option A'}),
          _node('Text', props: <String, Object?>{'text': 'Option B'}),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _refreshRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'PullToRefreshBox',
      props: <String, Object?>{
        'isRefreshing': true,
        'contentAlignment': 'topCenter',
        'onRefresh': <String, Object?>{'__actionId': 'refresh'},
      },
      slots: <String, List<Map<String, Object?>>>{
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Refresh content'}),
        ],
        'indicator': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Refreshing now'}),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _canvasSizeRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Canvas',
      props: <String, Object?>{
        'width': 80,
        'height': 48,
        'onSizeChanged': <String, Object?>{'__actionId': 'canvas_size'},
        'commands': <Map<String, Object?>>[
          <String, Object?>{
            'type': 'drawLine',
            'x1': 0,
            'y1': 0,
            'x2': 80,
            'y2': 48,
            'color': '#f44336',
          },
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _provideTextStyleRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'ProvideTextStyle',
      props: <String, Object?>{
        'style': 'bodyMedium',
        'fontSize': 18,
        'fontWeight': 'bold',
      },
      slots: <String, List<Map<String, Object?>>>{
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Styled slot'}),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _timePickerDialogRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'TimePickerDialog',
      props: <String, Object?>{
        'containerColor': '#ffffff',
        'shape': 12,
        'onDismissRequest': <String, Object?>{
          '__actionId': 'dismiss_time_picker',
        },
      },
      slots: <String, List<Map<String, Object?>>>{
        'title': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Pick time'}),
        ],
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Clock content'}),
        ],
        'modeToggleButton': <Map<String, Object?>>[
          _node(
            'TextButton',
            props: <String, Object?>{
              'text': 'Keyboard',
              'onClick': <String, Object?>{'__actionId': 'mode_toggle'},
            },
          ),
        ],
        'dismissButton': <Map<String, Object?>>[
          _node(
            'TextButton',
            props: <String, Object?>{
              'text': 'Cancel',
              'onClick': <String, Object?>{'__actionId': 'cancel_time'},
            },
          ),
        ],
        'confirmButton': <Map<String, Object?>>[
          _node(
            'Button',
            props: <String, Object?>{
              'text': 'OK',
              'onClick': <String, Object?>{'__actionId': 'confirm_time'},
            },
          ),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _scaffoldRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Scaffold',
      props: <String, Object?>{
        'containerColor': '#ffffff',
        'contentColor': '#e91e63',
      },
      slots: <String, List<Map<String, Object?>>>{
        'topBar': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Top tools'}),
        ],
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Scaffold body'}),
        ],
        'bottomBar': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Bottom tools'}),
        ],
        'snackbarHost': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Snack zone'}),
        ],
        'floatingActionButton': <Map<String, Object?>>[
          _node(
            'ExtendedFloatingActionButton',
            props: <String, Object?>{
              'onClick': <String, Object?>{'__actionId': 'fab_add'},
            },
            slots: <String, List<Map<String, Object?>>>{
              'content': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Add'}),
              ],
            },
          ),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _cardSurfaceRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'Card',
          props: <String, Object?>{
            'containerColor': '#102030',
            'containerAlpha': 0.5,
            'contentColor': '#ffffff',
            'contentPadding': <String, Object?>{'all': 8},
            'shape': 10,
            'border': <String, Object?>{
              'width': 2,
              'color': '#ff0000',
              'alpha': 0.25,
            },
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Card content'}),
            ],
          },
        ),
        _node(
          'Surface',
          props: <String, Object?>{
            'color': '#f5f5f5',
            'alpha': 0.75,
            'contentColor': '#00695c',
            'shadowElevation': 7,
            'shape': 6,
            'contentPadding': <String, Object?>{'all': 4},
            'onClick': <String, Object?>{'__actionId': 'surface_click'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Surface content'},
              ),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _buttonStyleRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'Button',
          props: <String, Object?>{
            'containerColor': '#00695c',
            'contentColor': '#ffffff',
            'disabledContainerColor': '#78909c',
            'disabledContentColor': '#263238',
            'contentPadding': <String, Object?>{
              'horizontal': 18,
              'vertical': 7,
            },
            'onClick': <String, Object?>{'__actionId': 'slot_button'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Slot Button'}),
            ],
          },
        ),
        _node(
          'Button',
          props: <String, Object?>{
            'text': 'Disabled Button',
            'enabled': false,
            'containerColor': '#00695c',
            'contentColor': '#ffffff',
            'disabledContainerColor': '#78909c',
            'disabledContentColor': '#263238',
          },
        ),
        _node(
          'OutlinedButton',
          props: <String, Object?>{
            'text': 'Outline',
            'contentColor': '#e91e63',
            'shape': 9,
            'border': <String, Object?>{'width': 3, 'color': '#e91e63'},
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _fabVariantsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'SmallFloatingActionButton',
          props: <String, Object?>{
            'shape': 10,
            'containerColor': '#4caf50',
            'contentColor': '#ffffff',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Icon', props: <String, Object?>{'name': 'add'}),
            ],
          },
        ),
        _node(
          'FloatingActionButton',
          props: <String, Object?>{
            'containerColor': '#2196f3',
            'contentColor': '#ffffff',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Icon', props: <String, Object?>{'name': 'edit'}),
            ],
          },
        ),
        _node(
          'LargeFloatingActionButton',
          props: <String, Object?>{
            'shape': 14,
            'containerColor': '#ff9800',
            'contentColor': '#000000',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Icon', props: <String, Object?>{'name': 'star'}),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _iconToggleRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'FilledIconToggleButton',
          props: <String, Object?>{
            'checked': true,
            'shape': 18,
            'onCheckedChange': <String, Object?>{'__actionId': 'toggle_icon'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Unchecked toggle'},
              ),
            ],
            'selectedIcon': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Selected toggle'},
              ),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _plainIconButtonRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'IconButton',
          props: <String, Object?>{
            'icon': 'close',
            'width': 32,
            'height': 32,
            'onClick': <String, Object?>{'__actionId': 'close'},
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _navigationRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'NavigationBar',
          props: <String, Object?>{
            'containerColor': '#fafafa',
            'tonalElevation': 3,
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node(
                'NavigationBarItem',
                props: <String, Object?>{
                  'selected': true,
                  'onClick': <String, Object?>{'__actionId': 'bar_selected'},
                },
                slots: <String, List<Map<String, Object?>>>{
                  'icon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar plain icon'},
                    ),
                  ],
                  'selectedIcon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar selected icon'},
                    ),
                  ],
                  'label': <Map<String, Object?>>[
                    _node('Text', props: <String, Object?>{'text': 'Bar home'}),
                  ],
                  'badge': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar badge'},
                    ),
                  ],
                },
              ),
              _node(
                'NavigationBarItem',
                props: <String, Object?>{
                  'selected': false,
                  'onClick': <String, Object?>{'__actionId': 'bar_inactive'},
                },
                slots: <String, List<Map<String, Object?>>>{
                  'icon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar inactive icon'},
                    ),
                  ],
                  'label': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar inactive'},
                    ),
                  ],
                },
              ),
            ],
          },
        ),
        _node(
          'NavigationRail',
          slots: <String, List<Map<String, Object?>>>{
            'header': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Rail header'}),
            ],
            'content': <Map<String, Object?>>[
              _node(
                'NavigationRailItem',
                props: <String, Object?>{
                  'selected': true,
                  'alwaysShowLabel': true,
                  'onClick': <String, Object?>{'__actionId': 'rail_selected'},
                },
                slots: <String, List<Map<String, Object?>>>{
                  'icon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Rail plain icon'},
                    ),
                  ],
                  'selectedIcon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Rail selected icon'},
                    ),
                  ],
                  'label': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Rail item'},
                    ),
                  ],
                },
              ),
            ],
          },
        ),
        _node(
          'NavigationDrawerItem',
          props: <String, Object?>{
            'selected': true,
            'shape': 12,
            'onClick': <String, Object?>{'__actionId': 'drawer_item'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'icon': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Drawer icon'}),
            ],
            'label': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Drawer item'}),
            ],
            'badge': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Drawer badge'}),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _tabsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'PrimaryScrollableTabRow',
      props: <String, Object?>{
        'selectedTabIndex': 1,
        'containerColor': '#ffffff',
        'contentColor': '#111111',
        'edgePadding': 4,
      },
      slots: <String, List<Map<String, Object?>>>{
        'tabs': <Map<String, Object?>>[
          _node(
            'Tab',
            props: <String, Object?>{
              'selected': false,
              'unselectedContentColor': '#607d8b',
              'onClick': <String, Object?>{'__actionId': 'overview_tab'},
            },
            slots: <String, List<Map<String, Object?>>>{
              'content': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Overview tab'}),
              ],
            },
          ),
          _node(
            'Tab',
            props: <String, Object?>{
              'selected': true,
              'selectedContentColor': '#e91e63',
              'onClick': <String, Object?>{'__actionId': 'details_tab'},
            },
            slots: <String, List<Map<String, Object?>>>{
              'content': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Details tab'}),
              ],
            },
          ),
          _node(
            'LeadingIconTab',
            props: <String, Object?>{
              'selected': false,
              'enabled': false,
              'unselectedContentColor': '#9e9e9e',
              'onClick': <String, Object?>{'__actionId': 'leading_tab'},
            },
            slots: <String, List<Map<String, Object?>>>{
              'icon': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Leading icon'}),
              ],
              'text': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Leading text'}),
              ],
            },
          ),
        ],
        'indicator': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Tab indicator'}),
        ],
        'divider': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Tab divider'}),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _chipsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'AssistChip',
          props: <String, Object?>{
            'shape': 10,
            'containerColor': '#e3f2fd',
            'contentColor': '#0d47a1',
            'onClick': <String, Object?>{'__actionId': 'assist_chip'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'leadingIcon': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Assist leading'}),
            ],
            'label': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Assist label'}),
            ],
            'trailingIcon': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Assist trailing'},
              ),
            ],
          },
        ),
        _node(
          'FilterChip',
          props: <String, Object?>{
            'selected': true,
            'selectedContainerColor': '#fff59d',
            'onClick': <String, Object?>{'__actionId': 'filter_chip'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'leadingIcon': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Filter leading'}),
            ],
            'label': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Filter label'}),
            ],
          },
        ),
        _node(
          'InputChip',
          props: <String, Object?>{
            'selected': true,
            'onDismiss': <String, Object?>{'__actionId': 'dismiss_input_chip'},
            'onClick': <String, Object?>{'__actionId': 'input_chip'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'avatar': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Input avatar'}),
            ],
            'label': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Input label'}),
            ],
            'trailingIcon': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Input dismiss'}),
            ],
          },
        ),
        _node(
          'ElevatedSuggestionChip',
          props: <String, Object?>{
            'onClick': <String, Object?>{'__actionId': 'suggestion_chip'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'icon': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Suggestion icon'},
              ),
            ],
            'label': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Suggestion label'},
              ),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

Map<String, Object?> _node(
  String type, {
  Map<String, Object?> props = const <String, Object?>{},
  List<Map<String, Object?>> children = const <Map<String, Object?>>[],
  Map<String, List<Map<String, Object?>>> slots =
      const <String, List<Map<String, Object?>>>{},
}) {
  return <String, Object?>{
    'type': type,
    'props': props,
    'children': children,
    'slots': slots,
  };
}
