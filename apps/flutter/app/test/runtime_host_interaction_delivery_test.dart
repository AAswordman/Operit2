import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/RuntimeHostInteractionDelivery.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';

/// Builds a file-open event without depending on native platform channels.
RuntimeHostInteractionRequest _request(String id) =>
    RuntimeHostInteractionRequest.fromJson({
      'requestId': id,
      'kind': 'file_open',
      'fileOpen': {'path': '/tmp/file'},
    });

/// Builds the same typed result returned by a native file opener.
RuntimeHostInteractionResponse _response({String? error}) =>
    RuntimeHostInteractionResponse.fromJson({
      'error': error,
      if (error == null) 'fileOpen': {'success': true, 'error': null},
    });

/// Exercises dispatch ownership separately from the result delivery connection.
void main() {
  late RuntimeHostInteractionDelivery delivery;
  late RuntimeHostInteractionResponse result;
  late List<RuntimeHostInteractionResponse> sent;
  late List<Object> reported;
  var dispatchCount = 0;
  Object? dispatchError;
  Object? responseError;
  Completer<RuntimeHostInteractionResponse>? nativeResult;

  setUp(() {
    result = _response();
    sent = [];
    reported = [];
    dispatchCount = 0;
    dispatchError = null;
    responseError = null;
    nativeResult = null;
    delivery = RuntimeHostInteractionDelivery(
      dispatch: (_) async {
        dispatchCount++;
        if (dispatchError != null) throw dispatchError!;
        return nativeResult?.future ?? result;
      },
      respond: (id, response) async {
        sent.add(response);
        if (responseError != null) throw responseError!;
      },
      errorResponse: (error) => _response(error: error.toString()),
      onDispatchError: (error, stack) => reported.add(error),
    );
  });

  test(
    'duplicate events share execution and wait for the native result',
    () async {
      nativeResult = Completer();
      final first = delivery.deliver(_request('open'));
      final replay = delivery.deliver(_request('open'));
      expect(identical(first, replay), isTrue);
      expect(dispatchCount, 1);
      expect(sent, isEmpty);

      nativeResult!.complete(result);
      expect(await first, isA<Duration>());
      await replay;
      expect(sent, [same(result)]);
      expect(delivery.pending, isEmpty);
    },
  );

  for (final error in <Object>[
    StateError('connection closed'),
    const CoreLinkError(code: 'PEER_LINK_CLOSED', message: 'peer disconnected'),
  ]) {
    test('reconnect resends the original result after $error', () async {
      responseError = error;
      await expectLater(
        delivery.deliver(_request('open')),
        throwsA(same(error)),
      );
      expect(delivery.pending.single.requestId, 'open');

      responseError = null;
      await delivery.deliver(delivery.pending.single);
      expect(dispatchCount, 1);
      expect(sent, [same(result), same(result)]);
      expect(delivery.pending, isEmpty);
    });
  }

  test(
    'native errors remain visible and survive a response disconnect',
    () async {
      dispatchError = StateError('no application can open this file');
      responseError = StateError('connection closed');
      await expectLater(delivery.deliver(_request('failed')), throwsStateError);
      final original = sent.single;
      expect(original.error, dispatchError.toString());
      expect(reported, [same(dispatchError)]);

      responseError = null;
      await delivery.deliver(delivery.pending.single);
      expect(dispatchCount, 1);
      expect(reported, hasLength(1));
      expect(sent.last, same(original));
      expect(delivery.pending, isEmpty);
    },
  );

  test('expired broker requests retire their cached result', () async {
    responseError = const CoreLinkError(
      code: 'COMMAND_ERROR',
      message: 'host interaction request not found: expired',
    );
    await expectLater(
      delivery.deliver(_request('expired')),
      throwsA(same(responseError)),
    );
    expect(dispatchCount, 1);
    expect(delivery.pending, isEmpty);
  });

  test(
    'lost acceptance acknowledgement never repeats the native action',
    () async {
      responseError = StateError('accepted but connection lost before reply');
      await expectLater(
        delivery.deliver(_request('accepted')),
        throwsStateError,
      );
      responseError = const CoreLinkError(
        code: 'COMMAND_ERROR',
        message: 'host interaction request not found: accepted',
      );
      await expectLater(
        delivery.deliver(delivery.pending.single),
        throwsA(same(responseError)),
      );
      expect(dispatchCount, 1);
      expect(delivery.pending, isEmpty);
    },
  );

  test(
    'uninstall prevents an unfinished native action from answering',
    () async {
      nativeResult = Completer();
      final oldSession = delivery.deliver(_request('old-session'));
      delivery.clear();
      nativeResult!.complete(result);
      expect(await oldSession, isNull);
      expect(sent, isEmpty);
      expect(delivery.pending, isEmpty);

      nativeResult = null;
      await delivery.deliver(_request('new-session'));
      expect(dispatchCount, 2);
      expect(sent, [same(result)]);
    },
  );
}
