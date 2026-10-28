import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

void main() {
  test(
    'guidance uses the dedicated active-turn route without cancellation',
    () async {
      final bridge = _GuidanceBridge(encodeCoreLink(<Object?>[0, null]));

      await ChatViewModel(
        bridge: bridge,
      ).steerMessage(chatId: 'chat-a', messageText: 'Use a narrower plan');

      expect(bridge.requests, hasLength(1));
      final request = bridge.requests.single;
      expect(request.target, 'core/chatRuntimeHolderMain');
      expect(request.methodName, 'steerMessage');
      expect(request.methodName, isNot('cancelMessage'));
      expect(request.args, <String, Object?>{
        'chatId': 'chat-a',
        'messageText': 'Use a narrower plan',
      });
    },
  );

  test(
    'guidance propagates a closed-turn error without another send',
    () async {
      final bridge = _GuidanceBridge(
        encodeCoreLink(<Object?>[
          1,
          'TURN_CLOSED',
          'The current chat turn is no longer accepting guidance',
          null,
          null,
          null,
        ]),
      );

      await expectLater(
        ChatViewModel(
          bridge: bridge,
        ).steerMessage(chatId: 'chat-a', messageText: 'Too late'),
        throwsA(
          isA<CoreLinkError>()
              .having((error) => error.code, 'code', 'TURN_CLOSED')
              .having(
                (error) => error.message,
                'message',
                'The current chat turn is no longer accepting guidance',
              ),
        ),
      );
      expect(bridge.requests, hasLength(1));
      expect(bridge.requests.single.methodName, 'steerMessage');
    },
  );
}

class _GuidanceBridge extends OperitRuntimeBridge {
  _GuidanceBridge(this.response);

  final Uint8List response;
  final List<CoreCallRequest> requests = <CoreCallRequest>[];

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    requests.add(request);
    return response;
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) {
    throw UnsupportedError('Push streams are not used by this test');
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    throw UnsupportedError('Watch snapshots are not used by this test');
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    throw UnsupportedError('Watch streams are not used by this test');
  }
}
