// ignore_for_file: file_names

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

  Map<String, Object?> toJson() {
    return {'segments': segments};
  }
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

  Map<String, Object?> toJson() {
    return {
      'requestId': requestId,
      'targetPath': targetPath.toJson(),
      'methodName': methodName,
      'args': args,
    };
  }
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

  Map<String, Object?> toJson() {
    return {
      'requestId': requestId,
      'targetPath': targetPath.toJson(),
      'propertyName': propertyName,
      'args': args,
    };
  }
}

class CorePushRequest {
  /// Creates a client-owned input stream targeting one Core method.
  const CorePushRequest({
    required this.requestId,
    required this.targetPath,
    required this.methodName,
  });

  final String requestId;
  final CoreObjectPath targetPath;
  final String methodName;

  /// Encodes this push target for the Link carrier.
  Map<String, Object?> toJson() {
    return {
      'requestId': requestId,
      'targetPath': targetPath.toJson(),
      'methodName': methodName,
    };
  }
}

abstract class CorePushSink {
  /// Sends one ordered argument value into the input stream.
  void add(Object? args);

  /// Completes the input stream after all queued values are sent.
  Future<void> close();
}

class CoreEvent {
  const CoreEvent({
    required this.requestId,
    required this.targetPath,
    required this.propertyName,
    required this.kind,
    required this.value,
  });

  factory CoreEvent.fromJson(Map<String, Object?> json) {
    return CoreEvent(
      requestId: json['requestId'] as String?,
      targetPath: CoreObjectPath(
        ((json['targetPath'] as Map<String, Object?>)['segments']
                as List<Object?>)
            .cast<String>(),
      ),
      propertyName: json['propertyName'] as String,
      kind: json['kind'] as String,
      value: json['value'],
    );
  }

  final String? requestId;
  final CoreObjectPath targetPath;
  final String propertyName;
  final String kind;
  final Object? value;

  Map<String, Object?> toJson() {
    return {
      'requestId': requestId,
      'targetPath': targetPath.toJson(),
      'propertyName': propertyName,
      'kind': kind,
      'value': value,
    };
  }
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
