// ignore_for_file: file_names

part of 'CharacterSettingsPanel.dart';

sealed class _CharacterGroupEditorResult {
  const _CharacterGroupEditorResult();
}

class _CharacterGroupEditorSave extends _CharacterGroupEditorResult {
  const _CharacterGroupEditorSave(this.group);

  final core_proxy.CharacterGroupCard group;
}

class _CharacterGroupEditorExportJson extends _CharacterGroupEditorResult {
  const _CharacterGroupEditorExportJson();
}

class _CharacterGroupEditorDelete extends _CharacterGroupEditorResult {
  const _CharacterGroupEditorDelete();
}

class _CharacterGroupEditorDialog extends StatefulWidget {
  const _CharacterGroupEditorDialog({
    required this.title,
    required this.group,
    required this.cards,
    required this.showItemActions,
  });

  final String title;
  final core_proxy.CharacterGroupCard group;
  final List<core_proxy.CharacterCard> cards;
  final bool showItemActions;

  static Future<_CharacterGroupEditorResult?> show({
    required BuildContext context,
    required String title,
    required core_proxy.CharacterGroupCard group,
    required List<core_proxy.CharacterCard> cards,
    required bool showItemActions,
  }) {
    return showDialog<_CharacterGroupEditorResult>(
      context: context,
      builder: (context) => _CharacterGroupEditorDialog(
        title: title,
        group: group,
        cards: cards,
        showItemActions: showItemActions,
      ),
    );
  }

  @override
  State<_CharacterGroupEditorDialog> createState() =>
      _CharacterGroupEditorDialogState();
}

class _CharacterGroupEditorDialogState
    extends State<_CharacterGroupEditorDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameController;
  late final TextEditingController _descriptionController;
  late Set<String> _selectedCardIds;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController(text: widget.group.name);
    _descriptionController = TextEditingController(
      text: widget.group.description,
    );
    _selectedCardIds = widget.group.members
        .map((member) => member.characterCardId)
        .toSet();
  }

  @override
  void dispose() {
    _nameController.dispose();
    _descriptionController.dispose();
    super.dispose();
  }

  void _save() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    final members = <core_proxy.GroupMemberConfig>[];
    for (final card in widget.cards) {
      if (_selectedCardIds.contains(card.id)) {
        members.add(
          core_proxy.GroupMemberConfig(
            characterCardId: card.id,
            orderIndex: members.length,
          ),
        );
      }
    }
    Navigator.of(context).pop(
      _CharacterGroupEditorSave(
        core_proxy.CharacterGroupCard(
          id: widget.group.id,
          name: _nameController.text.trim(),
          description: _descriptionController.text.trim(),
          members: members,
          createdAt: widget.group.createdAt,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OperitDialogScaffold(
      title: widget.title,
      maxWidth: 620,
      showCloseButton: true,
      actions: <Widget>[
        if (widget.showItemActions)
          TextButton(
            onPressed: () =>
                Navigator.of(context).pop(const _CharacterGroupEditorDelete()),
            child: Text(l10n.delete),
          ),
        if (widget.showItemActions)
          TextButton(
            onPressed: () => Navigator.of(
              context,
            ).pop(const _CharacterGroupEditorExportJson()),
            child: Text(l10n.settingsCharactersExportJson),
          ),
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
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              _DialogTextField(
                controller: _nameController,
                label: l10n.settingsCharactersGroupName,
                requiredField: true,
              ),
              _DialogTextField(
                controller: _descriptionController,
                label: l10n.settingsCharactersDescription,
              ),
              Padding(
                padding: const EdgeInsets.only(top: 8, bottom: 8),
                child: Text(
                  l10n.settingsCharactersGroupMembersTitle,
                  style: Theme.of(context).textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              for (final card in widget.cards)
                _CharacterGroupMemberOptionTile(
                  card: card,
                  selected: _selectedCardIds.contains(card.id),
                  onTap: () {
                    setState(() {
                      if (_selectedCardIds.contains(card.id)) {
                        _selectedCardIds.remove(card.id);
                      } else {
                        _selectedCardIds.add(card.id);
                      }
                    });
                  },
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CharacterGroupMemberOptionTile extends StatelessWidget {
  const _CharacterGroupMemberOptionTile({
    required this.card,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.CharacterCard card;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(10);
    final descriptionText = card.description.trim();
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Material(
        color: selected
            ? colorScheme.primaryContainer.withValues(alpha: 0.16)
            : colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
        shape: RoundedRectangleBorder(
          borderRadius: radius,
          side: BorderSide(
            color: selected
                ? colorScheme.primary.withValues(alpha: 0.45)
                : colorScheme.outlineVariant.withValues(alpha: 0.28),
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          borderRadius: radius,
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: Row(
              children: <Widget>[
                Icon(
                  selected ? Icons.check_circle : Icons.circle_outlined,
                  size: 18,
                  color: selected
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 10),
                SizedBox(
                  width: 28,
                  height: 28,
                  child: ClipOval(
                    child: CharacterAvatarImage(
                      avatarUri: card.avatarUri,
                      fit: BoxFit.cover,
                    ),
                  ),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        card.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      if (descriptionText.isNotEmpty)
                        Text(
                          descriptionText,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

enum _CharacterCardImportAction { nativeJson, tavernJson }

enum _CharacterCardExportAction { nativeJson, tavernJson }

class _DialogActionOptionTile extends StatelessWidget {
  const _DialogActionOptionTile({
    required this.icon,
    required this.title,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(10);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Material(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
        shape: RoundedRectangleBorder(
          borderRadius: radius,
          side: BorderSide(
            color: colorScheme.outlineVariant.withValues(alpha: 0.28),
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          borderRadius: radius,
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            child: Row(
              children: <Widget>[
                Icon(icon, size: 20, color: colorScheme.primary),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    title,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                Icon(
                  Icons.chevron_right,
                  size: 18,
                  color: colorScheme.onSurfaceVariant,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _CharacterCardExportDialog extends StatelessWidget {
  const _CharacterCardExportDialog();

  static Future<_CharacterCardExportAction?> show({
    required BuildContext context,
  }) {
    return showDialog<_CharacterCardExportAction>(
      context: context,
      builder: (context) => const _CharacterCardExportDialog(),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OperitDialogScaffold(
      title: l10n.settingsCharactersExport,
      maxWidth: 420,
      showCloseButton: true,
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          _DialogActionOptionTile(
            icon: Icons.data_object_outlined,
            title: l10n.settingsCharactersExportJson,
            onTap: () => Navigator.of(
              context,
            ).pop(_CharacterCardExportAction.nativeJson),
          ),
          _DialogActionOptionTile(
            icon: Icons.badge_outlined,
            title: l10n.settingsCharactersExportTavernJson,
            onTap: () => Navigator.of(
              context,
            ).pop(_CharacterCardExportAction.tavernJson),
          ),
        ],
      ),
    );
  }
}

class _CharacterCardImportDialog extends StatelessWidget {
  const _CharacterCardImportDialog();

  static Future<_CharacterCardImportAction?> show({
    required BuildContext context,
  }) {
    return showDialog<_CharacterCardImportAction>(
      context: context,
      builder: (context) => const _CharacterCardImportDialog(),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OperitDialogScaffold(
      title: l10n.settingsCharactersImport,
      maxWidth: 420,
      showCloseButton: true,
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          _DialogActionOptionTile(
            icon: Icons.data_object_outlined,
            title: l10n.settingsCharactersImportJson,
            onTap: () => Navigator.of(
              context,
            ).pop(_CharacterCardImportAction.nativeJson),
          ),
          _DialogActionOptionTile(
            icon: Icons.badge_outlined,
            title: l10n.settingsCharactersImportTavernJson,
            onTap: () => Navigator.of(
              context,
            ).pop(_CharacterCardImportAction.tavernJson),
          ),
        ],
      ),
    );
  }
}

class _DialogTextField extends StatelessWidget {
  const _DialogTextField({
    required this.controller,
    required this.label,
    this.requiredField = false,
    this.maxLines = 1,
  });

  final TextEditingController controller;
  final String label;
  final bool requiredField;
  final int maxLines;

  @override
  Widget build(BuildContext context) {
    final textStyle = Theme.of(context).textTheme.bodyMedium;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: TextFormField(
        controller: controller,
        style: textStyle,
        maxLines: maxLines,
        keyboardType: TextInputType.text,
        decoration: InputDecoration(labelText: label),
        validator: (value) {
          final text = value?.trim() ?? '';
          if (requiredField && text.isEmpty) {
            return label;
          }
          return null;
        },
      ),
    );
  }
}

class _DialogExpandableTextField extends StatelessWidget {
  const _DialogExpandableTextField({
    required this.controller,
    required this.label,
    required this.maxLines,
  });

  final TextEditingController controller;
  final String label;
  final int maxLines;

  Future<void> _openFullscreenEditor(BuildContext context) async {
    final text = await _FullscreenTextEditDialog.show(
      context: context,
      title: label,
      initialText: controller.text,
    );
    if (text == null) {
      return;
    }
    controller.text = text;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final textStyle = Theme.of(context).textTheme.bodyMedium;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: TextFormField(
        controller: controller,
        style: textStyle,
        maxLines: maxLines,
        decoration: InputDecoration(
          labelText: label,
          suffixIconConstraints: const BoxConstraints.tightFor(
            width: 36,
            height: 36,
          ),
          suffixIcon: IconButton(
            tooltip: l10n.fullscreenInput,
            iconSize: 18,
            visualDensity: VisualDensity.compact,
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints.tightFor(width: 28, height: 28),
            icon: const Icon(Icons.fullscreen),
            onPressed: () => _openFullscreenEditor(context),
          ),
        ),
      ),
    );
  }
}

class _FullscreenTextEditDialog extends StatefulWidget {
  const _FullscreenTextEditDialog({
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
          _FullscreenTextEditDialog(title: title, initialText: initialText),
    );
  }

  @override
  State<_FullscreenTextEditDialog> createState() =>
      _FullscreenTextEditDialogState();
}

class _FullscreenTextEditDialogState extends State<_FullscreenTextEditDialog> {
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
        style: Theme.of(context).textTheme.bodyMedium,
        autofocus: true,
        expands: true,
        minLines: null,
        maxLines: null,
        textAlignVertical: TextAlignVertical.top,
        decoration: InputDecoration(
          labelText: widget.title,
          alignLabelWithHint: true,
        ),
      ),
    );
  }
}

class _DialogToolAccessConfigureField extends StatelessWidget {
  const _DialogToolAccessConfigureField({
    required this.label,
    required this.valueText,
    required this.onConfigure,
  });

  final String label;
  final String valueText;
  final VoidCallback onConfigure;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onConfigure,
          child: InputDecorator(
            decoration: InputDecoration(labelText: label),
            child: Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    valueText,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                ),
                const SizedBox(width: 12),
                const Icon(Icons.tune_outlined),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({
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

String? _cardNameFor(List<core_proxy.CharacterCard> cards, String id) {
  for (final card in cards) {
    if (card.id == id) {
      return card.name;
    }
  }
  return null;
}

List<String> _tagNamesFor(List<core_proxy.PromptTag> tags, List<String> ids) {
  final names = <String>[];
  for (final id in ids) {
    for (final tag in tags) {
      if (tag.id == id) {
        names.add(tag.name);
      }
    }
  }
  return names;
}

String _tagTypeText(core_proxy.TagType tagType) {
  return tagType.value;
}

const String _chatModelFollowGlobal = 'FOLLOW_GLOBAL';
const String _chatModelFixedConfig = 'FIXED_CONFIG';
const String _memoryBindingCharacter = 'CHARACTER';
const String _memoryBindingShared = 'SHARED';
String _characterOwnerKey(String characterCardId) {
  return 'character:${characterCardId.trim()}';
}

String _sharedOwnerKey(String sharedMemoryId) {
  return 'shared:${sharedMemoryId.trim()}';
}

core_proxy.TtsConfig? _ttsConfigById(
  List<core_proxy.TtsConfig> configs,
  String? id,
) {
  if (id == null) {
    return null;
  }
  for (final config in configs) {
    if (config.id == id) {
      return config;
    }
  }
  return null;
}

String _ttsConfigBindingText(core_proxy.TtsConfig? config, String? id) {
  if (config != null) {
    return _ttsConfigDisplayText(config);
  }
  if (id != null) {
    return 'TTS 配置不存在：$id';
  }
  return '请选择 TTS 配置';
}

String _ttsConfigDisplayText(core_proxy.TtsConfig config) {
  return '${_ttsConfigProviderText(config)} · ${_ttsConfigModelVoiceText(config)}';
}

String _ttsConfigProviderText(core_proxy.TtsConfig config) {
  final endpoint = config.endpoint.trim();
  if (endpoint.isEmpty) {
    return config.providerType;
  }
  return '${config.providerType} · $endpoint';
}

String _ttsConfigModelVoiceText(core_proxy.TtsConfig config) {
  final model = config.model.trim();
  final voice = config.voice.trim();
  if (model.isEmpty && voice.isEmpty) {
    return '系统默认音色';
  }
  if (model.isEmpty) {
    return voice;
  }
  if (voice.isEmpty) {
    return model;
  }
  return '$model · $voice';
}

String _ttsConfigSearchText(core_proxy.TtsConfig config) {
  return '${config.name} ${config.providerType} ${config.endpoint} '
          '${config.model} ${config.voice}'
      .toLowerCase();
}

class ToolAccessOption {
  /// Creates one selectable tool access option.
  const ToolAccessOption({
    required this.key,
    required this.title,
    this.subtitle = '',
  });

  final String key;
  final String title;
  final String subtitle;
}

int _compareToolAccessOption(ToolAccessOption left, ToolAccessOption right) {
  return left.title.toLowerCase().compareTo(right.title.toLowerCase());
}

String _normalizeChatModelBindingMode(String mode) {
  return mode == _chatModelFixedConfig
      ? _chatModelFixedConfig
      : _chatModelFollowGlobal;
}

String _normalizeMemoryBindingMode(String mode) {
  return mode == _memoryBindingShared
      ? _memoryBindingShared
      : _memoryBindingCharacter;
}

core_proxy.ProviderModelSummary? _providerModelSummaryById(
  List<core_proxy.ProviderModelSummary> summaries,
  String? id,
) {
  for (final summary in summaries) {
    if (summary.modelId == id) {
      return summary;
    }
  }
  return null;
}

String _characterModelBindingText(
  core_proxy.ProviderModelSummary? summary,
  String? modelId,
) {
  if (summary != null) {
    return '${summary.providerName} · ${summary.modelId}';
  }
  if (modelId != null) {
    return '模型不存在：$modelId';
  }
  return '请选择模型配置';
}

String _memoryBindingSummary(core_proxy.CharacterCard card) {
  if (card.memoryBindingMode == _memoryBindingShared) {
    return '共享记忆';
  }
  return '角色记忆';
}

core_proxy.CharacterCardToolAccessConfig _normalizedToolAccessConfig(
  core_proxy.CharacterCardToolAccessConfig config,
) {
  return core_proxy.CharacterCardToolAccessConfig(
    enabled: config.enabled,
    allowedBuiltinTools: _normalizedEntries(config.allowedBuiltinTools),
    allowedPackages: _normalizedEntries(config.allowedPackages),
    allowedSkills: _normalizedEntries(config.allowedSkills),
    allowedMcpServers: _normalizedEntries(config.allowedMcpServers),
  );
}

List<String> _normalizedEntries(List<String> values) {
  final seen = <String>{};
  final entries = <String>[];
  for (final value in values) {
    final entry = value.trim();
    if (entry.isNotEmpty && seen.add(entry)) {
      entries.add(entry);
    }
  }
  return entries;
}

bool _toolAccessHasExternalSelections(
  core_proxy.CharacterCardToolAccessConfig config,
) {
  return config.allowedPackages.isNotEmpty ||
      config.allowedSkills.isNotEmpty ||
      config.allowedMcpServers.isNotEmpty;
}

String _toolAccessSummary(
  AppLocalizations l10n,
  core_proxy.CharacterCardToolAccessConfig config,
) {
  final normalized = _normalizedToolAccessConfig(config);
  if (!normalized.enabled) {
    return l10n.settingsCharactersToolAccessFollowGlobal;
  }
  if (normalized.allowedBuiltinTools.isEmpty &&
      normalized.allowedPackages.isEmpty &&
      normalized.allowedSkills.isEmpty &&
      normalized.allowedMcpServers.isEmpty) {
    return l10n.settingsCharactersToolAccessEmpty;
  }
  return l10n.settingsCharactersToolAccessSummaryCounts(
    normalized.allowedBuiltinTools.length,
    normalized.allowedPackages.length,
    normalized.allowedSkills.length,
    normalized.allowedMcpServers.length,
  );
}

Map<String, Object?> _jsonObjectFromText(String text) {
  final decoded = jsonDecode(text);
  final converted = _convertJsonNode(decoded);
  if (converted is! Map<String, Object?>) {
    throw const FormatException('JSON root must be an object');
  }
  return converted;
}

Object? _convertJsonNode(Object? value) {
  if (value is Map) {
    return <String, Object?>{
      for (final entry in value.entries)
        entry.key.toString(): _convertJsonNode(entry.value),
    };
  }
  if (value is List) {
    return <Object?>[for (final item in value) _convertJsonNode(item)];
  }
  return value;
}

core_proxy.CharacterCard _characterCardWith(
  core_proxy.CharacterCard card, {
  String? id,
  List<String>? attachedTagIds,
  bool? isDefault,
  int? createdAt,
  int? updatedAt,
}) {
  return core_proxy.CharacterCard(
    id: id ?? card.id,
    name: card.name,
    description: card.description,
    characterSetting: card.characterSetting,
    openingStatement: card.openingStatement,
    otherContentChat: card.otherContentChat,
    otherContentVoice: card.otherContentVoice,
    avatarUri: card.avatarUri,
    attachedTagIds: attachedTagIds ?? card.attachedTagIds,
    advancedCustomPrompt: card.advancedCustomPrompt,
    marks: card.marks,
    chatModelBindingMode: card.chatModelBindingMode,
    chatModelId: card.chatModelId,
    ttsConfigId: card.ttsConfigId,
    memoryBindingMode: card.memoryBindingMode,
    sharedMemoryId: card.sharedMemoryId,
    sharedMemoryMounts: const <core_proxy.CharacterSharedMemoryMount>[],
    toolAccessConfig: card.toolAccessConfig,
    isDefault: isDefault ?? card.isDefault,
    createdAt: createdAt ?? card.createdAt,
    updatedAt: updatedAt ?? card.updatedAt,
  );
}
