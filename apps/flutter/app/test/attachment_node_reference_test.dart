import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/ui/features/chat/components/style/bubble/BubbleUserMessageComposable.dart'
    as bubble;
import 'package:operit2/ui/features/chat/components/style/cursor/UserMessageComposable.dart'
    as cursor;
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

void main() {
  final viewModel = ChatViewModel();

  test(
    'attachment references retain the actual source and portable tool path',
    () {
      const attachment = AttachmentInfo(
        vfsPath: '/operitapp/outputs/family/.operit/attachments/id/report.pdf',
        filePath:
            '/device-b/runtime/data/chat_outputs/family/.operit/attachments/id/report.pdf',
        nodeId: 'core-b',
        fileName: '报价单".pdf',
        mimeType: 'application/pdf',
        fileSize: 3,
        content: '',
      );
      final reference = viewModel.createAttachmentReference(attachment);
      expect(reference, contains('node_id="core-b"'));
      expect(
        reference,
        contains(
          'path="/operitapp/outputs/family/.operit/attachments/id/report.pdf"',
        ),
      );
      expect(
        reference,
        contains(
          'id="/operitapp/outputs/family/.operit/attachments/id/report.pdf"',
        ),
      );
      expect(reference, isNot(contains('/device-b/')));
      expect(reference, contains('filename="报价单&quot;.pdf"'));
    },
  );

  test('both message styles still parse location metadata and escaped paths', () {
    const attachment = AttachmentInfo(
      vfsPath:
          '/operitapp/outputs/family/.operit/attachments/id/report&copy.pdf',
      filePath:
          '/device-b/runtime/data/chat_outputs/family/.operit/attachments/id/report&copy.pdf',
      nodeId: 'core-b',
      fileName: '报价单".pdf',
      mimeType: 'application/pdf',
      fileSize: 3,
      content: 'inline <content>',
    );
    final reference = viewModel.createAttachmentReference(attachment);
    final bubbleResult = bubble.parseMessageContent(reference);
    final cursorResult = cursor.parseMessageContent(reference);
    expect(bubbleResult.processedText, isEmpty);
    expect(cursorResult.processedText, isEmpty);
    const toolPath =
        '/operitapp/outputs/family/.operit/attachments/id/report&copy.pdf';
    expect(bubbleResult.trailingAttachments.single.id, toolPath);
    expect(cursorResult.trailingAttachments.single.id, toolPath);
    expect(
      bubbleResult.trailingAttachments.single.filename,
      attachment.fileName,
    );
    expect(
      cursorResult.trailingAttachments.single.filename,
      attachment.fileName,
    );
    expect(bubbleResult.trailingAttachments.single.content, attachment.content);
    expect(cursorResult.trailingAttachments.single.content, attachment.content);
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

  test(
    'Windows attachments use their recorded locator without interpreting the drive',
    () {
      const attachment = AttachmentInfo(
        vfsPath: '/operitapp/outputs/family/.operit/attachments/id/report.pdf',
        filePath:
            r'C:\runtime\data\chat_attachments\chat-a\attachment-a\report.pdf',
        nodeId: 'core-windows',
        fileName: 'report.pdf',
        mimeType: 'application/pdf',
        fileSize: 0,
        content: '',
      );
      expect(
        viewModel.createAttachmentReference(attachment),
        contains(
          'path="/operitapp/outputs/family/.operit/attachments/id/report.pdf"',
        ),
      );
    },
  );

  test('legacy JSON has no invented source node', () {
    final attachment = AttachmentInfo.fromJson(<String, Object?>{
      'filePath': '/old/report.pdf',
      'fileName': 'report.pdf',
      'mimeType': 'application/pdf',
      'fileSize': 3,
      'content': '',
    });
    expect(attachment.nodeId, isNull);
    expect(
      viewModel.createAttachmentReference(attachment),
      isNot(contains('node_id=')),
    );
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

  test('inline text remains self-contained and XML attributes are escaped', () {
    const attachment = AttachmentInfo(
      vfsPath: null,
      filePath: 'pasted_text_1',
      nodeId: null,
      fileName: 'pasted_text.txt',
      mimeType: 'text/plain',
      fileSize: 3,
      content: '"<&',
    );
    final reference = viewModel.createAttachmentReference(attachment);
    expect(reference, isNot(contains('node_id=')));
    expect(reference, contains('id="pasted_text_1"'));
    expect(reference, isNot(contains(' path=')));
    expect(reference, contains('content="&quot;&lt;&amp;"'));
  });
}
