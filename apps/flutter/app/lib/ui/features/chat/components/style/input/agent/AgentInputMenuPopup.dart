// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../../../../core/application/PluginHotReload.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../common/CharacterAvatar.dart';
import '../../../../../../common/contributions/ChatUiContributionModels.dart';
import '../../../../../../common/contributions/ContributionPresentationResult.dart';
import '../../../../../../common/contributions/ToolPkgChatUiCatalog.dart';
import '../../../../../../common/icons/MaterialIconNameResolver.dart';
import '../../../../../../main/navigation/ToolPkgCatalogChangeBus.dart';
import '../../../../../packages/screens/ToolPkgUiLauncherScreen.dart';
import '../../../../viewmodel/ChatSelectionTransition.dart';
import '../../../../viewmodel/ChatViewModel.dart';

class AgentInputMenuPopup extends StatefulWidget {
  /// Creates the shared menu with a lifecycle guard owned by its real composer.
  const AgentInputMenuPopup({
    super.key,
    required this.viewModel,
    required this.currentChatId,
    required this.onDismiss,
    required this.isChatContextCurrent,
    this.leadingChildren = const <Widget>[],
  });

  final ChatViewModel viewModel;
  final String? currentChatId;
  final VoidCallback onDismiss;
  final bool Function() isChatContextCurrent;
  final List<Widget> leadingChildren;

  @override
  State<AgentInputMenuPopup> createState() => _AgentInputMenuPopupState();
}

class _AgentInputMenuPopupState extends State<AgentInputMenuPopup> {
  Future<_AgentInputMenuData>? _settingsFuture;
  Timer? _pluginChangeTimer;
  String? _settingsSignature;
  bool _checkingPluginChangeVersion = false;
  bool _toolsExpanded = false;
  bool _behaviorExpanded = false;
  bool _pluginsExpanded = false;
  Future<List<core_proxy.ToolPkgNavigationEntry>>? _pluginEntriesFuture;
  StreamSubscription<void>? _pluginCatalogSubscription;
  String? _pluginLanguageCode;
  bool _openingPluginEntry = false;

  /// Starts menu observers and listens to mutations of the existing UI catalog.
  @override
  void initState() {
    super.initState();
    _settingsFuture = _loadSettings();
    _startPluginChangeObserver();
    _pluginCatalogSubscription = ToolPkgCatalogChangeBus.listen(
      _reloadPluginEntries,
    );
    PluginHotReload.revision.addListener(_reloadPluginEntries);
  }

  /// Reloads localized plugin navigation entries when the host locale changes.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final languageCode = Localizations.localeOf(context).languageCode;
    if (_pluginLanguageCode != languageCode) {
      _pluginLanguageCode = languageCode;
      _pluginEntriesFuture = _loadPluginEntries(languageCode);
    }
  }

  /// Keeps menu data scoped to the current chat and runtime client.
  @override
  void didUpdateWidget(covariant AgentInputMenuPopup oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentChatId != widget.currentChatId ||
        oldWidget.viewModel != widget.viewModel) {
      _settingsSignature = null;
      _settingsFuture = _loadSettings();
      if (oldWidget.viewModel != widget.viewModel) {
        _reloadPluginEntries();
      }
    }
  }

  /// Releases both chat observers and plugin catalog listeners with the popup.
  @override
  void dispose() {
    _pluginChangeTimer?.cancel();
    unawaited(_pluginCatalogSubscription?.cancel());
    PluginHotReload.revision.removeListener(_reloadPluginEntries);
    super.dispose();
  }

  /// Reads toolbox entries from the actual registry without selecting a package.
  Future<List<core_proxy.ToolPkgNavigationEntry>> _loadPluginEntries(
    String languageCode,
  ) async {
    final entries = await widget.viewModel.clients.application
        .packageManager()
        .getToolPkgNavigationEntries(useEnglish: languageCode == 'en');
    return List<core_proxy.ToolPkgNavigationEntry>.unmodifiable(
      entries.where((entry) => entry.surface == 'toolbox'),
    );
  }

  /// Refreshes registered menu actions after package or hot-reload changes.
  void _reloadPluginEntries() {
    if (!mounted) {
      return;
    }
    final languageCode = _pluginLanguageCode;
    if (languageCode == null) {
      return;
    }
    setState(() {
      _pluginEntriesFuture = _loadPluginEntries(languageCode);
    });
  }

  /// Invokes the declared action or opens the exact catalog-owned Compose route.
  Future<void> _openPluginEntry(core_proxy.ToolPkgNavigationEntry entry) async {
    if (_openingPluginEntry) {
      return;
    }
    final clients = widget.viewModel.clients;
    final viewModel = widget.viewModel;
    final chatId = widget.currentChatId;
    final navigator = Navigator.of(context, rootNavigator: true);
    final messenger = ScaffoldMessenger.of(context);
    final languageCode = Localizations.localeOf(context).languageCode;
    setState(() => _openingPluginEntry = true);
    try {
      final manager = clients.application.packageManager();
      final action = entry.action;
      if (action != null) {
        await manager.runToolPkgNavigationEntryAction(
          containerPackageName: entry.containerPackageName,
          entryId: entry.entryId,
          functionName: action.functionName,
          inlineFunctionSource: action.functionSource,
          eventPayload: <String, Object?>{
            'entryId': entry.entryId,
            'routeId': entry.routeId,
            'surface': entry.surface,
            'title': entry.title,
            'description': entry.description,
            'chatId': chatId,
          },
        );
        if (mounted &&
            widget.viewModel == viewModel &&
            widget.currentChatId == chatId) {
          widget.onDismiss();
        }
        return;
      }
      final routes = await manager.getToolPkgUiRoutes(
        runtime: 'compose_dsl',
        useEnglish: languageCode == 'en',
      );
      final matchingRoutes = routes.where(
        (route) =>
            route.containerPackageName == entry.containerPackageName &&
            route.routeId == entry.routeId,
      );
      if (matchingRoutes.length != 1) {
        throw StateError(
          'A menu entry must resolve to exactly one registered Compose route: '
          '${entry.containerPackageName}/${entry.routeId}',
        );
      }
      final route = matchingRoutes.single;
      final plugin = await manager.getToolPkgContainerRuntime(
        containerPackageName: route.containerPackageName,
      );
      if (plugin == null) {
        throw StateError(
          'Menu route container is not registered: ${route.containerPackageName}',
        );
      }
      if (!mounted ||
          widget.viewModel != viewModel ||
          widget.currentChatId != chatId ||
          !navigator.mounted) {
        return;
      }
      widget.onDismiss();
      await navigator.push<void>(
        MaterialPageRoute<void>(
          builder: (context) => ToolPkgUiLauncherScreen(
            clients: clients,
            plugin: plugin,
            initialRouteId: route.routeId,
            initialModuleSpec: route.moduleSpec,
          ),
        ),
      );
    } catch (error) {
      if (messenger.mounted) {
        messenger.showSnackBar(SnackBar(content: Text('插件入口打开失败：$error')));
      }
    } finally {
      if (mounted) {
        setState(() => _openingPluginEntry = false);
      }
    }
  }

  /// Renders registered actions with their localized title and Material icon.
  Widget _buildPluginEntries() {
    return FutureBuilder<List<core_proxy.ToolPkgNavigationEntry>>(
      future: _pluginEntriesFuture,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return Padding(
            padding: const EdgeInsets.all(8),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Text('插件入口加载失败：${snapshot.error}'),
                TextButton(
                  onPressed: _reloadPluginEntries,
                  child: const Text('重试'),
                ),
              ],
            ),
          );
        }
        final entries = snapshot.data;
        if (entries == null) {
          return const Padding(
            padding: EdgeInsets.all(12),
            child: LinearProgressIndicator(),
          );
        }
        return Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            for (final entry in entries)
              ListTile(
                dense: true,
                enabled: !_openingPluginEntry,
                leading: entry.icon == null
                    ? null
                    : Icon(
                        MaterialIconNameResolver.resolve(entry.icon!),
                        size: 20,
                      ),
                title: Text(entry.title),
                subtitle: entry.description.isEmpty
                    ? null
                    : Text(
                        entry.description,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                      ),
                trailing: const Icon(Icons.open_in_new, size: 16),
                onTap: () => _openPluginEntry(entry),
              ),
          ],
        );
      },
    );
  }

  void _startPluginChangeObserver() {
    _pluginChangeTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      _checkPluginChangeVersion();
    });
  }

  /// Refreshes native settings and registered toggle metadata for this chat.
  Future<void> _checkPluginChangeVersion() async {
    if (_checkingPluginChangeVersion) {
      return;
    }
    _checkingPluginChangeVersion = true;
    try {
      final chatId = widget.currentChatId;
      final viewModel = widget.viewModel;
      final settings = await viewModel.chatCore.chatInputMenuSettings(
        chatId: chatId,
      );
      if (!mounted ||
          widget.currentChatId != chatId ||
          widget.viewModel != viewModel) {
        return;
      }
      final signature = jsonEncode(_menuData(settings).toJson());
      if (_settingsSignature != signature) {
        _settingsSignature = signature;
        setState(() {
          _settingsFuture = Future.value(_menuData(settings));
        });
      }
    } catch (error) {
      debugPrint('Chat input menu refresh: $error');
    } finally {
      _checkingPluginChangeVersion = false;
    }
  }

  /// Reads native composer settings without consuming retired plugin-domain flags.
  Future<_AgentInputMenuData> _loadSettings() async {
    final settings = await widget.viewModel.chatCore.chatInputMenuSettings(
      chatId: widget.currentChatId,
    );
    return _menuData(settings);
  }

  /// Selects only host-owned behavior and actual registered plugin contributions.
  _AgentInputMenuData _menuData(core_proxy.ChatInputMenuSettings settings) {
    return _AgentInputMenuData(
      permissionMode: settings.permissionMode,
      disableStreamOutput: settings.disableStreamOutput,
      pluginToggles: settings.pluginToggles,
    );
  }

  /// Updates only native fields; null fields remain unchanged under the Core contract.
  Future<void> _saveSettings({
    core_proxy.AiPermissionMode? permissionMode,
    bool? disableStreamOutput,
  }) => widget.viewModel.chatCore.saveChatInputMenuSettings(
    chatId: widget.currentChatId,
    enableMemoryAutoUpdate: null,
    permissionMode: permissionMode,
    disableStreamOutput: disableStreamOutput,
    disableUserPreferenceDescription: null,
  );

  void _reloadSettings() {
    setState(() {
      _settingsFuture = _loadSettings();
    });
  }

  Future<void> _setPermissionMode(_ToolPermissionMode mode) async {
    await _saveSettings(permissionMode: mode.permissionMode);
    _reloadSettings();
  }

  Future<void> _toggleDisableStreamOutput(_AgentInputMenuData data) async {
    await _saveSettings(disableStreamOutput: !data.disableStreamOutput);
    _reloadSettings();
  }

  Future<void> _togglePlugin(
    core_proxy.InputMenuToggleDefinitionSnapshot toggle,
  ) async {
    await widget.viewModel.chatCore.triggerChatInputMenuToggle(
      toggleId: toggle.id,
      chatId: widget.currentChatId,
    );
    _reloadSettings();
  }

  /// Builds the input menu popup with optional leading entries.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: Colors.transparent,
      child: Card(
        margin: EdgeInsets.zero,
        color: colorScheme.surfaceContainer,
        elevation: 4,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 300, maxHeight: 420),
          child: FutureBuilder<_AgentInputMenuData>(
            future: _settingsFuture,
            builder: (context, snapshot) {
              if (snapshot.hasError) {
                return Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text('菜单加载失败：${snapshot.error}'),
                      TextButton(
                        onPressed: _reloadSettings,
                        child: const Text('重试'),
                      ),
                    ],
                  ),
                );
              }
              final data = snapshot.data;
              if (data == null) {
                return const SizedBox(
                  width: 300,
                  height: 96,
                  child: Center(child: CircularProgressIndicator()),
                );
              }
              return SingleChildScrollView(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    _ChatSessionSummarySection(
                      viewModel: widget.viewModel,
                      currentChatId: widget.currentChatId,
                      onDismiss: widget.onDismiss,
                      isChatContextCurrent: widget.isChatContextCurrent,
                    ),
                    const Divider(height: 1),
                    ...widget.leadingChildren,
                    _MenuSection(
                      icon: Icons.security_outlined,
                      title: '工具',
                      value: data.toolPermissionMode.label,
                      expanded: _toolsExpanded,
                      onTap: () {
                        setState(() {
                          _toolsExpanded = !_toolsExpanded;
                        });
                      },
                      children: <Widget>[
                        Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 12,
                            vertical: 3,
                          ),
                          child: _PermissionModeSelector(
                            selectedMode: data.toolPermissionMode,
                            onSelected: _setPermissionMode,
                          ),
                        ),
                      ],
                    ),
                    _MenuSection(
                      icon: Icons.bolt_outlined,
                      title: '行为',
                      value: data.disableStreamOutput ? '非流式' : '流式',
                      expanded: _behaviorExpanded,
                      onTap: () {
                        setState(() {
                          _behaviorExpanded = !_behaviorExpanded;
                        });
                      },
                      children: <Widget>[
                        _SwitchRow(
                          icon: Icons.speed_outlined,
                          title: '流式输出',
                          value: data.disableStreamOutput ? '关' : '开',
                          checked: !data.disableStreamOutput,
                          onTap: () => _toggleDisableStreamOutput(data),
                        ),
                      ],
                    ),
                    _MenuSection(
                      icon: Icons.extension_outlined,
                      title: '插件',
                      value: data.pluginSummary,
                      expanded: _pluginsExpanded,
                      onTap: () {
                        setState(() {
                          _pluginsExpanded = !_pluginsExpanded;
                        });
                      },
                      children: <Widget>[
                        for (final toggle in data.pluginToggles)
                          _SwitchRow(
                            icon: Icons.hub,
                            materialIconName: toggle.icon,
                            title: toggle.title ?? toggle.id,
                            value: toggle.isChecked ? '开' : '关',
                            checked: toggle.isChecked,
                            enabled: toggle.isEnabled,
                            onTap: () => _togglePlugin(toggle),
                          ),
                        _buildPluginEntries(),
                      ],
                    ),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

class _ChatSessionSummarySection extends StatefulWidget {
  /// Retains the real chat owner while the surrounding overlay is dismissed.
  const _ChatSessionSummarySection({
    required this.viewModel,
    required this.currentChatId,
    required this.onDismiss,
    required this.isChatContextCurrent,
  });

  final ChatViewModel viewModel;
  final String? currentChatId;
  final VoidCallback onDismiss;
  final bool Function() isChatContextCurrent;

  /// Creates the state for the current chat summary section.
  @override
  State<_ChatSessionSummarySection> createState() =>
      _ChatSessionSummarySectionState();
}

class _ChatSessionSummarySectionState
    extends State<_ChatSessionSummarySection> {
  Timer? _summaryTimer;
  bool _pollingSummary = false;
  Future<double>? _maxContextLengthFuture;
  int _currentWindowSize = 0;
  int _inputTokenCount = 0;
  int _outputTokenCount = 0;
  bool _statsExpanded = false;
  Future<ChatUiContext>? _contextActionsFuture;
  StreamSubscription<void>? _contextCatalogSubscription;
  bool _presentingContextAction = false;

  /// Loads plugin actions and native statistics for this exact menu context.
  @override
  void initState() {
    super.initState();
    _maxContextLengthFuture = _loadSummary();
    _contextActionsFuture = _loadChatUiContext();
    _contextCatalogSubscription = ToolPkgCatalogChangeBus.listen(
      _reloadChatUiContext,
    );
    PluginHotReload.revision.addListener(_reloadChatUiContext);
    _summaryTimer = Timer.periodic(const Duration(seconds: 1), (_) async {
      if (_pollingSummary) return;
      _pollingSummary = true;
      try {
        final chatId = widget.currentChatId;
        final viewModel = widget.viewModel;
        final value = await _loadSummary();
        if (mounted &&
            widget.currentChatId == chatId &&
            widget.viewModel == viewModel) {
          setState(() {
            _maxContextLengthFuture = Future.value(value);
          });
        }
      } catch (error) {
        debugPrint('Chat menu summary refresh: $error');
      } finally {
        _pollingSummary = false;
      }
    });
  }

  /// Replaces context futures when the actual chat or runtime owner changes.
  @override
  void didUpdateWidget(covariant _ChatSessionSummarySection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentChatId != widget.currentChatId ||
        oldWidget.viewModel != widget.viewModel) {
      _currentWindowSize = 0;
      _inputTokenCount = 0;
      _outputTokenCount = 0;
      _maxContextLengthFuture = _loadSummary();
      _contextActionsFuture = _loadChatUiContext();
    }
  }

  /// Loads presentation actions from the actual enabled public API owners.
  Future<ChatUiContext> _loadChatUiContext() => ToolPkgChatUiCatalog(
    clients: widget.viewModel.clients,
  ).loadContext(chatId: widget.currentChatId);

  /// Refreshes context after a durable plugin action or package catalog change.
  void _reloadChatUiContext() {
    if (!mounted) return;
    setState(() => _contextActionsFuture = _loadChatUiContext());
  }

  /// Refreshes the captured runtime descriptor only after its own valid completion.
  Future<void> _presentChatUiAction(ChatUiAction action) async {
    if (_presentingContextAction || !widget.isChatContextCurrent()) return;
    final viewModel = widget.viewModel;
    final clients = viewModel.clients;
    final chatCore = viewModel.chatCore;
    final chatId = widget.currentChatId;
    final ownerIsCurrent = widget.isChatContextCurrent;
    final dismissMenu = widget.onDismiss;
    final presentationContext = Navigator.of(
      context,
      rootNavigator: true,
    ).overlay!.context;
    final messenger = ScaffoldMessenger.of(context);
    final initialChatId = Completer<String?>();
    StreamSubscription<String?>? selectionSubscription;
    Object? selectionError;
    StackTrace? selectionStackTrace;
    var selectionChanged = ChatSelectionTransition.requests.value != null;
    setState(() => _presentingContextAction = true);

    /// Expires this invocation when any pending chat selection starts.
    void onSelectionTransition() {
      if (ChatSelectionTransition.requests.value != null) {
        selectionChanged = true;
      }
    }

    /// Validates the real composer, root host and every observed selection event.
    bool ownsCurrentContext() =>
        !selectionChanged && ownerIsCurrent() && presentationContext.mounted;

    /// Surfaces broken selection observation instead of publishing stale metadata.
    void requireSelectionObservation() {
      final error = selectionError;
      if (error != null) {
        Error.throwWithStackTrace(error, selectionStackTrace!);
      }
    }

    ChatSelectionTransition.requests.addListener(onSelectionTransition);
    try {
      selectionSubscription = viewModel.watchCurrentChatId().listen(
        (currentChatId) {
          if (currentChatId != chatId) selectionChanged = true;
          if (!initialChatId.isCompleted) {
            initialChatId.complete(currentChatId);
          }
        },
        onError: (Object error, StackTrace stackTrace) {
          selectionError = error;
          selectionStackTrace = stackTrace;
          if (!initialChatId.isCompleted) {
            initialChatId.completeError(error, stackTrace);
          }
        },
        onDone: () {
          final error = StateError('The chat selection observer closed.');
          selectionError = error;
          selectionStackTrace = StackTrace.current;
          if (!initialChatId.isCompleted) {
            initialChatId.completeError(error, selectionStackTrace);
          }
        },
      );
      dismissMenu();
      await initialChatId.future;
      requireSelectionObservation();
      if (!ownsCurrentContext() || !presentationContext.mounted) return;
      final result = await action.present(
        context: presentationContext,
        clients: clients,
        hostState: <String, Object?>{'chatId': chatId},
      );
      if (result?.status != ContributionPresentationStatus.completed ||
          !ownsCurrentContext()) {
        return;
      }
      requireSelectionObservation();
      final currentChatId = await chatCore.currentChatIdFlow().first;
      if (currentChatId != chatId || !ownsCurrentContext()) return;
      requireSelectionObservation();
      // A new-session active choice has no existing runtime descriptor to publish.
      if (chatId != null) {
        await chatCore.chatConfiguration(chatId: chatId);
      }
      if (!ownsCurrentContext()) return;
      requireSelectionObservation();
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
    } catch (error) {
      if (ownsCurrentContext() && messenger.mounted) {
        messenger.showSnackBar(SnackBar(content: Text('插件操作失败：$error')));
      }
    } finally {
      ChatSelectionTransition.requests.removeListener(onSelectionTransition);
      await selectionSubscription?.cancel();
      if (mounted) setState(() => _presentingContextAction = false);
    }
  }

  /// Renders only the display fields and actions supplied by registered owners.
  Widget _buildContextActions() => FutureBuilder<ChatUiContext>(
    future: _contextActionsFuture,
    builder: (context, snapshot) {
      if (snapshot.connectionState != ConnectionState.done) {
        return const Padding(
          padding: EdgeInsets.all(12),
          child: LinearProgressIndicator(),
        );
      }
      if (snapshot.hasError) {
        return Padding(
          padding: const EdgeInsets.all(12),
          child: Text('会话插件加载失败：${snapshot.error}'),
        );
      }
      final data = snapshot.requireData;
      final identity = data.identity;
      return Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          if (identity != null)
            ListTile(
              dense: true,
              leading: SizedBox(
                width: 32,
                height: 32,
                child: ClipOval(
                  child: CharacterAvatarImage(
                    avatarUri: identity.avatarUri,
                    fit: BoxFit.cover,
                  ),
                ),
              ),
              title: Text(
                identity.title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              enabled: !_presentingContextAction,
              onTap: () => _presentChatUiAction(identity.action),
            ),
          for (final selector in data.selectors)
            ListTile(
              dense: true,
              leading: selector.icon == null
                  ? null
                  : Icon(
                      MaterialIconNameResolver.resolve(selector.icon!),
                      size: 20,
                    ),
              title: Text(selector.title),
              trailing: const Icon(Icons.chevron_right, size: 20),
              enabled: !_presentingContextAction,
              onTap: () => _presentChatUiAction(selector.action),
            ),
        ],
      );
    },
  );

  /// Reads native token statistics without consulting any plugin domain records.
  Future<double> _loadSummary() async {
    final chatId = widget.currentChatId;
    final viewModel = widget.viewModel;
    final summary = await viewModel.chatCore.chatInputMenuSummary(
      chatId: chatId,
    );
    if (mounted &&
        widget.currentChatId == chatId &&
        widget.viewModel == viewModel) {
      setState(() {
        _currentWindowSize = summary.currentWindowSize;
        _inputTokenCount = summary.inputTokenCount;
        _outputTokenCount = summary.outputTokenCount;
      });
    }
    return summary.maxContextLength;
  }

  /// Stops overlay observers without cancelling the already detached presentation.
  @override
  void dispose() {
    _summaryTimer?.cancel();
    unawaited(_contextCatalogSubscription?.cancel());
    PluginHotReload.revision.removeListener(_reloadChatUiContext);
    super.dispose();
  }

  /// Builds registered chat context actions and native token statistics.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        _buildContextActions(),
        _buildNativeStatistics(colorScheme, textTheme),
      ],
    );
  }

  /// Reports native statistics failures separately from plugin-owned selectors.
  Widget _buildNativeStatistics(ColorScheme colorScheme, TextTheme textTheme) =>
      FutureBuilder<double>(
        future: _maxContextLengthFuture,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Padding(
              padding: const EdgeInsets.all(12),
              child: Text('统计加载失败：${snapshot.error}'),
            );
          }
          final maxContextLength = snapshot.data;
          final maxContextTokens = maxContextLength == null
              ? null
              : (maxContextLength * 1024).round();
          final contextUsagePercentage = maxContextTokens == null
              ? null
              : _contextUsagePercentage(maxContextTokens);
          final totalTokenCount = _inputTokenCount + _outputTokenCount;
          return Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              InkWell(
                onTap: () {
                  setState(() {
                    _statsExpanded = !_statsExpanded;
                  });
                },
                child: ConstrainedBox(
                  constraints: const BoxConstraints(minHeight: 40),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 12),
                    child: Row(
                      children: <Widget>[
                        Icon(
                          Icons.data_usage_outlined,
                          size: 17,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(width: 12),
                        Text('统计', style: textTheme.bodySmall),
                        const Spacer(),
                        Text(
                          contextUsagePercentage == null
                              ? '加载中...'
                              : '${contextUsagePercentage.toStringAsFixed(0)}%',
                          style: textTheme.bodySmall?.copyWith(
                            color: colorScheme.primary,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        const SizedBox(width: 6),
                        Icon(
                          _statsExpanded
                              ? Icons.keyboard_arrow_up
                              : Icons.keyboard_arrow_down,
                          size: 20,
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
              if (_statsExpanded)
                ColoredBox(
                  color: colorScheme.surface.withValues(alpha: 0.42),
                  child: Padding(
                    padding: const EdgeInsets.fromLTRB(40, 6, 12, 8),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        _ChatStatValueRow(
                          label: '上下文窗口',
                          value: _contextWindowLabel(
                            currentWindowSize: _currentWindowSize,
                            maxContextTokens: maxContextTokens,
                          ),
                        ),
                        _ChatStatValueRow(
                          label: '输入 Token',
                          value: _formatTokenCount(_inputTokenCount),
                        ),
                        _ChatStatValueRow(
                          label: '输出 Token',
                          value: _formatTokenCount(_outputTokenCount),
                        ),
                        _ChatStatValueRow(
                          label: '总 Token',
                          value: _formatTokenCount(totalTokenCount),
                          highlighted: true,
                        ),
                      ],
                    ),
                  ),
                ),
            ],
          );
        },
      );

  /// Calculates the current context usage percentage for the summary row.
  double _contextUsagePercentage(int maxContextTokens) {
    if (maxContextTokens <= 0) {
      return 0;
    }
    return (_currentWindowSize / maxContextTokens * 100).clamp(0, 999);
  }

  /// Formats the context window values used in the expanded statistics.
  String _contextWindowLabel({
    required int currentWindowSize,
    required int? maxContextTokens,
  }) {
    final maxTokens = maxContextTokens;
    if (maxTokens == null) {
      return '加载中...';
    }
    return '${_formatTokenCount(currentWindowSize)} / ${_formatTokenCount(maxTokens)}';
  }

  /// Formats token counts with compact thousands separators.
  String _formatTokenCount(int value) {
    return value.toString().replaceAllMapped(
      RegExp(r'(?<=\d)(?=(\d{3})+$)'),
      (match) => ',',
    );
  }
}

class _ChatStatValueRow extends StatelessWidget {
  const _ChatStatValueRow({
    required this.label,
    required this.value,
    this.highlighted = false,
  });

  final String label;
  final String value;
  final bool highlighted;

  /// Builds one value row in the expanded chat statistics section.
  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(label, style: textTheme.labelSmall)),
          Text(
            value,
            style: textTheme.labelSmall?.copyWith(
              color: highlighted
                  ? colorScheme.primary
                  : colorScheme.onSurfaceVariant,
              fontWeight: highlighted ? FontWeight.w700 : FontWeight.normal,
            ),
          ),
        ],
      ),
    );
  }
}

class _AgentInputMenuData {
  /// Creates menu state without carrying plugin-owned preference flags.
  const _AgentInputMenuData({
    required this.permissionMode,
    required this.disableStreamOutput,
    required this.pluginToggles,
  });

  final core_proxy.AiPermissionMode permissionMode;
  final bool disableStreamOutput;
  final List<core_proxy.InputMenuToggleDefinitionSnapshot> pluginToggles;

  /// Encodes only native composer settings and registered plugin toggle metadata.
  Map<String, Object?> toJson() => <String, Object?>{
    'permissionMode': permissionMode.toJson(),
    'disableStreamOutput': disableStreamOutput,
    'pluginToggles': pluginToggles
        .map((toggle) => toggle.toJson())
        .toList(growable: false),
  };

  _ToolPermissionMode get toolPermissionMode {
    return switch (permissionMode) {
      core_proxy.AiPermissionMode.readOnly => _ToolPermissionMode.readOnly,
      core_proxy.AiPermissionMode.workspaceWrite =>
        _ToolPermissionMode.workspaceWrite,
      core_proxy.AiPermissionMode.full => _ToolPermissionMode.full,
    };
  }

  String get pluginSummary {
    final enabledCount = pluginToggles
        .where((toggle) => toggle.isChecked)
        .length;
    return '$enabledCount/${pluginToggles.length}';
  }
}

enum _ToolPermissionMode {
  readOnly('只读', core_proxy.AiPermissionMode.readOnly),
  workspaceWrite('读写', core_proxy.AiPermissionMode.workspaceWrite),
  full('完整', core_proxy.AiPermissionMode.full);

  const _ToolPermissionMode(this.label, this.permissionMode);

  final String label;
  final core_proxy.AiPermissionMode permissionMode;
}

class _MenuSection extends StatelessWidget {
  const _MenuSection({
    required this.icon,
    required this.title,
    required this.value,
    required this.expanded,
    required this.onTap,
    required this.children,
  });

  final IconData icon;
  final String title;
  final String value;
  final bool expanded;
  final VoidCallback onTap;
  final List<Widget> children;

  /// Builds a compact group without nesting another card inside the menu.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        InkWell(
          onTap: onTap,
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 36),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Row(
                children: <Widget>[
                  Icon(
                    icon,
                    size: 16,
                    color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                  ),
                  const SizedBox(width: 12),
                  Text(title, style: textTheme.bodySmall),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      value,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.end,
                      style: textTheme.bodySmall!.copyWith(
                        color: colorScheme.primary,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 6),
                  Icon(
                    expanded
                        ? Icons.keyboard_arrow_up
                        : Icons.keyboard_arrow_down,
                    size: 20,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ),
        ),
        if (expanded)
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 0, 8, 4),
            child: Column(mainAxisSize: MainAxisSize.min, children: children),
          ),
      ],
    );
  }
}

class _SwitchRow extends StatelessWidget {
  const _SwitchRow({
    required this.icon,
    this.materialIconName,
    required this.title,
    required this.value,
    required this.checked,
    this.enabled = true,
    required this.onTap,
  });

  final IconData icon;
  final String? materialIconName;
  final String title;
  final String value;
  final bool checked;
  final bool enabled;
  final VoidCallback onTap;

  /// Builds a compact switch row with a layout-sized switch control.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final iconColor = !enabled
        ? colorScheme.onSurfaceVariant.withValues(alpha: 0.45)
        : checked
        ? colorScheme.primary
        : colorScheme.onSurfaceVariant;
    final iconName = materialIconName?.trim();
    final resolvedIcon = iconName == null || iconName.isEmpty
        ? icon
        : MaterialIconNameResolver.resolveOrNull(iconName);
    if (resolvedIcon == null) {
      throw ArgumentError.value(
        materialIconName,
        'materialIconName',
        'Unknown Material icon name',
      );
    }
    return InkWell(
      onTap: enabled ? onTap : null,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 32),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              Icon(resolvedIcon, size: 16, color: iconColor),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall!.copyWith(
                    color: enabled
                        ? colorScheme.onSurface
                        : colorScheme.onSurfaceVariant.withValues(alpha: 0.65),
                  ),
                ),
              ),
              Text(
                value,
                style: textTheme.bodySmall!.copyWith(
                  color: enabled
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant.withValues(alpha: 0.65),
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(width: 8),
              SizedBox(
                width: 40,
                height: 28,
                child: FittedBox(
                  child: Switch(
                    value: checked,
                    onChanged: enabled ? (_) => onTap() : null,
                    materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _PermissionModeSelector extends StatelessWidget {
  const _PermissionModeSelector({
    required this.selectedMode,
    required this.onSelected,
  });

  final _ToolPermissionMode selectedMode;
  final ValueChanged<_ToolPermissionMode> onSelected;

  /// Builds the compact three-way permission mode selector.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Row(
      children: <Widget>[
        for (final mode in _ToolPermissionMode.values) ...[
          Expanded(
            child: InkWell(
              borderRadius: BorderRadius.circular(6),
              onTap: () => onSelected(mode),
              child: Container(
                height: 30,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: mode == selectedMode
                      ? colorScheme.primaryContainer
                      : Colors.transparent,
                  border: Border.all(
                    color: mode == selectedMode
                        ? colorScheme.primary
                        : colorScheme.outline.withValues(alpha: 0.35),
                  ),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  mode.label,
                  style: textTheme.bodySmall!.copyWith(
                    color: mode == selectedMode
                        ? colorScheme.onPrimaryContainer
                        : colorScheme.onSurface,
                    fontWeight: mode == selectedMode
                        ? FontWeight.w600
                        : FontWeight.normal,
                  ),
                ),
              ),
            ),
          ),
          if (mode != _ToolPermissionMode.values.last) const SizedBox(width: 6),
        ],
      ],
    );
  }
}
