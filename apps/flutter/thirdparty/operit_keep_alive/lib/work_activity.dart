import 'dart:convert';

import 'package:flutter/foundation.dart';

/// Represents an execution outcome independently of any operating-system surface.
enum WorkStatus { running, waiting, completed, failed, cancelled }

/// Describes which owner keeps system presentation current after UI detachment.
enum WorkUpdatesOwner { host, client }

/// Names the system surface actually supplied by a presentation host.
enum WorkSystemSurface { tray, notification, none }

/// Carries explicit host presentation capabilities without selecting a platform in Dart.
@immutable
class WorkPresentationCapabilities {
  /// Constructs an acknowledged presentation capability report.
  const WorkPresentationCapabilities({
    required this.updatesOwner,
    required this.surface,
    required this.liveUpdates,
  });

  /// Rejects unknown capability values at the native protocol boundary.
  factory WorkPresentationCapabilities.fromMap(Map<Object?, Object?> map) {
    final owner = switch (map['updatesOwner']) {
      'host' => WorkUpdatesOwner.host,
      'client' => WorkUpdatesOwner.client,
      _ => throw const FormatException('Unknown work updates owner'),
    };
    final surface = switch (map['surface']) {
      'tray' => WorkSystemSurface.tray,
      'notification' => WorkSystemSurface.notification,
      'none' => WorkSystemSurface.none,
      _ => throw const FormatException('Unknown work system surface'),
    };
    final live = map['liveUpdates'];
    if (live is! bool) {
      throw const FormatException('Invalid live update capability');
    }
    return WorkPresentationCapabilities(
      updatesOwner: owner,
      surface: surface,
      liveUpdates: live,
    );
  }

  final WorkUpdatesOwner updatesOwner;
  final WorkSystemSurface surface;
  final bool liveUpdates;
}

/// Measures the named step alone; its percentage never describes an entire agent run.
@immutable
class WorkStepProgress {
  /// Requires the producer's explicit step and percentage.
  const WorkStepProgress({required this.value, required this.scope});
  final double value;
  final String scope;
}

/// Carries stable identity, lifecycle, attention, and progress for one actual execution.
@immutable
class WorkActivity {
  /// Constructs one validated execution record.
  const WorkActivity({
    required this.workId,
    required this.activation,
    required this.runId,
    required this.title,
    required this.status,
    required this.phase,
    required this.detail,
    required this.startedAtMillis,
    required this.endedAtMillis,
    required this.progress,
    required this.permissionRequestIds,
    required this.canStop,
  });

  /// Decodes the shared Core work schema and rejects contradictory lifecycle fields.
  factory WorkActivity.fromMap(Map<String, Object?> map) {
    final status = WorkStatus.values.byName(_string(map, 'status'));
    final started = _integer(map, 'startedAtMillis');
    final ended = map['endedAtMillis'];
    final canStop = map['canStop'];
    final requests = map['permissionRequestIds'];
    if ((ended != null && ended is! int) ||
        canStop is! bool ||
        requests is! List<Object?> ||
        requests.any((id) => id is! String)) {
      throw const FormatException(
        'Invalid work lifecycle or permission requests',
      );
    }
    final active = status == WorkStatus.running || status == WorkStatus.waiting;
    if (active != (ended == null) ||
        (!active && canStop) ||
        (ended is int && ended < started)) {
      throw const FormatException('Contradictory work lifecycle');
    }
    WorkStepProgress? progress;
    final rawProgress = map['progress'];
    if (rawProgress != null) {
      if (rawProgress is! Map<String, Object?>) {
        throw const FormatException('Invalid step progress');
      }
      final value = rawProgress['value'];
      if (value is! num || !value.isFinite || value < 0 || value > 100) {
        throw const FormatException('Invalid step percentage');
      }
      progress = WorkStepProgress(
        value: value.toDouble(),
        scope: _string(rawProgress, 'scope'),
      );
    }
    final phase = _string(map, 'phase');
    if (!const {
      'processing',
      'connecting',
      'receiving',
      'executingTool',
      'processingToolResult',
      'summarizing',
      'executingPlan',
      'ended',
    }.contains(phase)) {
      throw FormatException('Unknown work phase: $phase');
    }
    return WorkActivity(
      workId: _string(map, 'workId'),
      activation: _activation(map['activation']),
      runId: _integer(map, 'runId'),
      title: _string(map, 'title'),
      status: status,
      phase: phase,
      detail: _string(map, 'detail', allowEmpty: true),
      startedAtMillis: started,
      endedAtMillis: ended as int?,
      progress: progress,
      permissionRequestIds: List<String>.unmodifiable(requests.cast<String>()),
      canStop: canStop,
    );
  }

  final String workId;
  final Map<String, String> activation;
  final int runId;
  final String title;
  final WorkStatus status;
  final String phase;
  final String detail;
  final int startedAtMillis;
  final int? endedAtMillis;
  final WorkStepProgress? progress;
  final List<String> permissionRequestIds;
  final bool canStop;

  /// Reports real execution activity, including work blocked on a human decision.
  bool get isActive => endedAtMillis == null;

  /// Formats elapsed wall-clock time and freezes its value at the recorded outcome.
  String get elapsedText {
    final end = isActive ? DateTime.now().millisecondsSinceEpoch : endedAtMillis!;
    final seconds = Duration(milliseconds: end - startedAtMillis).inSeconds;
    return '${(seconds ~/ 60).toString().padLeft(2, '0')}:${(seconds % 60).toString().padLeft(2, '0')}';
  }

  /// Produces display-only fields while preserving exact action identity.
  Map<String, Object?> presentation(WorkPresentationLabels labels) => {
    'workId': workId,
    'activation': activation,
    'title': title,
    'subtitle': labels.describe(this),
    'elapsedText': elapsedText,
    'status': status.name,
    'canStop': canStop,
    'openLabel': status == WorkStatus.waiting ? labels.review : labels.open,
    'stopLabel': labels.stop,
    'startedAtMillis': startedAtMillis,
    'endedAtMillis': endedAtMillis,
    'progress': progress == null
        ? null
        : {'value': progress!.value, 'scope': progress!.scope},
  };
}

/// Holds one complete revision so clients and hosts can discard older deliveries.
@immutable
class WorkActivitySnapshot {
  /// Creates a full process snapshot.
  const WorkActivitySnapshot({required this.revision, required this.works});

  /// Validates the versioned Core payload instead of guessing absent fields.
  factory WorkActivitySnapshot.fromJson(String encoded) {
    final decoded = jsonDecode(encoded);
    if (decoded is! Map<String, Object?> ||
        decoded['schemaVersion'] != 1 ||
        decoded['works'] is! List<Object?>) {
      throw const FormatException('Unsupported work snapshot schema');
    }
    final works = (decoded['works']! as List<Object?>)
        .map((value) {
          if (value is! Map<String, Object?>) {
            throw const FormatException('Invalid work record');
          }
          return WorkActivity.fromMap(value);
        })
        .toList(growable: false);
    if (works.map((work) => work.workId).toSet().length != works.length) {
      throw const FormatException('Duplicate work identity');
    }
    return WorkActivitySnapshot(
      revision: _integer(decoded, 'revision'),
      works: List.unmodifiable(works),
    );
  }

  final int revision;
  final List<WorkActivity> works;

  /// Lists all running work and pending human decisions.
  List<WorkActivity> get active =>
      works.where((work) => work.isActive).toList(growable: false);

  /// Builds the shared native menu/notification protocol with explicit application wording.
  Map<String, Object?> presentation(WorkPresentationLabels labels) {
    final running = active;
    final waiting = running
        .where((work) => work.status == WorkStatus.waiting)
        .length;
    final visible = [
      ...running,
      ...works.where((work) => !work.isActive).take(1),
    ];
    return {
      'revision': revision,
      'activeCount': running.length,
      'waitingCount': waiting,
      'summary': labels.summary(running.length, waiting),
      'items': visible
          .map((work) => work.presentation(labels))
          .toList(growable: false),
    };
  }
}

/// Supplies application wording without exposing chat or tool business objects to hosts.
@immutable
class WorkPresentationLabels {
  /// Requires complete wording for every lifecycle and execution phase.
  const WorkPresentationLabels({
    required this.open,
    required this.review,
    required this.stop,
    required this.idle,
    required this.running,
    required this.waiting,
    required this.waitingCount,
    required this.completed,
    required this.failed,
    required this.cancelled,
    required this.phases,
  });
  final String open,
      review,
      stop,
      idle,
      running,
      waiting,
      waitingCount,
      completed,
      failed,
      cancelled;
  final Map<String, String> phases;

  /// Describes current activity with explicit step-scoped progress.
  String describe(WorkActivity work) {
    final state = switch (work.status) {
      WorkStatus.waiting => waiting,
      WorkStatus.completed => completed,
      WorkStatus.failed => failed,
      WorkStatus.cancelled => cancelled,
      WorkStatus.running => phases[work.phase]!,
    };
    final progress = work.progress;
    final parts = [
      state,
      if (work.detail.isNotEmpty) work.detail,
      if (progress != null)
        '${progress.scope} ${progress.value.toStringAsFixed(0)}%',
    ];
    return parts.join(' · ');
  }

  /// Formats a compact aggregate without changing per-work lifecycle information.
  String summary(int active, int attention) => active == 0
      ? idle
      : '$active $running${attention == 0 ? '' : ' · $attention $waitingCount'}';
}

/// Carries an explicit user action emitted by a system work surface.
@immutable
class WorkAction {
  /// Requires the exact execution and a supported action name.
  const WorkAction({
    required this.workId,
    required this.activation,
    required this.action,
  });

  /// Rejects malformed action callbacks before they reach the Core.
  factory WorkAction.fromMap(Map<Object?, Object?> map) {
    final id = map['workId'];
    final action = map['action'];
    final activation = _activation(map['activation']);
    if (id is! String || id.isEmpty || (action != 'open' && action != 'stop')) {
      throw const FormatException('Invalid work action');
    }
    return WorkAction(
      workId: id,
      activation: activation,
      action: action! as String,
    );
  }
  final String workId;
  final Map<String, String> activation;
  final String action;
}

/// Reads a required protocol string while preserving intentional empty detail fields.
String _string(
  Map<String, Object?> map,
  String key, {
  bool allowEmpty = false,
}) {
  final value = map[key];
  if (value is! String || (!allowEmpty && value.trim().isEmpty)) {
    throw FormatException('Invalid work field: $key');
  }
  return value;
}

/// Reads an exact nonnegative JSON integer.
int _integer(Map<String, Object?> map, String key) {
  final value = map[key];
  if (value is! int || value < 0) {
    throw FormatException('Invalid work integer: $key');
  }
  return value;
}

/// Validates an opaque application destination without interpreting its business fields.
Map<String, String> _activation(Object? value) {
  if (value is! Map ||
      value.isEmpty ||
      value.keys.any((key) => key is! String) ||
      value.values.any((entry) => entry is! String)) {
    throw const FormatException('Invalid work activation payload');
  }
  return Map<String, String>.unmodifiable(value.cast<String, String>());
}
