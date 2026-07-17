// ignore_for_file: file_names

import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Describes one error that has stopped the current application session.
class FatalErrorReport {
  const FatalErrorReport({
    required this.source,
    required this.error,
    required this.stackTrace,
  });

  final String source;
  final Object error;
  final StackTrace? stackTrace;

  /// Formats the report for native crash views and clipboard export.
  String get details {
    final buffer = StringBuffer()
      ..writeln('Unhandled error source: $source')
      ..writeln()
      ..writeln(error);
    if (stackTrace != null) {
      buffer
        ..writeln()
        ..writeln('Dart stack trace:')
        ..writeln(stackTrace);
    }
    return buffer.toString();
  }
}

/// Owns the process-wide transition from the product UI to a fatal error view.
class UnhandledErrorReporter {
  UnhandledErrorReporter._();

  static final ValueNotifier<FatalErrorReport?> fatalError =
      ValueNotifier<FatalErrorReport?>(null);
  static bool _fatalErrorDeliveryScheduled = false;

  /// Records a fatal error for the active application container.
  static void report({
    required String source,
    required Object error,
    required StackTrace? stackTrace,
  }) {
    if (fatalError.value != null || _fatalErrorDeliveryScheduled) {
      return;
    }
    final report = FatalErrorReport(
      source: source,
      error: error,
      stackTrace: stackTrace,
    );
    _fatalErrorDeliveryScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _fatalErrorDeliveryScheduled = false;
      if (fatalError.value == null) {
        fatalError.value = report;
      }
    });
    WidgetsBinding.instance.scheduleFrame();
  }
}

/// Replaces the product tree after an unrecoverable application error.
class FatalErrorHost extends StatelessWidget {
  const FatalErrorHost({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<FatalErrorReport?>(
      valueListenable: UnhandledErrorReporter.fatalError,
      builder: (context, report, _) {
        if (report == null) {
          return child;
        }
        return FatalErrorScreen(report: report);
      },
    );
  }
}

/// Starts a minimal Flutter container when startup fails before the app tree exists.
class FatalErrorApplication extends StatelessWidget {
  const FatalErrorApplication({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'Operit2',
      home: ValueListenableBuilder<FatalErrorReport?>(
        valueListenable: UnhandledErrorReporter.fatalError,
        builder: (context, report, _) {
          if (report == null) {
            return const SizedBox.expand();
          }
          return FatalErrorScreen(report: report);
        },
      ),
    );
  }
}

/// Displays the final Flutter crash surface and asks supported native hosts to take over.
class FatalErrorScreen extends StatefulWidget {
  const FatalErrorScreen({required this.report, super.key});

  final FatalErrorReport report;

  @override
  State<FatalErrorScreen> createState() => _FatalErrorScreenState();
}

class _FatalErrorScreenState extends State<FatalErrorScreen> {
  static const MethodChannel _nativeCrashChannel = MethodChannel(
    'operit/crash',
  );

  @override
  void initState() {
    super.initState();
    unawaited(_presentNativeCrashScreen());
  }

  /// Invokes the platform's native crash surface where the host owns one.
  Future<void> _presentNativeCrashScreen() {
    if (kIsWeb ||
        !(Platform.isAndroid ||
            Platform.isWindows ||
            Platform.isLinux ||
            Platform.isIOS ||
            Platform.isMacOS ||
            Platform.operatingSystem == 'ohos')) {
      return Future<void>.value();
    }
    return _nativeCrashChannel.invokeMethod<void>('present', <String, Object>{
      'details': widget.report.details,
    });
  }

  /// Copies the diagnostic report without leaving the fatal screen.
  Future<void> _copyDetails() {
    return Clipboard.setData(ClipboardData(text: widget.report.details));
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Scaffold(
      backgroundColor: colorScheme.surface,
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 840),
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Icon(Icons.error_outline, color: colorScheme.error, size: 42),
                  const SizedBox(height: 16),
                  Text(
                    'Operit2 has stopped',
                    style: Theme.of(context).textTheme.headlineSmall,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'A fatal error prevented this session from continuing.',
                    style: Theme.of(context).textTheme.bodyLarge,
                  ),
                  const SizedBox(height: 20),
                  Expanded(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        border: Border.all(color: colorScheme.outlineVariant),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: SingleChildScrollView(
                        padding: const EdgeInsets.all(16),
                        child: SelectableText(
                          widget.report.details,
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(fontFamily: 'OperitTerminalMono'),
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  Align(
                    alignment: Alignment.centerRight,
                    child: FilledButton.icon(
                      onPressed: _copyDetails,
                      icon: const Icon(Icons.copy),
                      label: const Text('Copy details'),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
