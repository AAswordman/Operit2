// ignore_for_file: file_names

import 'package:flutter/material.dart';
import '../../../theme/OperitThemeAssets.dart';

import 'ThemeAppearancePreview.dart';
import 'ThemeCreationDialog.dart';
import '../components/SettingsControlStyles.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/OperitDialog.dart';
import '../../../theme/ThemeTemplateService.dart';

class ThemeConfigurationControl extends StatefulWidget {
  /// Creates the visual theme library using the synchronized appearance owner.
  const ThemeConfigurationControl({
    super.key,
    this.clients = const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge()),
    this.onEdit,
    this.onAppearanceChanged,
  });

  final GeneratedCoreProxyClients clients;
  final Future<void> Function(String name)? onEdit;
  final Future<void> Function()? onAppearanceChanged;

  /// Creates the catalog observer and explicit configuration action state.
  @override
  State<ThemeConfigurationControl> createState() =>
      _ThemeConfigurationControlState();
}

class _ThemeConfigurationControlState extends State<ThemeConfigurationControl> {
  late GeneratedApplicationThemeConfigManagerCoreProxy _manager;
  late UserPreferencesManager _preferences;
  late ThemeAssetStore _assets;
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
    _assets = ThemeAssetStore(
      runtimeStorage: widget.clients.repositoryRuntimeStorageRepository,
    );
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
        final theme = Theme.of(context);
        final activeConfig = state.configs
            .where((config) => config.id == state.activeThemeConfigId)
            .firstOrNull;
        final activeSnapshot = activeConfig == null
            ? null
            : ThemePreferenceSnapshot.fromJson(activeConfig.snapshot);
        return CustomScrollView(
          key: const PageStorageKey('theme-library-scroll'),
          slivers: [
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(20, 20, 20, 16),
              sliver: SliverToBoxAdapter(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    LayoutBuilder(
                      builder: (context, constraints) {
                        final createButton = FilledButton.icon(
                          key: const ValueKey('theme-config-create'),
                          onPressed: _busy ? null : _create,
                          icon: const Icon(Icons.add, size: 18),
                          label: Text(l10n.settingsAppearanceThemeNew),
                        );
                        if (activeSnapshot == null) {
                          return Align(
                            alignment: AlignmentDirectional.centerEnd,
                            child: createButton,
                          );
                        }
                        final modeControls = _ThemeModeQuickControls(
                          value: activeSnapshot.themeMode,
                          onChanged: (mode) {
                            if (!_busy && mode != activeSnapshot.themeMode) {
                              _run(() async {
                                await _manager.patchAppearance(
                                  initialSnapshot: activeSnapshot.toJson(),
                                  values: {'theme_mode': mode.name},
                                  removeKeys: const [],
                                );
                                await widget.onAppearanceChanged?.call();
                              });
                            }
                          },
                        );
                        if (constraints.maxWidth >= 640 &&
                            MediaQuery.textScalerOf(context).scale(14) <= 20) {
                          return Row(
                            children: [
                              Expanded(
                                child: Align(
                                  alignment: AlignmentDirectional.centerStart,
                                  child: modeControls,
                                ),
                              ),
                              const SizedBox(width: 16),
                              createButton,
                            ],
                          );
                        }
                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            modeControls,
                            const SizedBox(height: 12),
                            Align(
                              alignment: AlignmentDirectional.centerEnd,
                              child: createButton,
                            ),
                          ],
                        );
                      },
                    ),
                    const SizedBox(height: 16),
                    Text(
                      l10n.settingsAppearanceThemeConfigEditingHint,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                    if (_busy) ...[
                      const SizedBox(height: 12),
                      const LinearProgressIndicator(),
                    ],
                    if (_error != null) ...[
                      const SizedBox(height: 12),
                      Text(
                        _error!,
                        style: TextStyle(color: theme.colorScheme.error),
                      ),
                    ],
                  ],
                ),
              ),
            ),
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(20, 0, 20, 24),
              sliver: SliverList(
                delegate: SliverChildBuilderDelegate((context, index) {
                  final config = state.configs[index];
                  final active = config.id == state.activeThemeConfigId;
                  return Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: _ThemeCard(
                      key: ValueKey('theme-card:${config.id}'),
                      config: config,
                      assets: _assets,
                      active: active,
                      busy: _busy,
                      onApply: () => _run(() async {
                        await _manager.apply(id: config.id);
                      }),
                      onEdit: () => _edit(config, active: active),
                      onPreview: () => _preview(config),
                      onRename: () => _rename(config),
                      onDuplicate: () =>
                          _create(initialSource: 'config:${config.id}'),
                      onDelete: active ? null : () => _delete(config),
                    ),
                  );
                }, childCount: state.configs.length),
              ),
            ),
          ],
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

  /// Opens the active configuration directly and applies only a different selection.
  Future<void> _edit(
    core.CoreDataPreferencesThemeConfigManagerThemeConfig config, {
    required bool active,
  }) async {
    if (active) {
      await widget.onEdit?.call(config.name);
      return;
    }
    var applied = false;
    await _run(() async {
      await _manager.apply(id: config.id);
      applied = true;
    });
    if (applied && mounted) await widget.onEdit?.call(config.name);
  }

  Future<void> _preview(
    core.CoreDataPreferencesThemeConfigManagerThemeConfig config,
  ) async {
    final accepted = await showDialog<bool>(
      context: context,
      builder: (context) => OperitDialogScaffold(
        title: config.name,
        maxWidth: 580,
        maxHeight: 700,
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(AppLocalizations.of(context)!.close),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(
              AppLocalizations.of(context)!.settingsAppearanceThemeApply,
            ),
          ),
        ],
        child: ThemeAppearancePreview(
          snapshot: ThemePreferenceSnapshot.fromJson(config.snapshot),
          thumbnail: false,
          assets: _assets,
        ),
      ),
    );
    if (accepted == true && mounted) {
      await _run(() async {
        await _manager.apply(id: config.id);
      });
    }
  }

  Future<void> _delete(
    core.CoreDataPreferencesThemeConfigManagerThemeConfig config,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final accepted = await showDialog<bool>(
      context: context,
      builder: (context) => OperitDialogScaffold(
        title: l10n.settingsAppearanceThemeDeleteTitle,
        expandContent: false,
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(l10n.cancel),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(
              l10n.delete,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        ],
        child: Text(l10n.settingsAppearanceThemeDeleteConfirm(config.name)),
      ),
    );
    if (accepted == true && mounted) {
      await _run(() => _manager.delete(id: config.id));
    }
  }

  /// Creates from a visual source picker, then opens the new theme's editor.
  Future<void> _create({String initialSource = 'defaults'}) async {
    final service = ThemeTemplateService(widget.clients);
    List<ThemeTemplateSource>? templates;
    List<core.CoreDataPreferencesThemeConfigManagerThemeConfig>? configs;
    ThemePreferenceSnapshot? current;
    await _run(() async {
      configs = await _manager.list();
      templates = await service.listTemplates();
      current = await _preferences.resolveThemePreferenceSnapshot();
    });
    if (!mounted || templates == null || configs == null || current == null) {
      return;
    }
    final choice = await showDialog<ThemeCreationChoice>(
      context: context,
      builder: (_) => ThemeCreationDialog(
        configs: configs!,
        templates: templates!,
        current: current!,
        service: service,
        assets: _assets,
        initialSource: initialSource,
      ),
    );
    if (choice == null || !mounted) return;
    String? createdName;
    await _run(() async {
      final Map<String, Object?> snapshot;
      if (choice.source == 'defaults') {
        snapshot = UserPreferencesManager.defaultThemePreferenceSnapshot
            .toJson();
      } else if (choice.source == 'current') {
        snapshot = current!.toJson();
      } else if (choice.source.startsWith('config:')) {
        snapshot = (await _manager.getValue(
          id: choice.source.substring(7),
        )).snapshot;
      } else {
        final template = templates!.singleWhere(
          (source) =>
              choice.source ==
              'template:${source.packageName}:${source.templateId}',
        );
        snapshot = (await service.createSnapshot(template)).toJson();
      }
      final config = await _manager.create(
        name: choice.name,
        snapshot: snapshot,
      );
      await _manager.apply(id: config.id);
      createdName = config.name;
    });
    if (createdName != null && mounted) await widget.onEdit?.call(createdName!);
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

/// Keeps selection and editing visible; occasional actions live in one menu.
class _ThemeCard extends StatelessWidget {
  const _ThemeCard({
    super.key,
    required this.config,
    required this.assets,
    required this.active,
    required this.busy,
    required this.onApply,
    required this.onEdit,
    required this.onPreview,
    required this.onRename,
    required this.onDuplicate,
    required this.onDelete,
  });

  final core.CoreDataPreferencesThemeConfigManagerThemeConfig config;
  final ThemeAssetStore assets;
  final bool active;
  final bool busy;
  final VoidCallback onApply;
  final VoidCallback onEdit;
  final VoidCallback onPreview;
  final VoidCallback onRename;
  final VoidCallback onDuplicate;
  final VoidCallback? onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final snapshot = ThemePreferenceSnapshot.fromJson(config.snapshot);
    final mode = switch (snapshot.themeMode) {
      ThemeMode.light => l10n.settingsAppearanceThemeLight,
      ThemeMode.dark => l10n.settingsAppearanceThemeDark,
      ThemeMode.system => l10n.settingsAppearanceThemeSystem,
    };
    return Semantics(
      selected: active,
      container: true,
      child: Material(
        color: scheme.surfaceContainerLow,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(
            color: active ? scheme.primary : scheme.outlineVariant,
            width: active ? 2 : 1,
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          key: ValueKey('theme-apply:${config.id}'),
          onTap: busy || active ? null : onApply,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final compact = constraints.maxWidth < 520;
              return Padding(
                padding: const EdgeInsets.all(10),
                child: Row(
                  children: [
                    SizedBox(
                      width: compact
                          ? (constraints.maxWidth * .32).clamp(56.0, 112.0)
                          : 196,
                      height: compact ? 104 : 112,
                      child: ThemeAppearancePreview(
                        key: ValueKey('theme-thumbnail:${config.id}'),
                        snapshot: snapshot,
                        assets: assets,
                      ),
                    ),
                    SizedBox(width: compact ? 12 : 20),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Text(
                            config.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.titleMedium?.copyWith(
                              fontWeight: FontWeight.w700,
                            ),
                          ),
                          const SizedBox(height: 5),
                          Row(
                            children: [
                              if (active) ...[
                                Icon(
                                  Icons.check_circle,
                                  size: 14,
                                  color: scheme.primary,
                                ),
                                const SizedBox(width: 4),
                              ],
                              Flexible(
                                child: Text(
                                  active
                                      ? l10n.settingsAppearanceThemeActive
                                      : mode,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: theme.textTheme.bodySmall?.copyWith(
                                    color: active
                                        ? scheme.primary
                                        : scheme.onSurfaceVariant,
                                  ),
                                ),
                              ),
                            ],
                          ),
                          if (compact)
                            Align(
                              alignment: Alignment.centerLeft,
                              child: TextButton(
                                key: ValueKey('theme-edit:${config.id}'),
                                style: TextButton.styleFrom(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 0,
                                  ),
                                  alignment: Alignment.centerLeft,
                                  minimumSize: const Size(48, 36),
                                ),
                                onPressed: busy ? null : onEdit,
                                child: Text(l10n.edit),
                              ),
                            ),
                        ],
                      ),
                    ),
                    if (!compact) ...[
                      IconButton(
                        key: ValueKey('theme-preview:${config.id}'),
                        tooltip: l10n.settingsAppearanceLivePreviewTitle,
                        onPressed: busy ? null : onPreview,
                        icon: const Icon(Icons.open_in_full, size: 18),
                      ),
                      const SizedBox(width: 8),
                      OutlinedButton.icon(
                        key: ValueKey('theme-edit:${config.id}'),
                        onPressed: busy ? null : onEdit,
                        icon: const Icon(Icons.edit_outlined, size: 16),
                        label: Text(l10n.edit),
                      ),
                      const SizedBox(width: 4),
                    ],
                    PopupMenuButton<String>(
                      key: ValueKey('theme-menu:${config.id}'),
                      enabled: !busy,
                      tooltip: l10n.settingsAppearanceThemeMore,
                      icon: const Icon(Icons.more_vert, size: 20),
                      itemBuilder: (_) => [
                        PopupMenuItem(
                          value: 'preview',
                          child: Text(l10n.settingsAppearanceLivePreviewTitle),
                        ),
                        PopupMenuItem(
                          value: 'rename',
                          child: Text(l10n.settingsAppearanceThemeConfigRename),
                        ),
                        PopupMenuItem(
                          value: 'duplicate',
                          child: Text(l10n.settingsAppearanceThemeDuplicate),
                        ),
                        PopupMenuItem(
                          value: 'delete',
                          enabled: onDelete != null,
                          child: Text(
                            l10n.delete,
                            style: TextStyle(
                              color: onDelete == null
                                  ? scheme.onSurface.withValues(alpha: 0.38)
                                  : scheme.error,
                            ),
                          ),
                        ),
                      ],
                      onSelected: (action) {
                        switch (action) {
                          case 'preview':
                            onPreview();
                          case 'rename':
                            onRename();
                          case 'duplicate':
                            onDuplicate();
                          case 'delete':
                            onDelete?.call();
                        }
                      },
                    ),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

/// Keeps every shortcut reachable when a sidebar, zoom or large text narrows the pane.
class _ThemeModeQuickControls extends StatelessWidget {
  const _ThemeModeQuickControls({required this.value, required this.onChanged});
  final ThemeMode value;
  final ValueChanged<ThemeMode> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final segments = [
      ButtonSegment(
        value: ThemeMode.system,
        icon: const Icon(Icons.brightness_auto_outlined, size: 18),
        label: Text(l10n.settingsAppearanceThemeSystem),
      ),
      ButtonSegment(
        value: ThemeMode.light,
        icon: const Icon(Icons.light_mode_outlined, size: 18),
        label: Text(l10n.settingsAppearanceThemeLight),
      ),
      ButtonSegment(
        value: ThemeMode.dark,
        icon: const Icon(Icons.dark_mode_outlined, size: 18),
        label: Text(l10n.settingsAppearanceThemeDark),
      ),
    ];
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth >= 420 &&
            MediaQuery.textScalerOf(context).scale(14) <= 20) {
          return SettingsSegmentedSelector<ThemeMode>(
            value: value,
            onChanged: onChanged,
            segments: segments,
          );
        }
        return Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            for (final segment in segments)
              ChoiceChip(
                selected: value == segment.value,
                showCheckmark: false,
                avatar: segment.icon,
                label: segment.label!,
                onSelected: (_) => onChanged(segment.value),
              ),
          ],
        );
      },
    );
  }
}
