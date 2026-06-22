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

class _CharacterCardEditorCopyJson extends _CharacterCardEditorResult {
  const _CharacterCardEditorCopyJson();
}

class _CharacterCardEditorCopyTavernJson extends _CharacterCardEditorResult {
  const _CharacterCardEditorCopyTavernJson();
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
  final List<_TtsConfigSummary> ttsConfigs;
  final bool enableMemoryAutoUpdate;
  final bool disableUserPreferenceDescription;
  final Future<void> Function(bool enabled) onSaveMemoryAutoUpdate;
  final Future<void> Function(bool enabled) onSavePreferenceDescription;
  final List<_ToolAccessOption> builtinToolOptions;
  final List<_ToolAccessOption> packageToolOptions;
  final List<_ToolAccessOption> skillToolOptions;
  final List<_ToolAccessOption> mcpToolOptions;
  final List<core_proxy.PromptTag> tags;

  static Future<_CharacterCardEditorResult?> show({
    required BuildContext context,
    required String title,
    required core_proxy.CharacterCard card,
    required bool showItemActions,
    required List<core_proxy.ProviderModelSummary> modelSummaries,
    required List<core_proxy.SharedMemoryStore> sharedMemoryStores,
    required List<_TtsConfigSummary> ttsConfigs,
    required bool enableMemoryAutoUpdate,
    required bool disableUserPreferenceDescription,
    required Future<void> Function(bool enabled) onSaveMemoryAutoUpdate,
    required Future<void> Function(bool enabled) onSavePreferenceDescription,
    required List<_ToolAccessOption> builtinToolOptions,
    required List<_ToolAccessOption> packageToolOptions,
    required List<_ToolAccessOption> skillToolOptions,
    required List<_ToolAccessOption> mcpToolOptions,
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
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('请选择共享记忆库')),
      );
      return;
    }
    if (_memoryBindingMode == _memoryBindingShared &&
        !widget.sharedMemoryStores.any((store) => store.id == _sharedMemoryId)) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('共享记忆库不存在，请重新选择')),
      );
      return;
    }
    if (_ttsBindingEnabled &&
        (_ttsConfigId == null ||
            !widget.ttsConfigs.any((config) => config.id == _ttsConfigId))) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('请选择 TTS 配置')),
      );
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
      title: AppLocalizations.of(
        context,
      )!.settingsModelFunctionMappingsSelect(
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

  Future<void> _exportCard() async {
    final action = await _CharacterCardExportDialog.show(context: context);
    if (!mounted || action == null) {
      return;
    }
    switch (action) {
      case _CharacterCardExportAction.nativeJson:
        Navigator.of(context).pop(const _CharacterCardEditorCopyJson());
      case _CharacterCardExportAction.tavernJson:
        Navigator.of(context).pop(const _CharacterCardEditorCopyTavernJson());
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final selectedModel = _providerModelSummaryById(
      widget.modelSummaries,
      _chatModelId,
    );
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
                          subtitleOff: l10n.settingsCharactersChatModelFollowGlobal,
                          subtitleOn: l10n.settingsCharactersChatModelFixedConfig,
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
                          subtitleOff: '不绑定 TTS 配置',
                          subtitleOn: '使用指定 TTS 配置',
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
                              Padding(
                                padding: const EdgeInsets.only(bottom: 6),
                                child: DropdownButtonFormField<String?>(
                                  initialValue: widget.ttsConfigs.any(
                                    (config) => config.id == _ttsConfigId,
                                  )
                                      ? _ttsConfigId
                                      : null,
                                  items: <DropdownMenuItem<String?>>[
                                    for (final config in widget.ttsConfigs)
                                      DropdownMenuItem<String?>(
                                        value: config.id,
                                        child: Text(
                                          '${config.name} · ${config.model} · ${config.voice}',
                                        ),
                                      ),
                                  ],
                                  onChanged: (value) {
                                    setState(() {
                                      _ttsConfigId = value;
                                    });
                                  },
                                  style: OperitFormStyles.dropdownTextStyle(context),
                                  decoration: const InputDecoration(
                                    labelText: 'TTS 配置',
                                  ),
                                ),
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
                          children: <Widget>[
                            if (widget.sharedMemoryStores.isEmpty)
                              const Padding(
                                padding: EdgeInsets.only(bottom: 12),
                                child: Text('还没有共享记忆库'),
                              )
                            else
                              DropdownButtonFormField<String>(
                                initialValue: widget.sharedMemoryStores.any(
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
                        ),
                        _BindingSwitchSection(
                          title: l10n.settingsCharactersToolAccess,
                          subtitleOff: l10n.settingsCharactersToolAccessFollowGlobal,
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
          final text = '${summary.modelId} ${summary.providerName} '
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

class _CharacterToolAccessDialog extends StatefulWidget {
  const _CharacterToolAccessDialog({
    required this.config,
    required this.builtinOptions,
    required this.packageOptions,
    required this.skillOptions,
    required this.mcpOptions,
  });

  final core_proxy.CharacterCardToolAccessConfig config;
  final List<_ToolAccessOption> builtinOptions;
  final List<_ToolAccessOption> packageOptions;
  final List<_ToolAccessOption> skillOptions;
  final List<_ToolAccessOption> mcpOptions;

  static Future<core_proxy.CharacterCardToolAccessConfig?> show({
    required BuildContext context,
    required core_proxy.CharacterCardToolAccessConfig config,
    required List<_ToolAccessOption> builtinOptions,
    required List<_ToolAccessOption> packageOptions,
    required List<_ToolAccessOption> skillOptions,
    required List<_ToolAccessOption> mcpOptions,
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

  @override
  State<_CharacterToolAccessDialog> createState() =>
      _CharacterToolAccessDialogState();
}

class _CharacterToolAccessDialogState
    extends State<_CharacterToolAccessDialog> {
  late Set<String> _builtinTools;
  late Set<String> _packages;
  late Set<String> _skills;
  late Set<String> _mcpServers;

  @override
  void initState() {
    super.initState();
    final config = _normalizedToolAccessConfig(widget.config);
    _builtinTools = config.allowedBuiltinTools.toSet();
    _packages = config.allowedPackages.toSet();
    _skills = config.allowedSkills.toSet();
    _mcpServers = config.allowedMcpServers.toSet();
  }

  void _save() {
    Navigator.of(context).pop(
      core_proxy.CharacterCardToolAccessConfig(
        enabled: widget.config.enabled,
        allowedBuiltinTools: _builtinTools.toList(growable: false)..sort(),
        allowedPackages: _packages.toList(growable: false)..sort(),
        allowedSkills: _skills.toList(growable: false)..sort(),
        allowedMcpServers: _mcpServers.toList(growable: false)..sort(),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsCharactersToolAccessConfigure),
      content: SizedBox(
        width: 620,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              _ToolAccessOptionGroup(
                title: l10n.settingsCharactersBuiltinTools,
                emptyText: l10n.settingsCharactersToolAccessEmptyBuiltin,
                options: widget.builtinOptions,
                selectedKeys: _builtinTools,
                onChanged: (key, selected) {
                  setState(() {
                    _setSelection(_builtinTools, key, selected);
                  });
                },
              ),
              _ToolAccessOptionGroup(
                title: l10n.settingsCharactersAllowedPackages,
                emptyText: l10n.settingsCharactersToolAccessEmptyPackages,
                options: widget.packageOptions,
                selectedKeys: _packages,
                onChanged: (key, selected) {
                  setState(() {
                    _setSelection(_packages, key, selected);
                  });
                },
              ),
              _ToolAccessOptionGroup(
                title: l10n.settingsCharactersAllowedSkills,
                emptyText: l10n.settingsCharactersToolAccessEmptySkills,
                options: widget.skillOptions,
                selectedKeys: _skills,
                onChanged: (key, selected) {
                  setState(() {
                    _setSelection(_skills, key, selected);
                  });
                },
              ),
              _ToolAccessOptionGroup(
                title: l10n.settingsCharactersAllowedMcpServers,
                emptyText: l10n.settingsCharactersToolAccessEmptyMcp,
                options: widget.mcpOptions,
                selectedKeys: _mcpServers,
                onChanged: (key, selected) {
                  setState(() {
                    _setSelection(_mcpServers, key, selected);
                  });
                },
              ),
            ],
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

class _ToolAccessOptionGroup extends StatelessWidget {
  const _ToolAccessOptionGroup({
    required this.title,
    required this.emptyText,
    required this.options,
    required this.selectedKeys,
    required this.onChanged,
  });

  final String title;
  final String emptyText;
  final List<_ToolAccessOption> options;
  final Set<String> selectedKeys;
  final void Function(String key, bool selected) onChanged;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return Theme(
      data: Theme.of(context).copyWith(
        listTileTheme: const ListTileThemeData(
          dense: true,
          minVerticalPadding: 0,
          horizontalTitleGap: 8,
        ),
      ),
      child: ExpansionTile(
        title: Text(
          title,
          style: textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w700),
        ),
        tilePadding: const EdgeInsets.symmetric(horizontal: 6),
        childrenPadding: const EdgeInsets.only(bottom: 6),
        visualDensity: VisualDensity.compact,
        minTileHeight: 42,
        children: <Widget>[
          if (options.isEmpty)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Text(emptyText, style: textTheme.bodyMedium),
            )
          else
            for (final option in options)
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                visualDensity: VisualDensity.compact,
                title: Text(
                  option.title,
                  style: textTheme.bodyMedium?.copyWith(
                    fontWeight: FontWeight.w500,
                  ),
                ),
                subtitle: option.subtitle.isEmpty
                    ? null
                    : Text(option.subtitle, style: textTheme.bodySmall),
                value: selectedKeys.contains(option.key),
                onChanged: (selected) {
                  if (selected == null) {
                    return;
                  }
                  onChanged(option.key, selected);
                },
              ),
        ],
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

