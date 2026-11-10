import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:operit2/core/host/IncomingAttachmentService.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

import 'streamed_file_attachment_test.dart' show UploadBridge;

/// Resets the single-upload fixture's counters for each independent native input.
class _BatchUploadBridge extends UploadBridge {
  /// Models a fresh transfer session for every file in a share operation.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    if (request.methodName == 'beginAttachmentUpload') {
      received = 0;
      completed = false;
    }
    return super.callBytes(request);
  }
}

/// Gives asynchronous inbox delivery and its cleanup a complete event-loop turn.
Future<void> settleInbox() =>
    Future<void>.delayed(const Duration(milliseconds: 20));

/// Creates an owned file input without touching a real platform filesystem.
SelectedFileInput file(String name) => SelectedFileInput.fromStream(
  name: name,
  byteLength: 3,
  stream: Stream.value(Uint8List.fromList([1, 2, 3])),
);

/// Sends an incoming native event through the actual adapter's MethodChannel receiver.
Future<void> nativeFiles(Object files) async {
  final completed = Completer<void>();
  TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .handlePlatformMessage(
        'operit/incoming_files',
        const StandardMethodCodec().encodeMethodCall(
          MethodCall('files', files),
        ),
        (_) => completed.complete(),
      );
  await completed.future;
}

/// Verifies startup queuing, warm delivery, stream transfer and conversation ownership.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'cold-start files wait for navigation and a selected composer',
    () async {
      final service = IncomingAttachmentService();
      final input = file('cold.txt');
      final delivered = <String>[];
      var navigated = 0;
      service.receive([input]);
      final owner = Object();
      service.installComposer(owner, (files) async {
        delivered.addAll(files.map((file) => file.name));
      });
      await settleInbox();
      expect(delivered, isEmpty);
      service.installNavigation(() => navigated++);
      await settleInbox();
      expect(navigated, 1);
      expect(delivered, ['cold.txt']);
      await expectLater(input.readChunk(), throwsStateError);
      service.clearComposer(owner);
      service.clearNavigation();
    },
  );

  test(
    'share batches are serialized and the next batch waits across a chat switch',
    () async {
      final service = IncomingAttachmentService();
      final first = file('first.txt');
      final second = file('second.txt');
      final blocked = Completer<void>();
      final started = Completer<void>();
      final ownerA = Object();
      final ownerB = Object();
      final delivered = <String>[];
      service.installNavigation(() {});
      service.installComposer(ownerA, (files) async {
        delivered.add('chat-a:${files.single.name}');
        started.complete();
        await blocked.future;
      });
      service.receive([first]);
      service.receive([second]);
      await started.future;
      service.clearComposer(ownerA);
      blocked.complete();
      await settleInbox();
      expect(delivered, ['chat-a:first.txt']);
      service.installComposer(ownerB, (files) async {
        delivered.add('chat-b:${files.single.name}');
      });
      // Disposing the old screen must not disconnect the newly selected composer.
      service.clearComposer(ownerA);
      await settleInbox();
      expect(delivered, ['chat-a:first.txt', 'chat-b:second.txt']);
      await expectLater(first.readChunk(), throwsStateError);
      await expectLater(second.readChunk(), throwsStateError);
      service.clearComposer(ownerB);
      service.clearNavigation();
    },
  );

  for (final adapter in [
    OperitFolderAccessAndroid(),
    OperitFolderAccessIOS(),
  ]) {
    test(
      '${adapter.runtimeType} attaches startup and warm files through the drop upload API',
      () async {
        const channel = MethodChannel('operit/incoming_files');
        final closed = <String>[];
        final reads = <String, int>{};
        final messenger =
            TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
        messenger.setMockMethodCallHandler(channel, (call) async {
          switch (call.method) {
            case 'ready':
              return [
                [
                  {'token': 'startup', 'name': '原文件.pdf', 'byteLength': 3},
                ],
              ];
            case 'readChunk':
              final args = call.arguments as Map;
              expect(args['maxBytes'], selectedFileChunkBytes);
              final token = args['token'] as String;
              reads[token] = (reads[token] ?? 0) + 1;
              return reads[token] == 1
                  ? Uint8List.fromList([1, 2, 3])
                  : Uint8List(0);
            case 'close':
              closed.add((call.arguments as Map)['token'] as String);
              return null;
            default:
              throw StateError(
                'Unexpected native input operation ${call.method}',
              );
          }
        });
        final service = IncomingAttachmentService();
        final bridge = _BatchUploadBridge();
        final viewModel = ChatViewModel(bridge: bridge);
        final errors = <Object>[];
        service.installNavigation(() {});
        final owner = Object();
        service.installComposer(owner, (files) async {
          for (final input in files) {
            await viewModel.attachSelectedFile(
              input,
              expectedChatId: 'current-chat',
            );
          }
        });
        await adapter.receiveIncomingFiles(
          onFiles: service.receive,
          onError: (error, _) => errors.add(error),
        );
        await settleInbox();
        await nativeFiles([
          {'token': 'warm-1', 'name': 'photo.jpg', 'byteLength': 3},
          {'token': 'warm-2', 'name': 'notes.txt', 'byteLength': 3},
        ]);
        await settleInbox();
        expect(errors, isEmpty);
        expect(closed, ['startup', 'warm-1', 'warm-2']);
        final attached = bridge.requests
            .where((request) => request.methodName == 'attachUploadedFile')
            .toList();
        expect(attached, hasLength(3));
        expect(
          attached.map((request) => (request.args as Map)['expectedChatId']),
          everyElement('current-chat'),
        );
        final uploads = bridge.requests.where(
          (request) => request.methodName == 'beginAttachmentUpload',
        );
        expect(uploads.map((request) => (request.args as Map)['fileName']), [
          '原文件.pdf',
          'photo.jpg',
          'notes.txt',
        ]);
        expect(bridge.methods, isNot(contains('sendMessage')));
        service.clearComposer(owner);
        service.clearNavigation();
        messenger.setMockMethodCallHandler(channel, null);
      },
    );
  }
}
