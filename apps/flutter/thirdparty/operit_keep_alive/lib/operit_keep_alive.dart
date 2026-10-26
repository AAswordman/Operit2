import 'dart:async';
import 'dart:convert';
import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'work_activity.dart';
export 'work_activity.dart';

/// Describes the execution protection actually supplied by the current host.
enum KeepAliveProtection { continuous, timeLimited, unavailable }

/// Carries validated host capabilities and the current process-owned lease count.
@immutable
class KeepAliveReport {
  /// Creates one explicit protection report without implying an OS lifetime guarantee.
  const KeepAliveReport({
    required this.protection,
    required this.closeToTray,
    required this.activeLeases,
    required this.detail,
  });

  /// Decodes the shared native and browser protocol without inventing capability defaults.
  factory KeepAliveReport.fromMap(Map<Object?, Object?> value) {
    final protection = switch (value['protection']) {
      'continuous' => KeepAliveProtection.continuous,
      'timeLimited' => KeepAliveProtection.timeLimited,
      'unavailable' => KeepAliveProtection.unavailable,
      final value => throw FormatException(
        'Unknown keep-alive protection: $value',
      ),
    };
    final count = value['activeLeases'];
    final tray = value['closeToTray'];
    final detail = value['detail'];
    if (count is! int || count < 0 || tray is! bool || detail is! String) {
      throw const FormatException('Invalid keep-alive host report');
    }
    return KeepAliveReport(
      protection: protection,
      closeToTray: tray,
      activeLeases: count,
      detail: detail,
    );
  }

  final KeepAliveProtection protection;
  final bool closeToTray;
  final int activeLeases;
  final String detail;
}

/// Supplies presentation and the host service used by the embedding application.
@immutable
class KeepAliveOptions {
  /// Requires explicit application text and the Android Core service integration class.
  const KeepAliveOptions({
    required this.applicationName,
    required this.androidServiceClassName,
    required this.openLabel,
    required this.exitLabel,
    required this.quitMessage,
  });

  final String applicationName;
  final String androidServiceClassName;
  final String openLabel;
  final String exitLabel;
  final String quitMessage;

  /// Encodes identical arguments for every registered platform implementation.
  Map<String, Object> toMap() => {
    'applicationName': applicationName,
    'androidServiceClassName': androidServiceClassName,
    'openLabel': openLabel,
    'exitLabel': exitLabel,
    'quitMessage': quitMessage,
  };
}

/// Owns one explicit execution lease independently of a Flutter route or widget.
class KeepAliveLease {
  /// Associates an acknowledged native lease with its owning controller.
  KeepAliveLease._(this._owner, this.id, this.report);

  final OperitKeepAlive _owner;
  final String id;
  final KeepAliveReport report;
  Future<void>? _release;

  /// Releases this lease once and shares the same result with concurrent callers.
  Future<void> release() => _release ??= _owner._release(id);
}

/// Controls the main process through the plugin transport without platform branches.
class OperitKeepAlive {
  /// Accepts a channel explicitly so transport behavior can be tested independently.
  OperitKeepAlive({
    MethodChannel channel = const MethodChannel('operit/keep_alive'),
  }) : _channel = channel;

  static final OperitKeepAlive instance = OperitKeepAlive();

  final MethodChannel _channel;
  final ValueNotifier<KeepAliveReport?> _report = ValueNotifier(null);
  Future<KeepAliveReport>? _initialization;
  final String _session =
      '${DateTime.now().microsecondsSinceEpoch}-${Random.secure().nextInt(0x100000000)}';
  int _nextLease = 0;
  int _nextWorkRevision = 0;
  final StreamController<String> _workErrors =
      StreamController<String>.broadcast();

  /// Observes original native presentation and action failures without suppressing them.
  Stream<String> get workErrors => _workErrors.stream;
  final StreamController<WorkAction> _workActions =
      StreamController<WorkAction>.broadcast();

  /// Observes exact execution actions emitted by native work surfaces.
  Stream<WorkAction> get workActions => _workActions.stream;

  /// Queries native presentation ownership after main-process initialization.
  Future<WorkPresentationCapabilities> workPresentationCapabilities() async {
    final initialization = _initialization;
    if (initialization == null) {
      throw StateError('Keep-alive is not initialized');
    }
    await initialization;
    final map = await _channel.invokeMapMethod<Object?, Object?>(
      'workCapabilities',
      <String, Object>{},
    );
    if (map == null) throw StateError('Work capabilities returned no report');
    return WorkPresentationCapabilities.fromMap(map);
  }

  /// Releases queued cold-start actions after the application installs its work receiver.
  Future<void> enableWorkActions() =>
      _channel.invokeMethod<void>('workActionsReady', <String, Object>{});

  /// Publishes a complete revision to the registered native presentation host.
  Future<void> publishWork(
    WorkActivitySnapshot snapshot,
    WorkPresentationLabels labels,
  ) async {
    final initialization = _initialization;
    if (initialization == null) {
      throw StateError('Keep-alive is not initialized');
    }
    await initialization;
    final presentation = snapshot.presentation(labels);
    await _channel.invokeMethod<void>('publishWork', {
      ...presentation,
      'revision': ++_nextWorkRevision,
      'json': jsonEncode({...presentation, 'revision': _nextWorkRevision}),
    });
  }

  /// Observes the latest acknowledged capability, including host expiration events.
  ValueListenable<KeepAliveReport?> get report => _report;

  /// Installs the main window close policy and reads real host capabilities once.
  Future<KeepAliveReport> initialize(KeepAliveOptions options) {
    return _initialization ??= _initialize(options);
  }

  /// Registers protection events before applying process-owned initialization.
  Future<KeepAliveReport> _initialize(KeepAliveOptions options) async {
    _channel.setMethodCallHandler(_handleHostEvent);
    return _invokeReport('initialize', options.toMap());
  }

  /// Acquires protection after initialization and returns the host's actual status.
  Future<KeepAliveLease> acquire({required String reason}) async {
    final initialization = _initialization;
    if (initialization == null) {
      throw StateError('Keep-alive must be initialized by the main process');
    }
    await initialization;
    if (reason.trim().isEmpty) {
      throw ArgumentError.value(
        reason,
        'reason',
        'Must describe the protected work',
      );
    }
    final id = '$_session-lease-${++_nextLease}';
    final report = await _invokeReport('acquire', {'id': id, 'reason': reason});
    return KeepAliveLease._(this, id, report);
  }

  /// Keeps a finite operation protected until its success, failure, or cancellation.
  Future<T> run<T>({
    required String reason,
    required Future<T> Function() work,
  }) async {
    final lease = await acquire(reason: reason);
    try {
      return await work();
    } finally {
      await lease.release();
    }
  }

  /// Removes one acknowledged lease and publishes the resulting protection state.
  Future<void> _release(String id) async {
    await _invokeReport('release', {'id': id});
  }

  /// Rejects missing or malformed host responses instead of reporting false success.
  Future<KeepAliveReport> _invokeReport(
    String method,
    Map<String, Object> arguments,
  ) async {
    final response = await _channel.invokeMapMethod<Object?, Object?>(
      method,
      arguments,
    );
    if (response == null) {
      throw StateError('Keep-alive $method returned no report');
    }
    final report = KeepAliveReport.fromMap(response);
    _report.value = report;
    return report;
  }

  /// Publishes expiration and host failures through the same report observed by the app.
  Future<void> _handleHostEvent(MethodCall call) async {
    if (call.method == 'workPresentationError') {
      final arguments = call.arguments;
      if (arguments is! Map<Object?, Object?> ||
          arguments['message'] is! String) {
        throw const FormatException('Invalid work presentation error');
      }
      _workErrors.add(arguments['message']! as String);
      return;
    }
    if (call.method == 'workAction') {
      final arguments = call.arguments;
      if (arguments is! Map<Object?, Object?>) {
        throw const FormatException('Work action requires a map');
      }
      _workActions.add(WorkAction.fromMap(arguments));
      return;
    }
    if (call.method != 'protectionChanged') {
      throw MissingPluginException('Unknown keep-alive event: ${call.method}');
    }
    final arguments = call.arguments;
    if (arguments is! Map<Object?, Object?>) {
      throw const FormatException('Keep-alive event requires a host report');
    }
    _report.value = KeepAliveReport.fromMap(arguments);
  }
}
