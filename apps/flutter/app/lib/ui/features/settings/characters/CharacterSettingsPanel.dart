// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../common/components/OperitDialog.dart';
import '../../../theme/OperitFormStyles.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';
import 'MemoryGraphScreen.dart';

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
    return _CharacterSettingsData(
      cards: await cardManager.getAllCharacterCards(),
      groups: await groupManager.getAllCharacterGroupCards(),
      sharedMemoryStores: await sharedMemoryManager.getAllSharedMemoryStores(),
      tags: await promptTagManager.getAllTags(),
      modelSummaries: await modelManager.getAllModelSummaries(),
      builtinToolOptions: toolNames
          .map((toolName) => _ToolAccessOption(key: toolName, title: toolName))
          .toList(growable: false),
      packageToolOptions: packageOptions,
      skillToolOptions: skillOptions,
      mcpToolOptions: mcpOptions,
      activeCardId: await cardManager.observeActiveCharacterCardIdSnapshot(),
      activeGroupId: await groupManager.observeActiveCharacterGroupIdSnapshot(),
      enableMemoryAutoUpdate: await apiPreferences
          .enableMemoryAutoUpdateFlowSnapshot(),
      disableUserPreferenceDescription: await apiPreferences
          .disableUserPreferenceDescriptionFlowSnapshot(),
    );
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
      builtinToolOptions: data.builtinToolOptions,
      packageToolOptions: data.packageToolOptions,
      skillToolOptions: data.skillToolOptions,
      mcpToolOptions: data.mcpToolOptions,
      tags: data.tags,
    );
    if (result == null) {
      return;
    }
    final edited = switch (result) {
      _CharacterCardEditorSave(:final card) => card,
      _CharacterCardEditorCopyJson() ||
      _CharacterCardEditorCopyTavernJson() ||
      _CharacterCardEditorDelete() => null,
    };
    if (edited == null) {
      return;
    }
    await widget.clients.preferencesCharacterCardManager.createCharacterCard(
      card: edited,
    );
    _reload();
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
      case _CharacterCardEditorSave(:final card):
        await widget.clients.preferencesCharacterCardManager
            .updateCharacterCard(card: card);
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

  Future<void> _createTag() async {
    final l10n = AppLocalizations.of(context)!;
    final edited = await _PromptTagEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersCreateTag,
    );
    if (edited == null) {
      return;
    }
    await widget.clients.preferencesPromptTagManager.createPromptTag(
      name: edited.name,
      description: edited.description,
      promptContent: edited.promptContent,
      tagType: 'CUSTOM',
    );
    _reload();
  }

  Future<void> _editTag(core_proxy.PromptTag tag) async {
    final l10n = AppLocalizations.of(context)!;
    final edited = await _PromptTagEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersEditTag,
      tag: tag,
    );
    if (edited == null) {
      return;
    }
    await widget.clients.preferencesPromptTagManager.updatePromptTag(
      id: tag.id,
      name: edited.name,
      description: edited.description,
      promptContent: edited.promptContent,
      tagType: tag.tagType,
    );
    _reload();
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
    if (confirmed != true) {
      return;
    }
    await widget.clients.preferencesPromptTagManager.deletePromptTag(
      id: tag.id,
    );
    _reload();
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

  Future<void> _toggleMemoryAutoUpdate(_CharacterSettingsData data) async {
    await widget.clients.preferencesApiPreferences.saveEnableMemoryAutoUpdate(
      isEnabled: !data.enableMemoryAutoUpdate,
    );
    _reload();
  }

  Future<void> _togglePreferenceDescription(_CharacterSettingsData data) async {
    await widget.clients.preferencesApiPreferences
        .saveDisableUserPreferenceDescription(
          isDisabled: !data.disableUserPreferenceDescription,
        );
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final horizontalPadding = 16.0;
    return FutureBuilder<_CharacterSettingsData>(
      future: _future,
      builder: (context, snapshot) {
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
              title: l10n.settingsCharactersTagsSection,
              action: FilledButton.icon(
                onPressed: _createTag,
                style: SettingsControlStyles.sectionFilledButton(),
                icon: const Icon(Icons.add, size: 18),
                label: Text(l10n.create),
              ),
              children: <Widget>[
                if (data.tags.isEmpty)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Text(l10n.settingsCharactersNoTags),
                  )
                else
                  for (final tag in data.tags)
                    _PromptTagTile(
                      tag: tag,
                      onEdit: () => _editTag(tag),
                      onDelete: () => _deleteTag(tag),
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
                    onActivate: () => _activateGroup(group),
                    onEdit: () => _editGroup(group, data),
                  ),
              ],
            ),
            _SectionCard(
              title: '记忆',
              action: FilledButton.icon(
                onPressed: _createSharedMemoryStore,
                style: SettingsControlStyles.sectionFilledButton(),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('共享记忆库'),
              ),
              children: <Widget>[
                _SwitchLine(
                  title: l10n.settingsCharactersMemoryAutoUpdate,
                  subtitle: l10n.settingsCharactersMemoryAutoUpdateDescription,
                  value: data.enableMemoryAutoUpdate,
                  onChanged: (_) => _toggleMemoryAutoUpdate(data),
                ),
                _SwitchLine(
                  title: l10n.settingsCharactersPreferenceDescription,
                  subtitle:
                      l10n.settingsCharactersPreferenceDescriptionSubtitle,
                  value: !data.disableUserPreferenceDescription,
                  onChanged: (_) => _togglePreferenceDescription(data),
                ),
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
          ],
        );
      },
    );
  }
}

class _CharacterSettingsData {
  const _CharacterSettingsData({
    required this.cards,
    required this.groups,
    required this.sharedMemoryStores,
    required this.tags,
    required this.modelSummaries,
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
  final List<core_proxy.SharedMemoryStore> sharedMemoryStores;
  final List<core_proxy.PromptTag> tags;
  final List<core_proxy.ProviderModelSummary> modelSummaries;
  final List<_ToolAccessOption> builtinToolOptions;
  final List<_ToolAccessOption> packageToolOptions;
  final List<_ToolAccessOption> skillToolOptions;
  final List<_ToolAccessOption> mcpToolOptions;
  final String? activeCardId;
  final String? activeGroupId;
  final bool enableMemoryAutoUpdate;
  final bool disableUserPreferenceDescription;
}

class _CharacterCardTile extends StatelessWidget {
  const _CharacterCardTile({
    required this.card,
    required this.tags,
    required this.active,
    required this.onActivate,
    required this.onEdit,
    required this.onEditUserMarkdown,
    required this.onOpenMemoryGraph,
  });

  final core_proxy.CharacterCard card;
  final List<core_proxy.PromptTag> tags;
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
      leading: Icon(active ? Icons.check_circle : Icons.face_outlined),
      title: Text(card.name),
      subtitle: Text(
        [
          if (card.description.trim().isNotEmpty) card.description.trim(),
          if (tagNames.isNotEmpty) tagNames.join(', '),
          card.chatModelBindingMode,
          '共享记忆 ${card.sharedMemoryMounts.length}',
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
          icon: Icons.description_outlined,
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

class _PromptTagTile extends StatelessWidget {
  const _PromptTagTile({
    required this.tag,
    required this.onEdit,
    required this.onDelete,
  });

  final core_proxy.PromptTag tag;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return _SettingsEntityTile(
      leading: const Icon(Icons.sell_outlined),
      title: Text(tag.name),
      subtitle: Text(
        [
          if (tag.description.trim().isNotEmpty) tag.description.trim(),
          _tagTypeText(tag.tagType),
        ].join(' · '),
      ),
      onTap: onEdit,
      actions: <Widget>[
        SettingsEntityIconButton(
          tooltip: l10n.delete,
          icon: Icons.delete_outline,
          onPressed: onDelete,
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
      leading: Icon(active ? Icons.check_circle : Icons.groups_outlined),
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
          icon: Icons.description_outlined,
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
  const _SharedMemoryStoreEditResult({
    required this.name,
  });

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
    Navigator.of(context).pop(
      _SharedMemoryStoreEditResult(
        name: _nameController.text.trim(),
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
            decoration: const InputDecoration(
              labelText: 'USER.md',
              alignLabelWithHint: true,
            ),
          ),
        ),
      ),
    );
  }
}

sealed class _CharacterCardEditorResult {
  const _CharacterCardEditorResult();
}

class _CharacterCardEditorSave extends _CharacterCardEditorResult {
  const _CharacterCardEditorSave(this.card);

  final core_proxy.CharacterCard card;
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
  late List<core_proxy.CharacterSharedMemoryMount> _sharedMemoryMounts;
  late List<String> _attachedTagIds;
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
    _sharedMemoryMounts = List<core_proxy.CharacterSharedMemoryMount>.from(
      card.sharedMemoryMounts,
    );
    _attachedTagIds = List<String>.from(card.attachedTagIds);
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
    final card = widget.card;
    Navigator.of(context).pop(
      _CharacterCardEditorSave(
        core_proxy.CharacterCard(
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
          sharedMemoryMounts: _normalizedSharedMemoryMounts(
            _sharedMemoryMounts,
          ),
          toolAccessConfig: normalizedToolAccessConfig,
          isDefault: card.isDefault,
          createdAt: card.createdAt,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
      ),
    );
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

  void _setSharedMemoryMount(
    core_proxy.SharedMemoryStore store, {
    required bool? readable,
    required bool? writable,
  }) {
    final next = <core_proxy.CharacterSharedMemoryMount>[
      for (final mount in _sharedMemoryMounts)
        if (mount.sharedMemoryId != store.id) mount,
    ];
    final current = _sharedMemoryMountById(_sharedMemoryMounts, store.id);
    final nextReadable = readable ?? current?.readable ?? false;
    final nextWritable = writable ?? current?.writable ?? false;
    if (nextReadable || nextWritable) {
      next.add(
        core_proxy.CharacterSharedMemoryMount(
          sharedMemoryId: store.id,
          readable: nextReadable,
          writable: nextWritable,
        ),
      );
    }
    setState(() {
      _sharedMemoryMounts = next;
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
        child: SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(24, 12, 24, 16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
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
              Theme(
                data: Theme.of(context).copyWith(
                  dividerColor: Colors.transparent,
                  dividerTheme: const DividerThemeData(
                    color: Colors.transparent,
                  ),
                ),
                child: ExpansionTile(
                  title: Text(
                    l10n.settingsAdvanced,
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  tilePadding: EdgeInsets.zero,
                  childrenPadding: const EdgeInsets.only(top: 8),
                  shape: const Border(),
                  collapsedShape: const Border(),
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
                    _CharacterTagPicker(
                      tags: widget.tags,
                      selectedTagIds: _attachedTagIds,
                      onChanged: (tagId, selected) {
                        setState(() {
                          if (selected) {
                            if (!_attachedTagIds.contains(tagId)) {
                              _attachedTagIds.add(tagId);
                            }
                          } else {
                            _attachedTagIds.remove(tagId);
                          }
                        });
                      },
                    ),
                    _DialogDropdown<String>(
                      label: l10n.settingsCharactersChatModelBindingMode,
                      value: _chatModelBindingMode,
                      items: <DropdownMenuItem<String>>[
                        DropdownMenuItem<String>(
                          value: _chatModelFollowGlobal,
                          child: Text(
                            l10n.settingsCharactersChatModelFollowGlobal,
                          ),
                        ),
                        DropdownMenuItem<String>(
                          value: _chatModelFixedConfig,
                          child: Text(
                            l10n.settingsCharactersChatModelFixedConfig,
                          ),
                        ),
                      ],
                      onChanged: (value) {
                        if (value == null) {
                          return;
                        }
                        setState(() {
                          _chatModelBindingMode = value;
                        });
                      },
                    ),
                    if (_chatModelBindingMode == _chatModelFixedConfig) ...[
                      _DialogDropdown<String>(
                        label: l10n.settingsCharactersChatModelConfig,
                        value: selectedModel?.modelId,
                        items: widget.modelSummaries
                            .map(
                              (summary) => DropdownMenuItem<String>(
                                value: summary.modelId,
                                child: Text(
                                  '${summary.providerName} · ${summary.modelId}',
                                ),
                              ),
                            )
                            .toList(growable: false),
                        onChanged: (value) {
                          setState(() {
                            _chatModelId = value;
                          });
                        },
                      ),
                    ],
                    _CharacterSharedMemoryMountPicker(
                      stores: widget.sharedMemoryStores,
                      mounts: _sharedMemoryMounts,
                      onReadableChanged: (store, value) =>
                          _setSharedMemoryMount(
                            store,
                            readable: value,
                            writable: null,
                          ),
                      onWritableChanged: (store, value) =>
                          _setSharedMemoryMount(
                            store,
                            readable: null,
                            writable: value,
                          ),
                    ),
                    _DialogDropdown<bool>(
                      label: l10n.settingsCharactersToolAccess,
                      value: _toolAccessConfig.enabled,
                      items: <DropdownMenuItem<bool>>[
                        DropdownMenuItem<bool>(
                          value: false,
                          child: Text(
                            l10n.settingsCharactersToolAccessFollowGlobal,
                          ),
                        ),
                        DropdownMenuItem<bool>(
                          value: true,
                          child: Text(l10n.settingsCharactersToolAccessCustom),
                        ),
                      ],
                      onChanged: (value) {
                        if (value == null) {
                          return;
                        }
                        setState(() {
                          _toolAccessConfig =
                              core_proxy.CharacterCardToolAccessConfig(
                                enabled: value,
                                allowedBuiltinTools:
                                    _toolAccessConfig.allowedBuiltinTools,
                                allowedPackages:
                                    _toolAccessConfig.allowedPackages,
                                allowedSkills: _toolAccessConfig.allowedSkills,
                                allowedMcpServers:
                                    _toolAccessConfig.allowedMcpServers,
                              );
                        });
                      },
                    ),
                    if (_toolAccessConfig.enabled)
                      _DialogToolAccessConfigureField(
                        label: l10n.settingsCharactersToolAccessConfigure,
                        valueText: toolAccessSummary,
                        onConfigure: _openToolAccessDialog,
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
    return ExpansionTile(
      title: Text(title),
      tilePadding: EdgeInsets.zero,
      childrenPadding: EdgeInsets.zero,
      children: <Widget>[
        if (options.isEmpty)
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: Text(emptyText),
          )
        else
          for (final option in options)
            CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              dense: true,
              visualDensity: VisualDensity.compact,
              title: Text(option.title),
              subtitle: option.subtitle.isEmpty ? null : Text(option.subtitle),
              value: selectedKeys.contains(option.key),
              onChanged: (selected) {
                if (selected == null) {
                  return;
                }
                onChanged(option.key, selected);
              },
            ),
      ],
    );
  }
}

class _CharacterTagPicker extends StatelessWidget {
  const _CharacterTagPicker({
    required this.tags,
    required this.selectedTagIds,
    required this.onChanged,
  });

  final List<core_proxy.PromptTag> tags;
  final List<String> selectedTagIds;
  final void Function(String tagId, bool selected) onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final selectedNames = _tagNamesFor(tags, selectedTagIds);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            l10n.settingsCharactersTags,
            style: const TextStyle(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 6),
          if (tags.isEmpty)
            Text(
              l10n.settingsCharactersNoTags,
              style: TextStyle(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            )
          else
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final tag in tags)
                  FilterChip(
                    selected: selectedTagIds.contains(tag.id),
                    onSelected: (selected) => onChanged(tag.id, selected),
                    label: Text(tag.name),
                  ),
              ],
            ),
          if (selectedNames.isNotEmpty) ...[
            const SizedBox(height: 6),
            Text(
              selectedNames.join(' · '),
              style: Theme.of(context).textTheme.bodySmall!.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _CharacterSharedMemoryMountPicker extends StatelessWidget {
  const _CharacterSharedMemoryMountPicker({
    required this.stores,
    required this.mounts,
    required this.onReadableChanged,
    required this.onWritableChanged,
  });

  final List<core_proxy.SharedMemoryStore> stores;
  final List<core_proxy.CharacterSharedMemoryMount> mounts;
  final void Function(core_proxy.SharedMemoryStore store, bool value)
  onReadableChanged;
  final void Function(core_proxy.SharedMemoryStore store, bool value)
  onWritableChanged;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text('共享记忆挂载', style: textTheme.titleSmall),
          const SizedBox(height: 6),
          if (stores.isEmpty)
            Text(
              '还没有共享记忆库',
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            )
          else
            Column(
              children: <Widget>[
                for (final store in stores)
                  _SharedMemoryMountRow(
                    store: store,
                    mount: _sharedMemoryMountById(mounts, store.id),
                    onReadableChanged: onReadableChanged,
                    onWritableChanged: onWritableChanged,
                  ),
              ],
            ),
        ],
      ),
    );
  }
}

class _SharedMemoryMountRow extends StatelessWidget {
  const _SharedMemoryMountRow({
    required this.store,
    required this.mount,
    required this.onReadableChanged,
    required this.onWritableChanged,
  });

  final core_proxy.SharedMemoryStore store;
  final core_proxy.CharacterSharedMemoryMount? mount;
  final void Function(core_proxy.SharedMemoryStore store, bool value)
  onReadableChanged;
  final void Function(core_proxy.SharedMemoryStore store, bool value)
  onWritableChanged;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Text(
              store.name,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.bodyMedium,
            ),
          ),
          const SizedBox(width: 8),
          FilterChip(
            selected: mount?.readable ?? false,
            onSelected: (value) => onReadableChanged(store, value),
            label: const Text('读取'),
          ),
          const SizedBox(width: 6),
          FilterChip(
            selected: mount?.writable ?? false,
            onSelected: (value) => onWritableChanged(store, value),
            label: const Text('写入'),
          ),
        ],
      ),
    );
  }
}

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

class _SwitchLine extends StatelessWidget {
  const _SwitchLine({
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
    return SwitchListTile(
      contentPadding: EdgeInsets.zero,
      dense: true,
      visualDensity: VisualDensity.compact,
      title: Text(title),
      subtitle: Text(
        subtitle,
        style: TextStyle(color: colorScheme.onSurfaceVariant),
      ),
      value: value,
      onChanged: onChanged,
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

String _tagTypeText(Object? tagType) {
  final value = '$tagType';
  final buffer = StringBuffer();
  for (final codeUnit in value.codeUnits) {
    final isUppercase = codeUnit >= 65 && codeUnit <= 90;
    final isUnderscore = codeUnit == 95;
    if (isUppercase || isUnderscore) {
      buffer.writeCharCode(codeUnit);
    } else if (buffer.isNotEmpty) {
      break;
    }
  }
  final text = buffer.toString();
  return text.isEmpty ? value : text;
}

const String _chatModelFollowGlobal = 'FOLLOW_GLOBAL';
const String _chatModelFixedConfig = 'FIXED_CONFIG';
const Set<String> _hiddenToolNames = <String>{
  'package_proxy',
  'proxy',
  'search',
};

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

core_proxy.CharacterSharedMemoryMount? _sharedMemoryMountById(
  List<core_proxy.CharacterSharedMemoryMount> mounts,
  String? id,
) {
  for (final mount in mounts) {
    if (mount.sharedMemoryId == id) {
      return mount;
    }
  }
  return null;
}

List<core_proxy.CharacterSharedMemoryMount> _normalizedSharedMemoryMounts(
  List<core_proxy.CharacterSharedMemoryMount> mounts,
) {
  final seen = <String>{};
  final normalized = <core_proxy.CharacterSharedMemoryMount>[];
  for (final mount in mounts) {
    final id = mount.sharedMemoryId.trim();
    if (id.isEmpty || !seen.add(id)) {
      continue;
    }
    if (!mount.readable && !mount.writable) {
      continue;
    }
    normalized.add(
      core_proxy.CharacterSharedMemoryMount(
        sharedMemoryId: id,
        readable: mount.readable,
        writable: mount.writable,
      ),
    );
  }
  return normalized;
}

String _characterOwnerKey(String characterCardId) {
  return 'character:$characterCardId';
}

String _sharedOwnerKey(String sharedMemoryId) {
  return 'shared:$sharedMemoryId';
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
    attachedTagIds: card.attachedTagIds,
    advancedCustomPrompt: card.advancedCustomPrompt,
    marks: card.marks,
    chatModelBindingMode: card.chatModelBindingMode,
    chatModelId: card.chatModelId,
    sharedMemoryMounts: _normalizedSharedMemoryMounts(
      card.sharedMemoryMounts,
    ),
    toolAccessConfig: card.toolAccessConfig,
    isDefault: isDefault ?? card.isDefault,
    createdAt: createdAt ?? card.createdAt,
    updatedAt: updatedAt ?? card.updatedAt,
  );
}
