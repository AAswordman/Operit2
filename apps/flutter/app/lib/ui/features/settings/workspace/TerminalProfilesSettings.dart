// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';

/// Edits device-owned profiles using fields registered by the active Core host.
class TerminalProfilesSettings extends StatefulWidget {
  /// Creates a terminal settings section backed by the selected Core connection.
  const TerminalProfilesSettings({super.key, required this.clients});

  final GeneratedCoreProxyClients clients;

  /// Creates the profile editor state.
  @override
  State<TerminalProfilesSettings> createState() =>
      _TerminalProfilesSettingsState();
}

class _TerminalProfilesSettingsState extends State<TerminalProfilesSettings> {
  TerminalProfileSettings? _settings;
  List<TerminalProfileSupport> _support = [];
  List<TerminalDistribution> _distributions = [];
  List<RuntimeTerminalInstall> _installs = [];
  Timer? _pollTimer;
  bool _polling = false;
  Object? _error;
  bool _saving = false;

  /// Loads the saved profiles and host-registered editing capabilities together.
  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  /// Replaces the visible snapshot only after both Core reads complete.
  Future<void> _load() async {
    try {
      final api = widget.clients.servicesRuntimeTerminalService;
      final (settings, support, distributions, installs) = await (
        api.terminalProfileSettings(),
        api.terminalProfileSupport(),
        api.terminalDistributions(),
        api.terminalDistributionInstalls(),
      ).wait;
      if (!mounted) return;
      setState(() {
        _settings = settings;
        _support = support;
        _distributions = distributions;
        _installs = installs;
        _error = null;
      });
      _schedulePoll();
    } catch (error) {
      if (mounted) setState(() => _error = error);
    }
  }

  /// Stops UI observation while Core-owned installation workers retain their lifetime.
  @override
  void dispose() {
    _pollTimer?.cancel();
    super.dispose();
  }

  /// Recognizes terminal job states from the shared installation protocol.
  bool _finished(RuntimeTerminalInstall job) =>
      const {'completed', 'failed', 'cancelled'}.contains(job.state);

  /// Polls only while a Core-owned job is active, including jobs started before reconnecting.
  void _schedulePoll() {
    _pollTimer?.cancel();
    if (_installs.any((job) => !_finished(job))) {
      _pollTimer = Timer.periodic(
        const Duration(seconds: 1),
        (_) => unawaited(_pollInstalls()),
      );
    }
  }

  /// Refreshes completed profiles without issuing overlapping polling requests.
  Future<void> _pollInstalls() async {
    if (_polling) return;
    _polling = true;
    try {
      final api = widget.clients.servicesRuntimeTerminalService;
      final installs = await api.terminalDistributionInstalls();
      final completed = installs.any(
        (job) =>
            job.state == 'completed' &&
            !_installs.any(
              (old) =>
                  old.installId == job.installId && old.state == 'completed',
            ),
      );
      final settings = completed ? await api.terminalProfileSettings() : null;
      if (!mounted) return;
      setState(() {
        _installs = installs;
        if (settings != null) _settings = settings;
        _error = null;
      });
      _schedulePoll();
    } catch (error) {
      if (mounted) setState(() => _error = error);
    } finally {
      _polling = false;
    }
  }

  /// Names a fresh instance so the same distribution can be installed more than once.
  Future<void> _download(TerminalDistribution distribution) async {
    final l10n = AppLocalizations.of(context)!;
    var instanceName = distribution.name;
    final name = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.terminalDistributionDownload),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('${distribution.name} · ${distribution.architecture}'),
            if (distribution.notice.isNotEmpty) ...[
              const SizedBox(height: 12),
              Text(distribution.notice),
            ],
            const SizedBox(height: 16),
            TextFormField(
              initialValue: distribution.name,
              onChanged: (value) => instanceName = value,
              decoration: InputDecoration(labelText: l10n.terminalProfileName),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, instanceName.trim()),
            child: Text(l10n.terminalDistributionDownload),
          ),
        ],
      ),
    );
    if (!mounted || name == null) return;
    try {
      final job = await widget.clients.servicesRuntimeTerminalService
          .startTerminalDistributionInstall(
            distributionId: distribution.id,
            name: name,
          );
      if (!mounted) return;
      setState(() => _installs = [..._installs, job]);
      _schedulePoll();
    } catch (error) {
      if (mounted) setState(() => _error = error);
    }
  }

  /// Requests cancellation and lets Core finish cleaning its private staging files.
  Future<void> _cancelInstall(RuntimeTerminalInstall job) async {
    try {
      await widget.clients.servicesRuntimeTerminalService
          .cancelTerminalDistributionInstall(installId: job.installId);
      await _pollInstalls();
    } catch (error) {
      if (mounted) setState(() => _error = error);
    }
  }

  /// Translates installation protocol phases into localized user-facing text.
  String _installState(AppLocalizations l10n, String state) => switch (state) {
    'queued' => l10n.terminalInstallQueued,
    'downloading' => l10n.terminalInstallDownloading,
    'verifying' => l10n.terminalInstallVerifying,
    'extracting' => l10n.terminalInstallExtracting,
    'completed' => l10n.terminalInstallCompleted,
    'cancelled' => l10n.terminalInstallCancelled,
    'failed' => l10n.terminalInstallFailed,
    _ => state,
  };

  /// Publishes a complete validated registry and retains the last saved UI state on failure.
  Future<void> _save(TerminalProfileSettings settings) async {
    setState(() => _saving = true);
    try {
      final saved = await widget.clients.servicesRuntimeTerminalService
          .saveTerminalProfileSettings(settings: settings);
      if (mounted) {
        setState(() {
          _settings = saved;
          _error = null;
        });
      }
    } catch (error) {
      if (mounted) setState(() => _error = error);
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  /// Opens one editable profile without changing the current default implicitly.
  Future<void> _edit([TerminalProfile? existing]) async {
    final profile = await showDialog<TerminalProfile>(
      context: context,
      builder: (context) =>
          _TerminalProfileDialog(profile: existing, support: _support),
    );
    if (!mounted || profile == null) return;
    final settings = _settings!;
    final profiles = [...settings.profiles];
    final index = profiles.indexWhere((entry) => entry.id == profile.id);
    if (index == -1) {
      profiles.add(profile);
    } else {
      profiles[index] = profile;
    }
    await _save(
      TerminalProfileSettings(
        revision: settings.revision,
        defaultProfileId: settings.defaultProfileId,
        profiles: profiles,
      ),
    );
  }

  /// Deletes a non-default profile after explaining that its running sessions stay open.
  Future<void> _delete(TerminalProfile profile) async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.terminalProfileDelete),
        content: Text(l10n.terminalProfileDeleteDescription),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(l10n.cancel),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(l10n.delete),
          ),
        ],
      ),
    );
    if (!mounted || confirmed != true) return;
    final settings = _settings!;
    await _save(
      TerminalProfileSettings(
        revision: settings.revision,
        defaultProfileId: settings.defaultProfileId,
        profiles: settings.profiles
            .where((entry) => entry.id != profile.id)
            .toList(),
      ),
    );
  }

  /// Shows saved profiles with explicit default, edit, and delete actions.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final settings = _settings;
    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l10n.terminalProfilesTitle,
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(l10n.terminalProfilesDescription),
            if (_error != null) ...[
              const SizedBox(height: 8),
              SelectableText(
                '$_error',
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
              TextButton(onPressed: _load, child: Text(l10n.retry)),
            ],
            if (settings == null && _error == null) const M3LoadingIndicator(),
            if (settings != null) ...[
              const SizedBox(height: 8),
              RadioGroup<String>(
                groupValue: settings.defaultProfileId,
                onChanged: (id) {
                  if (id != null && !_saving) {
                    unawaited(
                      _save(
                        TerminalProfileSettings(
                          revision: settings.revision,
                          defaultProfileId: id,
                          profiles: settings.profiles,
                        ),
                      ),
                    );
                  }
                },
                child: Column(
                  children: [
                    for (final profile in settings.profiles)
                      ListTile(
                        contentPadding: EdgeInsets.zero,
                        leading: Radio<String>(
                          value: profile.id,
                          enabled: !_saving,
                        ),
                        title: Text(profile.name),
                        subtitle: Text(
                          '${profile.terminal} / ${profile.terminalType}',
                        ),
                        trailing: Wrap(
                          children: [
                            IconButton(
                              tooltip: l10n.edit,
                              onPressed: _saving ? null : () => _edit(profile),
                              icon: const Icon(Icons.edit_outlined),
                            ),
                            IconButton(
                              tooltip: l10n.delete,
                              onPressed:
                                  _saving ||
                                      settings.defaultProfileId == profile.id
                                  ? null
                                  : () => _delete(profile),
                              icon: const Icon(Icons.delete_outline),
                            ),
                          ],
                        ),
                        onTap: _saving
                            ? null
                            : () => _save(
                                TerminalProfileSettings(
                                  revision: settings.revision,
                                  defaultProfileId: profile.id,
                                  profiles: settings.profiles,
                                ),
                              ),
                      ),
                  ],
                ),
              ),
              if (_saving) const LinearProgressIndicator(),
              const SizedBox(height: 8),
              OutlinedButton.icon(
                onPressed: _saving || _support.isEmpty ? null : _edit,
                icon: const Icon(Icons.add),
                label: Text(l10n.terminalProfileAdd),
              ),
              if (_distributions.isNotEmpty) ...[
                const Divider(height: 24),
                Text(
                  l10n.terminalDistributionsTitle,
                  style: Theme.of(context).textTheme.titleSmall,
                ),
                const SizedBox(height: 8),
                Text(l10n.terminalDistributionsDescription),
                for (final distribution in _distributions)
                  ListTile(
                    contentPadding: EdgeInsets.zero,
                    title: Text(distribution.name),
                    subtitle: Text(
                      [
                        '${distribution.architecture} · '
                            '${(distribution.downloadBytes / 1048576).toStringAsFixed(1)} MB',
                        if (distribution.notice.isNotEmpty) distribution.notice,
                      ].join('\n'),
                    ),
                    trailing: IconButton(
                      tooltip: l10n.terminalDistributionDownload,
                      onPressed: _saving ? null : () => _download(distribution),
                      icon: const Icon(Icons.download_outlined),
                    ),
                  ),
              ],
              for (final job in _installs)
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  title: Text(job.name),
                  subtitle: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(_installState(l10n, job.state)),
                      if (!_finished(job)) ...[
                        const SizedBox(height: 6),
                        LinearProgressIndicator(
                          value:
                              job.state == 'downloading' && job.totalBytes > 0
                              ? (job.downloadedBytes / job.totalBytes).clamp(
                                  0,
                                  1,
                                )
                              : null,
                        ),
                      ],
                      if (job.error != null) SelectableText(job.error!),
                    ],
                  ),
                  trailing: _finished(job)
                      ? null
                      : IconButton(
                          tooltip: l10n.cancel,
                          onPressed: () => _cancelInstall(job),
                          icon: const Icon(Icons.close),
                        ),
                ),
            ],
          ],
        ),
      ),
    );
  }
}

/// Edits a host-declared implementation without platform checks in Flutter.
class _TerminalProfileDialog extends StatefulWidget {
  /// Creates a dialog for a new profile or an existing saved profile.
  const _TerminalProfileDialog({this.profile, required this.support});
  final TerminalProfile? profile;
  final List<TerminalProfileSupport> support;

  /// Creates the form state.
  @override
  State<_TerminalProfileDialog> createState() => _TerminalProfileDialogState();
}

class _TerminalProfileDialogState extends State<_TerminalProfileDialog> {
  final _form = GlobalKey<FormState>();
  late final TextEditingController _name;
  final Map<String, TextEditingController> _options = {};
  late TerminalProfileSupport _implementation;

  /// Initializes the saved implementation and its registered fields.
  @override
  void initState() {
    super.initState();
    _name = TextEditingController(text: widget.profile?.name ?? '');
    _implementation = widget.profile == null
        ? widget.support.first
        : widget.support.firstWhere(
            (entry) =>
                entry.terminal == widget.profile!.terminal &&
                entry.terminalType == widget.profile!.terminalType,
          );
    for (final field
        in widget.support.expand((entry) => entry.fields).toSet()) {
      _options[field] = TextEditingController(
        text: widget.profile?.options[field] ?? '',
      );
    }
  }

  /// Releases all text controllers when the dialog closes.
  @override
  void dispose() {
    _name.dispose();
    for (final controller in _options.values) {
      controller.dispose();
    }
    super.dispose();
  }

  /// Validates structured arguments before returning the edited profile.
  void _submit() {
    if (!_form.currentState!.validate()) return;
    Navigator.pop(
      context,
      TerminalProfile(
        id:
            widget.profile?.id ??
            'user_${DateTime.now().microsecondsSinceEpoch}',
        name: _name.text.trim(),
        terminal: _implementation.terminal,
        terminalType: _implementation.terminalType,
        options: {
          for (final field in _implementation.fields)
            if (_options[field]!.text.trim().isNotEmpty)
              field: _options[field]!.text.trim(),
        },
      ),
    );
  }

  /// Labels the protocol field registered by the host.
  String _label(AppLocalizations l10n, String field) => switch (field) {
    'rootfs' => l10n.terminalProfileRootfs,
    'executable' => l10n.terminalProfileExecutable,
    'arguments' => l10n.terminalProfileArguments,
    'wslDistribution' => l10n.terminalProfileWslDistribution,
    _ => field,
  };

  /// Validates the JSON argument array without interpreting shell text.
  String? _validateOption(String field, String? text) {
    if (field != 'arguments' || text == null || text.trim().isEmpty) {
      return null;
    }
    try {
      final value = jsonDecode(text);
      if (value is List &&
          value.every(
            (entry) => entry is String && !entry.contains('\u0000'),
          )) {
        return null;
      }
    } on FormatException {
      /* Report malformed structured arguments below. */
    }
    return AppLocalizations.of(context)!.terminalProfileArgumentsError;
  }

  /// Builds a responsive form from the host's supported configuration fields.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(
        widget.profile == null
            ? l10n.terminalProfileAdd
            : l10n.terminalProfileEdit,
      ),
      scrollable: true,
      content: SizedBox(
        width: 480,
        child: Form(
          key: _form,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextFormField(
                controller: _name,
                decoration: InputDecoration(
                  labelText: l10n.terminalProfileName,
                ),
                validator: (value) => value == null || value.trim().isEmpty
                    ? l10n.terminalProfileNameRequired
                    : null,
              ),
              const SizedBox(height: 16),
              DropdownButtonFormField<String>(
                initialValue:
                    '${_implementation.terminal}/${_implementation.terminalType}',
                isExpanded: true,
                decoration: InputDecoration(
                  labelText: l10n.terminalProfileImplementation,
                ),
                items: [
                  for (final entry in widget.support)
                    DropdownMenuItem(
                      value: '${entry.terminal}/${entry.terminalType}',
                      child: Text('${entry.terminal} / ${entry.terminalType}'),
                    ),
                ],
                onChanged: (value) => setState(() {
                  _implementation = widget.support.firstWhere(
                    (entry) =>
                        '${entry.terminal}/${entry.terminalType}' == value,
                  );
                }),
              ),
              if (_implementation.fields.isEmpty)
                Padding(
                  padding: const EdgeInsets.only(top: 16),
                  child: Text(l10n.terminalProfileNoOptions),
                ),
              for (final field in _implementation.fields)
                Padding(
                  padding: const EdgeInsets.only(top: 16),
                  child: TextFormField(
                    controller: _options[field],
                    minLines: 1,
                    maxLines: field == 'arguments' ? 4 : 2,
                    decoration: InputDecoration(
                      labelText: _label(l10n, field),
                      helperText: switch (field) {
                        'rootfs' => l10n.terminalProfileRootfsHelp,
                        'arguments' => l10n.terminalProfileArgumentsHelp,
                        'wslDistribution' => l10n.terminalProfileWslHelp,
                        _ => null,
                      },
                      helperMaxLines: 5,
                    ),
                    validator: (value) => _validateOption(field, value),
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _submit, child: Text(l10n.save)),
      ],
    );
  }
}
