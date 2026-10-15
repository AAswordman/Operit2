import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/FfiCoreProxy.dart';
import 'package:operit2/core/bridge/FfiCoreTransport.dart';
import 'package:operit2/core/bridge/CoreByteTransport.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';

typedef _FixtureSendNative =
    Bool Function(
      Pointer<Void>,
      Int64,
      Pointer<Uint8>,
      UintPtr,
      Pointer<Pointer<Void>>,
      UintPtr,
    );

late Pointer<Uint8> Function(int) _fixtureAllocate;
late bool Function(
  Pointer<Void>,
  int,
  Pointer<Uint8>,
  int,
  Pointer<Pointer<Void>>,
  int,
)
_fixtureSend;
late DynamicLibrary _fixtureLibrary;
late Pointer<NativeFunction<Void Function(Pointer<Void>)>>
_fixtureBufferRelease;
late Pointer<Void> Function(int) _fixtureBufferAllocate;
late Pointer<Uint8> Function(Pointer<Void>) _fixtureBufferData;
late void Function(Pointer<Void>) _fixtureBufferSeal;
late Pointer<Void> Function(Pointer<Void>) _fixtureBufferRetain;
late Pointer<Uint8> Function(Pointer<Void>) _fixtureBufferView;
late int Function(Pointer<Void>) _fixtureBufferLength;
late int Function() _fixtureLiveAllocations;

/// Loads the production Rust sender in a dependency-free native test library.
Future<void> _loadPortFixture() async {
  final output = await Directory('build/native_tests').create(recursive: true);
  final manifest = Directory.current.uri
      .resolve(
        '../../../hosts/common/operit-host-native-common/tests/port-fixture/Cargo.toml',
      )
      .toFilePath();
  final result = await Process.run(
    'cargo',
    [
      'build',
      '--offline',
      '--manifest-path',
      manifest,
      '--target-dir',
      '${output.absolute.path}/cargo',
      '--message-format=json',
    ],
    environment: {...Platform.environment, 'RUSTFLAGS': '-Awarnings'},
  );
  if (result.exitCode != 0) {
    throw StateError('Native fixture failed: ${result.stderr}');
  }
  final artifacts = const LineSplitter()
      .convert(result.stdout as String)
      .map((line) => jsonDecode(line) as Map<String, dynamic>)
      .where(
        (message) =>
            message['reason'] == 'compiler-artifact' &&
            message['target']['name'] == 'operit_dart_port_fixture',
      );
  final library = (artifacts.single['filenames'] as List).first as String;
  final native = DynamicLibrary.open(library);
  _fixtureLibrary = native;
  _fixtureBufferRelease = native
      .lookup<NativeFunction<Void Function(Pointer<Void>)>>(
        'fixture_buffer_release',
      );
  _fixtureBufferAllocate = native
      .lookupFunction<
        Pointer<Void> Function(UintPtr),
        Pointer<Void> Function(int)
      >('fixture_buffer_allocate');
  _fixtureBufferData = native
      .lookupFunction<
        Pointer<Uint8> Function(Pointer<Void>),
        Pointer<Uint8> Function(Pointer<Void>)
      >('fixture_buffer_data');
  _fixtureBufferSeal = native
      .lookupFunction<
        Void Function(Pointer<Void>),
        void Function(Pointer<Void>)
      >('fixture_buffer_seal');
  _fixtureBufferRetain = native
      .lookupFunction<
        Pointer<Void> Function(Pointer<Void>),
        Pointer<Void> Function(Pointer<Void>)
      >('fixture_buffer_retain');
  _fixtureBufferView = native
      .lookupFunction<
        Pointer<Uint8> Function(Pointer<Void>),
        Pointer<Uint8> Function(Pointer<Void>)
      >('fixture_buffer_view');
  _fixtureBufferLength = native
      .lookupFunction<
        UintPtr Function(Pointer<Void>),
        int Function(Pointer<Void>)
      >('fixture_buffer_length');
  _fixtureAllocate = native
      .lookupFunction<
        Pointer<Uint8> Function(UintPtr),
        Pointer<Uint8> Function(int)
      >('fixture_allocate');
  _fixtureSend = native
      .lookupFunction<
        _FixtureSendNative,
        bool Function(
          Pointer<Void>,
          int,
          Pointer<Uint8>,
          int,
          Pointer<Pointer<Void>>,
          int,
        )
      >('fixture_send');
  _fixtureLiveAllocations = native
      .lookupFunction<UintPtr Function(), int Function()>(
        'fixture_live_allocations',
      );
}

class _HostConnection {
  /// Allocates a token released by the real native allocator finalizer.
  _HostConnection() : token = calloc<Uint8>().cast<Void>();

  final Pointer<Void> token;
  late Pointer<Void> post;
  late int port;
  final List<List<Object?>> items = [];
  final List<String> closedPushes = [];
  final List<String> closedWatches = [];
  final Map<String, int> watches = {};
  final Map<String, int> delayed = {};
  Pointer<Uint8>? lastPayload;

  /// Publishes function pointers using the same versioned descriptor as the Rust host.
  String descriptor() => jsonEncode({
    'version': 2,
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
                Pointer<Pointer<Void>>,
                UintPtr,
              )
            >(_submit)
            .address
            .toString(),
    'allocate': Pointer.fromFunction<Pointer<Uint8> Function(UintPtr)>(
      _allocate,
    ).address.toString(),
    'bufferAllocate': _fixtureLibrary
        .lookup<NativeFunction<Pointer<Void> Function(UintPtr)>>(
          'fixture_buffer_allocate',
        )
        .address
        .toString(),
    'bufferData': _fixtureLibrary
        .lookup<NativeFunction<Pointer<Uint8> Function(Pointer<Void>)>>(
          'fixture_buffer_data',
        )
        .address
        .toString(),
    'bufferSeal': _fixtureLibrary
        .lookup<NativeFunction<Void Function(Pointer<Void>)>>(
          'fixture_buffer_seal',
        )
        .address
        .toString(),
    'bufferRetain': _fixtureLibrary
        .lookup<NativeFunction<Pointer<Void> Function(Pointer<Void>)>>(
          'fixture_buffer_retain',
        )
        .address
        .toString(),
    'bufferRelease': _fixtureBufferRelease.address.toString(),
    'free': Pointer.fromFunction<Void Function(Pointer<Uint8>, UintPtr)>(
      _free,
    ).address.toString(),
    'release': calloc.nativeFree.address.toString(),
  });

  /// Transfers an immutable external buffer through the actual Dart VM native API.
  void send(
    int kind,
    int id,
    Uint8List payload, [
    List<CoreLinkBufferSpan> buffers = const [],
  ]) {
    final bytes = _fixtureAllocate(9 + payload.length);
    final frame = bytes.asTypedList(9 + payload.length);
    frame[0] = kind;
    ByteData.sublistView(frame).setInt64(1, id, Endian.little);
    frame.setRange(9, frame.length, payload);
    lastPayload = bytes;
    final handles = calloc<Pointer<Void>>(buffers.length + 1);
    try {
      for (var i = 0; i < buffers.length; i++) {
        handles[i] = Pointer<Void>.fromAddress(buffers[i].buffer.token as int);
      }
      expect(
        _fixtureSend(post, port, bytes, frame.length, handles, buffers.length),
        isTrue,
      );
    } finally {
      calloc.free(handles);
    }
  }

  /// Completes one request with a compact success result.
  void reply(int id, Object? value) {
    final message = encodeCoreLinkMessage([0, value]).share(_shareFixture);
    send(0, id, message.bytes, message.buffers);
  }

  /// Emits one event before the watch-open response to exercise buffering.
  void event(int id, String subscription, String value) => send(
    1,
    id,
    encodeCoreLink([
      subscription,
      [null, 'core/test7', 'items', 'Snapshot', value],
    ]),
  );
}

final Map<int, _HostConnection> _connections = {};
int _allocatedBuffers = 0;
int _freedBuffers = 0;

/// Captures the VM's thread-safe post entry point and connection-specific port.
bool _attach(Pointer<Void> token, Pointer<Void> post, int port) {
  final connection = _connections[token.address]!;
  connection.post = post;
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
  Pointer<Pointer<Void>> handles,
  int count,
) {
  final connection = _connections[token.address]!;
  final bytes = Uint8List.fromList(pointer.asTypedList(length));
  final buffers = <CoreLinkSharedBuffer>[];
  for (var i = 0; i < count; i++) {
    final token = _fixtureBufferRetain(handles[i]);
    final view = _fixtureBufferView(token)
        .asTypedList(
          _fixtureBufferLength(token),
          finalizer: _fixtureBufferRelease,
          token: token,
        )
        .asUnmodifiableView();
    buffers.add(
      CoreLinkSharedBuffer(view, token.address, _fixtureBufferRelease.address),
    );
  }
  attachCoreLinkBuffers(bytes, buffers);
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
        connection.reply(id, [
          null,
          'core/test7',
          'items',
          'Snapshot',
          'snapshot',
        ]);
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
      target: 'core/test7',
      methodName: method,
      args: args,
    );

/// Creates a watch whose property selects the test host's stream behavior.
CoreWatchRequest _watch(String property) => CoreWatchRequest(
  requestId: 'watch',
  target: 'core/test7',
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

  setUpAll(_loadPortFixture);

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

  test('VM collection releases delivered shared ownership receipts', () async {
    final messages = ReceivePort();
    final exit = ReceivePort();
    final isolate = await Isolate.spawn(_receiveOneEnvelope, [
      messages.sendPort,
      _fixtureLibrary
          .lookup<NativeFunction<Pointer<Void> Function(Pointer<Void>)>>(
            'fixture_buffer_retain',
          )
          .address,
      _fixtureBufferRelease.address,
    ], onExit: exit.sendPort);
    final port = await messages.first as SendPort;
    messages.close();
    final baseline = _fixtureLiveAllocations();
    final token = _fixtureBufferAllocate(1024 * 1024);
    _fixtureBufferSeal(token);
    final handles = calloc<Pointer<Void>>()..value = token;
    final frame = _fixtureAllocate(9);
    try {
      expect(
        _fixtureSend(
          NativeApi.postCObject.cast(),
          port.nativePort,
          frame,
          9,
          handles,
          1,
        ),
        isTrue,
      );
      _fixtureBufferRelease.asFunction<void Function(Pointer<Void>)>()(token);
      await exit.first;
      for (
        var attempt = 0;
        attempt < 16 && _fixtureLiveAllocations() != baseline;
        attempt++
      ) {
        final pressure = List.generate(
          32,
          (index) => Uint8List(1024 * 1024)..[0] = index,
        );
        await Future<void>.delayed(const Duration(milliseconds: 10));
        expect(pressure.last[0], 31);
      }
      expect(_fixtureLiveAllocations(), baseline);
    } finally {
      calloc.free(handles);
      exit.close();
      isolate.kill();
    }
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
        .setMockMethodCallHandler(const MethodChannel('operit/runtime'), (
          call,
        ) async {
          attempts++;
          if (attempts == 1) {
            throw PlatformException(
              code: 'OPERIT_RUNTIME_ERROR',
              message: 'Runtime and workspace roots are not configured',
            );
          }
          host = _HostConnection();
          _connections[host.token.address] = host;
          return host.descriptor();
        });
    await expectLater(
      proxy.call(_call('before', 'echo', 'before')),
      throwsA(isA<PlatformException>()),
    );
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
    'response views share native storage and outlive the connection',
    () async {
      final source = await transport.shareBytes(Uint8List.fromList([7, 9]));
      final response = await transport.request(
        CoreTransportOperation.call,
        encodeNativeCoreCallRequest(_call('shared', 'echo', source)),
      );
      final value = decodeNativeCoreResult(response) as Uint8List;
      expect(value, [7, 9]);
      expect(() => response[0] = 255, throwsUnsupportedError);
      expect(() => value[0] = 255, throwsUnsupportedError);
      final sourceToken = coreLinkSharedSpan(source)!.buffer.token as int;
      final receivedToken = coreLinkSharedSpan(value)!.buffer.token as int;
      expect(
        _fixtureBufferView(Pointer<Void>.fromAddress(sourceToken)).address,
        _fixtureBufferView(Pointer<Void>.fromAddress(receivedToken)).address,
      );
      await proxy.call(_call('next', 'echo', 'another allocation'));
      await transport.close();
      await pumpEventQueue();
      expect(value, [7, 9]);
      expect(decodeNativeCoreResult(response), [7, 9]);
    },
  );

  test(
    'concurrent nested shared slices preserve addresses and view identity',
    () async {
      final source = await transport.shareBytes(
        Uint8List.fromList([3, 5, 7, 9]),
      );
      final slice = sliceCoreLinkBytes(source, 1, 3);
      final values = await Future.wait(
        List.generate(16, (index) async {
          final result = await transport.request(
            CoreTransportOperation.call,
            encodeNativeCoreCallRequest(
              _call('$index', 'echo', {
                'data': [source, slice],
              }),
            ),
          );
          return decodeNativeCoreResult(result) as Map;
        }),
      );
      final original = coreLinkSharedSpan(source)!;
      for (final value in values) {
        final data = value['data'] as List;
        final full = coreLinkSharedSpan(data[0] as Uint8List)!;
        final part = coreLinkSharedSpan(data[1] as Uint8List)!;
        expect(identical(full.buffer, original.buffer), isTrue);
        expect((data[1] as Uint8List).toList(), [5, 7]);
        expect(
          _fixtureBufferView(
                Pointer<Void>.fromAddress(part.buffer.token as int),
              ).address +
              part.offset,
          _fixtureBufferView(
                Pointer<Void>.fromAddress(original.buffer.token as int),
              ).address +
              1,
        );
      }
    },
  );

  test(
    'push preserves order and releases a failed stream exactly once',
    () async {
      final push = await proxy.push(
        const CorePushRequest(
          requestId: 'push',
          target: 'core/test7',
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

  test('rejected posts release their native allocation immediately', () async {
    await proxy.call(_call('connected', 'echo', null));
    await transport.close();
    final bytes = _fixtureAllocate(64);
    final allocated = _fixtureLiveAllocations();
    expect(_fixtureSend(host.post, host.port, bytes, 64, nullptr, 0), isFalse);
    expect(_fixtureLiveAllocations(), allocated - 1);
  });

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

/// Lets VM isolate teardown exercise finalizers on a received ownership envelope.
void _receiveOneEnvelope(List<Object> arguments) {
  final parent = arguments[0] as SendPort;
  final retain =
      Pointer<
            NativeFunction<Pointer<Void> Function(Pointer<Void>)>
          >.fromAddress(arguments[1] as int)
          .asFunction<Pointer<Void> Function(Pointer<Void>)>();
  final release =
      Pointer<NativeFunction<Void Function(Pointer<Void>)>>.fromAddress(
        arguments[2] as int,
      );
  final port = ReceivePort();
  parent.send(port.sendPort);
  port.listen((dynamic message) {
    final envelope = message as List;
    if (envelope.length != 5 || (envelope[2] as Uint8List).length != 1) {
      throw StateError('Invalid fixture ownership receipt');
    }
    final token = retain(Pointer<Void>.fromAddress(envelope[1] as int));
    final view = Pointer<Uint8>.fromAddress(
      envelope[3] as int,
    ).asTypedList(envelope[4] as int, finalizer: release, token: token);
    if (view.first != 0 || view.last != 0) {
      throw StateError('Invalid adopted binary contents');
    }
    Isolate.exit();
  });
}

/// Publishes one fixture binary value through the same native immutable lifecycle.
CoreLinkBufferSpan _shareFixture(Uint8List bytes) {
  final shared = coreLinkSharedSpan(bytes);
  if (shared != null) return shared;
  final token = _fixtureBufferAllocate(bytes.length);
  final pointer = _fixtureBufferData(token);
  pointer.asTypedList(bytes.length).setAll(0, bytes);
  _fixtureBufferSeal(token);
  final view = pointer
      .asTypedList(bytes.length, finalizer: _fixtureBufferRelease, token: token)
      .asUnmodifiableView();
  return CoreLinkBufferSpan(
    CoreLinkSharedBuffer(view, token.address, _fixtureBufferRelease.address),
    0,
    bytes.length,
  );
}
