// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';

import 'CoreLinkProtocol.dart';

typedef CoreEmbeddedStreamFactory =
    Stream<T> Function<T>(
      String streamId,
      String target,
      String propertyName,
      Object? args,
      T Function(CoreLinkValueReader reader) decode,
    );

/// Encodes one Link value using the protocol's only MessagePack representation.
Uint8List encodeCoreLink(Object? value) {
  final writer = _CoreLinkMessagePackWriter();
  writer.writeValue(value);
  return writer.takeBytes();
}

/// Prepares one value for a host carrier without flattening its binary contents.
CoreLinkMessage encodeCoreLinkMessage(Object? value) {
  final writer = _CoreLinkMessagePackWriter();
  writer.writeValue(value);
  return writer.takeMessage();
}

/// Decodes one complete MessagePack Link value.
T decodeCoreLink<T>(
  Uint8List bytes, {
  T Function(CoreLinkValueReader reader)? decode,
  String? target,
  CoreEmbeddedStreamFactory? embeddedStreamFactory,
}) {
  final reader = _CoreLinkMessagePackReader(
    bytes,
    target: target,
    embeddedStreamFactory: embeddedStreamFactory,
  );
  final value = decode == null ? reader.readValue() as T : decode(reader);
  reader.expectDone();
  return value;
}

/// Decodes independent full values emitted by a Core Stream protocol.
///
/// Flow and State protocols use the incremental decoders below instead.
class CoreLinkStreamEventValueDecoder {
  /// Passes the original payload directly to the typed decoder without a base.
  T decode<T>(CoreEvent event, {required T Function(Uint8List bytes) decode}) {
    if (event.kind != 'Snapshot' && event.kind != 'Changed') {
      throw FormatException(
        'Unsupported Core stream event kind: ${event.kind}',
      );
    }
    final bytes = event.valueBytes;
    if (bytes == null) {
      throw StateError('Core stream event has no payload bytes');
    }
    return _decodeCoreLinkEventBytes(
      bytes,
      decode,
      stage: 'dart-stream-messagepack',
    );
  }
}

/// Measures typed watch decoding independently of the value or stream protocol.
T _decodeCoreLinkEventBytes<T>(
  Uint8List bytes,
  T Function(Uint8List bytes) decode, {
  String stage = 'dart-messagepack',
}) {
  const profile = bool.fromEnvironment('OPERIT_PROFILE_COMPOSE_STAGES');
  final watch = profile ? (Stopwatch()..start()) : null;
  final result = decode(bytes);
  if (watch != null) {
    _profileCoreLinkDecode(stage, watch, bytes.length);
  }
  return result;
}

/// Reconstructs complete typed watch payloads from ordered Core events.
class CoreLinkEventValueDecoder {
  Object? _previousValue;
  var _hasPreviousValue = false;

  /// Decodes one watch event after applying protocol-level incremental payloads.
  T decode<T>(CoreEvent event, {required T Function(Uint8List bytes) decode}) {
    const profile = bool.fromEnvironment('OPERIT_PROFILE_COMPOSE_STAGES');
    final bytes = completeValueBytes(event, profileStages: profile);
    return _decodeCoreLinkEventBytes(bytes, decode);
  }

  /// Decodes one complete event with the typed Link reader API.
  ///
  /// Kept for callers that predate the byte-oriented [decode] callback.
  T decodeValue<T>(
    CoreEvent event, {
    required T Function(CoreLinkValueReader reader) decode,
    CoreEmbeddedStreamFactory? embeddedStreamFactory,
  }) {
    return decodeCoreLink<T>(
      completeValueBytes(event),
      decode: decode,
      target: event.target,
      embeddedStreamFactory: embeddedStreamFactory,
    );
  }

  /// Returns complete MessagePack bytes for one ordered Core watch event.
  Uint8List completeValueBytes(CoreEvent event, {bool profileStages = false}) {
    final watch = profileStages ? (Stopwatch()..start()) : null;
    final currentValue = _readEventValue(event);
    if (watch != null) {
      _profileCoreLinkDecode(
        'dart-watch-read',
        watch,
        event.valueBytes!.length,
      );
      watch.reset();
    }
    final completeValue = switch (event.kind) {
      'Snapshot' || 'Changed' => _replacePreviousValue(currentValue),
      'Delta' => _applyDeltaEvent(currentValue),
      _ => throw FormatException(
        'Unsupported Core watch event kind: ${event.kind}',
      ),
    };
    if (watch != null) {
      _profileCoreLinkDecode(
        'dart-watch-retain',
        watch,
        event.valueBytes!.length,
      );
      watch.reset();
    }
    final bytes = event.kind == 'Delta'
        ? _encodeRetainedCoreLink(completeValue)
        : event.valueBytes!;
    if (watch != null && event.kind == 'Delta') {
      _profileCoreLinkDecode('dart-watch-reencode', watch, bytes.length);
    }
    return bytes;
  }

  /// Reads one event payload as an untyped Link value.
  Object? _readEventValue(CoreEvent event) {
    final valueBytes = event.valueBytes;
    if (valueBytes == null) {
      throw StateError('Core watch event has no payload bytes');
    }
    return decodeCoreLink<Object?>(valueBytes);
  }

  /// Stores a complete event payload as the new incremental base value.
  Object? _replacePreviousValue(Object? value) {
    _previousValue = _copyCoreLinkValue(value);
    _hasPreviousValue = true;
    return _previousValue;
  }

  /// Applies one protocol delta to the last complete event payload.
  Object? _applyDeltaEvent(Object? delta) {
    if (!_hasPreviousValue) {
      throw StateError('Core watch delta arrived before a full value');
    }
    _previousValue = _applyCoreLinkDelta(_previousValue, delta);
    return _previousValue;
  }
}

/// Retains individual wire values so unchanged list elements need no re-encoding.
class _CoreLinkListEntry<T> {
  /// Retains a decoded item and its original MessagePack bytes.
  _CoreLinkListEntry.decoded(this.bytes, this.value);

  /// Holds a freshly decoded item until the current delta has been fully applied.
  _CoreLinkListEntry.pending(this.pendingValue);

  Uint8List? bytes;
  late T value;
  Object? pendingValue;
  bool dirty = false;
}

/// Decodes list watch deltas without re-encoding or decoding unchanged elements.
class CoreLinkListEventDecoder<T> {
  List<_CoreLinkListEntry<T>>? _entries;

  /// Applies one ordered watch event and returns a complete typed list.
  List<T> decode(
    CoreEvent event, {
    required T Function(CoreLinkValueReader reader) decodeItem,
    CoreEmbeddedStreamFactory? embeddedStreamFactory,
  }) {
    final valueBytes = event.valueBytes;
    if (valueBytes == null) {
      throw StateError('Core watch event has no payload bytes');
    }
    final entries = switch (event.kind) {
      'Snapshot' || 'Changed' => _readList(
        valueBytes,
        event.target,
        decodeItem,
        embeddedStreamFactory,
      ),
      'Delta' => _applyListDelta(valueBytes),
      _ => throw FormatException(
        'Unsupported Core watch event kind: ${event.kind}',
      ),
    };
    for (final entry in entries) {
      if (entry.dirty || entry.bytes == null) {
        final bytes = _encodeRetainedCoreLink(entry.pendingValue);
        entry.value = decodeCoreLink<T>(
          bytes,
          decode: decodeItem,
          target: event.target,
          embeddedStreamFactory: embeddedStreamFactory,
        );
        entry.bytes = bytes;
        entry.pendingValue = null;
        entry.dirty = false;
      }
    }
    _entries = entries;
    return entries.map((entry) => entry.value).toList(growable: false);
  }

  /// Reads a complete MessagePack array while retaining each element's wire slice.
  List<_CoreLinkListEntry<T>> _readList(
    Uint8List bytes,
    String target,
    T Function(CoreLinkValueReader reader) decodeItem,
    CoreEmbeddedStreamFactory? embeddedStreamFactory,
  ) {
    return decodeCoreLink<List<_CoreLinkListEntry<T>>>(
      bytes,
      target: target,
      decode: (reader) {
        final length = reader.readArrayLength();
        return List<_CoreLinkListEntry<T>>.generate(length, (_) {
          final itemBytes = reader.readValueBytes();
          return _CoreLinkListEntry<T>.decoded(
            itemBytes,
            decodeCoreLink<T>(
              itemBytes,
              decode: decodeItem,
              target: target,
              embeddedStreamFactory: embeddedStreamFactory,
            ),
          );
        }, growable: false);
      },
    );
  }

  /// Applies list-indexed operations to a shallow copy of the retained entries.
  List<_CoreLinkListEntry<T>> _applyListDelta(Uint8List bytes) {
    final previous = _entries;
    if (previous == null) {
      throw StateError('Core watch delta arrived before a full value');
    }
    final delta = _coreLinkMap(
      decodeCoreLink<Object?>(bytes),
      'incremental delta',
    );
    final operations = delta[_coreDeltaMarker];
    if (operations is! List) {
      throw const FormatException('Core incremental delta marker is missing');
    }
    var entries = List<_CoreLinkListEntry<T>>.of(previous);
    for (final operation in operations) {
      final fields = _coreLinkMap(operation, 'incremental operation');
      final name = fields['op'];
      final path = fields['path'];
      if (name is! String) {
        throw const FormatException(
          'Core incremental operation name is missing',
        );
      }
      if (path is! List) {
        throw const FormatException(
          'Core incremental operation path is missing',
        );
      }
      if (name == 'set' && !fields.containsKey('value')) {
        throw const FormatException(
          'Core incremental set operation value is missing',
        );
      }
      if (path.isEmpty) {
        if (name != 'set') {
          throw const FormatException(
            'Core incremental root remove is invalid',
          );
        }
        final root = fields['value'];
        if (root is! List) {
          throw const FormatException(
            'Core incremental list root must be an array',
          );
        }
        entries = root
            .map((item) => _CoreLinkListEntry<T>.pending(item))
            .toList();
        continue;
      }
      final index = path.first;
      if (index is! int) {
        throw const FormatException(
          'Core incremental list path segment must be an integer',
        );
      }
      if (path.length == 1) {
        if (name == 'set') {
          final entry = _CoreLinkListEntry<T>.pending(fields['value']);
          if (index == entries.length) {
            entries.add(entry);
          } else if (index >= 0 && index < entries.length) {
            entries[index] = entry;
          } else {
            throw FormatException(
              'Core incremental list append index is invalid: $index',
            );
          }
        } else if (name == 'remove') {
          if (index < 0 || index >= entries.length) {
            throw FormatException(
              'Core incremental list removal index is invalid: $index',
            );
          }
          entries.removeAt(index);
        } else {
          throw FormatException(
            'Unsupported Core incremental operation: $name',
          );
        }
        continue;
      }
      if (index < 0 || index >= entries.length) {
        throw FormatException(
          'Core incremental list path does not exist: $index',
        );
      }
      var entry = entries[index];
      if (!entry.dirty && entry.bytes != null) {
        entry = _CoreLinkListEntry<T>.pending(
          decodeCoreLink<Object?>(entry.bytes!),
        );
        entry.dirty = true;
        entries[index] = entry;
      }
      entry.pendingValue = _applyCoreDeltaOperation(entry.pendingValue, {
        'op': name,
        'path': path.sublist(1),
        if (name == 'set') 'value': fields['value'],
      });
    }
    return entries;
  }
}

const _coreDeltaMarker = r'$coreDelta';

/// Creates a mutable copy of a decoded Link value tree.
Object? _copyCoreLinkValue(Object? value) {
  if (value is Uint8List) {
    if (coreLinkSharedSpan(value) != null) return value;
    return Uint8List.fromList(value);
  }
  if (value is List) {
    return value.map(_copyCoreLinkValue).toList(growable: true);
  }
  if (value is Map) {
    final result = <String, Object?>{};
    for (final entry in value.entries) {
      final key = entry.key;
      if (key is! String) {
        throw FormatException(
          'Link map key must be a string: ${key.runtimeType}',
        );
      }
      result[key] = _copyCoreLinkValue(entry.value);
    }
    return result;
  }
  return value;
}

/// Applies one encoded Core value delta to a complete base value.
Object? _applyCoreLinkDelta(Object? base, Object? delta) {
  final deltaFields = _coreLinkMap(delta, 'incremental delta');
  final operations = deltaFields[_coreDeltaMarker];
  if (operations is! List) {
    throw const FormatException('Core incremental delta marker is missing');
  }
  var result = _copyCoreLinkValue(base);
  for (final operation in operations) {
    result = _applyCoreDeltaOperation(result, operation);
  }
  return result;
}

/// Applies one set or remove operation to a mutable Link value tree.
Object? _applyCoreDeltaOperation(Object? target, Object? operation) {
  final fields = _coreLinkMap(operation, 'incremental operation');
  final operationName = fields['op'];
  final path = fields['path'];
  if (operationName is! String) {
    throw const FormatException('Core incremental operation name is missing');
  }
  if (path is! List) {
    throw const FormatException('Core incremental operation path is missing');
  }
  switch (operationName) {
    case 'set':
      if (!fields.containsKey('value')) {
        throw const FormatException(
          'Core incremental set operation value is missing',
        );
      }
      return _setCoreLinkValueAtPath(
        target,
        path,
        _copyCoreLinkValue(fields['value']),
      );
    case 'remove':
      _removeCoreLinkValueAtPath(target, path);
      return target;
    default:
      throw FormatException(
        'Unsupported Core incremental operation: $operationName',
      );
  }
}

/// Replaces or appends one value at a typed map/list path.
Object? _setCoreLinkValueAtPath(
  Object? target,
  List<Object?> path,
  Object? value,
) {
  if (path.isEmpty) {
    return value;
  }
  final segment = path.first;
  if (target is Map<String, Object?>) {
    if (segment is! String) {
      throw const FormatException(
        'Core incremental map path segment must be a string',
      );
    }
    if (path.length == 1) {
      target[segment] = value;
      return target;
    }
    final child = target[segment];
    if (child == null && !target.containsKey(segment)) {
      throw FormatException(
        'Core incremental map path does not exist: $segment',
      );
    }
    target[segment] = _setCoreLinkValueAtPath(child, path.sublist(1), value);
    return target;
  }
  if (target is List<Object?>) {
    if (segment is! int) {
      throw const FormatException(
        'Core incremental list path segment must be an integer',
      );
    }
    if (path.length == 1) {
      if (segment == target.length) {
        target.add(value);
        return target;
      }
      if (segment >= 0 && segment < target.length) {
        target[segment] = value;
        return target;
      }
      throw FormatException(
        'Core incremental list append index is invalid: $segment',
      );
    }
    if (segment < 0 || segment >= target.length) {
      throw FormatException(
        'Core incremental list path does not exist: $segment',
      );
    }
    target[segment] = _setCoreLinkValueAtPath(
      target[segment],
      path.sublist(1),
      value,
    );
    return target;
  }
  throw const FormatException('Core incremental path traverses a scalar value');
}

/// Removes one value at a typed map/list path.
void _removeCoreLinkValueAtPath(Object? target, List<Object?> path) {
  if (path.isEmpty) {
    throw const FormatException('Core incremental root remove is invalid');
  }
  final segment = path.first;
  if (target is Map<String, Object?>) {
    if (segment is! String) {
      throw const FormatException(
        'Core incremental map path segment must be a string',
      );
    }
    if (path.length == 1) {
      if (!target.containsKey(segment)) {
        throw FormatException(
          'Core incremental map removal path does not exist: $segment',
        );
      }
      target.remove(segment);
      return;
    }
    final child = target[segment];
    if (child == null && !target.containsKey(segment)) {
      throw FormatException(
        'Core incremental map path does not exist: $segment',
      );
    }
    _removeCoreLinkValueAtPath(child, path.sublist(1));
    return;
  }
  if (target is List<Object?>) {
    if (segment is! int) {
      throw const FormatException(
        'Core incremental list path segment must be an integer',
      );
    }
    if (path.length == 1) {
      if (segment >= 0 && segment < target.length) {
        target.removeAt(segment);
        return;
      }
      throw FormatException(
        'Core incremental list removal index is invalid: $segment',
      );
    }
    if (segment < 0 || segment >= target.length) {
      throw FormatException(
        'Core incremental list path does not exist: $segment',
      );
    }
    _removeCoreLinkValueAtPath(target[segment], path.sublist(1));
    return;
  }
  throw const FormatException('Core incremental path traverses a scalar value');
}

/// Reads a decoded Link value as a string-keyed map.
Map<String, Object?> _coreLinkMap(Object? value, String label) {
  if (value is Map<String, Object?>) {
    return value;
  }
  if (value is Map) {
    final result = <String, Object?>{};
    for (final entry in value.entries) {
      final key = entry.key;
      if (key is! String) {
        throw FormatException(
          'Core $label map key must be a string: ${key.runtimeType}',
        );
      }
      result[key] = entry.value;
    }
    return result;
  }
  throw FormatException('Core $label must be a map');
}

/// Encodes a CoreProxy call using the compact native bridge tuple format.
CoreLinkMessage encodeNativeCoreCallRequest(CoreCallRequest request) {
  final writer = _CoreLinkMessagePackWriter();
  writer.writeArrayHeader(4);
  writer.writeValue(request.requestId);
  _writeNativeCorePath(writer, request.target);
  writer.writeValue(request.methodName);
  writer.writeValue(request.args);
  return writer.takeMessage();
}

/// Encodes a CoreProxy push-open request using the compact native tuple format.
CoreLinkMessage encodeNativeCorePushOpenRequest(CorePushRequest request) {
  final writer = _CoreLinkMessagePackWriter();
  writer.writeArrayHeader(4);
  writer.writeValue(request.requestId);
  _writeNativeCorePath(writer, request.target);
  writer.writeValue(request.methodName);
  writer.writeValue(request.args);
  return writer.takeMessage();
}

/// Encodes one ordered CoreProxy push item using the compact native tuple format.
CoreLinkMessage encodeNativeCorePushItem(
  String pushId,
  int sequence,
  Object? args,
) {
  final writer = _CoreLinkMessagePackWriter();
  writer.writeArrayHeader(3);
  writer.writeValue(pushId);
  writer.writeValue(sequence);
  writer.writeValue(args);
  return writer.takeMessage();
}

/// Encodes a CoreProxy watch snapshot request using the compact native tuple format.
CoreLinkMessage encodeNativeCoreWatchSnapshotRequest(CoreWatchRequest request) {
  final writer = _CoreLinkMessagePackWriter();
  writer.writeArrayHeader(4);
  writer.writeValue(request.requestId);
  _writeNativeCorePath(writer, request.target);
  writer.writeValue(request.propertyName);
  writer.writeValue(request.args);
  return writer.takeMessage();
}

/// Encodes a CoreProxy watch stream open request using the compact native tuple format.
CoreLinkMessage encodeNativeCoreWatchStreamRequest(
  String subscriptionId,
  CoreWatchRequest request,
) {
  final writer = _CoreLinkMessagePackWriter();
  writer.writeArrayHeader(5);
  writer.writeValue(subscriptionId);
  writer.writeValue(request.requestId);
  _writeNativeCorePath(writer, request.target);
  writer.writeValue(request.propertyName);
  writer.writeValue(request.args);
  return writer.takeMessage();
}

/// Writes one Core object identity as its compact native path field.
void _writeNativeCorePath(_CoreLinkMessagePackWriter writer, String objectId) {
  writer.writeValue(objectId);
}

/// Decodes a compact native bridge result and returns its successful value.
T decodeNativeCoreResult<T>(
  Uint8List bytes, {
  T Function(CoreLinkValueReader reader)? decode,
  String? target,
  CoreEmbeddedStreamFactory? embeddedStreamFactory,
}) {
  final reader = _CoreLinkMessagePackReader(
    bytes,
    target: target,
    embeddedStreamFactory: embeddedStreamFactory,
  );
  final value = _readNativeCoreResult(
    reader,
    () => decode == null ? reader.readValue() as T : decode(reader),
  );
  reader.expectDone();
  return value;
}

/// Decodes one compact native bridge push-open result.
String decodeNativeCorePushOpenResult(Uint8List bytes) {
  return _decodeNativeCoreStringResult(bytes, 'push open');
}

/// Decodes one compact native bridge watch stream open result.
String decodeNativeCoreWatchStreamResult(Uint8List bytes) {
  return _decodeNativeCoreStringResult(bytes, 'watch stream open');
}

/// Decodes one compact native bridge watch snapshot result.
CoreEvent decodeNativeCoreWatchSnapshotResult(Uint8List bytes) {
  final reader = _CoreLinkMessagePackReader(bytes);
  final event = _readNativeCoreResult(
    reader,
    () => _readNativeCoreEvent(reader),
  );
  reader.expectDone();
  return event;
}

/// Decodes one compact native bridge acknowledgement result.
void decodeNativeCoreVoidResult(Uint8List bytes) {
  final value = decodeNativeCoreResult(bytes);
  if (value != null) {
    throw FormatException('Native core acknowledgement value must be null');
  }
}

/// Decodes one compact native bridge watch channel event frame.
NativeCoreWatchFrame decodeNativeCoreWatchFrame(Uint8List bytes) {
  final reader = _CoreLinkMessagePackReader(bytes);
  final itemCount = reader.readArrayLength();
  if (itemCount == 4) {
    final status = reader.readValue();
    final subscriptionId = reader.readValue();
    final code = reader.readValue();
    final message = reader.readValue();
    reader.expectDone();
    if (status != 1 ||
        subscriptionId is! String ||
        code is! String ||
        message is! String) {
      throw const FormatException(
        'Native core watch error frame fields have invalid types',
      );
    }
    throw CoreLinkError(code: code, message: message);
  }
  if (itemCount != 2) {
    throw FormatException(
      'Native core watch event must contain two values, got $itemCount',
    );
  }
  final subscriptionId = reader.readValue();
  if (subscriptionId is! String) {
    throw FormatException('Native core watch subscription id must be a string');
  }
  final event = _readNativeCoreEvent(reader);
  reader.expectDone();
  return NativeCoreWatchFrame(subscriptionId: subscriptionId, event: event);
}

/// Decodes one compact native bridge string result.
String _decodeNativeCoreStringResult(Uint8List bytes, String operation) {
  final value = decodeNativeCoreResult(bytes);
  if (value is! String) {
    throw FormatException('Native core $operation result must be a string');
  }
  return value;
}

/// Reads one native bridge result header and dispatches its success payload reader.
T _readNativeCoreResult<T>(
  _CoreLinkMessagePackReader reader,
  T Function() readSuccess,
) {
  final itemCount = reader.readArrayLength();
  final status = reader.readValue();
  if (status == 0) {
    if (itemCount != 2) {
      throw FormatException(
        'Native core success response must contain two values, got $itemCount',
      );
    }
    return readSuccess();
  }
  if (status == 1) {
    if (itemCount != 6) {
      throw FormatException(
        'Native core error response must contain six values, got $itemCount',
      );
    }
    final code = reader.readValue();
    final message = reader.readValue();
    final details = reader.readValue();
    final location = _readNativeCoreErrorLocation(reader);
    final backtrace = reader.readValue();
    if (code is! String || message is! String || backtrace is! String?) {
      throw FormatException('Native core error fields have invalid types');
    }
    reader.expectDone();
    throw CoreLinkError(
      code: code,
      message: message,
      details: details,
      location: location,
      backtrace: backtrace,
    );
  }
  throw FormatException('Native core response status is invalid: $status');
}

/// Reads one compact native CoreProxy event tuple.
CoreEvent _readNativeCoreEvent(_CoreLinkMessagePackReader reader) {
  final itemCount = reader.readArrayLength();
  if (itemCount != 5) {
    throw FormatException(
      'Native core event must contain five values, got $itemCount',
    );
  }
  final requestId = reader.readValue();
  if (requestId is! String?) {
    throw FormatException(
      'Native core event request id must be a string or null',
    );
  }
  final target = _readNativeCorePath(reader);
  final propertyName = reader.readString();
  final kind = reader.readString();
  final valueBytes = reader.readValueBytes();
  return CoreEvent.raw(
    requestId: requestId,
    target: target,
    propertyName: propertyName,
    kind: kind,
    valueBytes: valueBytes,
    decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
  );
}

/// Reads one compact native Core object identity.
String _readNativeCorePath(_CoreLinkMessagePackReader reader) {
  final objectId = reader.readValue();
  if (objectId is! String) {
    throw FormatException('Native core target must be a string');
  }
  return objectId;
}

/// Decodes one fixed embedded-stream pool object address.
String _readCoreTargetValue(Object? value) {
  if (value is! String) {
    throw FormatException(
      'Embedded Core stream target must be a string address',
    );
  }
  return value;
}

/// Reads one compact native CoreProxy error location.
CoreLinkErrorLocation? _readNativeCoreErrorLocation(
  _CoreLinkMessagePackReader reader,
) {
  final itemCount = reader.readArrayLengthOrNull();
  if (itemCount == null) {
    return null;
  }
  if (itemCount != 3) {
    throw FormatException(
      'Native core error location must contain three values, got $itemCount',
    );
  }
  final file = reader.readValue();
  final line = reader.readValue();
  final column = reader.readValue();
  if (file is! String || line is! int || column is! int) {
    throw FormatException('Native core error location has invalid fields');
  }
  return CoreLinkErrorLocation(file: file, line: line, column: column);
}

/// Represents one compact native bridge watch channel event.
class NativeCoreWatchFrame {
  const NativeCoreWatchFrame({
    required this.subscriptionId,
    required this.event,
  });

  final String subscriptionId;
  final CoreEvent event;
}

/// Retains a host-owned immutable binary allocation and its opaque ownership token.
class CoreLinkSharedBuffer {
  /// Binds a host allocation without exposing host-specific APIs to the codec.
  CoreLinkSharedBuffer(this.bytes, this.token, this.domain) {
    _sharedByteSpans[bytes] = CoreLinkBufferSpan(this, 0, bytes.length);
  }
  final Uint8List bytes;
  final Object token;
  final Object domain;
}

/// Identifies an immutable range while retaining its complete backing allocation.
class CoreLinkBufferSpan {
  /// Describes one validated range of a shared allocation.
  CoreLinkBufferSpan(this.buffer, this.offset, this.length);
  final CoreLinkSharedBuffer buffer;
  final int offset;
  final int length;

  /// Creates a byte view whose ownership and range survive a subsequent send.
  Uint8List view() {
    final bytes = Uint8List.sublistView(buffer.bytes, offset, offset + length);
    _sharedByteSpans[bytes] = this;
    return bytes;
  }
}

final _sharedByteSpans = Expando<CoreLinkBufferSpan>('Core shared bytes');
final _sharedDecodeBuffers = Expando<List<CoreLinkSharedBuffer>>(
  'Core frame buffers',
);

/// Finds the explicit ownership retained by an external byte view.
CoreLinkBufferSpan? coreLinkSharedSpan(Uint8List bytes) =>
    _sharedByteSpans[bytes];

/// Retains a frame's shared allocations for every later typed decode.
void attachCoreLinkBuffers(
  Uint8List bytes,
  List<CoreLinkSharedBuffer> buffers,
) {
  _sharedDecodeBuffers[bytes] = buffers;
}

/// Slices bytes while preserving both payload ownership and decode context.
Uint8List sliceCoreLinkBytes(Uint8List bytes, int start, [int? end]) {
  final stop = end ?? bytes.length;
  final span = _sharedByteSpans[bytes];
  final view = Uint8List.sublistView(bytes, start, stop);
  if (span != null) {
    _sharedByteSpans[view] = CoreLinkBufferSpan(
      span.buffer,
      span.offset + start,
      stop - start,
    );
  }
  final buffers = _sharedDecodeBuffers[bytes];
  if (buffers != null) _sharedDecodeBuffers[view] = buffers;
  return view;
}

/// Separates one control message from its retained binary allocations.
class CoreLinkSharedMessage {
  /// Retains every buffer until the transport has taken its native references.
  CoreLinkSharedMessage(this.bytes, this.buffers);
  final Uint8List bytes;
  final List<CoreLinkBufferSpan> buffers;
}

/// Marks a binary value that has not been copied into an intermediate wire buffer.
class _CoreLinkBinary {
  /// Retains the source until its carrier materializes or shares it.
  _CoreLinkBinary(this.bytes);
  final Uint8List bytes;
}

/// Holds one message's metadata and binary values until its carrier consumes it.
class CoreLinkMessage {
  /// Wraps control bytes without introducing an additional payload allocation.
  CoreLinkMessage.bytes(Uint8List bytes) : _parts = [bytes];

  /// Retains metadata and binary spans belonging to exactly one wire message.
  CoreLinkMessage._(this._parts);
  final List<Object> _parts;

  /// Reports the size of the portable inline representation.
  int get length => _parts.fold(
    0,
    (total, part) =>
        total +
        switch (part) {
          Uint8List bytes => bytes.length,
          _CoreLinkBinary(:final bytes) =>
            bytes.length + _binaryHeader(bytes.length).length,
          _ => throw StateError('Invalid message part'),
        },
  );

  /// Writes the portable representation into an exact destination.
  void writeTo(Uint8List destination) {
    if (destination.length != length) {
      throw ArgumentError('Core Link destination length mismatch');
    }
    var offset = 0;
    for (final part in _parts) {
      final Uint8List bytes;
      if (part is _CoreLinkBinary) {
        final header = _binaryHeader(part.bytes.length);
        destination.setRange(offset, offset + header.length, header);
        offset += header.length;
        bytes = part.bytes;
      } else {
        bytes = part as Uint8List;
      }
      destination.setRange(offset, offset + bytes.length, bytes);
      offset += bytes.length;
    }
  }

  /// Materializes portable bytes for a carrier that requires inline binary values.
  Uint8List toBytes() {
    if (_parts.length == 1 && _parts.single is Uint8List) {
      return _parts.single as Uint8List;
    }
    final bytes = Uint8List(length);
    writeTo(bytes);
    return bytes;
  }

  /// Encodes only metadata while the host provides shared ownership for binary values.
  CoreLinkSharedMessage share(CoreLinkBufferSpan Function(Uint8List) retain) {
    final output = BytesBuilder(copy: false);
    final buffers = <CoreLinkBufferSpan>[];
    for (final part in _parts) {
      if (part is _CoreLinkBinary) {
        final span = retain(part.bytes);
        output.add(_sharedReference(buffers.length, span.offset, span.length));
        buffers.add(span);
      } else {
        output.add(part as Uint8List);
      }
    }
    return CoreLinkSharedMessage(output.takeBytes(), buffers);
  }

  /// Preserves shared values when a Dart Flow reconstructs local decode metadata.
  Uint8List retainedBytes() {
    final output = BytesBuilder(copy: false);
    final buffers = <CoreLinkSharedBuffer>[];
    for (final part in _parts) {
      if (part is _CoreLinkBinary) {
        final span = coreLinkSharedSpan(part.bytes);
        if (span != null) {
          output.add(
            _sharedReference(buffers.length, span.offset, span.length),
          );
          buffers.add(span.buffer);
        } else {
          output.add(_binaryHeader(part.bytes.length));
          output.add(part.bytes);
        }
      } else {
        output.add(part as Uint8List);
      }
    }
    final bytes = output.takeBytes();
    attachCoreLinkBuffers(bytes, buffers);
    return bytes;
  }
}

/// Encodes the portable MessagePack binary prefix without visiting its contents.
Uint8List _binaryHeader(int length) {
  if (length <= 0xff) return Uint8List.fromList([0xc4, length]);
  if (length <= 0xffff) {
    return Uint8List.fromList([0xc5, length >> 8, length & 255]);
  }
  final bytes = Uint8List(5)..[0] = 0xc6;
  ByteData.sublistView(bytes).setUint32(1, length);
  return bytes;
}

/// Encodes one fixed-size shared-buffer reference for the native carrier.
Uint8List _sharedReference(int index, int offset, int length) {
  final bytes = Uint8List(27)
    ..[0] = 0xc7
    ..[1] = 24
    ..[2] = 0x42;
  final data = ByteData.sublistView(bytes);
  for (final entry in [(3, index), (11, offset), (19, length)]) {
    data.setUint32(entry.$1, entry.$2 ~/ 0x100000000);
    data.setUint32(entry.$1 + 4, entry.$2 & 0xffffffff);
  }
  return bytes;
}

/// Re-encodes reconstructed Flow metadata while retaining existing shared values.
Uint8List _encodeRetainedCoreLink(Object? value) {
  final writer = _CoreLinkMessagePackWriter();
  writer.writeValue(value);
  return writer.takeMessage().retainedBytes();
}

/// Writes metadata contiguously and retains binary spans for final destination writes.
class _CoreLinkMessagePackWriter {
  Uint8List _bytes = Uint8List(128);
  int _length = 0;
  final List<Object> _parts = [];

  /// Grows one contiguous output allocation to fit the next encoded segment.
  void _ensureCapacity(int additional) {
    final required = _length + additional;
    if (required <= _bytes.length) return;
    final doubled = _bytes.length * 2;
    final grown = Uint8List(required > doubled ? required : doubled);
    grown.setRange(0, _length, _bytes);
    _bytes = grown;
  }

  /// Seals encoded metadata before a binary span or the end of the message.
  void _flush() {
    if (_length == 0) return;
    _parts.add(Uint8List.sublistView(_bytes, 0, _length));
    _bytes = Uint8List(128);
    _length = 0;
  }

  /// Writes any supported Link value.
  void writeValue(Object? value) {
    if (value == null) {
      _writeByte(0xc0);
      return;
    }
    if (value is bool) {
      _writeByte(value ? 0xc3 : 0xc2);
      return;
    }
    if (value is int) {
      _writeInt(value);
      return;
    }
    if (value is double) {
      _writeFloat64(value);
      return;
    }
    if (value is String) {
      _writeString(value);
      return;
    }
    if (value is Uint8List) {
      _writeBinary(value);
      return;
    }
    if (value is List) {
      _writeArray(value);
      return;
    }
    if (value is Map) {
      _writeMap(value);
      return;
    }
    throw FormatException('Unsupported Link value type: ${value.runtimeType}');
  }

  /// Returns the exact encoded byte range without copying the output allocation.
  Uint8List takeBytes() {
    return takeMessage().toBytes();
  }

  /// Finishes one message without flattening its retained binary spans.
  CoreLinkMessage takeMessage() {
    _flush();
    return CoreLinkMessage._(_parts);
  }

  /// Writes an array header with the requested item count.
  void writeArrayHeader(int length) {
    if (length <= 15) {
      _writeByte(0x90 | length);
    } else if (length <= 0xffff) {
      _writeByte(0xdc);
      _writeUnsigned(2, length);
    } else {
      _writeByte(0xdd);
      _writeUnsigned(4, length);
    }
  }

  /// Writes one byte after validating its range.
  void _writeByte(int value) {
    _ensureCapacity(1);
    _bytes[_length++] = value & 0xff;
  }

  /// Writes a signed or unsigned integer using the smallest MessagePack form.
  void _writeInt(int value) {
    if (value >= 0) {
      _writeUint(value);
      return;
    }
    if (value >= -32) {
      _writeByte(value & 0xff);
      return;
    }
    if (value >= -0x80) {
      _writeByte(0xd0);
      _writeByte(value);
      return;
    }
    if (value >= -0x8000) {
      _writeByte(0xd1);
      _writeSigned(2, value);
      return;
    }
    if (value >= -0x80000000) {
      _writeByte(0xd2);
      _writeSigned(4, value);
      return;
    }
    _writeByte(0xd3);
    _writeSigned(8, value);
  }

  /// Writes a non-negative integer using the smallest MessagePack form.
  void _writeUint(int value) {
    if (value <= 0x7f) {
      _writeByte(value);
      return;
    }
    if (value <= 0xff) {
      _writeByte(0xcc);
      _writeByte(value);
      return;
    }
    if (value <= 0xffff) {
      _writeByte(0xcd);
      _writeUnsigned(2, value);
      return;
    }
    if (value <= 0xffffffff) {
      _writeByte(0xce);
      _writeUnsigned(4, value);
      return;
    }
    _writeByte(0xcf);
    _writeUnsigned(8, value);
  }

  /// Writes an unsigned integer as big-endian bytes.
  void _writeUnsigned(int byteCount, int value) {
    for (var shift = (byteCount - 1) * 8; shift >= 0; shift -= 8) {
      _writeByte(value >> shift);
    }
  }

  /// Writes a signed integer as two's-complement big-endian bytes.
  void _writeSigned(int byteCount, int value) {
    var encoded = value;
    if (value < 0) {
      encoded += 1 << (byteCount * 8);
    }
    _writeUnsigned(byteCount, encoded);
  }

  /// Writes a double using the MessagePack float64 form.
  void _writeFloat64(double value) {
    _writeByte(0xcb);
    _ensureCapacity(8);
    ByteData.sublistView(_bytes).setFloat64(_length, value);
    _length += 8;
  }

  /// Writes UTF-8 directly into the output, preserving surrogate replacement semantics.
  void _writeString(String value) {
    // Five reserved bytes hold the largest string prefix. Each UTF-16 code unit
    // needs at most three UTF-8 bytes, including an unpaired surrogate.
    _ensureCapacity(5 + value.length * 3);
    final bytes = _bytes;
    final start = _length + 5;
    var offset = start;
    for (var index = 0; index < value.length; index++) {
      var codePoint = value.codeUnitAt(index);
      if (codePoint <= 0x7f) {
        bytes[offset++] = codePoint;
        continue;
      }
      if (codePoint >= 0xd800 && codePoint <= 0xdfff) {
        final next = index + 1 < value.length ? value.codeUnitAt(index + 1) : 0;
        if (codePoint <= 0xdbff && next >= 0xdc00 && next <= 0xdfff) {
          codePoint = 0x10000 + ((codePoint - 0xd800) << 10) + next - 0xdc00;
          index++;
        } else {
          codePoint = 0xfffd;
        }
      }
      if (codePoint <= 0x7ff) {
        bytes[offset++] = 0xc0 | (codePoint >> 6);
        bytes[offset++] = 0x80 | (codePoint & 0x3f);
      } else if (codePoint <= 0xffff) {
        bytes[offset++] = 0xe0 | (codePoint >> 12);
        bytes[offset++] = 0x80 | ((codePoint >> 6) & 0x3f);
        bytes[offset++] = 0x80 | (codePoint & 0x3f);
      } else {
        bytes[offset++] = 0xf0 | (codePoint >> 18);
        bytes[offset++] = 0x80 | ((codePoint >> 12) & 0x3f);
        bytes[offset++] = 0x80 | ((codePoint >> 6) & 0x3f);
        bytes[offset++] = 0x80 | (codePoint & 0x3f);
      }
    }
    final length = offset - start;
    if (length <= 31) {
      _writeByte(0xa0 | length);
    } else if (length <= 0xff) {
      _writeByte(0xd9);
      _writeByte(length);
    } else if (length <= 0xffff) {
      _writeByte(0xda);
      _writeUnsigned(2, length);
    } else {
      _writeByte(0xdb);
      _writeUnsigned(4, length);
    }
    // Compact the prefix gap using an overlapping typed-data copy.
    if (_length != start) {
      bytes.setRange(_length, _length + length, bytes, start);
    }
    _length += length;
  }

  /// Writes native bytes using the MessagePack binary family.
  void _writeBinary(Uint8List value) {
    _flush();
    _parts.add(_CoreLinkBinary(value));
  }

  /// Writes a Link array.
  void _writeArray(List<Object?> value) {
    writeArrayHeader(value.length);
    for (final item in value) {
      writeValue(item);
    }
  }

  /// Writes a Link map with string keys.
  void _writeMap(Map<Object?, Object?> value) {
    final length = value.length;
    if (length <= 15) {
      _writeByte(0x80 | length);
    } else if (length <= 0xffff) {
      _writeByte(0xde);
      _writeUnsigned(2, length);
    } else {
      _writeByte(0xdf);
      _writeUnsigned(4, length);
    }
    for (final entry in value.entries) {
      final key = entry.key;
      if (key is! String) {
        throw FormatException(
          'Link map key must be a string: ${key.runtimeType}',
        );
      }
      _writeString(key);
      writeValue(entry.value);
    }
  }
}

/// Exposes typed reads over a single MessagePack value payload.
abstract interface class CoreLinkValueReader {
  /// Reads one dynamically typed Link value.
  Object? readValue();

  /// Reads one embedded stream descriptor and opens its generic item stream.
  Stream<T> readEmbeddedStream<T>(
    T Function(CoreLinkValueReader reader) decode,
  );

  /// Reads one untouched MessagePack value as a byte view.
  Uint8List readValueBytes();

  /// Reads one MessagePack array header.
  int readArrayLength();

  /// Reads one MessagePack map header.
  int readMapLength();

  /// Reads one UTF-8 string value.
  String readString();

  /// Reads one boolean value.
  bool readBool();

  /// Reads one integer value.
  int readInt();

  /// Reads one floating-point value.
  double readDouble();

  /// Reads one binary value.
  Uint8List readBytes();

  /// Consumes a nil marker when present.
  bool readNull();

  /// Reads a nullable value without constructing an intermediate container.
  T? readNullable<T>(T Function() decode) {
    if (readNull()) {
      return null;
    }
    return decode();
  }

  /// Skips one complete MessagePack value without allocating its contents.
  void skipValue();

  /// Reports whether the next value is encoded as a string.
  bool isNextString();
}

/// Reads the Link protocol's MessagePack value set without dart2js uint64 accessors.
class _CoreLinkMessagePackReader implements CoreLinkValueReader {
  final Uint8List _bytes;
  final CoreEmbeddedStreamFactory? _embeddedStreamFactory;
  int _offset = 0;

  /// Creates a reader over one complete MessagePack payload.
  _CoreLinkMessagePackReader(
    this._bytes, {
    String? target,
    CoreEmbeddedStreamFactory? embeddedStreamFactory,
  }) : _embeddedStreamFactory = embeddedStreamFactory;

  /// Reads a nullable value without constructing a dynamic container.
  @override
  T? readNullable<T>(T Function() decode) {
    if (readNull()) {
      return null;
    }
    return decode();
  }

  /// Reads one embedded stream descriptor and opens its generic item stream.
  @override
  Stream<T> readEmbeddedStream<T>(
    T Function(CoreLinkValueReader reader) decode,
  ) {
    final fieldCount = readMapLength();
    String? streamId;
    String? target;
    String? propertyName;
    Object? args;
    for (var index = 0; index < fieldCount; index += 1) {
      final key = readString();
      if (key != '\$coreStream') {
        skipValue();
        continue;
      }
      final descriptorCount = readMapLength();
      for (
        var descriptorIndex = 0;
        descriptorIndex < descriptorCount;
        descriptorIndex += 1
      ) {
        final descriptorKey = readString();
        if (descriptorKey == 'streamId') {
          streamId = readString();
        } else if (descriptorKey == 'target') {
          target = _readCoreTargetValue(readValue());
        } else if (descriptorKey == 'propertyName') {
          propertyName = readString();
        } else if (descriptorKey == 'args') {
          args = readValue();
        } else {
          skipValue();
        }
      }
    }
    final resolvedStreamId = streamId;
    final resolvedTarget = target;
    final resolvedPropertyName = propertyName;
    final factory = _embeddedStreamFactory;
    if (resolvedStreamId == null ||
        resolvedTarget == null ||
        resolvedPropertyName == null ||
        factory == null) {
      throw StateError('Embedded Core stream requires a stream factory');
    }
    return factory<T>(
      resolvedStreamId,
      resolvedTarget,
      resolvedPropertyName,
      args,
      decode,
    );
  }

  /// Reads any supported Link value.
  @override
  Object? readValue() {
    final marker = _readByte();
    if (marker <= 0x7f) {
      return marker;
    }
    if (marker >= 0xe0) {
      return marker - 0x100;
    }
    if ((marker & 0xe0) == 0xa0) {
      return _readString(marker & 0x1f);
    }
    if ((marker & 0xf0) == 0x90) {
      return _readArray(marker & 0x0f);
    }
    if ((marker & 0xf0) == 0x80) {
      return _readMap(marker & 0x0f);
    }

    switch (marker) {
      case 0xc0:
        return null;
      case 0xc2:
        return false;
      case 0xc3:
        return true;
      case 0xc4:
        return _readBinary(_readUnsigned(1));
      case 0xc5:
        return _readBinary(_readUnsigned(2));
      case 0xc6:
        return _readBinary(_readUnsigned(4));
      case 0xc7:
        return _readSharedBinary(_readUnsigned(1));
      case 0xca:
        return _readFloat32();
      case 0xcb:
        return _readFloat64();
      case 0xcc:
        return _readUnsigned(1);
      case 0xcd:
        return _readUnsigned(2);
      case 0xce:
        return _readUnsigned(4);
      case 0xcf:
        return _readUnsigned(8);
      case 0xd0:
        return _readSigned(1);
      case 0xd1:
        return _readSigned(2);
      case 0xd2:
        return _readSigned(4);
      case 0xd3:
        return _readSigned(8);
      case 0xd9:
        return _readString(_readUnsigned(1));
      case 0xda:
        return _readString(_readUnsigned(2));
      case 0xdb:
        return _readString(_readUnsigned(4));
      case 0xdc:
        return _readArray(_readUnsigned(2));
      case 0xdd:
        return _readArray(_readUnsigned(4));
      case 0xde:
        return _readMap(_readUnsigned(2));
      case 0xdf:
        return _readMap(_readUnsigned(4));
    }

    throw FormatException(
      'Unsupported MessagePack marker 0x${marker.toRadixString(16)}',
    );
  }

  /// Reads one untouched MessagePack value as a byte view.
  @override
  Uint8List readValueBytes() {
    final start = _offset;
    skipValue();
    return sliceCoreLinkBytes(_bytes, start, _offset);
  }

  /// Reads one MessagePack map header and returns its item count.
  @override
  int readMapLength() {
    final marker = _readByte();
    if ((marker & 0xf0) == 0x80) {
      return marker & 0x0f;
    }
    switch (marker) {
      case 0xde:
        return _readUnsigned(2);
      case 0xdf:
        return _readUnsigned(4);
    }
    throw FormatException(
      'Expected MessagePack map, got marker 0x${marker.toRadixString(16)}',
    );
  }

  /// Reads one MessagePack array header and returns its item count.
  @override
  int readArrayLength() {
    final marker = _readByte();
    return _readArrayLengthForMarker(marker);
  }

  /// Reads one MessagePack array header or a nil marker.
  int? readArrayLengthOrNull() {
    final marker = _readByte();
    if (marker == 0xc0) {
      return null;
    }
    return _readArrayLengthForMarker(marker);
  }

  /// Decodes one MessagePack array marker into its element count.
  int _readArrayLengthForMarker(int marker) {
    if ((marker & 0xf0) == 0x90) {
      return marker & 0x0f;
    }
    switch (marker) {
      case 0xdc:
        return _readUnsigned(2);
      case 0xdd:
        return _readUnsigned(4);
    }
    throw FormatException(
      'Expected MessagePack array, got marker 0x${marker.toRadixString(16)}',
    );
  }

  /// Reads one UTF-8 string value.
  @override
  String readString() {
    final marker = _readByte();
    if ((marker & 0xe0) == 0xa0) {
      return _readString(marker & 0x1f);
    }
    switch (marker) {
      case 0xd9:
        return _readString(_readUnsigned(1));
      case 0xda:
        return _readString(_readUnsigned(2));
      case 0xdb:
        return _readString(_readUnsigned(4));
    }
    throw FormatException(
      'Expected MessagePack string, got marker 0x${marker.toRadixString(16)}',
    );
  }

  /// Reads one boolean value.
  @override
  bool readBool() {
    final marker = _readByte();
    return switch (marker) {
      0xc2 => false,
      0xc3 => true,
      _ => throw FormatException(
        'Expected MessagePack bool, got marker 0x${marker.toRadixString(16)}',
      ),
    };
  }

  /// Reads one integer value without constructing a dynamic value.
  @override
  int readInt() {
    final marker = _readByte();
    if (marker <= 0x7f) {
      return marker;
    }
    if (marker >= 0xe0) {
      return marker - 0x100;
    }
    return switch (marker) {
      0xcc => _readUnsigned(1),
      0xcd => _readUnsigned(2),
      0xce => _readUnsigned(4),
      0xcf => _readUnsigned(8),
      0xd0 => _readSigned(1),
      0xd1 => _readSigned(2),
      0xd2 => _readSigned(4),
      0xd3 => _readSigned(8),
      _ => throw FormatException(
        'Expected MessagePack integer, got marker 0x${marker.toRadixString(16)}',
      ),
    };
  }

  /// Reads one floating-point value without constructing a dynamic value.
  @override
  double readDouble() {
    final marker = _readByte();
    return switch (marker) {
      0xca => _readFloat32(),
      0xcb => _readFloat64(),
      0xcc => _readUnsigned(1).toDouble(),
      0xcd => _readUnsigned(2).toDouble(),
      0xce => _readUnsigned(4).toDouble(),
      0xcf => _readUnsigned(8).toDouble(),
      0xd0 => _readSigned(1).toDouble(),
      0xd1 => _readSigned(2).toDouble(),
      0xd2 => _readSigned(4).toDouble(),
      0xd3 => _readSigned(8).toDouble(),
      _ when marker <= 0x7f => marker.toDouble(),
      _ when marker >= 0xe0 => (marker - 0x100).toDouble(),
      _ => throw FormatException(
        'Expected MessagePack number, got marker 0x${marker.toRadixString(16)}',
      ),
    };
  }

  /// Reads one binary value.
  @override
  Uint8List readBytes() {
    final marker = _readByte();
    return switch (marker) {
      0xc4 => _readBinary(_readUnsigned(1)),
      0xc5 => _readBinary(_readUnsigned(2)),
      0xc6 => _readBinary(_readUnsigned(4)),
      0xc7 => _readSharedBinary(_readUnsigned(1)),
      _ => throw FormatException(
        'Expected MessagePack binary, got marker 0x${marker.toRadixString(16)}',
      ),
    };
  }

  /// Consumes a nil marker when present.
  @override
  bool readNull() {
    if (_offset < _bytes.length && _bytes[_offset] == 0xc0) {
      _offset += 1;
      return true;
    }
    return false;
  }

  /// Skips one complete MessagePack value without allocating its contents.
  @override
  void skipValue() {
    final marker = _readByte();
    if (marker <= 0x7f || marker >= 0xe0) {
      return;
    }
    if ((marker & 0xe0) == 0xa0) {
      _skipBytes(marker & 0x1f);
      return;
    }
    if ((marker & 0xf0) == 0x90) {
      _skipValues(marker & 0x0f);
      return;
    }
    if ((marker & 0xf0) == 0x80) {
      _skipMapValues(marker & 0x0f);
      return;
    }
    switch (marker) {
      case 0xc0:
      case 0xc2:
      case 0xc3:
        return;
      case 0xc4:
        _skipBytes(_readUnsigned(1));
        return;
      case 0xc5:
        _skipBytes(_readUnsigned(2));
        return;
      case 0xc6:
        _skipBytes(_readUnsigned(4));
        return;
      case 0xc7:
        _readSharedBinary(_readUnsigned(1));
        return;
      case 0xca:
        _skipBytes(4);
        return;
      case 0xcb:
        _skipBytes(8);
        return;
      case 0xcc:
      case 0xd0:
        _skipBytes(1);
        return;
      case 0xcd:
      case 0xd1:
        _skipBytes(2);
        return;
      case 0xce:
      case 0xd2:
        _skipBytes(4);
        return;
      case 0xcf:
      case 0xd3:
        _skipBytes(8);
        return;
      case 0xd9:
        _skipBytes(_readUnsigned(1));
        return;
      case 0xda:
        _skipBytes(_readUnsigned(2));
        return;
      case 0xdb:
        _skipBytes(_readUnsigned(4));
        return;
      case 0xdc:
        _skipValues(_readUnsigned(2));
        return;
      case 0xdd:
        _skipValues(_readUnsigned(4));
        return;
      case 0xde:
        _skipMapValues(_readUnsigned(2));
        return;
      case 0xdf:
        _skipMapValues(_readUnsigned(4));
        return;
    }
    throw FormatException(
      'Unsupported MessagePack marker 0x${marker.toRadixString(16)}',
    );
  }

  /// Reports whether the next value is encoded as a string.
  @override
  bool isNextString() {
    if (_offset >= _bytes.length) {
      throw const FormatException('Unexpected end of Link payload');
    }
    final marker = _bytes[_offset];
    return (marker & 0xe0) == 0xa0 ||
        marker == 0xd9 ||
        marker == 0xda ||
        marker == 0xdb;
  }

  /// Verifies the reader consumed the complete payload.
  void expectDone() {
    if (_offset != _bytes.length) {
      throw FormatException(
        'Trailing bytes after Link payload: ${_bytes.length - _offset}',
      );
    }
  }

  /// Reads one byte from the payload.
  int _readByte() {
    _require(1);
    return _bytes[_offset++];
  }

  /// Reads an unsigned big-endian integer with explicit byte composition.
  int _readUnsigned(int byteCount) {
    _require(byteCount);
    var value = 0;
    for (var i = 0; i < byteCount; i += 1) {
      value = (value * 0x100) + _bytes[_offset++];
    }
    return value;
  }

  /// Reads a signed two's-complement big-endian integer.
  int _readSigned(int byteCount) {
    final unsigned = _readUnsigned(byteCount);
    final signBit = 1 << ((byteCount * 8) - 1);
    if ((unsigned & signBit) == 0) {
      return unsigned;
    }
    return unsigned - (1 << (byteCount * 8));
  }

  /// Reads a MessagePack float32 value.
  double _readFloat32() {
    _require(4);
    final data = ByteData.sublistView(_bytes, _offset, _offset + 4);
    _offset += 4;
    return data.getFloat32(0);
  }

  /// Reads a MessagePack float64 value.
  double _readFloat64() {
    _require(8);
    final data = ByteData.sublistView(_bytes, _offset, _offset + 8);
    _offset += 8;
    return data.getFloat64(0);
  }

  /// Validates and resolves a shared slice without reading its binary contents.
  Uint8List _readSharedBinary(int length) {
    if (length != 24 || _readByte() != 0x42) {
      throw const FormatException('Invalid shared buffer extension');
    }
    final index = _readUnsigned(8);
    final offset = _readUnsigned(8);
    final size = _readUnsigned(8);
    final buffers = _sharedDecodeBuffers[_bytes];
    if (buffers == null || index < 0 || index >= buffers.length) {
      throw const FormatException('Invalid shared buffer index');
    }
    final buffer = buffers[index];
    if (offset < 0 ||
        size < 0 ||
        offset > buffer.bytes.length ||
        size > buffer.bytes.length - offset) {
      throw const FormatException('Invalid shared buffer range');
    }
    return CoreLinkBufferSpan(buffer, offset, size).view();
  }

  /// Reads native bytes.
  Uint8List _readBinary(int length) {
    _require(length);
    final value = Uint8List.sublistView(_bytes, _offset, _offset + length);
    _offset += length;
    return value;
  }

  /// Reads a UTF-8 string.
  String _readString(int length) {
    _require(length);
    final value = const Utf8Decoder().convert(
      _bytes,
      _offset,
      _offset + length,
    );
    _offset += length;
    return value;
  }

  /// Decodes an owned, growable array so deltas can resize it without cloning.
  List<Object?> _readArray(int length) {
    return List<Object?>.generate(length, (_) => readValue(), growable: true);
  }

  /// Reads a Link map with string keys.
  Map<String, Object?> _readMap(int length) {
    final value = <String, Object?>{};
    for (var i = 0; i < length; i += 1) {
      final key = readValue();
      if (key is! String) {
        throw FormatException(
          'Link map key must be a string: ${key.runtimeType}',
        );
      }
      value[key] = readValue();
    }
    return value;
  }

  /// Skips a fixed number of MessagePack values.
  void _skipValues(int count) {
    for (var i = 0; i < count; i += 1) {
      skipValue();
    }
  }

  /// Skips a fixed number of MessagePack map entries.
  void _skipMapValues(int count) {
    for (var i = 0; i < count; i += 1) {
      skipValue();
      skipValue();
    }
  }

  /// Advances past a raw byte segment after validating its length.
  void _skipBytes(int count) {
    _require(count);
    _offset += count;
  }

  /// Checks that the requested byte count is present.
  void _require(int byteCount) {
    if (_offset + byteCount > _bytes.length) {
      throw FormatException('Unexpected end of Link payload');
    }
  }
}

/// Measures generic watch decoding when the diagnostic flag is enabled.
void _profileCoreLinkDecode(String stage, Stopwatch watch, int bytes) {
  final duration = watch.elapsedMicroseconds / 1000;
  debugPrintSynchronously(
    'event=compose_pipeline_stage stage=$stage durationMs=${duration.toStringAsFixed(3)} '
    'wallMicros=${DateTime.now().microsecondsSinceEpoch} bytes=$bytes',
  );
}
