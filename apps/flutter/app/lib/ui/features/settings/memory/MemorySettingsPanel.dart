// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/CharacterAvatar.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../common/components/OperitDialog.dart';
import '../characters/MemoryGraphScreen.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

const String _memoryBindingShared = 'SHARED';

String _characterOwnerKey(String cardId) => 'character:$cardId';

String _sharedOwnerKey(String storeId) => 'shared:$storeId';

class MemorySettingsPanel extends StatefulWidget {
  const MemorySettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<MemorySettingsPanel> createState() => _MemorySettingsPanelState();
}

class _MemorySettingsPanelState extends State<MemorySettingsPanel> {
  Future<_MemorySettingsData>? _future;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  GeneratedApplicationUserMarkdownRepositoryCoreProxy _userMarkdownRepository(
    String ownerKey,
  ) {
    return widget.clients.application.userMarkdownRepository(
      ownerKey: ownerKey,
    );
  }

  Future<_MemorySettingsData> _load() async {
    final cardManager = widget.clients.preferencesCharacterCardManager;
    final sharedMemoryManager =
        widget.clients.preferencesSharedMemoryStoreManager;
    final apiPreferences = widget.clients.preferencesApiPreferences;
    final activePromptManager = widget.clients.preferencesActivePromptManager;

    final cards = await cardManager.getAllCharacterCards();
    final sharedMemoryStores =
        await sharedMemoryManager.getAllSharedMemoryStores();
    final activePrompt = await activePromptManager.getActivePrompt();
    final enableMemoryAutoUpdate =
        await apiPreferences.enableMemoryAutoUpdateFlow().first;
    final disableUserPreferenceDescription =
        await apiPreferences.disableUserPreferenceDescriptionFlow().first;

    String? activeCardId;
    if (activePrompt != null &&
        activePrompt.tag == 'CharacterCard' &&
        activePrompt.id.trim().isNotEmpty) {
      activeCardId = activePrompt.id.trim();
    }

    return _MemorySettingsData(
      cards: cards,
      sharedMemoryStores: sharedMemoryStores,
      activeCardId: activeCardId,
      enableMemoryAutoUpdate: enableMemoryAutoUpdate,
      disableUserPreferenceDescription: disableUserPreferenceDescription,
    );
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _saveMemoryAutoUpdate(bool enabled) async {
    await widget.clients.preferencesApiPreferences.saveEnableMemoryAutoUpdate(
      isEnabled: enabled,
    );
    _reload();
  }

  Future<void> _savePreferenceDescription(bool enabled) async {
    await widget.clients.preferencesApiPreferences
        .saveDisableUserPreferenceDescription(isDisabled: !enabled);
    _reload();
  }

  Future<void> _createSharedMemoryStore() async {
    final edited = await _SharedMemoryStoreEditorDialog.show(
      context: context,
      title: '新建共享记忆库',
    );
    if (edited == null) {
      return;
    }
    await widget.clients.preferencesSharedMemoryStoreManager
        .createSharedMemoryStore(name: edited.name);
    _reload();
  }

  Future<void> _renameSharedMemoryStore(
    core_proxy.SharedMemoryStore store,
  ) async {
    final edited = await _SharedMemoryStoreEditorDialog.show(
      context: context,
      title: '编辑共享记忆库',
      store: store,
    );
    if (edited == null) {
      return;
    }
    await widget.clients.preferencesSharedMemoryStoreManager
        .renameSharedMemoryStore(id: store.id, name: edited.name);
    _reload();
  }

  Future<void> _deleteSharedMemoryStore(
    core_proxy.SharedMemoryStore store,
  ) async {
    await widget.clients.preferencesSharedMemoryStoreManager
        .deleteSharedMemoryStore(id: store.id);
    _reload();
  }

  Future<void> _editOwnerUserMarkdown({
    required String ownerKey,
    required String titleName,
  }) async {
    final l10n = AppLocalizations.of(context)!;
    final repository = _userMarkdownRepository(ownerKey);
    final content = await repository.readUserMarkdown();
    if (!mounted) {
      return;
    }
    final edited = await _UserMarkdownEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersUserMarkdownTitle(titleName),
      initialText: content,
    );
    if (edited == null) {
      return;
    }
    await repository.writeUserMarkdown(content: edited);
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(l10n.settingsCharactersUserMarkdownSaved)),
    );
  }

  Future<void> _openMemoryGraph({
    required String ownerKey,
    required String titleName,
  }) async {
    await MemoryGraphScreen.open(
      context: context,
      bridge: widget.clients.bridge,
      ownerKey: ownerKey,
      ownerName: titleName,
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return FutureBuilder<_MemorySettingsData>(
      future: _future,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
        }
        final data = snapshot.data;
        if (data == null) {
          return const M3LoadingPane();
        }
        final storesById = <String, core_proxy.SharedMemoryStore>{
          for (final store in data.sharedMemoryStores) store.id: store,
        };
        return ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
          children: <Widget>[
            _MemorySectionCard(
              title: '全局记忆设置',
              icon: Icons.tune_outlined,
              children: <Widget>[
                _MemorySwitchRow(
                  title: l10n.settingsCharactersPreferenceDescription,
                  subtitle:
                      l10n.settingsCharactersPreferenceDescriptionSubtitle,
                  value: !data.disableUserPreferenceDescription,
                  onChanged: _savePreferenceDescription,
                ),
                const SizedBox(height: 6),
                _MemorySwitchRow(
                  title: l10n.settingsCharactersMemoryAutoUpdate,
                  subtitle:
                      l10n.settingsCharactersMemoryAutoUpdateDescription,
                  value: data.enableMemoryAutoUpdate,
                  onChanged: _saveMemoryAutoUpdate,
                ),
              ],
            ),
            _MemorySectionCard(
              title: '共享记忆库',
              icon: Icons.hub_outlined,
              action: SettingsSectionAddButton(
                tooltip: l10n.create,
                label: l10n.create,
                onPressed: _createSharedMemoryStore,
              ),
              children: <Widget>[
                if (data.sharedMemoryStores.isEmpty)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Text(
                      '当前还没有共享记忆库，创建后可被多个角色卡挂载共享。',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  )
                else
                  for (final store in data.sharedMemoryStores)
                    _SharedMemoryStoreTile(
                      store: store,
                      mountedCardCount: data.cards
                          .where(
                            (card) =>
                                card.memoryBindingMode
                                        .trim()
                                        .toUpperCase() ==
                                    _memoryBindingShared &&
                                card.sharedMemoryId == store.id,
                          )
                          .length,
                      onEdit: () => _renameSharedMemoryStore(store),
                      onDelete: () => _deleteSharedMemoryStore(store),
                      onEditUserMarkdown: () => _editOwnerUserMarkdown(
                        ownerKey: _sharedOwnerKey(store.id),
                        titleName: store.name,
                      ),
                      onOpenMemoryGraph: () => _openMemoryGraph(
                        ownerKey: _sharedOwnerKey(store.id),
                        titleName: store.name,
                      ),
                    ),
              ],
            ),
            _MemorySectionCard(
              title: '角色记忆库',
              icon: Icons.badge_outlined,
              children: <Widget>[
                for (final card in data.cards)
                  _CharacterMemoryTile(
                    card: card,
                    active: card.id == data.activeCardId,
                    sharedStoreName:
                        card.sharedMemoryId == null
                            ? null
                            : storesById[card.sharedMemoryId!]?.name,
                    onEditUserMarkdown: () => _editOwnerUserMarkdown(
                      ownerKey: _characterOwnerKey(card.id),
                      titleName: card.name,
                    ),
                    onOpenMemoryGraph: () => _openMemoryGraph(
                      ownerKey: _characterOwnerKey(card.id),
                      titleName: card.name,
                    ),
                  ),
              ],
            ),
          ],
        );
      },
    );
  }
}

class _MemorySettingsData {
  const _MemorySettingsData({
    required this.cards,
    required this.sharedMemoryStores,
    required this.activeCardId,
    required this.enableMemoryAutoUpdate,
    required this.disableUserPreferenceDescription,
  });

  final List<core_proxy.CharacterCard> cards;
  final List<core_proxy.SharedMemoryStore> sharedMemoryStores;
  final String? activeCardId;
  final bool enableMemoryAutoUpdate;
  final bool disableUserPreferenceDescription;
}

class _MemorySectionCard extends StatelessWidget {
  const _MemorySectionCard({
    required this.title,
    required this.children,
    this.icon,
    this.action,
    this.initiallyExpanded = true,
  });

  final String title;
  final List<Widget> children;
  final IconData? icon;
  final Widget? action;
  final bool initiallyExpanded;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Material(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        shape: RoundedRectangleBorder(
          borderRadius: radius,
          side: BorderSide(
            color: colorScheme.outlineVariant.withValues(alpha: 0.18),
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: OperitGlassSurface(
          color: Colors.transparent,
          borderRadius: radius,
          material: true,
          clip: false,
          child: ExpansionTile(
            initiallyExpanded: initiallyExpanded,
            tilePadding: const EdgeInsets.symmetric(horizontal: 14),
            childrenPadding: const EdgeInsets.fromLTRB(14, 0, 14, 12),
            shape: RoundedRectangleBorder(borderRadius: radius),
            collapsedShape: RoundedRectangleBorder(borderRadius: radius),
            title: Row(
              children: <Widget>[
                if (icon != null) ...<Widget>[
                  Icon(icon, size: 18, color: colorScheme.onSurfaceVariant),
                  const SizedBox(width: 8),
                ],
                Expanded(
                  child: Text(
                    title,
                    style: SettingsControlStyles.sectionTitleTextStyle(context),
                  ),
                ),
                if (action != null) ...<Widget>[
                  action!,
                  const SizedBox(width: 4),
                ],
              ],
            ),
            children: children,
          ),
        ),
      ),
    );
  }
}

class _MemorySwitchRow extends StatelessWidget {
  const _MemorySwitchRow({
    required this.title,
    required this.subtitle,
    required this.value,
    required this.onChanged,
  });

  final String title;
  final String subtitle;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.24),
        ),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 8, 10, 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: <Widget>[
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    title,
                    style: textTheme.bodyMedium?.copyWith(
                      color: colorScheme.onSurface,
                      fontWeight: FontWeight.w700,
                      fontSize: 14,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    subtitle,
                    style: textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                      fontSize: 12,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Transform.scale(
              scale: 0.82,
              child: Switch(value: value, onChanged: onChanged),
            ),
          ],
        ),
      ),
    );
  }
}

class _SharedMemoryStoreTile extends StatelessWidget {
  const _SharedMemoryStoreTile({
    required this.store,
    required this.mountedCardCount,
    required this.onEdit,
    required this.onDelete,
    required this.onEditUserMarkdown,
    required this.onOpenMemoryGraph,
  });

  final core_proxy.SharedMemoryStore store;
  final int mountedCardCount;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final VoidCallback onEditUserMarkdown;
  final VoidCallback onOpenMemoryGraph;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return _MemoryEntityTile(
      leading: const Icon(Icons.hub_outlined),
      title: Text(store.name),
      badges: <Widget>[
        SettingsInfoBadge(label: _sharedOwnerKey(store.id)),
        if (mountedCardCount > 0)
          SettingsInfoBadge(label: '$mountedCardCount 个角色已挂载'),
      ],
      onTap: onEdit,
      actions: <Widget>[
        SettingsEntityIconButton(
          tooltip: l10n.settingsCharactersOpenMemoryGraph,
          icon: Icons.account_tree_outlined,
          onPressed: onOpenMemoryGraph,
        ),
        SettingsEntityIconButton(
          tooltip: l10n.settingsCharactersEditUserMarkdown,
          icon: Icons.assignment_ind_outlined,
          onPressed: onEditUserMarkdown,
        ),
        SettingsEntityIconButton(
          tooltip: l10n.delete,
          icon: Icons.delete_outline,
          onPressed: onDelete,
        ),
      ],
    );
  }
}

class _CharacterMemoryTile extends StatelessWidget {
  const _CharacterMemoryTile({
    required this.card,
    required this.active,
    required this.sharedStoreName,
    required this.onEditUserMarkdown,
    required this.onOpenMemoryGraph,
  });

  final core_proxy.CharacterCard card;
  final bool active;
  final String? sharedStoreName;
  final VoidCallback onEditUserMarkdown;
  final VoidCallback onOpenMemoryGraph;

  String _bindingSummary() {
    if (card.memoryBindingMode.trim().toUpperCase() == _memoryBindingShared) {
      final resolvedName = sharedStoreName?.trim();
      if (resolvedName != null && resolvedName.isNotEmpty) {
        return '共享记忆：$resolvedName';
      }
      final sharedId = card.sharedMemoryId?.trim();
      if (sharedId != null && sharedId.isNotEmpty) {
        return '共享记忆：$sharedId';
      }
      return '共享记忆';
    }
    return '角色独立记忆';
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final descriptionText = card.description.trim();
    return _MemoryEntityTile(
      active: active,
      leading: _MemoryListAvatar(
        avatar: CharacterAvatarImage(
          avatarUri: card.avatarUri,
          fit: BoxFit.cover,
        ),
        active: active,
      ),
      title: Text(card.name),
      subtitle: descriptionText.isEmpty ? null : Text(descriptionText),
      badges: <Widget>[
        SettingsInfoBadge(label: _bindingSummary()),
        SettingsInfoBadge(label: _characterOwnerKey(card.id)),
      ],
      onTap: onOpenMemoryGraph,
      actions: <Widget>[
        SettingsEntityIconButton(
          tooltip: l10n.settingsCharactersOpenMemoryGraph,
          icon: Icons.account_tree_outlined,
          onPressed: onOpenMemoryGraph,
        ),
        SettingsEntityIconButton(
          tooltip: l10n.settingsCharactersEditUserMarkdown,
          icon: Icons.assignment_ind_outlined,
          onPressed: onEditUserMarkdown,
        ),
      ],
    );
  }
}

class _MemoryListAvatar extends StatelessWidget {
  const _MemoryListAvatar({required this.avatar, required this.active});

  final Widget avatar;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox(
      width: 32,
      height: 32,
      child: Stack(
        clipBehavior: Clip.none,
        children: <Widget>[
          Positioned.fill(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest,
                shape: BoxShape.circle,
              ),
              child: ClipOval(
                child: IconTheme(
                  data: IconThemeData(color: colorScheme.onSurfaceVariant),
                  child: avatar,
                ),
              ),
            ),
          ),
          if (active)
            Positioned(
              right: -1,
              bottom: -1,
              child: Container(
                width: 14,
                height: 14,
                decoration: BoxDecoration(
                  color: colorScheme.primary,
                  shape: BoxShape.circle,
                  border: Border.all(color: colorScheme.surface, width: 1.5),
                ),
                child: Icon(Icons.check, color: colorScheme.onPrimary, size: 9),
              ),
            ),
        ],
      ),
    );
  }
}

class _MemoryEntityTile extends StatelessWidget {
  const _MemoryEntityTile({
    required this.leading,
    required this.title,
    required this.actions,
    this.subtitle,
    this.badges = const <Widget>[],
    this.active = false,
    this.onTap,
  });

  final Widget leading;
  final Widget title;
  final Widget? subtitle;
  final List<Widget> badges;
  final List<Widget> actions;
  final bool active;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Material(
        color: active
            ? colorScheme.primaryContainer.withValues(alpha: 0.16)
            : colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
        shape: RoundedRectangleBorder(
          borderRadius: radius,
          side: BorderSide(
            color: active
                ? colorScheme.primary.withValues(alpha: 0.45)
                : colorScheme.outlineVariant.withValues(alpha: 0.28),
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          borderRadius: radius,
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            child: LayoutBuilder(
              builder: (context, constraints) {
                final content = Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: <Widget>[
                    SizedBox(
                      width: 34,
                      child: IconTheme.merge(
                        data: IconThemeData(
                          color: colorScheme.onSurfaceVariant,
                          size: 20,
                        ),
                        child: leading,
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: DefaultTextStyle.merge(
                        style: TextStyle(color: colorScheme.onSurface),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisSize: MainAxisSize.min,
                          children: <Widget>[
                            DefaultTextStyle.merge(
                              style: Theme.of(context).textTheme.titleSmall!
                                  .copyWith(fontWeight: FontWeight.w700),
                              child: title,
                            ),
                            if (subtitle != null) ...<Widget>[
                              const SizedBox(height: 2),
                              DefaultTextStyle.merge(
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: Theme.of(context).textTheme.bodySmall!
                                    .copyWith(
                                      color: colorScheme.onSurfaceVariant,
                                      height: 1.25,
                                    ),
                                child: subtitle!,
                              ),
                            ],
                            if (badges.isNotEmpty) ...<Widget>[
                              const SizedBox(height: 5),
                              Wrap(
                                spacing: 6,
                                runSpacing: 4,
                                children: badges,
                              ),
                            ],
                          ],
                        ),
                      ),
                    ),
                  ],
                );
                final actionBar = Align(
                  alignment: Alignment.centerRight,
                  child: Wrap(
                    spacing: 4,
                    runSpacing: 4,
                    alignment: WrapAlignment.end,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: actions,
                  ),
                );
                if (actions.isEmpty) {
                  return content;
                }
                if (constraints.maxWidth < 390) {
                  return Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: <Widget>[
                      content,
                      const SizedBox(height: 6),
                      actionBar,
                    ],
                  );
                }
                return Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: <Widget>[
                    Expanded(child: content),
                    const SizedBox(width: 8),
                    ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 190),
                      child: actionBar,
                    ),
                  ],
                );
              },
            ),
          ),
        ),
      ),
    );
  }
}

class _SharedMemoryStoreEditResult {
  const _SharedMemoryStoreEditResult({required this.name});

  final String name;
}

class _SharedMemoryStoreEditorDialog extends StatefulWidget {
  const _SharedMemoryStoreEditorDialog({required this.title, this.store});

  final String title;
  final core_proxy.SharedMemoryStore? store;

  static Future<_SharedMemoryStoreEditResult?> show({
    required BuildContext context,
    required String title,
    core_proxy.SharedMemoryStore? store,
  }) {
    return showDialog<_SharedMemoryStoreEditResult>(
      context: context,
      builder: (context) =>
          _SharedMemoryStoreEditorDialog(title: title, store: store),
    );
  }

  @override
  State<_SharedMemoryStoreEditorDialog> createState() =>
      _SharedMemoryStoreEditorDialogState();
}

class _SharedMemoryStoreEditorDialogState
    extends State<_SharedMemoryStoreEditorDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameController;

  @override
  void initState() {
    super.initState();
    final store = widget.store;
    _nameController = TextEditingController(text: store?.name ?? '');
  }

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  void _save() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    Navigator.of(
      context,
    ).pop(_SharedMemoryStoreEditResult(name: _nameController.text.trim()));
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OperitDialogScaffold(
      title: widget.title,
      maxWidth: 520,
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
      child: Form(
        key: _formKey,
        child: TextFormField(
          controller: _nameController,
          autofocus: true,
          decoration: const InputDecoration(labelText: '名称'),
          validator: (value) {
            if (value == null || value.trim().isEmpty) {
              return l10n.required;
            }
            return null;
          },
          onFieldSubmitted: (_) => _save(),
        ),
      ),
    );
  }
}

class _UserMarkdownEditorDialog extends StatefulWidget {
  const _UserMarkdownEditorDialog({
    required this.title,
    required this.initialText,
  });

  final String title;
  final String initialText;

  static Future<String?> show({
    required BuildContext context,
    required String title,
    required String initialText,
  }) {
    return showDialog<String>(
      context: context,
      builder: (context) =>
          _UserMarkdownEditorDialog(title: title, initialText: initialText),
    );
  }

  @override
  State<_UserMarkdownEditorDialog> createState() =>
      _UserMarkdownEditorDialogState();
}

class _UserMarkdownEditorDialogState extends State<_UserMarkdownEditorDialog> {
  late final TextEditingController _controller;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.initialText);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _save() {
    Navigator.of(context).pop(_controller.text);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final textTheme = Theme.of(context).textTheme;
    return OperitDialogScaffold(
      title: widget.title,
      maxWidth: 760,
      maxHeight: 620,
      showCloseButton: true,
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
      child: TextField(
        controller: _controller,
        autofocus: true,
        expands: true,
        minLines: null,
        maxLines: null,
        textAlignVertical: TextAlignVertical.top,
        style: textTheme.bodyMedium?.copyWith(
          fontFamily: 'monospace',
          height: 1.35,
        ),
        decoration: InputDecoration(
          labelText: l10n.settingsCharactersUserMarkdownContent,
          alignLabelWithHint: true,
        ),
      ),
    );
  }
}
