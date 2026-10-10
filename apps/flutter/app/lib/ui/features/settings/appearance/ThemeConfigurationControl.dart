// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/OperitDialog.dart';

class ThemeConfigurationControl extends StatefulWidget {
  /// Creates the named theme selector using the ordinary appearance owner.
  const ThemeConfigurationControl({
    super.key,
    this.clients = const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge()),
  });

  final GeneratedCoreProxyClients clients;

  /// Creates the catalog observer and explicit configuration action state.
  @override
  State<ThemeConfigurationControl> createState() =>
      _ThemeConfigurationControlState();
}

class _ThemeConfigurationControlState extends State<ThemeConfigurationControl> {
  late GeneratedApplicationThemeConfigManagerCoreProxy _manager;
  late UserPreferencesManager _preferences;
  late Stream<core.ThemeConfigState> _catalog;
  bool _busy = false;
  String? _error;

  /// Connects the actual catalog without constructing local theme records.
  @override
  void initState() {
    super.initState();
    _connect();
  }

  /// Reconnects the observer when the owning runtime client changes.
  @override
  void didUpdateWidget(covariant ThemeConfigurationControl oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.clients != widget.clients) _connect();
  }

  /// Opens the existing theme manager and the same ordinary preference reader.
  void _connect() {
    _manager = widget.clients.application.themeConfigManager();
    _preferences = UserPreferencesManager(clients: widget.clients);
    _catalog = _manager.watch();
    _error = null;
  }

  /// Displays committed named themes and reports unavailable or failed operations.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return StreamBuilder<core.ThemeConfigState>(
      stream: _catalog,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return Padding(
            padding: const EdgeInsets.all(16),
            child: Text(
              l10n.settingsAppearanceThemeConfigFailed('${snapshot.error}'),
            ),
          );
        }
        final state = snapshot.data;
        if (state == null) return const LinearProgressIndicator();
        final inactive = state.configs
            .where((config) => config.id != state.activeThemeConfigId)
            .toList();
        return Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 8, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: InputDecorator(
                      decoration: InputDecoration(
                        labelText: l10n.settingsAppearanceThemeConfig,
                      ),
                      child: DropdownButtonHideUnderline(
                        child: DropdownButton<String>(
                          value: state.activeThemeConfigId ?? '',
                          isDense: true,
                          isExpanded: true,
                          items: [
                            DropdownMenuItem(
                              value: '',
                              child: Text(
                                l10n.settingsAppearanceThemeConfigCustom,
                              ),
                            ),
                            for (final config in state.configs)
                              DropdownMenuItem(
                                value: config.id,
                                child: Text(
                                  config.name,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                          ],
                          onChanged: _busy
                              ? null
                              : (id) {
                                  if (id == null) {
                                    throw StateError(
                                      'A theme selection must have an explicit value.',
                                    );
                                  }
                                  _run(() async {
                                    if (id.isEmpty) {
                                      final current = await _preferences
                                          .resolveThemePreferenceSnapshot();
                                      await _manager.useCustomAppearance(
                                        snapshot: current.toJson(),
                                      );
                                    } else {
                                      await _manager.apply(id: id);
                                    }
                                  });
                                },
                        ),
                      ),
                    ),
                  ),
                  IconButton(
                    key: const ValueKey('theme-config-create'),
                    tooltip: l10n.settingsAppearanceThemeConfigCreate,
                    onPressed: _busy ? null : _create,
                    icon: const Icon(Icons.add),
                  ),
                  IconButton(
                    key: const ValueKey('theme-config-rename'),
                    tooltip: l10n.settingsAppearanceThemeConfigRename,
                    onPressed: _busy || state.activeThemeConfigId == null
                        ? null
                        : () => _rename(
                            state.configs.singleWhere(
                              (config) =>
                                  config.id == state.activeThemeConfigId,
                            ),
                          ),
                    icon: const Icon(Icons.edit_outlined),
                  ),
                  PopupMenuButton<String>(
                    key: const ValueKey('theme-config-delete'),
                    tooltip: l10n.settingsAppearanceThemeConfigDelete,
                    enabled: !_busy && inactive.isNotEmpty,
                    icon: const Icon(Icons.delete_outline),
                    itemBuilder: (_) => [
                      for (final config in inactive)
                        PopupMenuItem(
                          value: config.id,
                          child: Text(config.name),
                        ),
                    ],
                    onSelected: (id) => _run(() => _manager.delete(id: id)),
                  ),
                ],
              ),
              const SizedBox(height: 6),
              Text(
                state.activeThemeConfigId == null
                    ? l10n.settingsAppearanceThemeConfigCustomHint
                    : l10n.settingsAppearanceThemeConfigEditingHint,
                style: Theme.of(context).textTheme.bodySmall,
              ),
              if (_busy) const LinearProgressIndicator(),
              if (_error != null)
                Text(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
            ],
          ),
        );
      },
    );
  }

  /// Preserves the original operation failure and exposes it without changing selections.
  Future<void> _run(Future<void> Function() action) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await action();
    } catch (error) {
      if (mounted) {
        setState(
          () => _error = AppLocalizations.of(
            context,
          )!.settingsAppearanceThemeConfigFailed('$error'),
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  /// Creates a theme with the declared initial appearance and selects its assigned ID.
  Future<void> _create() async {
    final name = await _readName(
      AppLocalizations.of(context)!.settingsAppearanceThemeConfigCreate,
      '',
      AppLocalizations.of(context)!.create,
    );
    if (name == null || !mounted) return;
    await _run(() async {
      final config = await _manager.create(
        name: name,
        snapshot: UserPreferencesManager.defaultThemePreferenceSnapshot
            .toJson(),
      );
      await _manager.apply(id: config.id);
    });
  }

  /// Renames the selected configuration while retaining its current persisted appearance.
  Future<void> _rename(core.ThemeConfig config) async {
    final name = await _readName(
      AppLocalizations.of(context)!.settingsAppearanceThemeConfigRename,
      config.name,
      AppLocalizations.of(context)!.save,
    );
    if (name == null || !mounted) return;
    await _run(() async {
      await _manager.rename(id: config.id, name: name);
    });
  }

  /// Requires an explicit nonblank name before creating or renaming a saved theme.
  Future<String?> _readName(
    String title,
    String initial,
    String actionLabel,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    var selectedName = initial;
    final form = GlobalKey<FormState>();
    return showDialog<String>(
      context: context,
      builder: (dialogContext) => OperitDialogScaffold(
        title: title,
        expandContent: false,
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () {
              if (form.currentState!.validate()) {
                form.currentState!.save();
                Navigator.of(dialogContext).pop(selectedName);
              }
            },
            child: Text(actionLabel),
          ),
        ],
        child: Form(
          key: form,
          child: TextFormField(
            initialValue: initial,
            onSaved: (value) {
              selectedName = value!.trim();
            },
            autofocus: true,
            decoration: InputDecoration(
              labelText: l10n.settingsAppearanceThemeConfigName,
            ),
            validator: (value) => value == null || value.trim().isEmpty
                ? l10n.settingsAppearanceThemeConfigNameRequired
                : null,
          ),
        ),
      ),
    );
  }
}
