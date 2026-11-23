import 'package:flutter/widgets.dart';

/// Exposes host-owned visits without tying execution to a Flutter engine or view.
abstract class PlatformWebVisitHost {
  /// Replays active visits, then reports task state and completion in order.
  Stream<Map<String, Object?>> watchVisits();

  /// Starts an explicitly requested visit and returns its terminal response.
  Future<Map<String, Object?>> visit(Map<String, Object?> request);

  /// Applies a user action to the authoritative host task.
  Future<void> control(String requestId, String action);

  /// Presents the existing browser instance; mounting never navigates again.
  Widget buildSurface(String requestId);
}
