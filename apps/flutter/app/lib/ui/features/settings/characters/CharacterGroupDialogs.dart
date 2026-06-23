part of 'CharacterSettingsPanel.dart';

sealed class _CharacterGroupEditorResult {
  const _CharacterGroupEditorResult();
}

class _CharacterGroupEditorSave extends _CharacterGroupEditorResult {
  const _CharacterGroupEditorSave(this.group);

  final core_proxy.CharacterGroupCard group;
}

class _CharacterGroupEditorCopyJson extends _CharacterGroupEditorResult {
  const _CharacterGroupEditorCopyJson();
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
                  label: l10n.settingsCharactersGroupName,
                  requiredField: true,
                ),
                _DialogTextField(
                  controller: _descriptionController,
                  label: l10n.settingsCharactersDescription,
                ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Padding(
                    padding: const EdgeInsets.only(top: 8, bottom: 4),
                    child: Text(
                      l10n.settingsCharactersGroupMembersTitle,
                      style: const TextStyle(fontWeight: FontWeight.w800),
                    ),
                  ),
                ),
                for (final card in widget.cards)
                  CheckboxListTile(
                    contentPadding: EdgeInsets.zero,
                    dense: true,
                    visualDensity: VisualDensity.compact,
                    title: Text(card.name),
                    subtitle: card.description.trim().isEmpty
                        ? null
                        : Text(card.description.trim()),
                    value: _selectedCardIds.contains(card.id),
                    onChanged: (value) {
                      setState(() {
                        if (value == true) {
                          _selectedCardIds.add(card.id);
                        } else {
                          _selectedCardIds.remove(card.id);
                        }
                      });
                    },
                  ),
              ],
            ),
          ),
        ),
      ),
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
            ).pop(const _CharacterGroupEditorCopyJson()),
            child: Text(l10n.settingsCharactersCopyJson),
          ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
      ],
    );
  }
}

enum _CharacterCardImportAction { nativeJson, tavernJson }

enum _CharacterCardExportAction { nativeJson, tavernJson }

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
    return AlertDialog(
      title: Text(l10n.settingsCharactersExport),
      content: SizedBox(
        width: 360,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            ListTile(
              leading: const Icon(Icons.data_object_outlined),
              title: Text(l10n.settingsCharactersCopyJson),
              onTap: () => Navigator.of(
                context,
              ).pop(_CharacterCardExportAction.nativeJson),
            ),
            ListTile(
              leading: const Icon(Icons.badge_outlined),
              title: Text(l10n.settingsCharactersCopyTavernJson),
              onTap: () => Navigator.of(
                context,
              ).pop(_CharacterCardExportAction.tavernJson),
            ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
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
    return AlertDialog(
      title: Text(l10n.settingsCharactersImport),
      content: SizedBox(
        width: 360,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            ListTile(
              leading: const Icon(Icons.data_object_outlined),
              title: Text(l10n.settingsCharactersImportJson),
              onTap: () => Navigator.of(
                context,
              ).pop(_CharacterCardImportAction.nativeJson),
            ),
            ListTile(
              leading: const Icon(Icons.badge_outlined),
              title: Text(l10n.settingsCharactersImportTavernJson),
              onTap: () => Navigator.of(
                context,
              ).pop(_CharacterCardImportAction.tavernJson),
            ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
    );
  }
}

class _JsonImportDialog extends StatefulWidget {
  const _JsonImportDialog({required this.title, required this.label});

  final String title;
  final String label;

  static Future<String?> show({
    required BuildContext context,
    required String title,
    required String label,
  }) {
    return showDialog<String>(
      context: context,
      builder: (context) => _JsonImportDialog(title: title, label: label),
    );
  }

  @override
  State<_JsonImportDialog> createState() => _JsonImportDialogState();
}

class _JsonImportDialogState extends State<_JsonImportDialog> {
  final _formKey = GlobalKey<FormState>();
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    Navigator.of(context).pop(_controller.text.trim());
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 640,
        child: Form(
          key: _formKey,
          child: TextFormField(
            controller: _controller,
            autofocus: true,
            minLines: 12,
            maxLines: 18,
            decoration: InputDecoration(labelText: widget.label),
            validator: (value) {
              final text = value?.trim() ?? '';
              if (text.isEmpty) {
                return widget.label;
              }
              return null;
            },
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _submit,
          child: Text(l10n.settingsCharactersImportJson),
        ),
      ],
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
          padding: const EdgeInsets.all(20),
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
        ),
      ),
    );
  }
}

class _DialogDropdown<T> extends StatelessWidget {
  const _DialogDropdown({
    required this.label,
    required this.value,
    required this.items,
    required this.onChanged,
  });

  final String label;
  final T? value;
  final List<DropdownMenuItem<T>> items;
  final ValueChanged<T?> onChanged;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: DropdownButtonFormField<T>(
        initialValue: value,
        items: items,
        onChanged: items.isEmpty ? null : onChanged,
        style: OperitFormStyles.dropdownTextStyle(context),
        decoration: InputDecoration(labelText: label),
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
    this.action,
  });

  final String title;
  final List<Widget> children;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              LayoutBuilder(
                builder: (context, constraints) {
                  final titleText = Text(
                    title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: SettingsControlStyles.sectionTitleTextStyle(context),
                  );
                  if (action == null) {
                    return titleText;
                  }
                  if (constraints.maxWidth < 420) {
                    return Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        titleText,
                        const SizedBox(height: 6),
                        Align(alignment: Alignment.centerRight, child: action!),
                      ],
                    );
                  }
                  return Row(
                    crossAxisAlignment: CrossAxisAlignment.center,
                    children: <Widget>[
                      Expanded(child: titleText),
                      const SizedBox(width: 12),
                      Flexible(
                        flex: 0,
                        child: Align(
                          alignment: Alignment.centerRight,
                          child: action!,
                        ),
                      ),
                    ],
                  );
                },
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

class _ExpandableSectionCard extends StatelessWidget {
  const _ExpandableSectionCard({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Theme(
          data: Theme.of(context).copyWith(
            dividerColor: Colors.transparent,
            dividerTheme: const DividerThemeData(color: Colors.transparent),
          ),
          child: ExpansionTile(
            initiallyExpanded: false,
            tilePadding: const EdgeInsets.fromLTRB(14, 2, 14, 2),
            childrenPadding: const EdgeInsets.fromLTRB(14, 0, 14, 10),
            shape: const Border(),
            collapsedShape: const Border(),
            title: Text(
              title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: SettingsControlStyles.sectionTitleTextStyle(context),
            ),
            children: <Widget>[
              Align(
                alignment: Alignment.centerLeft,
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
const Set<String> _hiddenToolNames = <String>{
  'package_proxy',
  'proxy',
  'search',
};

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

class _ToolAccessOption {
  const _ToolAccessOption({
    required this.key,
    required this.title,
    this.subtitle = '',
  });

  final String key;
  final String title;
  final String subtitle;
}

int _compareToolAccessOption(_ToolAccessOption left, _ToolAccessOption right) {
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

String _mcpServerSubtitle(core_proxy.ServerConfig config) {
  final parts = <String>[
    if ((config.type ?? '').trim().isNotEmpty) config.type!.trim(),
    if ((config.url ?? '').trim().isNotEmpty) config.url!.trim(),
    if (config.command.trim().isNotEmpty) config.command.trim(),
  ];
  return parts.join(' · ');
}

void _setSelection(Set<String> values, String key, bool selected) {
  if (selected) {
    values.add(key);
  } else {
    values.remove(key);
  }
}

Map<String, Object?> _jsonObjectFromText(String text) {
  final decoded = jsonDecode(text);
  final converted = _convertJsonNode(decoded);
  if (converted is! Map<String, Object?>) {
    throw const FormatException('JSON root must be an object');
  }
  return converted;
}

String _requestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';

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
