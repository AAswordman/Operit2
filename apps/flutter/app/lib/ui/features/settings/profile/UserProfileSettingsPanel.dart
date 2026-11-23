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
import '../../packages/components/StoreVisuals.dart';
import '../../packages/screens/GitHubOAuthLoginDialog.dart';
import 'UserProfileSummaryTile.dart';

enum _IdentityAction { rename, chooseAvatar, clearAvatar, delete }

class UserProfileSettingsPanel extends StatefulWidget {
  /// Creates the identity settings panel with its Core API client.
  const UserProfileSettingsPanel({
    super.key,
    this.onProfileChanged,
    this.clients = const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge()),
  });

  final VoidCallback? onProfileChanged;
  final GeneratedCoreProxyClients clients;

  /// Creates the profile editor state for the active runtime identity.
  @override
  State<UserProfileSettingsPanel> createState() =>
      _UserProfileSettingsPanelState();
}

class _UserProfileSettingsPanelState extends State<UserProfileSettingsPanel> {
  /// Returns the Core clients used by account and active-device actions.
  GeneratedCoreProxyClients get _clients => widget.clients;

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
    final hasAvatar = avatarUri != null && avatarUri.trim().isNotEmpty;
    final otherIdentities = manager.identities
        .where((identity) => identity.id != activeIdentity.id)
        .toList(growable: false);
    final scheme = Theme.of(context).colorScheme;
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
      children: <Widget>[
        StoreContentWidth(
          maxWidth: 640,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              _ProfileCard(
                child: Column(
                  children: <Widget>[
                    _ProfileRow(
                      title: l10n.settingsUserProfileIdentities,
                      action: FilledButton.tonalIcon(
                        onPressed: _createIdentity,
                        style: FilledButton.styleFrom(
                          padding: const EdgeInsets.symmetric(horizontal: 12),
                        ),
                        icon: const Icon(Icons.add_rounded, size: 18),
                        label: Text(l10n.runtimeIdentityCreate),
                      ),
                      actionWidthFraction: 0.58,
                    ),
                    ColoredBox(
                      color: scheme.primaryContainer.withValues(alpha: 0.30),
                      child: _ProfileRow(
                        leading: hasAvatar
                            ? UserProfileAvatar(
                                storagePath: avatarUri,
                                size: 40,
                              )
                            : const _ProfileGlyph(
                                icon: Icons.person_rounded,
                                current: true,
                              ),
                        title: runtimeIdentityDisplayName(activeIdentity, l10n),
                        subtitle: l10n.runtimeIdentity,
                        current: true,
                        menu: _identityMenu(
                          activeIdentity,
                          isCurrent: true,
                          hasAvatar: hasAvatar,
                        ),
                      ),
                    ),
                    for (final identity in otherIdentities) ...<Widget>[
                      Divider(
                        height: 1,
                        indent: 68,
                        endIndent: 16,
                        color: storeCardBorderColor(scheme),
                      ),
                      _ProfileRow(
                        leading: const _ProfileGlyph(
                          icon: Icons.person_outline_rounded,
                        ),
                        title: runtimeIdentityDisplayName(identity, l10n),
                        action: TextButton(
                          onPressed: () => _switchIdentity(identity),
                          child: Text(l10n.runtimeIdentityActivate),
                        ),
                        menu: _identityMenu(identity),
                      ),
                    ],
                  ],
                ),
              ),
              const SizedBox(height: 16),
              _buildGitHubCard(l10n),
            ],
          ),
        ),
      ],
    );
  }

  /// Shows operations that apply to the selected identity.
  Widget _identityMenu(
    RuntimeIdentity identity, {
    bool isCurrent = false,
    bool hasAvatar = false,
  }) {
    final l10n = AppLocalizations.of(context)!;
    final scheme = Theme.of(context).colorScheme;
    return PopupMenuButton<_IdentityAction>(
      key: ValueKey<String>('identity-actions-${identity.id}'),
      tooltip: l10n.runtimeIdentityActions,
      icon: const Icon(Icons.more_horiz_rounded),
      onSelected: (action) {
        switch (action) {
          case _IdentityAction.rename:
            unawaited(_renameIdentity(identity));
          case _IdentityAction.chooseAvatar:
            unawaited(_chooseAvatar());
          case _IdentityAction.clearAvatar:
            unawaited(_clearAvatar());
          case _IdentityAction.delete:
            unawaited(_deleteIdentity(identity));
        }
      },
      itemBuilder: (context) => <PopupMenuEntry<_IdentityAction>>[
        PopupMenuItem<_IdentityAction>(
          value: _IdentityAction.rename,
          child: _menuLabel(Icons.edit_outlined, l10n.runtimeIdentityRename),
        ),
        if (isCurrent) ...<PopupMenuEntry<_IdentityAction>>[
          PopupMenuItem<_IdentityAction>(
            value: _IdentityAction.chooseAvatar,
            child: _menuLabel(
              Icons.image_outlined,
              l10n.settingsUserProfileChooseAvatar,
            ),
          ),
          if (hasAvatar)
            PopupMenuItem<_IdentityAction>(
              value: _IdentityAction.clearAvatar,
              child: _menuLabel(
                Icons.person_off_outlined,
                l10n.settingsUserProfileClearAvatar,
              ),
            ),
        ] else
          PopupMenuItem<_IdentityAction>(
            value: _IdentityAction.delete,
            child: _menuLabel(
              Icons.delete_outline_rounded,
              l10n.runtimeIdentityDelete,
              color: scheme.error,
            ),
          ),
      ],
    );
  }

  /// Builds the icon and label for a menu command.
  Widget _menuLabel(IconData icon, String label, {Color? color}) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Icon(icon, size: 19, color: color),
        const SizedBox(width: 10),
        Flexible(
          child: Text(label, style: TextStyle(color: color)),
        ),
      ],
    );
  }

  /// Displays account status and the relevant GitHub action.
  Widget _buildGitHubCard(AppLocalizations l10n) {
    final user = _githubUser;
    final error = _githubError;
    final status = switch ((error, _loadingGitHub, _loggedIn)) {
      (_, true, _) => l10n.settingsUserProfileGitHubLoading,
      (final Object error, false, _) =>
        l10n.settingsUserProfileGitHubStatusError(error.toString()),
      (_, false, true) => _githubAccountLabel(user!),
      _ => l10n.settingsUserProfileNotLoggedIn,
    };
    return _ProfileCard(
      child: _ProfileRow(
        leading: _loggedIn && user != null
            ? CircleAvatar(
                radius: 20,
                backgroundImage: NetworkImage(user.avatarUrl),
              )
            : const _ProfileGlyph(icon: Icons.code_rounded),
        title: l10n.settingsUserProfileGitHub,
        subtitle: status,
        action: _loadingGitHub
            ? const Padding(
                padding: EdgeInsets.all(10),
                child: SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              )
            : _loggedIn || error != null
            ? TextButton(
                onPressed: _logoutGitHub,
                child: Text(l10n.settingsUserProfileLogout),
              )
            : TextButton(
                onPressed: _loginGitHub,
                child: Text(l10n.settingsUserProfileLogin),
              ),
      ),
    );
  }

  /// Shows the GitHub handle with its display name when set.
  String _githubAccountLabel(
    core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser user,
  ) {
    final name = user.name?.trim();
    return name == null || name.isEmpty
        ? '@${user.login}'
        : '$name · @${user.login}';
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
        await _clients.server.runtimeRemoteLinkService
            .updateCurrentDeviceUserName(userName: name);
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

  /// Confirms removal of an inactive identity from this installation.
  Future<void> _deleteIdentity(RuntimeIdentity identity) async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        icon: Icon(
          Icons.delete_outline_rounded,
          color: Theme.of(context).colorScheme.error,
        ),
        title: Text(
          l10n.runtimeIdentityDeleteTitle(
            runtimeIdentityDisplayName(identity, l10n),
          ),
        ),
        content: Text(l10n.runtimeIdentityDeleteDescription),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(MaterialLocalizations.of(context).cancelButtonLabel),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
              foregroundColor: Theme.of(context).colorScheme.onError,
            ),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(l10n.runtimeIdentityDelete),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    try {
      await RuntimeBootstrapManager.instance.deleteIdentity(identity.id);
    } catch (error, stackTrace) {
      _showError(error, stackTrace);
    }
  }

  /// Opens the shared GitHub OAuth browser-choice flow for this identity.
  void _loginGitHub() {
    unawaited(
      showGitHubOAuthLoginDialog(
        context: context,
        clients: _clients,
        onLoginCompleted: _handleGitHubLoginCompleted,
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
      var loggedIn = await auth.isLoggedIn();
      final user = await auth.getCurrentUserInfo();
      if (loggedIn && user == null) {
        // An interrupted legacy login can leave a session without its profile.
        // Clear it so this page recovers instead of trapping the user in an
        // unrecoverable error state.
        await auth.logout();
        loggedIn = false;
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

class _ProfileCard extends StatelessWidget {
  /// Groups identity or account rows into one surface.
  const _ProfileCard({required this.child});

  final Widget child;

  /// Applies the same surface and border used by the package management cards.
  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: storeCardColor(scheme),
      borderRadius: storeCardRadius,
      border: Border.all(color: storeCardBorderColor(scheme)),
      material: true,
      child: child,
    );
  }
}

class _ProfileRow extends StatelessWidget {
  /// Describes a row with a flexible title and trailing account controls.
  const _ProfileRow({
    required this.title,
    this.leading,
    this.subtitle,
    this.current = false,
    this.action,
    this.actionWidthFraction = 0.42,
    this.menu,
  });

  final Widget? leading;
  final String title;
  final String? subtitle;
  final bool current;
  final Widget? action;
  final double actionWidthFraction;
  final Widget? menu;

  /// Lays out the row while reserving room for long identity names.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final subtitle = this.subtitle;
    return Padding(
      padding: const EdgeInsetsDirectional.fromSTEB(16, 8, 12, 8),
      child: LayoutBuilder(
        builder: (context, constraints) => Row(
          children: <Widget>[
            if (leading case final leading?) ...<Widget>[
              leading,
              const SizedBox(width: 12),
            ],
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    title,
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  if (subtitle != null) ...<Widget>[
                    const SizedBox(height: 2),
                    Text(
                      subtitle,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: current
                            ? theme.colorScheme.primary
                            : theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ],
              ),
            ),
            if (action case final action?) ...<Widget>[
              const SizedBox(width: 8),
              ConstrainedBox(
                constraints: BoxConstraints(
                  maxWidth: constraints.maxWidth * actionWidthFraction,
                ),
                child: action,
              ),
            ],
            ?menu,
          ],
        ),
      ),
    );
  }
}

class _ProfileGlyph extends StatelessWidget {
  /// Describes the icon shown in place of a row image.
  const _ProfileGlyph({required this.icon, this.current = false});

  final IconData icon;
  final bool current;

  /// Builds a tinted square that matches the extension catalog icon treatment.
  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      width: 40,
      height: 40,
      decoration: BoxDecoration(
        color: current
            ? scheme.primaryContainer
            : scheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(10),
      ),
      child: Icon(
        icon,
        size: 20,
        color: current ? scheme.onPrimaryContainer : scheme.onSurfaceVariant,
      ),
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
