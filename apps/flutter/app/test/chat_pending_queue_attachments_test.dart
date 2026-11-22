import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

/// Captures the exact generated Core calls made by the chat queue boundary.
class _QueueBridge extends OperitRuntimeBridge {
  final List<CoreCallRequest> calls = [];

  /// Attachments currently held by the Core composer snapshot.
  List<core_proxy.AttachmentInfo> coreAttachments = <core_proxy.AttachmentInfo>[];

  /// Item returned by the next automatic dequeue, when present.
  core_proxy.PendingQueueMessageItem? nextItem;

  List<CoreCallRequest> callsNamed(String methodName) =>
      calls.where((call) => call.methodName == methodName).toList();

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    Object? value;
    switch (request.methodName) {
      case 'enqueuePendingQueueMessage':
      case 'restorePendingQueueMessage':
      case 'clearAttachments':
      case 'sendUserMessage':
        value = null;
      case 'attachments':
        value = coreAttachments.map((item) => item.toJson()).toList();
      case 'takeNextPendingQueueMessageIfReady':
        value = nextItem?.toJson();
      case 'dispatchChatInputSubmitRequested':
        value = null;
      default:
        throw StateError('Unexpected method: ${request.methodName}');
    }
    return encodeCoreLink([0, value]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnsupportedError('no streaming send in queue tests');

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnsupportedError('no snapshots in queue tests');

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnsupportedError('no subscriptions in queue tests');
}

core_proxy.AttachmentInfo _attachment(String path) => core_proxy.AttachmentInfo(
  filePath: path,
  vfsPath: null,
  nodeId: null,
  fileName: path.split('/').last,
  mimeType: 'image/png',
  fileSize: 9,
  content: '',
);

void main() {
  test('enqueue forwards composer attachments to the Core queue call', () async {
    final bridge = _QueueBridge();
    final viewModel = ChatViewModel(bridge: bridge);
    final attachment = _attachment('/tmp/queued.png');
    await viewModel.enqueuePendingQueueMessage(
      chatId: 'chat-a',
      messageText: 'look at this',
      attachments: <core_proxy.AttachmentInfo>[attachment],
    );
    final call = bridge.callsNamed('enqueuePendingQueueMessage').single;
    final args = call.args as Map<String, Object?>;
    expect(args['chatId'], 'chat-a');
    expect(args['messageText'], 'look at this');
    expect(args['attachments'], <Object?>[attachment.toJson()]);
  });

  test('automatic dequeue decodes the attachments stored on the item', () async {
    final bridge = _QueueBridge();
    final attachment = _attachment('/tmp/queued.png');
    bridge.nextItem = core_proxy.PendingQueueMessageItem(
      id: 4,
      text: 'look at this',
      attachments: <core_proxy.AttachmentInfo>[attachment],
    );
    final viewModel = ChatViewModel(bridge: bridge);
    final item = await viewModel.takeNextPendingQueueMessageIfReady('chat-a');
    expect(item, isNotNull);
    expect(item!.attachments.single.filePath, '/tmp/queued.png');
  });

  test('restore round-trips the full item including attachments', () async {
    final bridge = _QueueBridge();
    final attachment = _attachment('/tmp/queued.png');
    final viewModel = ChatViewModel(bridge: bridge);
    final item = core_proxy.PendingQueueMessageItem(
      id: 4,
      text: 'look at this',
      attachments: <core_proxy.AttachmentInfo>[attachment],
    );
    await viewModel.restorePendingQueueMessage(chatId: 'chat-a', message: item);
    final args =
        bridge.callsNamed('restorePendingQueueMessage').single.args
            as Map<String, Object?>;
    expect((args['message'] as Map<String, Object?>)['attachments'], <Object?>[
      attachment.toJson(),
    ]);
  });

  test('queued send uses its captured attachments and leaves Core untouched', () async {
    final bridge = _QueueBridge();
    bridge.coreAttachments = <core_proxy.AttachmentInfo>[
      _attachment('/tmp/composer.png'),
    ];
    final viewModel = ChatViewModel(bridge: bridge);
    final queued = _attachment('/tmp/queued.png');
    await viewModel.sendUserMessage(
      'look at this',
      chatIdOverride: 'chat-a',
      attachmentsOverride: <core_proxy.AttachmentInfo>[queued],
    );
    final args =
        bridge.callsNamed('sendUserMessage').single.args as Map<String, Object?>;
    expect(args['attachments'], <Object?>[queued.toJson()]);
    expect(bridge.callsNamed('clearAttachments'), isEmpty);
    expect(bridge.callsNamed('attachments'), isEmpty);
  });

  test('normal send still reads and clears the Core composer attachments', () async {
    final bridge = _QueueBridge();
    final composer = _attachment('/tmp/composer.png');
    bridge.coreAttachments = <core_proxy.AttachmentInfo>[composer];
    final viewModel = ChatViewModel(bridge: bridge);
    await viewModel.sendUserMessage('hello', chatIdOverride: 'chat-a');
    final args =
        bridge.callsNamed('sendUserMessage').single.args as Map<String, Object?>;
    expect(args['attachments'], <Object?>[composer.toJson()]);
    expect(bridge.callsNamed('clearAttachments'), hasLength(1));
  });

  test('screen enqueue captures attachments and only refreshes after Core accepts', () {
    final source = File(
      'lib/ui/features/chat/screens/AIChatScreen.dart',
    ).readAsStringSync();
    final enqueue = source.substring(
      source.indexOf('Future<void> _enqueueDraftToPendingQueueInRuntime()'),
      source.indexOf('void _clearSubmittedDraft('),
    );
    expect(enqueue, contains('final submittedAttachments = _attachments;'));
    expect(enqueue, contains('attachments: submittedAttachments,'));
    expect(enqueue, contains('await _refreshAttachments();'));
    expect(enqueue, isNot(contains('clearAttachments(')));
    expect(source, isNot(contains('_clearSubmittedAttachments')));
  });

  test('screen dequeued send delivers the item attachments end to end', () {
    final source = File(
      'lib/ui/features/chat/screens/AIChatScreen.dart',
    ).readAsStringSync();
    final send = source.substring(
      source.indexOf('Future<void> _sendQueuedItemNow('),
      source.indexOf('void _setPendingQueueExpanded('),
    );
    expect(send, contains('attachments: item.attachments,'));
    expect(send, contains('attachmentsOverride: item.attachments,'));
    expect(send, isNot(contains('attachments: const [],')));
  });
}
