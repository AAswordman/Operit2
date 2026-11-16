// ignore_for_file: file_names

import 'package:flutter/foundation.dart' show AsyncCallback;
import 'package:flutter/material.dart';

import '../../../../../../core/host/AppToastController.dart';
import '../../../../../../core/link/CoreLinkProtocol.dart';
import '../../../../../../core/logging/ClientLogger.dart';
import '../../../../../../l10n/generated/app_localizations.dart';

typedef WorkspaceExternalOpenRunner =
    Future<void> Function(AsyncCallback action);

typedef WorkspaceExternalOpenBuilder =
    Widget Function(
      BuildContext context,
      bool opening,
      WorkspaceExternalOpenRunner run,
    );

/// Retains error structure/correlation while keeping file capabilities out of diagnostics.
String _redactFileCapabilities(String text) => text
    .replaceAll(RegExp(r'''(?:content|file)://[^\s"'<>]+'''), '[file URI]')
    .replaceAllMapped(
      RegExp(
        r'''(^|[\s"'=(\[{])(?:[A-Za-z]:[\\/]|/)[^\s"'<>]+''',
        multiLine: true,
      ),
      (match) => '${match[1]}[path]',
    )
    .replaceAll(RegExp(r'\b[0-9a-fA-F]{32}\b'), '[file token]');

/// Awaits one file-open action and presents failures beside its initiating UI.
class WorkspaceExternalOpenAction extends StatefulWidget {
  const WorkspaceExternalOpenAction({
    super.key,
    required this.identity,
    required this.builder,
  });

  /// Changes when this control starts referring to a different file or owner.
  final Object identity;
  final WorkspaceExternalOpenBuilder builder;

  @override
  State<WorkspaceExternalOpenAction> createState() =>
      _WorkspaceExternalOpenActionState();
}

class _WorkspaceExternalOpenActionState
    extends State<WorkspaceExternalOpenAction> {
  bool _opening = false;
  int _generation = 0;

  @override
  void didUpdateWidget(covariant WorkspaceExternalOpenAction oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.identity != widget.identity) {
      _generation++;
      _opening = false;
    }
  }

  @override
  void dispose() {
    // Invalidates presentation only; an Android task already dispatched keeps
    // running until its host result arrives. This does not cancel that task.
    _generation++;
    super.dispose();
  }

  Future<void> _run(AsyncCallback action, int generation) async {
    if (!mounted || generation != _generation || _opening) {
      return;
    }
    setState(() => _opening = true);
    try {
      await action();
    } on Object catch (error, stackTrace) {
      final diagnostic = _redactFileCapabilities(
        error is CoreLinkError ? error.toDiagnosticString() : error.toString(),
      );
      final safeStackTrace = StackTrace.fromString(
        _redactFileCapabilities(stackTrace.toString()),
      );
      if (ClientLogger.isInitialized) {
        ClientLogger.e(
          'External workspace file open failed',
          tag: 'WorkspaceFileOpen',
          error: diagnostic,
          stackTrace: safeStackTrace,
        );
      } else {
        debugPrint('WorkspaceFileOpen: $diagnostic\n$safeStackTrace');
      }
      if (mounted && generation == _generation) {
        final l10n = AppLocalizations.of(context)!;
        final timedOut =
            error is CoreLinkError &&
            error.code == 'COMMAND_ERROR' &&
            (error.message == 'host interaction timed out' ||
                (error.message.startsWith('host interaction timed out: ') &&
                    error.message
                        .substring('host interaction timed out: '.length)
                        .trim()
                        .isNotEmpty));
        AppToastController.instance.show(
          timedOut
              ? l10n.workspaceFileOpenTimedOut
              : l10n.workspaceFileOperationFailed(
                  _redactFileCapabilities(
                    error is CoreLinkError ? error.message : error.toString(),
                  ),
                ),
        );
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _opening = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final generation = _generation;
    return widget.builder(
      context,
      _opening,
      (action) => _run(action, generation),
    );
  }
}
