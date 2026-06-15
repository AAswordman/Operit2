// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/runtime/RemotePairingBridge.dart';
import '../../../../core/runtime/RuntimeConnectionManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class RuntimeSettingsPanel extends StatefulWidget {
  const RuntimeSettingsPanel({super.key, this.embedded = false});

  final bool embedded;

  @override
  State<RuntimeSettingsPanel> createState() => _RuntimeSettingsPanelState();
}

class _RuntimeSettingsPanelState extends State<RuntimeSettingsPanel> {
  bool _busy = false;
  String? _testMessage;
  bool _testFailed = false;

  RuntimeConnectionManager get _manager => RuntimeConnectionManager.instance;

  @override
  void initState() {
    super.initState();
    _manager.addListener(_syncFromManager);
  }

  @override
  void dispose() {
    _manager.removeListener(_syncFromManager);
    super.dispose();
  }

  void _syncFromManager() {
    if (mounted) {
      setState(() {});
    }
  }

  Future<void> _useLocal() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      await _manager.setLocal();
      if (mounted) {
        setState(() {
          _testMessage = l10n.settingsRuntimeSwitchedLocal;
          _testFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _testMessage = l10n.settingsRuntimeTestFailed(error.toString());
          _testFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _usePairedRemote(String name) async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      await _manager.usePairedRemote(name);
      if (mounted) {
        setState(() {
          _testMessage = l10n.settingsRuntimeSwitchedRemote;
          _testFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _testMessage = l10n.settingsRuntimeTestFailed(error.toString());
          _testFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _deletePairedRemote(String name) async {
    setState(() => _busy = true);
    try {
      await _manager.removePairedRemote(name);
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _testCurrent() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() {
      _busy = true;
      _testMessage = l10n.settingsRuntimeTesting;
      _testFailed = false;
    });
    try {
      final version = await const ProxyCoreRuntimeBridge().callApplication(
        'coreVersion',
      );
      if (mounted) {
        setState(() {
          _testMessage = l10n.settingsRuntimeTestResult(version.toString());
          _testFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _testMessage = l10n.settingsRuntimeTestFailed(error.toString());
          _testFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _pairRemote() async {
    final result = await _RemotePairDialog.show(context);
    if (result == null || !mounted) {
      return;
    }
    setState(() => _busy = true);
    try {
      await _manager.setRemote(name: result.name, session: result.session);
      if (mounted) {
        setState(() {
          _testMessage = AppLocalizations.of(
            context,
          )!.settingsRuntimeSwitchedRemote;
          _testFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _testMessage = AppLocalizations.of(
            context,
          )!.settingsRuntimeTestFailed(error.toString());
          _testFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final config = _manager.config;
    final localSelected = config.mode == RuntimeConnectionMode.local;
    final remoteSelected = config.mode == RuntimeConnectionMode.remote;
    final children = <Widget>[
      _SectionCard(
        title: l10n.settingsRuntimeConnection,
        children: <Widget>[
          LayoutBuilder(
            builder: (context, constraints) {
              final cards = <Widget>[
                _RuntimeModeCard(
                  icon: Icons.computer_outlined,
                  title: l10n.settingsRuntimeLocalTitle,
                  description: l10n.settingsRuntimeLocalDescription,
                  selected: localSelected,
                  actionLabel: l10n.settingsRuntimeUseLocal,
                  actionIcon: Icons.check_circle_outline,
                  onPressed: localSelected || _busy ? null : _useLocal,
                ),
                _RuntimeModeCard(
                  icon: Icons.cloud_outlined,
                  title: l10n.settingsRuntimeRemoteTitle,
                  description: l10n.settingsRuntimeRemoteDescription,
                  selected: remoteSelected,
                  actionLabel: l10n.settingsRuntimePairRemote,
                  actionIcon: Icons.add_link_outlined,
                  onPressed: _busy ? null : _pairRemote,
                  child: _RemoteSessionList(
                    config: config,
                    busy: _busy,
                    onUse: _usePairedRemote,
                    onDelete: _deletePairedRemote,
                  ),
                ),
              ];
              if (constraints.maxWidth >= 720) {
                return Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Expanded(child: cards[0]),
                    const SizedBox(width: 10),
                    Expanded(child: cards[1]),
                  ],
                );
              }
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  cards[0],
                  const SizedBox(height: 10),
                  cards[1],
                ],
              );
            },
          ),
          const SizedBox(height: 10),
          Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 10,
            runSpacing: 8,
            children: <Widget>[
              TextButton.icon(
                style: SettingsControlStyles.sectionTextButton(),
                onPressed: _busy ? null : _testCurrent,
                icon: const Icon(Icons.network_check_outlined, size: 18),
                label: Text(l10n.settingsRuntimeTestCurrent),
              ),
              if (_testMessage != null)
                _InlineStatus(message: _testMessage!, failed: _testFailed),
            ],
          ),
        ],
      ),
    ];
    if (widget.embedded) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: children,
      );
    }
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
      children: children,
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: SettingsControlStyles.sectionTitleTextStyle(context),
              ),
              const SizedBox(height: 8),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

class _RuntimeModeCard extends StatelessWidget {
  const _RuntimeModeCard({
    required this.icon,
    required this.title,
    required this.description,
    required this.selected,
    required this.actionLabel,
    required this.actionIcon,
    required this.onPressed,
    this.child,
  });

  final IconData icon;
  final String title;
  final String description;
  final bool selected;
  final String actionLabel;
  final IconData actionIcon;
  final VoidCallback? onPressed;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final borderColor = selected
        ? colorScheme.primary.withValues(alpha: 0.36)
        : colorScheme.outlineVariant.withValues(alpha: 0.18);
    final background = selected
        ? colorScheme.primaryContainer.withValues(alpha: 0.28)
        : colorScheme.surface.withValues(alpha: 0.24);
    return OperitGlassSurface(
      color: background,
      borderRadius: BorderRadius.circular(10),
      border: Border.all(color: borderColor),
      material: true,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 12, 12, 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(icon, size: 22, color: colorScheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w800,
                    ),
                  ),
                ),
                if (selected) SettingsActivePill(label: l10n.settingsActive),
              ],
            ),
            const SizedBox(height: 8),
            Text(
              description,
              style: theme.textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
            if (child != null) ...<Widget>[const SizedBox(height: 10), child!],
            const SizedBox(height: 10),
            TextButton.icon(
              style: SettingsControlStyles.sectionTextButton(),
              onPressed: onPressed,
              icon: Icon(actionIcon, size: 18),
              label: Text(actionLabel),
            ),
          ],
        ),
      ),
    );
  }
}

class _RemoteSessionList extends StatelessWidget {
  const _RemoteSessionList({
    required this.config,
    required this.busy,
    required this.onUse,
    required this.onDelete,
  });

  final RuntimeConnectionConfig config;
  final bool busy;
  final ValueChanged<String> onUse;
  final ValueChanged<String> onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final entries = config.remoteSessions.entries.toList(growable: false);
    if (entries.isEmpty) {
      return Text(
        l10n.settingsRuntimeNoPairedRemote,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      );
    }
    return Column(
      children: <Widget>[
        for (final entry in entries)
          _RemoteSessionTile(
            name: entry.key,
            session: entry.value,
            active:
                config.mode == RuntimeConnectionMode.remote &&
                config.activeRemoteName == entry.key,
            busy: busy,
            onUse: () => onUse(entry.key),
            onDelete: () => onDelete(entry.key),
          ),
      ],
    );
  }
}

class _RemoteSessionTile extends StatelessWidget {
  const _RemoteSessionTile({
    required this.name,
    required this.session,
    required this.active,
    required this.busy,
    required this.onUse,
    required this.onDelete,
  });

  final String name;
  final PairedRemoteSessionRecord session;
  final bool active;
  final bool busy;
  final VoidCallback onUse;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colorScheme.surface.withValues(alpha: 0.22),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: colorScheme.outlineVariant.withValues(alpha: 0.16),
          ),
        ),
        child: ListTile(
          dense: true,
          contentPadding: const EdgeInsets.symmetric(horizontal: 10),
          title: Text(
            session.remoteDeviceInfo.displayName,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          subtitle: Text(
            '$name · ${session.baseUrl}',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          trailing: Wrap(
            spacing: 2,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: <Widget>[
              if (active)
                SettingsActivePill(label: l10n.settingsActive)
              else
                SettingsSetActiveButton(
                  label: l10n.settingsActivate,
                  onPressed: busy ? null : onUse,
                ),
              IconButton(
                tooltip: l10n.delete,
                icon: const Icon(Icons.delete_outline),
                onPressed: busy ? null : onDelete,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _InlineStatus extends StatelessWidget {
  const _InlineStatus({required this.message, required this.failed});

  final String message;
  final bool failed;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Text(
      message,
      style: Theme.of(context).textTheme.bodySmall?.copyWith(
        color: failed ? colorScheme.error : colorScheme.primary,
        fontWeight: FontWeight.w700,
      ),
    );
  }
}

class _RemotePairDialog extends StatefulWidget {
  const _RemotePairDialog();

  static Future<_RemotePairResult?> show(BuildContext context) {
    return showDialog<_RemotePairResult>(
      context: context,
      builder: (context) => const _RemotePairDialog(),
    );
  }

  @override
  State<_RemotePairDialog> createState() => _RemotePairDialogState();
}

class _RemotePairDialogState extends State<_RemotePairDialog> {
  final TextEditingController _baseUrlController = TextEditingController();
  final TextEditingController _tokenController = TextEditingController();
  final TextEditingController _codeController = TextEditingController();
  RemotePairStartResult? _pairing;
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _baseUrlController.dispose();
    _tokenController.dispose();
    _codeController.dispose();
    super.dispose();
  }

  Future<void> _start() async {
    final l10n = AppLocalizations.of(context)!;
    final baseUrl = _baseUrlController.text.trim();
    final token = _tokenController.text.trim();
    if (baseUrl.isEmpty || token.isEmpty) {
      setState(() {
        _error =
            '${l10n.settingsRuntimeBaseUrl} / ${l10n.settingsRuntimePairToken}: ${l10n.required}';
      });
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final pairing = await const RemotePairingBridge().start(
        baseUrl: baseUrl,
        token: token,
      );
      if (mounted) {
        setState(() => _pairing = pairing);
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error.toString());
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _finish() async {
    final pairing = _pairing;
    if (pairing == null) {
      return;
    }
    final l10n = AppLocalizations.of(context)!;
    final pairingCode = _codeController.text.trim();
    if (pairingCode.isEmpty) {
      setState(() {
        _error = '${l10n.settingsRuntimePairCode}: ${l10n.required}';
      });
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final session = await const RemotePairingBridge().finish(
        pairingId: pairing.pairingId,
        pairingCode: pairingCode,
      );
      if (mounted) {
        Navigator.of(context).pop(
          _RemotePairResult(
            name: pairing.coreDeviceInfo.displayName,
            session: session,
          ),
        );
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error.toString());
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final pairing = _pairing;
    return AlertDialog(
      title: Text(l10n.settingsRuntimePairRemote),
      content: SizedBox(
        width: 460,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            TextField(
              controller: _baseUrlController,
              enabled: pairing == null,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimeBaseUrl,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _tokenController,
              enabled: pairing == null,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimePairToken,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            if (pairing != null) ...<Widget>[
              const SizedBox(height: 10),
              TextField(
                controller: _codeController,
                decoration: InputDecoration(
                  labelText: l10n.settingsRuntimePairCode,
                  border: const OutlineInputBorder(),
                  isDense: true,
                ),
              ),
            ],
            if (_error != null) ...<Widget>[
              const SizedBox(height: 10),
              Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
            ],
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _busy ? null : (pairing == null ? _start : _finish),
          child: Text(
            pairing == null
                ? l10n.settingsRuntimeStartPairing
                : l10n.settingsRuntimeFinishPairing,
          ),
        ),
      ],
    );
  }
}

class _RemotePairResult {
  const _RemotePairResult({required this.name, required this.session});

  final String name;
  final PairedRemoteSessionRecord session;
}
