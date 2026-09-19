import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/FfiCoreProxy.dart';
import 'package:operit2/core/bridge/FfiCoreTransport.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';

typedef _Post = Bool Function(Int64, Pointer<_DartObject>);

final class _TypedData extends Struct {
  @Int32()
  external int type;
  @IntPtr()
  external int length;
  external Pointer<Uint8> bytes;
}

final class _ExternalData extends Struct {
  @Int32()
  external int type;
  @IntPtr()
  external int length;
  external Pointer<Void> bytes;
  external Pointer<Void> peer;
  external Pointer<Void> callback;
}

final class _ObjectValue extends Union {
  external _TypedData data;
  external _ExternalData externalData;
  @Int64()
  external int integer;
  @Double()
  external double doubleValue;
}

final class _DartObject extends Struct {
  @Int32()
  external int type;
  external _ObjectValue value;
}

class _HostConnection {
  /// Allocates a token released by the real native allocator finalizer.
  _HostConnection() : token = calloc<Uint8>().cast<Void>();

  final Pointer<Void> token;
  late bool Function(int, Pointer<_DartObject>) post;
  late int port;
  final List<List<Object?>> items = [];
  final List<String> closedPushes = [];
  final List<String> closedWatches = [];
  final Map<String, int> watches = {};
  final Map<String, int> delayed = {};

  /// Publishes function pointers using the same versioned descriptor as the Rust host.
  String descriptor() => jsonEncode({
    'version': 1,
    'session': token.address.toString(),
    'attach':
        Pointer.fromFunction<
              Bool Function(Pointer<Void>, Pointer<Void>, Int64)
            >(_attach, false)
            .address
            .toString(),
    'submit':
        Pointer.fromFunction<
              Void Function(
                Pointer<Void>,
                Uint32,
                Int64,
                Pointer<Uint8>,
                UintPtr,
              )
            >(_submit)
            .address
            .toString(),
    'allocate': Pointer.fromFunction<Pointer<Uint8> Function(UintPtr)>(
      _allocate,
    ).address.toString(),
    'free': Pointer.fromFunction<Void Function(Pointer<Uint8>, UintPtr)>(
      _free,
    ).address.toString(),
    'release': calloc.nativeFree.address.toString(),
  });

  /// Copies a framed typed-data message through the actual Dart VM native API.
  void send(int kind, int id, Uint8List payload) {
    final bytes = calloc<Uint8>(9 + payload.length);
    final frame = bytes.asTypedList(9 + payload.length);
    frame[0] = kind;
    ByteData.sublistView(frame).setInt64(1, id, Endian.little);
    frame.setRange(9, frame.length, payload);
    final object = calloc<_DartObject>();
    object.ref.type = 7;
    object.ref.value.data
      ..type = 2
      ..length = frame.length
      ..bytes = bytes;
    try {
      expect(post(port, object), isTrue);
    } finally {
      calloc.free(object);
      calloc.free(bytes);
    }
  }

  /// Completes one request with a compact success result.
  void reply(int id, Object? value) => send(0, id, encodeCoreLink([0, value]));

  /// Emits one event before the watch-open response to exercise buffering.
  void event(int id, String subscription, String value) => send(
    1,
    id,
    encodeCoreLink([
      subscription,
      [null, 7, 'items', 'Snapshot', value],
    ]),
  );
}

final Map<int, _HostConnection> _connections = {};
int _allocatedBuffers = 0;
int _freedBuffers = 0;

/// Captures the VM's thread-safe post entry point and connection-specific port.
bool _attach(Pointer<Void> token, Pointer<Void> post, int port) {
  final connection = _connections[token.address]!;
  connection.post = post
      .cast<NativeFunction<_Post>>()
      .asFunction<bool Function(int, Pointer<_DartObject>)>();
  connection.port = port;
  return true;
}

/// Allocates the request buffer exposed by the test host ABI.
Pointer<Uint8> _allocate(int length) {
  _allocatedBuffers++;
  return calloc<Uint8>(length);
}

/// Records the release of every submitted request buffer.
void _free(Pointer<Uint8> pointer, int length) {
  _freedBuffers++;
  calloc.free(pointer);
}

/// Copies input synchronously and dispatches the response after the FFI call returns.
void _submit(
  Pointer<Void> token,
  int operation,
  int id,
  Pointer<Uint8> pointer,
  int length,
) {
  final connection = _connections[token.address]!;
  final bytes = Uint8List.fromList(pointer.asTypedList(length));
  _free(pointer, length);
  scheduleMicrotask(() {
    switch (operation) {
      case 0:
        final call = decodeCoreLink<List<Object?>>(bytes);
        if (call[2] == 'delayed') {
          connection.delayed[call[0] as String] = id;
        } else {
          connection.reply(id, call[3]);
        }
      case 1:
        connection.reply(id, 'push-1');
      case 2:
        final item = decodeCoreLink<List<Object?>>(bytes);
        connection.items.add(item);
        if (item[2] == 'fail') {
          connection.send(
            0,
            id,
            encodeCoreLink([1, 'SEND_FAILED', 'send failed', null, null, null]),
          );
        } else {
          connection.reply(id, null);
        }
      case 3:
        connection.closedPushes.add(utf8.decode(bytes));
        connection.reply(id, null);
      case 4:
        connection.reply(id, [null, 7, 'items', 'Snapshot', 'snapshot']);
      case 5:
        final watch = decodeCoreLink<List<Object?>>(bytes);
        final subscription = watch[0] as String;
        connection.watches[subscription] = id;
        connection.event(id, subscription, 'early');
        if (watch[3] == 'openDelayed') return;
        connection.reply(id, subscription);
        if (watch[3] == 'failure') {
          connection.send(
            2,
            id,
            encodeCoreLink([
              1,
              'WATCH_FAILED',
              'source failed',
              null,
              null,
              null,
            ]),
          );
          connection.send(3, id, Uint8List(0));
        } else if (watch[3] == 'finite') {
          connection.send(3, id, Uint8List(0));
        }
      case 6:
        connection.closedWatches.add(utf8.decode(bytes));
        connection.reply(id, null);
    }
  });
}

/// Creates a compact call request for transport integration tests.
CoreCallRequest _call(String id, String method, Object? args) =>
    CoreCallRequest(
      requestId: id,
      targetObjectId: 7,
      methodName: method,
      args: args,
    );

/// Creates a watch whose property selects the test host's stream behavior.
CoreWatchRequest _watch(String property) => CoreWatchRequest(
  requestId: 'watch',
  targetObjectId: 7,
  propertyName: property,
  args: null,
);

/// Exercises direct ABI calls and real VM port delivery without compiling the application.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late FfiCoreTransport transport;
  late FfiCoreProxy proxy;
  late _HostConnection host;
  late List<String> platformMethods;

  setUp(() {
    platformMethods = [];
    transport = FfiCoreTransport();
    proxy = FfiCoreProxy(transport: transport);
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(const MethodChannel('operit/runtime'), (
          call,
        ) async {
          platformMethods.add(call.method);
          expect(call.method, 'connectCoreFfi');
          host = _HostConnection();
          _connections[host.token.address] = host;
          return host.descriptor();
        });
  });

  tearDown(() async {
    await transport.close();
    expect(_allocatedBuffers, _freedBuffers);
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(const MethodChannel('operit/runtime'), null);
  });

  test('calls and snapshots use FFI after one host bootstrap', () async {
    expect(
      await proxy.call(_call('one', 'echo', Uint8List.fromList([0, 255]))),
      [0, 255],
    );
    expect(await proxy.call(_call('two', 'echo', 'text')), 'text');
    expect((await proxy.watchSnapshot(_watch('items'))).value, 'snapshot');
    expect(platformMethods, ['connectCoreFfi']);
  });

  test('retries connection after storage becomes configured', () async {
    var attempts = 0;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(const MethodChannel('operit/runtime'), (call) async {
          attempts++;
          if (attempts == 1) {
            throw PlatformException(code: 'OPERIT_RUNTIME_ERROR',
                message: 'Runtime and workspace roots are not configured');
          }
          host = _HostConnection();
          _connections[host.token.address] = host;
          return host.descriptor();
        });
    await expectLater(proxy.call(_call('before', 'echo', 'before')),
        throwsA(isA<PlatformException>()));
    expect(await proxy.call(_call('after', 'echo', 'ready')), 'ready');
    expect(attempts, 2);
  });

  test('concurrent replies retain their request identity', () async {
    final first = proxy.call(_call('one', 'delayed', null));
    final second = proxy.call(_call('two', 'delayed', null));
    await pumpEventQueue();
    host.reply(host.delayed['two']!, 'second');
    host.reply(host.delayed['one']!, 'first');
    expect(await Future.wait([first, second]), ['first', 'second']);
  });

  test(
    'push preserves order and releases a failed stream exactly once',
    () async {
      final push = await proxy.push(
        const CorePushRequest(
          requestId: 'push',
          targetObjectId: 7,
          methodName: 'input',
        ),
      );
      await Future.wait([push.add('first'), push.add('second')]);
      await expectLater(push.add('fail'), throwsA(isA<CoreLinkError>()));
      final closing = push.close();
      expect(identical(closing, push.close()), isTrue);
      await expectLater(closing, throwsA(isA<CoreLinkError>()));
      expect(host.items.map((item) => item[1]), [0, 1, 2]);
      expect(host.closedPushes, ['push-1']);
      expect(() => push.add('late'), throwsStateError);
    },
  );

  test(
    'early events survive open acknowledgement and finite sources close',
    () async {
      final events = await proxy.watchStream(_watch('finite')).toList();
      expect(events.map((event) => event.value), ['early']);
      expect(host.closedWatches, hasLength(1));
    },
  );

  test('source failures reach the listener and release the watch', () async {
    final values = <Object?>[];
    final errors = <Object>[];
    final done = Completer<void>();
    proxy
        .watchStream(_watch('failure'))
        .listen(
          (event) => values.add(event.value),
          onError: errors.add,
          onDone: done.complete,
        );
    await done.future;
    expect(values, ['early']);
    expect(
      errors.single,
      isA<CoreLinkError>().having(
        (error) => error.code,
        'code',
        'WATCH_FAILED',
      ),
    );
    expect(host.closedWatches, hasLength(1));
  });

  test(
    'cancellation during opening waits for acknowledgement and releases native state',
    () async {
      final received = Completer<void>();
      final subscription = proxy
          .watchStream(_watch('openDelayed'))
          .listen((_) => received.complete());
      await received.future;
      final cancelling = subscription.cancel();
      final watch = host.watches.entries.single;
      host.reply(watch.value, watch.key);
      await cancelling;
      expect(host.closedWatches, [watch.key]);
    },
  );

  test(
    'closing a connection fails pending work and rejects later calls',
    () async {
      final pending = proxy.call(_call('one', 'delayed', null));
      final assertion = expectLater(pending, throwsStateError);
      await pumpEventQueue();
      await transport.close();
      await assertion;
      await expectLater(
        proxy.call(_call('two', 'echo', null)),
        throwsStateError,
      );
    },
  );

  test(
    'independent connections do not consume each other\'s watch events',
    () async {
      final firstEvents = <Object?>[];
      final first = proxy
          .watchStream(_watch('items'))
          .listen((event) => firstEvents.add(event.value));
      await pumpEventQueue();
      final firstHost = host;
      final otherTransport = FfiCoreTransport();
      final otherProxy = FfiCoreProxy(transport: otherTransport);
      final secondEvents = <Object?>[];
      final second = otherProxy
          .watchStream(_watch('items'))
          .listen((event) => secondEvents.add(event.value));
      try {
        await pumpEventQueue();
        final secondHost = host;
        final firstWatch = firstHost.watches.entries.single;
        final secondWatch = secondHost.watches.entries.single;
        firstHost.event(firstWatch.value, firstWatch.key, 'first only');
        secondHost.event(secondWatch.value, secondWatch.key, 'second only');
        await pumpEventQueue();
        expect(firstEvents, ['early', 'first only']);
        expect(secondEvents, ['early', 'second only']);
        await first.cancel();
        expect(firstHost.closedWatches, [firstWatch.key]);
        expect(secondHost.closedWatches, isEmpty);
      } finally {
        await first.cancel();
        await second.cancel();
        await otherTransport.close();
      }
    },
  );

  test(
    'invalid transport frames fail pending work and detach the connection',
    () async {
      final pending = proxy.call(_call('one', 'delayed', null));
      final assertion = expectLater(pending, throwsFormatException);
      await pumpEventQueue();
      host.send(255, 0, Uint8List(0));
      await assertion;
      await expectLater(
        proxy.call(_call('two', 'echo', null)),
        throwsStateError,
      );
    },
  );
}
