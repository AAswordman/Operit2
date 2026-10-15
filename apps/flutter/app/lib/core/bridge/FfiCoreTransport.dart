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
    Void Function(
      Pointer<Void>,
      Uint32,
      Int64,
      Pointer<Uint8>,
      UintPtr,
      Pointer<Pointer<Void>>,
      UintPtr,
    );
typedef _Submit =
    void Function(
      Pointer<Void>,
      int,
      int,
      Pointer<Uint8>,
      int,
      Pointer<Pointer<Void>>,
      int,
    );
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
    CoreLinkMessage payload,
  ) async => (await _connection()).request(operation, payload);

  /// Places ordinary bytes in host-owned immutable storage for constant-cost reuse.
  Future<Uint8List> shareBytes(Uint8List bytes) async =>
      (await _connection())._retainBuffer(bytes).view();

  /// Forwards one watch while preserving cancellation of its native subscription.
  @override
  Stream<Uint8List> watch(
    String subscriptionId,
    CoreLinkMessage payload,
  ) async* {
    final connection = await _connection();
    yield* connection.watch(subscriptionId, payload);
  }
}

class _FfiConnection implements Finalizable {
  /// Binds the versioned ABI and registers a VM-owned port for asynchronous replies.
  _FfiConnection(Map<String, dynamic> descriptor) {
    if (descriptor['version'] != 2 || NativeApi.majorVersion != 2) {
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
    _bufferAllocate =
        Pointer<NativeFunction<Pointer<Void> Function(UintPtr)>>.fromAddress(
          _address(descriptor, 'bufferAllocate'),
        ).asFunction<Pointer<Void> Function(int)>();
    _bufferData =
        Pointer<
              NativeFunction<Pointer<Uint8> Function(Pointer<Void>)>
            >.fromAddress(_address(descriptor, 'bufferData'))
            .asFunction<Pointer<Uint8> Function(Pointer<Void>)>();
    _bufferSeal =
        Pointer<NativeFunction<Void Function(Pointer<Void>)>>.fromAddress(
          _address(descriptor, 'bufferSeal'),
        ).asFunction<void Function(Pointer<Void>)>();
    _bufferRetain =
        Pointer<
              NativeFunction<Pointer<Void> Function(Pointer<Void>)>
            >.fromAddress(_address(descriptor, 'bufferRetain'))
            .asFunction<Pointer<Void> Function(Pointer<Void>)>();
    _bufferRelease =
        Pointer<NativeFunction<Void Function(Pointer<Void>)>>.fromAddress(
          _address(descriptor, 'bufferRelease'),
        );
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
  late final Pointer<Void> Function(int) _bufferAllocate;
  late final Pointer<Uint8> Function(Pointer<Void>) _bufferData;
  late final void Function(Pointer<Void>) _bufferSeal;
  late final Pointer<Void> Function(Pointer<Void>) _bufferRetain;
  late final Pointer<NativeFunction<Void Function(Pointer<Void>)>>
  _bufferRelease;
  late final NativeFinalizer _finalizer;
  late final void Function(Pointer<Void>) _release;
  bool _closed = false;
  final Map<int, Completer<Uint8List>> _pending = {};
  final Map<int, StreamController<Uint8List>> _watches = {};
  int _nextRequest = 1;
  final _views = <(int, int), WeakReference<CoreLinkSharedBuffer>>{};
  late final _viewFinalizer =
      Finalizer<((int, int), WeakReference<CoreLinkSharedBuffer>)>((entry) {
        if (identical(_views[entry.$1], entry.$2)) {
          _views.remove(entry.$1);
        }
      });

  /// Retains only weak identities so collected buffers do not remain cached.
  CoreLinkSharedBuffer _rememberBuffer(
    Pointer<Uint8> pointer,
    int length,
    Pointer<Void> token,
  ) {
    final bytes = pointer
        .asTypedList(length, finalizer: _bufferRelease, token: token)
        .asUnmodifiableView();
    final buffer = CoreLinkSharedBuffer(
      bytes,
      token.address,
      _bufferRelease.address,
    );
    final key = (pointer.address, length);
    final reference = WeakReference(buffer);
    _views[key] = reference;
    _viewFinalizer.attach(buffer, (key, reference));
    return buffer;
  }

  /// Creates immutable shared ownership with one initial copy from ordinary Dart bytes.
  CoreLinkBufferSpan _retainBuffer(Uint8List source) {
    final shared = coreLinkSharedSpan(source);
    if (shared != null) {
      if (shared.buffer.domain != _bufferRelease.address) {
        throw StateError('Shared buffer belongs to a different host runtime');
      }
      return shared;
    }
    final token = _bufferAllocate(source.length);
    final pointer = _bufferData(token);
    pointer.asTypedList(source.length).setAll(0, source);
    _bufferSeal(token);
    final buffer = _rememberBuffer(pointer, source.length, token);
    return CoreLinkBufferSpan(buffer, 0, source.length);
  }

  /// Fills the final Rust-owned request buffer without a flattened Dart copy.
  Future<Uint8List> _send(
    int id,
    CoreTransportOperation operation,
    CoreLinkMessage payload,
  ) {
    if (_closed) {
      return Future.error(StateError('Core FFI connection is closed'));
    }
    final keepAlive = _FfiMessage(payload.share(_retainBuffer));
    final shared = keepAlive.shared;
    final metadataLength = shared.bytes.length;
    final tableOffset =
        (metadataLength + sizeOf<IntPtr>() - 1) & -sizeOf<IntPtr>();
    final totalLength = tableOffset + shared.buffers.length * sizeOf<IntPtr>();
    final pointer = _allocate(totalLength);
    final completer = Completer<Uint8List>();
    _pending[id] = completer;
    bool submitted = false;
    try {
      pointer.asTypedList(metadataLength).setAll(0, shared.bytes);
      pointer
          .asTypedList(totalLength)
          .fillRange(metadataLength, tableOffset, 0);
      final handles = (pointer + tableOffset).cast<Pointer<Void>>();
      for (var index = 0; index < shared.buffers.length; index++) {
        handles[index] = Pointer<Void>.fromAddress(
          shared.buffers[index].buffer.token as int,
        );
      }
      _submit(
        _session,
        operation.index,
        id,
        pointer,
        metadataLength,
        handles,
        shared.buffers.length,
      );
      submitted = true;
    } catch (error, stack) {
      _pending.remove(id);
      completer.completeError(error, stack);
    } finally {
      if (!submitted) _free(pointer, totalLength);
    }
    return completer.future;
  }

  /// Assigns a connection-local correlation id to one request.
  Future<Uint8List> request(
    CoreTransportOperation operation,
    CoreLinkMessage payload,
  ) => _send(_nextRequest++, operation, payload);

  /// Buffers early watch frames and closes Rust state after open acknowledgement.
  Stream<Uint8List> watch(String subscriptionId, CoreLinkMessage payload) {
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
              CoreLinkMessage.bytes(utf8.encode(subscriptionId)),
            ),
          );
        }
      },
    );
    return controller.stream;
  }

  /// Routes shared byte frames using the existing ABI header and read-only views.
  void _receive(dynamic message) {
    try {
      final keepAlive = _FfiEnvelope(message as List<dynamic>);
      final envelope = keepAlive.values;
      if (envelope.isEmpty || (envelope.length - 1) % 4 != 0) {
        throw const FormatException('Invalid Core FFI shared envelope');
      }
      final bytes = (envelope[0] as Uint8List).asUnmodifiableView();
      final buffers = <CoreLinkSharedBuffer>[];
      for (var index = 1; index < envelope.length; index += 4) {
        final token = envelope[index] as int;
        final receipt = envelope[index + 1] as Uint8List;
        final address = envelope[index + 2] as int;
        final length = envelope[index + 3] as int;
        if (receipt.length != 1 || token == 0 || address == 0 || length < 0) {
          throw const FormatException('Invalid Core FFI buffer receipt');
        }
        final existing = _views[(address, length)]?.target;
        final buffer =
            existing ??
            _rememberBuffer(
              Pointer<Uint8>.fromAddress(address),
              length,
              _bufferRetain(Pointer<Void>.fromAddress(token)),
            );
        buffers.add(buffer);
      }
      attachCoreLinkBuffers(bytes, buffers);
      if (bytes.length < 9) {
        throw const FormatException('Truncated Core FFI frame');
      }
      final kind = bytes[0];
      final id = ByteData.sublistView(bytes, 1, 9).getInt64(0, Endian.little);
      final payload = sliceCoreLinkBytes(bytes, 9);
      switch (kind) {
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
          throw FormatException('Unknown Core FFI frame type: $kind');
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
    _views.clear();
    final controllers = _watches.values.toList();
    _watches.clear();
    for (final controller in controllers) {
      controller.addError(failure, stack);
      unawaited(controller.close());
    }
  }
}

/// Keeps every external byte view alive until synchronous native retention completes.
class _FfiMessage implements Finalizable {
  /// Retains the frame's external allocations throughout the enclosing FFI call.
  _FfiMessage(this.shared);
  final CoreLinkSharedMessage shared;
}

/// Keeps VM-owned receipts alive until synchronous native adoption is complete.
class _FfiEnvelope implements Finalizable {
  /// Retains every receipt throughout this delivery callback.
  _FfiEnvelope(this.values);
  final List<dynamic> values;
}
