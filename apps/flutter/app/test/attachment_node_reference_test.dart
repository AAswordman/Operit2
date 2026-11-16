import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    show AttachmentInfo;
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleUserMessageComposable.dart'
    as bubble;
import 'package:operit2/ui/features/chat/components/style/cursor/UserMessageComposable.dart'
    as cursor;

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

  test('media pooling preserves the stable file location in both styles', () {
    const markup =
        '<link type="image" id="pooled-image"/>'
        '<attachment id="/operitapp/outputs/family/photo.png" filename="photo.png" type="image/png" '
        'path="/operitapp/outputs/family/photo.png" node_id="origin" size="3"/>';
    for (final attachments in [
      bubble.parseMessageContent(markup).imageAttachments,
      cursor.parseMessageContent(markup).imageAttachments,
    ]) {
      final image = attachments.single;
      expect(image.id, 'pooled-image');
      expect(image.path, '/operitapp/outputs/family/photo.png');
      expect(image.nodeId, 'origin');
    }
  });

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