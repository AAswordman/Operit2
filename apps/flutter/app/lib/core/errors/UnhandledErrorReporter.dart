// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

class UnhandledErrorReporter {
  UnhandledErrorReporter._();

  static final GlobalKey<NavigatorState> navigatorKey =
      GlobalKey<NavigatorState>();

  static bool _dialogOpen = false;
  static bool _presentScheduled = false;
  static final List<String> _pendingDetails = <String>[];

  static void report({
    required String source,
    required Object error,
    required StackTrace? stackTrace,
  }) {
    _pendingDetails.add(_formatDetails(source, error, stackTrace));
    _schedulePresent();
  }

  static void _schedulePresent() {
    if (_presentScheduled) {
      return;
    }
    _presentScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _presentScheduled = false;
      _presentPending();
    });
  }

  static void _presentPending() {
    final context = navigatorKey.currentContext;
    if (context == null) {
      if (_pendingDetails.isNotEmpty) {
        _schedulePresent();
      }
      return;
    }
    if (_dialogOpen) {
      return;
    }
    if (_pendingDetails.isEmpty) {
      return;
    }
    final details = _pendingDetails.removeAt(0);
    _dialogOpen = true;
    showDialog<void>(
      context: context,
      builder: (context) => _UnhandledErrorDialog(details: details),
    ).whenComplete(() {
      _dialogOpen = false;
      if (_pendingDetails.isNotEmpty) {
        _schedulePresent();
      }
    });
  }

  static String _formatDetails(
    String source,
    Object error,
    StackTrace? stackTrace,
  ) {
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

class _UnhandledErrorDialog extends StatelessWidget {
  const _UnhandledErrorDialog({required this.details});

  final String details;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return AlertDialog(
      title: const Text('未捕获异常 / Unhandled error'),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 720, maxHeight: 460),
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.45),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: colorScheme.outlineVariant),
          ),
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(12),
            child: SelectableText(
              details,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                fontFamily: 'OperitTerminalMono',
                color: colorScheme.onSurface,
              ),
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton.icon(
          onPressed: () async {
            await Clipboard.setData(ClipboardData(text: details));
            if (!context.mounted) {
              return;
            }
            ScaffoldMessenger.maybeOf(
              context,
            )?.showSnackBar(const SnackBar(content: Text('已复制 / Copied')));
          },
          icon: const Icon(Icons.copy, size: 18),
          label: const Text('复制 / Copy'),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('关闭 / Close'),
        ),
      ],
    );
  }
}
