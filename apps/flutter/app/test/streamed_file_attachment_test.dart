import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/host/SelectedFileInput.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

class UploadBridge extends OperitRuntimeBridge {
  UploadBridge({this.nodeId = 'receiver'});
  final String nodeId;
  final methods = <String>[];
  final requests = <CoreCallRequest>[];
  final chunkLengths = <int>[];
  String? failure;
  Completer<void>? nextChunk;
  int received = 0;
  int maxFileBytes = 16 * 1024 * 1024;
  int maxTotalBytes = 256 * 1024 * 1024;
  bool completed = false;

  String reference(String name, int size) => jsonEncode({
    'format': 'operit_file_reference',
    'version': 1,
    'nodeId': nodeId,
    'path': '/virtual/local/attachments/source',
    'fileName': name,
    'fileSize': size,
  });

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    methods.add(request.methodName);
    requests.add(request);
    if (failure == request.methodName) throw StateError('failure: $failure');
    final args = request.args! as Map;
    final name = args['fileName'] as String? ?? 'file.bin';
    Object? result;
    switch (request.methodName) {
      case 'selectedAttachmentPolicy':
        result = jsonEncode({
          'maxFileBytes': maxFileBytes,
          'maxTotalBytes': maxTotalBytes,
        });
      case 'beginAttachmentUpload':
        result = 'opaque-upload';
      case 'registerAttachmentSource':
        result = reference(name, args['expectedByteLength'] as int);
      case 'completeAttachmentSource':
        result = reference(name, received);
      case 'createReferenceAttachment':
        final source = jsonDecode(args['referenceJson'] as String) as Map;
        result = metadata(
          '${source['fileName']}.operit-ref.json',
          (args['referenceJson'] as String).length,
        );
      case 'completeSelectedAttachment':
        result = metadata(name, received);
      case 'attachUploadedFile':
        break;
      default:
        throw StateError('Unexpected ${request.methodName}');
    }
    return encodeCoreLink([0, result]);
  }

  Map<String, Object?> metadata(String name, int size) => {
    'filePath':
        '/runtime/data/chat_outputs/family/.operit/attachments/id/$name',
    'vfsPath': '/operitapp/outputs/family/.operit/attachments/id/$name',
    'nodeId': nodeId,
    'fileName': name,
    'mimeType': 'application/octet-stream',
    'fileSize': size,
    'content': '',
  };

  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    expect(
      request.methodName,
      anyOf('writeSelectedAttachment', 'writeAttachmentSource'),
    );
    methods.add(request.methodName);
    return UploadSink(this, request);
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class UploadSink implements CorePushSink {
  UploadSink(this.bridge, this.request);
  final UploadBridge bridge;
  final CorePushRequest request;
  @override
  Future<void> add(Object? args) async {
    final bytes = args! as Uint8List;
    expect(bytes.length, lessThanOrEqualTo(selectedFileChunkBytes));
    if (bytes.isEmpty) {
      bridge.completed = true;
      return;
    }
    bridge.chunkLengths.add(bytes.length);
    bridge.received += bytes.length;
    if (bridge.failure == request.methodName) {
      throw StateError('write failed');
    }
    final limit = request.methodName == 'writeSelectedAttachment'
        ? bridge.maxFileBytes
        : (request.args! as Map)['maxBytes'] as int;
    if (bridge.received > limit) throw StateError('retention limit exceeded');
    await bridge.nextChunk?.future;
  }

  @override
  Future<void> close() async {
    if (!bridge.completed) throw StateError('missing completion marker');
    final expected = (request.args! as Map)['expectedByteLength'] as int?;
    if (expected != null && expected != bridge.received) {
      throw StateError('size mismatch');
    }
  }
}

void main() {
  test('a missing chat rejects input before reading or allocating', () async {
    final bridge = UploadBridge();
    final file = SelectedFileInput.fromStream(
      name: 'a.bin',
      byteLength: 3,
      stream: Stream.value(Uint8List(3)),
    );
    await expectLater(
      ChatViewModel(
        bridge: bridge,
      ).attachSelectedFile(file, expectedChatId: null),
      throwsStateError,
    );
    expect(bridge.methods, isEmpty);
    await expectLater(file.readChunk(), throwsStateError);
  });

  test(
    'a multi-gigabyte durable file sends only a locator from the actual source node',
    () async {
      final receiver = UploadBridge();
      final sourceNode = UploadBridge(nodeId: 'source-node');
      var reads = 0;
      Stream<Uint8List> source() async* {
        reads++;
        throw StateError('must not read');
      }

      final file = SelectedFileInput.fromStream(
        name: 'huge.zip',
        byteLength: 8 * 1024 * 1024 * 1024,
        sourceJson: '{"backend":"native","root":"/source/huge.zip"}',
        stream: source(),
      );
      await ChatViewModel(
        bridge: receiver,
        attachmentSourceBridge: sourceNode,
      ).attachSelectedFile(file, expectedChatId: 'chat-a');
      expect(reads, 0);
      expect(sourceNode.methods, ['registerAttachmentSource']);
      expect(receiver.methods, [
        'selectedAttachmentPolicy',
        'createReferenceAttachment',
        'attachUploadedFile',
      ]);
      final saved =
          receiver.requests
                  .firstWhere(
                    (r) => r.methodName == 'createReferenceAttachment',
                  )
                  .args!
              as Map;
      expect(
        jsonDecode(saved['referenceJson'] as String)['nodeId'],
        'source-node',
      );
      await expectLater(file.readChunk(), throwsStateError);
    },
  );

  test(
    'small files write once to final storage with bounded backpressure',
    () async {
      final bridge = UploadBridge()..nextChunk = Completer<void>();
      var produced = 0;
      Stream<Uint8List> source() async* {
        for (var i = 0; i < 3; i++) {
          produced++;
          yield Uint8List(selectedFileChunkBytes);
        }
      }

      final file = SelectedFileInput.fromStream(
        name: 'a.bin',
        byteLength: 3 * selectedFileChunkBytes,
        stream: source(),
      );
      final uploading = ChatViewModel(
        bridge: bridge,
      ).attachSelectedFile(file, expectedChatId: 'chat-a');
      await Future<void>.delayed(const Duration(milliseconds: 20));
      expect(produced, 1);
      expect(bridge.chunkLengths, hasLength(1));
      bridge.nextChunk!.complete();
      await uploading;
      expect(bridge.methods, [
        'selectedAttachmentPolicy',
        'beginAttachmentUpload',
        'writeSelectedAttachment',
        'completeSelectedAttachment',
        'attachUploadedFile',
      ]);
      expect(bridge.completed, isTrue);
    },
  );

  test(
    'ephemeral selections beyond the retention limit fail before streaming',
    () async {
      final bridge = UploadBridge()..maxTotalBytes = 64;
      var read = false;
      Stream<Uint8List> source() async* {
        read = true;
        yield Uint8List(1024);
      }

      final file = SelectedFileInput.fromStream(
        name: 'large.bin',
        byteLength: 1024,
        stream: source(),
      );
      await expectLater(
        ChatViewModel(
          bridge: bridge,
        ).attachSelectedFile(file, expectedChatId: 'chat-a'),
        throwsStateError,
      );
      expect(read, isFalse);
      expect(bridge.methods, ['selectedAttachmentPolicy']);
    },
  );

  test(
    'an unknown-length input is retained on the source node and publishes only its locator',
    () async {
      final receiver = UploadBridge();
      final source = UploadBridge(nodeId: 'browser-node');
      final file = SelectedFileInput.fromStream(
        name: 'blob.bin',
        byteLength: null,
        stream: Stream.value(Uint8List(3)),
      );
      await ChatViewModel(
        bridge: receiver,
        attachmentSourceBridge: source,
      ).attachSelectedFile(file, expectedChatId: 'chat-a');
      expect(source.received, 3);
      expect(source.completed, isTrue);
      expect(source.methods, [
        'writeAttachmentSource',
        'completeAttachmentSource',
      ]);
      expect(receiver.chunkLengths, isEmpty);
      expect(receiver.methods, contains('createReferenceAttachment'));
    },
  );

  test(
    'cancellation does not send a successful EOF or attach a partial source',
    () async {
      final bridge = UploadBridge();
      var cancelled = false;
      Stream<Uint8List> source() async* {
        yield Uint8List(3);
        cancelled = true;
        yield Uint8List(3);
      }

      final file = SelectedFileInput.fromStream(
        name: 'a.bin',
        byteLength: 6,
        stream: source(),
      );
      await expectLater(
        ChatViewModel(bridge: bridge).attachSelectedFile(
          file,
          expectedChatId: 'chat-a',
          isCancelled: () => cancelled,
        ),
        throwsStateError,
      );
      expect(bridge.completed, isFalse);
      expect(bridge.methods, isNot(contains('attachUploadedFile')));
      await expectLater(file.readChunk(), throwsStateError);
    },
  );

  for (final failure in [
    'selectedAttachmentPolicy',
    'beginAttachmentUpload',
    'writeSelectedAttachment',
    'completeSelectedAttachment',
    'attachUploadedFile',
  ]) {
    test('$failure failure always releases the selected input', () async {
      final bridge = UploadBridge()..failure = failure;
      final file = SelectedFileInput.fromStream(
        name: 'a.bin',
        byteLength: 3,
        stream: Stream.value(Uint8List(3)),
      );
      await expectLater(
        ChatViewModel(
          bridge: bridge,
        ).attachSelectedFile(file, expectedChatId: 'chat-a'),
        throwsStateError,
      );
      await expectLater(file.readChunk(), throwsStateError);
    });
  }
}
