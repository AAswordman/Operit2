import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/FfiCoreProxy.dart';
import 'package:operit2/core/bridge/FfiCoreTransport.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';

/// Builds the production Rust transport with a deterministic test Core.
Future<DynamicLibrary> _loadFixture() async {
  final output = await Directory('build/native_tests').create(recursive: true);
  final manifest = Directory.current.uri
      .resolve('../native/operit-flutter-bridge/tests/ffi-receiver/Cargo.toml')
      .toFilePath();
  final result = await Process.run(
    'cargo',
    [
      'build',
      '--offline',
      '--release',
      '--manifest-path',
      manifest,
      '--target-dir',
      '${output.absolute.path}/cargo',
      '--message-format=json',
    ],
    environment: {...Platform.environment, 'RUSTFLAGS': '-Awarnings'},
  );
  final messages = const LineSplitter()
      .convert(result.stdout as String)
      .map((line) => jsonDecode(line) as Map<String, dynamic>)
      .toList();
  if (result.exitCode != 0) {
    final errors = messages
        .where((entry) => entry['reason'] == 'compiler-message')
        .map((entry) => entry['message']['rendered'])
        .join('\n');
    throw StateError(
      'Native receiver fixture failed: ${result.stderr}\n$errors',
    );
  }
  final artifact = messages.singleWhere(
    (entry) =>
        entry['reason'] == 'compiler-artifact' &&
        entry['target']['name'] == 'operit_ffi_receiver_fixture',
  );
  return DynamicLibrary.open((artifact['filenames'] as List).first as String);
}

/// Checks every byte and the binary ranges returned by the native test source.
void _checkValue(Object? value, int seed, int size) {
  final items = value as List;
  expect(items[0], seed);
  for (final (index, length) in [
    (1, size),
    (2, size),
    (3, size - size ~/ 2),
    (4, 0),
  ]) {
    final bytes = items[index] as Uint8List;
    expect(bytes.length, length);
    expect(bytes.every((byte) => byte == seed % 251), isTrue);
  }
}

/// Allocates and releases short-lived Dart objects to exercise native finalizers during delivery.
Future<void> _memoryPressure() async {
  for (var round = 0; round < 8; round++) {
    final pressure = List.generate(
      8,
      (index) => Uint8List(1024 * 1024)..[0] = index,
    );
    expect(pressure.last[0], 7);
    await Future<void>.delayed(Duration.zero);
  }
}

/// Exercises real Rust replies, stream batches and native ownership without malformed input.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final rounds = int.parse(
    Platform.environment['OPERIT_FFI_STRESS_ROUNDS'] ?? '20',
  );
  late DynamicLibrary library;
  late FfiCoreTransport transport;
  late FfiCoreProxy proxy;
  late int connections;
  late Map<String, dynamic> descriptor;

  setUpAll(() async {
    library = await _loadFixture();
  });
  setUp(() async {
    await ClientLogger.initialize();
    await ClientLogger.clear();
    connections = 0;
    transport = FfiCoreTransport();
    proxy = FfiCoreProxy(transport: transport);
    final connect = library
        .lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
          'ffi_receiver_connect',
        );
    final free = library
        .lookupFunction<
          Void Function(Pointer<Utf8>),
          void Function(Pointer<Utf8>)
        >('ffi_receiver_free_descriptor');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(const MethodChannel('operit/runtime'), (
          call,
        ) async {
          expect(call.method, 'connectCoreFfi');
          connections++;
          final encoded = connect();
          try {
            final text = encoded.toDartString();
            descriptor = jsonDecode(text) as Map<String, dynamic>;
            return text;
          } finally {
            free(encoded);
          }
        });
  });
  tearDown(() async {
    await transport.close();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(const MethodChannel('operit/runtime'), null);
  });

  /// Identifies each operation independently from the transport correlation counter.
  CoreCallRequest call(String id, Object? args, [String method = 'echo']) =>
      CoreCallRequest(
        requestId: id,
        target: 'receiver-fixture',
        methodName: method,
        args: args,
      );

  /// Opens finite and slow streams through the real Rust stream owner.
  CoreWatchRequest watch(
    String id,
    String property,
    int count,
    int seed,
    int size,
  ) => CoreWatchRequest(
    requestId: id,
    target: 'receiver-fixture',
    propertyName: property,
    args: {'count': count, 'seed': seed, 'size': size},
  );

  test(
    'native concurrent replies and shared binary slices survive allocation pressure',
    () async {
      for (var round = 0; round < rounds; round++) {
        final source = await transport.shareBytes(
          Uint8List.fromList(List.generate(8192, (index) => index % 251)),
        );
        final slice = sliceCoreLinkBytes(source, 7, 2055);
        await Future.wait([
          _memoryPressure(),
          ...List.generate(128, (index) async {
            final result =
                await proxy.call(
                      call('$round-$index', [
                        index,
                        source,
                        slice,
                        Uint8List(0),
                      ]),
                    )
                    as List;
            expect(result[0], index);
            expect(result[1], source);
            expect(result[2], slice);
            expect(result[3], isEmpty);
          }),
        ]);
      }
      expect(
        connections,
        1,
        reason: 'A hidden reconnect must fail this reproduction attempt',
      );
      expect(
        await ClientLogger.readText(),
        isNot(contains('Core FFI connection failed')),
      );
    },
    timeout: const Timeout(Duration(minutes: 3)),
  );

  test(
    'native stream batches interleave with replies, cancellation and request failures',
    () async {
      for (var round = 0; round < rounds; round++) {
        await Future.wait([
          _memoryPressure(),
          ...List.generate(16, (index) async {
            final seed = round * 2048 + index * 64;
            var received = 0;
            await for (final event in proxy.watchStream(
              watch(
                'finite-$round-$index',
                'finite',
                64,
                seed,
                index.isEven ? 0 : 257,
              ),
            )) {
              _checkValue(event.value, seed + received, index.isEven ? 0 : 257);
              received++;
            }
            expect(received, 64);
          }),
          ...List.generate(32, (index) async {
            final subscription = proxy
                .watchStream(
                  watch('cancel-$round-$index', 'slow', 64, index, 127),
                )
                .listen((_) {});
            await Future<void>.delayed(Duration.zero);
            await subscription.cancel();
          }),
          ...List.generate(32, (index) async {
            await expectLater(
              proxy.call(call('error-$round-$index', null, 'error')),
              throwsA(isA<CoreLinkError>()),
            );
            expect(await proxy.call(call('alive-$round-$index', index)), index);
          }),
        ]);
      }
      expect(connections, 1, reason: 'No source failure may close the session');
      expect(
        await ClientLogger.readText(),
        isNot(contains('Core FFI connection failed')),
      );
    },
    timeout: const Timeout(Duration(minutes: 3)),
  );

  test(
    'native failed watch reports its opening error before completion',
    () async {
      final reported = await Future.wait(
        List.generate(512, (index) async {
          try {
            await proxy
                .watchStream(watch('failed-$index', 'error', 0, 0, 0))
                .toList();
            return false;
          } on CoreLinkError {
            return true;
          }
        }),
      );
      expect(connections, 1);
      expect(
        await ClientLogger.readText(),
        isNot(contains('Core FFI connection failed')),
      );
      expect(
        reported.where((value) => value).length,
        512,
        reason: 'A stream completion must not suppress an opening failure',
      );
    },
  );

  test(
    'native opening panic preserves its cause and keeps other calls alive',
    () async {
      final errors = await Future.wait(
        List.generate(16, (index) async {
          try {
            await proxy
                .watchStream(watch('panic-$index', 'panic', 0, 0, 0))
                .toList();
            fail('The opening panic must reach its listener');
          } on CoreLinkError catch (error) {
            return error;
          }
        }),
      );
      for (final error in errors) {
        expect(error.code, 'FATAL_CORE_PANIC');
        expect(error.message, 'Watch opening panicked');
      }
      expect(await proxy.call(call('after-panic', 'alive')), 'alive');
      expect(
        connections,
        1,
        reason: 'A source panic must not close the connection',
      );
    },
  );

  test(
    'native failure record survives connection closure and logger reopening',
    () async {
      expect(await proxy.call(call('setup', 'alive')), 'alive');
      final configure = library
          .lookupFunction<
            Void Function(Pointer<Utf8>),
            void Function(Pointer<Utf8>)
          >('ffi_receiver_configure_logs');
      final root = await Directory('build/native_tests').createTemp('ffi-log-');
      final path = root.absolute.path.toNativeUtf8();
      final report =
          Pointer<
                NativeFunction<Void Function(Pointer<Uint8>, UintPtr)>
              >.fromAddress(int.parse(descriptor['reportFailure'] as String))
              .asFunction<void Function(Pointer<Uint8>, int)>();
      const message = 'receiver persistence test: 原始错误\nstackTrace=original';
      final bytes = utf8.encode(message);
      final buffer = calloc<Uint8>(bytes.length);
      try {
        configure(path);
        await transport.close();
        buffer.asTypedList(bytes.length).setAll(0, bytes);
        report(buffer, bytes.length);
        expect(
          connections,
          1,
          reason: 'Logging must not reconnect the closed session',
        );
        final log = File('${root.path}/ffi.log');
        final persisted = await log.readAsString();
        expect(persisted, contains('E/CoreFfiTransport'));
      expect(persisted, contains('pid='));
        expect(persisted, contains('原始错误'));
        expect(persisted, contains('stackTrace=original'));
        configure(path);
        expect(await log.readAsString(), persisted);
      } finally {
        calloc.free(buffer);
        calloc.free(path);
      }
    },
  );
}
