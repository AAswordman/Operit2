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
}

class CoreLinkError implements Exception {
  const CoreLinkError({required this.code, required this.message});

  factory CoreLinkError.fromJson(Map<String, Object?> json) {
    return CoreLinkError(
      code: json['code'] as String,
      message: json['message'] as String,
    );
  }

  final String code;
  final String message;

  @override
  String toString() {
    return '$code: $message';
  }
}
