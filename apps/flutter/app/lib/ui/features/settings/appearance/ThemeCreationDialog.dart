// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart'
    show PreparedThemeVisual, ThemeAssetResolver;

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/OperitDialog.dart';
import '../../../theme/ThemeTemplateService.dart';
import 'ThemeAppearancePreview.dart';

class ThemeCreationChoice {
  const ThemeCreationChoice(this.name, this.source);
  final String name;
  final String source;
}

/// Presents a live sample alongside source selection before creating anything.
class ThemeCreationDialog extends StatefulWidget {
  const ThemeCreationDialog({
    super.key,
    required this.configs,
    required this.templates,
    required this.current,
    required this.service,
    this.initialSource = 'defaults',
    this.assets,
  });

  final List<core.CoreDataPreferencesThemeConfigManagerThemeConfig> configs;
  final List<ThemeTemplateSource> templates;
  final ThemePreferenceSnapshot current;
  final ThemeTemplateService service;
  final ThemeAssetResolver? assets;
  final String initialSource;

  @override
  State<ThemeCreationDialog> createState() => _ThemeCreationDialogState();
}

class _ThemeCreationDialogState extends State<ThemeCreationDialog> {
  final _form = GlobalKey<FormState>();
  final _detailsScroll = ScrollController();
  final _sourcesScroll = ScrollController();
  final _narrowScroll = ScrollController();
  late String _source = widget.initialSource;
  String _name = '';
  PreparedThemeVisual? _visual;
  bool _loading = false;
  Object? _error;
  int _revision = 0;

  ThemePreferenceSnapshot get _snapshot {
    if (_source == 'current') return widget.current;
    if (_source.startsWith('config:')) {
      return ThemePreferenceSnapshot.fromJson(
        widget.configs
            .singleWhere((config) => config.id == _source.substring(7))
            .snapshot,
      );
    }
    return UserPreferencesManager.defaultThemePreferenceSnapshot;
  }

  Future<void> _select(String source) async {
    final revision = ++_revision;
    final previous = _visual;
    setState(() {
      _source = source;
      _visual = null;
      _error = null;
      _loading = source.startsWith('template:');
    });
    WidgetsBinding.instance.addPostFrameCallback((_) => previous?.dispose());
    if (!_loading) return;
    try {
      final template = widget.templates.singleWhere(
        (template) =>
            source == 'template:${template.packageName}:${template.templateId}',
      );
      final visual = await widget.service.preparePreview(template);
      if (!mounted || revision != _revision) {
        visual.dispose();
        return;
      }
      setState(() {
        _visual = visual;
        _loading = false;
      });
    } catch (error) {
      if (mounted && revision == _revision) {
        setState(() {
          _error = error;
          _loading = false;
        });
      }
    }
  }

  @override
  void dispose() {
    _revision++;
    _detailsScroll.dispose();
    _sourcesScroll.dispose();
    _narrowScroll.dispose();
    _visual?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OperitDialogScaffold(
      title: l10n.settingsAppearanceThemeConfigCreate,
      maxWidth: 940,
      maxHeight: 760,
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _loading || _error != null
              ? null
              : () {
                  if (!_form.currentState!.validate()) return;
                  _form.currentState!.save();
                  Navigator.pop(context, ThemeCreationChoice(_name, _source));
                },
          child: Text(l10n.create),
        ),
      ],
      child: LayoutBuilder(
        builder: (context, constraints) {
          final name = Form(
            key: _form,
            child: TextFormField(
              autofocus: false,
              decoration: InputDecoration(
                labelText: l10n.settingsAppearanceThemeConfigName,
              ),
              onSaved: (value) => _name = value!.trim(),
              validator: (value) => value == null || value.trim().isEmpty
                  ? l10n.settingsAppearanceThemeConfigNameRequired
                  : null,
            ),
          );
          final wide = constraints.maxWidth >= 680;
          final previewHeight = (constraints.maxHeight * .7).clamp(
            160.0,
            wide ? 360.0 : 196.0,
          );
          final preview = Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                l10n.settingsAppearanceLivePreviewTitle,
                style: Theme.of(context).textTheme.titleSmall,
              ),
              const SizedBox(height: 12),
              SizedBox(
                key: const ValueKey('theme-creation-preview'),
                height: previewHeight,
                child: _loading
                    ? const Center(child: CircularProgressIndicator())
                    : _error != null
                    ? Center(
                        child: Text(
                          l10n.settingsAppearanceThemeConfigFailed('$_error'),
                        ),
                      )
                    : ThemeAppearancePreview(
                        snapshot: _snapshot,
                        assets: widget.assets,
                        visual: _visual,
                        thumbnail: !wide,
                      ),
              ),
            ],
          );
          final sources = _sources(context);
          if (wide) {
            return Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: _scrollPane(
                    key: const ValueKey('theme-creation-details-scroll'),
                    controller: _detailsScroll,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [name, const SizedBox(height: 20), preview],
                    ),
                  ),
                ),
                const SizedBox(width: 28),
                Expanded(
                  child: _scrollPane(
                    key: const ValueKey('theme-creation-scroll'),
                    controller: _sourcesScroll,
                    child: sources,
                  ),
                ),
              ],
            );
          }
          return _scrollPane(
            key: const ValueKey('theme-creation-scroll'),
            controller: _narrowScroll,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                name,
                const SizedBox(height: 20),
                preview,
                const SizedBox(height: 24),
                sources,
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _scrollPane({
    required Key key,
    required ScrollController controller,
    required Widget child,
  }) => Scrollbar(
    controller: controller,
    thumbVisibility: true,
    child: SingleChildScrollView(
      key: key,
      controller: controller,
      primary: false,
      child: child,
    ),
  );

  Widget _sources(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    Widget option(
      String source,
      String title, {
      ThemePreferenceSnapshot? snapshot,
      String? subtitle,
    }) {
      final selected = _source == source;
      final scheme = Theme.of(context).colorScheme;
      return Padding(
        padding: const EdgeInsets.only(bottom: 10),
        child: Semantics(
          selected: selected,
          button: true,
          child: Material(
            color: selected
                ? scheme.primaryContainer.withValues(alpha: 0.35)
                : scheme.surfaceContainerLow,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(12),
              side: BorderSide(
                color: selected ? scheme.primary : scheme.outlineVariant,
              ),
            ),
            clipBehavior: Clip.antiAlias,
            child: InkWell(
              key: ValueKey('theme-source-$source'),
              onTap: () => _select(source),
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: LayoutBuilder(
                  builder: (context, constraints) {
                    final compact = constraints.maxWidth < 220;
                    final thumbnail = SizedBox(
                      width: compact ? 60 : 76,
                      height: compact ? 44 : 58,
                      child: snapshot == null
                          ? Icon(
                              Icons.extension_outlined,
                              color: scheme.primary,
                            )
                          : ClipRRect(
                              borderRadius: BorderRadius.circular(6),
                              child: ThemeAppearancePreview(
                                snapshot: snapshot,
                                assets: widget.assets,
                              ),
                            ),
                    );
                    final description = Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          title,
                          style: Theme.of(context).textTheme.bodyMedium
                              ?.copyWith(fontWeight: FontWeight.w600),
                        ),
                        if (subtitle != null) ...[
                          const SizedBox(height: 4),
                          Text(
                            subtitle,
                            style: Theme.of(context).textTheme.bodySmall
                                ?.copyWith(color: scheme.onSurfaceVariant),
                          ),
                        ],
                      ],
                    );
                    final selectedIcon = Icon(
                      selected
                          ? Icons.check_circle
                          : Icons.radio_button_unchecked,
                      size: 18,
                      color: selected ? scheme.primary : scheme.outline,
                    );
                    if (compact) {
                      return Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          Row(
                            children: [thumbnail, const Spacer(), selectedIcon],
                          ),
                          const SizedBox(height: 8),
                          description,
                        ],
                      );
                    }
                    return Row(
                      children: [
                        thumbnail,
                        const SizedBox(width: 12),
                        Expanded(child: description),
                        const SizedBox(width: 8),
                        selectedIcon,
                      ],
                    );
                  },
                ),
              ),
            ),
          ),
        ),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          l10n.settingsAppearanceThemeConfigSource,
          style: Theme.of(context).textTheme.titleSmall,
        ),
        const SizedBox(height: 12),
        option(
          'defaults',
          l10n.settingsAppearanceThemeConfigFromDefaults,
          snapshot: UserPreferencesManager.defaultThemePreferenceSnapshot,
          subtitle: l10n.settingsAppearanceThemeConfigDefaultsHint,
        ),
        option(
          'current',
          l10n.settingsAppearanceThemeConfigFromCurrent,
          snapshot: widget.current,
          subtitle: l10n.settingsAppearanceThemeConfigCurrentHint,
        ),
        if (widget.configs.isNotEmpty) ...[
          const SizedBox(height: 16),
          Text(
            l10n.settingsAppearanceThemeConfigSavedSources,
            style: Theme.of(context).textTheme.titleSmall,
          ),
          const SizedBox(height: 12),
          for (final config in widget.configs)
            option(
              'config:${config.id}',
              config.name,
              snapshot: ThemePreferenceSnapshot.fromJson(config.snapshot),
            ),
        ],
        if (widget.templates.isNotEmpty) ...[
          const SizedBox(height: 16),
          Text(
            l10n.settingsAppearanceThemeConfigPluginSources,
            style: Theme.of(context).textTheme.titleSmall,
          ),
          const SizedBox(height: 12),
          for (final template in widget.templates)
            option(
              'template:${template.packageName}:${template.templateId}',
              template.label,
            ),
        ],
      ],
    );
  }
}
