import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    show AttachmentInfo;
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/attachments/AttachmentViewerDialog.dart';
import 'package:operit2/ui/features/chat/components/attachments/ChatMessageImageAttachments.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatAttachmentImagePreview.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleUserMessageComposable.dart'
    as bubble;
import 'package:operit2/ui/features/chat/components/style/cursor/UserMessageComposable.dart'
    as cursor;

/// Verifies persistent image references independently of temporary media pools.
void main() {
  test('both message styles still parse location metadata and escaped paths', () {
    const toolPath =
        '/operitapp/outputs/family/.operit/attachments/id/report&copy.pdf';
    const markup =
        '<attachment id="/operitapp/outputs/family/.operit/attachments/id/report&amp;copy.pdf" '
        'filename="报价单&quot;.pdf" type="application/pdf" node_id="core-b" '
        'path="/operitapp/outputs/family/.operit/attachments/id/report&amp;copy.pdf" '
        'size="3" content="inline &lt;content&gt;"/>';
    final bubbleResult = bubble.parseMessageContent(markup);
    final cursorResult = cursor.parseMessageContent(markup);
    expect(bubbleResult.processedText, isEmpty);
    expect(cursorResult.processedText, isEmpty);
    expect(bubbleResult.trailingAttachments.single.id, toolPath);
    expect(cursorResult.trailingAttachments.single.id, toolPath);
    expect(bubbleResult.trailingAttachments.single.filename, '报价单".pdf');
    expect(cursorResult.trailingAttachments.single.filename, '报价单".pdf');
    expect(bubbleResult.trailingAttachments.single.content, 'inline <content>');
    expect(cursorResult.trailingAttachments.single.content, 'inline <content>');
    expect(bubbleResult.trailingAttachments.single.size, 3);
    expect(cursorResult.trailingAttachments.single.size, 3);
    for (final attachments in [
      bubbleResult.trailingAttachments,
      cursorResult.trailingAttachments,
    ]) {
      expect(attachments.single.path, toolPath);
      expect(attachments.single.nodeId, 'core-b');
    }

    const paired =
        '<attachment id="/operitapp/data/data/chat_attachments/chat-a/attachment-a/file.pdf" filename="file.pdf" '
        'type="application/pdf" node_id="core-b" '
        'path="/operitapp/data/data/chat_attachments/chat-a/attachment-a/file.pdf" size="3">body</attachment>';
    expect(
      bubble.parseMessageContent(paired).trailingAttachments.single.content,
      'body',
    );
    expect(
      cursor.parseMessageContent(paired).trailingAttachments.single.content,
      'body',
    );
  });

  test('sent images retain their file identity in both styles', () {
    const markup =
        '<link type="image" id="pooled-image"/>'
        '<attachment id="/operitapp/outputs/family/photo.png" filename="photo.png" type="image/png" '
        'path="/operitapp/outputs/family/photo.png" node_id="origin" size="3"/>';
    for (final attachments in [
      bubble.parseMessageContent(markup).imageAttachments,
      cursor.parseMessageContent(markup).imageAttachments,
    ]) {
      final image = attachments.single;
      expect(image.id, '/operitapp/outputs/family/photo.png');
      expect(image.mediaPoolType, isNull);
      expect(image.path, '/operitapp/outputs/family/photo.png');
      expect(image.nodeId, 'origin');
    }
  });

  test('historical images without path metadata retain their original file', () {
    const path = '/runtime/data/chat_attachments/chat-a/id/photo.png';
    const markup =
        '<link type="image" id="expired-image"/>'
        '<attachment id="$path" filename="photo.png" type="image/png" size="3"/>';
    for (final attachments in [
      bubble.parseMessageContent(markup).imageAttachments,
      cursor.parseMessageContent(markup).imageAttachments,
    ]) {
      final image = attachments.single;
      expect(image.id, path);
      expect(image.mediaPoolType, isNull);
      expect(image.path, isNull);
      expect(image.nodeId, isNull);
    }
  });

  test('images with only a media link retain their pool identity', () {
    const markup = '<link type="image" id="pool-only-image"/>';
    for (final attachments in [
      bubble.parseMessageContent(markup).imageAttachments,
      cursor.parseMessageContent(markup).imageAttachments,
    ]) {
      final image = attachments.single;
      expect(image.id, 'pool-only-image');
      expect(image.mediaPoolType, 'image');
      expect(image.path, isNull);
    }
  });

  for (final isBubble in [false, true]) {
    testWidgets(
      'sent image and lightbox load original after remount, bubble=$isBubble',
      (tester) async {
        final directory = Directory.systemTemp.createTempSync('sent-image-');
        addTearDown(() => directory.deleteSync(recursive: true));
        final file = File('${directory.path}/photo.png');
        final bytes = base64Decode(
          'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAANSURBVBhXY2Bg+P8fAAMCAf/Jsq3uAAAAAElFTkSuQmCC',
        );
        file.writeAsBytesSync(bytes);
        // The pool ID is deliberately unregistered, as it is after a restart.
        final markup =
            '<link type="image" id="expired-pool-image"/>'
            '<attachment id="old-file-identity" filename="photo.png" '
            'type="image/png" path="${file.uri}" size="${bytes.length}"/>';

        // Reparse persisted markup and reload file bytes with no Flutter cache.
        for (var mount = 0; mount < 2; mount++) {
          final attachments = isBubble
              ? bubble.parseMessageContent(markup).imageAttachments
              : cursor.parseMessageContent(markup).imageAttachments;
          await tester.pumpWidget(
            MaterialApp(
              localizationsDelegates: AppLocalizations.localizationsDelegates,
              supportedLocales: AppLocalizations.supportedLocales,
              home: Scaffold(
                body: ChatMessageAttachments(
                  attachments: attachments,
                  enableDialogs: true,
                  isBubbleMode: isBubble,
                ),
              ),
            ),
          );
          await _loadAttachmentImages(tester, 1);
          expect(find.byType(Image), findsOneWidget);
          expect(find.byIcon(Icons.broken_image_outlined), findsNothing);
          expect(
            tester
                .widget<ChatAttachmentImagePreview>(
                  find.byType(ChatAttachmentImagePreview),
                )
                .attachmentPath,
            file.uri.toString(),
          );
          await tester.tap(
            find.descendant(
              of: find.byType(ChatMessageAttachments),
              matching: find.byType(InkWell),
            ),
          );
          await _loadAttachmentImages(tester, 2);
          expect(find.byType(AttachmentViewerDialog), findsOneWidget);
          expect(find.byType(Image), findsNWidgets(2));
          expect(find.byIcon(Icons.broken_image_outlined), findsNothing);
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
          PaintingBinding.instance.imageCache.clear();
          PaintingBinding.instance.imageCache.clearLiveImages();
        }
      },
    );
  }

  test('legacy JSON has no invented source node', () {
    final attachment = AttachmentInfo.fromJson(<String, Object?>{
      'filePath': '/old/report.pdf',
      'fileName': 'report.pdf',
      'mimeType': 'application/pdf',
      'fileSize': 3,
      'content': '',
    });
    expect(attachment.nodeId, isNull);
  });

  test(
    'MessagePack preserves a source node and accepts legacy maps without it',
    () {
      final payload = <String, Object?>{
        'filePath':
            '/device-b/runtime/data/chat_attachments/chat-a/attachment-a/report.pdf',
        'nodeId': 'core-b',
        'fileName': 'report.pdf',
        'mimeType': 'application/pdf',
        'fileSize': 3,
        'content': '',
      };
      final attachment = decodeCoreLink<AttachmentInfo>(
        encodeCoreLink(payload),
        decode: AttachmentInfo.fromMessagePack,
      );
      expect(attachment.nodeId, 'core-b');
      payload.remove('nodeId');
      final legacy = decodeCoreLink<AttachmentInfo>(
        encodeCoreLink(payload),
        decode: AttachmentInfo.fromMessagePack,
      );
      expect(legacy.nodeId, isNull);
    },
  );

  test('inline text stays self-contained and escaped attributes decode', () {
    const markup =
        '<attachment id="pasted_text_1" filename="pasted_text.txt" '
        'type="text/plain" size="3" content="&quot;&lt;&amp;"/>';
    for (final attachments in [
      bubble.parseMessageContent(markup).trailingAttachments,
      cursor.parseMessageContent(markup).trailingAttachments,
    ]) {
      final inline = attachments.single;
      expect(inline.id, 'pasted_text_1');
      expect(inline.content, '"<&');
      expect(inline.path, isNull);
      expect(inline.nodeId, isNull);
    }
  });
}

/// Lets real file reads and image decoding complete outside the fake clock.
Future<void> _loadAttachmentImages(WidgetTester tester, int count) async {
  for (var attempt = 0; attempt < 100; attempt++) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pump();
    if (find.byType(Image).evaluate().length == count) break;
  }
  expect(find.byType(Image), findsNWidgets(count));
  for (final element in find.byType(Image).evaluate()) {
    final image = element.widget as Image;
    await tester.runAsync(() => precacheImage(image.image, element));
  }
  await tester.pumpAndSettle();
}
