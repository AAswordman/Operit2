// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';

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
  UserPreferencesManager get _userPreferencesManager =>
      UserPreferencesManager(clients: widget.clients);

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<_CharacterSettingsData> _load() async {
    final cardManager = widget.clients.preferencesCharacterCardManager;
    final groupManager = widget.clients.preferencesCharacterGroupCardManager;
    final userPreferencesManager = _userPreferencesManager;
    final apiPreferences = widget.clients.preferencesApiPreferences;
    final modelManager = widget.clients.preferencesModelConfigManager;
    final toolHandler = widget.clients.permissionsAiToolHandler;
    final packageManager = widget.clients.permissionsPackToolPackageManager;
    final skillRepository = widget.clients.skillRepository;
    final mcpLocalServer = widget.clients.mcpLocalServer;
    final promptTagManager = widget.clients.preferencesPromptTagManager;
    await cardManager.initializeIfNeeded();
    await groupManager.initializeIfNeeded();
    await userPreferencesManager.initializeIfNeeded(
      defaultProfileName: 'Default',
    );
    await modelManager.initializeIfNeeded();
    await toolHandler.registerDefaultTools();
    final profileIds = await userPreferencesManager.profileListFlowSnapshot();
    final profiles = <core_proxy.PreferenceProfile>[];
    for (final profileId in profileIds) {
      profiles.add(
        await userPreferencesManager.getProfile(profileId: profileId),
      );
    }
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
      preferenceProfiles: profiles,
      tags: await promptTagManager.getAllTags(),
      modelSummaries: await modelManager.getAllConfigSummaries(),
      builtinToolOptions: toolNames
          .map((toolName) => _ToolAccessOption(key: toolName, title: toolName))
          .toList(growable: false),
      packageToolOptions: packageOptions,
      skillToolOptions: skillOptions,
      mcpToolOptions: mcpOptions,
      activeCardId: await cardManager.observeActiveCharacterCardIdSnapshot(),
      activeGroupId: await groupManager.observeActiveCharacterGroupIdSnapshot(),
      activePreferenceProfileId: await userPreferencesManager
          .activeProfileIdFlowSnapshot(),
      categoryLocks: await userPreferencesManager
          .categoryLockStatusFlowSnapshot(),
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
      chatModelConfigId: null,
      chatModelIndex: 0,
      memoryProfileBindingMode: 'FOLLOW_GLOBAL',
      memoryProfileId: null,
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
    final edited = await _CharacterCardEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersCreateCard,
      card: card,
      modelSummaries: data.modelSummaries,
      preferenceProfiles: data.preferenceProfiles,
      builtinToolOptions: data.builtinToolOptions,
      packageToolOptions: data.packageToolOptions,
      skillToolOptions: data.skillToolOptions,
      mcpToolOptions: data.mcpToolOptions,
      tags: data.tags,
    );
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
    final edited = await _CharacterCardEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersEditCard,
      card: card,
      modelSummaries: data.modelSummaries,
      preferenceProfiles: data.preferenceProfiles,
      builtinToolOptions: data.builtinToolOptions,
      packageToolOptions: data.packageToolOptions,
      skillToolOptions: data.skillToolOptions,
      mcpToolOptions: data.mcpToolOptions,
      tags: data.tags,
    );
    if (edited == null) {
      return;
    }
    await widget.clients.preferencesCharacterCardManager.updateCharacterCard(
      card: edited,
    );
    _reload();
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
    final edited = await _CharacterGroupEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersCreateGroup,
      group: group,
      cards: data.cards,
    );
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
    final edited = await _CharacterGroupEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersEditGroup,
      group: group,
      cards: data.cards,
    );
    if (edited == null) {
      return;
    }
    await widget.clients.preferencesCharacterGroupCardManager
        .updateCharacterGroupCard(group: edited);
    _reload();
  }

  Future<void> _deleteGroup(core_proxy.CharacterGroupCard group) async {
    await widget.clients.preferencesCharacterGroupCardManager
        .deleteCharacterGroupCard(groupId: group.id);
    _reload();
  }

  Future<void> _createPreferenceProfile() async {
    final l10n = AppLocalizations.of(context)!;
    final edited = await _PreferenceProfileEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersCreatePreferenceProfile,
    );
    if (edited == null) {
      return;
    }
    final createdProfileId = await _userPreferencesManager.createProfile(
      name: edited.name,
      isDefault: false,
    );
    final created = await _userPreferencesManager.getProfile(
      profileId: createdProfileId,
    );
    await _userPreferencesManager.updateProfile(
      _preferenceProfileWith(
        created,
        birthDate: edited.birthDate,
        gender: edited.gender,
        personality: edited.personality,
        identity: edited.identity,
        occupation: edited.occupation,
        aiStyle: edited.aiStyle,
        isInitialized: true,
      ),
    );
    _reload();
  }

  Future<void> _editPreferenceProfile(
    core_proxy.PreferenceProfile profile,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    final edited = await _PreferenceProfileEditorDialog.show(
      context: context,
      title: l10n.settingsCharactersEditPreferenceProfile,
      profile: profile,
    );
    if (edited == null) {
      return;
    }
    await _userPreferencesManager.updateProfile(
      _preferenceProfileWith(
        profile,
        name: edited.name,
        birthDate: edited.birthDate,
        gender: edited.gender,
        personality: edited.personality,
        identity: edited.identity,
        occupation: edited.occupation,
        aiStyle: edited.aiStyle,
        isInitialized: true,
      ),
    );
    _reload();
  }

  Future<void> _activatePreferenceProfile(
    core_proxy.PreferenceProfile profile,
  ) async {
    await _userPreferencesManager.setActiveProfile(profileId: profile.id);
    _reload();
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

  Future<void> _togglePreferenceLock(String category, bool locked) async {
    await _userPreferencesManager.setCategoryLocked(
      category: category,
      locked: !locked,
    );
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final width = MediaQuery.sizeOf(context).width;
    final horizontalPadding = width < 520 ? 16.0 : 28.0;
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
            24,
            horizontalPadding,
            36,
          ),
          children: <Widget>[
            _SectionCard(
              title: l10n.settingsCharactersCardsSection,
              action: Wrap(
                spacing: 8,
                runSpacing: 8,
                alignment: WrapAlignment.end,
                children: <Widget>[
                  TextButton.icon(
                    onPressed: _importCharacterCardJson,
                    icon: const Icon(Icons.upload_file_outlined),
                    label: Text(l10n.settingsCharactersImportJson),
                  ),
                  TextButton.icon(
                    onPressed: _importTavernCharacterCardJson,
                    icon: const Icon(Icons.person_add_alt_1_outlined),
                    label: Text(l10n.settingsCharactersImportTavernJson),
                  ),
                  FilledButton.icon(
                    onPressed: () => _createCard(data),
                    icon: const Icon(Icons.add),
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
                    onExport: () => _copyCharacterCardJson(card),
                    onExportTavern: () => _copyCharacterCardTavernJson(card),
                    onDelete: card.isDefault ? null : () => _deleteCard(card),
                  ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsCharactersTagsSection,
              action: FilledButton.icon(
                onPressed: _createTag,
                icon: const Icon(Icons.add),
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
                runSpacing: 8,
                alignment: WrapAlignment.end,
                children: <Widget>[
                  TextButton.icon(
                    onPressed: _importCharacterGroupJson,
                    icon: const Icon(Icons.upload_file_outlined),
                    label: Text(l10n.settingsCharactersImportJson),
                  ),
                  FilledButton.icon(
                    onPressed: () => _createGroup(data),
                    icon: const Icon(Icons.add),
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
                    onExport: () => _copyCharacterGroupJson(group),
                    onDelete: () => _deleteGroup(group),
                  ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsCharactersPreferenceProfilesSection,
              action: FilledButton.icon(
                onPressed: _createPreferenceProfile,
                icon: const Icon(Icons.add),
                label: Text(l10n.create),
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
                for (final profile in data.preferenceProfiles)
                  _PreferenceProfileTile(
                    profile: profile,
                    active: profile.id == data.activePreferenceProfileId,
                    onActivate: () => _activatePreferenceProfile(profile),
                    onEdit: () => _editPreferenceProfile(profile),
                  ),
              ],
            ),
            _SectionCard(
              title: l10n.settingsCharactersPreferenceLocksSection,
              children: <Widget>[
                for (final entry in _preferenceLockLabels(l10n).entries)
                  _SwitchLine(
                    title: entry.value,
                    subtitle: l10n.settingsCharactersPreferenceLockDescription,
                    value: data.categoryLocks[entry.key] == true,
                    onChanged: (_) => _togglePreferenceLock(
                      entry.key,
                      data.categoryLocks[entry.key] == true,
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
    required this.preferenceProfiles,
    required this.tags,
    required this.modelSummaries,
    required this.builtinToolOptions,
    required this.packageToolOptions,
    required this.skillToolOptions,
    required this.mcpToolOptions,
    required this.activeCardId,
    required this.activeGroupId,
    required this.activePreferenceProfileId,
    required this.categoryLocks,
    required this.enableMemoryAutoUpdate,
    required this.disableUserPreferenceDescription,
  });

  final List<core_proxy.CharacterCard> cards;
  final List<core_proxy.CharacterGroupCard> groups;
  final List<core_proxy.PreferenceProfile> preferenceProfiles;
  final List<core_proxy.PromptTag> tags;
  final List<core_proxy.ModelConfigSummary> modelSummaries;
  final List<_ToolAccessOption> builtinToolOptions;
  final List<_ToolAccessOption> packageToolOptions;
  final List<_ToolAccessOption> skillToolOptions;
  final List<_ToolAccessOption> mcpToolOptions;
  final String? activeCardId;
  final String? activeGroupId;
  final String activePreferenceProfileId;
  final Map<String, bool> categoryLocks;
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
    required this.onExport,
    required this.onExportTavern,
    required this.onDelete,
  });

  final core_proxy.CharacterCard card;
  final List<core_proxy.PromptTag> tags;
  final bool active;
  final VoidCallback onActivate;
  final VoidCallback onEdit;
  final VoidCallback onExport;
  final VoidCallback onExportTavern;
  final VoidCallback? onDelete;

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
          card.memoryProfileBindingMode,
        ].join(' · '),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: active ? null : onActivate,
          child: Text(active ? l10n.settingsActive : l10n.settingsActivate),
        ),
        IconButton(
          tooltip: l10n.edit,
          onPressed: onEdit,
          icon: const Icon(Icons.edit_outlined),
        ),
        PopupMenuButton<_CharacterCardExportAction>(
          tooltip: l10n.settingsCharactersCopyJson,
          icon: const Icon(Icons.copy_outlined),
          onSelected: (action) {
            switch (action) {
              case _CharacterCardExportAction.nativeJson:
                onExport();
                break;
              case _CharacterCardExportAction.tavernJson:
                onExportTavern();
                break;
            }
          },
          itemBuilder: (context) =>
              <PopupMenuEntry<_CharacterCardExportAction>>[
                PopupMenuItem<_CharacterCardExportAction>(
                  value: _CharacterCardExportAction.nativeJson,
                  child: Text(l10n.settingsCharactersCopyJson),
                ),
                PopupMenuItem<_CharacterCardExportAction>(
                  value: _CharacterCardExportAction.tavernJson,
                  child: Text(l10n.settingsCharactersCopyTavernJson),
                ),
              ],
        ),
        IconButton(
          tooltip: l10n.delete,
          onPressed: onDelete,
          icon: const Icon(Icons.delete_outline),
        ),
      ],
    );
  }
}

enum _CharacterCardExportAction { nativeJson, tavernJson }

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
      actions: <Widget>[
        IconButton(
          tooltip: l10n.edit,
          onPressed: onEdit,
          icon: const Icon(Icons.edit_outlined),
        ),
        IconButton(
          tooltip: l10n.delete,
          onPressed: onDelete,
          icon: const Icon(Icons.delete_outline),
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
    required this.onExport,
    required this.onDelete,
  });

  final core_proxy.CharacterGroupCard group;
  final bool active;
  final List<core_proxy.CharacterCard> cards;
  final VoidCallback onActivate;
  final VoidCallback onEdit;
  final VoidCallback onExport;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final memberNames = group.members
        .map((member) => _cardNameFor(cards, member.characterCardId))
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
      actions: <Widget>[
        TextButton(
          onPressed: active ? null : onActivate,
          child: Text(active ? l10n.settingsActive : l10n.settingsActivate),
        ),
        IconButton(
          tooltip: l10n.edit,
          onPressed: onEdit,
          icon: const Icon(Icons.edit_outlined),
        ),
        IconButton(
          tooltip: l10n.settingsCharactersCopyJson,
          onPressed: onExport,
          icon: const Icon(Icons.copy_outlined),
        ),
        IconButton(
          tooltip: l10n.delete,
          onPressed: onDelete,
          icon: const Icon(Icons.delete_outline),
        ),
      ],
    );
  }
}

class _PreferenceProfileTile extends StatelessWidget {
  const _PreferenceProfileTile({
    required this.profile,
    required this.active,
    required this.onActivate,
    required this.onEdit,
  });

  final core_proxy.PreferenceProfile profile;
  final bool active;
  final VoidCallback onActivate;
  final VoidCallback onEdit;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return _SettingsEntityTile(
      leading: Icon(active ? Icons.check_circle : Icons.psychology_outlined),
      title: Text(profile.name),
      subtitle: Text(
        [
          if (profile.identity.trim().isNotEmpty) profile.identity.trim(),
          if (profile.personality.trim().isNotEmpty) profile.personality.trim(),
          if (profile.aiStyle.trim().isNotEmpty) profile.aiStyle.trim(),
        ].join(' · '),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: active ? null : onActivate,
          child: Text(active ? l10n.settingsActive : l10n.settingsActivate),
        ),
        IconButton(
          tooltip: l10n.edit,
          onPressed: onEdit,
          icon: const Icon(Icons.edit_outlined),
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
  });

  final Widget leading;
  final Widget title;
  final Widget subtitle;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              SizedBox(
                width: 48,
                child: Padding(
                  padding: const EdgeInsets.only(top: 2),
                  child: IconTheme.merge(
                    data: IconThemeData(color: colorScheme.onSurfaceVariant),
                    child: leading,
                  ),
                ),
              ),
              Expanded(
                child: DefaultTextStyle.merge(
                  style: TextStyle(color: colorScheme.onSurface),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      DefaultTextStyle.merge(
                        style: const TextStyle(
                          fontWeight: FontWeight.w700,
                          fontSize: 16,
                        ),
                        child: title,
                      ),
                      const SizedBox(height: 4),
                      DefaultTextStyle.merge(
                        style: TextStyle(
                          color: colorScheme.onSurfaceVariant,
                          height: 1.35,
                        ),
                        child: subtitle,
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          if (actions.isNotEmpty) ...[
            const SizedBox(height: 8),
            Padding(
              padding: const EdgeInsets.only(left: 48),
              child: Align(
                alignment: Alignment.centerRight,
                child: Wrap(
                  spacing: 4,
                  runSpacing: 4,
                  alignment: WrapAlignment.end,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: actions,
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _PreferenceProfileEditResult {
  const _PreferenceProfileEditResult({
    required this.name,
    required this.birthDate,
    required this.gender,
    required this.personality,
    required this.identity,
    required this.occupation,
    required this.aiStyle,
  });

  final String name;
  final int birthDate;
  final String gender;
  final String personality;
  final String identity;
  final String occupation;
  final String aiStyle;
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

class _PreferenceProfileEditorDialog extends StatefulWidget {
  const _PreferenceProfileEditorDialog({required this.title, this.profile});

  final String title;
  final core_proxy.PreferenceProfile? profile;

  static Future<_PreferenceProfileEditResult?> show({
    required BuildContext context,
    required String title,
    core_proxy.PreferenceProfile? profile,
  }) {
    return showDialog<_PreferenceProfileEditResult>(
      context: context,
      builder: (context) =>
          _PreferenceProfileEditorDialog(title: title, profile: profile),
    );
  }

  @override
  State<_PreferenceProfileEditorDialog> createState() =>
      _PreferenceProfileEditorDialogState();
}

class _PreferenceProfileEditorDialogState
    extends State<_PreferenceProfileEditorDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameController;
  late final TextEditingController _birthDateController;
  late final TextEditingController _genderController;
  late final TextEditingController _personalityController;
  late final TextEditingController _identityController;
  late final TextEditingController _occupationController;
  late final TextEditingController _aiStyleController;

  @override
  void initState() {
    super.initState();
    final profile = widget.profile;
    _nameController = TextEditingController(text: profile?.name ?? '');
    _birthDateController = TextEditingController(
      text: (profile?.birthDate ?? 0).toString(),
    );
    _genderController = TextEditingController(text: profile?.gender ?? '');
    _personalityController = TextEditingController(
      text: profile?.personality ?? '',
    );
    _identityController = TextEditingController(text: profile?.identity ?? '');
    _occupationController = TextEditingController(
      text: profile?.occupation ?? '',
    );
    _aiStyleController = TextEditingController(text: profile?.aiStyle ?? '');
  }

  @override
  void dispose() {
    _nameController.dispose();
    _birthDateController.dispose();
    _genderController.dispose();
    _personalityController.dispose();
    _identityController.dispose();
    _occupationController.dispose();
    _aiStyleController.dispose();
    super.dispose();
  }

  void _save() {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    Navigator.of(context).pop(
      _PreferenceProfileEditResult(
        name: _nameController.text.trim(),
        birthDate: int.parse(_birthDateController.text.trim()),
        gender: _genderController.text.trim(),
        personality: _personalityController.text.trim(),
        identity: _identityController.text.trim(),
        occupation: _occupationController.text.trim(),
        aiStyle: _aiStyleController.text.trim(),
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
                  label: l10n.settingsCharactersPreferenceProfileName,
                  requiredField: true,
                ),
                _DialogTextField(
                  controller: _birthDateController,
                  label: l10n.settingsCharactersPreferenceBirthDate,
                  numberOnly: true,
                ),
                _DialogTextField(
                  controller: _genderController,
                  label: l10n.settingsCharactersPreferenceGender,
                ),
                _DialogTextField(
                  controller: _personalityController,
                  label: l10n.settingsCharactersPreferencePersonality,
                  maxLines: 3,
                ),
                _DialogTextField(
                  controller: _identityController,
                  label: l10n.settingsCharactersPreferenceIdentity,
                  maxLines: 3,
                ),
                _DialogTextField(
                  controller: _occupationController,
                  label: l10n.settingsCharactersPreferenceOccupation,
                ),
                _DialogTextField(
                  controller: _aiStyleController,
                  label: l10n.settingsCharactersPreferenceAiStyle,
                  maxLines: 3,
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

class _CharacterCardEditorDialog extends StatefulWidget {
  const _CharacterCardEditorDialog({
    required this.title,
    required this.card,
    required this.modelSummaries,
    required this.preferenceProfiles,
    required this.builtinToolOptions,
    required this.packageToolOptions,
    required this.skillToolOptions,
    required this.mcpToolOptions,
    required this.tags,
  });

  final String title;
  final core_proxy.CharacterCard card;
  final List<core_proxy.ModelConfigSummary> modelSummaries;
  final List<core_proxy.PreferenceProfile> preferenceProfiles;
  final List<_ToolAccessOption> builtinToolOptions;
  final List<_ToolAccessOption> packageToolOptions;
  final List<_ToolAccessOption> skillToolOptions;
  final List<_ToolAccessOption> mcpToolOptions;
  final List<core_proxy.PromptTag> tags;

  static Future<core_proxy.CharacterCard?> show({
    required BuildContext context,
    required String title,
    required core_proxy.CharacterCard card,
    required List<core_proxy.ModelConfigSummary> modelSummaries,
    required List<core_proxy.PreferenceProfile> preferenceProfiles,
    required List<_ToolAccessOption> builtinToolOptions,
    required List<_ToolAccessOption> packageToolOptions,
    required List<_ToolAccessOption> skillToolOptions,
    required List<_ToolAccessOption> mcpToolOptions,
    required List<core_proxy.PromptTag> tags,
  }) {
    return showDialog<core_proxy.CharacterCard>(
      context: context,
      builder: (context) => _CharacterCardEditorDialog(
        title: title,
        card: card,
        modelSummaries: modelSummaries,
        preferenceProfiles: preferenceProfiles,
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
  String? _chatModelConfigId;
  late int _chatModelIndex;
  late String _memoryProfileBindingMode;
  String? _memoryProfileId;
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
    _chatModelConfigId = card.chatModelConfigId;
    _chatModelIndex = card.chatModelIndex;
    _memoryProfileBindingMode = _normalizeMemoryProfileBindingMode(
      card.memoryProfileBindingMode,
    );
    _memoryProfileId = card.memoryProfileId;
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
    final selectedConfig = _modelSummaryById(
      widget.modelSummaries,
      _chatModelConfigId,
    );
    final selectedModelNames = selectedConfig == null
        ? const <String>[]
        : _modelNames(selectedConfig.modelName);
    final normalizedModelIndex =
        _chatModelBindingMode == _chatModelFixedConfig &&
            selectedModelNames.isNotEmpty
        ? _chatModelIndex.clamp(0, selectedModelNames.length - 1)
        : 0;
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
        chatModelConfigId: _chatModelBindingMode == _chatModelFixedConfig
            ? _chatModelConfigId
            : null,
        chatModelIndex: normalizedModelIndex,
        memoryProfileBindingMode: _memoryProfileBindingMode,
        memoryProfileId: _memoryProfileBindingMode == _memoryFixedProfile
            ? _memoryProfileId
            : null,
        toolAccessConfig: normalizedToolAccessConfig,
        isDefault: card.isDefault,
        createdAt: card.createdAt,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
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

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final selectedConfig = _modelSummaryById(
      widget.modelSummaries,
      _chatModelConfigId,
    );
    final modelNames = selectedConfig == null
        ? const <String>[]
        : _modelNames(selectedConfig.modelName);
    final modelIndexValue =
        _chatModelIndex >= 0 && _chatModelIndex < modelNames.length
        ? _chatModelIndex
        : null;
    final memoryProfileValue =
        _preferenceProfileById(widget.preferenceProfiles, _memoryProfileId) ==
            null
        ? null
        : _memoryProfileId;
    final toolAccessSummary = _toolAccessSummary(l10n, _toolAccessConfig);
    return AlertDialog(
      title: Text(widget.title),
      content: SizedBox(
        width: 680,
        child: Form(
          key: _formKey,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
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
                _DialogTextField(
                  controller: _characterSettingController,
                  label: l10n.settingsCharactersCharacterSetting,
                  maxLines: 6,
                ),
                _DialogTextField(
                  controller: _openingStatementController,
                  label: l10n.settingsCharactersOpeningStatement,
                  maxLines: 3,
                ),
                ExpansionTile(
                  title: Text(l10n.settingsAdvanced),
                  tilePadding: EdgeInsets.zero,
                  childrenPadding: EdgeInsets.zero,
                  children: <Widget>[
                    _DialogTextField(
                      controller: _otherContentChatController,
                      label: l10n.settingsCharactersOtherContentChat,
                      maxLines: 4,
                    ),
                    _DialogTextField(
                      controller: _otherContentVoiceController,
                      label: l10n.settingsCharactersOtherContentVoice,
                      maxLines: 4,
                    ),
                    _DialogTextField(
                      controller: _advancedPromptController,
                      label: l10n.settingsCharactersAdvancedPrompt,
                      maxLines: 4,
                    ),
                    _DialogTextField(
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
                        value: selectedConfig?.id,
                        items: widget.modelSummaries
                            .map(
                              (summary) => DropdownMenuItem<String>(
                                value: summary.id,
                                child: Text(summary.name),
                              ),
                            )
                            .toList(growable: false),
                        onChanged: (value) {
                          setState(() {
                            _chatModelConfigId = value;
                            _chatModelIndex = 0;
                          });
                        },
                      ),
                      _DialogDropdown<int>(
                        label: l10n.settingsCharactersChatModelIndex,
                        value: modelIndexValue,
                        items: <DropdownMenuItem<int>>[
                          for (
                            var index = 0;
                            index < modelNames.length;
                            index++
                          )
                            DropdownMenuItem<int>(
                              value: index,
                              child: Text(modelNames[index]),
                            ),
                        ],
                        onChanged: (value) {
                          if (value == null) {
                            return;
                          }
                          setState(() {
                            _chatModelIndex = value;
                          });
                        },
                      ),
                    ],
                    _DialogDropdown<String>(
                      label: l10n.settingsCharactersMemoryBindingMode,
                      value: _memoryProfileBindingMode,
                      items: <DropdownMenuItem<String>>[
                        DropdownMenuItem<String>(
                          value: _memoryFollowGlobal,
                          child: Text(
                            l10n.settingsCharactersMemoryProfileFollowGlobal,
                          ),
                        ),
                        DropdownMenuItem<String>(
                          value: _memoryFixedProfile,
                          child: Text(
                            l10n.settingsCharactersMemoryProfileFixedProfile,
                          ),
                        ),
                      ],
                      onChanged: (value) {
                        if (value == null) {
                          return;
                        }
                        setState(() {
                          _memoryProfileBindingMode = value;
                        });
                      },
                    ),
                    if (_memoryProfileBindingMode == _memoryFixedProfile)
                      _DialogDropdown<String>(
                        label: l10n.settingsCharactersMemoryProfile,
                        value: memoryProfileValue,
                        items: widget.preferenceProfiles
                            .map(
                              (profile) => DropdownMenuItem<String>(
                                value: profile.id,
                                child: Text(profile.name),
                              ),
                            )
                            .toList(growable: false),
                        onChanged: (value) {
                          setState(() {
                            _memoryProfileId = value;
                          });
                        },
                      ),
                    SwitchListTile(
                      contentPadding: EdgeInsets.zero,
                      title: Text(l10n.settingsCharactersToolAccess),
                      subtitle: Text(toolAccessSummary),
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
                                allowedSkills: _toolAccessConfig.allowedSkills,
                                allowedMcpServers:
                                    _toolAccessConfig.allowedMcpServers,
                              );
                        });
                      },
                    ),
                    ListTile(
                      contentPadding: EdgeInsets.zero,
                      title: Text(l10n.settingsCharactersToolAccessConfigure),
                      subtitle: Text(toolAccessSummary),
                      trailing: const Icon(Icons.tune_outlined),
                      onTap: _openToolAccessDialog,
                    ),
                  ],
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
              style: TextStyle(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
                fontSize: 12,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _CharacterGroupEditorDialog extends StatefulWidget {
  const _CharacterGroupEditorDialog({
    required this.title,
    required this.group,
    required this.cards,
  });

  final String title;
  final core_proxy.CharacterGroupCard group;
  final List<core_proxy.CharacterCard> cards;

  static Future<core_proxy.CharacterGroupCard?> show({
    required BuildContext context,
    required String title,
    required core_proxy.CharacterGroupCard group,
    required List<core_proxy.CharacterCard> cards,
  }) {
    return showDialog<core_proxy.CharacterGroupCard>(
      context: context,
      builder: (context) =>
          _CharacterGroupEditorDialog(title: title, group: group, cards: cards),
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
      core_proxy.CharacterGroupCard(
        id: widget.group.id,
        name: _nameController.text.trim(),
        description: _descriptionController.text.trim(),
        members: members,
        createdAt: widget.group.createdAt,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
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
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _save, child: Text(l10n.save)),
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
    this.numberOnly = false,
    this.maxLines = 1,
  });

  final TextEditingController controller;
  final String label;
  final bool requiredField;
  final bool numberOnly;
  final int maxLines;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: TextFormField(
        controller: controller,
        maxLines: maxLines,
        keyboardType: numberOnly ? TextInputType.number : TextInputType.text,
        inputFormatters: numberOnly
            ? <TextInputFormatter>[FilteringTextInputFormatter.digitsOnly]
            : null,
        decoration: InputDecoration(labelText: label),
        validator: (value) {
          final text = value?.trim() ?? '';
          if (requiredField && text.isEmpty) {
            return label;
          }
          if (numberOnly && text.isEmpty) {
            return label;
          }
          return null;
        },
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
        decoration: InputDecoration(labelText: label),
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
    final radius = BorderRadius.circular(18);
    return Padding(
      padding: const EdgeInsets.only(bottom: 14),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(18, 16, 18, 14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(title, style: const TextStyle(fontWeight: FontWeight.w800)),
              if (action != null) ...[
                const SizedBox(height: 10),
                Align(alignment: Alignment.centerRight, child: action!),
              ],
              const SizedBox(height: 8),
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

String _cardNameFor(List<core_proxy.CharacterCard> cards, String id) {
  for (final card in cards) {
    if (card.id == id) {
      return card.name;
    }
  }
  return _shortIdentifier(id);
}

String _shortIdentifier(String value) {
  final text = value.trim();
  if (text.length <= 12) {
    return text;
  }
  return '${text.substring(0, 8)}...${text.substring(text.length - 4)}';
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
  final match = RegExp(r'[A-Z_]+').firstMatch(value);
  return match?.group(0) ?? value;
}

const String _chatModelFollowGlobal = 'FOLLOW_GLOBAL';
const String _chatModelFixedConfig = 'FIXED_CONFIG';
const String _memoryFollowGlobal = 'FOLLOW_GLOBAL';
const String _memoryFixedProfile = 'FIXED_PROFILE';
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

String _normalizeMemoryProfileBindingMode(String mode) {
  return mode == _memoryFixedProfile
      ? _memoryFixedProfile
      : _memoryFollowGlobal;
}

List<String> _modelNames(String raw) {
  return raw
      .split(',')
      .map((item) => item.trim())
      .where((item) => item.isNotEmpty)
      .toList(growable: false);
}

core_proxy.ModelConfigSummary? _modelSummaryById(
  List<core_proxy.ModelConfigSummary> summaries,
  String? id,
) {
  for (final summary in summaries) {
    if (summary.id == id) {
      return summary;
    }
  }
  return null;
}

core_proxy.PreferenceProfile? _preferenceProfileById(
  List<core_proxy.PreferenceProfile> profiles,
  String? id,
) {
  for (final profile in profiles) {
    if (profile.id == id) {
      return profile;
    }
  }
  return null;
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

Map<String, String> _preferenceLockLabels(AppLocalizations l10n) {
  return <String, String>{
    'birthDate': l10n.settingsCharactersPreferenceBirthDate,
    'gender': l10n.settingsCharactersPreferenceGender,
    'personality': l10n.settingsCharactersPreferencePersonality,
    'identity': l10n.settingsCharactersPreferenceIdentity,
    'occupation': l10n.settingsCharactersPreferenceOccupation,
    'aiStyle': l10n.settingsCharactersPreferenceAiStyle,
  };
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
    chatModelConfigId: card.chatModelConfigId,
    chatModelIndex: card.chatModelIndex,
    memoryProfileBindingMode: card.memoryProfileBindingMode,
    memoryProfileId: card.memoryProfileId,
    toolAccessConfig: card.toolAccessConfig,
    isDefault: isDefault ?? card.isDefault,
    createdAt: createdAt ?? card.createdAt,
    updatedAt: updatedAt ?? card.updatedAt,
  );
}

core_proxy.PreferenceProfile _preferenceProfileWith(
  core_proxy.PreferenceProfile profile, {
  String? name,
  int? birthDate,
  String? gender,
  String? personality,
  String? identity,
  String? occupation,
  String? aiStyle,
  bool? isInitialized,
}) {
  return core_proxy.PreferenceProfile(
    id: profile.id,
    name: name ?? profile.name,
    birthDate: birthDate ?? profile.birthDate,
    gender: gender ?? profile.gender,
    personality: personality ?? profile.personality,
    identity: identity ?? profile.identity,
    occupation: occupation ?? profile.occupation,
    aiStyle: aiStyle ?? profile.aiStyle,
    isInitialized: isInitialized ?? profile.isInitialized,
  );
}
