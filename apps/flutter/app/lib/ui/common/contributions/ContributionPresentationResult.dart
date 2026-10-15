// ignore_for_file: file_names

import 'dart:math';

import 'package:flutter/foundation.dart';

/// Contains the caller-owned Presentation V1 invocation and opaque provider input.
@immutable
class ContributionPresentationContext {
  /// Creates an exact context received from a trusted host invocation.
  const ContributionPresentationContext({
    required this.requestId,
    required this.input,
  });

  /// Allocates an independent identifier for each actual presentation request.
  factory ContributionPresentationContext.create({required Object? input}) {
    final random = Random.secure();
    final requestId = List<String>.generate(
      16,
      (_) => random.nextInt(256).toRadixString(16).padLeft(2, '0'),
      growable: false,
    ).join();
    return ContributionPresentationContext(requestId: requestId, input: input);
  }

  final String requestId;
  final Object? input;

  /// Encodes the locked Presentation V1 context without interpreting its input.
  Map<String, Object?> toJson() => <String, Object?>{
    'requestId': requestId,
    'input': input,
  };
}

/// Distinguishes explicit completion from explicit cancellation of a presentation.
enum ContributionPresentationStatus { completed, cancelled }

/// Carries only one of the two locked Presentation V1 completion envelopes.
@immutable
class ContributionPresentationResult {
  /// Creates a completed invocation while preserving its opaque JSON value.
  const ContributionPresentationResult.completed({
    required this.requestId,
    required this.value,
  }) : status = ContributionPresentationStatus.completed;

  /// Creates an explicitly cancelled invocation without a completion value.
  const ContributionPresentationResult.cancelled({required this.requestId})
    : status = ContributionPresentationStatus.cancelled,
      value = null;

  static const String completeType = 'toolpkg.presentation.complete';
  static const String cancelType = 'toolpkg.presentation.cancel';

  final String requestId;
  final ContributionPresentationStatus status;
  final Object? value;

  /// Accepts only an exact V1 type and the identifier owned by this invocation.
  static ContributionPresentationResult? fromActionResult(
    Object? raw, {
    required ContributionPresentationContext request,
  }) {
    if (raw is! Map || raw['requestId'] != request.requestId) {
      return null;
    }
    switch (raw['type']) {
      case completeType:
        if (!raw.containsKey('value')) {
          throw const FormatException(
            'A completed Presentation V1 envelope needs a value field.',
          );
        }
        return ContributionPresentationResult.completed(
          requestId: request.requestId,
          value: raw['value'],
        );
      case cancelType:
        return ContributionPresentationResult.cancelled(
          requestId: request.requestId,
        );
      default:
        return null;
    }
  }

  /// Validates a normalized result against the invocation that owns its dialog.
  void validateRequest(ContributionPresentationContext request) {
    if (requestId != request.requestId) {
      throw StateError('A presentation result belongs to another invocation.');
    }
  }
}
