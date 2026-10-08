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
    'attachment references retain the actual source and portable temp path',
    () {
      const attachment = AttachmentInfo(
        filePath: '/device-b/runtime/temp/clean_on_exit/report.pdf',
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
        contains('path="/app/data/temp/clean_on_exit/report.pdf"'),
      );
      expect(
        reference,
        contains('id="/app/data/temp/clean_on_exit/report.pdf"'),
      );
      expect(reference, isNot(contains('/device-b/')));
      expect(reference, contains('filename="报价单&quot;.pdf"'));
    },
  );

  test('both message styles still parse location metadata and escaped paths', () {
    const attachment = AttachmentInfo(
      filePath: '/device-b/runtime/temp/clean_on_exit/report&copy.pdf',
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
    const toolPath = '/app/data/temp/clean_on_exit/report&copy.pdf';
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

    const paired =
        '<attachment id="/app/data/temp/clean_on_exit/file.pdf" filename="file.pdf" '
        'type="application/pdf" node_id="core-b" '
        'path="/app/data/temp/clean_on_exit/file.pdf" size="3">body</attachment>';
    expect(
      bubble.parseMessageContent(paired).trailingAttachments.single.content,
      'body',
    );
    expect(
      cursor.parseMessageContent(paired).trailingAttachments.single.content,
      'body',
    );
  });

  test('Windows temporary paths produce the same file-tool locator', () {
    const attachment = AttachmentInfo(
      filePath: r'C:\runtime\temp\clean_on_exit\report.pdf',
      nodeId: 'core-windows',
      fileName: 'report.pdf',
      mimeType: 'application/pdf',
      fileSize: 0,
      content: '',
    );
    expect(
      viewModel.createAttachmentReference(attachment),
      contains('path="/app/data/temp/clean_on_exit/report.pdf"'),
    );
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
    expect(
      viewModel.createAttachmentReference(attachment),
      isNot(contains('node_id=')),
    );
  });

  test(
    'MessagePack preserves a source node and accepts legacy maps without it',
    () {
      final payload = <String, Object?>{
        'filePath': '/device-b/runtime/temp/clean_on_exit/report.pdf',
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
