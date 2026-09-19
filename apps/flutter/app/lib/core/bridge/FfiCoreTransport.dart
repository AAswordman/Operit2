// ignore_for_file: file_names
import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:flutter/services.dart';

import '../link/CoreLinkCodec.dart';
import 'CoreByteTransport.dart';
import 'FfiCoreAddress.dart';

typedef _AttachNative = Bool Function(Pointer<Void>, Pointer<Void>, Int64);
typedef _Attach = bool Function(Pointer<Void>, Pointer<Void>, int);
typedef _SubmitNative =
    Void Function(Pointer<Void>, Uint32, Int64, Pointer<Uint8>, UintPtr);
typedef _Submit = void Function(Pointer<Void>, int, int, Pointer<Uint8>, int);
typedef _AllocateNative = Pointer<Uint8> Function(UintPtr);
typedef _Allocate = Pointer<Uint8> Function(int);
typedef _FreeNative = Void Function(Pointer<Uint8>, UintPtr);
typedef _Free = void Function(Pointer<Uint8>, int);

class FfiCoreTransport implements CoreByteTransport {
  /// Creates a lazily attached connection for this Dart isolate.
  FfiCoreTransport();

  static final FfiCoreTransport instance = FfiCoreTransport();
  Future<_FfiConnection>? _opening;
  bool _closed = false;

  /// Attaches once to the runtime owned by the registered platform host.
  Future<_FfiConnection> _connection() {
    if (_closed) throw StateError('Core FFI transport is closed');
    return _opening ??= _connect().catchError((Object error, StackTrace stack) {
      _opening = null;
      Error.throwWithStackTrace(error, stack);
    });
  }

  /// Releases this isolate's connection without destroying the platform runtime.
  Future<void> close() async {
    _closed = true;
    final opening = _opening;
    if (opening != null) (await opening).close();
  }

  /// Reads the host function table without selecting a platform or library path.
  Future<_FfiConnection> _connect() async {
    final encoded = await const MethodChannel(
      'operit/runtime',
    ).invokeMethod<String>('connectCoreFfi');
    if (encoded == null) throw StateError('Host returned no FFI descriptor');
    return _FfiConnection(jsonDecode(encoded) as Map<String, dynamic>);
  }

  /// Dispatches directly through the function table after host initialization.
  @override
  Future<Uint8List> request(
    CoreTransportOperation operation,
    Uint8List payload,
  ) async => (await _connection()).request(operation, payload);

  /// Forwards one watch while preserving cancellation of its native subscription.
  @override
  Stream<Uint8List> watch(String subscriptionId, Uint8List payload) async* {
    final connection = await _connection();
    yield* connection.watch(subscriptionId, payload);
  }
}

class _FfiConnection implements Finalizable {
  /// Binds the versioned ABI and registers a VM-owned port for asynchronous replies.
  _FfiConnection(Map<String, dynamic> descriptor) {
    if (descriptor['version'] != 1 || NativeApi.majorVersion != 2) {
      throw StateError('Unsupported Core FFI or Dart native API version');
    }
    _session = Pointer<Void>.fromAddress(_address(descriptor, 'session'));
    _submit = Pointer<NativeFunction<_SubmitNative>>.fromAddress(
      _address(descriptor, 'submit'),
    ).asFunction<_Submit>();
    _allocate = Pointer<NativeFunction<_AllocateNative>>.fromAddress(
      _address(descriptor, 'allocate'),
    ).asFunction<_Allocate>();
    _free = Pointer<NativeFunction<_FreeNative>>.fromAddress(
      _address(descriptor, 'free'),
    ).asFunction<_Free>();
    _finalizer = NativeFinalizer(
      Pointer<NativeFunction<Void Function(Pointer<Void>)>>.fromAddress(
        _address(descriptor, 'release'),
      ),
    );
    _release =
        Pointer<NativeFunction<Void Function(Pointer<Void>)>>.fromAddress(
          _address(descriptor, 'release'),
        ).asFunction<void Function(Pointer<Void>)>();
    _finalizer.attach(this, _session, detach: this);
    final attach = Pointer<NativeFunction<_AttachNative>>.fromAddress(
      _address(descriptor, 'attach'),
    ).asFunction<_Attach>();
    if (!attach(
      _session,
      NativeApi.postCObject.cast(),
      _port.sendPort.nativePort,
    )) {
      close();
      throw StateError('Core FFI connection was already attached');
    }
    _port.listen(_receive);
  }

  /// Decodes one exact native address from the string-based FFI descriptor.
  static int _address(Map<String, dynamic> descriptor, String name) {
    final value = descriptor[name];
    if (value is! String) {
      throw FormatException('FFI descriptor address $name must be a string');
    }
    return decodeCoreFfiAddress(value);
  }

  final ReceivePort _port = ReceivePort('operit-core-ffi');
  late final Pointer<Void> _session;
  late final _Submit _submit;
  late final _Allocate _allocate;
  late final _Free _free;
  late final NativeFinalizer _finalizer;
  late final void Function(Pointer<Void>) _release;
  bool _closed = false;
  final Map<int, Completer<Uint8List>> _pending = {};
  final Map<int, StreamController<Uint8List>> _watches = {};
  int _nextRequest = 1;

  /// Transfers one allocated request buffer to Rust without a second input copy.
  Future<Uint8List> _send(
    int id,
    CoreTransportOperation operation,
    Uint8List payload,
  ) {
    if (_closed) {
      return Future.error(StateError('Core FFI connection is closed'));
    }
    final completer = Completer<Uint8List>();
    _pending[id] = completer;
    final pointer = _allocate(payload.length);
    bool submitted = false;
    try {
      pointer.asTypedList(payload.length).setAll(0, payload);
      _submit(_session, operation.index, id, pointer, payload.length);
      submitted = true;
    } catch (error, stack) {
      _pending.remove(id);
      completer.completeError(error, stack);
    } finally {
      if (!submitted) _free(pointer, payload.length);
    }
    return completer.future;
  }

  /// Assigns a connection-local correlation id to one request.
  Future<Uint8List> request(
    CoreTransportOperation operation,
    Uint8List payload,
  ) => _send(_nextRequest++, operation, payload);

  /// Buffers early watch frames and closes Rust state after open acknowledgement.
  Stream<Uint8List> watch(String subscriptionId, Uint8List payload) {
    final id = _nextRequest++;
    late final StreamController<Uint8List> controller;
    Future<void>? opening;
    bool opened = false;
    bool cancelled = false;
    controller = StreamController<Uint8List>(
      onListen: () {
        _watches[id] = controller;
        opening = () async {
          try {
            final response = await _send(
              id,
              CoreTransportOperation.watchStream,
              payload,
            );
            final actualId = decodeNativeCoreWatchStreamResult(response);
            opened = true;
            if (actualId != subscriptionId) {
              throw StateError('Core FFI watch acknowledgement mismatch');
            }
          } catch (error, stack) {
            _watches.remove(id);
            if (!cancelled && !controller.isClosed) {
              controller.addError(error, stack);
              unawaited(controller.close());
            }
          }
        }();
      },
      onCancel: () async {
        cancelled = true;
        _watches.remove(id);
        await opening;
        if (opened && !_closed) {
          decodeNativeCoreVoidResult(
            await request(
              CoreTransportOperation.closeWatchStream,
              Uint8List.fromList(utf8.encode(subscriptionId)),
            ),
          );
        }
      },
    );
    return controller.stream;
  }

  /// Routes copied VM messages by correlation id without using the platform thread.
  void _receive(dynamic message) {
    try {
      final bytes = message as Uint8List;
      if (bytes.length < 9) {
        throw const FormatException('Truncated Core FFI frame');
      }
      final id = ByteData.sublistView(bytes, 1, 9).getInt64(0, Endian.little);
      final payload = Uint8List.sublistView(bytes, 9);
      switch (bytes[0]) {
        case 0:
          final pending = _pending.remove(id);
          if (pending == null) throw StateError('Unknown Core FFI reply: $id');
          pending.complete(payload);
        case 1:
          _watches[id]?.add(payload);
        case 2:
          final controller = _watches[id];
          if (controller != null) {
            try {
              decodeNativeCoreVoidResult(payload);
              throw const FormatException('Expected Core FFI watch error');
            } catch (error, stack) {
              controller.addError(error, stack);
            }
          }
        case 3:
          final controller = _watches.remove(id);
          if (controller != null) unawaited(controller.close());
        default:
          throw FormatException('Unknown Core FFI frame type: ${bytes[0]}');
      }
    } catch (error, stack) {
      close(error, stack);
    }
  }

  /// Fails pending work and detaches native resources exactly once.
  void close([Object? error, StackTrace? stack]) {
    if (_closed) return;
    _closed = true;
    final failure = error ?? StateError('Core FFI connection is closed');
    _port.close();
    _finalizer.detach(this);
    _release(_session);
    for (final pending in _pending.values) {
      pending.completeError(failure, stack);
    }
    _pending.clear();
    final controllers = _watches.values.toList();
    _watches.clear();
    for (final controller in controllers) {
      controller.addError(failure, stack);
      unawaited(controller.close());
    }
  }
}
