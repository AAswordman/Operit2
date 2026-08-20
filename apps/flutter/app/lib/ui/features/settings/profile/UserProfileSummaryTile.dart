// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../core/runtime/RuntimeBootstrapManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../../theme/OperitTheme.dart';
import '../../../theme/OperitThemeAssets.dart';

class UserProfileSummaryTile extends StatefulWidget {
  const UserProfileSummaryTile({
    super.key,
    required this.selected,
    required this.revision,
    required this.onTap,
  });

  final bool selected;
  final int revision;
  final VoidCallback onTap;

  /// Creates state that observes identity metadata and GitHub account status.
  @override
  State<UserProfileSummaryTile> createState() => _UserProfileSummaryTileState();
}

class _UserProfileSummaryTileState extends State<UserProfileSummaryTile> {
  final GeneratedCoreProxyClients _clients = const GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );

  bool _loadingGitHub = true;
  bool _loggedIn = false;
  core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? _githubUser;
  Object? _githubError;

  /// Starts identity observation and loads the current GitHub session.
  @override
  void initState() {
    super.initState();
    RuntimeBootstrapManager.instance.addListener(_handleIdentityChanged);
    unawaited(_loadGitHubState());
  }

  /// Reloads GitHub state after the profile page reports a mutation.
  @override
  void didUpdateWidget(covariant UserProfileSummaryTile oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.revision != widget.revision) {
      unawaited(_loadGitHubState());
    }
  }

  /// Stops observing bootstrap identity metadata.
  @override
  void dispose() {
    RuntimeBootstrapManager.instance.removeListener(_handleIdentityChanged);
    super.dispose();
  }

  /// Builds the compact profile entry shown above all settings categories.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final identity = RuntimeBootstrapManager.instance.activeIdentity;
    final profileName = runtimeIdentityDisplayName(identity, l10n);
    final avatarUri = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.customUserAvatarUri;
    final foreground = widget.selected
        ? colorScheme.onPrimaryContainer
        : colorScheme.onSurface;
    final background = widget.selected
        ? colorScheme.primaryContainer
        : colorScheme.surfaceContainerHighest.withValues(alpha: 0.46);
    return Padding(
      padding: const EdgeInsets.fromLTRB(0, 0, 0, 12),
      child: OperitGlassSurface(
        color: background,
        layer: OperitGlassSurfaceLayer.card,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: widget.selected
              ? colorScheme.primary.withValues(alpha: 0.28)
              : colorScheme.outlineVariant.withValues(alpha: 0.2),
        ),
        material: true,
        child: InkWell(
          borderRadius: BorderRadius.circular(12),
          onTap: widget.onTap,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(12, 12, 10, 12),
            child: Row(
              children: <Widget>[
                UserProfileAvatar(storagePath: avatarUri, size: 48),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        profileName,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.titleSmall?.copyWith(
                          color: foreground,
                          fontWeight: FontWeight.w800,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Text(
                        _githubStatus(l10n),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: foreground.withValues(alpha: 0.7),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Icon(Icons.chevron_right, color: foreground, size: 20),
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// Rebuilds the summary after an identity is created or renamed.
  void _handleIdentityChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  /// Loads the exact GitHub session status from Core preferences.
  Future<void> _loadGitHubState() async {
    if (mounted) {
      setState(() {
        _loadingGitHub = true;
        _githubError = null;
      });
    }
    try {
      final auth = _clients.preferencesGitHubAuthPreferences;
      final loggedIn = await auth.isLoggedIn();
      final user = await auth.getCurrentUserInfo();
      if (loggedIn && user == null) {
        throw StateError('GitHub session has no user profile');
      }
      if (!mounted) {
        return;
      }
      setState(() {
        _loggedIn = loggedIn;
        _githubUser = user;
        _loadingGitHub = false;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _githubError = error;
        _loadingGitHub = false;
      });
    }
  }

  /// Formats the GitHub status displayed below the profile name.
  String _githubStatus(AppLocalizations l10n) {
    if (_loadingGitHub) {
      return l10n.settingsUserProfileGitHubLoading;
    }
    final error = _githubError;
    if (error != null) {
      return l10n.settingsUserProfileGitHubStatusError(error.toString());
    }
    if (!_loggedIn) {
      return l10n.settingsUserProfileNotLoggedIn;
    }
    return l10n.settingsUserProfileGitHubAccount(_githubUser!.login);
  }
}

class UserProfileAvatar extends StatelessWidget {
  const UserProfileAvatar({
    super.key,
    required this.storagePath,
    required this.size,
  });

  final String? storagePath;
  final double size;

  /// Builds either the configured user image or an empty profile placeholder.
  @override
  Widget build(BuildContext context) {
    final normalizedPath = storagePath?.trim();
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      width: size,
      height: size,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: colorScheme.surfaceContainerHighest,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.45),
        ),
      ),
      child: normalizedPath == null || normalizedPath.isEmpty
          ? Icon(
              Icons.person_outline,
              size: size * 0.54,
              color: colorScheme.onSurfaceVariant,
            )
          : ThemeAssetImage(storagePath: normalizedPath, fit: BoxFit.cover),
    );
  }
}

/// Returns the configured identity name or the localized unconfigured label.
String runtimeIdentityDisplayName(
  RuntimeIdentity identity,
  AppLocalizations l10n,
) {
  final name = identity.name.trim();
  return name.isEmpty ? l10n.settingsUserProfileUnnamed : name;
}
