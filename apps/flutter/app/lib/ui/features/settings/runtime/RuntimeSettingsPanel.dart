// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/runtime/RuntimeConnectionManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class RuntimeSettingsPanel extends StatefulWidget {
  const RuntimeSettingsPanel({super.key});

  @override
  State<RuntimeSettingsPanel> createState() => _RuntimeSettingsPanelState();
}

class _RuntimeSettingsPanelState extends State<RuntimeSettingsPanel> {
  final TextEditingController _nameController = TextEditingController();
  final TextEditingController _baseUrlController = TextEditingController();
  final TextEditingController _sessionIdController = TextEditingController();
  final TextEditingController _deviceIdController = TextEditingController();
  final TextEditingController _sessionSecretController =
      TextEditingController();
  bool _busy = false;

  RuntimeConnectionManager get _manager => RuntimeConnectionManager.instance;

  @override
  void initState() {
    super.initState();
    _manager.addListener(_syncFromManager);
    _syncFromManager();
  }

  @override
  void dispose() {
    _manager.removeListener(_syncFromManager);
    _nameController.dispose();
    _baseUrlController.dispose();
    _sessionIdController.dispose();
    _deviceIdController.dispose();
    _sessionSecretController.dispose();
    super.dispose();
  }

  void _syncFromManager() {
    final config = _manager.config;
    final session = config.remoteSession;
    _nameController.text = config.remoteName;
    _baseUrlController.text = session?.baseUrl ?? '';
    _sessionIdController.text = session?.sessionId ?? '';
    _deviceIdController.text = session?.deviceId ?? '';
    _sessionSecretController.text = session?.sessionSecret ?? '';
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
        _showMessage(l10n.settingsRuntimeSwitchedLocal);
      }
    } catch (error) {
      if (mounted) {
        _showMessage(l10n.settingsRuntimeTestFailed(error.toString()));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _useRemote() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      await _manager.setRemote(
        name: _nameController.text.trim(),
        session: PairedRemoteSessionRecord(
          baseUrl: _baseUrlController.text.trim(),
          sessionId: _sessionIdController.text.trim(),
          deviceId: _deviceIdController.text.trim(),
          sessionSecret: _sessionSecretController.text.trim(),
        ),
      );
      if (mounted) {
        _showMessage(l10n.settingsRuntimeSwitchedRemote);
      }
    } catch (error) {
      if (mounted) {
        _showMessage(l10n.settingsRuntimeTestFailed(error.toString()));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _testCurrent() async {
    final l10n = AppLocalizations.of(context)!;
    setState(() => _busy = true);
    try {
      final version = await const ProxyCoreRuntimeBridge().callApplication(
        'coreVersion',
      );
      if (mounted) {
        _showMessage(l10n.settingsRuntimeTestResult(version.toString()));
      }
    } catch (error) {
      if (mounted) {
        _showMessage(l10n.settingsRuntimeTestFailed(error.toString()));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  void _showMessage(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final config = _manager.config;
    final modeText = switch (config.mode) {
      RuntimeConnectionMode.local => l10n.settingsRuntimeLocalMode,
      RuntimeConnectionMode.remote => l10n.settingsRuntimeRemoteMode,
    };
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
      children: <Widget>[
        _SectionCard(
          title: l10n.settingsRuntimeConnection,
          children: <Widget>[
            Text(
              l10n.settingsRuntimeConnectionDescription,
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 10),
            _InfoLine(label: l10n.settingsRuntimeCurrentMode, value: modeText),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                FilledButton.icon(
                  style: SettingsControlStyles.sectionFilledButton(),
                  onPressed: _busy ? null : _useLocal,
                  icon: const Icon(Icons.computer_outlined, size: 18),
                  label: Text(l10n.settingsRuntimeUseLocal),
                ),
                TextButton.icon(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: _busy ? null : _testCurrent,
                  icon: const Icon(Icons.network_check_outlined, size: 18),
                  label: Text(l10n.settingsRuntimeTestCurrent),
                ),
              ],
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsRuntimeUseRemote,
          children: <Widget>[
            TextField(
              controller: _nameController,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimeRemoteName,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _baseUrlController,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimeBaseUrl,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _sessionIdController,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimeSessionId,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _deviceIdController,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimeDeviceId,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _sessionSecretController,
              obscureText: true,
              decoration: InputDecoration(
                labelText: l10n.settingsRuntimeSessionSecret,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 10),
            FilledButton.icon(
              style: SettingsControlStyles.sectionFilledButton(),
              onPressed: _busy ? null : _useRemote,
              icon: const Icon(Icons.hub_outlined, size: 18),
              label: Text(l10n.settingsRuntimeSaveRemote),
            ),
          ],
        ),
      ],
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

class _InfoLine extends StatelessWidget {
  const _InfoLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(label)),
          const SizedBox(width: 12),
          Text(value, style: TextStyle(color: colorScheme.onSurfaceVariant)),
        ],
      ),
    );
  }
}
