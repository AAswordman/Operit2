// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../l10n/generated/app_localizations.dart';

/// Shows the appearance error and offers only explicit retry and reset actions.
class ThemeAppearanceErrorScreen extends StatefulWidget {
  /// Receives repair operations without reading or replacing appearance during rendering.
  const ThemeAppearanceErrorScreen({
    super.key,
    required this.error,
    required this.onRetry,
    required this.onReset,
  });

  final Object error;
  final Future<void> Function() onRetry;
  final Future<void> Function() onReset;

  /// Owns the progress and operation errors for user-triggered repairs.
  @override
  State<ThemeAppearanceErrorScreen> createState() =>
      _ThemeAppearanceErrorScreenState();
}

class _ThemeAppearanceErrorScreenState
    extends State<ThemeAppearanceErrorScreen> {
  bool _busy = false;
  Object? _actionError;

  /// Runs exactly the requested repair and leaves failures visible to the user.
  Future<void> _run(Future<void> Function() action) async {
    setState(() {
      _busy = true;
      _actionError = null;
    });
    try {
      await action();
    } catch (error) {
      if (mounted) setState(() => _actionError = error);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  /// Keeps repair actions accessible even when the saved palette or font cannot render.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Scaffold(
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 560),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  l10n.settingsAppearanceLoadFailed,
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 12),
                Text(l10n.settingsAppearanceRepairHint),
                const SizedBox(height: 12),
                SelectableText(widget.error.toString()),
                if (_actionError != null) ...<Widget>[
                  const SizedBox(height: 12),
                  SelectableText(
                    _actionError.toString(),
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ],
                const SizedBox(height: 20),
                Wrap(
                  spacing: 12,
                  runSpacing: 8,
                  children: <Widget>[
                    OutlinedButton(
                      key: const ValueKey('appearance-retry'),
                      onPressed: _busy ? null : () => _run(widget.onRetry),
                      child: Text(l10n.settingsAppearanceRetry),
                    ),
                    FilledButton(
                      key: const ValueKey('appearance-reset'),
                      onPressed: _busy ? null : () => _run(widget.onReset),
                      child: Text(l10n.settingsAppearanceResetAfterError),
                    ),
                  ],
                ),
                if (_busy) ...<Widget>[
                  const SizedBox(height: 16),
                  const LinearProgressIndicator(),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}
