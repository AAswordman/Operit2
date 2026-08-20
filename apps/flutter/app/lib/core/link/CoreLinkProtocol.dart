// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';

class CoreObjectPath {
  const CoreObjectPath(this.segments);

  factory CoreObjectPath.parse(String path) {
    return CoreObjectPath(
      path
          .split('.')
          .map((segment) => segment.trim())
          .where((segment) => segment.isNotEmpty)
          .toList(growable: false),
    );
  }

  final List<String> segments;

  String get key => segments.join('.');
}

class CoreCallRequest {
  const CoreCallRequest({
    required this.requestId,
    required this.targetPath,
    required this.methodName,
    required this.args,
  });

  final String requestId;
  final CoreObjectPath targetPath;
  final String methodName;
  final Object? args;
}

class CoreWatchRequest {
  const CoreWatchRequest({
    required this.requestId,
    required this.targetPath,
    required this.propertyName,
    required this.args,
  });

  final String requestId;
  final CoreObjectPath targetPath;
  final String propertyName;
  final Object? args;
}

class CorePushRequest {
  /// Creates a client-owned input stream targeting one Core method.
  const CorePushRequest({
    required this.requestId,
    required this.targetPath,
    required this.methodName,
    this.args = const <String, Object?>{},
  });

  final String requestId;
  final CoreObjectPath targetPath;
  final String methodName;
  final Object? args;
}

abstract class CorePushSink {
  /// Sends one ordered argument value into the input stream.
  Future<void> add(Object? args);

  /// Completes the input stream after all queued values are sent.
  Future<void> close();
}

class CoreEvent {
  CoreEvent({
    required this.requestId,
    required this.targetPath,
    required this.propertyName,
    required this.kind,
    required Object? value,
  }) : _value = value,
       _valueBytes = null,
       _decodeValue = null;

  CoreEvent.raw({
    required this.requestId,
    required this.targetPath,
    required this.propertyName,
    required this.kind,
    required Uint8List valueBytes,
    required Object? Function(Uint8List bytes) decodeValue,
  }) : _value = null,
       _valueBytes = valueBytes,
       _decodeValue = decodeValue;

  final String? requestId;
  final CoreObjectPath targetPath;
  final String propertyName;
  final String kind;
  final Uint8List? _valueBytes;
  final Object? Function(Uint8List bytes)? _decodeValue;
  Object? _value;
  var _hasDecodedValue = false;

  /// Returns the generic payload only when a generic consumer reads it.
  Object? get value {
    final bytes = _valueBytes;
    final decodeValue = _decodeValue;
    if (bytes == null || decodeValue == null || _hasDecodedValue) {
      return _value;
    }
    _value = decodeValue(bytes);
    _hasDecodedValue = true;
    return _value;
  }

  /// Exposes the untouched MessagePack payload for typed generated consumers.
  Uint8List? get valueBytes => _valueBytes;
}

class CoreLinkErrorLocation {
  const CoreLinkErrorLocation({
    required this.file,
    required this.line,
    required this.column,
  });

  factory CoreLinkErrorLocation.fromJson(Map<String, Object?> json) {
    return CoreLinkErrorLocation(
      file: json['file'] as String,
      line: json['line'] as int,
      column: json['column'] as int,
    );
  }

  final String file;
  final int line;
  final int column;

  @override
  String toString() {
    return '$file:$line:$column';
  }
}

class CoreLinkError implements Exception {
  const CoreLinkError({
    required this.code,
    required this.message,
    this.details,
    this.location,
    this.backtrace,
  });

  factory CoreLinkError.fromJson(Map<String, Object?> json) {
    final locationJson = json['location'] as Map<String, Object?>?;
    return CoreLinkError(
      code: json['code'] as String,
      message: json['message'] as String,
      details: json['details'],
      location: locationJson == null
          ? null
          : CoreLinkErrorLocation.fromJson(locationJson),
      backtrace: json['backtrace'] as String?,
    );
  }

  final String code;
  final String message;
  final Object? details;
  final CoreLinkErrorLocation? location;
  final String? backtrace;

  @override
  String toString() {
    final buffer = StringBuffer('$code: $message');
    final location = this.location;
    if (location != null) {
      buffer.write('\nRust error location: $location');
    }
    final backtrace = this.backtrace;
    if (backtrace != null && backtrace.isNotEmpty) {
      buffer.write('\nRust backtrace:\n$backtrace');
    }
    return buffer.toString();
  }
}
