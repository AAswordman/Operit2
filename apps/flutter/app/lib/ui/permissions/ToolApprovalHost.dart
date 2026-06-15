// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../core/permissions/ToolApprovalBridge.dart';
import '../../core/permissions/ToolApprovalModels.dart';
import '../../l10n/generated/app_localizations.dart';

class ToolApprovalHost extends StatefulWidget {
  const ToolApprovalHost({
    super.key,
    required this.child,
    this.bridge = const ToolApprovalBridge(),
  });

  final Widget child;
  final ToolApprovalBridge bridge;

  @override
  State<ToolApprovalHost> createState() => _ToolApprovalHostState();
}

class _ToolApprovalHostState extends State<ToolApprovalHost> {
  Timer? _pollTimer;
  Timer? _clockTimer;
  ToolApprovalRequest? _request;
  bool _polling = false;

  @override
  void initState() {
    super.initState();
    _pollTimer = Timer.periodic(
      const Duration(milliseconds: 160),
      (_) => _pollRequest(),
    );
    _clockTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted && _request != null) {
        setState(() {});
      }
    });
    _pollRequest();
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    _clockTimer?.cancel();
    super.dispose();
  }

  Future<void> _pollRequest() async {
    if (_polling) {
      return;
    }
    _polling = true;
    try {
      final request = await widget.bridge.currentPermissionRequest();
      if (!mounted) {
        return;
      }
      final current = _request;
      if (!_sameRequest(current, request)) {
        setState(() {
          _request = request;
        });
      }
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'tool approval host',
          context: ErrorDescription('polling tool approval request'),
        ),
      );
    } finally {
      _polling = false;
    }
  }

  Future<void> _respond(ToolApprovalResult result) async {
    final request = _request;
    if (request == null) {
      return;
    }
    await widget.bridge.respondPermissionRequest(request, result);
    if (mounted) {
      setState(() {
        _request = null;
      });
    }
  }

  bool _sameRequest(ToolApprovalRequest? a, ToolApprovalRequest? b) {
    return a?.remoteRequestId == b?.remoteRequestId &&
        a?.requestedAtMillis == b?.requestedAtMillis &&
        a?.tool.name == b?.tool.name;
  }

  @override
  Widget build(BuildContext context) {
    final request = _request;
    return Stack(
      children: [
        widget.child,
        if (request != null)
          _ToolApprovalModal(request: request, onResult: _respond),
      ],
    );
  }
}

class _ToolApprovalModal extends StatelessWidget {
  const _ToolApprovalModal({required this.request, required this.onResult});

  final ToolApprovalRequest request;
  final ValueChanged<ToolApprovalResult> onResult;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final elapsedSeconds =
        ((DateTime.now().millisecondsSinceEpoch - request.requestedAtMillis) /
                1000)
            .floor()
            .clamp(0, 60);
    return Positioned.fill(
      child: Material(
        color: colorScheme.scrim.withValues(alpha: 0.36),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: Material(
              color: colorScheme.surfaceContainerHigh,
              elevation: 16,
              borderRadius: BorderRadius.circular(8),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(20, 18, 20, 16),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Icon(
                          Icons.verified_user_outlined,
                          color: colorScheme.primary,
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Text(
                            l10n.toolApprovalTitle,
                            style: Theme.of(context).textTheme.titleMedium,
                          ),
                        ),
                        Text(
                          '${elapsedSeconds}s / 60s',
                          style: Theme.of(context).textTheme.labelMedium
                              ?.copyWith(color: colorScheme.onSurfaceVariant),
                        ),
                      ],
                    ),
                    const SizedBox(height: 16),
                    _InfoLine(
                      label: l10n.toolApprovalToolLabel,
                      value: request.tool.name,
                    ),
                    const SizedBox(height: 8),
                    _InfoLine(
                      label: l10n.toolApprovalActionLabel,
                      value: request.description,
                    ),
                    if (request.tool.parameters.isNotEmpty) ...[
                      const SizedBox(height: 12),
                      _ParameterList(parameters: request.tool.parameters),
                    ],
                    const SizedBox(height: 18),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: [
                        TextButton.icon(
                          onPressed: () => onResult(ToolApprovalResult.deny),
                          icon: const Icon(Icons.close),
                          label: Text(l10n.toolApprovalDeny),
                        ),
                        const SizedBox(width: 8),
                        FilledButton.tonalIcon(
                          onPressed: () => onResult(ToolApprovalResult.allow),
                          icon: const Icon(Icons.check),
                          label: Text(l10n.toolApprovalAllowOnce),
                        ),
                        const SizedBox(width: 8),
                        FilledButton.icon(
                          onPressed: () =>
                              onResult(ToolApprovalResult.alwaysAllow),
                          icon: const Icon(Icons.done_all),
                          label: Text(l10n.toolApprovalAlwaysAllow),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _InfoLine extends StatelessWidget {
  const _InfoLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 44,
          child: Text(
            label,
            style: Theme.of(context).textTheme.labelLarge?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
        ),
        Expanded(
          child: SelectableText(
            value,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        ),
      ],
    );
  }
}

class _ParameterList extends StatelessWidget {
  const _ParameterList({required this.parameters});

  final List<ToolApprovalParameter> parameters;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      width: double.infinity,
      constraints: const BoxConstraints(maxHeight: 180),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainer,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: colorScheme.outlineVariant),
      ),
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (final parameter in parameters)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 3),
                child: Text.rich(
                  TextSpan(
                    children: [
                      TextSpan(
                        text: '${parameter.name}=',
                        style: TextStyle(color: colorScheme.primary),
                      ),
                      TextSpan(text: parameter.value),
                    ],
                  ),
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
