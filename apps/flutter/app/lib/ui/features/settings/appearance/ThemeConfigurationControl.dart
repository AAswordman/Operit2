// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/OperitDialog.dart';
import '../../../theme/ThemeTemplateService.dart';

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

  /// Creates an independent configuration from defaults, an existing configuration, or a plugin template.
  Future<void> _create() async {
    final l10n = AppLocalizations.of(context)!;
    final service = ThemeTemplateService(widget.clients);
    List<ThemeTemplateSource>? templates;
    List<core.CoreDataPreferencesThemeConfigManagerThemeConfig>? configs;
    await _run(() async {
      configs = await _manager.list();
      templates = await service.listTemplates();
    });
    if (!mounted || templates == null || configs == null) return;
    final form = GlobalKey<FormState>();
    var name = '';
    var source = 'defaults';
    final sources = <String, Object>{
      for (final config in configs!) 'config:${config.id}': config,
      for (final template in templates!)
        'template:${template.packageName}:${template.templateId}': template,
    };
    final accepted = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (context, setDialogState) => OperitDialogScaffold(
          title: l10n.settingsAppearanceThemeConfigCreate,
          maxWidth: 520,
          maxHeight: 640,
          expandContent: false,
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(false),
              child: Text(l10n.cancel),
            ),
            FilledButton(
              onPressed: () {
                if (!form.currentState!.validate()) return;
                form.currentState!.save();
                Navigator.of(dialogContext).pop(true);
              },
              child: Text(l10n.create),
            ),
          ],
          child: SingleChildScrollView(
            key: const ValueKey('theme-creation-scroll'),
            child: Form(
              key: form,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  TextFormField(
                    autofocus: true,
                    decoration: InputDecoration(
                      labelText: l10n.settingsAppearanceThemeConfigName,
                    ),
                    onSaved: (value) => name = value!.trim(),
                    validator: (value) => value == null || value.trim().isEmpty
                        ? l10n.settingsAppearanceThemeConfigNameRequired
                        : null,
                  ),
                  const SizedBox(height: 24),
                  _ThemeCreationSources(
                    value: source,
                    configs: configs!,
                    templates: templates!,
                    onSelected: (value) => setDialogState(() => source = value),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    if (accepted != true || !mounted) return;
    await _run(() async {
      final Map<String, Object?> snapshot;
      if (source == 'defaults') {
        snapshot = UserPreferencesManager.defaultThemePreferenceSnapshot
            .toJson();
      } else if (source == 'current') {
        snapshot = (await _preferences.resolveThemePreferenceSnapshot())
            .toJson();
      } else {
        final selected = sources[source];
        switch (selected) {
          case core.CoreDataPreferencesThemeConfigManagerThemeConfig():
            snapshot = (await _manager.getValue(id: selected.id)).snapshot;
          case ThemeTemplateSource():
            snapshot = (await service.createSnapshot(selected)).toJson();
          default:
            throw StateError('Creation source is no longer available');
        }
      }
      final config = await _manager.create(name: name, snapshot: snapshot);
      await _manager.apply(id: config.id);
    });
  }

  /// Renames the selected configuration while retaining its current persisted appearance.
  Future<void> _rename(
    core.CoreDataPreferencesThemeConfigManagerThemeConfig config,
  ) async {
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

/// Shows creation sources directly with separate groups for saved configurations and plugin templates.
class _ThemeCreationSources extends StatelessWidget {
  /// Receives the current registry snapshot and a single explicit selection.
  const _ThemeCreationSources({
    required this.value,
    required this.configs,
    required this.templates,
    required this.onSelected,
  });

  final String value;
  final List<core.CoreDataPreferencesThemeConfigManagerThemeConfig> configs;
  final List<ThemeTemplateSource> templates;
  final ValueChanged<String> onSelected;

  /// Keeps each option visible and wraps pairs into single columns on narrow layouts.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth < 360
            ? constraints.maxWidth
            : (constraints.maxWidth - 12) / 2;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              l10n.settingsAppearanceThemeConfigSource,
              style: theme.textTheme.titleSmall,
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 12,
              runSpacing: 10,
              children: <Widget>[
                SizedBox(
                  width: width,
                  child: _ThemeCreationSourceOption(
                    key: const ValueKey('theme-source-defaults'),
                    title: l10n.settingsAppearanceThemeConfigFromDefaults,
                    subtitle: l10n.settingsAppearanceThemeConfigDefaultsHint,
                    icon: Icons.palette_outlined,
                    selected: value == 'defaults',
                    onTap: () => onSelected('defaults'),
                  ),
                ),
                SizedBox(
                  width: width,
                  child: _ThemeCreationSourceOption(
                    key: const ValueKey('theme-source-current'),
                    title: l10n.settingsAppearanceThemeConfigFromCurrent,
                    subtitle: l10n.settingsAppearanceThemeConfigCurrentHint,
                    icon: Icons.copy_outlined,
                    selected: value == 'current',
                    onTap: () => onSelected('current'),
                  ),
                ),
              ],
            ),
            if (configs.isNotEmpty) ...<Widget>[
              const SizedBox(height: 24),
              Text(
                l10n.settingsAppearanceThemeConfigSavedSources,
                style: theme.textTheme.titleSmall,
              ),
              const SizedBox(height: 10),
              Wrap(
                spacing: 12,
                runSpacing: 10,
                children: <Widget>[
                  for (final config in configs)
                    SizedBox(
                      width: width,
                      child: _ThemeCreationSourceOption(
                        key: ValueKey('theme-source-config:${config.id}'),
                        title: config.name,
                        icon: Icons.layers_outlined,
                        selected: value == 'config:${config.id}',
                        onTap: () => onSelected('config:${config.id}'),
                      ),
                    ),
                ],
              ),
            ],
            if (templates.isNotEmpty) ...<Widget>[
              const SizedBox(height: 24),
              Text(
                l10n.settingsAppearanceThemeConfigPluginSources,
                style: theme.textTheme.titleSmall,
              ),
              const SizedBox(height: 6),
              Text(
                l10n.settingsAppearanceThemeConfigSourceHint,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 10),
              Wrap(
                spacing: 12,
                runSpacing: 10,
                children: <Widget>[
                  for (final template in templates)
                    SizedBox(
                      width: width,
                      child: _ThemeCreationSourceOption(
                        key: ValueKey(
                          'theme-source-template:${template.packageName}:${template.templateId}',
                        ),
                        title: template.label,
                        icon: Icons.extension_outlined,
                        selected:
                            value ==
                            'template:${template.packageName}:${template.templateId}',
                        onTap: () => onSelected(
                          'template:${template.packageName}:${template.templateId}',
                        ),
                      ),
                    ),
                ],
              ),
            ],
          ],
        );
      },
    );
  }
}

/// Displays a source with an explicit selection mark and the application's theme colors.
class _ThemeCreationSourceOption extends StatelessWidget {
  /// Describes one creation source without storing its selection independently.
  const _ThemeCreationSourceOption({
    super.key,
    required this.title,
    this.subtitle,
    required this.icon,
    required this.selected,
    required this.onTap,
  });

  final String title;
  final String? subtitle;
  final IconData icon;
  final bool selected;
  final VoidCallback onTap;

  /// Exposes selection to assistive technology and keeps the entire card clickable.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Semantics(
      selected: selected,
      button: true,
      child: Material(
        color: selected
            ? scheme.primary.withValues(alpha: 0.07)
            : scheme.surface,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(
            color: selected ? scheme.primary : scheme.outlineVariant,
            width: selected ? 1.5 : 1,
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Padding(
                  padding: const EdgeInsets.only(top: 2),
                  child: Icon(
                    icon,
                    size: 20,
                    color: selected ? scheme.primary : scheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        title,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      if (subtitle != null) ...<Widget>[
                        const SizedBox(height: 4),
                        Text(
                          subtitle!,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: scheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Icon(
                  selected ? Icons.check_circle : Icons.radio_button_unchecked,
                  size: 18,
                  color: selected ? scheme.primary : scheme.outline,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
