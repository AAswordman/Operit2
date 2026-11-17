import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/markdown/EnhancedCodeBlock.dart';
import 'package:operit2/ui/common/markdown/MarkdownImageRenderer.dart';
import 'package:operit2/ui/features/chat/components/ChatArea.dart';
import 'package:operit2/ui/features/chat/components/part/CustomXmlRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/ToolCallResultMergeRender.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleAiMessageComposable.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleSurface.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleUserMessageComposable.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

import 'support/markdown_event_fixture.dart';
import 'support/semantic_markdown_fixture.dart';

const _codeSource =
    '**标题** [链接](https://example.com)\n\n'
    '```dart\nfinal value = 1;\n```';
const _tableSource = '| 名称 | 数量 |\n| --- | ---: |\n| 苹果 | **2** |';

// These numbers are the reviewed viewport matrix, independent of the helper
// whose implementation the tests exercise. Widths refer to the chat pane.
const _widths = <
  ({
    double viewport,
    double normalColumn,
    double normalAvatar,
    double normal,
    double wide,
  })
>[
  (
    viewport: 320,
    normalColumn: 288,
    normalAvatar: 210.8,
    normal: 244.8,
    wide: 288,
  ),
  (
    viewport: 360,
    normalColumn: 328,
    normalAvatar: 244.8,
    normal: 278.8,
    wide: 328,
  ),
  (
    viewport: 464,
    normalColumn: 432,
    normalAvatar: 333.2,
    normal: 367.2,
    wide: 432,
  ),
  (
    viewport: 934,
    normalColumn: 700,
    normalAvatar: 561,
    normal: 595,
    wide: 902,
  ),
  (
    viewport: 1574,
    normalColumn: 700,
    normalAvatar: 561,
    normal: 595,
    wide: 1040,
  ),
  (
    viewport: 513.7,
    normalColumn: 481.7,
    normalAvatar: 375.445,
    normal: 409.445,
    wide: 481.7,
  ),
];

void main() {
  setUp(ClientLogger.initialize);

  for (final row in _widths) {
    for (final wide in [false, true]) {
      for (final avatar in [false, true]) {
        testWidgets(
          'production chat pane ${row.viewport}, wide=$wide, avatar=$avatar',
          (tester) async {
            final expectedWidth = wide
                ? row.wide
                : avatar
                ? row.normalAvatar
                : row.normal;
            final scroll = ScrollController();
            final autoScroll = ValueNotifier(false);
            addTearDown(scroll.dispose);
            addTearDown(autoScroll.dispose);

            for (final user in [false, true]) {
              await _mount(
                tester,
                _snapshot(wide: wide, avatar: avatar),
                viewport: row.viewport,
                message: _message(
                  user: user,
                  content: user
                      ? List.filled(80, 'A long user message.').join(' ')
                      : _codeSource,
                ),
                scroll: scroll,
                autoScroll: autoScroll,
              );
              final root = find.byType(
                user
                    ? BubbleUserMessageComposable
                    : BubbleAiMessageComposable,
              );
              final surface = find.descendant(
                of: root,
                matching: find.byType(BubbleSurface),
              );
              expect(surface, findsOneWidget);
              expect(
                _maximumWidth(tester, surface),
                closeTo(expectedWidth, 0.01),
              );
              final outer = tester.getRect(surface);
              expect(outer.width, lessThanOrEqualTo(expectedWidth + 0.01));
              if (!user) {
                // The real Markdown code node requests the full bubble width.
                expect(outer.width, closeTo(expectedWidth, 0.01));
              }
              expect(outer.height, greaterThanOrEqualTo(44));
              _expectInsets(tester, surface, left: 8, right: 8);

              final pane = tester.getRect(find.byType(ChatArea));
              final columnWidth = wide ? row.wide : row.normalColumn;
              final columnLeft =
                  pane.left + 16 + (row.viewport - 32 - columnWidth) / 2;
              final sideSlot = avatar && !wide ? 40.0 : 0.0;
              expect(
                user ? outer.right : outer.left,
                closeTo(
                  user
                      ? columnLeft + columnWidth - sideSlot
                      : columnLeft + sideSlot,
                  0.01,
                ),
              );
              final avatarFinder = _avatar(root);
              expect(
                avatarFinder,
                avatar ? findsOneWidget : findsNothing,
              );
              if (avatar) {
                final avatarRect = tester.getRect(avatarFinder);
                expect(avatarRect.size, const Size(32, 32));
                expect(
                  wide
                      ? outer.top - avatarRect.bottom
                      : user
                      ? avatarRect.left - outer.right
                      : outer.left - avatarRect.right,
                  closeTo(wide ? 6 : 8, 0.01),
                );
              }
              expect(tester.takeException(), isNull);
              await tester.pumpWidget(const SizedBox.shrink());
            }
          },
        );
      }
    }
  }

  for (final sides in [(8.0, 8.0), (12.0, 12.0), (0.0, 32.0), (30.0, 2.0)]) {
    for (final user in [false, true]) {
      testWidgets('keeps explicit bubble insets $sides, user=$user', (
        tester,
      ) async {
        await _mount(
          tester,
          _snapshot(left: sides.$1, right: sides.$2),
          message: _message(user: user, content: user ? 'short' : '名称'),
        );
        _expectInsets(
          tester,
          find.byType(BubbleSurface),
          left: sides.$1,
          right: sides.$2,
        );
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      });
    }
  }

  testWidgets('a new default source supplies compact insets to both roles', (
    tester,
  ) async {
    final snapshot = ThemePreferenceSnapshot.fromJson({
      ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      'userChatStyle': 'bubble',
      'aiChatStyle': 'bubble',
      'bubbleShowAvatar': false,
    });
    for (final user in [false, true]) {
      await _mount(
        tester,
        snapshot,
        message: _message(user: user, content: user ? 'hi' : '名称'),
      );
      _expectInsets(tester, find.byType(BubbleSurface), left: 8, right: 8);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    }
  });

  testWidgets('short user content keeps measured insets separate from the cap', (
    tester,
  ) async {
    await _mount(
      tester,
      _snapshot(),
      message: _message(user: true, content: 'hi'),
    );
    final surface = find.byType(BubbleSurface);
    final compactWidth = tester.getSize(surface).width;
    expect(_maximumWidth(tester, surface), closeTo(244.8, 0.01));
    expect(compactWidth, lessThanOrEqualTo(244.8));
    _expectInsets(tester, surface, left: 8, right: 8);
    _previewInsets(tester, {
      'bubbleUserContentPaddingLeft': 12.0,
      'bubbleUserContentPaddingRight': 12.0,
    });
    await tester.pumpAndSettle();
    _expectInsets(tester, surface, left: 12, right: 12);
    final comfortableWidth = tester.getSize(surface).width;
    expect(comfortableWidth, lessThanOrEqualTo(244.8));
    expect(comfortableWidth - compactWidth, closeTo(8, 0.01));
    debugPrint(
      'Short user bubble: compact=$compactWidth, comfortable=$comfortableWidth, '
      'maximum=244.8',
    );
    expect(_maximumWidth(tester, surface), closeTo(244.8, 0.01));
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  for (final scale in [0.85, 1.0, 1.3]) {
    for (final systemScale in [1.0, 2.0]) {
      testWidgets('font=$scale, system=$systemScale keeps bubble padding', (
        tester,
      ) async {
        await _mount(
          tester,
          _snapshot(fontScale: scale),
          message: _message(content: semanticMarkdownQuoteSource),
          systemScale: systemScale,
        );
        final surface = find.byType(BubbleSurface);
        _expectInsets(tester, surface, left: 8, right: 8);
        expect(
          MediaQuery.textScalerOf(tester.element(surface)).scale(10),
          closeTo(10 * systemScale, 0.01),
        );
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      });
    }
  }

  for (final padding in [8.0, 12.0]) {
    testWidgets(
      'semantic lists and nested quotes add only their own insets $padding',
      (tester) async {
        await _mount(
          tester,
          _snapshot(left: padding, right: padding),
          message: _message(content: semanticMarkdownBlocksSource),
        );
        final surfaceLeft = tester.getTopLeft(find.byType(BubbleSurface)).dx;
        expect(
          tester.getTopLeft(_richText('粗体')).dx - surfaceLeft,
          closeTo(padding + 12, 0.01),
        );
        expect(
          tester.getTopLeft(_richText('缩进项')).dx - surfaceLeft,
          closeTo(padding + 28, 0.01),
        );
        await tester.pumpWidget(const SizedBox.shrink());

        await _mount(
          tester,
          _snapshot(left: padding, right: padding),
          message: _message(content: semanticMarkdownQuoteSource),
        );
        final quoteSurfaceLeft = tester.getTopLeft(find.byType(BubbleSurface)).dx;
        for (final (label, extra) in [
          ('结束', 0.0),
          ('引用', 6.0),
          ('列表', 18.0),
          ('嵌套引用', 12.0),
        ]) {
          expect(
            tester.getTopLeft(_richText(label)).dx - quoteSurfaceLeft,
            closeTo(padding + extra, 0.01),
            reason: label,
          );
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );

    testWidgets('table cells and code gutter keep their separate insets $padding', (
      tester,
    ) async {
      await _mount(
        tester,
        _snapshot(left: padding, right: padding),
        message: _message(content: _tableSource),
      );
      expect(
        tester.getTopLeft(_richText('名称')).dx -
            tester.getTopLeft(find.byType(BubbleSurface)).dx,
        closeTo(padding + 9, 0.01),
      );
      await tester.pumpWidget(const SizedBox.shrink());
      await _mount(
        tester,
        _snapshot(left: padding, right: padding),
        message: _message(content: _codeSource),
      );
      final code = find.descendant(
        of: find.byType(EnhancedCodeBlock),
        matching: _richText('final value = 1;'),
      );
      expect(
        tester.getTopLeft(code).dx -
            tester.getTopLeft(find.byType(BubbleSurface)).dx,
        closeTo(padding + 37, 0.01),
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  for (final source in [
    '<think>Thinking geometry</think>',
    '<tool name="read_file"><param name="path">file.dart</param></tool>',
    '<status>Geometry status</status>',
  ]) {
    testWidgets('activity block follows the bubble content frame: $source', (
      tester,
    ) async {
      await _mount(
        tester,
        _snapshot(wide: true),
        message: _message(content: source),
      );
      final tool = source.startsWith('<tool ');
      if (tool && find.byType(MergedToolCallResultRow).evaluate().isEmpty) {
        await tester.tap(find.byIcon(Icons.keyboard_arrow_right));
        // A tool without a result keeps its streaming cursor animated.
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 500));
      }
      final block = find.byType(
        tool ? MergedToolCallResultRow : CustomXmlRenderer,
      ).first;
      final before = tester.getTopLeft(block).dx;
      _previewInsets(tester, {
        'bubbleAiContentPaddingLeft': 12.0,
        'bubbleAiContentPaddingRight': 12.0,
      });
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      expect(tester.getTopLeft(block).dx, closeTo(before + 4, 0.01));
      _expectInsets(tester, find.byType(BubbleSurface), left: 12, right: 12);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  testWidgets('a standalone AI image bypasses the text bubble padding', (
    tester,
  ) async {
    const image =
        '![pixel](data:image/png;base64,'
        'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6Q'
        'AAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQ'
        'AAAANSURBVBhXY2Bg+P8fAAMCAf/Jsq3uAAAAAElFTkSuQmCC)';
    await _mount(
      tester,
      _snapshot(left: 30, right: 2),
      message: _message(content: image),
    );
    expect(find.byType(BubbleSurface), findsNothing);
    expect(find.byType(MarkdownImageRenderer), findsOneWidget);
    expect(
      tester.getSize(find.byType(MarkdownImageRenderer)).width,
      closeTo(244.8, 0.01),
    );
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

ThemePreferenceSnapshot _snapshot({
  bool wide = false,
  bool avatar = false,
  double left = 8,
  double right = 8,
  double fontScale = 1,
}) => ThemePreferenceSnapshot.fromJson({
  ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
  'userChatStyle': 'bubble',
  'aiChatStyle': 'bubble',
  'bubbleWideLayoutEnabled': wide,
  'bubbleShowAvatar': avatar,
  'bubbleUserContentPaddingLeft': left,
  'bubbleUserContentPaddingRight': right,
  'bubbleAiContentPaddingLeft': left,
  'bubbleAiContentPaddingRight': right,
  'fontScale': fontScale,
  'showRoleName': false,
  'showThinkingProcess': true,
  'showUserName': false,
  'showModelName': false,
  'showModelProvider': false,
  'showMessageTokenStats': false,
  'showMessageTimingStats': false,
  'showMessageTimestamp': false,
});

ChatUiMessage _message({
  bool user = false,
  required String content,
}) => ChatUiMessage(
  pluginExtensions: const {},
  sender: user ? 'user' : 'ai',
  parts: [
    MessagePart(
      partId: 'body',
      sequence: 0,
      kind: MessagePartKind.markdown,
      content: content,
      toolCallId: null,
      toolName: null,
      attributes: const {},
    ),
  ],
  timestamp: user ? 1 : 2,
  roleName: '',
  selectedVariantIndex: 0,
  variantCount: 1,
  provider: '',
  modelName: '',
  inputTokens: 0,
  outputTokens: 0,
  cachedInputTokens: 0,
  sentAt: 0,
  outputDurationMs: 0,
  waitDurationMs: 0,
  completedAt: 0,
  displayMode: ChatMessageDisplayMode.normal,
  isFavorite: false,
  contentStream: null,
);

Future<List<MarkdownStreamEvent>> _splitContent(String source) =>
    semanticMarkdownFixtures.any((fixture) => fixture['source'] == source)
    ? splitSemanticMarkdownFixture(source)
    : splitMarkdownEventFixture(source);

Future<void> _mount(
  WidgetTester tester,
  ThemePreferenceSnapshot snapshot, {
  required ChatUiMessage message,
  double viewport = 320,
  double systemScale = 1,
  ScrollController? scroll,
  ValueNotifier<bool>? autoScroll,
}) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = const Size(1800, 1800);
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
  final controller = scroll ?? ScrollController();
  final follow = autoScroll ?? ValueNotifier(false);
  if (scroll == null) addTearDown(controller.dispose);
  if (autoScroll == null) addTearDown(follow.dispose);
  final clients = GeneratedCoreProxyClients(_GeometryBridge());
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot: snapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Builder(
        builder: (context) => Scaffold(
          body: MediaQuery(
            data: MediaQuery.of(context).copyWith(
              textScaler: TextScaler.linear(systemScale),
            ),
            child: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: viewport,
                height: 1500,
                child: ChatArea(
                  messages: [message],
                  isLoading: false,
                  errorMessage: null,
                  scrollController: controller,
                  currentChatId: null,
                  identityAvatarUri: null,
                  clients: clients,
                  packageManager: clients.application.packageManager(),
                  autoScrollToBottomListenable: follow,
                  hasOlderDisplayHistory: false,
                  hasNewerDisplayHistory: false,
                  isLoadingDisplayWindow: false,
                  loadLocatorEntries: (_, _) async => [],
                  onRevealMessageForLocator: (_) async => false,
                  onAutoScrollToBottomChanged: (_) {},
                  onLoadOlderDisplayWindow: () async {},
                  onLoadNewerDisplayWindow: () async {},
                  onShowLatestDisplayWindow: () async {},
                  onToggleFavoriteMessage: (_, _) async {},
                  onDeleteMessage: (_) async {},
                  onDeleteMessagesFrom: (_) async => true,
                  onDeleteMessageVariant: (_, _) async {},
                  onSelectMessageVariant: (_, _) async {},
                  onRollbackToMessage: (_) {},
                  onSelectMessageToEdit: (_) {},
                  onRegenerateMessage: (_) async {},
                  onInsertSummary: (_) {},
                  onCreateBranch: (_) async {},
                  onReplyToMessage: (_) {},
                  onPlayVoice: (_) async {},
                  onToggleMultiSelectMode: (_) {},
                  onToggleMessageSelection: (_) {},
                  onRefreshRequested: () async {},
                  bottomContentInset: 0,
                  splitMarkdownContent: _splitContent,
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Finder _richText(String text) => find.byWidgetPredicate(
  (widget) => widget is RichText && widget.text.toPlainText().trim() == text,
);

void _previewInsets(WidgetTester tester, Map<String, Object?> updates) {
  final controller = OperitTheme.of(
    tester.element(find.byType(BubbleSurface)),
  );
  controller.setInitialThemePreferenceSnapshot(
    ThemePreferenceSnapshot.fromJson({
      ...controller.themePreferenceSnapshot.toJson(),
      ...updates,
    }),
  );
  controller.previewThemeSettings();
}

Finder _avatar(Finder root) => find.descendant(
  of: root,
  matching: find.byWidgetPredicate(
    (widget) =>
        widget is Container &&
        widget.constraints?.minWidth == 32 &&
        widget.constraints?.maxWidth == 32 &&
        widget.constraints?.minHeight == 32 &&
        widget.constraints?.maxHeight == 32,
  ),
);

double _maximumWidth(WidgetTester tester, Finder surface) {
  double? maximum;
  tester.element(surface).visitAncestorElements((element) {
    final widget = element.widget;
    if (widget is ConstrainedBox && widget.constraints.minHeight == 44) {
      maximum = widget.constraints.maxWidth;
      return false;
    }
    return true;
  });
  expect(maximum, isNotNull);
  return maximum!;
}

void _expectInsets(
  WidgetTester tester,
  Finder surface, {
  required double left,
  required double right,
}) {
  final padding = tester.widget<BubbleSurface>(surface).child as Padding;
  expect(padding.padding, EdgeInsets.fromLTRB(left, 12, right, 12));
  final outer = tester.getRect(find.byWidget(padding));
  final inner = tester.getRect(find.byWidget(padding.child!));
  expect(inner.left - outer.left, closeTo(left, 0.01));
  expect(outer.right - inner.right, closeTo(right, 0.01));
  expect(inner.top - outer.top, closeTo(12, 0.01));
  expect(outer.bottom - inner.bottom, closeTo(12, 0.01));
  final painted = tester.getRect(surface);
  expect(outer.left, closeTo(painted.left, 0.01));
  expect(outer.right, closeTo(painted.right, 0.01));
}

/// Provides only the unrelated catalog/revision boundaries needed by ChatArea.
class _GeometryBridge extends OperitRuntimeBridge {
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    switch (request.methodName) {
      case 'getToolPkgNavigationEntries':
      case 'getToolPkgUiRoutes':
        return encodeCoreLink([0, const <Object?>[]]);
      default:
        throw StateError('Unexpected call: ${request.methodName}');
    }
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName == 'messageTransformRevisionFlow') {
      return Stream.value(
        CoreEvent.raw(
          requestId: request.requestId,
          target: request.target,
          propertyName: request.propertyName,
          kind: 'Snapshot',
          valueBytes: encodeCoreLink(0),
          decodeValue: decodeCoreLink<Object?>,
        ),
      );
    }
    throw StateError('Unexpected watch: ${request.propertyName}');
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) => throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
}
