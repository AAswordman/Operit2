// ignore_for_file: file_names

import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../common/components/OperitDialog.dart';
import '../../../theme/OperitFormStyles.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';
import '../tts/TtsSettingsPanel.dart';
import 'MemoryGraphScreen.dart';
part 'CharacterSettingsPanelWidgets.dart';
part 'CharacterCardEditorDialog.dart';
part 'CharacterGroupDialogs.dart';

class CharacterSettingsPanel extends StatefulWidget {
  const CharacterSettingsPanel({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<CharacterSettingsPanel> createState() => _CharacterSettingsPanelState();
}

class _CharacterSettingsPanelState extends State<CharacterSettingsPanel> {
  Future<_CharacterSettingsData>? _future;
  final TtsConfigManagementController _ttsConfigController =
      TtsConfigManagementController();

  GeneratedRepositoryUserMarkdownRepositoryCoreProxy _userMarkdownRepository(
    String ownerKey,
  ) {
    return GeneratedRepositoryUserMarkdownRepositoryCoreProxy(
      widget.clients.bridge,
      CoreObjectPath(<String>[
        'repository',
        'userMarkdownRepository',
        ownerKey,
      ]),
    );
  }

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<_CharacterSettingsData> _load() async {
    final cardManager = widget.clients.preferencesCharacterCardManager;
    final groupManager = widget.clients.preferencesCharacterGroupCardManager;
    final sharedMemoryManager =
        widget.clients.preferencesSharedMemoryStoreManager;
    final apiPreferences = widget.clients.preferencesApiPreferences;
    final modelManager = widget.clients.preferencesModelConfigManager;
    final toolHandler = widget.clients.permissionsAiToolHandler;
    final packageManager = widget.clients.permissionsPackToolPackageManager;
    final skillRepository = widget.clients.skillRepository;
    final mcpLocalServer = widget.clients.mcpLocalServer;
    final promptTagManager = widget.clients.preferencesPromptTagManager;
    await cardManager.initializeIfNeeded();
    await groupManager.initializeIfNeeded();
    await modelManager.initializeIfNeeded();
    await toolHandler.registerDefaultTools();
    final toolNames =
        (await toolHandler.getAllToolNames())
            .where((toolName) => !_hiddenToolNames.contains(toolName))
            .toList(growable: false)
          ..sort(
            (left, right) => left.toLowerCase().compareTo(right.toLowerCase()),
          );
    final enabledPackageNames = await packageManager.getEnabledPackageNames();
    final packageOptions = <_ToolAccessOption>[];
    for (final packageName in enabledPackageNames) {
      final isContainer = await packageManager.isToolPkgContainer(
        packageName: packageName,
      );
      if (!isContainer) {
        packageOptions.add(
          _ToolAccessOption(key: packageName, title: packageName),
        );
      }
    }
    packageOptions.sort(_compareToolAccessOption);
    final skillOptions =
        (await skillRepository.getAiVisibleSkillPackages()).entries
            .map(
              (entry) => _ToolAccessOption(
                key: entry.key,
                title: entry.key,
                subtitle: entry.value.description,
              ),
            )
            .toList(growable: false)
          ..sort(_compareToolAccessOption);
    final mcpOptions =
        (await mcpLocalServer.getAllMcpServers()).entries
            .map(
              (entry) => _ToolAccessOption(
                key: entry.key,
                title: entry.key,
                subtitle: _mcpServerSubtitle(entry.value),
              ),
            )
            .toList(growable: false)
          ..sort(_compareToolAccessOption);
    final cards = await cardManager.getAllCharacterCards();
    final groups = await groupManager.getAllCharacterGroupCards();
    final activePrompt = _activePromptSelection(
      await widget.clients.preferencesActivePromptManager.getActivePrompt(),
    );
    final preferencesManager = UserPreferencesManager(clients: widget.clients);
    return _CharacterSettingsData(
      cards: cards,
      groups: groups,
      cardAvatarUris: await _loadCharacterCardAvatarUris(
        cards,
        preferencesManager,
      ),
      groupAvatarUris: await _loadCharacterGroupAvatarUris(
        groups,
        preferencesManager,
      ),
      sharedMemoryStores: await sharedMemoryManager.getAllSharedMemoryStores(),
      tags: await promptTagManager.getAllTags(),
      modelSummaries: await modelManager.getAllModelSummaries(),
      ttsConfigs: await _loadTtsConfigs(),
      builtinToolOptions: toolNames
          .map((toolName) => _ToolAccessOption(key: toolName, title: toolName))
          .toList(growable: false),
      packageToolOptions: packageOptions,
      skillToolOptions: skillOptions,
      mcpToolOptions: mcpOptions,
      activeCardId: activePrompt.cardId,
      activeGroupId: activePrompt.groupId,
      enableMemoryAutoUpdate: await apiPreferences
          .enableMemoryAutoUpdateFlowSnapshot(),
      disableUserPreferenceDescription: await apiPreferences
          .disableUserPreferenceDescriptionFlowSnapshot(),
    );
  }

  Future<List<_TtsConfigSummary>> _loadTtsConfigs() async {
    final value = await widget.clients.bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('preferences.ttsConfigManager'),
        methodName: 'getAllTtsConfigs',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>)
        .map((item) => _TtsConfigSummary.fromJson(item as Map<String, Object?>))
        .toList(growable: false);
  }

  Future<Map<String, String>> _loadCharacterCardAvatarUris(
    List<core_proxy.CharacterCard> cards,
    UserPreferencesManager preferencesManager,
  ) async {
    final avatarUris = <String, String>{};
    for (final card in cards) {
      final hasTheme = await preferencesManager.hasCharacterCardTheme(card.id);
      if (hasTheme) {
        final snapshot = await preferencesManager
            .resolveThemePreferenceSnapshot(characterCardId: card.id);
        final avatarUri = snapshot.customAiAvatarUri?.trim();
        if (avatarUri != null && avatarUri.isNotEmpty) {
          avatarUris[card.id] = avatarUri;
        }
      }
    }
    return avatarUris;
  }

  Future<Map<String, String>> _loadCharacterGroupAvatarUris(
    List<core_proxy.CharacterGroupCard> groups,
    UserPreferencesManager preferencesManager,
  ) async {
    final avatarUris = <String, String>{};
    for (final group in groups) {
      final hasTheme = await preferencesManager.hasCharacterGroupTheme(
        group.id,
      );
      if (hasTheme) {
        final snapshot = await preferencesManager
            .resolveThemePreferenceSnapshot(characterGroupId: group.id);
        final avatarUri = snapshot.customAiAvatarUri?.trim();
        if (avatarUri != null && avatarUri.isNotEmpty) {
          avatarUris[group.id] = avatarUri;
        }
      }
    }
    return avatarUris;
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _copyCharacterCardJson(core_proxy.CharacterCard card) async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = const JsonEncoder.withIndent('  ').convert(card.toJson());
    await Clipboard.setData(ClipboardData(text: jsonText));
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(l10n.settingsCharactersJsonCopied(card.name))),
    );
  }

  Future<void> _copyCharacterCardTavernJson(
    core_proxy.CharacterCard card,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    try {
      final jsonText = await widget.clients.preferencesCharacterCardManager
          .exportCharacterCardToTavernJson(characterCardId: card.id);
      await Clipboard.setData(ClipboardData(text: jsonText));
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.settingsCharactersTavernJsonCopied(card.name)),
        ),
      );
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.settingsCharactersTavernJsonCopyError('$error')),
        ),
      );
    }
  }

  Future<void> _importCharacterCardJson() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _JsonImportDialog.show(
      context: context,
      title: l10n.settingsCharactersImportCardJson,
      label: l10n.settingsCharactersJsonInput,
    );
    if (jsonText == null) {
      return;
    }
    try {
      final now = DateTime.now().millisecondsSinceEpoch;
      final imported = core_proxy.CharacterCard.fromJson(
        _jsonObjectFromText(jsonText),
      );
      final card = _characterCardWith(
        imported,
        id: '',
        isDefault: false,
        createdAt: now,
        updatedAt: now,
      );
      await widget.clients.preferencesCharacterCardManager.createCharacterCard(
        card: card,
      );
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsCharactersImportCardJsonDone)),
      );
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.settingsCharactersImportJsonError('$error')),
        ),
      );
    }
  }

  Future<void> _importTavernCharacterCardJson() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _JsonImportDialog.show(
      context: context,
      title: l10n.settingsCharactersImportTavernJson,
      label: l10n.settingsCharactersTavernJsonInput,
    );
    if (jsonText == null) {
      return;
    }
    try {
      await widget.clients.preferencesCharacterCardManager
          .createCharacterCardFromTavernJson(jsonString: jsonText);
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsCharactersImportTavernJsonDone)),
      );
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.settingsCharactersImportTavernJsonError('$error')),
        ),
      );
    }
  }

  Future<void> _chooseCharacterCardImport() async {
    final action = await _CharacterCardImportDialog.show(context: context);
    if (action == null) {
      return;
    }
    switch (action) {
      case _CharacterCardImportAction.nativeJson:
        await _importCharacterCardJson();
      case _CharacterCardImportAction.tavernJson:
        await _importTavernCharacterCardJson();
    }
  }

  Future<void> _copyCharacterGroupJson(
    core_proxy.CharacterGroupCard group,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = const JsonEncoder.withIndent('  ').convert(group.toJson());
    await Clipboard.setData(ClipboardData(text: jsonText));
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(l10n.settingsCharactersJsonCopied(group.name))),
    );
  }

  Future<void> _importCharacterGroupJson() async {
    final l10n = AppLocalizations.of(context)!;
    final jsonText = await _JsonImportDialog.show(
      context: context,
      title: l10n.settingsCharactersImportGroupJson,
      label: l10n.settingsCharactersJsonInput,
    );
    if (jsonText == null) {
      return;
    }
    try {
      final now = DateTime.now().millisecondsSinceEpoch;
      final imported = core_proxy.CharacterGroupCard.fromJson(
        _jsonObjectFromText(jsonText),
      );
      final group = core_proxy.CharacterGroupCard(
        id: '',
        name: imported.name,
        description: imported.description,
        members: imported.members,
        createdAt: now,
        updatedAt: now,
      );
      await widget.clients.preferencesCharacterGroupCardManager
          .createCharacterGroupCard(group: group);
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.settingsCharactersImportGroupJsonDone)),
      );
      _reload();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.settingsCharactersImportJsonError('$error')),
        ),
      );
    }
  }

  Future<void> _createCard(_CharacterSettingsData data) async {
    final l10n = AppLocalizations.of(context)!;
    final now = DateTime.now().millisecondsSinceEpoch;
    final card = core_proxy.CharacterCard(
      id: '',
      name: '',
      description: '',
      characterSetting: '',
      openingStatement: '',
      otherContentChat: '',
      otherContentVoice: '',
      attachedTagIds: const <String>[],
      advancedCustomPrompt: '',
      marks: '',
      chatModelBindingMode: 'FOLLOW_GLOBAL',
      chatModelId: null,
      ttsConfigId: null,
      memoryBindingMode: _memoryBindingCharacter,
      sharedMemoryId: null,
      sharedMemoryMounts: const <core_proxy.CharacterSharedMemoryMount>[],
      toolAccessConfig: const core_proxy.CharacterCardToolAccessConfig(
        enabled: false,
        allowedBuiltinTools: <String>[],
        allowedPackages: <String>[],
        allowedSkills: <String>[],
        allowedMcpServers: <String>[],
      ),
      isDefault: false,
      createdAt: now,
      updatedAt: now,
    );
    final result = await _CharacterCardEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersCreateCard,
      card: card,
      showItemActions: false,
      modelSummaries: data.modelSummaries,
      sharedMemoryStores: data.sharedMemoryStores,
      ttsConfigs: data.ttsConfigs,
      enableMemoryAutoUpdate: data.enableMemoryAutoUpdate,
      disableUserPreferenceDescription: data.disableUserPreferenceDescription,
      onSaveMemoryAutoUpdate: _saveMemoryAutoUpdate,
      onSavePreferenceDescription: _savePreferenceDescription,
      builtinToolOptions: data.builtinToolOptions,
      packageToolOptions: data.packageToolOptions,
      skillToolOptions: data.skillToolOptions,
      mcpToolOptions: data.mcpToolOptions,
      tags: data.tags,
    );
    if (result == null) {
      return;
    }
    switch (result) {
      case _CharacterCardEditorSave(:final card, :final tagChanges):
        final edited = await _applyCharacterCardTagChanges(
          card: card,
          tagChanges: tagChanges,
        );
        await widget.clients.preferencesCharacterCardManager
            .createCharacterCard(card: edited);
        _reload();
      case _CharacterCardEditorCopyJson() ||
          _CharacterCardEditorCopyTavernJson() ||
          _CharacterCardEditorDelete():
        return;
    }
  }

  Future<void> _editCard(
    core_proxy.CharacterCard card,
    _CharacterSettingsData data,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final result = await _CharacterCardEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersEditCard,
      card: card,
      showItemActions: true,
      modelSummaries: data.modelSummaries,
      sharedMemoryStores: data.sharedMemoryStores,
      ttsConfigs: data.ttsConfigs,
      enableMemoryAutoUpdate: data.enableMemoryAutoUpdate,
      disableUserPreferenceDescription: data.disableUserPreferenceDescription,
      onSaveMemoryAutoUpdate: _saveMemoryAutoUpdate,
      onSavePreferenceDescription: _savePreferenceDescription,
      builtinToolOptions: data.builtinToolOptions,
      packageToolOptions: data.packageToolOptions,
      skillToolOptions: data.skillToolOptions,
      mcpToolOptions: data.mcpToolOptions,
      tags: data.tags,
    );
    if (result == null) {
      return;
    }
    switch (result) {
      case _CharacterCardEditorSave(:final card, :final tagChanges):
        final edited = await _applyCharacterCardTagChanges(
          card: card,
          tagChanges: tagChanges,
        );
        await widget.clients.preferencesCharacterCardManager
            .updateCharacterCard(card: edited);
        _reload();
      case _CharacterCardEditorCopyJson():
        await _copyCharacterCardJson(card);
      case _CharacterCardEditorCopyTavernJson():
        await _copyCharacterCardTavernJson(card);
      case _CharacterCardEditorDelete():
        await _deleteCard(card);
    }
  }

  Future<void> _deleteCard(core_proxy.CharacterCard card) async {
    await widget.clients.preferencesCharacterCardManager.deleteCharacterCard(
      id: card.id,
    );
    _reload();
  }

  Future<core_proxy.CharacterCard> _applyCharacterCardTagChanges({
    required core_proxy.CharacterCard card,
    required _PromptTagChangeSet tagChanges,
  }) async {
    final promptTagManager = widget.clients.preferencesPromptTagManager;
    final createdTagIds = <String, String>{};
    for (final draft in tagChanges.created) {
      final createdId = await promptTagManager.createPromptTag(
        name: draft.values.name,
        description: draft.values.description,
        promptContent: draft.values.promptContent,
        tagType: core_proxy.TagType.custom,
      );
      createdTagIds[draft.draftId] = createdId;
    }
    for (final draft in tagChanges.updated) {
      await promptTagManager.updatePromptTag(
        id: draft.tagId,
        name: draft.values.name,
        description: draft.values.description,
        promptContent: draft.values.promptContent,
        tagType: draft.tagType,
      );
    }
    for (final tagId in tagChanges.deletedTagIds) {
      await promptTagManager.deletePromptTag(id: tagId);
    }
    final deletedTagIds = tagChanges.deletedTagIds.toSet();
    final attachedTagIds = <String>[];
    for (final tagId in card.attachedTagIds) {
      final resolvedTagId = createdTagIds[tagId] ?? tagId;
      if (deletedTagIds.contains(tagId) ||
          deletedTagIds.contains(resolvedTagId) ||
          attachedTagIds.contains(resolvedTagId)) {
        continue;
      }
      attachedTagIds.add(resolvedTagId);
    }
    return _characterCardWith(card, attachedTagIds: attachedTagIds);
  }

  Future<void> _activateCard(core_proxy.CharacterCard card) async {
    await widget.clients.chatRuntimeHolderMain.switchActiveCharacterCardTarget(
      characterCardId: card.id,
    );
    _reload();
  }

  Future<void> _activateGroup(core_proxy.CharacterGroupCard group) async {
    await widget.clients.chatRuntimeHolderMain.switchActiveCharacterGroupTarget(
      characterGroupId: group.id,
    );
    _reload();
  }

  Future<void> _createGroup(_CharacterSettingsData data) async {
    final l10n = AppLocalizations.of(context)!;
    final now = DateTime.now().millisecondsSinceEpoch;
    final group = core_proxy.CharacterGroupCard(
      id: '',
      name: '',
      description: '',
      members: const <core_proxy.GroupMemberConfig>[],
      createdAt: now,
      updatedAt: now,
    );
    final result = await _CharacterGroupEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersCreateGroup,
      group: group,
      cards: data.cards,
      showItemActions: false,
    );
    if (result == null) {
      return;
    }
    final edited = switch (result) {
      _CharacterGroupEditorSave(:final group) => group,
      _CharacterGroupEditorCopyJson() || _CharacterGroupEditorDelete() => null,
    };
    if (edited == null) {
      return;
    }
    await widget.clients.preferencesCharacterGroupCardManager
        .createCharacterGroupCard(group: edited);
    _reload();
  }

  Future<void> _editGroup(
    core_proxy.CharacterGroupCard group,
    _CharacterSettingsData data,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final result = await _CharacterGroupEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersEditGroup,
      group: group,
      cards: data.cards,
      showItemActions: true,
    );
    if (result == null) {
      return;
    }
    switch (result) {
      case _CharacterGroupEditorSave(:final group):
        await widget.clients.preferencesCharacterGroupCardManager
            .updateCharacterGroupCard(group: group);
        _reload();
      case _CharacterGroupEditorCopyJson():
        await _copyCharacterGroupJson(group);
      case _CharacterGroupEditorDelete():
        await _deleteGroup(group);
    }
  }

  Future<void> _deleteGroup(core_proxy.CharacterGroupCard group) async {
    await widget.clients.preferencesCharacterGroupCardManager
        .deleteCharacterGroupCard(groupId: group.id);
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

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final horizontalPadding = 16.0;
    return FutureBuilder<_CharacterSettingsData>(
      future: _future,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
        }
        final data = snapshot.data;
        if (data == null) {
          return const M3LoadingPane();
        }
        return ListView(
          padding: EdgeInsets.fromLTRB(
            horizontalPadding,
            12,
            horizontalPadding,
            20,
          ),
          children: <Widget>[
            _SectionCard(
              title: l10n.settingsCharactersCardsSection,
              action: Wrap(
                spacing: 8,
                runSpacing: 4,
                alignment: WrapAlignment.end,
                children: <Widget>[
                  TextButton.icon(
                    onPressed: _chooseCharacterCardImport,
                    style: SettingsControlStyles.sectionTextButton(),
                    icon: const Icon(Icons.upload_file_outlined, size: 18),
                    label: Text(l10n.settingsCharactersImport),
                  ),
                  FilledButton.icon(
                    onPressed: () => _createCard(data),
                    style: SettingsControlStyles.sectionFilledButton(),
                    icon: const Icon(Icons.add, size: 18),
                    label: Text(l10n.create),
                  ),
                ],
              ),
              children: <Widget>[
                for (final card in data.cards)
                  _CharacterCardTile(
                    card: card,
                    tags: data.tags,
                    avatarUri: data.cardAvatarUris[card.id],
                    active: card.id == data.activeCardId,
                    onActivate: () => _activateCard(card),
                    onEdit: () => _editCard(card, data),
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
            _SectionCard(
              title: l10n.settingsCharactersGroupsSection,
              action: Wrap(
                spacing: 8,
                runSpacing: 4,
                alignment: WrapAlignment.end,
                children: <Widget>[
                  TextButton.icon(
                    onPressed: _importCharacterGroupJson,
                    style: SettingsControlStyles.sectionTextButton(),
                    icon: const Icon(Icons.upload_file_outlined, size: 18),
                    label: Text(l10n.settingsCharactersImportJson),
                  ),
                  FilledButton.icon(
                    onPressed: () => _createGroup(data),
                    style: SettingsControlStyles.sectionFilledButton(),
                    icon: const Icon(Icons.add, size: 18),
                    label: Text(l10n.create),
                  ),
                ],
              ),
              children: <Widget>[
                for (final group in data.groups)
                  _CharacterGroupTile(
                    group: group,
                    active: group.id == data.activeGroupId,
                    cards: data.cards,
                    avatarUri: data.groupAvatarUris[group.id],
                    onActivate: () => _activateGroup(group),
                    onEdit: () => _editGroup(group, data),
                  ),
              ],
            ),
            _ExpandableSectionCard(
              title: l10n.settingsAdvanced,
              children: <Widget>[
                _AdvancedSettingsGroup(
                  title: '共享记忆',
                  description: '配置可被多个角色卡挂载的共享记忆库。',
                  action: FilledButton.icon(
                    onPressed: _createSharedMemoryStore,
                    style: SettingsControlStyles.sectionFilledButton(),
                    icon: const Icon(Icons.add, size: 18),
                    label: Text(l10n.create),
                  ),
                  children: <Widget>[
                    for (final store in data.sharedMemoryStores)
                      _SharedMemoryStoreTile(
                        store: store,
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
                const SizedBox(height: 16),
                _AdvancedSettingsGroup(
                  title: '共享 TTS 配置',
                  description: '配置可被角色卡显式绑定的语音合成服务。',
                  action: FilledButton.icon(
                    onPressed: _ttsConfigController.create,
                    style: SettingsControlStyles.sectionFilledButton(),
                    icon: const Icon(Icons.add, size: 18),
                    label: Text(l10n.create),
                  ),
                  children: <Widget>[
                    TtsConfigManagementSection(
                      clients: widget.clients,
                      controller: _ttsConfigController,
                    ),
                  ],
                ),
              ],
            ),
          ],
        );
      },
    );
  }
}
