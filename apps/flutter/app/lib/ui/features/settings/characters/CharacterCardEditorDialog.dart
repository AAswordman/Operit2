part of 'CharacterSettingsPanel.dart';

sealed class _CharacterCardEditorResult {
  const _CharacterCardEditorResult();
}

class _CharacterCardEditorSave extends _CharacterCardEditorResult {
  const _CharacterCardEditorSave({
    required this.card,
    required this.tagChanges,
  });

  final core_proxy.CharacterCard card;
  final _PromptTagChangeSet tagChanges;
}

class _CharacterCardEditorExportJson extends _CharacterCardEditorResult {
  const _CharacterCardEditorExportJson();
}

class _CharacterCardEditorExportTavernJson extends _CharacterCardEditorResult {
  const _CharacterCardEditorExportTavernJson();
}

class _CharacterCardEditorDelete extends _CharacterCardEditorResult {
  const _CharacterCardEditorDelete();
}

class _CharacterCardEditorDialog extends StatefulWidget {
  const _CharacterCardEditorDialog({
    required this.title,
    required this.card,
    required this.showItemActions,
    required this.modelSummaries,
    required this.sharedMemoryStores,
    required this.ttsConfigs,
    required this.enableMemoryAutoUpdate,
    required this.disableUserPreferenceDescription,
    required this.onSaveMemoryAutoUpdate,
    required this.onSavePreferenceDescription,
    required this.builtinToolOptions,
    required this.packageToolOptions,
    required this.skillToolOptions,
    required this.mcpToolOptions,
    required this.tags,
  });

  final String title;
  final core_proxy.CharacterCard card;
  final bool showItemActions;
  final List<core_proxy.ProviderModelSummary> modelSummaries;
  final List<core_proxy.SharedMemoryStore> sharedMemoryStores;
  final List<core_proxy.TtsConfig> ttsConfigs;
  final bool enableMemoryAutoUpdate;
  final bool disableUserPreferenceDescription;
  final Future<void> Function(bool enabled) onSaveMemoryAutoUpdate;
  final Future<void> Function(bool enabled) onSavePreferenceDescription;
  final List<ToolAccessOption> builtinToolOptions;
  final List<ToolAccessOption> packageToolOptions;
  final List<ToolAccessOption> skillToolOptions;
  final List<ToolAccessOption> mcpToolOptions;
  final List<core_proxy.PromptTag> tags;

  static Future<_CharacterCardEditorResult?> show({
    required BuildContext context,
    required String title,
    required core_proxy.CharacterCard card,
    required bool showItemActions,
    required List<core_proxy.ProviderModelSummary> modelSummaries,
    required List<core_proxy.SharedMemoryStore> sharedMemoryStores,
    required List<core_proxy.TtsConfig> ttsConfigs,
    required bool enableMemoryAutoUpdate,
    required bool disableUserPreferenceDescription,
    required Future<void> Function(bool enabled) onSaveMemoryAutoUpdate,
    required Future<void> Function(bool enabled) onSavePreferenceDescription,
    required List<ToolAccessOption> builtinToolOptions,
    required List<ToolAccessOption> packageToolOptions,
    required List<ToolAccessOption> skillToolOptions,
    required List<ToolAccessOption> mcpToolOptions,
    required List<core_proxy.PromptTag> tags,
  }) {
    return showDialog<_CharacterCardEditorResult>(
      context: context,
      builder: (context) => _CharacterCardEditorDialog(
        title: title,
        card: card,
        showItemActions: showItemActions,
        modelSummaries: modelSummaries,
        sharedMemoryStores: sharedMemoryStores,
        ttsConfigs: ttsConfigs,
        enableMemoryAutoUpdate: enableMemoryAutoUpdate,
        disableUserPreferenceDescription: disableUserPreferenceDescription,
        onSaveMemoryAutoUpdate: onSaveMemoryAutoUpdate,
        onSavePreferenceDescription: onSavePreferenceDescription,
        builtinToolOptions: builtinToolOptions,
        packageToolOptions: packageToolOptions,
        skillToolOptions: skillToolOptions,
        mcpToolOptions: mcpToolOptions,
        tags: tags,
      ),
    );
  }

  @override
  State<_CharacterCardEditorDialog> createState() =>
      _CharacterCardEditorDialogState();
}

class _CharacterCardEditorDialogState
    extends State<_CharacterCardEditorDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameController;
  late final TextEditingController _descriptionController;
  late final TextEditingController _characterSettingController;
  late final TextEditingController _openingStatementController;
  late final TextEditingController _otherContentChatController;
  late final TextEditingController _otherContentVoiceController;
  late final TextEditingController _advancedPromptController;
  late final TextEditingController _marksController;
  String? _avatarUri;
  late String _chatModelBindingMode;
  String? _chatModelId;
  late bool _ttsBindingEnabled;
  String? _ttsConfigId;
  late String _memoryBindingMode;
  String? _sharedMemoryId;
  late bool _enableMemoryAutoUpdate;
  late bool _disableUserPreferenceDescription;
  late List<String> _attachedTagIds;
  late List<core_proxy.PromptTag> _tags;
  final List<_PromptTagCreateDraft> _createdTagDrafts =
      <_PromptTagCreateDraft>[];
  final Map<String, _PromptTagUpdateDraft> _updatedTagDrafts =
      <String, _PromptTagUpdateDraft>{};
  final Set<String> _deletedTagIds = <String>{};
  int _nextDraftTagIndex = 0;
  late core_proxy.CharacterCardToolAccessConfig _toolAccessConfig;

  @override
  void initState() {
    super.initState();
    final card = widget.card;
    _nameController = TextEditingController(text: card.name);
    _descriptionController = TextEditingController(text: card.description);
    _characterSettingController = TextEditingController(
      text: card.characterSetting,
    );
    _openingStatementController = TextEditingController(
      text: card.openingStatement,
    );
    _otherContentChatController = TextEditingController(
      text: card.otherContentChat,
    );
    _otherContentVoiceController = TextEditingController(
      text: card.otherContentVoice,
    );
    _advancedPromptController = TextEditingController(
      text: card.advancedCustomPrompt,
    );
    _marksController = TextEditingController(text: card.marks);
    _avatarUri = card.avatarUri;
    _chatModelBindingMode = _normalizeChatModelBindingMode(
      card.chatModelBindingMode,
    );
    _chatModelId = card.chatModelId;
    _ttsBindingEnabled = card.ttsConfigId != null;
    _ttsConfigId = card.ttsConfigId;
    _memoryBindingMode = _normalizeMemoryBindingMode(card.memoryBindingMode);
    _sharedMemoryId = card.sharedMemoryId;
    _enableMemoryAutoUpdate = widget.enableMemoryAutoUpdate;
    _disableUserPreferenceDescription = widget.disableUserPreferenceDescription;
    _attachedTagIds = List<String>.from(card.attachedTagIds);
    _tags = List<core_proxy.PromptTag>.from(widget.tags);
    _toolAccessConfig = _normalizedToolAccessConfig(card.toolAccessConfig);
  }

  @override
  void dispose() {
    _nameController.dispose();
    _descriptionController.dispose();
    _characterSettingController.dispose();
    _openingStatementController.dispose();
    _otherContentChatController.dispose();
    _otherContentVoiceController.dispose();
    _advancedPromptController.dispose();
    _marksController.dispose();
    super.dispose();
  }

  void _save() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    final l10n = AppLocalizations.of(context)!;
    final normalizedToolAccessConfig = _normalizedToolAccessConfig(
      _toolAccessConfig,
    );
    if (normalizedToolAccessConfig.enabled &&
        _toolAccessHasExternalSelections(normalizedToolAccessConfig) &&
        !normalizedToolAccessConfig.allowedBuiltinTools.contains(
          'use_package',
        )) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.settingsCharactersToolAccessRequiresUsePackage),
        ),
      );
      return;
    }
    if (_memoryBindingMode == _memoryBindingShared &&
        (_sharedMemoryId == null || _sharedMemoryId!.trim().isEmpty)) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('请选择共享记忆库')));
      return;
    }
    if (_memoryBindingMode == _memoryBindingShared &&
        !widget.sharedMemoryStores.any(
          (store) => store.id == _sharedMemoryId,
        )) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('共享记忆库不存在，请重新选择')));
      return;
    }
    if (_ttsBindingEnabled &&
        (_ttsConfigId == null ||
            !widget.ttsConfigs.any((config) => config.id == _ttsConfigId))) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('请选择 TTS 配置')));
      return;
    }
    final card = widget.card;
    Navigator.of(context).pop(
      _CharacterCardEditorSave(
        card: core_proxy.CharacterCard(
          id: card.id,
          name: _nameController.text.trim(),
          description: _descriptionController.text.trim(),
          characterSetting: _characterSettingController.text,
          openingStatement: _openingStatementController.text,
          otherContentChat: _otherContentChatController.text,
          otherContentVoice: _otherContentVoiceController.text,
          avatarUri: _normalizedAvatarUri(_avatarUri),
          attachedTagIds: List<String>.from(_attachedTagIds),
          advancedCustomPrompt: _advancedPromptController.text,
          marks: _marksController.text,
          chatModelBindingMode: _chatModelBindingMode,
          chatModelId: _chatModelBindingMode == _chatModelFixedConfig
              ? _chatModelId
              : null,
          ttsConfigId: _ttsBindingEnabled ? _ttsConfigId : null,
          memoryBindingMode: _memoryBindingMode,
          sharedMemoryId: _memoryBindingMode == _memoryBindingShared
              ? _sharedMemoryId
              : null,
          sharedMemoryMounts: const <core_proxy.CharacterSharedMemoryMount>[],
          toolAccessConfig: normalizedToolAccessConfig,
          isDefault: card.isDefault,
          createdAt: card.createdAt,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
        tagChanges: _PromptTagChangeSet(
          created: List<_PromptTagCreateDraft>.from(_createdTagDrafts),
          updated: List<_PromptTagUpdateDraft>.from(_updatedTagDrafts.values),
          deletedTagIds: List<String>.from(_deletedTagIds),
        ),
      ),
    );
  }

  Future<void> _createTag() async {
    final l10n = AppLocalizations.of(context)!;
    final edited = await _PromptTagEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersCreateTag,
    );
    if (!mounted || edited == null) {
      return;
    }
    final now = DateTime.now().millisecondsSinceEpoch;
    final draftId = 'draft_prompt_tag_${++_nextDraftTagIndex}';
    final tag = core_proxy.PromptTag(
      id: draftId,
      name: edited.name,
      description: edited.description,
      promptContent: edited.promptContent,
      tagType: core_proxy.TagType.custom,
      createdAt: now,
      updatedAt: now,
    );
    setState(() {
      _tags = <core_proxy.PromptTag>[..._tags, tag];
      _createdTagDrafts.add(
        _PromptTagCreateDraft(draftId: draftId, values: edited),
      );
      _attachedTagIds = <String>[..._attachedTagIds, draftId];
    });
  }

  Future<void> _editTag(core_proxy.PromptTag tag) async {
    final l10n = AppLocalizations.of(context)!;
    final edited = await _PromptTagEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersEditTag,
      tag: tag,
    );
    if (!mounted || edited == null) {
      return;
    }
    final updatedTag = core_proxy.PromptTag(
      id: tag.id,
      name: edited.name,
      description: edited.description,
      promptContent: edited.promptContent,
      tagType: tag.tagType,
      createdAt: tag.createdAt,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    final tagIndex = _tags.indexWhere((item) => item.id == tag.id);
    if (tagIndex < 0) {
      throw StateError('Unknown prompt tag: ${tag.id}');
    }
    final draftIndex = _createdTagDrafts.indexWhere(
      (draft) => draft.draftId == tag.id,
    );
    setState(() {
      _tags = <core_proxy.PromptTag>[
        ..._tags.take(tagIndex),
        updatedTag,
        ..._tags.skip(tagIndex + 1),
      ];
      if (draftIndex >= 0) {
        _createdTagDrafts[draftIndex] = _PromptTagCreateDraft(
          draftId: tag.id,
          values: edited,
        );
      } else {
        _updatedTagDrafts[tag.id] = _PromptTagUpdateDraft(
          tagId: tag.id,
          values: edited,
          tagType: tag.tagType,
        );
      }
    });
  }

  Future<void> _deleteTag(core_proxy.PromptTag tag) async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.settingsCharactersDeleteTag),
        content: Text(l10n.settingsCharactersDeleteTagMessage(tag.name)),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(l10n.delete),
          ),
        ],
      ),
    );
    if (!mounted || confirmed != true) {
      return;
    }
    final tagIndex = _tags.indexWhere((item) => item.id == tag.id);
    if (tagIndex < 0) {
      throw StateError('Unknown prompt tag: ${tag.id}');
    }
    final draftIndex = _createdTagDrafts.indexWhere(
      (draft) => draft.draftId == tag.id,
    );
    setState(() {
      _tags = <core_proxy.PromptTag>[
        ..._tags.take(tagIndex),
        ..._tags.skip(tagIndex + 1),
      ];
      _attachedTagIds = <String>[
        for (final tagId in _attachedTagIds)
          if (tagId != tag.id) tagId,
      ];
      if (draftIndex >= 0) {
        _createdTagDrafts.removeAt(draftIndex);
      } else {
        _updatedTagDrafts.remove(tag.id);
        _deletedTagIds.add(tag.id);
      }
    });
  }

  void _setTagSelected(String tagId, bool selected) {
    setState(() {
      if (selected) {
        if (!_attachedTagIds.contains(tagId)) {
          _attachedTagIds.add(tagId);
        }
      } else {
        _attachedTagIds.remove(tagId);
      }
    });
  }

  Future<void> _openTagManager() async {
    final l10n = AppLocalizations.of(context)!;
    await showDialog<void>(
      context: context,
      builder: (dialogContext) {
        return StatefulBuilder(
          builder: (context, setDialogState) {
            Future<void> runTagAction(Future<void> Function() action) async {
              await action();
              if (mounted) {
                setDialogState(() {});
              }
            }

            return OperitDialogScaffold(
              title: l10n.settingsCharactersManageTags,
              maxWidth: 560,
              maxHeight: 560,
              showCloseButton: true,
              onClose: () => Navigator.of(dialogContext).pop(),
              contentPadding: const EdgeInsets.fromLTRB(20, 8, 20, 16),
              actions: <Widget>[
                TextButton.icon(
                  onPressed: () => runTagAction(_createTag),
                  icon: const Icon(Icons.add, size: 18),
                  label: Text(l10n.settingsCharactersCreateTag),
                ),
                FilledButton(
                  onPressed: () => Navigator.of(dialogContext).pop(),
                  child: Text(l10n.close),
                ),
              ],
              child: _CharacterTagManagerList(
                tags: _tags,
                selectedTagIds: _attachedTagIds,
                onChanged: (tagId, selected) {
                  _setTagSelected(tagId, selected);
                  setDialogState(() {});
                },
                onEditTag: (tag) => runTagAction(() => _editTag(tag)),
                onDeleteTag: (tag) => runTagAction(() => _deleteTag(tag)),
              ),
            );
          },
        );
      },
    );
  }

  Future<void> _selectChatModel() async {
    final selected = await _CharacterModelSelectorDialog.show(
      context: context,
      title: AppLocalizations.of(context)!.settingsModelFunctionMappingsSelect(
        AppLocalizations.of(context)!.settingsCharactersChatModelConfig,
      ),
      summaries: widget.modelSummaries,
      currentModelId: _chatModelId,
    );
    if (selected == null) {
      return;
    }
    setState(() {
      _chatModelId = selected.modelId;
    });
  }

  Future<void> _selectTtsConfig() async {
    final selected = await _CharacterTtsConfigSelectorDialog.show(
      context: context,
      configs: widget.ttsConfigs,
      currentConfigId: _ttsConfigId,
    );
    if (selected == null) {
      return;
    }
    setState(() {
      _ttsConfigId = selected.id;
    });
  }

  Future<void> _setMemoryAutoUpdate(bool enabled) async {
    await widget.onSaveMemoryAutoUpdate(enabled);
    if (!mounted) {
      return;
    }
    setState(() {
      _enableMemoryAutoUpdate = enabled;
    });
  }

  Future<void> _setPreferenceDescription(bool enabled) async {
    await widget.onSavePreferenceDescription(enabled);
    if (!mounted) {
      return;
    }
    setState(() {
      _disableUserPreferenceDescription = !enabled;
    });
  }

  Future<void> _openToolAccessDialog() async {
    final edited = await _CharacterToolAccessDialog.show(
      context: context,
      config: _toolAccessConfig,
      builtinOptions: widget.builtinToolOptions,
      packageOptions: widget.packageToolOptions,
      skillOptions: widget.skillToolOptions,
      mcpOptions: widget.mcpToolOptions,
    );
    if (edited == null) {
      return;
    }
    setState(() {
      _toolAccessConfig = _normalizedToolAccessConfig(edited);
    });
  }

  /// Imports the selected avatar into runtime storage for the edited card.
  Future<void> _pickAvatarImage() async {
    const imageGroup = XTypeGroup(
      label: 'image',
      extensions: <String>['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif'],
    );
    final file = await openFile(acceptedTypeGroups: <XTypeGroup>[imageGroup]);
    if (file == null) {
      return;
    }
    final avatarUri = await CharacterAvatarStore().importFile(file);
    if (!mounted) {
      return;
    }
    setState(() {
      _avatarUri = avatarUri;
    });
  }

  Future<void> _exportCard() async {
    final action = await _CharacterCardExportDialog.show(context: context);
    if (!mounted || action == null) {
      return;
    }
    switch (action) {
      case _CharacterCardExportAction.nativeJson:
        Navigator.of(context).pop(const _CharacterCardEditorExportJson());
      case _CharacterCardExportAction.tavernJson:
        Navigator.of(context).pop(const _CharacterCardEditorExportTavernJson());
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final selectedModel = _providerModelSummaryById(
      widget.modelSummaries,
      _chatModelId,
    );
    final selectedTtsConfig = _ttsConfigById(widget.ttsConfigs, _ttsConfigId);
    final toolAccessSummary = _toolAccessSummary(l10n, _toolAccessConfig);
    final dialogActions = <Widget>[
      if (widget.showItemActions && !widget.card.isDefault)
        TextButton(
          onPressed: () =>
              Navigator.of(context).pop(const _CharacterCardEditorDelete()),
          child: Text(l10n.delete),
        ),
      if (widget.showItemActions)
        TextButton(
          onPressed: _exportCard,
          child: Text(l10n.settingsCharactersExport),
        ),
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: Text(l10n.cancel),
      ),
      FilledButton(onPressed: _save, child: Text(l10n.save)),
    ];
    return OperitDialogScaffold(
      title: widget.title,
      maxWidth: 760,
      maxHeight: 820,
      showCloseButton: true,
      onClose: () => Navigator.of(context).pop(),
      actions: dialogActions,
      contentPadding: EdgeInsets.zero,
      child: Form(
        key: _formKey,
        child: DefaultTabController(
          length: 3,
          child: Column(
            mainAxisSize: MainAxisSize.max,
            children: <Widget>[
              const TabBar(
                tabs: <Widget>[
                  Tab(text: '基础'),
                  Tab(text: '内容'),
                  Tab(text: '绑定'),
                ],
              ),
              Expanded(
                child: TabBarView(
                  children: <Widget>[
                    _CharacterCardEditorTabBody(
                      children: <Widget>[
                        _DialogTextField(
                          controller: _nameController,
                          label: l10n.settingsCharactersCardName,
                          requiredField: true,
                        ),
                        _DialogTextField(
                          controller: _descriptionController,
                          label: l10n.settingsCharactersDescription,
                        ),
                        _CharacterAvatarEditorField(
                          avatarUri: _avatarUri,
                          onChoose: _pickAvatarImage,
                          onClear: () {
                            setState(() {
                              _avatarUri = null;
                            });
                          },
                        ),
                        _DialogExpandableTextField(
                          controller: _characterSettingController,
                          label: l10n.settingsCharactersCharacterSetting,
                          maxLines: 6,
                        ),
                        _DialogExpandableTextField(
                          controller: _openingStatementController,
                          label: l10n.settingsCharactersOpeningStatement,
                          maxLines: 3,
                        ),
                        _CharacterTagPicker(
                          tags: _tags,
                          selectedTagIds: _attachedTagIds,
                          onManageTags: _openTagManager,
                          onChanged: (tagId, selected) {
                            _setTagSelected(tagId, selected);
                          },
                        ),
                      ],
                    ),
                    _CharacterCardEditorTabBody(
                      children: <Widget>[
                        _DialogExpandableTextField(
                          controller: _otherContentChatController,
                          label: l10n.settingsCharactersOtherContentChat,
                          maxLines: 4,
                        ),
                        _DialogExpandableTextField(
                          controller: _otherContentVoiceController,
                          label: l10n.settingsCharactersOtherContentVoice,
                          maxLines: 4,
                        ),
                        _DialogExpandableTextField(
                          controller: _advancedPromptController,
                          label: l10n.settingsCharactersAdvancedPrompt,
                          maxLines: 4,
                        ),
                        _DialogExpandableTextField(
                          controller: _marksController,
                          label: l10n.settingsCharactersMarks,
                          maxLines: 3,
                        ),
                      ],
                    ),
                    _CharacterCardEditorTabBody(
                      children: <Widget>[
                        _BindingSwitchSection(
                          title: '聊天模型',
                          subtitleOff:
                              l10n.settingsCharactersChatModelFollowGlobal,
                          subtitleOn:
                              l10n.settingsCharactersChatModelFixedConfig,
                          value: _chatModelBindingMode == _chatModelFixedConfig,
                          onChanged: (value) {
                            setState(() {
                              _chatModelBindingMode = value
                                  ? _chatModelFixedConfig
                                  : _chatModelFollowGlobal;
                              if (!value) {
                                _chatModelId = null;
                              }
                            });
                          },
                          children: <Widget>[
                            _DialogToolAccessConfigureField(
                              label: l10n.settingsCharactersChatModelConfig,
                              valueText: _characterModelBindingText(
                                selectedModel,
                                _chatModelId,
                              ),
                              onConfigure: _selectChatModel,
                            ),
                          ],
                        ),
                        _BindingSwitchSection(
                          title: 'TTS 配置',
                          subtitleOff: '跟随全局 TTS 配置',
                          subtitleOn: '使用角色卡 TTS 配置',
                          value: _ttsBindingEnabled,
                          onChanged: widget.ttsConfigs.isEmpty
                              ? null
                              : (value) {
                                  setState(() {
                                    _ttsBindingEnabled = value;
                                    if (!value) {
                                      _ttsConfigId = null;
                                    }
                                  });
                                },
                          children: <Widget>[
                            if (widget.ttsConfigs.isEmpty)
                              const Padding(
                                padding: EdgeInsets.only(bottom: 6),
                                child: Text('还没有 TTS 配置'),
                              )
                            else
                              _DialogToolAccessConfigureField(
                                label: 'TTS 配置',
                                valueText: _ttsConfigBindingText(
                                  selectedTtsConfig,
                                  _ttsConfigId,
                                ),
                                onConfigure: _selectTtsConfig,
                              ),
                          ],
                        ),
                        _BindingSwitchSection(
                          title: '记忆绑定',
                          subtitleOff: '使用角色记忆',
                          subtitleOn: '使用共享记忆',
                          value: _memoryBindingMode == _memoryBindingShared,
                          onChanged: widget.sharedMemoryStores.isEmpty
                              ? null
                              : (value) {
                                  setState(() {
                                    _memoryBindingMode = value
                                        ? _memoryBindingShared
                                        : _memoryBindingCharacter;
                                    if (!value) {
                                      _sharedMemoryId = null;
                                    }
                                  });
                                },
                          footerChildren: <Widget>[
                            Wrap(
                              spacing: 8,
                              runSpacing: 6,
                              children: <Widget>[
                                _BindingTogglePill(
                                  label: '读取记忆',
                                  selected: !_disableUserPreferenceDescription,
                                  onTap: () => _setPreferenceDescription(
                                    _disableUserPreferenceDescription,
                                  ),
                                ),
                                _BindingTogglePill(
                                  label: '写入记忆',
                                  selected: _enableMemoryAutoUpdate,
                                  onTap: () => _setMemoryAutoUpdate(
                                    !_enableMemoryAutoUpdate,
                                  ),
                                ),
                              ],
                            ),
                          ],
                          children: <Widget>[
                            if (widget.sharedMemoryStores.isEmpty)
                              const Padding(
                                padding: EdgeInsets.only(bottom: 12),
                                child: Text('还没有共享记忆库'),
                              )
                            else
                              DropdownButtonFormField<String>(
                                initialValue:
                                    widget.sharedMemoryStores.any(
                                      (store) => store.id == _sharedMemoryId,
                                    )
                                    ? _sharedMemoryId
                                    : null,
                                items: <DropdownMenuItem<String>>[
                                  for (final store in widget.sharedMemoryStores)
                                    DropdownMenuItem<String>(
                                      value: store.id,
                                      child: Text(store.name),
                                    ),
                                ],
                                onChanged: (value) {
                                  setState(() {
                                    _sharedMemoryId = value;
                                  });
                                },
                                decoration: const InputDecoration(
                                  labelText: '共享记忆库',
                                ),
                              ),
                          ],
                        ),
                        _BindingSwitchSection(
                          title: l10n.settingsCharactersToolAccess,
                          subtitleOff:
                              l10n.settingsCharactersToolAccessFollowGlobal,
                          subtitleOn: l10n.settingsCharactersToolAccessCustom,
                          value: _toolAccessConfig.enabled,
                          onChanged: (value) {
                            setState(() {
                              _toolAccessConfig =
                                  core_proxy.CharacterCardToolAccessConfig(
                                    enabled: value,
                                    allowedBuiltinTools:
                                        _toolAccessConfig.allowedBuiltinTools,
                                    allowedPackages:
                                        _toolAccessConfig.allowedPackages,
                                    allowedSkills:
                                        _toolAccessConfig.allowedSkills,
                                    allowedMcpServers:
                                        _toolAccessConfig.allowedMcpServers,
                                  );
                            });
                          },
                          children: <Widget>[
                            _DialogToolAccessConfigureField(
                              label: l10n.settingsCharactersToolAccessConfigure,
                              valueText: toolAccessSummary,
                              onConfigure: _openToolAccessDialog,
                            ),
                          ],
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CharacterCardEditorTabBody extends StatelessWidget {
  const _CharacterCardEditorTabBody({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(18, 10, 18, 10),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: children,
      ),
    );
  }
}

String? _normalizedAvatarUri(String? value) {
  final trimmed = value?.trim();
  return trimmed == null || trimmed.isEmpty ? null : trimmed;
}

class _CharacterAvatarEditorField extends StatelessWidget {
  const _CharacterAvatarEditorField({
    required this.avatarUri,
    required this.onChoose,
    required this.onClear,
  });

  final String? avatarUri;
  final VoidCallback onChoose;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final path = _normalizedAvatarUri(avatarUri);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: InputDecorator(
        decoration: const InputDecoration(labelText: '角色头像'),
        child: Row(
          children: <Widget>[
            SizedBox(
              width: 44,
              height: 44,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: colorScheme.surfaceContainerHighest,
                  shape: BoxShape.circle,
                ),
                child: ClipOval(
                  child: CharacterAvatarImage(
                    avatarUri: path,
                    fit: BoxFit.cover,
                  ),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                path ?? '未设置',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            ),
            const SizedBox(width: 8),
            TextButton(onPressed: onChoose, child: const Text('选择')),
            if (path != null)
              TextButton(onPressed: onClear, child: const Text('清除')),
          ],
        ),
      ),
    );
  }
}

class _BindingSwitchSection extends StatelessWidget {
  const _BindingSwitchSection({
    required this.title,
    required this.subtitleOff,
    required this.subtitleOn,
    required this.value,
    required this.onChanged,
    required this.children,
    this.footerChildren = const <Widget>[],
  });

  final String title;
  final String subtitleOff;
  final String subtitleOn;
  final bool value;
  final ValueChanged<bool>? onChanged;
  final List<Widget> children;
  final List<Widget> footerChildren;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final enabled = onChanged != null;
    final titleColor = enabled
        ? colorScheme.onSurface
        : colorScheme.onSurface.withValues(alpha: 0.46);
    final subtitleColor = enabled
        ? colorScheme.onSurfaceVariant
        : colorScheme.onSurfaceVariant.withValues(alpha: 0.46);
    return Padding(
      padding: const EdgeInsets.only(bottom: 7),
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.all(
            color: colorScheme.outlineVariant.withValues(alpha: 0.24),
          ),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(10, 6, 10, 6),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              ConstrainedBox(
                constraints: const BoxConstraints(minHeight: 46),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: <Widget>[
                    Expanded(
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          Text(
                            title,
                            style: textTheme.bodyMedium?.copyWith(
                              color: titleColor,
                              fontWeight: FontWeight.w700,
                              fontSize: 14,
                            ),
                          ),
                          const SizedBox(height: 1),
                          Text(
                            value ? subtitleOn : subtitleOff,
                            style: textTheme.bodySmall?.copyWith(
                              color: subtitleColor,
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
              if (value || onChanged == null) ...[
                const SizedBox(height: 6),
                ...children,
              ],
              if (footerChildren.isNotEmpty) ...[
                const SizedBox(height: 6),
                Divider(height: 1, color: colorScheme.outlineVariant),
                const SizedBox(height: 4),
                ...footerChildren,
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _BindingTogglePill extends StatelessWidget {
  const _BindingTogglePill({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final backgroundColor = selected
        ? colorScheme.primaryContainer.withValues(alpha: 0.86)
        : colorScheme.surfaceContainerHighest.withValues(alpha: 0.36);
    final foregroundColor = selected
        ? colorScheme.onPrimaryContainer
        : colorScheme.onSurfaceVariant;
    return Material(
      color: backgroundColor,
      shape: StadiumBorder(
        side: BorderSide(
          color: selected
              ? colorScheme.primary.withValues(alpha: 0.42)
              : colorScheme.outlineVariant.withValues(alpha: 0.42),
        ),
      ),
      child: InkWell(
        onTap: onTap,
        customBorder: const StadiumBorder(),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(
                selected ? Icons.check_rounded : Icons.close_rounded,
                size: 16,
                color: foregroundColor,
              ),
              const SizedBox(width: 6),
              Text(
                label,
                style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  color: foregroundColor,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CharacterModelSelectorDialog extends StatefulWidget {
  const _CharacterModelSelectorDialog({
    required this.title,
    required this.summaries,
    required this.currentModelId,
  });

  final String title;
  final List<core_proxy.ProviderModelSummary> summaries;
  final String? currentModelId;

  static Future<core_proxy.ProviderModelSummary?> show({
    required BuildContext context,
    required String title,
    required List<core_proxy.ProviderModelSummary> summaries,
    required String? currentModelId,
  }) {
    return showDialog<core_proxy.ProviderModelSummary>(
      context: context,
      builder: (context) => _CharacterModelSelectorDialog(
        title: title,
        summaries: summaries,
        currentModelId: currentModelId,
      ),
    );
  }

  @override
  State<_CharacterModelSelectorDialog> createState() =>
      _CharacterModelSelectorDialogState();
}

class _CharacterModelSelectorDialogState
    extends State<_CharacterModelSelectorDialog> {
  final _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _selectModel(core_proxy.ProviderModelSummary summary) {
    Navigator.of(context).pop(summary);
  }

  List<core_proxy.ProviderModelSummary> _filteredModels() {
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) {
      return widget.summaries;
    }
    return widget.summaries
        .where((summary) {
          final text =
              '${summary.modelId} ${summary.providerName} '
                      '${summary.providerTypeId}'
                  .toLowerCase();
          return text.contains(query);
        })
        .toList(growable: false);
  }

  Widget _modelList(AppLocalizations l10n) {
    final filteredModels = _filteredModels();
    return Column(
      children: <Widget>[
        TextField(
          controller: _searchController,
          decoration: InputDecoration(
            prefixIcon: const Icon(Icons.search),
            labelText: l10n.search,
          ),
          onChanged: (_) => setState(() {}),
        ),
        const SizedBox(height: 8),
        Expanded(
          child: filteredModels.isEmpty
              ? Center(child: Text(l10n.noData))
              : ListView.builder(
                  itemCount: filteredModels.length,
                  itemBuilder: (context, index) {
                    final summary = filteredModels[index];
                    return _CharacterModelOptionTile(
                      summary: summary,
                      selected: summary.modelId == widget.currentModelId,
                      onTap: () => _selectModel(summary),
                    );
                  },
                ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    if (widget.summaries.isEmpty) {
      return AlertDialog(
        title: Text(widget.title),
        content: SizedBox(width: 420, child: Text(l10n.noData)),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(l10n.cancel),
          ),
        ],
      );
    }
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(width: 560, height: 480, child: _modelList(l10n)),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
    );
  }
}

class _CharacterModelOptionTile extends StatelessWidget {
  const _CharacterModelOptionTile({
    required this.summary,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.ProviderModelSummary summary;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      type: MaterialType.transparency,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 9),
          child: Row(
            children: <Widget>[
              SizedBox(
                width: 24,
                child: Icon(
                  selected ? Icons.check_circle : Icons.circle_outlined,
                  size: 20,
                  color: selected
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      summary.modelId,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    Text(
                      summary.providerName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(color: colorScheme.onSurfaceVariant),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CharacterTtsConfigSelectorDialog extends StatefulWidget {
  const _CharacterTtsConfigSelectorDialog({
    required this.configs,
    required this.currentConfigId,
  });

  final List<core_proxy.TtsConfig> configs;
  final String? currentConfigId;

  static Future<core_proxy.TtsConfig?> show({
    required BuildContext context,
    required List<core_proxy.TtsConfig> configs,
    required String? currentConfigId,
  }) {
    return showDialog<core_proxy.TtsConfig>(
      context: context,
      builder: (context) => _CharacterTtsConfigSelectorDialog(
        configs: configs,
        currentConfigId: currentConfigId,
      ),
    );
  }

  @override
  State<_CharacterTtsConfigSelectorDialog> createState() =>
      _CharacterTtsConfigSelectorDialogState();
}

class _CharacterTtsConfigSelectorDialogState
    extends State<_CharacterTtsConfigSelectorDialog> {
  final _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  List<core_proxy.TtsConfig> _filteredConfigs() {
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) {
      return widget.configs;
    }
    return widget.configs
        .where((config) => _ttsConfigSearchText(config).contains(query))
        .toList(growable: false);
  }

  void _selectConfig(core_proxy.TtsConfig config) {
    Navigator.of(context).pop(config);
  }

  Widget _configList(AppLocalizations l10n) {
    final filteredConfigs = _filteredConfigs();
    return Column(
      children: <Widget>[
        TextField(
          controller: _searchController,
          decoration: InputDecoration(
            prefixIcon: const Icon(Icons.search),
            labelText: l10n.search,
          ),
          onChanged: (_) => setState(() {}),
        ),
        const SizedBox(height: 8),
        Expanded(
          child: filteredConfigs.isEmpty
              ? Center(child: Text(l10n.noData))
              : ListView.builder(
                  itemCount: filteredConfigs.length,
                  itemBuilder: (context, index) {
                    final config = filteredConfigs[index];
                    return _CharacterTtsConfigOptionTile(
                      config: config,
                      selected: config.id == widget.currentConfigId,
                      onTap: () => _selectConfig(config),
                    );
                  },
                ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    if (widget.configs.isEmpty) {
      return AlertDialog(
        title: const Text('选择 TTS 配置'),
        content: SizedBox(width: 420, child: Text(l10n.noData)),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(l10n.cancel),
          ),
        ],
      );
    }
    return AlertDialog(
      title: const Text('选择 TTS 配置'),
      content: SizedBox(width: 560, height: 480, child: _configList(l10n)),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
    );
  }
}

class _CharacterTtsConfigOptionTile extends StatelessWidget {
  const _CharacterTtsConfigOptionTile({
    required this.config,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.TtsConfig config;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      type: MaterialType.transparency,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 9),
          child: Row(
            children: <Widget>[
              SizedBox(
                width: 24,
                child: Icon(
                  selected ? Icons.check_circle : Icons.circle_outlined,
                  size: 20,
                  color: selected
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      _ttsConfigModelVoiceText(config),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    Text(
                      _ttsConfigProviderText(config),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(color: colorScheme.onSurfaceVariant),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CharacterToolAccessDialog extends StatefulWidget {
  const _CharacterToolAccessDialog({
    required this.config,
    required this.builtinOptions,
    required this.packageOptions,
    required this.skillOptions,
    required this.mcpOptions,
  });

  final core_proxy.CharacterCardToolAccessConfig config;
  final List<ToolAccessOption> builtinOptions;
  final List<ToolAccessOption> packageOptions;
  final List<ToolAccessOption> skillOptions;
  final List<ToolAccessOption> mcpOptions;

  /// Opens the character-card tool access editor.
  static Future<core_proxy.CharacterCardToolAccessConfig?> show({
    required BuildContext context,
    required core_proxy.CharacterCardToolAccessConfig config,
    required List<ToolAccessOption> builtinOptions,
    required List<ToolAccessOption> packageOptions,
    required List<ToolAccessOption> skillOptions,
    required List<ToolAccessOption> mcpOptions,
  }) {
    return showDialog<core_proxy.CharacterCardToolAccessConfig>(
      context: context,
      builder: (context) => _CharacterToolAccessDialog(
        config: config,
        builtinOptions: builtinOptions,
        packageOptions: packageOptions,
        skillOptions: skillOptions,
        mcpOptions: mcpOptions,
      ),
    );
  }

  /// Creates the state object that owns the tab selection and local access lists.
  @override
  State<_CharacterToolAccessDialog> createState() =>
      _CharacterToolAccessDialogState();
}

class _CharacterToolAccessDialogState extends State<_CharacterToolAccessDialog>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  late final TextEditingController _searchController;
  late Set<String> _builtinTools;
  late Set<String> _packages;
  late Set<String> _skills;
  late Set<String> _mcpServers;
  int _selectedTabIndex = 0;

  /// Initializes local selections and controllers from the normalized config.
  @override
  void initState() {
    super.initState();
    final config = _normalizedToolAccessConfig(widget.config);
    _builtinTools = config.allowedBuiltinTools.toSet();
    _packages = config.allowedPackages.toSet();
    _skills = config.allowedSkills.toSet();
    _mcpServers = config.allowedMcpServers.toSet();
    _tabController = TabController(length: 4, vsync: this);
    _searchController = TextEditingController();
  }

  /// Releases the tab and search controllers owned by the dialog.
  @override
  void dispose() {
    _tabController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  /// Returns the options belonging to the active tab.
  List<ToolAccessOption> _optionsForTab(int tabIndex) {
    return switch (tabIndex) {
      0 => widget.builtinOptions,
      1 => widget.packageOptions,
      2 => widget.skillOptions,
      _ => widget.mcpOptions,
    };
  }

  /// Returns the mutable selection set belonging to the active tab.
  Set<String> _selectionForTab(int tabIndex) {
    return switch (tabIndex) {
      0 => _builtinTools,
      1 => _packages,
      2 => _skills,
      _ => _mcpServers,
    };
  }

  /// Returns the localized empty-state message belonging to one tab.
  String _emptyTextForTab(AppLocalizations l10n, int tabIndex) {
    return switch (tabIndex) {
      0 => l10n.settingsCharactersToolAccessEmptyBuiltin,
      1 => l10n.settingsCharactersToolAccessEmptyPackages,
      2 => l10n.settingsCharactersToolAccessEmptySkills,
      _ => l10n.settingsCharactersToolAccessEmptyMcp,
    };
  }

  /// Clears the search query when the user moves to another tab.
  void _handleTabChanged(int tabIndex) {
    _selectedTabIndex = tabIndex;
    _searchController.clear();
    setState(() {});
  }

  /// Filters options by their key, title, and description like the reference dialog.
  List<ToolAccessOption> _filterOptions(List<ToolAccessOption> options) {
    final searchText = _searchController.text.trim().toLowerCase();
    if (searchText.isEmpty) {
      return options;
    }
    return options
        .where((option) {
          final searchableText =
              '${option.key}\n${option.title}\n${option.subtitle}'
                  .toLowerCase();
          return searchableText.contains(searchText);
        })
        .toList(growable: false);
  }

  /// Toggles one option in the active tab without changing other tabs.
  void _toggleSelection(String key) {
    final selection = _selectionForTab(_selectedTabIndex);
    if (selection.contains(key)) {
      selection.remove(key);
    } else {
      selection.add(key);
    }
    setState(() {});
  }

  /// Saves the normalized access configuration and closes the dialog.
  void _save() {
    final normalized = _normalizedToolAccessConfig(
      core_proxy.CharacterCardToolAccessConfig(
        enabled: widget.config.enabled,
        allowedBuiltinTools: _builtinTools.toList()..sort(),
        allowedPackages: _packages.toList()..sort(),
        allowedSkills: _skills.toList()..sort(),
        allowedMcpServers: _mcpServers.toList()..sort(),
      ),
    );
    Navigator.of(context).pop(normalized);
  }

  /// Builds the compact empty state used by the reference dialog.
  Widget _buildEmptyState(String text) {
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox(
      width: double.infinity,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Text(
          text,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 12, color: colorScheme.onSurfaceVariant),
        ),
      ),
    );
  }

  /// Builds the searchable option list and its empty states.
  Widget _buildOptionList(
    AppLocalizations l10n,
    List<ToolAccessOption> currentOptions,
    List<ToolAccessOption> filteredOptions,
  ) {
    if (currentOptions.isEmpty) {
      return _buildEmptyState(_emptyTextForTab(l10n, _selectedTabIndex));
    }
    if (filteredOptions.isEmpty) {
      return _buildEmptyState(l10n.settingsCharactersToolAccessEmptySearch);
    }

    final colorScheme = Theme.of(context).colorScheme;
    final selectedKeys = _selectionForTab(_selectedTabIndex);
    return ListView.separated(
      shrinkWrap: true,
      padding: EdgeInsets.zero,
      itemCount: filteredOptions.length,
      separatorBuilder: (context, index) => Divider(
        height: 1,
        thickness: 0.5,
        color: colorScheme.outlineVariant.withValues(alpha: 0.4),
      ),
      itemBuilder: (context, index) {
        final option = filteredOptions[index];
        final selected = selectedKeys.contains(option.key);
        return Material(
          color: Colors.transparent,
          child: InkWell(
            onTap: () => _toggleSelection(option.key),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.center,
                children: <Widget>[
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          option.title,
                          style: const TextStyle(
                            fontSize: 13,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                        if (option.subtitle.trim().isNotEmpty)
                          Text(
                            option.subtitle,
                            style: TextStyle(
                              fontSize: 11,
                              color: colorScheme.onSurfaceVariant,
                            ),
                          ),
                      ],
                    ),
                  ),
                  IgnorePointer(
                    child: Checkbox(value: selected, onChanged: (_) {}),
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  /// Builds the compact four-tab tool access dialog copied from the Kotlin UI.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final currentOptions = _optionsForTab(_selectedTabIndex);
    final filteredOptions = _filterOptions(currentOptions);
    final tabTitles = <String>[
      l10n.settingsCharactersToolAccessTabBuiltin,
      l10n.settingsCharactersToolAccessTabPackage,
      l10n.settingsCharactersToolAccessTabSkill,
      l10n.settingsCharactersToolAccessTabMcp,
    ];

    return Dialog(
      backgroundColor: Colors.transparent,
      surfaceTintColor: Colors.transparent,
      insetPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 24),
      child: FractionallySizedBox(
        widthFactor: 0.94,
        child: ConstrainedBox(
          constraints: BoxConstraints(
            maxWidth: 560,
            maxHeight: MediaQuery.sizeOf(context).height * 0.86,
          ),
          child: Material(
            color: colorScheme.surface,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(12),
              side: BorderSide(color: colorScheme.outlineVariant),
            ),
            clipBehavior: Clip.antiAlias,
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  Align(
                    alignment: AlignmentDirectional.centerStart,
                    child: Text(
                      l10n.settingsCharactersToolAccessTitle,
                      style: const TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  const SizedBox(height: 10),
                  TabBar(
                    controller: _tabController,
                    isScrollable: true,
                    tabAlignment: TabAlignment.start,
                    labelPadding: const EdgeInsets.symmetric(horizontal: 10),
                    indicatorSize: TabBarIndicatorSize.label,
                    dividerHeight: 1,
                    labelStyle: const TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                    ),
                    unselectedLabelStyle: const TextStyle(fontSize: 12),
                    onTap: _handleTabChanged,
                    tabs: <Widget>[
                      for (final title in tabTitles) Tab(text: title),
                    ],
                  ),
                  const SizedBox(height: 10),
                  if (currentOptions.isNotEmpty) ...<Widget>[
                    TextField(
                      controller: _searchController,
                      onChanged: (value) => setState(() {}),
                      textInputAction: TextInputAction.search,
                      style: const TextStyle(fontSize: 14),
                      decoration: InputDecoration(
                        isDense: true,
                        prefixIcon: const Icon(Icons.search, size: 20),
                        suffixIcon: _searchController.text.isNotEmpty
                            ? IconButton(
                                onPressed: () {
                                  _searchController.clear();
                                  setState(() {});
                                },
                                icon: const Icon(Icons.close, size: 18),
                                tooltip: l10n.clear,
                              )
                            : null,
                        hintText:
                            l10n.settingsCharactersToolAccessSearchPlaceholder,
                        hintStyle: const TextStyle(fontSize: 14),
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 10,
                        ),
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(10),
                        ),
                      ),
                    ),
                    const SizedBox(height: 8),
                  ],
                  SizedBox(
                    width: double.infinity,
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(maxHeight: 320),
                      child: _buildOptionList(
                        l10n,
                        currentOptions,
                        filteredOptions,
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  Row(
                    children: <Widget>[
                      Expanded(
                        child: OutlinedButton(
                          onPressed: () => Navigator.of(context).pop(),
                          child: Text(l10n.cancel),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: FilledButton(
                          onPressed: _save,
                          child: Text(l10n.save),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _CharacterTagPicker extends StatelessWidget {
  const _CharacterTagPicker({
    required this.tags,
    required this.selectedTagIds,
    required this.onManageTags,
    required this.onChanged,
  });

  final List<core_proxy.PromptTag> tags;
  final List<String> selectedTagIds;
  final VoidCallback onManageTags;
  final void Function(String tagId, bool selected) onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  l10n.settingsCharactersTags,
                  style: const TextStyle(fontWeight: FontWeight.w700),
                ),
              ),
              TextButton.icon(
                onPressed: onManageTags,
                style: TextButton.styleFrom(
                  visualDensity: VisualDensity.compact,
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
                icon: const Icon(Icons.tune_outlined, size: 18),
                label: Text(l10n.settingsCharactersManageTags),
              ),
            ],
          ),
          const SizedBox(height: 6),
          if (tags.isEmpty)
            Text(
              l10n.settingsCharactersNoTags,
              style: TextStyle(color: colorScheme.onSurfaceVariant),
            )
          else
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                for (final tag in tags)
                  FilterChip(
                    materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    visualDensity: VisualDensity.compact,
                    selected: selectedTagIds.contains(tag.id),
                    label: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 160),
                      child: Text(tag.name, overflow: TextOverflow.ellipsis),
                    ),
                    onSelected: (value) => onChanged(tag.id, value),
                  ),
              ],
            ),
        ],
      ),
    );
  }
}

class _CharacterTagManagerList extends StatelessWidget {
  const _CharacterTagManagerList({
    required this.tags,
    required this.selectedTagIds,
    required this.onChanged,
    required this.onEditTag,
    required this.onDeleteTag,
  });

  final List<core_proxy.PromptTag> tags;
  final List<String> selectedTagIds;
  final void Function(String tagId, bool selected) onChanged;
  final ValueChanged<core_proxy.PromptTag> onEditTag;
  final ValueChanged<core_proxy.PromptTag> onDeleteTag;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    if (tags.isEmpty) {
      return Center(
        child: Text(
          l10n.settingsCharactersNoTags,
          textAlign: TextAlign.center,
          style: TextStyle(color: colorScheme.onSurfaceVariant),
        ),
      );
    }
    return ListView.separated(
      itemCount: tags.length,
      separatorBuilder: (context, index) =>
          Divider(height: 1, color: colorScheme.outlineVariant),
      itemBuilder: (context, index) {
        final tag = tags[index];
        return _CharacterTagManagerRow(
          tag: tag,
          selected: selectedTagIds.contains(tag.id),
          onSelected: (selected) => onChanged(tag.id, selected),
          onEdit: () => onEditTag(tag),
          onDelete: () => onDeleteTag(tag),
        );
      },
    );
  }
}

class _CharacterTagManagerRow extends StatelessWidget {
  const _CharacterTagManagerRow({
    required this.tag,
    required this.selected,
    required this.onSelected,
    required this.onEdit,
    required this.onDelete,
  });

  final core_proxy.PromptTag tag;
  final bool selected;
  final ValueChanged<bool> onSelected;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final subtitleParts = <String>[
      if (tag.description.trim().isNotEmpty) tag.description.trim(),
      _tagTypeText(tag.tagType),
    ];
    return ListTile(
      dense: true,
      contentPadding: EdgeInsets.zero,
      horizontalTitleGap: 8,
      minLeadingWidth: 28,
      onTap: () => onSelected(!selected),
      leading: Checkbox(
        value: selected,
        visualDensity: VisualDensity.compact,
        onChanged: (value) {
          if (value == null) {
            return;
          }
          onSelected(value);
        },
      ),
      title: Text(
        tag.name,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
      ),
      subtitle: subtitleParts.isEmpty
          ? null
          : Text(
              subtitleParts.join(' · '),
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          IconButton(
            tooltip: l10n.settingsCharactersEditTag,
            visualDensity: VisualDensity.compact,
            onPressed: onEdit,
            icon: const Icon(Icons.edit_outlined),
          ),
          IconButton(
            tooltip: l10n.settingsCharactersDeleteTag,
            visualDensity: VisualDensity.compact,
            onPressed: onDelete,
            icon: const Icon(Icons.delete_outline),
          ),
        ],
      ),
    );
  }
}
