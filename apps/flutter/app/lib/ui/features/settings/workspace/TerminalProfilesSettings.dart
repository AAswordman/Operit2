// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/TerminalLabels.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../components/SettingsControlStyles.dart';
import 'WorkspaceSettingsSection.dart';

const _wslDistributionField = 'wslDistribution';

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
  final Map<String, RuntimeTerminalInstall> _installs = {};
  final Map<String, String> _installNames = {};
  final Map<String, Object> _installErrors = {};
  final Set<String> _startingInstalls = {};
  final Set<String> _cancellingInstalls = {};
  Timer? _pollTimer;
  bool _polling = false;
  Object? _progressError;
  Object? _error;
  bool _saving = false;
  bool _openingEditor = false;

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
        for (final install in installs) {
          _installs[install.distributionId] = install;
        }
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
    if (_installs.values.any((job) => !_finished(job))) {
      _pollTimer ??= Timer.periodic(
        const Duration(milliseconds: 500),
        (_) => unawaited(_pollInstalls()),
      );
    } else {
      _pollTimer?.cancel();
      _pollTimer = null;
    }
  }

  /// Refreshes completed profiles without issuing overlapping polling requests.
  Future<void> _pollInstalls() async {
    if (_polling) return;
    _polling = true;
    final observed = Map.of(_installs);
    try {
      final api = widget.clients.servicesRuntimeTerminalService;
      final installs = await api.terminalDistributionInstalls();
      final latest = {for (final job in installs) job.distributionId: job};
      final completed = latest.entries.any((entry) {
        final previous = observed[entry.key];
        return entry.value.state == 'completed' &&
            (previous?.installId != entry.value.installId ||
                previous?.state != 'completed');
      });
      final settings = completed ? await api.terminalProfileSettings() : null;
      if (!mounted) return;
      setState(() {
        for (final entry in latest.entries) {
          if (_installs[entry.key]?.installId ==
                  observed[entry.key]?.installId ||
              _installs[entry.key]?.installId == entry.value.installId) {
            _installs[entry.key] = entry.value;
            if (_finished(entry.value)) {
              _cancellingInstalls.remove(entry.value.installId);
            }
          }
        }
        if (settings != null && settings.revision >= _settings!.revision) {
          _settings = settings;
        }
        _progressError = null;
      });
      _schedulePoll();
    } catch (error) {
      if (mounted) setState(() => _progressError = error);
    } finally {
      _polling = false;
    }
  }

  /// Names a fresh instance so the same distribution can be installed more than once.
  Future<void> _download(TerminalDistribution distribution) async {
    final l10n = AppLocalizations.of(context)!;
    final form = GlobalKey<FormState>();
    var instanceName = distribution.name;
    final name = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(distribution.name),
        scrollable: true,
        content: SizedBox(
          width: 420,
          child: Form(
            key: form,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(_distributionDetails(distribution)),
                if (distribution.notice.isNotEmpty) ...[
                  const SizedBox(height: 12),
                  Text(distribution.notice),
                ],
                const SizedBox(height: 16),
                TextFormField(
                  initialValue: distribution.name,
                  onChanged: (value) => instanceName = value,
                  decoration: InputDecoration(
                    labelText: l10n.terminalProfileName,
                  ),
                  validator: (value) => value == null || value.trim().isEmpty
                      ? l10n.terminalProfileNameRequired
                      : null,
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
          FilledButton(
            onPressed: () {
              if (form.currentState!.validate()) {
                Navigator.pop(context, instanceName.trim());
              }
            },
            child: Text(l10n.terminalDistributionDownload),
          ),
        ],
      ),
    );
    if (!mounted || name == null) return;
    await _startInstall(distribution.id, name);
  }

  /// Starts an attempt and retains its name so retry needs no dialog.
  Future<void> _startInstall(String distributionId, String name) async {
    if (_startingInstalls.contains(distributionId)) return;
    setState(() {
      _installNames[distributionId] = name;
      _installErrors.remove(distributionId);
      _startingInstalls.add(distributionId);
    });
    try {
      final job = await widget.clients.servicesRuntimeTerminalService
          .startTerminalDistributionInstall(
            distributionId: distributionId,
            name: name,
          );
      if (!mounted) return;
      setState(() => _installs[distributionId] = job);
      _schedulePoll();
      unawaited(_pollInstalls());
    } catch (error) {
      if (mounted) setState(() => _installErrors[distributionId] = error);
    } finally {
      if (mounted) setState(() => _startingInstalls.remove(distributionId));
    }
  }

  /// Displays the architecture and download size on both the row and dialog.
  String _distributionDetails(TerminalDistribution distribution) =>
      '${distribution.architecture} · '
      '${_formatMiB(distribution.downloadBytes)} MiB';

  /// Formats byte counts consistently for the catalog and in-flight progress.
  String _formatMiB(int bytes) => (bytes / 1048576).toStringAsFixed(1);

  /// Requests cancellation and lets Core finish cleaning its private staging files.
  Future<void> _cancelInstall(RuntimeTerminalInstall job) async {
    setState(() => _cancellingInstalls.add(job.installId));
    try {
      await widget.clients.servicesRuntimeTerminalService
          .cancelTerminalDistributionInstall(installId: job.installId);
      await _pollInstalls();
    } catch (error) {
      if (mounted)
        setState(() {
          _cancellingInstalls.remove(job.installId);
          _progressError = error;
        });
    }
  }

  /// Translates installation protocol phases into localized user-facing text.
  String _installState(AppLocalizations l10n, String state) => switch (state) {
    'queued' => l10n.terminalInstallQueued,
    'connecting' => l10n.terminalInstallConnecting,
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
    if (_openingEditor) return;
    _openingEditor = true;
    try {
      final support = await widget.clients.servicesRuntimeTerminalService
          .terminalProfileSupport();
      if (!mounted) return;
      final profile = await showDialog<TerminalProfile>(
        context: context,
        builder: (context) =>
            _TerminalProfileDialog(profile: existing, support: support),
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
          schemaVersion: settings.schemaVersion,
          revision: settings.revision,
          defaultProfileId: settings.defaultProfileId,
          profiles: profiles,
        ),
      );
    } catch (error) {
      if (mounted) setState(() => _error = error);
    } finally {
      _openingEditor = false;
    }
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
        schemaVersion: settings.schemaVersion,
        revision: settings.revision,
        defaultProfileId: settings.defaultProfileId,
        profiles: settings.profiles
            .where((entry) => entry.id != profile.id)
            .toList(),
      ),
    );
  }

  /// Saves a different default without modifying any launch profiles.
  void _selectDefault(TerminalProfileSettings settings, String? id) {
    if (id == null || id == settings.defaultProfileId || _saving) return;
    unawaited(
      _save(
        TerminalProfileSettings(
          schemaVersion: settings.schemaVersion,
          revision: settings.revision,
          defaultProfileId: id,
          profiles: settings.profiles,
        ),
      ),
    );
  }

  /// Distinguishes a WSL distribution from a native Bash profile in the list.
  String _profileDescription(AppLocalizations l10n, TerminalProfile profile) {
    final distribution = profile.options[_wslDistributionField];
    return distribution == null
        ? TerminalLabels.implementation(
            l10n,
            profile.terminal,
            profile.terminalType,
          )
        : 'WSL · $distribution';
  }

  /// Shows saved profiles with explicit default, edit, and delete actions.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final settings = _settings;
    final colors = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        WorkspaceSettingsSection(
          title: l10n.terminalProfilesTitle,
          description: l10n.terminalProfilesDescription,
          children: [
            if (_error != null) ...[
              SelectableText('$_error', style: TextStyle(color: colors.error)),
              Align(
                alignment: AlignmentDirectional.centerStart,
                child: TextButton(onPressed: _load, child: Text(l10n.retry)),
              ),
            ],
            if (settings == null && _error == null)
              const Center(child: M3LoadingIndicator()),
            if (settings != null) ...[
              RadioGroup<String>(
                groupValue: settings.defaultProfileId,
                onChanged: (id) => _selectDefault(settings, id),
                child: _TerminalSettingsRows(
                  rows: [
                    for (final profile in settings.profiles)
                      ListTile(
                        contentPadding: const EdgeInsetsDirectional.only(
                          end: 8,
                        ),
                        minLeadingWidth: 44,
                        horizontalTitleGap: 0,
                        leading: Radio<String>(
                          value: profile.id,
                          enabled: !_saving,
                        ),
                        title: Text(TerminalLabels.profileName(l10n, profile)),
                        subtitle: Text(_profileDescription(l10n, profile)),
                        trailing: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            IconButton(
                              style: SettingsControlStyles.entityIconButton(),
                              tooltip: l10n.edit,
                              onPressed: _saving ? null : () => _edit(profile),
                              icon: const Icon(Icons.edit_outlined),
                            ),
                            IconButton(
                              style: SettingsControlStyles.entityIconButton(),
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
                            : () => _selectDefault(settings, profile.id),
                      ),
                  ],
                ),
              ),
              if (_saving) const LinearProgressIndicator(),
              const SizedBox(height: 10),
              Align(
                alignment: AlignmentDirectional.centerStart,
                child: FilledButton.tonalIcon(
                  style: SettingsControlStyles.sectionFilledButton(),
                  onPressed: _saving || _support.isEmpty ? null : _edit,
                  icon: const Icon(Icons.add, size: 18),
                  label: Text(l10n.terminalProfileAdd),
                ),
              ),
            ],
          ],
        ),
        if (_distributions.isNotEmpty)
          WorkspaceSettingsSection(
            title: l10n.terminalDistributionsTitle,
            children: [
              if (_progressError != null)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: SelectableText(
                    '$_progressError',
                    style: TextStyle(color: colors.error),
                  ),
                ),
              _TerminalSettingsRows(
                rows: [
                  for (final distribution in _distributions)
                    _distributionRow(l10n, distribution),
                ],
              ),
            ],
          ),
      ],
    );
  }

  /// Keeps download progress and actions on the existing distribution row.
  Widget _distributionRow(
    AppLocalizations l10n,
    TerminalDistribution distribution,
  ) {
    final id = distribution.id;
    final job = _installs[id];
    final starting = _startingInstalls.contains(id);
    final active = job != null && !_finished(job);
    final cancelling = active && _cancellingInstalls.contains(job.installId);
    final requestError = _installErrors[id];
    final canRetry =
        requestError != null ||
        job?.state == 'failed' ||
        job?.state == 'cancelled';
    final error =
        requestError?.toString() ??
        (job?.state == 'failed' ? job?.error : null);
    final fraction =
        !starting &&
            !cancelling &&
            job != null &&
            job.state == 'downloading' &&
            job.totalBytes > 0
        ? (job.downloadedBytes / job.totalBytes).clamp(0.0, 1.0)
        : null;
    final theme = Theme.of(context);
    final status = starting
        ? l10n.terminalInstallQueued
        : cancelling
        ? l10n.terminalInstallCancelling
        : requestError != null
        ? l10n.terminalInstallFailed
        : job == null
        ? null
        : _installState(l10n, job.state);
    final instanceName = _installNames[id] ?? job?.name;
    return Padding(
      key: ValueKey('terminal-distribution-$id'),
      padding: const EdgeInsets.fromLTRB(14, 12, 12, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                Icons.terminal_rounded,
                color: theme.colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 12),
              Expanded(
                flex: 3,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(distribution.name, style: theme.textTheme.titleSmall),
                    Text(
                      _distributionDetails(distribution),
                      style: theme.textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              if (starting || active)
                TextButton(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: active && !starting && !cancelling
                      ? () => _cancelInstall(job)
                      : null,
                  child: Text(l10n.cancel),
                )
              else if (canRetry && instanceName != null)
                FilledButton.tonal(
                  style: SettingsControlStyles.sectionFilledButton(),
                  onPressed: () => _startInstall(id, instanceName),
                  child: Text(l10n.retry),
                )
              else
                Expanded(
                  flex: 2,
                  child: Align(
                    alignment: AlignmentDirectional.centerEnd,
                    child: TextButton(
                      style: SettingsControlStyles.sectionTextButton(),
                      onPressed: () => _download(distribution),
                      child: Text(
                        job?.state == 'completed'
                            ? l10n.terminalDistributionInstallAnother
                            : l10n.terminalDistributionDownload,
                        textAlign: TextAlign.center,
                      ),
                    ),
                  ),
                ),
            ],
          ),
          if (starting || active) ...[
            const SizedBox(height: 10),
            LinearProgressIndicator(
              value: fraction,
              semanticsLabel: status,
              borderRadius: BorderRadius.circular(4),
            ),
          ],
          if (status != null) ...[
            const SizedBox(height: 8),
            Wrap(
              spacing: 10,
              runSpacing: 2,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Text(
                  status,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: error == null
                        ? theme.colorScheme.onSurfaceVariant
                        : theme.colorScheme.error,
                  ),
                ),
                if (fraction != null)
                  Text(
                    '${(fraction * 100).floor()}% · ${_formatMiB(job!.downloadedBytes)} / ${_formatMiB(job.totalBytes)} MiB',
                    style: theme.textTheme.bodySmall,
                  ),
              ],
            ),
          ],
          if (error != null) ...[
            const SizedBox(height: 4),
            TextButton(
              style: SettingsControlStyles.sectionTextButton(),
              onPressed: () => showDialog<void>(
                context: context,
                builder: (context) => AlertDialog(
                  title: Text(l10n.terminalInstallFailed),
                  scrollable: true,
                  content: SelectableText(error),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.pop(context),
                      child: Text(l10n.close),
                    ),
                  ],
                ),
              ),
              child: Text(l10n.terminalInstallErrorDetails),
            ),
          ],
        ],
      ),
    );
  }
}

/// Groups related terminal controls inside a workspace settings section.
class _TerminalSettingsRows extends StatelessWidget {
  /// Creates a compact group of settings rows.
  const _TerminalSettingsRows({required this.rows});

  final List<Widget> rows;

  /// Applies the settings inset surface and separators to the supplied rows.
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Material(
      color: colors.surfaceContainerHighest.withValues(alpha: 0.30),
      borderRadius: BorderRadius.circular(12),
      clipBehavior: Clip.antiAlias,
      child: Column(
        children: [
          for (final (index, row) in rows.indexed) ...[
            if (index != 0)
              Divider(height: 1, color: colors.surface.withValues(alpha: 0.75)),
            row,
          ],
        ],
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
  String? _initialDisplayName;
  final Map<String, TextEditingController> _options = {};
  final List<_MountDraft> _mounts = [];
  late TerminalProfileSupport _implementation;
  late bool _usesWsl;
  String? _distribution;

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
    _distribution = widget.profile?.options[_wslDistributionField];
    _usesWsl = _distribution != null;
    final mounts = widget.profile?.options['mounts'];
    if (mounts != null) {
      for (final entry in jsonDecode(mounts) as List) {
        _mounts.add(
          _MountDraft(entry['source'] as String, entry['target'] as String),
        );
      }
    }
    for (final field
        in widget.support.expand((entry) => entry.fields).toSet()) {
      if (field == _wslDistributionField || field == 'mounts') continue;
      _options[field] = TextEditingController(
        text: widget.profile?.options[field] ?? '',
      );
    }
  }

  /// Exposes custom launcher overrides only when the existing WSL profile has one.
  Iterable<String> get _launchFields => _implementation.fields.where(
    (field) =>
        field != _wslDistributionField &&
        field != 'mounts' &&
        (!_usesWsl || widget.profile?.options.containsKey(field) == true),
  );

  /// Exposes mounts only when the selected launcher advertises native mount support.
  bool get _supportsMounts => _implementation.fields.contains('mounts');

  /// Shows the translated default while retaining its stored name on an unchanged save.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_initialDisplayName != null) return;
    final profile = widget.profile;
    _initialDisplayName = profile == null
        ? ''
        : TerminalLabels.profileName(AppLocalizations.of(context)!, profile);
    _name.text = _initialDisplayName!;
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
        name: widget.profile != null && _name.text == _initialDisplayName
            ? widget.profile!.name
            : _name.text.trim(),
        nameKey:
            _name.text == _initialDisplayName &&
                _implementation.terminal == widget.profile?.terminal &&
                _implementation.terminalType == widget.profile?.terminalType &&
                _distribution == widget.profile?.options[_wslDistributionField]
            ? widget.profile?.nameKey
            : null,
        terminal: _implementation.terminal,
        terminalType: _implementation.terminalType,
        options: {
          for (final field in _launchFields)
            if (_options[field]!.text.trim().isNotEmpty)
              field: _options[field]!.text.trim(),
          if (_usesWsl) _wslDistributionField: _distribution!,
          if (_supportsMounts && _mounts.isNotEmpty)
            'mounts': jsonEncode([
              for (final mount in _mounts)
                {'source': mount.source.trim(), 'target': mount.target.trim()},
            ]),
        },
      ),
    );
  }

  /// Labels the protocol field registered by the host.
  String _label(AppLocalizations l10n, String field) => switch (field) {
    'rootfs' => l10n.terminalProfileRootfs,
    'executable' => l10n.terminalProfileExecutable,
    'arguments' => l10n.terminalProfileArguments,
    _ => field,
  };

  /// Validates the JSON argument array without interpreting shell text.
  String? _validateOption(String field, String? text) {
    if (field == 'rootfs' &&
        _implementation.terminal == 'chroot' &&
        (text == null || text.trim().isEmpty)) {
      return AppLocalizations.of(context)!.terminalProfileRootfsRequired;
    }
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
    final distributions =
        _implementation.fieldChoices?[_wslDistributionField] ??
        const <String>[];
    final distributionOptions = {
      ...distributions,
      if (_distribution != null) _distribution!,
    };
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
              DropdownButtonFormField<(TerminalProfileSupport, bool)>(
                initialValue: (_implementation, _usesWsl),
                isExpanded: true,
                decoration: InputDecoration(
                  labelText: l10n.terminalProfileImplementation,
                ),
                items: [
                  for (final entry in widget.support)
                    DropdownMenuItem(
                      value: (entry, false),
                      child: Text(
                        TerminalLabels.implementation(
                          l10n,
                          entry.terminal,
                          entry.terminalType,
                        ),
                      ),
                    ),
                  for (final entry in widget.support)
                    if (entry.fields.contains(_wslDistributionField))
                      DropdownMenuItem(
                        value: (entry, true),
                        child: const Text('WSL'),
                      ),
                ],
                onChanged: (value) {
                  if (value == null) return;
                  setState(() {
                    _implementation = value.$1;
                    _usesWsl = value.$2;
                  });
                },
              ),
              if (_usesWsl)
                Padding(
                  padding: const EdgeInsets.only(top: 16),
                  child: DropdownButtonFormField<String>(
                    initialValue: _distribution,
                    isExpanded: true,
                    decoration: InputDecoration(
                      labelText: l10n.terminalProfileWslDistribution,
                      helperText: distributionOptions.isEmpty
                          ? l10n.terminalProfileWslHelp
                          : null,
                      helperMaxLines: 3,
                    ),
                    items: [
                      for (final distribution in distributionOptions)
                        DropdownMenuItem(
                          value: distribution,
                          child: Text(distribution),
                        ),
                    ],
                    onChanged: distributionOptions.isEmpty
                        ? null
                        : (value) {
                            if (value == null) return;
                            if (_name.text.trim().isEmpty ||
                                _name.text == _distribution) {
                              _name.text = value;
                            }
                            _distribution = value;
                          },
                    validator: (value) => value == null
                        ? l10n.terminalProfileWslDistributionRequired
                        : null,
                  ),
                ),
              if (!_usesWsl && _launchFields.isEmpty && !_supportsMounts)
                Padding(
                  padding: const EdgeInsets.only(top: 16),
                  child: Text(l10n.terminalProfileNoOptions),
                ),
              for (final field in _launchFields)
                Padding(
                  key: ValueKey(field),
                  padding: const EdgeInsets.only(top: 16),
                  child: TextFormField(
                    controller: _options[field],
                    minLines: 1,
                    maxLines: field == 'arguments' ? 4 : 2,
                    decoration: InputDecoration(
                      labelText: _label(l10n, field),
                      helperText: switch (field) {
                        'rootfs' when _implementation.terminal == 'chroot' =>
                          l10n.terminalProfileChrootRootfsHelp,
                        'rootfs' => l10n.terminalProfileRootfsHelp,
                        'arguments' => l10n.terminalProfileArgumentsHelp,
                        _ => null,
                      },
                      helperMaxLines: 5,
                    ),
                    validator: (value) => _validateOption(field, value),
                  ),
                ),
              if (_supportsMounts) _mountEditor(l10n),
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

  /// Renders editable source and target pairs without exposing serialized options.
  Widget _mountEditor(AppLocalizations l10n) {
    return Padding(
      padding: const EdgeInsets.only(top: 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l10n.terminalProfileMounts,
            style: Theme.of(context).textTheme.titleSmall,
          ),
          for (final mount in _mounts)
            Padding(
              key: ObjectKey(mount),
              padding: const EdgeInsets.only(top: 8),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: Column(
                      children: [
                        TextFormField(
                          initialValue: mount.source,
                          decoration: InputDecoration(
                            labelText: l10n.terminalProfileMountSource,
                          ),
                          onChanged: (value) => mount.source = value,
                          validator: _validateMountPath,
                        ),
                        const SizedBox(height: 8),
                        TextFormField(
                          initialValue: mount.target,
                          decoration: InputDecoration(
                            labelText: l10n.terminalProfileMountTarget,
                          ),
                          onChanged: (value) => mount.target = value,
                          validator: _validateMountPath,
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    tooltip: l10n.delete,
                    onPressed: () => setState(() => _mounts.remove(mount)),
                    icon: const Icon(Icons.remove_circle_outline),
                  ),
                ],
              ),
            ),
          TextButton.icon(
            onPressed: () => setState(() => _mounts.add(_MountDraft('', ''))),
            icon: const Icon(Icons.add, size: 18),
            label: Text(l10n.terminalProfileAddMount),
          ),
        ],
      ),
    );
  }

  /// Leaves platform-specific path checks to the terminal host.
  String? _validateMountPath(String? value) =>
      value == null || value.trim().isEmpty
      ? AppLocalizations.of(context)!.terminalProfileMountPathRequired
      : null;
}

/// Stores one in-progress directory mapping while the configuration dialog is open.
class _MountDraft {
  /// Initializes a saved mount or an empty row.
  _MountDraft(this.source, this.target);

  String source;
  String target;
}
