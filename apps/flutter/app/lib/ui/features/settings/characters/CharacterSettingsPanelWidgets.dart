part of 'CharacterSettingsPanel.dart';

class _AdvancedSettingsGroup extends StatelessWidget {
  const _AdvancedSettingsGroup({
    required this.title,
    required this.description,
    required this.children,
    this.action,
  });

  final String title;
  final String description;
  final Widget? action;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final radius = BorderRadius.circular(14);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colorScheme.surfaceContainerHigh.withValues(alpha: 0.34),
          borderRadius: radius,
          border: Border.all(
            color: colorScheme.outlineVariant.withValues(alpha: 0.24),
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              LayoutBuilder(
                builder: (context, _) {
                  final titleColumn = Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Container(
                        width: 4,
                        height: 34,
                        margin: const EdgeInsets.only(top: 2, right: 10),
                        decoration: BoxDecoration(
                          color: colorScheme.primary.withValues(alpha: 0.76),
                          borderRadius: BorderRadius.circular(999),
                        ),
                      ),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            Text(
                              title,
                              style: textTheme.titleSmall?.copyWith(
                                fontWeight: FontWeight.w800,
                              ),
                            ),
                            const SizedBox(height: 2),
                            Text(
                              description,
                              style: textTheme.bodySmall?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  );
                  if (action == null) {
                    return titleColumn;
                  }
                  return Row(
                    crossAxisAlignment: CrossAxisAlignment.center,
                    children: <Widget>[
                      Expanded(child: titleColumn),
                      const SizedBox(width: 8),
                      Flexible(flex: 0, child: action!),
                    ],
                  );
                },
              ),
              const SizedBox(height: 10),
              Divider(
                height: 1,
                color: colorScheme.outlineVariant.withValues(alpha: 0.18),
              ),
              const SizedBox(height: 10),
              Padding(
                padding: const EdgeInsets.only(left: 14),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: children,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CharacterSettingsData {
  const _CharacterSettingsData({
    required this.cards,
    required this.groups,
    required this.cardAvatarUris,
    required this.groupAvatarUris,
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
    required this.enableMemoryAutoUpdate,
    required this.disableUserPreferenceDescription,
  });

  final List<core_proxy.CharacterCard> cards;
  final List<core_proxy.CharacterGroupCard> groups;
  final Map<String, String> cardAvatarUris;
  final Map<String, String> groupAvatarUris;
  final List<core_proxy.SharedMemoryStore> sharedMemoryStores;
  final List<core_proxy.PromptTag> tags;
  final List<core_proxy.ProviderModelSummary> modelSummaries;
  final List<core_proxy.TtsConfig> ttsConfigs;
  final List<_ToolAccessOption> builtinToolOptions;
  final List<_ToolAccessOption> packageToolOptions;
  final List<_ToolAccessOption> skillToolOptions;
  final List<_ToolAccessOption> mcpToolOptions;
  final String? activeCardId;
  final String? activeGroupId;
  final bool enableMemoryAutoUpdate;
  final bool disableUserPreferenceDescription;
}

class _ActivePromptSelection {
  const _ActivePromptSelection({this.cardId, this.groupId});

  final String? cardId;
  final String? groupId;
}

_ActivePromptSelection _activePromptSelection(Object? activePrompt) {
  String? cardId;
  String? groupId;
  if (activePrompt is Map) {
    final characterGroup = activePrompt['CharacterGroup'];
    if (characterGroup is Map) {
      final id = characterGroup['id'];
      if (id is String && id.trim().isNotEmpty) {
        groupId = id.trim();
      }
    }
    final characterCard = activePrompt['CharacterCard'];
    if (characterCard is Map) {
      final id = characterCard['id'];
      if (id is String && id.trim().isNotEmpty) {
        cardId = id.trim();
      }
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
    return _SettingsEntityTile(
      leading: _SettingsListAvatar(
        imagePath: avatarUri,
        placeholder: Image.asset(
          _SettingsListAvatar.operitAvatarAsset,
          fit: BoxFit.cover,
        ),
        active: active,
      ),
      title: Text(card.name),
      subtitle: Text(
        [
          if (card.description.trim().isNotEmpty) card.description.trim(),
          if (tagNames.isNotEmpty) tagNames.join(', '),
          card.chatModelBindingMode,
          _memoryBindingSummary(card),
        ].join(' · '),
      ),
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
    required this.avatarUri,
    required this.onActivate,
    required this.onEdit,
  });

  final core_proxy.CharacterGroupCard group;
  final bool active;
  final List<core_proxy.CharacterCard> cards;
  final String? avatarUri;
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
      leading: _SettingsListAvatar(
        imagePath: avatarUri,
        placeholder: const Center(child: Icon(Icons.groups_outlined, size: 18)),
        active: active,
      ),
      title: Text(group.name),
      subtitle: Text(
        [
          l10n.settingsCharactersGroupMembers(group.members.length),
          if (memberNames.isNotEmpty) memberNames,
        ].join(' · '),
      ),
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

class _SettingsListAvatar extends StatelessWidget {
  const _SettingsListAvatar({
    required this.imagePath,
    required this.placeholder,
    required this.active,
  });

  static const String operitAvatarAsset = 'assets/images/operit_avatar.png';

  final String? imagePath;
  final Widget placeholder;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final path = imagePath;
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
                child: path != null && path.isNotEmpty
                    ? Image.file(File(path), fit: BoxFit.cover)
                    : IconTheme(
                        data: IconThemeData(
                          color: colorScheme.onSurfaceVariant,
                        ),
                        child: placeholder,
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

class _SharedMemoryStoreTile extends StatelessWidget {
  const _SharedMemoryStoreTile({
    required this.store,
    required this.onEdit,
    required this.onDelete,
    required this.onEditUserMarkdown,
    required this.onOpenMemoryGraph,
  });

  final core_proxy.SharedMemoryStore store;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final VoidCallback onEditUserMarkdown;
  final VoidCallback onOpenMemoryGraph;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return _SettingsEntityTile(
      leading: const Icon(Icons.hub_outlined),
      title: Text(store.name),
      subtitle: Text(_sharedOwnerKey(store.id)),
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

class _SettingsEntityTile extends StatelessWidget {
  const _SettingsEntityTile({
    required this.leading,
    required this.title,
    required this.subtitle,
    required this.actions,
    this.onTap,
  });

  final Widget leading;
  final Widget title;
  final Widget subtitle;
  final List<Widget> actions;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          borderRadius: BorderRadius.circular(8),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
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
                    const SizedBox(width: 8),
                    Expanded(
                      child: DefaultTextStyle.merge(
                        style: TextStyle(color: colorScheme.onSurface),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            DefaultTextStyle.merge(
                              style: Theme.of(context).textTheme.titleSmall!
                                  .copyWith(fontWeight: FontWeight.w700),
                              child: title,
                            ),
                            const SizedBox(height: 2),
                            DefaultTextStyle.merge(
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                              style: Theme.of(context).textTheme.bodySmall!
                                  .copyWith(
                                    color: colorScheme.onSurfaceVariant,
                                    height: 1.25,
                                  ),
                              child: subtitle,
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                );
                final actionBar = Align(
                  alignment: Alignment.centerRight,
                  child: Wrap(
                    spacing: 2,
                    runSpacing: 2,
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
                      const SizedBox(height: 4),
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
                      constraints: const BoxConstraints(maxWidth: 180),
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
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 580,
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
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
    );
  }
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
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 620,
        child: Form(
          key: _formKey,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                _DialogTextField(
                  controller: _nameController,
                  label: '名称',
                  requiredField: true,
                ),
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
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
    return Dialog.fullscreen(
      child: Scaffold(
        appBar: AppBar(
          title: Text(widget.title),
          leading: IconButton(
            tooltip: l10n.cancel,
            onPressed: () => Navigator.of(context).pop(),
            icon: const Icon(Icons.close),
          ),
          actions: <Widget>[
            TextButton(onPressed: _save, child: Text(l10n.save)),
          ],
        ),
        body: Padding(
          padding: const EdgeInsets.all(16),
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
        ),
      ),
    );
  }
}

