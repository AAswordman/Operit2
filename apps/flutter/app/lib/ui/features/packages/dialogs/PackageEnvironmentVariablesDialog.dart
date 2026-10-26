// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../utils/PackageDisplayUtils.dart';

class PackageEnvironmentVariablesDialog extends StatefulWidget {
  /// Creates an editor backed by the runtime environment preference store.
  const PackageEnvironmentVariablesDialog({super.key, required this.clients});

  final GeneratedCoreProxyClients clients;

  /// Opens the environment editor for all currently enabled tool packages.
  static Future<void> show({
    required BuildContext context,
    required GeneratedCoreProxyClients clients,
  }) {
    return showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (context) => PackageEnvironmentVariablesDialog(clients: clients),
    );
  }

  /// Creates the state that loads and persists declared environment values.
  @override
  State<PackageEnvironmentVariablesDialog> createState() =>
      _PackageEnvironmentVariablesDialogState();
}

class _PackageEnvironmentVariablesDialogState
    extends State<PackageEnvironmentVariablesDialog> {
  final Map<(String, String), TextEditingController> _controllers = {};
  final Map<(String, String), String> _initialValues = {};
  final Map<String, String> _owners = {};
  List<core_proxy.ToolPackage> _packages = <core_proxy.ToolPackage>[];
  bool _loading = true;
  bool _loaded = false;
  bool _saving = false;
  Object? _error;

  /// Starts loading package declarations and their effective runtime values.
  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  /// Releases each controller owned by an installation and variable name.
  @override
  void dispose() {
    for (final controller in _controllers.values) {
      controller.dispose();
    }
    super.dispose();
  }

  /// Loads enabled packages, including containers and their subpackages.
  Future<void> _load() async {
    try {
      final manager = widget.clients.application.packageManager();
      final available = await manager.getAvailablePackages();
      final enabled = (await manager.getEnabledPackageNames()).toSet();
      final packages =
          available.values
              .where(
                (package) =>
                    enabled.contains(package.name) && package.env.isNotEmpty,
              )
              .toList()
            ..sort((left, right) => left.name.compareTo(right.name));
      final preferences = widget.clients.preferences.envPreferences;
      final environments = await Future.wait(
        packages.map(
          (package) =>
              preferences.getPackageEnvironment(packageName: package.name),
        ),
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _packages = packages;
        for (var index = 0; index < packages.length; index++) {
          final package = packages[index];
          final environment = environments[index];
          _owners[package.name] = environment.ownerId;
          for (final variable in package.env) {
            final key = (environment.ownerId, variable.name);
            final controller = _controllers.putIfAbsent(
              key,
              () => TextEditingController(
                text: environment.variables[variable.name],
              ),
            );
            _initialValues[key] = controller.text;
          }
        }
        _loading = false;
        _loaded = true;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loading = false;
        _error = error;
      });
    }
  }

  /// Saves changed keys in their owning installation without replacing unrelated values.
  Future<void> _save() async {
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final preferences = widget.clients.preferences.envPreferences;
      final changes = <String, Map<String, String?>>{};
      for (final entry in _controllers.entries) {
        final value = entry.value.text;
        if (value == _initialValues[entry.key]) {
          continue;
        }
        final ownerChanges = changes.putIfAbsent(entry.key.$1, () => {});
        ownerChanges[entry.key.$2] = value.trim().isEmpty ? null : value;
      }
      for (final entry in changes.entries) {
        await preferences.editPackageEnvironment(
          packageName: entry.key,
          changes: entry.value,
        );
      }
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _saving = false;
        _error = error;
      });
    }
  }

  /// Builds the grouped editor with explicit loading and persistence errors.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    return PopScope(
      canPop: !_saving,
      child: AlertDialog(
        title: Text(l10n.packageConfigureEnvironmentVariables),
        content: SizedBox(
          width: 560,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              if (_loading)
                const Padding(
                  padding: EdgeInsets.all(24),
                  child: Center(child: M3LoadingIndicator()),
                ),
              if (_loaded && _packages.isEmpty)
                Text(l10n.packageNoEnvironmentVariables),
              if (_loaded && _packages.isNotEmpty)
                Flexible(
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxHeight: 400),
                    child: SingleChildScrollView(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: <Widget>[
                          for (final package in _packages) ...<Widget>[
                            Padding(
                              padding: const EdgeInsets.symmetric(vertical: 8),
                              child: Row(
                                children: <Widget>[
                                  const Icon(
                                    Icons.extension_outlined,
                                    size: 24,
                                  ),
                                  const SizedBox(width: 8),
                                  Expanded(
                                    child: Text(
                                      package.name,
                                      style: theme.textTheme.titleSmall,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                            for (final env in package.env)
                              _buildVariable(context, package.name, env),
                          ],
                        ],
                      ),
                    ),
                  ),
                ),
              if (_error != null) ...<Widget>[
                const SizedBox(height: 12),
                Text(
                  _error.toString(),
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ],
            ],
          ),
        ),
        actions: <Widget>[
          TextButton(
            onPressed: _saving ? null : () => Navigator.of(context).pop(),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: _loaded && !_saving ? _save : null,
            child: Text(l10n.save),
          ),
        ],
      ),
    );
  }

  /// Shows declaration metadata without applying the declared default value.
  Widget _buildVariable(
    BuildContext context,
    String packageName,
    core_proxy.EnvVar env,
  ) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final description = localizedText(env.description);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Wrap(
            spacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: <Widget>[
              Text(env.name, style: theme.textTheme.labelLarge),
              Text(
                env.requiredValue
                    ? l10n.required
                    : l10n.packageEnvironmentOptional,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: env.requiredValue
                      ? theme.colorScheme.error
                      : theme.colorScheme.onSurfaceVariant,
                ),
              ),
              if (env.defaultValue != null)
                Text(
                  l10n.packageEnvironmentDefault(env.defaultValue!),
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.primary,
                  ),
                ),
            ],
          ),
          if (description.trim().isNotEmpty)
            Text(
              description,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall,
            ),
          const SizedBox(height: 4),
          TextField(
            key: ValueKey<String>('package-env:$packageName:${env.name}'),
            controller: _controllers[(_owners[packageName]!, env.name)]!,
            enabled: !_saving,
            maxLines: 1,
            autocorrect: false,
            enableSuggestions: false,
            decoration: InputDecoration(
              isDense: true,
              border: const OutlineInputBorder(),
              hintText: env.requiredValue
                  ? l10n.packageEnvironmentInputRequired
                  : l10n.packageEnvironmentInputOptional,
            ),
          ),
        ],
      ),
    );
  }
}
