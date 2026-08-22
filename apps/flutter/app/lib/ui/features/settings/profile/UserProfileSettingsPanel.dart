// ignore_for_file: file_names

import 'dart:async';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../core/runtime/RuntimeBootstrapManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../../theme/OperitTheme.dart';
import '../../../theme/OperitThemeAssets.dart';
import '../../packages/screens/GitHubOAuthLoginDialog.dart';
import 'UserProfileSummaryTile.dart';

class UserProfileSettingsPanel extends StatefulWidget {
  const UserProfileSettingsPanel({super.key, this.onProfileChanged});

  final VoidCallback? onProfileChanged;

  /// Creates the profile editor state for the active runtime identity.
  @override
  State<UserProfileSettingsPanel> createState() =>
      _UserProfileSettingsPanelState();
}

class _UserProfileSettingsPanelState extends State<UserProfileSettingsPanel> {
  final GeneratedCoreProxyClients _clients = const GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );

  bool _loadingGitHub = true;
  bool _loggedIn = false;
  core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? _githubUser;
  Object? _githubError;

  /// Starts observing identity changes and loads the GitHub account state.
  @override
  void initState() {
    super.initState();
    RuntimeBootstrapManager.instance.addListener(_handleIdentityChanged);
    unawaited(_loadGitHubState());
  }

  /// Stops observing identity changes when the profile page is removed.
  @override
  void dispose() {
    RuntimeBootstrapManager.instance.removeListener(_handleIdentityChanged);
    super.dispose();
  }

  /// Builds the standalone profile, identity, and GitHub settings page.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final manager = RuntimeBootstrapManager.instance;
    final activeIdentity = manager.activeIdentity;
    final avatarUri = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.customUserAvatarUri;
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
      children: <Widget>[
        _ProfileOverview(
          avatarUri: avatarUri,
          name: runtimeIdentityDisplayName(activeIdentity, l10n),
          githubStatus: _githubStatus(l10n),
        ),
        const SizedBox(height: 12),
        _ProfileSection(
          title: l10n.settingsUserProfileOverview,
          children: <Widget>[
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: UserProfileAvatar(storagePath: avatarUri, size: 44),
              title: Text(l10n.settingsUserProfileAvatar),
              subtitle: Text(
                avatarUri == null || avatarUri.trim().isEmpty
                    ? l10n.settingsAppearanceAvatarDefault
                    : l10n.settingsAppearanceAvatarCustom,
              ),
            ),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                FilledButton.tonalIcon(
                  onPressed: _chooseAvatar,
                  icon: const Icon(Icons.image_outlined),
                  label: Text(l10n.settingsUserProfileChooseAvatar),
                ),
                OutlinedButton.icon(
                  onPressed: avatarUri == null || avatarUri.trim().isEmpty
                      ? null
                      : _clearAvatar,
                  icon: const Icon(Icons.person_off_outlined),
                  label: Text(l10n.settingsUserProfileClearAvatar),
                ),
              ],
            ),
            const SizedBox(height: 8),
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: const Icon(Icons.badge_outlined),
              title: Text(l10n.settingsUserProfileName),
              subtitle: Text(runtimeIdentityDisplayName(activeIdentity, l10n)),
              trailing: IconButton(
                tooltip: l10n.settingsUserProfileEditName,
                onPressed: () => _renameIdentity(activeIdentity),
                icon: const Icon(Icons.edit_outlined),
              ),
            ),
          ],
        ),
        _ProfileSection(
          title: l10n.settingsUserProfileIdentities,
          children: <Widget>[
            for (final identity in manager.identities)
              ListTile(
                contentPadding: EdgeInsets.zero,
                selected: identity.id == activeIdentity.id,
                leading: Icon(
                  identity.id == activeIdentity.id
                      ? Icons.radio_button_checked
                      : Icons.radio_button_unchecked,
                ),
                title: Text(runtimeIdentityDisplayName(identity, l10n)),
                subtitle: identity.id == activeIdentity.id
                    ? Text(l10n.runtimeIdentityCurrent)
                    : null,
                onTap: identity.id == activeIdentity.id
                    ? null
                    : () => _switchIdentity(identity),
                trailing: IconButton(
                  tooltip: l10n.runtimeIdentityRename,
                  onPressed: () => _renameIdentity(identity),
                  icon: const Icon(Icons.edit_outlined),
                ),
              ),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.tonalIcon(
                onPressed: _createIdentity,
                icon: const Icon(Icons.person_add_alt_1_outlined),
                label: Text(l10n.runtimeIdentityCreate),
              ),
            ),
          ],
        ),
        _ProfileSection(
          title: l10n.settingsUserProfileGitHub,
          children: <Widget>[
            if (_loadingGitHub)
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 16),
                child: LinearProgressIndicator(),
              )
            else if (_githubError != null)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 12),
                child: SelectableText(
                  l10n.settingsUserProfileGitHubStatusError(
                    _githubError.toString(),
                  ),
                ),
              )
            else
              ListTile(
                contentPadding: EdgeInsets.zero,
                leading: _GitHubAvatar(user: _githubUser),
                title: Text(
                  _loggedIn
                      ? _githubDisplayName(_githubUser!)
                      : l10n.settingsUserProfileNotLoggedIn,
                ),
                subtitle: _loggedIn
                    ? Text('@${_githubUser!.login}')
                    : Text(l10n.settingsUserProfileGitHubDescription),
                trailing: _loggedIn
                    ? OutlinedButton.icon(
                        onPressed: _logoutGitHub,
                        icon: const Icon(Icons.logout, size: 18),
                        label: Text(l10n.settingsUserProfileLogout),
                      )
                    : FilledButton.tonalIcon(
                        onPressed: _loginGitHub,
                        icon: const Icon(Icons.login, size: 18),
                        label: Text(l10n.settingsUserProfileLogin),
                      ),
              ),
          ],
        ),
      ],
    );
  }

  /// Rebuilds the identity list after bootstrap metadata changes.
  void _handleIdentityChanged() {
    if (mounted) {
      setState(() {});
    }
    widget.onProfileChanged?.call();
  }

  /// Selects and persists a new globally configured user avatar.
  Future<void> _chooseAvatar() async {
    try {
      const images = XTypeGroup(
        label: 'image',
        extensions: <String>['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif'],
      );
      final file = await openFile(acceptedTypeGroups: <XTypeGroup>[images]);
      if (file == null) {
        return;
      }
      final imported = await ThemeAssetStore().importFile(file);
      if (!mounted) {
        return;
      }
      await OperitTheme.of(context).saveActiveThemeUserAvatarSettings(
        customUserAvatarUri: imported.storagePath,
      );
      widget.onProfileChanged?.call();
    } catch (error, stackTrace) {
      _showError(error, stackTrace);
    }
  }

  /// Clears the configured user avatar while preserving other theme settings.
  Future<void> _clearAvatar() async {
    try {
      await OperitTheme.of(
        context,
      ).saveActiveThemeUserAvatarSettings(customUserAvatarUri: '');
      widget.onProfileChanged?.call();
    } catch (error, stackTrace) {
      _showError(error, stackTrace);
    }
  }

  /// Creates a new isolated identity with an optional display name.
  Future<void> _createIdentity() async {
    final l10n = AppLocalizations.of(context)!;
    final name = await _IdentityNameDialog.show(
      context: context,
      title: l10n.runtimeIdentityCreateTitle,
      label: l10n.runtimeIdentityName,
      initialName: '',
    );
    if (name == null) {
      return;
    }
    try {
      await RuntimeBootstrapManager.instance.createIdentity(name);
    } catch (error, stackTrace) {
      _showError(error, stackTrace);
    }
  }

  /// Renames one identity without moving its isolated storage directories.
  Future<void> _renameIdentity(RuntimeIdentity identity) async {
    final l10n = AppLocalizations.of(context)!;
    final name = await _IdentityNameDialog.show(
      context: context,
      title: l10n.runtimeIdentityRenameTitle,
      label: l10n.runtimeIdentityName,
      initialName: identity.name,
    );
    if (name == null || name == identity.name) {
      return;
    }
    try {
      final manager = RuntimeBootstrapManager.instance;
      final renamingActiveIdentity = identity.id == manager.activeIdentity.id;
      await manager.renameIdentity(identity.id, name);
      if (renamingActiveIdentity) {
        await _clients.server.runtimeRemoteLinkService.updateCurrentDeviceUserName(
          userName: name,
        );
      }
    } catch (error, stackTrace) {
      _showError(error, stackTrace);
    }
  }

  /// Confirms and activates another identity through the bootstrap manager.
  Future<void> _switchIdentity(RuntimeIdentity identity) async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      useRootNavigator: true,
      builder: (context) => AlertDialog(
        title: Text(
          l10n.runtimeIdentitySwitchTitle(
            runtimeIdentityDisplayName(identity, l10n),
          ),
        ),
        content: Text(l10n.runtimeIdentitySwitchDescription),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(MaterialLocalizations.of(context).cancelButtonLabel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(l10n.runtimeIdentitySwitchConfirm),
          ),
        ],
      ),
    );
    if (confirmed != true) {
      return;
    }
    try {
      await RuntimeBootstrapManager.instance.switchIdentity(identity.id);
    } catch (error, stackTrace) {
      _showError(error, stackTrace);
    }
  }

  /// Opens the existing visible GitHub OAuth flow for this identity.
  void _loginGitHub() {
    unawaited(
      showDialog<void>(
        context: context,
        barrierDismissible: false,
        builder: (context) => GitHubOAuthLoginDialog(
          clients: _clients,
          onLoginCompleted: _handleGitHubLoginCompleted,
        ),
      ),
    );
  }

  /// Reloads account state after GitHub OAuth completes successfully.
  Future<void> _handleGitHubLoginCompleted() async {
    await _loadGitHubState();
    widget.onProfileChanged?.call();
  }

  /// Clears the GitHub authentication session for the active identity.
  Future<void> _logoutGitHub() async {
    try {
      await _clients.preferencesGitHubAuthPreferences.logout();
      await _loadGitHubState();
      widget.onProfileChanged?.call();
    } catch (error, stackTrace) {
      _showError(error, stackTrace);
    }
  }

  /// Loads the exact GitHub login state and saved user profile from Core.
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

  /// Formats the current GitHub status for the profile overview.
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

  /// Logs one profile action failure and exposes its actual message to the user.
  void _showError(Object error, StackTrace stackTrace) {
    debugPrint('User profile action failed: $error\n$stackTrace');
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(error.toString()),
        behavior: SnackBarBehavior.floating,
      ),
    );
  }
}

class _ProfileOverview extends StatelessWidget {
  const _ProfileOverview({
    required this.avatarUri,
    required this.name,
    required this.githubStatus,
  });

  final String? avatarUri;
  final String name;
  final String githubStatus;

  /// Builds the first-viewport profile identity summary.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.primaryContainer.withValues(alpha: 0.52),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(12),
      border: Border.all(color: colorScheme.primary.withValues(alpha: 0.22)),
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Row(
          children: <Widget>[
            UserProfileAvatar(storagePath: avatarUri, size: 72),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    name,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.titleLarge?.copyWith(
                      color: colorScheme.onPrimaryContainer,
                      fontWeight: FontWeight.w800,
                    ),
                  ),
                  const SizedBox(height: 5),
                  Text(
                    githubStatus,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: colorScheme.onPrimaryContainer.withValues(
                        alpha: 0.72,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ProfileSection extends StatelessWidget {
  const _ProfileSection({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  /// Builds one un-nested profile settings section.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        layer: OperitGlassSurfaceLayer.card,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: Theme.of(
                  context,
                ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
              ),
              const SizedBox(height: 6),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

class _GitHubAvatar extends StatelessWidget {
  const _GitHubAvatar({required this.user});

  final core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? user;

  /// Builds the saved GitHub avatar or an explicit logged-out placeholder.
  @override
  Widget build(BuildContext context) {
    final currentUser = user;
    if (currentUser == null) {
      return const CircleAvatar(radius: 22, child: Icon(Icons.code_outlined));
    }
    return CircleAvatar(
      radius: 22,
      backgroundImage: NetworkImage(currentUser.avatarUrl),
    );
  }
}

class _IdentityNameDialog extends StatefulWidget {
  const _IdentityNameDialog({
    required this.title,
    required this.label,
    required this.initialName,
  });

  final String title;
  final String label;
  final String initialName;

  /// Opens the optional identity display-name editor.
  static Future<String?> show({
    required BuildContext context,
    required String title,
    required String label,
    required String initialName,
  }) {
    return showDialog<String>(
      context: context,
      useRootNavigator: true,
      builder: (context) => _IdentityNameDialog(
        title: title,
        label: label,
        initialName: initialName,
      ),
    );
  }

  /// Creates state that owns the dialog text controller.
  @override
  State<_IdentityNameDialog> createState() => _IdentityNameDialogState();
}

class _IdentityNameDialogState extends State<_IdentityNameDialog> {
  late final TextEditingController _controller;

  /// Initializes the editor with the current optional display name.
  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.initialName);
  }

  /// Disposes the dialog-owned text controller.
  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// Closes the dialog with the normalized optional display name.
  void _submit() {
    Navigator.of(context).pop(_controller.text.trim());
  }

  /// Builds the optional profile-name editor dialog.
  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.title),
      content: TextField(
        controller: _controller,
        autofocus: true,
        maxLength: 80,
        textInputAction: TextInputAction.done,
        decoration: InputDecoration(labelText: widget.label),
        onSubmitted: (_) => _submit(),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(MaterialLocalizations.of(context).cancelButtonLabel),
        ),
        FilledButton(
          onPressed: _submit,
          child: Text(MaterialLocalizations.of(context).okButtonLabel),
        ),
      ],
    );
  }
}

/// Returns the preferred GitHub account display name.
String _githubDisplayName(
  core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser user,
) {
  final name = user.name?.trim();
  return name == null || name.isEmpty ? user.login : name;
}
