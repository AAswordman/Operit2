import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/attachments/AttachmentViewerDialog.dart';
import 'package:operit2/ui/features/chat/components/attachments/ChatMessageImageAttachments.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatAttachmentImagePreview.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleUserMessageComposable.dart'
    as bubble;
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleSurface.dart';
import 'package:operit2/ui/features/chat/components/style/cursor/UserMessageComposable.dart'
    as cursor;
import 'package:operit2/ui/theme/OperitTheme.dart';

const _context = AttachmentData(
  id: 'third_party_context_1',
  filename: '插件上下文',
  type: 'text/plain',
  content: 'Complete context supplied by an arbitrary plugin.',
);
const _file = AttachmentData(
  id: '/runtime/data/chat_attachments/chat/file/DEM模拟试卷_100分.docx',
  filename: 'DEM模拟试卷_100分.docx',
  type: 'application/msword',
  size: 41779,
);

void main() {
  const reviewOutput = String.fromEnvironment('ATTACHMENT_REVIEW_OUTPUT');
  if (reviewOutput.isNotEmpty) {
    setUpAll(() async {
      final loader = FontLoader('AttachmentReviewFont');
      loader.addFont(
        File(
          '../packages/operit_ui/assets/fonts/noto-sans-sc.ttf',
        ).readAsBytes().then((bytes) => ByteData.sublistView(bytes)),
      );
      await loader.load();
      final icons = FontLoader('MaterialIcons');
      icons.addFont(rootBundle.load('fonts/MaterialIcons-Regular.otf'));
      await icons.load();
    });
  }
  test(
    'inline contexts are classified by their payload and source, across owners',
    () {
      for (final id in [
        'message_insert_extra_bundle_1',
        'moodlet_force_directive',
        'notifications_1',
        'location_1',
        'pasted_text_1',
        'screen_ocr_1',
        'package_attach:custom/skill',
        'workspace_mention:src/main.rs',
        'an_unknown_plugin_context',
      ]) {
        expect(
          AttachmentData(
            id: id,
            filename: 'context.txt',
            type: 'text/plain',
            content: 'content',
          ).isInlineContext,
          isTrue,
          reason: id,
        );
      }
      for (final type in [
        'application/json',
        'text/json',
        'application/xml',
        'application/vnd.plugin+json',
        'Text/Plain; charset=utf-8',
      ]) {
        expect(
          AttachmentData(
            id: 'plugin_context',
            filename: 'Context',
            type: type,
            content: '{}',
          ).isInlineContext,
          isTrue,
          reason: type,
        );
      }
    },
  );

  test(
    'real text files, binary files and media never become supplementary metadata',
    () {
      for (final id in [
        '/tmp/report.txt',
        r'C:\files\report.txt',
        'relative/report.txt',
        'file:///tmp/report.txt',
        'content://files/1',
        'https://example.com/report.txt',
        'report.txt',
      ]) {
        expect(
          AttachmentData(
            id: id,
            filename: 'report.txt',
            type: 'text/plain',
            content: 'extracted text',
          ).isInlineContext,
          isFalse,
          reason: id,
        );
      }
      expect(
        const AttachmentData(
          id: 'uploaded_1',
          filename: 'report.txt',
          type: 'text/plain',
        ).isInlineContext,
        isFalse,
      );
      expect(
        const AttachmentData(
          id: 'generated_1',
          filename: 'report.pdf',
          type: 'application/pdf',
          content: 'description',
        ).isInlineContext,
        isFalse,
      );
      expect(
        const AttachmentData(
          id: 'pool_1',
          filename: 'notes.txt',
          type: 'text/plain',
          content: 'text',
          mediaPoolType: 'file',
        ).isInlineContext,
        isFalse,
      );
      expect(
        const AttachmentData(
          id: 'pool_1',
          filename: 'Audio',
          type: 'audio/*',
          content: 'transcript',
          mediaPoolType: 'audio',
        ).isInlineContext,
        isFalse,
      );
    },
  );

  test('media pool types and fallback extensions open consistent viewers', () {
    for (final entry in {
      'photo.PNG': 'image',
      'recording.mp3': 'audio',
      'clip.mov': 'video',
    }.entries) {
      final attachment = AttachmentData(
        id: '/tmp/${entry.key}',
        filename: entry.key,
        type: 'application/octet-stream',
      );
      expect(attachment.previewMimeType, '${entry.value}/*');
      expect(attachment.isInlineContext, isFalse);
    }
    const audio = AttachmentData(
      id: 'pool',
      filename: 'clip.mp4',
      type: 'audio/mp4',
    );
    expect(audio.isAudio, isTrue);
    expect(audio.isVideo, isFalse);
  });

  test(
    'both message parsers retain inline bodies and preserve file distinctions',
    () {
      const markup =
          'hello\n<attachment id="third_party_context_1" filename="Context" type="text/plain">&lt;content&gt;</attachment>';
      for (final attachment in [
        bubble.parseMessageContent(markup).attachments.single,
        cursor.parseMessageContent(markup).attachments.single,
      ]) {
        expect(attachment.isInlineContext, isTrue);
        expect(attachment.content, '<content>');
      }
      const fileMarkup =
          '<attachment id="/tmp/report.txt" filename="report.txt" type="text/plain" size="100" content="inline text"/>';
      expect(
        bubble
            .parseMessageContent(fileMarkup)
            .attachments
            .single
            .isInlineContext,
        isFalse,
      );
      expect(
        cursor
            .parseMessageContent(fileMarkup)
            .attachments
            .single
            .isInlineContext,
        isFalse,
      );
    },
  );

  for (final isBubble in [false, true]) {
    testWidgets(
      'memory recall attachment folds into supplementary information, bubble=$isBubble',
      (tester) async {
        const content = '【相关记忆】\n内容: <context> & "quotes" </attachment>';
        final markup =
            'hello\n\n<attachment id="character_memory_recall" filename="相关记忆" type="text/plain" size="${content.length}">'
            '【相关记忆】\n内容: &lt;context&gt; &amp; &quot;quotes&quot; &lt;/attachment&gt;</attachment>';
        final bubbleParsed = bubble.parseMessageContent(markup);
        final cursorParsed = cursor.parseMessageContent(markup);
        expect(
          isBubble ? bubbleParsed.processedText : cursorParsed.processedText,
          'hello',
        );
        final attachments = isBubble
            ? bubbleParsed.attachments
            : cursorParsed.attachments;
        final attachment = attachments.single;
        expect(attachment.id, 'character_memory_recall');
        expect(attachment.filename, '相关记忆');
        expect(attachment.isInlineContext, isTrue);
        expect(attachment.content, content);
        await tester.pumpWidget(_app(attachments, isBubble: isBubble));
        await tester.pumpAndSettle();
        expect(find.text(content), findsNothing);
        expect(find.text('相关记忆'), findsNothing);
        await tester.tap(find.text('补充信息'));
        await tester.pumpAndSettle();
        expect(find.text(content), findsOneWidget);
        expect(tester.takeException(), isNull);
      },
    );
    testWidgets(
      'context is quiet message metadata with full text and copy, bubble=$isBubble',
      (tester) async {
        String? copied;
        tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          (call) async {
            if (call.method == 'Clipboard.setData') {
              copied = (call.arguments as Map)['text'] as String;
            }
            return null;
          },
        );
        addTearDown(
          () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
            SystemChannels.platform,
            null,
          ),
        );
        await tester.pumpWidget(_app([_context], isBubble: isBubble));
        await tester.pumpAndSettle();
        expect(
          tester.getSize(find.byType(ChatMessageContextMetadata)).height,
          lessThanOrEqualTo(18),
        );
        expect(find.text(_context.content), findsNothing);
        await tester.tap(find.text('补充信息'));
        await tester.pumpAndSettle();
        expect(find.byType(SelectableText), findsOneWidget);
        expect(find.text(_context.content), findsOneWidget);
        await tester.tap(find.byIcon(Icons.copy_outlined));
        await tester.pump();
        expect(copied, _context.content);
        await tester.tap(find.byIcon(Icons.close));
        await tester.pumpAndSettle();
        expect(find.byType(SelectableText), findsNothing);
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets(
      'files retain readable names beside folded contexts, bubble=$isBubble',
      (tester) async {
        final contexts = List.generate(
          5,
          (index) => AttachmentData(
            id: 'plugin_$index',
            filename: 'Context $index',
            type: 'text/plain',
            content: 'text $index',
          ),
        );
        await tester.pumpWidget(
          _app([_file, ...contexts], isBubble: isBubble, bubbleWidth: 44),
        );
        await tester.pumpAndSettle();
        expect(find.text(_file.filename), findsOneWidget);
        expect(find.text('DOCX · 40.8 KB'), findsNothing);
        final fileCard = find.byWidgetPredicate(
          (widget) => widget is Tooltip && widget.message == _file.filename,
        );
        expect(tester.getSize(fileCard), const Size(72, 72));
        expect(find.text('Context 3'), findsNothing);
        await tester.tap(find.text('补充信息 · 5'));
        await tester.pumpAndSettle();
        expect(find.text('Context 4'), findsOneWidget);
        expect(find.text(_file.filename), findsOneWidget);
        await tester.tap(find.byIcon(Icons.close));
        await tester.pumpAndSettle();
        expect(find.text('Context 4'), findsNothing);
        expect(tester.takeException(), isNull);
      },
    );
  }

  for (final count in [2, 3]) {
    final flatWidth = count == 2 ? 152.0 : 232.0;
    testWidgets('attachment deck uses its $count-card boundary', (tester) async {
      final files = _layoutFiles(count);
      await tester.pumpWidget(
        _app(files, isBubble: true, width: 280, bubbleWidth: flatWidth),
      );
      await tester.pumpAndSettle();
      _expectFlatLayout(tester, count);

      await tester.pumpWidget(
        _app(files, isBubble: true, width: 280, bubbleWidth: flatWidth - 1),
      );
      await tester.pumpAndSettle();
      expect(find.text('$count'), findsOneWidget);
      expect(
        tester.getSize(find.byType(ChatMessageAttachments)),
        const Size(88, 80),
      );
      await tester.tap(find.text('$count'));
      await tester.pumpAndSettle();
      expect(find.text('收起 ($count)'), findsOneWidget);
      for (final file in files) {
        expect(_layoutCard(file.filename), findsOneWidget);
      }
      await tester.tap(find.text('收起 ($count)'));
      await tester.pumpAndSettle();
      expect(find.text('$count'), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('density changes remeasure the real user bubble and deck', (
    tester,
  ) async {
    // Keep this sample around the 152dp boundary and measure the painted bubble;
    // SelectableText can retain caret room in addition to the glyph advances.
    await tester.pumpWidget(
      _bubbleAttachmentApp(text: 'abcdefghijklm', count: 2),
    );
    await tester.pumpAndSettle();
    final messageState = tester.state(
      find.byType(bubble.BubbleUserMessageComposable),
    );
    final surface = find.byType(BubbleSurface);
    final compactWidth = tester.getSize(surface).width;
    expect(compactWidth, inInclusiveRange(144, 151.99));
    expect(_measuredBubbleWidth(tester), closeTo(compactWidth, 0.01));
    expect(find.text('2'), findsOneWidget);

    _updateBubbleAppearance(tester, padding: 12);
    await tester.pumpAndSettle();
    expect(
      tester.state(find.byType(bubble.BubbleUserMessageComposable)),
      same(messageState),
    );
    final comfortableWidth = tester.getSize(surface).width;
    expect(comfortableWidth, closeTo(compactWidth + 8, 0.01));
    expect(_measuredBubbleWidth(tester), closeTo(comfortableWidth, 0.01));
    _expectFlatLayout(tester, 2);
    await tester.tap(_layoutCard('layout-0.docx'));
    await tester.pumpAndSettle();
    expect(find.byType(AttachmentViewerDialog), findsOneWidget);
    await tester.tap(
      find.descendant(
        of: find.byType(AttachmentViewerDialog),
        matching: find.byIcon(Icons.close_rounded),
      ),
    );
    await tester.pumpAndSettle();

    _updateBubbleAppearance(tester, padding: 8);
    await tester.pumpAndSettle();
    expect(tester.getSize(surface).width, closeTo(compactWidth, 0.01));
    expect(_measuredBubbleWidth(tester), closeTo(compactWidth, 0.01));
    expect(find.text('2'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('avatar and wide changes update the real three-file deck', (
    tester,
  ) async {
    await tester.pumpWidget(
      _bubbleAttachmentApp(
        text: List<String>.filled(60, 'W').join(),
        count: 3,
        avatar: true,
      ),
    );
    await tester.pumpAndSettle();
    expect(_measuredBubbleWidth(tester), closeTo(210.8, 0.01));
    expect(find.text('3'), findsOneWidget);

    _updateBubbleAppearance(tester, avatar: false);
    await tester.pumpAndSettle();
    expect(_measuredBubbleWidth(tester), closeTo(244.8, 0.01));
    _expectFlatLayout(tester, 3);

    _updateBubbleAppearance(tester, avatar: true, wide: true, padding: 12);
    await tester.pumpAndSettle();
    expect(_measuredBubbleWidth(tester), closeTo(288, 0.01));
    _expectFlatLayout(tester, 3);

    _updateBubbleAppearance(tester, wide: false, padding: 8);
    await tester.pumpAndSettle();
    expect(_measuredBubbleWidth(tester), closeTo(210.8, 0.01));
    expect(find.text('3'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  for (final count in [2, 3]) {
    for (final wide in [false, true]) {
      testWidgets('attachment-only user row keeps its deck, $count/wide=$wide', (
        tester,
      ) async {
        await tester.pumpWidget(
          _bubbleAttachmentApp(text: '', count: count, wide: wide),
        );
        await tester.pumpAndSettle();
        expect(find.byType(BubbleSurface), findsNothing);
        expect(find.text('$count'), findsOneWidget);
        expect(
          tester.getSize(find.byType(ChatMessageAttachments)),
          const Size(88, 80),
        );
        await tester.tap(find.text('$count'));
        await tester.pumpAndSettle();
        expect(find.text('收起 ($count)'), findsOneWidget);
        for (final file in _layoutFiles(count)) {
          expect(_layoutCard(file.filename), findsOneWidget);
        }
        expect(tester.takeException(), isNull);
      });
    }
  }

  testWidgets(
    'original file stack excludes inline contexts and expands normally',
    (tester) async {
      final images = List.generate(
        5,
        (index) => AttachmentData(
          id: '/missing-operit-preview/$index.png',
          filename: '$index.png',
          type: 'image/png',
        ),
      );
      await tester.pumpWidget(
        _app([...images, _context], isBubble: true, bubbleWidth: 80),
      );
      await tester.pump();
      expect(find.byType(ChatAttachmentImagePreview), findsNWidgets(3));
      expect(find.text('补充信息'), findsOneWidget);
      await tester.tap(find.text('5'));
      await tester.pump();
      await tester.pump();
      expect(find.byType(ChatAttachmentImagePreview), findsNWidgets(5));
      expect(find.text('补充信息'), findsOneWidget);
      await tester.tap(find.text('收起 (5)'));
      await tester.pump();
      expect(find.byType(ChatAttachmentImagePreview), findsNWidgets(3));
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets(
    'original file cards fit narrow windows while contexts support larger text',
    (tester) async {
      await tester.pumpWidget(_app([_file, _context], width: 180));
      await tester.pumpAndSettle();
      final card = find.byWidgetPredicate(
        (widget) => widget is Tooltip && widget.message == _file.filename,
      );
      expect(tester.getSize(card), const Size(72, 72));
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(_app([_context], width: 180, scale: 2));
      await tester.pumpAndSettle();
      expect(
        tester.getSize(find.byType(ChatMessageContextMetadata)).height,
        lessThanOrEqualTo(36),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('many injected contexts stay a single quiet metadata entry', (
    tester,
  ) async {
    final contexts = List.generate(
      30,
      (index) => AttachmentData(
        id: 'plugin_$index',
        filename: 'Context $index',
        type: 'text/plain',
        content: 'Body $index',
      ),
    );
    await tester.pumpWidget(_app(contexts));
    await tester.pumpAndSettle();
    expect(find.text('补充信息 · 30'), findsOneWidget);
    expect(tester.getSize(find.byType(ChatMessageAttachments)), Size.zero);
    expect(
      tester.getSize(find.byType(ChatMessageContextMetadata)).height,
      lessThanOrEqualTo(18),
    );
    expect(find.byIcon(Icons.description_outlined), findsNothing);
    expect(find.text('Context 0'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  for (final brightness in Brightness.values) {
    testWidgets(
      'file overlay keeps all four corners uniformly dimmed, $brightness',
      (tester) async {
        final key = GlobalKey();
        var dismissed = 0;
        await tester.pumpWidget(
          RepaintBoundary(
            key: key,
            child: MaterialApp(
              theme: _theme(brightness),
              home: Scaffold(
                backgroundColor: const Color(0xffc8d0dc),
                body: Builder(
                  builder: (context) => Center(
                    child: TextButton(
                      onPressed: () => showDialog<void>(
                        context: context,
                        barrierColor: Colors.transparent,
                        builder: (dialogContext) => AttachmentViewerDialog(
                          visible: true,
                          attachment: const ChatAttachment(
                            id: '/tmp/report.docx',
                            filename: 'report.docx',
                            mimeType: 'application/msword',
                          ),
                          onDismiss: () {
                            dismissed++;
                            Navigator.of(dialogContext).pop();
                          },
                        ),
                      ),
                      child: const Text('Open'),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.tap(find.text('Open'));
        await tester.pumpAndSettle();
        await tester.runAsync(() async {
          final image =
              await (key.currentContext!.findRenderObject()
                      as RenderRepaintBoundary)
                  .toImage();
          final data = (await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          List<int> pixel(int x, int y) => List.generate(
            4,
            (channel) => data.getUint8((y * image.width + x) * 4 + channel),
          );
          final expected = pixel(image.width ~/ 2, 4);
          expect(expected[0], inInclusiveRange(122, 126));
          for (final position in [
            Offset(4, 4),
            Offset(image.width - 5, 4),
            Offset(4, image.height - 5),
            Offset(image.width - 5, image.height - 5),
          ]) {
            expect(pixel(position.dx.toInt(), position.dy.toInt()), expected);
          }
          image.dispose();
        });
        await tester.tapAt(const Offset(5, 5));
        await tester.pumpAndSettle();
        expect(dismissed, 1);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('review render of mixed attachments', (tester) async {
    const output = String.fromEnvironment('ATTACHMENT_REVIEW_OUTPUT');
    final key = GlobalKey();
    await tester.pumpWidget(
      RepaintBoundary(
        key: key,
        child: _app([
          _file,
          const AttachmentData(
            id: '/tmp/recording.mp3',
            filename: '会议录音.mp3',
            type: 'audio/mpeg',
            size: 204800,
          ),
          const AttachmentData(
            id: 'message_insert_extra_bundle_1',
            filename: 'Time:2026-10-30 22:38',
            type: 'text/plain',
            content: '[Time]\n2026-10-30\n[Battery]\n82%',
          ),
          const AttachmentData(
            id: 'location_1',
            filename: 'location.json',
            type: 'application/json',
            content: '{"latitude":31}',
          ),
          const AttachmentData(
            id: 'workspace_mention:src/main.rs',
            filename: 'src/main.rs',
            type: 'text/plain',
            content: 'fn main() {}',
          ),
          _context,
        ], width: 520),
      ),
    );
    await tester.pumpAndSettle();
    if (output.isNotEmpty) {
      await tester.runAsync(() async {
        final image =
            await (key.currentContext!.findRenderObject()
                    as RenderRepaintBoundary)
                .toImage();
        final data = await image.toByteData(format: ui.ImageByteFormat.png);
        await File(output).writeAsBytes(data!.buffer.asUint8List());
        image.dispose();
      });
    }
    expect(tester.takeException(), isNull);
  });
}

ThemeData _theme([Brightness brightness = Brightness.light]) => ThemeData(
  fontFamily:
      const String.fromEnvironment('ATTACHMENT_REVIEW_OUTPUT').isNotEmpty
      ? 'AttachmentReviewFont'
      : null,
  colorScheme: ColorScheme.fromSeed(
    seedColor: Colors.blue,
    brightness: brightness,
  ),
  dialogTheme: DialogThemeData(
    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28)),
    clipBehavior: Clip.antiAlias,
  ),
);

List<AttachmentData> _layoutFiles(int count) => [
  for (var index = 0; index < count; index++)
    AttachmentData(
      id: '/tmp/layout-$index.docx',
      filename: 'layout-$index.docx',
      type: 'application/msword',
      size: 100,
    ),
];

Finder _layoutCard(String filename) => find.byWidgetPredicate(
  (widget) => widget is Tooltip && widget.message == filename,
);

void _expectFlatLayout(WidgetTester tester, int count) {
  expect(find.text('$count'), findsNothing);
  final cards = [
    for (final file in _layoutFiles(count))
      tester.getRect(_layoutCard(file.filename)),
  ];
  for (var index = 0; index < cards.length; index++) {
    expect(cards[index].size, const Size(72, 72));
    if (index > 0) {
      expect(cards[index].left - cards[index - 1].right, closeTo(8, 0.01));
      expect(cards[index].top, closeTo(cards[0].top, 0.01));
    }
  }
  expect(
    cards.last.right - cards.first.left,
    closeTo(count == 2 ? 152 : 232, 0.01),
  );
}

double _measuredBubbleWidth(WidgetTester tester) => tester
    .widget<ChatMessageAttachments>(find.byType(ChatMessageAttachments))
    .bubbleWidth!;

void _updateBubbleAppearance(
  WidgetTester tester, {
  double? padding,
  bool? avatar,
  bool? wide,
}) {
  final controller = OperitTheme.of(
    tester.element(find.byType(bubble.BubbleUserMessageComposable)),
  );
  controller.setInitialThemePreferenceSnapshot(
    ThemePreferenceSnapshot.fromJson({
      ...controller.themePreferenceSnapshot.toJson(),
      if (padding != null) ...{
        'bubbleUserContentPaddingLeft': padding,
        'bubbleUserContentPaddingRight': padding,
      },
      if (avatar != null) 'bubbleShowAvatar': avatar,
      if (wide != null) 'bubbleWideLayoutEnabled': wide,
    }),
  );
  controller.previewThemeSettings();
}

Widget _bubbleAttachmentApp({
  required String text,
  required int count,
  bool avatar = false,
  bool wide = false,
}) {
  final content = [
    text,
    for (final file in _layoutFiles(count))
      '<attachment id="${file.id}" filename="${file.filename}" '
          'type="${file.type}" size="${file.size}"/>',
  ].join('\n');
  final message = core_proxy.ChatMessage(
    pluginExtensions: const {},
    sender: 'user',
    parts: [
      core_proxy.MessagePart(
        partId: 'attachment-text',
        sequence: 0,
        kind: core_proxy.MessagePartKind.markdown,
        content: content,
        toolCallId: null,
        toolName: null,
        attributes: const {},
      ),
    ],
    timestamp: 1,
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
    displayMode: core_proxy.ChatMessageDisplayMode.normal,
    isFavorite: false,
    contentStream: null,
  );
  return OperitTheme(
    initialThemePreferenceSnapshot: ThemePreferenceSnapshot.fromJson({
      ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      'userChatStyle': 'bubble',
      'aiChatStyle': 'bubble',
      'bubbleShowAvatar': avatar,
      'bubbleWideLayoutEnabled': wide,
      'bubbleUserContentPaddingLeft': 8.0,
      'bubbleUserContentPaddingRight': 8.0,
      'showUserName': false,
    }),
    initialThemeIsReady: false,
    unconfiguredChildEnabled: true,
    hostInteractionHostsEnabled: false,
    child: MaterialApp(
      locale: const Locale('zh'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Builder(
        builder: (context) => Theme(
          data: Theme.of(context).copyWith(
            textTheme: Theme.of(context).textTheme.copyWith(
              bodyMedium: const TextStyle(
                fontFamily: 'Ahem',
                fontSize: 10,
                height: 1,
              ),
            ),
          ),
          child: Scaffold(
            body: SingleChildScrollView(
              child: Center(
                child: SizedBox(
                  width: 320,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 16),
                    child: bubble.BubbleUserMessageComposable(
                      key: const ValueKey('attachment-message'),
                      message: message,
                      backgroundColor: Colors.blue.shade50,
                      textColor: Colors.black,
                      bubbleContentPaddingLeft: OperitTheme.of(
                        context,
                      ).themePreferenceSnapshot.bubbleUserContentPaddingLeft,
                      bubbleContentPaddingRight: OperitTheme.of(
                        context,
                      ).themePreferenceSnapshot.bubbleUserContentPaddingRight,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

Widget _app(
  List<AttachmentData> attachments, {
  bool isBubble = false,
  double width = 520,
  double scale = 1,
  double? bubbleWidth,
}) => MaterialApp(
  debugShowCheckedModeBanner: false,
  theme: _theme(),
  home: Scaffold(
    body: MediaQuery(
      data: MediaQueryData(textScaler: TextScaler.linear(scale)),
      child: Center(
        child: SizedBox(
          width: width,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: isBubble
                ? CrossAxisAlignment.end
                : CrossAxisAlignment.start,
            children: <Widget>[
              ChatMessageAttachments(
                attachments: attachments,
                enableDialogs: true,
                isBubbleMode: isBubble,
                bubbleWidth: bubbleWidth,
              ),
              ChatMessageContextMetadata(
                attachments: attachments,
                enableDialogs: true,
              ),
            ],
          ),
        ),
      ),
    ),
  ),
);
