// ignore_for_file: file_names

import 'dart:math';
import 'package:flutter/material.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../../../l10n/generated/app_localizations.dart';

/// Edits a provider's draft key pool without writing application preferences.
class ApiKeyPoolEditor extends StatefulWidget {
  /// Creates the pool controls and an optional explicit credential probe.
  const ApiKeyPoolEditor({
    super.key,
    required this.enabled,
    required this.entries,
    required this.onEnabledChanged,
    required this.onChanged,
    this.onTestEntry,
    this.onTestingChanged,
    this.singleKey = '',
    this.onResetHealth,
  });
  final bool enabled;
  final List<core.ApiKeyInfo> entries;
  final ValueChanged<bool> onEnabledChanged;
  final ValueChanged<List<core.ApiKeyInfo>> onChanged;
  final Future<core.ApiKeyInfo> Function(core.ApiKeyInfo)? onTestEntry;
  final ValueChanged<bool>? onTestingChanged;
  final String singleKey;
  final Future<void> Function()? onResetHealth;

  /// Creates state for draft dialogs and sequential availability probes.
  @override
  State<ApiKeyPoolEditor> createState() => _ApiKeyPoolEditorState();
}

class _ApiKeyPoolEditorState extends State<ApiKeyPoolEditor> {
  bool _testing = false;
  bool _pause = false;
  int _tested = 0;
  int _total = 0;

  /// Copies an entry while preserving imported identity and usage metadata.
  core.ApiKeyInfo _copy(core.ApiKeyInfo entry, Map<String, Object?> changes) =>
      core.ApiKeyInfo.fromJson({...entry.toJson(), ...changes});

  /// Replaces one draft entry by stable identity.
  void _replace(core.ApiKeyInfo entry) => widget.onChanged([
    for (final current in widget.entries)
      current.id == entry.id ? entry : current,
  ]);

  /// Creates an opaque identifier without including the credential in metadata.
  String _newId() {
    final random = Random.secure();
    return List.generate(
      16,
      (_) => random.nextInt(256).toRadixString(16).padLeft(2, '0'),
    ).join();
  }

  /// Adds one or several newline-separated keys, or edits an existing entry.
  Future<void> _edit([core.ApiKeyInfo? entry]) async {
    final l10n = AppLocalizations.of(context)!;
    final name = TextEditingController(text: entry?.name ?? '');
    final keyText = TextEditingController(text: entry?.key ?? '');
    final form = GlobalKey<FormState>();
    final route = DialogRoute<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(
          entry == null
              ? l10n.settingsModelAddApiKey
              : l10n.settingsModelEditApiKey,
        ),
        content: SizedBox(
          width: 480,
          child: SingleChildScrollView(
            child: Form(
              key: form,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  TextFormField(
                    controller: name,
                    decoration: InputDecoration(
                      labelText: l10n.settingsModelApiKeyName,
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextFormField(
                    key: const ValueKey('pool-key-input'),
                    controller: keyText,
                    obscureText: entry != null,
                    minLines: entry == null ? 3 : 1,
                    maxLines: entry == null ? 6 : 1,
                    autocorrect: false,
                    enableSuggestions: false,
                    decoration: InputDecoration(
                      labelText: l10n.settingsModelApiKey,
                      helperText: entry == null
                          ? l10n.settingsModelPoolPaste
                          : null,
                    ),
                    validator: (value) => value == null || value.trim().isEmpty
                        ? l10n.settingsModelApiKey
                        : null,
                  ),
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () {
              if (form.currentState!.validate()) Navigator.pop(context, true);
            },
            child: Text(l10n.save),
          ),
        ],
      ),
    );
    final saved = await Navigator.of(context).push(route);
    if (saved == true && mounted) {
      if (entry != null) {
        final changed = keyText.text.trim() != entry.key;
        _replace(
          _copy(entry, {
            'name': name.text.trim(),
            'key': keyText.text.trim(),
            if (changed) ...{
              'availabilityStatus': 'UNTESTED',
              'usageCount': 0,
              'lastUsed': 0,
              'errorCount': 0,
            },
          }),
        );
      } else {
        final seen = widget.entries.map((entry) => entry.key.trim()).toSet();
        final keys = keyText.text
            .split(RegExp(r'\r?\n'))
            .map((value) => value.trim())
            .where((value) => value.isNotEmpty && seen.add(value))
            .toList();
        widget.onChanged([
          ...widget.entries,
          for (var i = 0; i < keys.length; i++)
            core.ApiKeyInfo(
              id: _newId(),
              key: keys[i],
              name: keys.length == 1
                  ? name.text.trim()
                  : '${name.text.trim()} ${i + 1}'.trim(),
              isEnabled: true,
              availabilityStatus: core.ApiKeyAvailabilityStatus.untested,
              usageCount: 0,
              lastUsed: 0,
              errorCount: 0,
            ),
        ]);
      }
    }
    await route.completed;
    name.dispose();
    keyText.dispose();
  }

  /// Tests enabled, untested entries and pauses after any in-flight probe finishes.
  Future<void> _test() async {
    final pending = widget.entries
        .where(
          (entry) =>
              entry.isEnabled &&
              entry.availabilityStatus ==
                  core.ApiKeyAvailabilityStatus.untested,
        )
        .toList();
    setState(() {
      _testing = true;
      _pause = false;
      _tested = 0;
      _total = pending.length;
    });
    widget.onTestingChanged?.call(true);
    var workingPool = List.of(widget.entries);
    try {
      for (final entry in pending) {
        if (_pause || !mounted) break;
        final result = await widget.onTestEntry!(entry);
        if (!mounted) break;
        workingPool = [
          for (final current in workingPool)
            current.id == result.id ? result : current,
        ];
        widget.onChanged(workingPool);
        setState(() => _tested++);
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
    } finally {
      if (mounted) {
        setState(() => _testing = false);
        widget.onTestingChanged?.call(false);
      }
    }
  }

  /// Clears explicit test marks and the runtime's temporary credential fault state.
  Future<void> _resetHealth() async {
    try {
      await widget.onResetHealth?.call();
      if (!mounted) return;
      widget.onChanged([
        for (final entry in widget.entries)
          _copy(entry, {'availabilityStatus': 'UNTESTED'}),
      ]);
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
    }
  }

  /// Maps persisted availability values to localized labels.
  String _status(
    AppLocalizations l10n,
    core.ApiKeyAvailabilityStatus status,
  ) => switch (status) {
    core.ApiKeyAvailabilityStatus.untested => l10n.settingsModelPoolUntested,
    core.ApiKeyAvailabilityStatus.available => l10n.settingsModelPoolAvailable,
    core.ApiKeyAvailabilityStatus.unavailable =>
      l10n.settingsModelPoolUnavailable,
  };

  /// Builds compact controls that also fit the mobile provider dialog.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: InputDecorator(
        decoration: const InputDecoration(
          contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 4),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SwitchListTile(
              key: const ValueKey('api-key-pool-toggle'),
              contentPadding: EdgeInsets.zero,
              title: Text(
                l10n.settingsModelApiKeyPool,
                style: theme.textTheme.bodyMedium,
              ),
              subtitle: Text(
                l10n.settingsModelApiKeyPoolDescription,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              value: widget.enabled,
              onChanged: _testing ? null : widget.onEnabledChanged,
            ),
            if (widget.enabled) ...[
              if (widget.singleKey.trim().isNotEmpty &&
                  !widget.entries.any(
                    (entry) => entry.key.trim() == widget.singleKey.trim(),
                  ))
                TextButton(
                  onPressed: _testing
                      ? null
                      : () => widget.onChanged([
                          ...widget.entries,
                          core.ApiKeyInfo(
                            id: _newId(),
                            key: widget.singleKey.trim(),
                            name: l10n.settingsModelApiKey,
                            isEnabled: true,
                            availabilityStatus:
                                core.ApiKeyAvailabilityStatus.untested,
                            usageCount: 0,
                            lastUsed: 0,
                            errorCount: 0,
                          ),
                        ]),
                  child: Text(l10n.settingsModelPoolAddSingle),
                ),
              FormField<bool>(
                validator: (_) {
                  final enabled = widget.entries.where(
                    (entry) => entry.isEnabled && entry.key.trim().isNotEmpty,
                  );
                  return !enabled.any(
                        (entry) =>
                            entry.availabilityStatus !=
                            core.ApiKeyAvailabilityStatus.unavailable,
                      )
                      ? l10n.settingsModelPoolNoEligible
                      : null;
                },
                builder: (field) => Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (widget.entries.isEmpty)
                      Text(l10n.settingsModelApiKeyPoolEmpty),
                    for (final entry in widget.entries)
                      Padding(
                        padding: const EdgeInsets.symmetric(vertical: 4),
                        child: Row(
                          children: [
                            Switch(
                              value: entry.isEnabled,
                              onChanged: _testing
                                  ? null
                                  : (value) => _replace(
                                      _copy(entry, {'isEnabled': value}),
                                    ),
                            ),
                            Expanded(
                              child: InkWell(
                                onTap: _testing ? null : () => _edit(entry),
                                child: Padding(
                                  padding: const EdgeInsets.symmetric(
                                    vertical: 8,
                                  ),
                                  child: Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.start,
                                    children: [
                                      Text(
                                        entry.name.isEmpty
                                            ? l10n.settingsModelApiKey
                                            : entry.name,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                      ),
                                      Text(
                                        '•••••••• · ${_status(l10n, entry.availabilityStatus)} · ${entry.usageCount} / ${entry.errorCount}',
                                        style: Theme.of(
                                          context,
                                        ).textTheme.bodySmall,
                                      ),
                                    ],
                                  ),
                                ),
                              ),
                            ),
                            IconButton(
                              tooltip: l10n.settingsModelEditApiKey,
                              onPressed: _testing ? null : () => _edit(entry),
                              icon: const Icon(Icons.edit_outlined),
                            ),
                            IconButton(
                              tooltip: l10n.delete,
                              onPressed: _testing
                                  ? null
                                  : () => widget.onChanged(
                                      widget.entries
                                          .where((item) => item.id != entry.id)
                                          .toList(),
                                    ),
                              icon: const Icon(Icons.delete_outline),
                            ),
                          ],
                        ),
                      ),
                    if (field.hasError)
                      Text(
                        field.errorText!,
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                  ],
                ),
              ),
              Wrap(
                spacing: 8,
                children: [
                  TextButton.icon(
                    onPressed: _testing ? null : _edit,
                    icon: const Icon(Icons.add),
                    label: Text(l10n.settingsModelAddApiKey),
                  ),
                  TextButton(
                    onPressed: _testing || widget.entries.isEmpty
                        ? null
                        : _resetHealth,
                    child: Text(l10n.settingsModelPoolClearMarks),
                  ),
                  if (widget.onTestEntry != null)
                    TextButton(
                      onPressed: _testing
                          ? () => setState(() => _pause = true)
                          : _test,
                      child: Text(
                        _testing ? l10n.pause : l10n.settingsModelPoolTest,
                      ),
                    ),
                ],
              ),
              if (_testing) ...[
                const LinearProgressIndicator(),
                Text(l10n.settingsModelTestProgress(_tested, _total)),
              ],
              if (widget.onTestEntry == null)
                Text(
                  l10n.settingsModelPoolNeedsModel,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
            ],
          ],
        ),
      ),
    );
  }
}
