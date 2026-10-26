import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/MarkdownImagePreview.dart';
import 'package:operit2/ui/common/markdown/MarkdownImageRenderer.dart';
import 'package:operit2/ui/common/markdown/MarkdownRemoteImage.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleAiMessageComposable.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleSurface.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleUserMessageComposable.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

import 'support/markdown_event_fixture.dart';

const _url =
    'data:image/png;base64,'
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAANSURBVBhXY2Bg+P8fAAMCAf/Jsq3uAAAAAElFTkSuQmCC';
const _markdown = '![图片说明]($_url)';

void main() {
  testWidgets('standalone image and its alt are centered within the block', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 400,
            child: MarkdownImageRenderer(
              imageMarkdown: _markdown,
              textColor: Colors.black,
            ),
          ),
        ),
      ),
    );
    await _settleImages(tester);
    final block = tester.getRect(find.byType(MarkdownImageRenderer));
    final image = tester.getRect(find.byType(Image));
    final caption = tester.getRect(find.text('图片说明'));
    expect(image.center.dx, closeTo(block.center.dx, 0.1));
    expect(caption.center.dx, closeTo(block.center.dx, 0.1));
    expect(caption.top, greaterThanOrEqualTo(image.bottom));
    expect(tester.widget<Text>(find.text('图片说明')).textAlign, TextAlign.center);
    expect(tester.takeException(), isNull);
  });

  testWidgets('image preview opens locally, zooms, pans freely, and closes', (
    tester,
  ) async {
    var linkOpened = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MarkdownImageRenderer(
            imageMarkdown: _markdown,
            textColor: Colors.black,
            onOpenLink: (_) => linkOpened = true,
          ),
        ),
      ),
    );
    await _settleImages(tester);
    await tester.tap(find.byType(Image));
    await tester.pumpAndSettle();
    expect(find.byType(MarkdownImagePreview), findsOneWidget);
    expect(linkOpened, isFalse);
    final viewer = tester.widget<InteractiveViewer>(
      find.byType(InteractiveViewer),
    );
    final controller = viewer.transformationController!;
    expect(controller.value.getMaxScaleOnAxis(), 1);

    // Drag at fit scale as well as when zoomed. The picture is unconstrained.
    await tester.drag(find.byType(InteractiveViewer), const Offset(90, 50));
    await tester.pumpAndSettle();
    expect(controller.value.getTranslation().x, greaterThan(50));
    await tester.tap(find.byIcon(Icons.zoom_in));
    await tester.pumpAndSettle();
    expect(controller.value.getMaxScaleOnAxis(), closeTo(1.5, 0.001));
    await tester.tap(find.byIcon(Icons.zoom_out));
    await tester.pumpAndSettle();
    expect(controller.value.getMaxScaleOnAxis(), closeTo(1, 0.001));
    await tester.tap(find.byIcon(Icons.fit_screen));
    await tester.pumpAndSettle();
    expect(controller.value, Matrix4.identity());

    // Pinch gesture must scale the image without toolbar involvement.
    final center = tester.getCenter(find.byType(InteractiveViewer));
    final first = await tester.startGesture(
      center - const Offset(50, 0),
      pointer: 1,
    );
    final second = await tester.startGesture(
      center + const Offset(50, 0),
      pointer: 2,
    );
    await first.moveTo(center - const Offset(100, 0));
    await second.moveTo(center + const Offset(100, 0));
    await tester.pump();
    expect(controller.value.getMaxScaleOnAxis(), greaterThan(1.4));
    await first.up();
    await second.up();
    await tester.pumpAndSettle();

    final fullImage = tester.widget<MarkdownRemoteImage>(
      find.descendant(
        of: find.byType(MarkdownImagePreview),
        matching: find.byType(MarkdownRemoteImage),
      ),
    );
    expect(fullImage.decodeAtFullResolution, isTrue);
    await tester.tap(find.byIcon(Icons.close));
    await tester.pumpAndSettle();
    expect(find.byType(MarkdownImagePreview), findsNothing);
    expect(tester.takeException(), isNull);
  });

  for (final source in [
    'https://example.com/photo.png',
    '/app/data/photo.png',
  ]) {
    testWidgets('preview retains the owning runtime for $source', (
      tester,
    ) async {
      final bridge = _ImageBridge();
      final clients = GeneratedCoreProxyClients(bridge);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: MarkdownRemoteImage(url: source, clients: clients),
          ),
        ),
      );
      await _settleImages(tester);
      await tester.tap(find.byType(Image));
      await _settleImages(tester);
      expect(find.byType(MarkdownImagePreview), findsOneWidget);
      final previewImage = tester.widget<MarkdownRemoteImage>(
        find.descendant(
          of: find.byType(MarkdownImagePreview),
          matching: find.byType(MarkdownRemoteImage),
        ),
      );
      expect(previewImage.clients, same(clients));
      expect(previewImage.url, source);
      expect(find.byIcon(Icons.broken_image_outlined), findsNothing);
      expect(bridge.sources, isNotEmpty);
      expect(bridge.sources.every((url) => url == source), isTrue);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('preview can be disabled for shared/static image rendering', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: MarkdownImageRenderer(
            imageMarkdown: _markdown,
            textColor: Colors.black,
            enablePreview: false,
          ),
        ),
      ),
    );
    await _settleImages(tester);
    await tester.tap(find.byType(Image));
    await tester.pumpAndSettle();
    expect(find.byType(MarkdownImagePreview), findsNothing);
  });

  for (final wide in [false, true]) {
    for (final sender in ['ai', 'user']) {
      testWidgets('$sender pure image bypasses bubble surface, wide=$wide', (
        tester,
      ) async {
        await _pumpBubble(tester, sender: sender, wide: wide);
        expect(find.byType(MarkdownImageRenderer), findsOneWidget);
        expect(find.byType(BubbleSurface), findsNothing);
        expect(find.text('图片说明'), findsOneWidget);
        await tester.tap(find.byType(Image));
        await tester.pumpAndSettle();
        expect(find.byType(MarkdownImagePreview), findsOneWidget);
        expect(tester.takeException(), isNull);
      });

      testWidgets('$sender text plus image retains bubble, wide=$wide', (
        tester,
      ) async {
        await _pumpBubble(
          tester,
          sender: sender,
          wide: wide,
          content: '图片在下面\n\n$_markdown',
        );
        expect(find.byType(BubbleSurface), findsOneWidget);
        expect(tester.takeException(), isNull);
      });
    }
  }

  test('only a single image qualifies for the bubble shortcut', () {
    expect(isStandaloneMarkdownImage(_markdown), isTrue);
    expect(isStandaloneMarkdownImage('$_markdown\n$_markdown'), isFalse);
    expect(isStandaloneMarkdownImage('说明 $_markdown'), isFalse);
    expect(
      isStandaloneMarkdownImage('![视频](https://example.com/clip.mp4)'),
      isFalse,
    );
    expect(
      isStandaloneMarkdownImage('![音频](https://example.com/song.mp3)'),
      isFalse,
    );
    expect(
      isStandaloneMarkdownImage('![未完成](https://example.com/a.png'),
      isFalse,
    );
  });
}

Future<void> _pumpBubble(
  WidgetTester tester, {
  required String sender,
  required bool wide,
  String content = _markdown,
}) async {
  final snapshot = ThemePreferenceSnapshot.fromJson({
    ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
    'bubbleWideLayoutEnabled': wide,
    'bubbleShowAvatar': false,
    'showRoleName': false,
  });
  final message = ChatMessage(
    pluginExtensions: const {},
    sender: sender,
    parts: [
      MessagePart(
        partId: 'image',
        sequence: 0,
        kind: MessagePartKind.markdown,
        content: content,
        toolCallId: null,
        toolName: null,
        attributes: const {},
      ),
    ],
    timestamp: 1,
    roleName: sender,
    selectedVariantIndex: 0,
    variantCount: 1,
    provider: 'test',
    modelName: 'test',
    inputTokens: 0,
    outputTokens: 0,
    cachedInputTokens: 0,
    sentAt: 0,
    outputDurationMs: 0,
    waitDurationMs: 0,
    completedAt: 1,
    displayMode: ChatMessageDisplayMode.normal,
    isFavorite: false,
    contentStream: null,
  );
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot: snapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: SingleChildScrollView(
            child: sender == 'ai'
                ? BubbleAiMessageComposable(
                    message: message,
                    backgroundColor: Colors.blue,
                    textColor: Colors.black,
                    splitMarkdownContent: splitMarkdownEventFixture,
                  )
                : BubbleUserMessageComposable(
                    message: message,
                    backgroundColor: Colors.blue,
                    textColor: Colors.black,
                  ),
          ),
        ),
      ),
    ),
  );
  await _settleImages(tester);
}

Future<void> _settleImages(WidgetTester tester) async {
  await tester.pumpAndSettle();
  for (final element in find.byType(Image).evaluate()) {
    final image = element.widget as Image;
    await tester.runAsync(() => precacheImage(image.image, element));
  }
  await tester.pumpAndSettle();
}

/// Serves actual PNG bytes over both runtime contracts used by Markdown images.
class _ImageBridge extends OperitRuntimeBridge {
  final bytes = UriData.parse(_url).contentAsBytes();
  final sources = <String>[];

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    expect(request.methodName, 'executeTool');
    final tool = (request.args as Map)['tool'] as Map;
    expect(tool['name'], 'read_file_binary');
    final path =
        ((tool['parameters'] as List).single as Map)['value'] as String;
    sources.add(path);
    return encodeCoreLink([
      0,
      {
        'toolName': 'read_file_binary',
        'success': true,
        'error': null,
        'result': {
          '__type': 'BinaryFileContentData',
          'path': path,
          'contentBase64': base64Encode(bytes),
          'size': bytes.length,
        },
      },
    ]);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    expect(request.propertyName, 'imageBytes');
    sources.add((request.args as Map)['url'] as String);
    yield CoreEvent.raw(
      requestId: request.requestId,
      target: request.target,
      propertyName: request.propertyName,
      kind: 'Changed',
      valueBytes: encodeCoreLink({
        'bytes': bytes,
        'displayUrl': null,
        'error': null,
      }),
      decodeValue: (data) => decodeCoreLink<Object?>(data),
    );
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
}
