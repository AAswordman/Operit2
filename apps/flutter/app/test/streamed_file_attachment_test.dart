import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/host/SelectedFileInput.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

class UploadBridge extends OperitRuntimeBridge {
  final methods = <String>[];
  final requests = <CoreCallRequest>[];
  final chunkLengths = <int>[];
  String? failure;
  Completer<void>? nextChunk;
  int received = 0;
  bool closed = false;

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    methods.add(request.methodName);
    requests.add(request);
    if (failure == request.methodName) throw StateError('failure: $failure');
    Object? result;
    switch (request.methodName) {
      case 'beginAttachmentUpload':
        result = 'opaque-upload';
      case 'completeAttachmentUpload':
        result = <String, Object?>{
          'filePath':
              '/receiver/runtime/temp/clean_on_exit/chat_attachment_uploads/upload-id/文档.apk',
          'nodeId': 'core-receiver',
          'fileName': '文档.apk',
          'mimeType': 'application/octet-stream',
          'fileSize': received,
          'content': '',
        };
      case 'attachUploadedFile':
      case 'discardAttachmentUpload':
        break;
      default:
        throw StateError('Unexpected ${request.methodName}');
    }
    return encodeCoreLink([0, result]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    expect(request.methodName, 'writeAttachmentUpload');
    expect(request.target, 'core/services.attachmentTransferManager');
    methods.add(request.methodName);
    return UploadSink(this);
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class UploadSink implements CorePushSink {
  UploadSink(this.bridge);
  final UploadBridge bridge;
  @override
  Future<void> add(Object? args) async {
    final bytes = args! as Uint8List;
    expect(bytes.length, lessThanOrEqualTo(selectedFileChunkBytes));
    bridge.chunkLengths.add(bytes.length);
    bridge.received += bytes.length;
    if (bridge.failure == 'writeAttachmentUpload') {
      throw StateError('write failed');
    }
    await bridge.nextChunk?.future;
  }

  @override
  Future<void> close() async {
    bridge.closed = true;
  }
}

void main() {
  test('a missing chat rejects input before allocating an upload', () async {
    final bridge = UploadBridge();
    final file = SelectedFileInput.fromStream(
      name: 'report.pdf',
      byteLength: 3,
      stream: Stream.value(Uint8List(3)),
    );
    await expectLater(
      ChatViewModel(bridge: bridge).attachSelectedFile(
        file,
        expectedChatId: null,
      ),
      throwsStateError,
    );
    expect(bridge.methods, isEmpty);
    await expectLater(file.readChunk(), throwsStateError);
  });

  test('143 MB file is streamed without complete-content arguments', () async {
    const length = 143388128;
    final bridge = UploadBridge();
    var generated = 0;
    Stream<Uint8List> source() async* {
      while (generated < length) {
        final count = (length - generated).clamp(0, selectedFileChunkBytes);
        generated += count;
        yield Uint8List(count);
      }
    }

    final file = SelectedFileInput.fromStream(
      name: '文档.apk',
      byteLength: length,
      stream: source(),
    );
    await ChatViewModel(
      bridge: bridge,
    ).attachSelectedFile(file, expectedChatId: 'chat-a');
    expect(bridge.received, length);
    expect(bridge.closed, isTrue);
    expect(bridge.methods, [
      'beginAttachmentUpload',
      'writeAttachmentUpload',
      'completeAttachmentUpload',
      'attachUploadedFile',
    ]);
    for (final request in bridge.requests) {
      expect(request.args.toString(), isNot(contains('base64Content')));
      expect(request.args.toString().length, lessThan(1024));
    }
    final attachment = (bridge.requests.last.args! as Map)['attachment'] as Map;
    expect(attachment['nodeId'], 'core-receiver');
    expect(attachment['fileName'], '文档.apk');
    await expectLater(file.readChunk(), throwsStateError);
  });

  test('awaiting sink writes prevents read-ahead', () async {
    final bridge = UploadBridge()..nextChunk = Completer<void>();
    var produced = 0;
    Stream<Uint8List> source() async* {
      for (var i = 0; i < 3; i++) {
        produced++;
        yield Uint8List(selectedFileChunkBytes);
      }
    }

    final file = SelectedFileInput.fromStream(
      name: '文档.apk',
      byteLength: 3 * selectedFileChunkBytes,
      stream: source(),
    );
    final uploading = ChatViewModel(
      bridge: bridge,
    ).attachSelectedFile(file, expectedChatId: 'chat-a');
    await Future<void>.delayed(const Duration(milliseconds: 20));
    expect(bridge.chunkLengths, hasLength(1));
    expect(produced, 1);
    bridge.nextChunk!.complete();
    await uploading;
    expect(produced, 3);
  });

  for (final failure in [
    'beginAttachmentUpload',
    'writeAttachmentUpload',
    'completeAttachmentUpload',
    'attachUploadedFile',
  ]) {
    test(
      '$failure failure closes input and discards allocated uploads',
      () async {
        final bridge = UploadBridge()..failure = failure;
        final file = SelectedFileInput.fromStream(
          name: '文档.apk',
          byteLength: 3,
          stream: Stream.value(Uint8List.fromList([0, 1, 255])),
        );
        await expectLater(
          ChatViewModel(
            bridge: bridge,
          ).attachSelectedFile(file, expectedChatId: 'chat-a'),
          throwsStateError,
        );
        if (failure == 'beginAttachmentUpload') {
          expect(bridge.methods, ['beginAttachmentUpload']);
        } else {
          expect(bridge.methods.last, 'discardAttachmentUpload');
        }
        await expectLater(file.readChunk(), throwsStateError);
        if (failure != 'attachUploadedFile') {
          expect(bridge.methods, isNot(contains('attachUploadedFile')));
        }
      },
    );
  }

  test('unknown-length and empty inputs finish with actual size', () async {
    for (final length in [0, 3]) {
      final bridge = UploadBridge();
      final file = SelectedFileInput.fromStream(
        name: '文档.apk',
        byteLength: null,
        stream: length == 0
            ? const Stream.empty()
            : Stream.value(Uint8List(length)),
      );
      await ChatViewModel(
        bridge: bridge,
      ).attachSelectedFile(file, expectedChatId: 'chat-a');
      final complete = bridge.requests.firstWhere(
        (request) => request.methodName == 'completeAttachmentUpload',
      );
      expect((complete.args! as Map)['expectedByteLength'], length);
    }
  });

  test(
    'cancelled uploads discard committed content and do not attach it',
    () async {
      final bridge = UploadBridge();
      var cancelled = false;
      Stream<Uint8List> source() async* {
        yield Uint8List(selectedFileChunkBytes);
        cancelled = true;
        yield Uint8List(selectedFileChunkBytes);
      }

      final file = SelectedFileInput.fromStream(
        name: '文档.apk',
        byteLength: null,
        stream: source(),
      );
      await expectLater(
        ChatViewModel(bridge: bridge).attachSelectedFile(
          file,
          expectedChatId: 'chat-start',
          isCancelled: () => cancelled,
        ),
        throwsStateError,
      );
      expect(bridge.chunkLengths, hasLength(1));
      expect(bridge.methods.last, 'discardAttachmentUpload');
      expect(bridge.methods, isNot(contains('attachUploadedFile')));
      await expectLater(file.readChunk(), throwsStateError);
    },
  );

  test('truncated source never registers an attachment', () async {
    final bridge = UploadBridge();
    final file = SelectedFileInput.fromStream(
      name: '文档.apk',
      byteLength: 10,
      stream: Stream.value(Uint8List(3)),
    );
    await expectLater(
      ChatViewModel(
        bridge: bridge,
      ).attachSelectedFile(file, expectedChatId: 'chat-a'),
      throwsStateError,
    );
    expect(bridge.methods.last, 'discardAttachmentUpload');
    expect(bridge.methods, isNot(contains('attachUploadedFile')));
  });
}
