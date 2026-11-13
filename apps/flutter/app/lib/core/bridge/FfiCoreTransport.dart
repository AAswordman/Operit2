// ignore_for_file: file_names
import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter/scheduler.dart';

import '../link/CoreLinkCodec.dart';
import '../logging/ClientLogger.dart';
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

  /// Shares one live connection and opens a new one after a failed session closes.
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
    return _FfiConnection(
      jsonDecode(encoded) as Map<String, dynamic>,
      onClosed: () => _opening = null,
    );
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
  _FfiConnection(
    Map<String, dynamic> descriptor, {
    required void Function() onClosed,
  }) : _onClosed = onClosed {
    if (descriptor['version'] != 4 || NativeApi.majorVersion != 2) {
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
    _reportFailure =
        Pointer<
              NativeFunction<Void Function(Pointer<Uint8>, UintPtr)>
            >.fromAddress(_address(descriptor, 'reportFailure'))
            .asFunction<void Function(Pointer<Uint8>, int)>();
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
      throw StateError('Unable to attach Core FFI connection');
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
  final void Function() _onClosed;
  late final Pointer<Void> _session;
  late final _Submit _submit;
  late final _Allocate _allocate;
  late final _Free _free;
  late final void Function(Pointer<Uint8>, int) _reportFailure;
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
  int _receivedBatches = 0;
  int _receivedFrames = 0;
  String _receiveStage = 'idle';
  int _receiveSlot = -1;
  final Stopwatch _receiveClock = Stopwatch()..start();
  int? _lastReceiveMillis;
  int? _receiveGapMillis;
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

  /// Buffers early frames and cancels the exact generation even during source opening.
  Stream<Uint8List> watch(String subscriptionId, CoreLinkMessage payload) {
    final id = _nextRequest++;
    late final StreamController<Uint8List> controller;
    bool openingFailed = false;
    bool cancelled = false;
    controller = StreamController<Uint8List>(
      onListen: () {
        _watches[id] = controller;
        unawaited(() async {
          try {
            final response = await _send(
              id,
              CoreTransportOperation.watchStream,
              payload,
            );
            final actualId = decodeNativeCoreWatchStreamResult(response);
            if (actualId != subscriptionId) {
              throw StateError('Core FFI watch acknowledgement mismatch');
            }
          } catch (error, stack) {
            openingFailed = true;
            _watches.remove(id);
            if (!cancelled && !controller.isClosed) {
              controller.addError(error, stack);
              unawaited(controller.close());
            }
          }
        }());
      },
      onCancel: () async {
        cancelled = true;
        _watches.remove(id);
        if (!openingFailed && !_closed) {
          decodeNativeCoreVoidResult(
            await request(
              CoreTransportOperation.closeWatchStream,
              encodeCoreLinkMessage([subscriptionId, id]),
            ),
          );
        }
      },
    );
    return controller.stream;
  }

  /// Routes one uniformly batched delivery while retaining each frame's own attachments.
  void _receive(dynamic message) {
    var index = -1;
    final now = _receiveClock.elapsedMilliseconds;
    _receiveGapMillis = _lastReceiveMillis == null
        ? null
        : now - _lastReceiveMillis!;
    _lastReceiveMillis = now;
    _receivedBatches++;
    _receiveStage = 'batch';
    _receiveSlot = -1;
    try {
      final frames = message as List<dynamic>;
      if (frames.isEmpty) throw const FormatException('Empty Core FFI batch');
      for (index = 0; index < frames.length; index++) {
        _receiveStage = 'frame';
        _receiveSlot = -1;
        _receiveFrame(frames[index] as List<dynamic>);
        _receivedFrames++;
      }
    } catch (error, stack) {
      close(error, stack, _failureContext(message, index));
    }
  }

  /// Captures bounded object shapes and aliases without traversing payloads or cycles.
  String _failureContext(dynamic message, int index) {
    final context = StringBuffer(
      'diagnostics=ffi-v1 nativePort=${_port.sendPort.nativePort} '
      'batchType=${message.runtimeType} batchIndex=$index '
      'receiveBatch=$_receivedBatches receivedFrames=$_receivedFrames '
      'stage=$_receiveStage failingSlot=$_receiveSlot '
      'connectionAgeMs=${_receiveClock.elapsedMilliseconds} '
      'receiveGapMs=$_receiveGapMillis '
      'nativeApi=${NativeApi.majorVersion}.${NativeApi.minorVersion}',
    );
    try {
      context.write(' lifecycle=${SchedulerBinding.instance.lifecycleState}');
    } catch (_) {
      context.write(' lifecycle=unavailable');
    }
    try {
      if (message is! List) return context.toString();
      context.write(
        ' batchLength=${message.length} batchIdentity=${identityHashCode(message)}',
      );
      if (index < 0 || index >= message.length) return context.toString();
      final frame = message[index];
      context.write(
        ' frameType=${frame.runtimeType} frame=${_diagnosticShape(frame)}',
      );
      // Inspect the bad frame first, so large batches cannot hide the fault in truncated logs.
      if (frame is List) {
        final slots = frame.length < 33 ? frame.length : 33;
        for (var slot = 0; slot < slots; slot++) {
          final value = frame[slot];
          context.write(
            '\nffi_rx_slot index=$index slot=$slot ${_diagnosticShape(value)}',
          );
          if (slot == 0 || slot % 4 == 2) {
            context.write(' ${_diagnosticAliases(message, frame, value)}');
          }
        }
        if (slots < frame.length) {
          context.write('\nffi_rx_slots_omitted=${frame.length - slots}');
          if (_receiveSlot >= slots && _receiveSlot < frame.length) {
            context.write(
              '\nffi_rx_failing_slot slot=$_receiveSlot ${_diagnosticShape(frame[_receiveSlot])}',
            );
          }
        }
      }
      final count = message.length < 128 ? message.length : 128;
      for (var frameIndex = 0; frameIndex < count; frameIndex++) {
        final candidate = message[frameIndex];
        context.write(
          '\nffi_rx_frame index=$frameIndex ${_diagnosticShape(candidate)}',
        );
        if (candidate is List && candidate.isNotEmpty) {
          context.write(' metadata=${_diagnosticShape(candidate[0])}');
        }
      }
      if (count < message.length) {
        context.write('\nffi_rx_frames_omitted=${message.length - count}');
      }
    } catch (diagnosticError) {
      // Diagnostic failures must never replace the original receiver error.
      context.write(
        '\nffi_rx_diagnostic_error type=${diagnosticError.runtimeType}',
      );
    }
    return context.toString();
  }

  /// Logs only byte headers and shallow type samples; never invokes arbitrary toString().
  String _diagnosticShape(dynamic value) {
    if (value == null) return 'type=Null';
    if (value is int) return 'type=int value=$value';
    final result = StringBuffer(
      'type=${value.runtimeType} identity=${identityHashCode(value)}',
    );
    if (value is! List) return result.toString();
    result.write(' length=${value.length}');
    if (value is Uint8List) {
      if (value.length >= 9) {
        result.write(
          ' kind=${value[0]} id=${ByteData.sublistView(value, 1, 9).getInt64(0, Endian.little)}',
        );
      }
      return result.toString();
    }
    final count = value.length < 16 ? value.length : 16;
    var byteValues = 0;
    var lists = 0;
    final types = <String>[];
    for (var i = 0; i < count; i++) {
      final entry = value[i];
      if (entry is int && entry >= 0 && entry <= 255) byteValues++;
      if (entry is List) lists++;
      if (i < 8) {
        types.add(
          '${entry.runtimeType}${entry is List ? "(length=${entry.length})" : ""}',
        );
      }
    }
    result.write(
      ' sampled=$count byteValues=$byteValues lists=$lists types=[${types.join(",")}]',
    );
    // A list of integers may be an alternate byte representation. Decode only its header.
    if (count >= 9 && byteValues == count) {
      final header = Uint8List.fromList(
        List<int>.generate(9, (i) => value[i] as int),
      );
      result.write(
        ' possibleKind=${header[0]} possibleId=${ByteData.sublistView(header, 1, 9).getInt64(0, Endian.little)}',
      );
    }
    return result.toString();
  }

  /// Compares identities with a bounded scan; self-referential arrays cannot recurse.
  String _diagnosticAliases(List batch, List frame, dynamic value) {
    if (value is! List) return 'aliases=not-list';
    final aliases = <String>[];
    if (identical(value, batch)) aliases.add('batch');
    if (identical(value, frame)) aliases.add('currentFrame');
    var scanned = 0;
    final count = batch.length < 128 ? batch.length : 128;
    for (var i = 0; i < count && scanned < 2048 && aliases.length < 16; i++) {
      final candidate = batch[i];
      scanned++;
      if (identical(value, candidate)) aliases.add('frame[$i]');
      if (candidate is! List) continue;
      final slots = candidate.length < 33 ? candidate.length : 33;
      for (var j = 0; j < slots && scanned < 2048 && aliases.length < 16; j++) {
        scanned++;
        if (identical(value, candidate[j])) aliases.add('frame[$i].slot[$j]');
      }
    }
    return 'aliases=[${aliases.join(",")}] aliasScan=$scanned aliasFrameLimit=$count';
  }

  /// Adopts one frame's receipts and routes its connection-local correlation id.
  void _receiveFrame(List<dynamic> message) {
    final keepAlive = _FfiEnvelope(message);
    final envelope = keepAlive.values;
    if (envelope.isEmpty || (envelope.length - 1) % 4 != 0) {
      throw const FormatException('Invalid Core FFI shared envelope');
    }
    _receiveStage = 'metadata';
    _receiveSlot = 0;
    final bytes = (envelope[0] as Uint8List).asUnmodifiableView();
    final buffers = <CoreLinkSharedBuffer>[];
    for (var index = 1; index < envelope.length; index += 4) {
      _receiveStage = 'receipt';
      _receiveSlot = index;
      final token = envelope[index] as int;
      _receiveSlot = index + 1;
      final receipt = envelope[index + 1] as Uint8List;
      _receiveSlot = index + 2;
      final address = envelope[index + 2] as int;
      _receiveSlot = index + 3;
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
    _receiveStage = 'decode';
    _receiveSlot = -1;
    attachCoreLinkBuffers(bytes, buffers);
    if (bytes.length < 9) {
      throw const FormatException('Truncated Core FFI frame');
    }
    final kind = bytes[0];
    final id = ByteData.sublistView(bytes, 1, 9).getInt64(0, Endian.little);
    final payload = sliceCoreLinkBytes(bytes, 9);
    _receiveStage = 'dispatch';
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
      case 4:
        decodeNativeCoreVoidResult(payload);
        throw const FormatException('Expected terminal Core connection error');
      default:
        throw FormatException('Unknown Core FFI frame type: $kind');
    }
  }

  /// Fails pending work and detaches native resources exactly once.
  void close([Object? error, StackTrace? stack, String? context]) {
    if (_closed) return;
    _closed = true;
    if (error != null) {
      final bytes = utf8.encode(
        'Core FFI connection failed pendingRequests=${_pending.length} '
        'activeWatches=${_watches.length} '
        'at=${DateTime.now().toIso8601String()} session=${_session.address} '
        'nextRequest=$_nextRequest pendingIds=${_pending.keys.take(8).toList()} '
        '$context\n$error\nstackTrace=$stack',
      );
      final pointer = _allocate(bytes.length);
      try {
        pointer.asTypedList(bytes.length).setAll(0, bytes);
        _reportFailure(pointer, bytes.length);
      } finally {
        _free(pointer, bytes.length);
      }
    }
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
    _onClosed();
    if (error != null && ClientLogger.isInitialized) {
      ClientLogger.e(
        'Core FFI connection failed $context',
        tag: 'CoreFfiTransport',
        error: error,
        stackTrace: stack,
      );
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
