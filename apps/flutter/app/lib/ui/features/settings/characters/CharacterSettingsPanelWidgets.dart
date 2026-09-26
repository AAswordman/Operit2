// ignore_for_file: file_names

part of 'CharacterSettingsPanel.dart';

class CharacterSettingsData {
  /// Creates an immutable character settings snapshot.
  const CharacterSettingsData({
    required this.cards,
    required this.groups,
    required this.sharedMemoryStores,
    required this.tags,
    required this.modelSummaries,
    required this.ttsConfigs,
    required this.builtinToolOptions,
    required this.packageToolOptions,
    required this.skillToolOptions,
    required this.mcpToolOptions,
    required this.activeCardId,
    required this.activeGroupId,
  });

  final List<core_proxy.CharacterCard> cards;
  final List<core_proxy.CharacterGroupCard> groups;
  final List<core_proxy.SharedMemoryStore> sharedMemoryStores;
  final List<core_proxy.PromptTag> tags;
  final List<core_proxy.ProviderModelSummary> modelSummaries;
  final List<core_proxy.TtsConfig> ttsConfigs;
  final List<ToolAccessOption> builtinToolOptions;
  final List<ToolAccessOption> packageToolOptions;
  final List<ToolAccessOption> skillToolOptions;
  final List<ToolAccessOption> mcpToolOptions;
  final String? activeCardId;
  final String? activeGroupId;
}

class _ActivePromptSelection {
  const _ActivePromptSelection({this.cardId, this.groupId});

  final String? cardId;
  final String? groupId;
}

_ActivePromptSelection _activePromptSelection(
  core_proxy.ActivePrompt? activePrompt,
) {
  String? cardId;
  String? groupId;
  if (activePrompt != null) {
    if (activePrompt.tag == 'CharacterCard' &&
        activePrompt.id.trim().isNotEmpty) {
      cardId = activePrompt.id.trim();
    } else if (activePrompt.tag == 'CharacterGroup' &&
        activePrompt.id.trim().isNotEmpty) {
      groupId = activePrompt.id.trim();
    }
  }
  return _ActivePromptSelection(cardId: cardId, groupId: groupId);
}

class _CharacterCardTile extends StatelessWidget {
  const _CharacterCardTile({
    required this.card,
    required this.tags,
    required this.avatarUri,
    required this.active,
    required this.onActivate,
    required this.onEdit,
    required this.onEditUserMarkdown,
    required this.onOpenMemoryGraph,
  });

  final core_proxy.CharacterCard card;
  final List<core_proxy.PromptTag> tags;
  final String? avatarUri;
  final bool active;
  final VoidCallback onActivate;
  final VoidCallback onEdit;
  final VoidCallback onEditUserMarkdown;
  final VoidCallback onOpenMemoryGraph;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final tagNames = _tagNamesFor(tags, card.attachedTagIds);
    final descriptionText = card.description.trim();
    final modelBadgeText =
        card.chatModelBindingMode.trim().toUpperCase() == _chatModelFixedConfig
        ? l10n.settingsCharactersChatModelFixedConfig
        : l10n.settingsCharactersChatModelFollowGlobal;
    return _SettingsEntityTile(
      active: active,
      leading: _SettingsListAvatar(
        avatar: CharacterAvatarImage(avatarUri: avatarUri, fit: BoxFit.cover),
        active: active,
      ),
      title: Text(card.name),
      subtitle: descriptionText.isEmpty ? null : Text(descriptionText),
      badges: <Widget>[
        SettingsInfoBadge(label: modelBadgeText),
        SettingsInfoBadge(label: _memoryBindingSummary(card)),
        for (final tagName in tagNames) SettingsInfoBadge(label: tagName),
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
        active
            ? SettingsActivePill(label: l10n.settingsActive)
            : SettingsSetActiveButton(
                label: l10n.settingsActivate,
                onPressed: onActivate,
              ),
      ],
    );
  }
}

class _CharacterGroupTile extends StatelessWidget {
  const _CharacterGroupTile({
    required this.group,
    required this.active,
    required this.cards,
    required this.onActivate,
    required this.onEdit,
  });

  final core_proxy.CharacterGroupCard group;
  final bool active;
  final List<core_proxy.CharacterCard> cards;
  final VoidCallback onActivate;
  final VoidCallback onEdit;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final memberNames = group.members
        .map((member) => _cardNameFor(cards, member.characterCardId))
        .nonNulls
        .join(', ');
    return _SettingsEntityTile(
      active: active,
      leading: _SettingsListAvatar(
        avatar: _CharacterGroupCompositeAvatar(group: group, cards: cards),
        active: active,
      ),
      title: Text(group.name),
      subtitle: memberNames.isEmpty ? null : Text(memberNames),
      badges: <Widget>[
        SettingsInfoBadge(
          label: l10n.settingsCharactersGroupMembers(group.members.length),
        ),
      ],
      onTap: onEdit,
      actions: <Widget>[
        active
            ? SettingsActivePill(label: l10n.settingsActive)
            : SettingsSetActiveButton(
                label: l10n.settingsActivate,
                onPressed: onActivate,
              ),
      ],
    );
  }
}

class _CharacterGroupCompositeAvatar extends StatelessWidget {
  const _CharacterGroupCompositeAvatar({
    required this.group,
    required this.cards,
  });

  final core_proxy.CharacterGroupCard group;
  final List<core_proxy.CharacterCard> cards;

  /// Builds the member-avatar grid used as the group's generated avatar.
  @override
  Widget build(BuildContext context) {
    final cardsById = <String, core_proxy.CharacterCard>{
      for (final card in cards) card.id: card,
    };
    final members = group.members.toList()
      ..sort((left, right) => left.orderIndex.compareTo(right.orderIndex));
    final tiles = members
        .take(9)
        .map((member) {
          final card = cardsById[member.characterCardId];
          return _CharacterGroupAvatarTileData(
            label: card?.name ?? group.name,
            avatarUri: card?.avatarUri,
          );
        })
        .toList(growable: false);
    final visibleTiles = tiles.isEmpty
        ? <_CharacterGroupAvatarTileData>[
            _CharacterGroupAvatarTileData(label: group.name, avatarUri: null),
          ]
        : tiles;
    final columns = visibleTiles.length == 1
        ? 1
        : visibleTiles.length <= 4
        ? 2
        : 3;

    return GridView.count(
      crossAxisCount: columns,
      crossAxisSpacing: 1,
      mainAxisSpacing: 1,
      padding: EdgeInsets.zero,
      physics: const NeverScrollableScrollPhysics(),
      children: <Widget>[
        for (final tile in visibleTiles) _CharacterGroupAvatarTile(tile: tile),
      ],
    );
  }
}

class _CharacterGroupAvatarTileData {
  const _CharacterGroupAvatarTileData({
    required this.label,
    required this.avatarUri,
  });

  final String label;
  final String? avatarUri;
}

class _CharacterGroupAvatarTile extends StatelessWidget {
  const _CharacterGroupAvatarTile({required this.tile});

  final _CharacterGroupAvatarTileData tile;

  /// Builds one member cell in a generated group avatar.
  @override
  Widget build(BuildContext context) {
    final avatarUri = tile.avatarUri?.trim();
    if (avatarUri != null && avatarUri.isNotEmpty) {
      return CharacterAvatarImage(avatarUri: avatarUri, fit: BoxFit.cover);
    }
    final label = tile.label.trim();
    final hue = label.hashCode.abs() % 360;
    final color = HSVColor.fromAHSV(1, hue.toDouble(), 0.35, 0.82).toColor();
    return ColoredBox(
      color: color,
      child: Center(
        child: Text(
          label.isEmpty ? '?' : label.substring(0, 1).toUpperCase(),
          style: const TextStyle(
            color: Colors.white,
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
    );
  }
}

class _SettingsListAvatar extends StatelessWidget {
  const _SettingsListAvatar({required this.avatar, required this.active});

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

class _SettingsEntityTile extends StatelessWidget {
  const _SettingsEntityTile({
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

class _PromptTagEditResult {
  const _PromptTagEditResult({
    required this.name,
    required this.description,
    required this.promptContent,
  });

  final String name;
  final String description;
  final String promptContent;
}

class _PromptTagCreateDraft {
  const _PromptTagCreateDraft({required this.draftId, required this.values});

  final String draftId;
  final _PromptTagEditResult values;
}

class _PromptTagUpdateDraft {
  const _PromptTagUpdateDraft({
    required this.tagId,
    required this.values,
    required this.tagType,
  });

  final String tagId;
  final _PromptTagEditResult values;
  final core_proxy.TagType tagType;
}

class _PromptTagChangeSet {
  const _PromptTagChangeSet({
    required this.created,
    required this.updated,
    required this.deletedTagIds,
  });

  final List<_PromptTagCreateDraft> created;
  final List<_PromptTagUpdateDraft> updated;
  final List<String> deletedTagIds;
}

class _PromptTagEditorDialog extends StatefulWidget {
  const _PromptTagEditorDialog({required this.title, this.tag});

  final String title;
  final core_proxy.PromptTag? tag;

  static Future<_PromptTagEditResult?> show({
    required BuildContext context,
    required String title,
    core_proxy.PromptTag? tag,
  }) {
    return showDialog<_PromptTagEditResult>(
      context: context,
      builder: (context) => _PromptTagEditorDialog(title: title, tag: tag),
    );
  }

  @override
  State<_PromptTagEditorDialog> createState() => _PromptTagEditorDialogState();
}

class _PromptTagEditorDialogState extends State<_PromptTagEditorDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameController;
  late final TextEditingController _descriptionController;
  late final TextEditingController _promptContentController;

  @override
  void initState() {
    super.initState();
    final tag = widget.tag;
    _nameController = TextEditingController(text: tag?.name ?? '');
    _descriptionController = TextEditingController(
      text: tag?.description ?? '',
    );
    _promptContentController = TextEditingController(
      text: tag?.promptContent ?? '',
    );
  }

  @override
  void dispose() {
    _nameController.dispose();
    _descriptionController.dispose();
    _promptContentController.dispose();
    super.dispose();
  }

  void _save() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    Navigator.of(context).pop(
      _PromptTagEditResult(
        name: _nameController.text.trim(),
        description: _descriptionController.text.trim(),
        promptContent: _promptContentController.text,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OperitDialogScaffold(
      title: widget.title,
      maxWidth: 580,
      showCloseButton: true,
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
      child: Form(
        key: _formKey,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              _DialogTextField(
                controller: _nameController,
                label: l10n.settingsCharactersTagName,
                requiredField: true,
              ),
              _DialogTextField(
                controller: _descriptionController,
                label: l10n.settingsCharactersTagDescription,
                maxLines: 2,
              ),
              _DialogTextField(
                controller: _promptContentController,
                label: l10n.settingsCharactersTagPromptContent,
                requiredField: true,
                maxLines: 8,
              ),
            ],
          ),
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
