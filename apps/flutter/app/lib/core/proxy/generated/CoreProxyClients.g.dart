// GENERATED CODE - DO NOT MODIFY BY HAND

import 'dart:async';
import 'dart:typed_data';

import '../../bridge/OperitRuntimeBridge.dart';
import '../../link/CoreLinkProtocol.dart';
import 'CoreProxyModels.g.dart';

String _coreProxyRequestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';

class GeneratedCoreProxyClients {
  const GeneratedCoreProxyClients(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `application`.
  GeneratedApplicationCoreProxy get application => GeneratedApplicationCoreProxy(bridge, CoreObjectPath.parse('application'));
  /// Returns a generated proxy client for `chatRuntimeHolder.main`.
  GeneratedChatRuntimeHolderMainCoreProxy get chatRuntimeHolderMain => GeneratedChatRuntimeHolderMainCoreProxy(bridge, CoreObjectPath.parse('chatRuntimeHolder.main'));
  GeneratedChatRuntimeHolderMainCoreProxy get chatRuntimeHolderFloating => GeneratedChatRuntimeHolderMainCoreProxy(bridge, CoreObjectPath.parse('chatRuntimeHolder.floating'));
  /// Returns a generated proxy client for `linkAccessStore`.
  GeneratedLinkAccessStoreCoreProxy get linkAccessStore => GeneratedLinkAccessStoreCoreProxy(bridge, CoreObjectPath.parse('linkAccessStore'));
  /// Returns a generated proxy client for `permissions.mcp.mCPManager`.
  GeneratedPermissionsMcpManagerCoreProxy get permissionsMcpManager => GeneratedPermissionsMcpManagerCoreProxy(bridge, CoreObjectPath.parse('permissions.mcp.mCPManager'));
  /// Returns a generated proxy client for `permissions.mcp_runtime.mCPLocalServer`.
  GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy get permissionsMcpRuntimeMcpLocalServer => GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy(bridge, CoreObjectPath.parse('permissions.mcp_runtime.mCPLocalServer'));
  /// Returns a generated proxy client for `permissions.mcp_runtime.plugins.mCPBridge`.
  GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy get permissionsMcpRuntimePluginsMcpBridge => GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy(bridge, CoreObjectPath.parse('permissions.mcp_runtime.plugins.mCPBridge'));
  /// Returns a generated proxy client for `permissions.toolPermissionSystem`.
  GeneratedPermissionsToolPermissionSystemCoreProxy get permissionsToolPermissionSystem => GeneratedPermissionsToolPermissionSystemCoreProxy(bridge, CoreObjectPath.parse('permissions.toolPermissionSystem'));
  /// Returns a generated proxy client for `preferences.activePromptManager`.
  GeneratedPreferencesActivePromptManagerCoreProxy get preferencesActivePromptManager => GeneratedPreferencesActivePromptManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.activePromptManager'));
  /// Returns a generated proxy client for `preferences.apiPreferences`.
  GeneratedPreferencesApiPreferencesCoreProxy get preferencesApiPreferences => GeneratedPreferencesApiPreferencesCoreProxy(bridge, CoreObjectPath.parse('preferences.apiPreferences'));
  /// Returns a generated proxy client for `preferences.characterCardManager`.
  GeneratedPreferencesCharacterCardManagerCoreProxy get preferencesCharacterCardManager => GeneratedPreferencesCharacterCardManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.characterCardManager'));
  /// Returns a generated proxy client for `preferences.characterCardToolAccessResolver`.
  GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy get preferencesCharacterCardToolAccessResolver => GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy(bridge, CoreObjectPath.parse('preferences.characterCardToolAccessResolver'));
  /// Returns a generated proxy client for `preferences.characterGroupCardManager`.
  GeneratedPreferencesCharacterGroupCardManagerCoreProxy get preferencesCharacterGroupCardManager => GeneratedPreferencesCharacterGroupCardManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.characterGroupCardManager'));
  /// Returns a generated proxy client for `preferences.envPreferences`.
  GeneratedPreferencesEnvPreferencesCoreProxy get preferencesEnvPreferences => GeneratedPreferencesEnvPreferencesCoreProxy(bridge, CoreObjectPath.parse('preferences.envPreferences'));
  /// Returns a generated proxy client for `preferences.functionalConfigManager`.
  GeneratedPreferencesFunctionalConfigManagerCoreProxy get preferencesFunctionalConfigManager => GeneratedPreferencesFunctionalConfigManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.functionalConfigManager'));
  /// Returns a generated proxy client for `preferences.gitHubAuthPreferences`.
  GeneratedPreferencesGitHubAuthPreferencesCoreProxy get preferencesGitHubAuthPreferences => GeneratedPreferencesGitHubAuthPreferencesCoreProxy(bridge, CoreObjectPath.parse('preferences.gitHubAuthPreferences'));
  /// Returns a generated proxy client for `preferences.modelConfigManager`.
  GeneratedPreferencesModelConfigManagerCoreProxy get preferencesModelConfigManager => GeneratedPreferencesModelConfigManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.modelConfigManager'));
  /// Returns a generated proxy client for `preferences.preferenceStorageManager`.
  GeneratedPreferencesPreferenceStorageManagerCoreProxy get preferencesPreferenceStorageManager => GeneratedPreferencesPreferenceStorageManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.preferenceStorageManager'));
  /// Returns a generated proxy client for `preferences.promptTagManager`.
  GeneratedPreferencesPromptTagManagerCoreProxy get preferencesPromptTagManager => GeneratedPreferencesPromptTagManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.promptTagManager'));
  /// Returns a generated proxy client for `preferences.sharedMemoryStoreManager`.
  GeneratedPreferencesSharedMemoryStoreManagerCoreProxy get preferencesSharedMemoryStoreManager => GeneratedPreferencesSharedMemoryStoreManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.sharedMemoryStoreManager'));
  /// Returns a generated proxy client for `preferences.skillVisibilityPreferences`.
  GeneratedPreferencesSkillVisibilityPreferencesCoreProxy get preferencesSkillVisibilityPreferences => GeneratedPreferencesSkillVisibilityPreferencesCoreProxy(bridge, CoreObjectPath.parse('preferences.skillVisibilityPreferences'));
  /// Returns a generated proxy client for `preferences.sttConfigManager`.
  GeneratedPreferencesSttConfigManagerCoreProxy get preferencesSttConfigManager => GeneratedPreferencesSttConfigManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.sttConfigManager'));
  /// Returns a generated proxy client for `preferences.ttsConfigManager`.
  GeneratedPreferencesTtsConfigManagerCoreProxy get preferencesTtsConfigManager => GeneratedPreferencesTtsConfigManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.ttsConfigManager'));
  /// Returns a generated proxy client for `preferences.userPreferencesManager`.
  GeneratedPreferencesUserPreferencesManagerCoreProxy get preferencesUserPreferencesManager => GeneratedPreferencesUserPreferencesManagerCoreProxy(bridge, CoreObjectPath.parse('preferences.userPreferencesManager'));
  /// Returns a generated proxy client for `providers.chat.conversationRoundManagerMirror`.
  GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy get providersChatConversationRoundManagerMirror => GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy(bridge, CoreObjectPath.parse('providers.chat.conversationRoundManagerMirror'));
  /// Returns a generated proxy client for `providers.chat.enhance.conversationRoundManager`.
  GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy get providersChatEnhanceConversationRoundManager => GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy(bridge, CoreObjectPath.parse('providers.chat.enhance.conversationRoundManager'));
  /// Returns a generated proxy client for `providers.chat.llmprovider.streamingJsonXmlConverter`.
  GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy get providersChatLlmproviderStreamingJsonXmlConverter => GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy(bridge, CoreObjectPath.parse('providers.chat.llmprovider.streamingJsonXmlConverter'));
  /// Returns a generated proxy client for `providers.market.marketStatsApiService`.
  GeneratedProvidersMarketStatsApiServiceCoreProxy get providersMarketStatsApiService => GeneratedProvidersMarketStatsApiServiceCoreProxy(bridge, CoreObjectPath.parse('providers.market.marketStatsApiService'));
  /// Returns a generated proxy client for `repository.chatHistoryManager`.
  GeneratedRepositoryChatHistoryManagerCoreProxy get repositoryChatHistoryManager => GeneratedRepositoryChatHistoryManagerCoreProxy(bridge, CoreObjectPath.parse('repository.chatHistoryManager'));
  /// Returns a generated proxy client for `repository.memoryRepository`.
  GeneratedRepositoryMemoryRepositoryCoreProxy get repositoryMemoryRepository => GeneratedRepositoryMemoryRepositoryCoreProxy(bridge, CoreObjectPath.parse('repository.memoryRepository'));
  /// Returns a generated proxy client for `repository.runtimeStorageRepository`.
  GeneratedRepositoryRuntimeStorageRepositoryCoreProxy get repositoryRuntimeStorageRepository => GeneratedRepositoryRuntimeStorageRepositoryCoreProxy(bridge, CoreObjectPath.parse('repository.runtimeStorageRepository'));
  /// Returns a generated proxy client for `repository.usageStatisticsStore`.
  GeneratedRepositoryUsageStatisticsStoreCoreProxy get repositoryUsageStatisticsStore => GeneratedRepositoryUsageStatisticsStoreCoreProxy(bridge, CoreObjectPath.parse('repository.usageStatisticsStore'));
  /// Returns a generated proxy client for `runtimeRemoteLinkService`.
  GeneratedRuntimeRemoteLinkServiceCoreProxy get runtimeRemoteLinkService => GeneratedRuntimeRemoteLinkServiceCoreProxy(bridge, CoreObjectPath.parse('runtimeRemoteLinkService'));
  /// Returns a generated proxy client for `services.archiveTransferManager`.
  GeneratedServicesArchiveTransferManagerCoreProxy get servicesArchiveTransferManager => GeneratedServicesArchiveTransferManagerCoreProxy(bridge, CoreObjectPath.parse('services.archiveTransferManager'));
  /// Returns a generated proxy client for `services.gitHubOAuthBrokerService`.
  GeneratedServicesGitHubOAuthBrokerServiceCoreProxy get servicesGitHubOAuthBrokerService => GeneratedServicesGitHubOAuthBrokerServiceCoreProxy(bridge, CoreObjectPath.parse('services.gitHubOAuthBrokerService'));
  /// Returns a generated proxy client for `services.localModelService`.
  GeneratedServicesLocalModelServiceCoreProxy get servicesLocalModelService => GeneratedServicesLocalModelServiceCoreProxy(bridge, CoreObjectPath.parse('services.localModelService'));
  /// Returns a generated proxy client for `services.localProviderService`.
  GeneratedServicesLocalProviderServiceCoreProxy get servicesLocalProviderService => GeneratedServicesLocalProviderServiceCoreProxy(bridge, CoreObjectPath.parse('services.localProviderService'));
  /// Returns a generated proxy client for `services.runtimeBrowserService`.
  GeneratedServicesRuntimeBrowserServiceCoreProxy get servicesRuntimeBrowserService => GeneratedServicesRuntimeBrowserServiceCoreProxy(bridge, CoreObjectPath.parse('services.runtimeBrowserService'));
  /// Returns a generated proxy client for `services.runtimeHostInfoService`.
  GeneratedServicesRuntimeHostInfoServiceCoreProxy get servicesRuntimeHostInfoService => GeneratedServicesRuntimeHostInfoServiceCoreProxy(bridge, CoreObjectPath.parse('services.runtimeHostInfoService'));
  /// Returns a generated proxy client for `services.runtimeHostInteractionService`.
  GeneratedServicesRuntimeHostInteractionServiceCoreProxy get servicesRuntimeHostInteractionService => GeneratedServicesRuntimeHostInteractionServiceCoreProxy(bridge, CoreObjectPath.parse('services.runtimeHostInteractionService'));
  /// Returns a generated proxy client for `services.runtimeTerminalService`.
  GeneratedServicesRuntimeTerminalServiceCoreProxy get servicesRuntimeTerminalService => GeneratedServicesRuntimeTerminalServiceCoreProxy(bridge, CoreObjectPath.parse('services.runtimeTerminalService'));
  /// Returns a generated proxy client for `services.snapshotImportManager`.
  GeneratedServicesSnapshotImportManagerCoreProxy get servicesSnapshotImportManager => GeneratedServicesSnapshotImportManagerCoreProxy(bridge, CoreObjectPath.parse('services.snapshotImportManager'));
  /// Returns a generated proxy client for `services.sttRecognitionService`.
  GeneratedServicesSttRecognitionServiceCoreProxy get servicesSttRecognitionService => GeneratedServicesSttRecognitionServiceCoreProxy(bridge, CoreObjectPath.parse('services.sttRecognitionService'));
  /// Returns a generated proxy client for `services.ttsPlaybackService`.
  GeneratedServicesTtsPlaybackServiceCoreProxy get servicesTtsPlaybackService => GeneratedServicesTtsPlaybackServiceCoreProxy(bridge, CoreObjectPath.parse('services.ttsPlaybackService'));
  /// Returns a generated proxy client for `services.ttsSynthesisService`.
  GeneratedServicesTtsSynthesisServiceCoreProxy get servicesTtsSynthesisService => GeneratedServicesTtsSynthesisServiceCoreProxy(bridge, CoreObjectPath.parse('services.ttsSynthesisService'));
  /// Returns a generated proxy client for `services.workspaceService`.
  GeneratedServicesWorkspaceServiceCoreProxy get servicesWorkspaceService => GeneratedServicesWorkspaceServiceCoreProxy(bridge, CoreObjectPath.parse('services.workspaceService'));
}

class GeneratedApplicationCoreProxy {
  const GeneratedApplicationCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Initializes persistent stores, prompt managers, tool handlers, plugins, and runtime events.
  Future<void> onCreate() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'onCreate',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Delivers one normalized host event to registered ToolPkg host-event hooks.
  Future<Object?> ingestRuntimeEvent({required RuntimeEvent event}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'ingestRuntimeEvent',
        args: <String, Object?>{'event': event.toJson()},
      ),
    );
    return value;
  }

  /// Applies host-specific OpenMP environment setup before runtime services start.
  Future<void> configureOpenMpEnvironment() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'configureOpenMpEnvironment',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Ensures background work infrastructure is available for runtime tasks.
  Future<void> ensureWorkManagerInitialized() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'ensureWorkManagerInitialized',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Registers JSON serialization rules used by generated bridge payloads.
  Future<void> initializeJsonSerializer() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeJsonSerializer',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Initializes application language resources before user-facing services are created.
  Future<void> initializeAppLanguage() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeAppLanguage',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Prepares model, functional, and user preference stores for runtime access.
  Future<void> initUserPreferencesManager() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initUserPreferencesManager',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Initializes platform permission preference state used by Android-facing tools.
  Future<void> initAndroidPermissionPreferences() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initAndroidPermissionPreferences',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Loads character and functional prompt data required by chat sessions.
  Future<void> initializeFunctionalPromptManager() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeFunctionalPromptManager',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Touches database-backed services early so schema setup happens during startup.
  Future<void> preloadDatabase() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'preloadDatabase',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Starts deployed MCP plugins according to the configured startup timeout.
  Future<void> initMcpPlugins() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initMcpPlugins',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Returns the initialized tool handler owned by this runtime.
  GeneratedApplicationAiToolHandlerCoreProxy aiToolHandler() {
    return GeneratedApplicationAiToolHandlerCoreProxy(bridge, CoreObjectPath(<String>[...targetPath.segments, 'aiToolHandler']));
  }

  /// Creates an MCP repository with this runtime's host and tool support.
  GeneratedApplicationMcpRepositoryCoreProxy mcpRepository() {
    return GeneratedApplicationMcpRepositoryCoreProxy(bridge, CoreObjectPath(<String>[...targetPath.segments, 'mcpRepository']));
  }

  /// Creates a skill repository with this runtime's host and tool support.
  GeneratedApplicationSkillRepositoryCoreProxy skillRepository() {
    return GeneratedApplicationSkillRepositoryCoreProxy(bridge, CoreObjectPath(<String>[...targetPath.segments, 'skillRepository']));
  }

  /// Creates a user-markdown repository using this runtime's configured storage host.
  GeneratedApplicationUserMarkdownRepositoryCoreProxy userMarkdownRepository({required String ownerKey}) {
    return GeneratedApplicationUserMarkdownRepositoryCoreProxy(bridge, CoreObjectPath(<String>[...targetPath.segments, 'userMarkdownRepository', ownerKey]));
  }

  /// Creates an input menu bridge backed by this application's tool package runtime.
  GeneratedApplicationInputMenuToggleBridgeCoreProxy inputMenuToggleBridge() {
    return GeneratedApplicationInputMenuToggleBridgeCoreProxy(bridge, CoreObjectPath(<String>[...targetPath.segments, 'inputMenuToggleBridge']));
  }

  /// Returns the shared package manager owned by the initialized tool handler.
  GeneratedApplicationPackageManagerCoreProxy packageManager() {
    return GeneratedApplicationPackageManagerCoreProxy(bridge, CoreObjectPath(<String>[...targetPath.segments, 'packageManager']));
  }

  /// Returns package names enabled in this application runtime.
  Future<List<String>> activePackageNames() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'active_package_names',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Tests one model connection using this application's provider runtime.
  Future<ModelConnectionTestReport> testModelConnection({required String providerId, required String modelId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'test_model_connection',
        args: <String, Object?>{'provider_id': providerId, 'model_id': modelId},
      ),
    );
    return ModelConnectionTestReport.fromJson(value as Map<String, Object?>);
  }

  /// Returns the Cargo package version compiled into the runtime crate.
  Future<String> coreVersion() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'coreVersion',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns structured in-memory application log entries.
  Future<Object?> logEntries() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'logEntries',
        args: const <String, Object?>{},
      ),
    );
    return value;
  }

  /// Reads the application log file as text.
  Future<String> logText() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'logText',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Reads the package-manager log file as text.
  Future<String> packageLogText() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'packageLogText',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the active application log file path.
  Future<String> logFilePath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'logFilePath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the active package-manager log file path.
  Future<String> packageLogFilePath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'packageLogFilePath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the user-visible Operit root directory path.
  Future<String> operitRootPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'operitRootPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the directory used for exported user artifacts.
  Future<String> exportsPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exportsPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the directory used for files removed during clean-on-exit maintenance.
  Future<String> cleanOnExitPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'cleanOnExitPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Clears the current runtime log files.
  Future<void> resetLogs() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resetLogs',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Combines sync clocks from key-value/object stores and SQL chat storage.
  Future<Object?> syncClock() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'syncClock',
        args: const <String, Object?>{},
      ),
    );
    return value;
  }

  /// Lists compacted sync operations newer than the provided device clock.
  Future<Object?> syncOperationsSince({required Object? clock, required List<String> domains, required int limit}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'syncOperationsSince',
        args: <String, Object?>{'clock': clock, 'domains': domains.map((item) => item).toList(growable: false), 'limit': limit},
      ),
    );
    return value;
  }

  /// Applies incoming sync operations to their owning persistent stores.
  Future<Object?> syncApplyOperations({required Object? operations}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'syncApplyOperations',
        args: <String, Object?>{'operations': operations},
      ),
    );
    return value;
  }

}

class GeneratedChatRuntimeHolderMainCoreProxy {
  const GeneratedChatRuntimeHolderMainCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Dispatches chat input change notifications from host-owned input widgets.
  Future<void> dispatchChatInputChanged({required String? chatIdOverride, required String messageText, required int selectionStart, required int selectionEnd, required int attachmentCount}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'dispatchChatInputChanged',
        args: <String, Object?>{'chatIdOverride': chatIdOverride, 'messageText': messageText, 'selectionStart': selectionStart, 'selectionEnd': selectionEnd, 'attachmentCount': attachmentCount},
      ),
    );
  }

  /// Dispatches submit_requested and returns the ToolPkg decision for the host input widget.
  Future<Object?> dispatchChatInputSubmitRequested({required String? chatIdOverride, required String messageText, required int selectionStart, required int selectionEnd, required int attachmentCount}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'dispatchChatInputSubmitRequested',
        args: <String, Object?>{'chatIdOverride': chatIdOverride, 'messageText': messageText, 'selectionStart': selectionStart, 'selectionEnd': selectionEnd, 'attachmentCount': attachmentCount},
      ),
    );
    return value;
  }

  /// Sends a user-authored message through the active chat runtime.
  Future<void> sendUserMessage({required PromptFunctionType promptFunctionType, required String? roleCardIdOverride, required String? chatIdOverride, required String messageText, required String? proxySenderNameOverride, required String? chatProviderIdOverride, required String? chatModelIdOverride, required List<AttachmentInfo> attachments, required ChatMessage? replyToMessage, required ChatTurnOptions turnOptions}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'sendUserMessage',
        args: <String, Object?>{'promptFunctionType': promptFunctionType.toJson(), 'roleCardIdOverride': roleCardIdOverride, 'chatIdOverride': chatIdOverride, 'messageText': messageText, 'proxySenderNameOverride': proxySenderNameOverride, 'chatProviderIdOverride': chatProviderIdOverride, 'chatModelIdOverride': chatModelIdOverride, 'attachments': attachments.map((item) => item.toJson()).toList(growable: false), 'replyToMessage': replyToMessage?.toJson(), 'turnOptions': turnOptions.toJson()},
      ),
    );
  }

  /// Cancels generation for the current chat.
  Future<void> cancelCurrentMessage() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'cancelCurrentMessage',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Cancels message generation for a specific chat id.
  Future<void> cancelMessage({required String chatId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'cancelMessage',
        args: <String, Object?>{'chatId': chatId},
      ),
    );
  }

  /// Returns the live provider response stream for the active turn of a chat.
  Future<Object?> getResponseStreamSnapshot({required String chatId}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'getResponseStream',
        args: <String, Object?>{'chatId': chatId},
      ),
    );
    return event.value;
  }

  /// Returns the live provider response stream for the active turn of a chat.
  Stream<Object?> getResponseStreamChanges({required String chatId}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'getResponseStream', args: <String, Object?>{'chatId': chatId}))
        .map((event) => event.value);
  }

  /// Adds one message to the queue owned by a specific chat.
  Future<void> enqueuePendingQueueMessage({required String chatId, required String messageText}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'enqueuePendingQueueMessage',
        args: <String, Object?>{'chatId': chatId, 'messageText': messageText},
      ),
    );
  }

  /// Deletes one queued message from a specific chat.
  Future<void> deletePendingQueueMessage({required String chatId, required int messageId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deletePendingQueueMessage',
        args: <String, Object?>{'chatId': chatId, 'messageId': messageId},
      ),
    );
  }

  /// Removes one queued message for editing or explicit user delivery.
  Future<PendingQueueMessageItem?> takePendingQueueMessage({required String chatId, required int messageId, required bool suppressNextAutoDequeue}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'takePendingQueueMessage',
        args: <String, Object?>{'chatId': chatId, 'messageId': messageId, 'suppressNextAutoDequeue': suppressNextAutoDequeue},
      ),
    );
    return value == null ? null : PendingQueueMessageItem.fromJson(value as Map<String, Object?>);
  }

  /// Clears a manual-send suppression after that message is not delivered.
  Future<void> clearPendingQueueAutoDequeueSuppression({required String chatId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearPendingQueueAutoDequeueSuppression',
        args: <String, Object?>{'chatId': chatId},
      ),
    );
  }

  /// Atomically removes the next queued message after a chat becomes ready.
  Future<PendingQueueMessageItem?> takeNextPendingQueueMessageIfReady({required String chatId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'takeNextPendingQueueMessageIfReady',
        args: <String, Object?>{'chatId': chatId},
      ),
    );
    return value == null ? null : PendingQueueMessageItem.fromJson(value as Map<String, Object?>);
  }

  /// Inserts a rejected queued message back at the front of its chat queue.
  Future<void> restorePendingQueueMessage({required String chatId, required PendingQueueMessageItem message}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'restorePendingQueueMessage',
        args: <String, Object?>{'chatId': chatId, 'message': message.toJson()},
      ),
    );
  }

  /// Updates whether a chat's pending-message queue is expanded in the UI.
  Future<void> setPendingQueueExpanded({required String chatId, required bool isExpanded}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setPendingQueueExpanded',
        args: <String, Object?>{'chatId': chatId, 'isExpanded': isExpanded},
      ),
    );
  }

  /// Splits markdown content into stable render events for the client.
  Future<List<MarkdownStreamEvent>> splitMarkdownContent({required String content}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'splitMarkdownContent',
        args: <String, Object?>{'content': content},
      ),
    );
    return (value as List<Object?>).map((item) => MarkdownStreamEvent.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Renders one XML block through registered ToolPkg XML render hooks.
  Future<Object?> renderToolPkgXml({required String tagName, required String xmlContent}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'renderToolPkgXml',
        args: <String, Object?>{'tagName': tagName, 'xmlContent': xmlContent},
      ),
    );
    return value;
  }

  /// Creates a new chat and makes it available through chat history state.
  Future<void> createNewChat({required String? characterCardName, required String? group, required bool inheritGroupFromCurrent, required bool setAsCurrentChat, required String? characterGroupId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createNewChat',
        args: <String, Object?>{'characterCardName': characterCardName, 'group': group, 'inheritGroupFromCurrent': inheritGroupFromCurrent, 'setAsCurrentChat': setAsCurrentChat, 'characterGroupId': characterGroupId},
      ),
    );
  }

  /// Switches the active chat and refreshes its runtime state.
  Future<void> switchChat({required String chatId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'switchChat',
        args: <String, Object?>{'chatId': chatId},
      ),
    );
  }

  /// Switches the local runtime selection without writing the global chat selection.
  Future<void> switchChatLocal({required String chatId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'switchChatLocal',
        args: <String, Object?>{'chatId': chatId},
      ),
    );
  }

  /// Changes the active character card target used when new chat turns are sent.
  Future<void> switchActiveCharacterCardTarget({required String characterCardId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'switchActiveCharacterCardTarget',
        args: <String, Object?>{'characterCardId': characterCardId},
      ),
    );
  }

  /// Changes the active character group target used when new group chat turns are sent.
  Future<void> switchActiveCharacterGroupTarget({required String characterGroupId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'switchActiveCharacterGroupTarget',
        args: <String, Object?>{'characterGroupId': characterGroupId},
      ),
    );
  }

  /// Updates the character card binding stored on an existing chat.
  Future<void> updateChatCharacterCard({required String chatId, required String? characterCardName}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateChatCharacterCard',
        args: <String, Object?>{'chatId': chatId, 'characterCardName': characterCardName},
      ),
    );
  }

  /// Updates the character group binding stored on an existing chat.
  Future<void> updateChatCharacterGroup({required String chatId, required String? characterGroupId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateChatCharacterGroup',
        args: <String, Object?>{'chatId': chatId, 'characterGroupId': characterGroupId},
      ),
    );
  }

  /// Synchronizes the current runtime chat id to the global chat selection.
  Future<void> syncCurrentChatIdToGlobal() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'syncCurrentChatIdToGlobal',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Deletes a chat history and updates current chat selection.
  Future<void> deleteChatHistory({required String chatId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteChatHistory',
        args: <String, Object?>{'chatId': chatId},
      ),
    );
  }

  /// Deletes one message from the current chat by visible message index.
  Future<void> deleteMessage({required int index}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteMessage',
        args: <String, Object?>{'index': index},
      ),
    );
  }

  /// Deletes multiple messages from the current chat by visible message indices.
  Future<bool> deleteMessages({required List<int> indices}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteMessages',
        args: <String, Object?>{'indices': indices.map((item) => item).toList(growable: false)},
      ),
    );
    return value as bool;
  }

  /// Replaces the content of one message and refreshes the stable context window.
  Future<bool> updateMessage({required int index, required String editedContent}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateMessage',
        args: <String, Object?>{'index': index, 'editedContent': editedContent},
      ),
    );
    return value as bool;
  }

  /// Deletes the selected message and every following message in the current chat.
  Future<bool> deleteMessagesFrom({required int index}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteMessagesFrom',
        args: <String, Object?>{'index': index},
      ),
    );
    return value as bool;
  }

  /// Deletes one alternate response variant from a message timestamp.
  Future<void> deleteMessageVariant({required int timestamp, required int variantIndex}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteMessageVariant',
        args: <String, Object?>{'timestamp': timestamp, 'variantIndex': variantIndex},
      ),
    );
  }

  /// Creates a branch chat from the current conversation at an optional message timestamp.
  Future<void> createBranch({required int? upToMessageTimestamp}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createBranch',
        args: <String, Object?>{'upToMessageTimestamp': upToMessageTimestamp},
      ),
    );
  }

  /// Generates and inserts a summary message around the selected user or AI message.
  Future<bool> insertSummary({required ChatMessage message}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'insertSummary',
        args: <String, Object?>{'message': message.toJson()},
      ),
    );
    return value as bool;
  }

  /// Returns branch chats that were derived from the requested parent chat.
  Future<List<ChatHistory>> getBranches({required String parentChatId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getBranches',
        args: <String, Object?>{'parentChatId': parentChatId},
      ),
    );
    return (value as List<Object?>).map((item) => ChatHistory.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Updates whether a chat is locked against destructive changes.
  Future<void> updateChatLocked({required String chatId, required bool locked}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateChatLocked',
        args: <String, Object?>{'chatId': chatId, 'locked': locked},
      ),
    );
  }

  /// Updates whether a chat is pinned in chat history ordering.
  Future<void> updateChatPinned({required String chatId, required bool pinned}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateChatPinned',
        args: <String, Object?>{'chatId': chatId, 'pinned': pinned},
      ),
    );
  }

  /// Applies a reordered chat list and optionally moves the active item into a group.
  Future<void> updateChatOrderAndGroup({required List<ChatHistoryListItem> reorderedHistories, required ChatHistoryListItem movedItem, required String? targetGroup}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateChatOrderAndGroup',
        args: <String, Object?>{'reorderedHistories': reorderedHistories.map((item) => item.toJson()).toList(growable: false), 'movedItem': movedItem.toJson(), 'targetGroup': targetGroup},
      ),
    );
  }

  /// Removes every message from the currently selected chat.
  Future<void> clearCurrentChat() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearCurrentChat',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Serializes all chat histories into a JSON archive string.
  Future<String> exportChatHistoriesToJson() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exportChatHistoriesToJson',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Imports chat histories from a JSON archive string.
  Future<ChatImportResult> importChatHistoriesFromJson({required String jsonString}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importChatHistoriesFromJson',
        args: <String, Object?>{'jsonString': jsonString},
      ),
    );
    return ChatImportResult.fromJson(value as Map<String, Object?>);
  }

  /// Updates the stored title of a chat history.
  Future<void> updateChatTitle({required String chatId, required String title}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateChatTitle',
        args: <String, Object?>{'chatId': chatId, 'title': title},
      ),
    );
  }

  /// Binds a chat to an existing workspace path.
  Future<void> bindChatToWorkspace({required String chatId, required String workspace}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'bindChatToWorkspace',
        args: <String, Object?>{'chatId': chatId, 'workspace': workspace},
      ),
    );
  }

  /// Creates the default workspace directory for a chat and returns its path.
  Future<String> createAndGetDefaultWorkspace({required String chatId, required String? projectType}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createAndGetDefaultWorkspace',
        args: <String, Object?>{'chatId': chatId, 'projectType': projectType},
      ),
    );
    return value as String;
  }

  /// Creates the default workspace for a chat and stores the workspace binding.
  Future<String> createAndBindDefaultWorkspace({required String chatId, required String? projectType}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createAndBindDefaultWorkspace',
        args: <String, Object?>{'chatId': chatId, 'projectType': projectType},
      ),
    );
    return value as String;
  }

  /// Removes the workspace binding from a chat without deleting workspace files.
  Future<void> unbindChatFromWorkspace({required String chatId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'unbindChatFromWorkspace',
        args: <String, Object?>{'chatId': chatId},
      ),
    );
  }

  /// Renames the workspace binding and chat title together.
  Future<void> renameWorkspaceAndChat({required String chatId, required String newWorkspace, required String newTitle}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'renameWorkspaceAndChat',
        args: <String, Object?>{'chatId': chatId, 'newWorkspace': newWorkspace, 'newTitle': newTitle},
      ),
    );
  }

  /// Shows file changes that would be applied when rewinding to a message index.
  Future<List<WorkspaceFileChange>> previewWorkspaceChangesForMessage({required int index}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'previewWorkspaceChangesForMessage',
        args: <String, Object?>{'index': index},
      ),
    );
    return (value as List<Object?>).map((item) => WorkspaceFileChange.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Restores the bound workspace to the snapshot before a message index.
  Future<bool> rewindWorkspaceForMessage({required int index}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'rewindWorkspaceForMessage',
        args: <String, Object?>{'index': index},
      ),
    );
    return value as bool;
  }

  /// Rolls the current chat back to a prior message index.
  Future<String?> rollbackToMessage({required int index}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'rollbackToMessage',
        args: <String, Object?>{'index': index},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Rewinds a user message and sends edited content as a new turn.
  Future<bool> rewindAndResendMessage({required int index, required String editedContent}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'rewindAndResendMessage',
        args: <String, Object?>{'index': index, 'editedContent': editedContent},
      ),
    );
    return value as bool;
  }

  /// Regenerates one AI message in place while preserving the surrounding chat history.
  Future<void> regenerateSingleAiMessage({required int index}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'regenerateSingleAiMessage',
        args: <String, Object?>{'index': index},
      ),
    );
  }

  /// Clears the token counters associated with the current chat service.
  Future<void> resetTokenStatistics() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resetTokenStatistics',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Recomputes cumulative token statistics for the current chat and service.
  Future<void> updateCumulativeStatistics() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateCumulativeStatistics',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Adds a file, pasted text, package, screen capture, notification capture, or location capture as an attachment.
  Future<void> handleAttachment({required String filePath}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'handleAttachment',
        args: <String, Object?>{'_filePath': filePath},
      ),
    );
  }

  /// Removes one attachment by its stored file path.
  Future<void> removeAttachment({required String filePath}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removeAttachment',
        args: <String, Object?>{'_filePath': filePath},
      ),
    );
  }

  /// Removes every pending attachment from the chat input.
  Future<void> clearAttachments() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearAttachments',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Returns whether any chat turn is currently streaming or processing.
  Future<bool> isLoading() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isLoading',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

  /// Returns the loading state flow shared with chat UI observers.
  Future<bool> isLoadingFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'isLoadingFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as bool;
  }

  /// Returns the loading state flow shared with chat UI observers.
  Stream<bool> isLoadingFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'isLoadingFlow', args: const <String, Object?>{}))
        .map((event) => event.value as bool);
  }

  /// Returns chat ids that currently have active streaming turns.
  Future<List<String>> activeStreamingChatIds() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'activeStreamingChatIds',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Returns the state flow of chat ids that currently have active streaming turns.
  Future<List<String>> activeStreamingChatIdsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'activeStreamingChatIdsFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Returns the state flow of chat ids that currently have active streaming turns.
  Stream<List<String>> activeStreamingChatIdsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'activeStreamingChatIdsFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => item as String).toList(growable: false));
  }

  /// Returns the state flow of processing states keyed by chat id.
  Future<Map<String, InputProcessingState>> inputProcessingStateByChatIdFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'inputProcessingStateByChatIdFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, InputProcessingState.fromJson(value)));
  }

  /// Returns the state flow of processing states keyed by chat id.
  Stream<Map<String, InputProcessingState>> inputProcessingStateByChatIdFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'inputProcessingStateByChatIdFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, InputProcessingState.fromJson(value))));
  }

  /// Returns transient toast messages emitted by chat input actions.
  Future<String?> toastEventFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'toastEventFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value == null ? null : event.value as String;
  }

  /// Returns transient toast messages emitted by chat input actions.
  Stream<String?> toastEventFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'toastEventFlow', args: const <String, Object?>{}))
        .map((event) => event.value == null ? null : event.value as String);
  }

  /// Clears the current transient toast event after the UI has consumed it.
  Future<void> clearToastEvent() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearToastEvent',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Returns the processing state for the currently selected chat.
  Future<InputProcessingState> currentChatInputProcessingState() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'currentChatInputProcessingState',
        args: const <String, Object?>{},
      ),
    );
    return InputProcessingState.fromJson(value);
  }

  /// Returns whether the currently selected chat is actively streaming.
  Future<bool> currentChatIsLoading() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'currentChatIsLoading',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

  /// Returns whether older messages exist beyond the current display window.
  Future<bool> hasOlderDisplayHistory() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'hasOlderDisplayHistory',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

  /// Returns whether newer messages exist beyond the current display window.
  Future<bool> hasNewerDisplayHistory() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'hasNewerDisplayHistory',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

  /// Returns whether the display-window loader is currently fetching messages.
  Future<bool> isLoadingDisplayWindow() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isLoadingDisplayWindow',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

  /// Returns the state flow of tool invocation counts keyed by chat id.
  Future<Map<String, int>> currentTurnToolInvocationCountByChatIdFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'currentTurnToolInvocationCountByChatIdFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as int));
  }

  /// Returns the state flow of tool invocation counts keyed by chat id.
  Stream<Map<String, int>> currentTurnToolInvocationCountByChatIdFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'currentTurnToolInvocationCountByChatIdFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as int)));
  }

  /// Returns the state flow of messages for the current chat.
  Future<List<ChatMessage>> chatHistoryFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'chatHistoryFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => ChatMessage.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns the state flow of messages for the current chat.
  Stream<List<ChatMessage>> chatHistoryFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'chatHistoryFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => ChatMessage.fromJson(item as Map<String, Object?>)).toList(growable: false));
  }

  /// Returns the state flow of the currently selected chat id.
  Future<String?> currentChatIdFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'currentChatIdFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value == null ? null : event.value as String;
  }

  /// Returns the state flow of the currently selected chat id.
  Stream<String?> currentChatIdFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'currentChatIdFlow', args: const <String, Object?>{}))
        .map((event) => event.value == null ? null : event.value as String);
  }

  /// Returns the state flow of all persisted chat histories.
  Future<List<ChatHistory>> chatHistoriesFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'chatHistoriesFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => ChatHistory.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns the state flow of all persisted chat histories.
  Stream<List<ChatHistory>> chatHistoriesFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'chatHistoriesFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => ChatHistory.fromJson(item as Map<String, Object?>)).toList(growable: false));
  }

  /// Returns chat history list items prepared for grouped history UI.
  Future<List<ChatHistoryListItem>> chatHistoryListItemsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'chatHistoryListItemsFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => ChatHistoryListItem.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns chat history list items prepared for grouped history UI.
  Stream<List<ChatHistoryListItem>> chatHistoryListItemsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'chatHistoryListItemsFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => ChatHistoryListItem.fromJson(item as Map<String, Object?>)).toList(growable: false));
  }

  /// Combines chat messages, selection, loading, display-window, and prompt state for the main chat UI.
  Future<ChatMainState> chatMainStateFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'chatMainStateFlow',
        args: const <String, Object?>{},
      ),
    );
    return ChatMainState.fromJson(event.value as Map<String, Object?>);
  }

  /// Combines chat messages, selection, loading, display-window, and prompt state for the main chat UI.
  Stream<ChatMainState> chatMainStateFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'chatMainStateFlow', args: const <String, Object?>{}))
        .map((event) => ChatMainState.fromJson(event.value as Map<String, Object?>));
  }

  /// Returns whether the chat history selector should be visible.
  Future<bool> showChatHistorySelector() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'showChatHistorySelector',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

  /// Returns a snapshot of pending input attachments.
  Future<List<AttachmentInfo>> attachments() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'attachments',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => AttachmentInfo.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns the current context window size state flow.
  Future<int> currentWindowSizeFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'currentWindowSizeFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as int;
  }

  /// Returns the current context window size state flow.
  Stream<int> currentWindowSizeFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'currentWindowSizeFlow', args: const <String, Object?>{}))
        .map((event) => event.value as int);
  }

  /// Returns the cumulative input token count state flow.
  Future<int> inputTokenCountFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'inputTokenCountFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as int;
  }

  /// Returns the cumulative input token count state flow.
  Stream<int> inputTokenCountFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'inputTokenCountFlow', args: const <String, Object?>{}))
        .map((event) => event.value as int);
  }

  /// Returns the cumulative output token count state flow.
  Future<int> outputTokenCountFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'outputTokenCountFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as int;
  }

  /// Returns the cumulative output token count state flow.
  Stream<int> outputTokenCountFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'outputTokenCountFlow', args: const <String, Object?>{}))
        .map((event) => event.value as int);
  }

  /// Returns whether this chat core has finished delegate initialization.
  Future<bool> isInitialized() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isInitialized',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

  /// Reloads chat messages using the display-window strategy for the requested chat.
  Future<void> reloadChatMessagesSmart({required String chatId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'reloadChatMessagesSmart',
        args: <String, Object?>{'chatId': chatId},
      ),
    );
  }

  /// Loads older messages into the current chat display window.
  Future<void> loadOlderMessagesForCurrentChat() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'loadOlderMessagesForCurrentChat',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Loads newer messages into the current chat display window.
  Future<void> loadNewerMessagesForCurrentChat() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'loadNewerMessagesForCurrentChat',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Moves the current chat display window to the latest messages.
  Future<void> showLatestMessagesForCurrentChat() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'showLatestMessagesForCurrentChat',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Searches a chat and returns lightweight message previews for navigation.
  Future<List<ChatMessageLocatorPreview>> loadChatMessageLocatorPreviews({required String chatId, required String query}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'loadChatMessageLocatorPreviews',
        args: <String, Object?>{'chatId': chatId, 'query': query},
      ),
    );
    return (value as List<Object?>).map((item) => ChatMessageLocatorPreview.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Marks or unmarks one message as a favorite by message timestamp.
  Future<void> setMessageFavorite({required int timestamp, required bool isFavorite}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setMessageFavorite',
        args: <String, Object?>{'timestamp': timestamp, 'isFavorite': isFavorite},
      ),
    );
  }

}

class GeneratedLinkAccessStoreCoreProxy {
  const GeneratedLinkAccessStoreCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Initializes and returns the runtime's persisted Link device identity.
  Future<LinkAccessIdentity> initializeIdentity({required RemoteDeviceInfo deviceInfo}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeIdentity',
        args: <String, Object?>{'deviceInfo': deviceInfo.toJson()},
      ),
    );
    return LinkAccessIdentity.fromJson(value as Map<String, Object?>);
  }

  /// Returns every accepted inbound session owned by this runtime.
  Future<Map<String, AcceptedRemoteSessionRecord>> inboundSessions() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'inboundSessions',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, AcceptedRemoteSessionRecord.fromJson(value as Map<String, Object?>)));
  }

  /// Persists one accepted inbound session owned by this runtime.
  Future<void> saveInboundSession({required String sessionId, required AcceptedRemoteSessionRecord record}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveInboundSession',
        args: <String, Object?>{'sessionId': sessionId, 'record': record.toJson()},
      ),
    );
  }

  /// Removes one accepted inbound session owned by this runtime.
  Future<void> removeInboundSession({required String sessionId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removeInboundSession',
        args: <String, Object?>{'sessionId': sessionId},
      ),
    );
  }

  /// Returns every named outbound session owned by this runtime.
  Future<Map<String, PairedRemoteSessionRecord>> outboundSessions() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'outboundSessions',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, PairedRemoteSessionRecord.fromJson(value as Map<String, Object?>)));
  }

  /// Persists one named outbound session owned by this runtime.
  Future<void> saveOutboundSession({required String name, required PairedRemoteSessionRecord record}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveOutboundSession',
        args: <String, Object?>{'name': name, 'record': record.toJson()},
      ),
    );
  }

  /// Removes one named outbound session owned by this runtime.
  Future<void> removeOutboundSession({required String name}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removeOutboundSession',
        args: <String, Object?>{'name': name},
      ),
    );
  }

  /// Returns every pending pairing owned by this runtime.
  Future<Map<String, RemotePairingCodeRecord>> pendingPairings() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'pendingPairings',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, RemotePairingCodeRecord.fromJson(value as Map<String, Object?>)));
  }

  /// Persists one pending pairing owned by this runtime.
  Future<void> savePendingPairing({required RemotePairingCodeRecord record}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'savePendingPairing',
        args: <String, Object?>{'record': record.toJson()},
      ),
    );
  }

  /// Removes one pending pairing owned by this runtime.
  Future<void> removePendingPairing({required String pairingId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removePendingPairing',
        args: <String, Object?>{'pairingId': pairingId},
      ),
    );
  }

  /// Returns every pending outbound pairing initiated by this runtime.
  Future<Map<String, PendingOutboundPairingRecord>> pendingOutboundPairings() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'pendingOutboundPairings',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, PendingOutboundPairingRecord.fromJson(value as Map<String, Object?>)));
  }

  /// Persists one pending outbound pairing initiated by this runtime.
  Future<void> savePendingOutboundPairing({required String pairingId, required PendingOutboundPairingRecord record}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'savePendingOutboundPairing',
        args: <String, Object?>{'pairingId': pairingId, 'record': record.toJson()},
      ),
    );
  }

  /// Removes one pending outbound pairing after it has completed or been cancelled.
  Future<void> removePendingOutboundPairing({required String pairingId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removePendingOutboundPairing',
        args: <String, Object?>{'pairingId': pairingId},
      ),
    );
  }

  /// Persists the active Link Access host configuration for this runtime.
  Future<void> saveHostConfig({required LinkAccessHostConfig config}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveHostConfig',
        args: <String, Object?>{'config': config.toJson()},
      ),
    );
  }

  /// Initializes and returns the active Link Access host configuration.
  Future<LinkAccessHostConfig> initializeHostConfig() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeHostConfig',
        args: const <String, Object?>{},
      ),
    );
    return LinkAccessHostConfig.fromJson(value as Map<String, Object?>);
  }

  /// Reads the active Link Access host configuration for this runtime.
  Future<LinkAccessHostConfig> hostConfig() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'hostConfig',
        args: const <String, Object?>{},
      ),
    );
    return LinkAccessHostConfig.fromJson(value as Map<String, Object?>);
  }

  /// Initializes and returns this runtime's Link auto-sync configuration.
  Future<LinkAccessAutoSyncConfig> initializeAutoSyncConfig() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeAutoSyncConfig',
        args: const <String, Object?>{},
      ),
    );
    return LinkAccessAutoSyncConfig.fromJson(value as Map<String, Object?>);
  }

  /// Reads this runtime's Link auto-sync configuration.
  Future<LinkAccessAutoSyncConfig> autoSyncConfig() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'autoSyncConfig',
        args: const <String, Object?>{},
      ),
    );
    return LinkAccessAutoSyncConfig.fromJson(value as Map<String, Object?>);
  }

  /// Persists this runtime's Link auto-sync configuration.
  Future<void> saveAutoSyncConfig({required LinkAccessAutoSyncConfig config}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveAutoSyncConfig',
        args: <String, Object?>{'config': config.toJson()},
      ),
    );
  }

  /// Initializes and returns this runtime's Link request routing configuration.
  Future<LinkAccessRoutingConfig> initializeRoutingConfig() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeRoutingConfig',
        args: const <String, Object?>{},
      ),
    );
    return LinkAccessRoutingConfig.fromJson(value as Map<String, Object?>);
  }

  /// Reads this runtime's Link request routing configuration.
  Future<LinkAccessRoutingConfig> routingConfig() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'routingConfig',
        args: const <String, Object?>{},
      ),
    );
    return LinkAccessRoutingConfig.fromJson(value as Map<String, Object?>);
  }

  /// Persists this runtime's Link request routing configuration.
  Future<void> saveRoutingConfig({required LinkAccessRoutingConfig config}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveRoutingConfig',
        args: <String, Object?>{'config': config.toJson()},
      ),
    );
  }

}

class GeneratedPermissionsMcpManagerCoreProxy {
  const GeneratedPermissionsMcpManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Returns whether a server name has been registered.
  Future<bool> isServerRegistered({required String serverName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isServerRegistered',
        args: <String, Object?>{'serverName': serverName},
      ),
    );
    return value as bool;
  }

  /// Returns the most recent connection failure detail for a server.
  Future<String?> getLastConnectionFailureReason({required String serverName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getLastConnectionFailureReason',
        args: <String, Object?>{'serverName': serverName},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Registers an MCP server from an endpoint URL and description.
  Future<void> registerServerFromEndpoint({required String serverName, required String endpoint, required String description}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'registerServerFromEndpoint',
        args: <String, Object?>{'serverName': serverName, 'endpoint': endpoint, 'description': description},
      ),
    );
  }

  /// Removes an MCP server and disconnects its cached client.
  Future<void> unregisterServer({required String serverName}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'unregisterServer',
        args: <String, Object?>{'serverName': serverName},
      ),
    );
  }

  /// Disconnects all cached MCP bridge clients.
  Future<void> shutdown() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'shutdown',
        args: const <String, Object?>{},
      ),
    );
  }

}

class GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy {
  const GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Reloads MCP server and plugin configuration from the runtime store.
  Future<void> reloadConfigurations() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'reloadConfigurations',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Rewrites the MCP configuration file after loading and sanitizing it.
  Future<void> saveMcpConfig() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveMCPConfig',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Rewrites the persisted MCP server status file after loading it.
  Future<void> saveServerStatus() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveServerStatus',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Adds or replaces a command-based MCP server entry in the local config.
  Future<void> addOrUpdateMcpServer({required String serverId, required String command, required List<String> args, required Map<String, String> env, required bool disabled, required List<String> autoApprove}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'addOrUpdateMCPServer',
        args: <String, Object?>{'serverId': serverId, 'command': command, 'args': args.map((item) => item).toList(growable: false), 'env': env.map((key, value) => MapEntry(key, value)), 'disabled': disabled, 'autoApprove': autoApprove.map((item) => item).toList(growable: false)},
      ),
    );
  }

  /// Adds or replaces a complete MCP server config after validation.
  Future<void> addOrUpdateMcpServerConfig({required String serverId, required ServerConfig serverConfig}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'addOrUpdateMCPServerConfig',
        args: <String, Object?>{'serverId': serverId, 'serverConfig': serverConfig.toJson()},
      ),
    );
  }

  /// Removes an MCP server config, metadata, status, and local plugin directory.
  Future<void> removeMcpServer({required String serverId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removeMCPServer',
        args: <String, Object?>{'serverId': serverId},
      ),
    );
  }

  /// Imports MCP server entries from a JSON config payload and returns the inserted count.
  Future<int> mergeConfigFromJson({required String jsonConfig}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'mergeConfigFromJson',
        args: <String, Object?>{'jsonConfig': jsonConfig},
      ),
    );
    return value as int;
  }

  /// Returns the absolute path of the MCP configuration file.
  Future<String> getConfigFilePath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getConfigFilePath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the directory used for local MCP plugin runtime files.
  Future<String> getConfigDirectory() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getConfigDirectory',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns one MCP server config by id.
  Future<ServerConfig?> getMcpServer({required String serverId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getMCPServer',
        args: <String, Object?>{'serverId': serverId},
      ),
    );
    return value == null ? null : ServerConfig.fromJson(value as Map<String, Object?>);
  }

  /// Returns every configured MCP server keyed by server id.
  Future<Map<String, ServerConfig>> getAllMcpServers() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllMCPServers',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, ServerConfig.fromJson(value as Map<String, Object?>)));
  }

  /// Adds or replaces display metadata for an installed MCP plugin.
  Future<void> addOrUpdatePluginMetadata({required String pluginId, required PluginMetadata metadata}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'addOrUpdatePluginMetadata',
        args: <String, Object?>{'pluginId': pluginId, 'metadata': metadata.toJson()},
      ),
    );
  }

  /// Removes display metadata for an MCP plugin.
  Future<void> removePluginMetadata({required String pluginId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removePluginMetadata',
        args: <String, Object?>{'pluginId': pluginId},
      ),
    );
  }

  /// Returns display metadata for one MCP plugin.
  Future<PluginMetadata?> getPluginMetadata({required String pluginId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPluginMetadata',
        args: <String, Object?>{'pluginId': pluginId},
      ),
    );
    return value == null ? null : PluginMetadata.fromJson(value as Map<String, Object?>);
  }

  /// Returns all MCP plugin metadata keyed by plugin id.
  Future<Map<String, PluginMetadata>> getAllPluginMetadata() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllPluginMetadata',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, PluginMetadata.fromJson(value as Map<String, Object?>)));
  }

  /// Updates runtime status, cached tool metadata, and timestamps for one MCP server.
  Future<void> updateServerStatus({required String serverId, required String? errorMessage, required List<CachedToolInfo>? cachedTools, required int? lastStartTime, required int? lastStopTime}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateServerStatus',
        args: <String, Object?>{'serverId': serverId, 'errorMessage': errorMessage, 'cachedTools': cachedTools?.map((item) => item.toJson()).toList(growable: false), 'lastStartTime': lastStartTime, 'lastStopTime': lastStopTime},
      ),
    );
  }

  /// Stores the latest discovered tools for an MCP server.
  Future<void> cacheServerTools({required String serverId, required List<CachedToolInfo> tools}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'cacheServerTools',
        args: <String, Object?>{'serverId': serverId, 'tools': tools.map((item) => item.toJson()).toList(growable: false)},
      ),
    );
  }

  /// Returns cached tool metadata for an MCP server.
  Future<List<CachedToolInfo>?> getCachedTools({required String serverId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCachedTools',
        args: <String, Object?>{'serverId': serverId},
      ),
    );
    return value == null ? null : (value as List<Object?>).map((item) => CachedToolInfo.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns whether an MCP server has non-empty tool cache newer than one day.
  Future<bool> hasValidToolCache({required String serverId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'hasValidToolCache',
        args: <String, Object?>{'serverId': serverId},
      ),
    );
    return value as bool;
  }

  /// Removes runtime status information for one MCP server.
  Future<void> removeServerStatus({required String serverId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removeServerStatus',
        args: <String, Object?>{'serverId': serverId},
      ),
    );
  }

  /// Returns runtime status information for one MCP server.
  Future<ServerStatus?> getServerStatus({required String serverId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getServerStatus',
        args: <String, Object?>{'serverId': serverId},
      ),
    );
    return value == null ? null : ServerStatus.fromJson(value as Map<String, Object?>);
  }

  /// Returns runtime status information for every known MCP server.
  Future<Map<String, ServerStatus>> getAllServerStatus() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllServerStatus',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, ServerStatus.fromJson(value as Map<String, Object?>)));
  }

  /// Returns whether the last status timestamps indicate that the server is running.
  Future<bool> isServerLikelyRunning({required String serverId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isServerLikelyRunning',
        args: <String, Object?>{'serverId': serverId},
      ),
    );
    return value as bool;
  }

  /// Returns whether a configured MCP server is enabled.
  Future<bool> isServerEnabled({required String serverId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isServerEnabled',
        args: <String, Object?>{'serverId': serverId},
      ),
    );
    return value as bool;
  }

  /// Enables or disables a configured MCP server.
  Future<void> setServerEnabled({required String serverId, required bool enabled}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setServerEnabled',
        args: <String, Object?>{'serverId': serverId, 'enabled': enabled},
      ),
    );
  }

  /// Returns the runtime directory used by an installed MCP plugin.
  Future<String> getPluginRuntimeDirectory({required String pluginId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPluginRuntimeDirectory',
        args: <String, Object?>{'pluginId': pluginId},
      ),
    );
    return value as String;
  }

  /// Exports one plugin server config as a pretty JSON document.
  Future<String> getPluginConfig({required String pluginId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPluginConfig',
        args: <String, Object?>{'pluginId': pluginId},
      ),
    );
    return value as String;
  }

  /// Saves one plugin server config from either a full MCP config JSON or a server JSON.
  Future<bool> savePluginConfig({required String pluginId, required String configJson}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'savePluginConfig',
        args: <String, Object?>{'pluginId': pluginId, 'configJson': configJson},
      ),
    );
    return value as bool;
  }

  /// Exports MCP config and server status as one JSON document.
  Future<String> exportConfigAsJson() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exportConfigAsJson',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Imports MCP config and server status from an exported JSON document.
  Future<bool> importConfigFromJson({required String json}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importConfigFromJson',
        args: <String, Object?>{'json': json},
      ),
    );
    return value as bool;
  }

}

class GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy {
  const GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Registers a local MCP service process with command, arguments, environment, and working directory.
  Future<Object?> registerMcpService({required String name, required String command, required List<String> args, required String? description, required Map<String, String> env, required String? cwd}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'registerMcpService',
        args: <String, Object?>{'name': name, 'command': command, 'args': args.map((item) => item).toList(growable: false), 'description': description, 'env': env.map((key, value) => MapEntry(key, value)), 'cwd': cwd},
      ),
    );
    return value;
  }

  /// Registers a remote MCP service endpoint with optional authentication headers.
  Future<Object?> registerRemoteMcpService({required String name, required String endpoint, required String? connectionType, required String? description, required String? bearerToken, required Map<String, String> headers}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'registerRemoteMcpService',
        args: <String, Object?>{'name': name, 'endpoint': endpoint, 'connectionType': connectionType, 'description': description, 'bearerToken': bearerToken, 'headers': headers.map((key, value) => MapEntry(key, value))},
      ),
    );
    return value;
  }

  /// Unregisters an MCP service and stops its active process or session.
  Future<Object?> unregisterMcpService({required String name}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'unregisterMcpService',
        args: <String, Object?>{'name': name},
      ),
    );
    return value;
  }

  /// Lists registered MCP services with active state and discovered tools.
  Future<Object?> listMcpServices({required String? serviceName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'listMcpServices',
        args: <String, Object?>{'serviceName': serviceName},
      ),
    );
    return value;
  }

  /// Stops an active MCP service without removing its registration.
  Future<Object?> unspawnMcpService({required String name}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'unspawnMcpService',
        args: <String, Object?>{'name': name},
      ),
    );
    return value;
  }

  /// Stores a tool list for a service when it is not currently active.
  Future<Object?> cacheTools({required String serviceName, required List<Object?> tools}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'cacheTools',
        args: <String, Object?>{'serviceName': serviceName, 'tools': tools.map((item) => item).toList(growable: false)},
      ),
    );
    return value;
  }

  /// Lists tools from an active service or its cached tool metadata.
  Future<Object?> listTools({required String serviceName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'listTools',
        args: <String, Object?>{'serviceName': serviceName},
      ),
    );
    return value;
  }

  /// Calls one tool on an active local or remote MCP service.
  Future<Object?> callTool({required String serviceName, required String method, required Object? params}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'callTool',
        args: <String, Object?>{'serviceName': serviceName, 'method': method, 'params': params},
      ),
    );
    return value;
  }

  /// Returns logs or the latest startup error for an MCP service.
  Future<Object?> getServiceLogs({required String serviceName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getServiceLogs',
        args: <String, Object?>{'serviceName': serviceName},
      ),
    );
    return value;
  }

  /// Stops all active MCP services and clears bridge registrations and cached tools.
  Future<Object?> resetBridge() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resetBridge',
        args: const <String, Object?>{},
      ),
    );
    return value;
  }

}

class GeneratedPermissionsToolPermissionSystemCoreProxy {
  const GeneratedPermissionsToolPermissionSystemCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Removes the active interactive permission requester.
  Future<void> clearAsyncPermissionRequester() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearAsyncPermissionRequester',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Clears approvals that were granted for the current runtime session.
  Future<void> clearSessionApprovals() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearSessionApprovals',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Saves the global AI permission mode used to allow or block tool effects.
  Future<void> saveAiPermissionMode({required AiPermissionMode mode}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveAiPermissionMode',
        args: <String, Object?>{'mode': mode.toJson()},
      ),
    );
  }

  /// Reads the global AI permission mode used to allow or block tool effects.
  Future<AiPermissionMode> getAiPermissionMode() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAiPermissionMode',
        args: const <String, Object?>{},
      ),
    );
    return AiPermissionMode.fromJson(value);
  }

  /// Builds the description shown to the requester for a tool invocation.
  Future<String> getOperationDescription({required Object? tool}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getOperationDescription',
        args: <String, Object?>{'tool': tool},
      ),
    );
    return value as String;
  }

  /// Asynchronously requests approval for a package tool invocation.
  Future<bool> checkPackageToolApprovalAsync({required Object? tool}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'checkPackageToolApprovalAsync',
        args: <String, Object?>{'tool': tool},
      ),
    );
    return value as bool;
  }

  /// Asynchronously requests approval for a tool invocation that can escape the sandbox boundary.
  Future<bool> checkSandboxEscapeApprovalAsync({required Object? tool}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'checkSandboxEscapeApprovalAsync',
        args: <String, Object?>{'tool': tool},
      ),
    );
    return value as bool;
  }

  /// Refreshes permission request state exposed to front-end observers.
  Future<bool> refreshPermissionRequestState() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'refreshPermissionRequestState',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

}

class GeneratedPreferencesActivePromptManagerCoreProxy {
  const GeneratedPreferencesActivePromptManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Returns the active prompt state derived from selected group and character card.
  Future<ActivePrompt> activePromptFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'activePromptFlow',
        args: const <String, Object?>{},
      ),
    );
    return ActivePrompt.fromJson(event.value);
  }

  /// Returns the active prompt state derived from selected group and character card.
  Stream<ActivePrompt> activePromptFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'activePromptFlow', args: const <String, Object?>{}))
        .map((event) => ActivePrompt.fromJson(event.value));
  }

  /// Reads the current active prompt snapshot.
  Future<ActivePrompt> getActivePrompt() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getActivePrompt',
        args: const <String, Object?>{},
      ),
    );
    return ActivePrompt.fromJson(value);
  }

  /// Stores the active prompt and clears the opposite prompt target.
  Future<void> setActivePrompt({required ActivePrompt prompt}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setActivePrompt',
        args: <String, Object?>{'prompt': prompt.toJson()},
      ),
    );
  }

  /// Activates a character group or card based on chat binding metadata.
  Future<void> activateForChatBinding({required String? characterCardName, required String? characterGroupId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'activateForChatBinding',
        args: <String, Object?>{'characterCardName': characterCardName, 'characterGroupId': characterGroupId},
      ),
    );
  }

  /// Returns the character card id that should be used for the next send operation.
  Future<String> resolveActiveCardIdForSend() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resolveActiveCardIdForSend',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

}

class GeneratedPreferencesApiPreferencesCoreProxy {
  const GeneratedPreferencesApiPreferencesCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Adds token counts to the cumulative total for one provider/model.
  Future<void> updateTokensForProviderModel({required String providerModel, required int inputTokens, required int outputTokens, required int cachedInputTokens}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateTokensForProviderModel',
        args: <String, Object?>{'providerModel': providerModel, 'inputTokens': inputTokens, 'outputTokens': outputTokens, 'cachedInputTokens': cachedInputTokens},
      ),
    );
  }

  /// Reads uncached input token count for one provider/model.
  Future<int> getInputTokensForProviderModel({required String providerModel}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getInputTokensForProviderModel',
        args: <String, Object?>{'providerModel': providerModel},
      ),
    );
    return value as int;
  }

  /// Reads cached input token count for one provider/model.
  Future<int> getCachedInputTokensForProviderModel({required String providerModel}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCachedInputTokensForProviderModel',
        args: <String, Object?>{'providerModel': providerModel},
      ),
    );
    return value as int;
  }

  /// Reads output token count for one provider/model.
  Future<int> getOutputTokensForProviderModel({required String providerModel}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getOutputTokensForProviderModel',
        args: <String, Object?>{'providerModel': providerModel},
      ),
    );
    return value as int;
  }

  /// Reads all provider/model token counters.
  Future<Map<String, List<int>>> getAllProviderModelTokens() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllProviderModelTokens',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, (value as List<Object?>).map((item) => item as int).toList(growable: false)));
  }

  /// Observes all provider/model token counters.
  Future<Map<String, List<int>>> allProviderModelTokensFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'allProviderModelTokensFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, (value as List<Object?>).map((item) => item as int).toList(growable: false)));
  }

  /// Observes all provider/model token counters.
  Stream<Map<String, List<int>>> allProviderModelTokensFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'allProviderModelTokensFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, (value as List<Object?>).map((item) => item as int).toList(growable: false))));
  }

  /// Clears every provider/model token counter.
  Future<void> resetAllProviderModelTokenCounts() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resetAllProviderModelTokenCounts',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Clears token counters for one provider/model.
  Future<void> resetProviderModelTokenCounts({required String providerModel}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resetProviderModelTokenCounts',
        args: <String, Object?>{'providerModel': providerModel},
      ),
    );
  }

  /// Observes whether thinking mode is enabled.
  Future<bool> enableThinkingModeFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'enableThinkingModeFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as bool;
  }

  /// Observes whether thinking mode is enabled.
  Stream<bool> enableThinkingModeFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'enableThinkingModeFlow', args: const <String, Object?>{}))
        .map((event) => event.value as bool);
  }

  /// Observes the persisted feature toggle map.
  Future<Map<String, bool>> featureTogglesFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'featureTogglesFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as bool));
  }

  /// Observes the persisted feature toggle map.
  Stream<Map<String, bool>> featureTogglesFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'featureTogglesFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as bool)));
  }

  /// Observes one feature toggle with an explicit default.
  Future<bool> featureToggleFlowSnapshot({required String featureKey, required bool defaultValue}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'featureToggleFlow',
        args: <String, Object?>{'featureKey': featureKey, 'defaultValue': defaultValue},
      ),
    );
    return event.value as bool;
  }

  /// Observes one feature toggle with an explicit default.
  Stream<bool> featureToggleFlowChanges({required String featureKey, required bool defaultValue}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'featureToggleFlow', args: <String, Object?>{'featureKey': featureKey, 'defaultValue': defaultValue}))
        .map((event) => event.value as bool);
  }

  /// Observes the current thinking quality level.
  Future<int> thinkingQualityLevelFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'thinkingQualityLevelFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as int;
  }

  /// Observes the current thinking quality level.
  Stream<int> thinkingQualityLevelFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'thinkingQualityLevelFlow', args: const <String, Object?>{}))
        .map((event) => event.value as int);
  }

  /// Observes whether memory auto-update is enabled.
  Future<bool> enableMemoryAutoUpdateFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'enableMemoryAutoUpdateFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as bool;
  }

  /// Observes whether memory auto-update is enabled.
  Stream<bool> enableMemoryAutoUpdateFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'enableMemoryAutoUpdateFlow', args: const <String, Object?>{}))
        .map((event) => event.value as bool);
  }

  /// Observes whether AI tools are enabled.
  Future<bool> enableToolsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'enableToolsFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as bool;
  }

  /// Observes whether AI tools are enabled.
  Stream<bool> enableToolsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'enableToolsFlow', args: const <String, Object?>{}))
        .map((event) => event.value as bool);
  }

  /// Observes per-tool prompt visibility settings.
  Future<Map<String, bool>> toolPromptVisibilityFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'toolPromptVisibilityFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as bool));
  }

  /// Observes per-tool prompt visibility settings.
  Stream<Map<String, bool>> toolPromptVisibilityFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'toolPromptVisibilityFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as bool)));
  }

  /// Observes whether streaming output is disabled.
  Future<bool> disableStreamOutputFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'disableStreamOutputFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as bool;
  }

  /// Observes whether streaming output is disabled.
  Stream<bool> disableStreamOutputFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'disableStreamOutputFlow', args: const <String, Object?>{}))
        .map((event) => event.value as bool);
  }

  /// Observes whether user preference descriptions are hidden from prompts.
  Future<bool> disableUserPreferenceDescriptionFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'disableUserPreferenceDescriptionFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as bool;
  }

  /// Observes whether user preference descriptions are hidden from prompts.
  Stream<bool> disableUserPreferenceDescriptionFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'disableUserPreferenceDescriptionFlow', args: const <String, Object?>{}))
        .map((event) => event.value as bool);
  }

  /// Observes the image-history turn limit.
  Future<int> maxImageHistoryUserTurnsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'maxImageHistoryUserTurnsFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as int;
  }

  /// Observes the image-history turn limit.
  Stream<int> maxImageHistoryUserTurnsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'maxImageHistoryUserTurnsFlow', args: const <String, Object?>{}))
        .map((event) => event.value as int);
  }

  /// Observes the media-history turn limit.
  Future<int> maxMediaHistoryUserTurnsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'maxMediaHistoryUserTurnsFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as int;
  }

  /// Observes the media-history turn limit.
  Stream<int> maxMediaHistoryUserTurnsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'maxMediaHistoryUserTurnsFlow', args: const <String, Object?>{}))
        .map((event) => event.value as int);
  }

  /// Observes the MCP startup timeout in seconds.
  Future<int> mcpStartupTimeoutSecondsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'mcpStartupTimeoutSecondsFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as int;
  }

  /// Observes the MCP startup timeout in seconds.
  Stream<int> mcpStartupTimeoutSecondsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'mcpStartupTimeoutSecondsFlow', args: const <String, Object?>{}))
        .map((event) => event.value as int);
  }

  /// Observes the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
  Future<int> toolPkgPreHookTimeoutSecondsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'toolPkgPreHookTimeoutSecondsFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as int;
  }

  /// Observes the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
  Stream<int> toolPkgPreHookTimeoutSecondsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'toolPkgPreHookTimeoutSecondsFlow', args: const <String, Object?>{}))
        .map((event) => event.value as int);
  }

  /// Saves the thinking-mode toggle.
  Future<void> saveEnableThinkingMode({required bool isEnabled}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveEnableThinkingMode',
        args: <String, Object?>{'isEnabled': isEnabled},
      ),
    );
  }

  /// Saves one feature toggle value.
  Future<void> saveFeatureToggle({required String featureKey, required bool isEnabled}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveFeatureToggle',
        args: <String, Object?>{'featureKey': featureKey, 'isEnabled': isEnabled},
      ),
    );
  }

  /// Saves the thinking quality level.
  Future<void> saveThinkingQualityLevel({required int level}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveThinkingQualityLevel',
        args: <String, Object?>{'level': level},
      ),
    );
  }

  /// Saves the memory auto-update toggle.
  Future<void> saveEnableMemoryAutoUpdate({required bool isEnabled}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveEnableMemoryAutoUpdate',
        args: <String, Object?>{'isEnabled': isEnabled},
      ),
    );
  }

  /// Saves the global tools toggle.
  Future<void> saveEnableTools({required bool isEnabled}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveEnableTools',
        args: <String, Object?>{'isEnabled': isEnabled},
      ),
    );
  }

  /// Saves prompt visibility for one tool.
  Future<void> saveToolPromptVisibility({required String toolName, required bool isVisible}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveToolPromptVisibility',
        args: <String, Object?>{'toolName': toolName, 'isVisible': isVisible},
      ),
    );
  }

  /// Replaces the full tool prompt visibility map.
  Future<void> saveToolPromptVisibilityMap({required Map<String, bool> visibilityMap}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveToolPromptVisibilityMap',
        args: <String, Object?>{'visibilityMap': visibilityMap.map((key, value) => MapEntry(key, value))},
      ),
    );
  }

  /// Reads the full tool prompt visibility map.
  Future<Map<String, bool>> getToolPromptVisibilityMap() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPromptVisibilityMap',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as bool));
  }

  /// Saves whether streaming output should be disabled.
  Future<void> saveDisableStreamOutput({required bool isDisabled}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveDisableStreamOutput',
        args: <String, Object?>{'isDisabled': isDisabled},
      ),
    );
  }

  /// Saves whether user preference descriptions should be hidden from prompts.
  Future<void> saveDisableUserPreferenceDescription({required bool isDisabled}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveDisableUserPreferenceDescription',
        args: <String, Object?>{'isDisabled': isDisabled},
      ),
    );
  }

  /// Saves image and media history turn limits.
  Future<void> updateMediaHistorySettings({required int maxImageHistoryUserTurns, required int maxMediaHistoryUserTurns}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateMediaHistorySettings',
        args: <String, Object?>{'maxImageHistoryUserTurns': maxImageHistoryUserTurns, 'maxMediaHistoryUserTurns': maxMediaHistoryUserTurns},
      ),
    );
  }

  /// Saves the MCP startup timeout in seconds.
  Future<void> saveMcpStartupTimeoutSeconds({required int seconds}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveMcpStartupTimeoutSeconds',
        args: <String, Object?>{'seconds': seconds},
      ),
    );
  }

  /// Saves the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
  Future<void> saveToolPkgPreHookTimeoutSeconds({required int seconds}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveToolPkgPreHookTimeoutSeconds',
        args: <String, Object?>{'seconds': seconds},
      ),
    );
  }

  /// Reads the MCP startup timeout in seconds.
  Future<int> getMcpStartupTimeoutSeconds() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getMcpStartupTimeoutSeconds',
        args: const <String, Object?>{},
      ),
    );
    return value as int;
  }

  /// Reads the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
  Future<int> getToolPkgPreHookTimeoutSeconds() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgPreHookTimeoutSeconds',
        args: const <String, Object?>{},
      ),
    );
    return value as int;
  }

  /// Updates thinking settings in one edit transaction.
  Future<void> updateThinkingSettings({required bool? enableThinkingMode, required int? thinkingQualityLevel}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateThinkingSettings',
        args: <String, Object?>{'enableThinkingMode': enableThinkingMode, 'thinkingQualityLevel': thinkingQualityLevel},
      ),
    );
  }

}

class GeneratedPreferencesCharacterCardManagerCoreProxy {
  const GeneratedPreferencesCharacterCardManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Returns the ordered list of character card ids as a preference flow.
  Future<List<String>> characterCardListFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'characterCardListFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Returns the ordered list of character card ids as a preference flow.
  Stream<List<String>> characterCardListFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'characterCardListFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => item as String).toList(growable: false));
  }

  /// Returns the active character card id as a preference flow.
  Future<String?> observeActiveCharacterCardIdSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'observeActiveCharacterCardId',
        args: const <String, Object?>{},
      ),
    );
    return event.value == null ? null : event.value as String;
  }

  /// Returns the active character card id as a preference flow.
  Stream<String?> observeActiveCharacterCardIdChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'observeActiveCharacterCardId', args: const <String, Object?>{}))
        .map((event) => event.value == null ? null : event.value as String);
  }

  /// Returns a character card preference flow for one card id.
  Future<CharacterCard> getCharacterCardFlowSnapshot({required String id}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'getCharacterCardFlow',
        args: <String, Object?>{'id': id},
      ),
    );
    return CharacterCard.fromJson(event.value as Map<String, Object?>);
  }

  /// Returns a character card preference flow for one card id.
  Stream<CharacterCard> getCharacterCardFlowChanges({required String id}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'getCharacterCardFlow', args: <String, Object?>{'id': id}))
        .map((event) => CharacterCard.fromJson(event.value as Map<String, Object?>));
  }

  /// Reads one character card snapshot by id.
  Future<CharacterCard> getCharacterCard({required String id}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCharacterCard',
        args: <String, Object?>{'id': id},
      ),
    );
    return CharacterCard.fromJson(value as Map<String, Object?>);
  }

  /// Creates a character card and returns the stored id.
  Future<String> createCharacterCard({required CharacterCard card}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createCharacterCard',
        args: <String, Object?>{'card': card.toJson()},
      ),
    );
    return value as String;
  }

  /// Updates all stored fields for an existing character card.
  Future<void> updateCharacterCard({required CharacterCard card}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateCharacterCard',
        args: <String, Object?>{'card': card.toJson()},
      ),
    );
  }

  /// Deletes a non-default character card and clears it from the active selection.
  Future<void> deleteCharacterCard({required String id}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteCharacterCard',
        args: <String, Object?>{'id': id},
      ),
    );
  }

  /// Sets the active character card id used by active prompt resolution.
  Future<void> setActiveCharacterCard({required String id}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setActiveCharacterCard',
        args: <String, Object?>{'id': id},
      ),
    );
  }

  /// Clears the active character card selection.
  Future<void> clearActiveCharacterCard() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearActiveCharacterCard',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Returns every character card sorted with the default card first.
  Future<List<CharacterCard>> getAllCharacterCards() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllCharacterCards',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => CharacterCard.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Finds a character card by its display name.
  Future<CharacterCard?> findCharacterCardByName({required String name}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'findCharacterCardByName',
        args: <String, Object?>{'name': name},
      ),
    );
    return value == null ? null : CharacterCard.fromJson(value as Map<String, Object?>);
  }

  /// Ensures the default character card exists in preferences.
  Future<void> initializeIfNeeded() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeIfNeeded',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Recreates the default character card with the built-in default content.
  Future<void> resetDefaultCharacterCard() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resetDefaultCharacterCard',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Combines character card content and attached tags into a prompt string.
  Future<String> combinePrompts({required String characterCardId, required List<String> additionalTagIds, required PromptFunctionType promptFunctionType}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'combinePrompts',
        args: <String, Object?>{'characterCardId': characterCardId, 'additionalTagIds': additionalTagIds.map((item) => item).toList(growable: false), 'promptFunctionType': promptFunctionType.toJson()},
      ),
    );
    return value as String;
  }

  /// Exports all character cards and prompt tags as backup JSON.
  Future<String> exportAllCharacterCardsToBackupContent() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exportAllCharacterCardsToBackupContent',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Imports character cards and prompt tags from backup JSON.
  Future<CharacterCardImportResult> importAllCharacterCardsFromBackupContent({required String jsonContent}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importAllCharacterCardsFromBackupContent',
        args: <String, Object?>{'jsonContent': jsonContent},
      ),
    );
    return CharacterCardImportResult.fromJson(value as Map<String, Object?>);
  }

  /// Creates a character card from Tavern card JSON.
  Future<String> createCharacterCardFromTavernJson({required String jsonString}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createCharacterCardFromTavernJson',
        args: <String, Object?>{'jsonString': jsonString},
      ),
    );
    return value as String;
  }

  /// Exports one character card as Tavern-compatible JSON.
  Future<String> exportCharacterCardToTavernJson({required String characterCardId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exportCharacterCardToTavernJson',
        args: <String, Object?>{'characterCardId': characterCardId},
      ),
    );
    return value as String;
  }

}

class GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy {
  const GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

}

class GeneratedPreferencesCharacterGroupCardManagerCoreProxy {
  const GeneratedPreferencesCharacterGroupCardManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Observes the ordered list of stored character group identifiers.
  Future<List<String>> characterGroupCardListFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'characterGroupCardListFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Observes the ordered list of stored character group identifiers.
  Stream<List<String>> characterGroupCardListFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'characterGroupCardListFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => item as String).toList(growable: false));
  }

  /// Observes the currently selected character group identifier.
  Future<String?> observeActiveCharacterGroupIdSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'observeActiveCharacterGroupId',
        args: const <String, Object?>{},
      ),
    );
    return event.value == null ? null : event.value as String;
  }

  /// Observes the currently selected character group identifier.
  Stream<String?> observeActiveCharacterGroupIdChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'observeActiveCharacterGroupId', args: const <String, Object?>{}))
        .map((event) => event.value == null ? null : event.value as String);
  }

  /// Observes all stored character groups sorted by most recent update time.
  Future<List<CharacterGroupCard>> allCharacterGroupCardsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'allCharacterGroupCardsFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => CharacterGroupCard.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Observes all stored character groups sorted by most recent update time.
  Stream<List<CharacterGroupCard>> allCharacterGroupCardsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'allCharacterGroupCardsFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => CharacterGroupCard.fromJson(item as Map<String, Object?>)).toList(growable: false));
  }

  /// Observes one character group by identifier.
  Future<CharacterGroupCard?> getCharacterGroupCardFlowSnapshot({required String id}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'getCharacterGroupCardFlow',
        args: <String, Object?>{'id': id},
      ),
    );
    return event.value == null ? null : CharacterGroupCard.fromJson(event.value as Map<String, Object?>);
  }

  /// Observes one character group by identifier.
  Stream<CharacterGroupCard?> getCharacterGroupCardFlowChanges({required String id}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'getCharacterGroupCardFlow', args: <String, Object?>{'id': id}))
        .map((event) => event.value == null ? null : CharacterGroupCard.fromJson(event.value as Map<String, Object?>));
  }

  /// Observes the full character group selected as active.
  Future<CharacterGroupCard?> activeCharacterGroupCardFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'activeCharacterGroupCardFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value == null ? null : CharacterGroupCard.fromJson(event.value as Map<String, Object?>);
  }

  /// Observes the full character group selected as active.
  Stream<CharacterGroupCard?> activeCharacterGroupCardFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'activeCharacterGroupCardFlow', args: const <String, Object?>{}))
        .map((event) => event.value == null ? null : CharacterGroupCard.fromJson(event.value as Map<String, Object?>));
  }

  /// Creates a character group and assigns timestamps plus an id when required.
  Future<String> createCharacterGroupCard({required CharacterGroupCard group}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createCharacterGroupCard',
        args: <String, Object?>{'group': group.toJson()},
      ),
    );
    return value as String;
  }

  /// Updates an existing character group and refreshes its update timestamp.
  Future<void> updateCharacterGroupCard({required CharacterGroupCard group}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateCharacterGroupCard',
        args: <String, Object?>{'group': group.toJson()},
      ),
    );
  }

  /// Deletes a character group and clears it from active selection when selected.
  Future<void> deleteCharacterGroupCard({required String groupId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteCharacterGroupCard',
        args: <String, Object?>{'groupId': groupId},
      ),
    );
  }

  /// Selects a character group as active or clears the active selection.
  Future<void> setActiveCharacterGroupCard({required String? groupId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setActiveCharacterGroupCard',
        args: <String, Object?>{'groupId': groupId},
      ),
    );
  }

  /// Reads one character group by identifier.
  Future<CharacterGroupCard?> getCharacterGroupCard({required String groupId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCharacterGroupCard',
        args: <String, Object?>{'groupId': groupId},
      ),
    );
    return value == null ? null : CharacterGroupCard.fromJson(value as Map<String, Object?>);
  }

  /// Reads every stored character group sorted by most recent update time.
  Future<List<CharacterGroupCard>> getAllCharacterGroupCards() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllCharacterGroupCards',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => CharacterGroupCard.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Creates the persisted character group list key when the store is empty.
  Future<void> initializeIfNeeded() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeIfNeeded',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Duplicates a character group and returns the newly created group id.
  Future<String?> duplicateCharacterGroupCard({required String sourceGroupId, required String? newName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'duplicateCharacterGroupCard',
        args: <String, Object?>{'sourceGroupId': sourceGroupId, 'newName': newName},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Copies external bindings owned by one character group to another group.
  Future<void> cloneBindingsFromCharacterGroup({required String sourceGroupId, required String targetGroupId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'cloneBindingsFromCharacterGroup',
        args: <String, Object?>{'_sourceGroupId': sourceGroupId, '_targetGroupId': targetGroupId},
      ),
    );
  }

  /// Exports all character groups as pretty-printed backup JSON.
  Future<String> exportAllCharacterGroupsToBackupContent() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exportAllCharacterGroupsToBackupContent',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Imports character groups from backup JSON and reports create, update, and skip counts.
  Future<CharacterGroupImportResult> importAllCharacterGroupsFromBackupContent({required String jsonContent}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importAllCharacterGroupsFromBackupContent',
        args: <String, Object?>{'jsonContent': jsonContent},
      ),
    );
    return CharacterGroupImportResult.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedPreferencesEnvPreferencesCoreProxy {
  const GeneratedPreferencesEnvPreferencesCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Reads an environment value from persistent preferences or the process environment.
  Future<String?> getEnv({required String key}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getEnv',
        args: <String, Object?>{'key': key},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Stores an environment value in persistent preferences.
  Future<void> setEnv({required String key, required String value}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setEnv',
        args: <String, Object?>{'key': key, 'value': value},
      ),
    );
  }

  /// Removes a persisted environment value by key.
  Future<void> removeEnv({required String key}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removeEnv',
        args: <String, Object?>{'key': key},
      ),
    );
  }

  /// Returns all persisted environment values keyed by variable name.
  Future<Map<String, String>> getAllEnv() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllEnv',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String));
  }

  /// Replaces the persisted environment values with the provided map.
  Future<void> setAllEnv({required Map<String, String> variables}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setAllEnv',
        args: <String, Object?>{'variables': variables.map((key, value) => MapEntry(key, value))},
      ),
    );
  }

}

class GeneratedPreferencesFunctionalConfigManagerCoreProxy {
  const GeneratedPreferencesFunctionalConfigManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Ensures model configuration exists and seeds function bindings when empty.
  Future<void> initializeIfNeeded() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeIfNeeded',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Observes the full mapping from runtime functions to provider model bindings.
  Future<Map<FunctionType, FunctionModelBinding>> functionModelBindingFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'functionModelBindingFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(FunctionType.fromJson(key), FunctionModelBinding.fromJson(value as Map<String, Object?>)));
  }

  /// Observes the full mapping from runtime functions to provider model bindings.
  Stream<Map<FunctionType, FunctionModelBinding>> functionModelBindingFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'functionModelBindingFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as Map<Object?, Object?>).map((key, value) => MapEntry(FunctionType.fromJson(key), FunctionModelBinding.fromJson(value as Map<String, Object?>))));
  }

  /// Saves the complete function-to-model binding map.
  Future<void> saveFunctionModelBinding({required Map<FunctionType, FunctionModelBinding> binding}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveFunctionModelBinding',
        args: <String, Object?>{'binding': binding.map((key, value) => MapEntry(key.toJson(), value.toJson()))},
      ),
    );
  }

  /// Reads the model binding currently assigned to one runtime function.
  Future<FunctionModelBinding> getModelBindingForFunction({required FunctionType functionType}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getModelBindingForFunction',
        args: <String, Object?>{'functionType': functionType.toJson()},
      ),
    );
    return FunctionModelBinding.fromJson(value as Map<String, Object?>);
  }

  /// Assigns one runtime function to the specified provider and model.
  Future<void> setModelForFunction({required FunctionType functionType, required String providerId, required String modelId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setModelForFunction',
        args: <String, Object?>{'functionType': functionType.toJson(), 'providerId': providerId, 'modelId': modelId},
      ),
    );
  }

  /// Restores one runtime function to the default provider and model.
  Future<void> resetFunctionConfig({required FunctionType functionType}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resetFunctionConfig',
        args: <String, Object?>{'functionType': functionType.toJson()},
      ),
    );
  }

  /// Restores every runtime function to the default provider and model map.
  Future<void> resetAllFunctionConfigs() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resetAllFunctionConfigs',
        args: const <String, Object?>{},
      ),
    );
  }

}

class GeneratedPreferencesGitHubAuthPreferencesCoreProxy {
  const GeneratedPreferencesGitHubAuthPreferencesCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Saves a GitHub login session including token metadata and user profile data.
  Future<void> saveAuthInfo({required String accessToken, required String tokenType, required Object? userInfo, required String? grantedScope}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveAuthInfo',
        args: <String, Object?>{'accessToken': accessToken, 'tokenType': tokenType, 'userInfo': userInfo, 'grantedScope': grantedScope},
      ),
    );
  }

  /// Updates the saved GitHub access token without replacing the saved user profile.
  Future<void> updateAccessToken({required String accessToken, required String tokenType, required String? grantedScope}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateAccessToken',
        args: <String, Object?>{'accessToken': accessToken, 'tokenType': tokenType, 'grantedScope': grantedScope},
      ),
    );
  }

  /// Returns the saved GitHub access token when the stored session is current.
  Future<String?> getCurrentAccessToken() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCurrentAccessToken',
        args: const <String, Object?>{},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Returns the saved GitHub user profile when the stored session is current.
  Future<CoreDataPreferencesGitHubAuthPreferencesGitHubUser?> getCurrentUserInfo() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCurrentUserInfo',
        args: const <String, Object?>{},
      ),
    );
    return value == null ? null : CoreDataPreferencesGitHubAuthPreferencesGitHubUser.fromJson(value as Map<String, Object?>);
  }

  /// Reports whether the saved GitHub authentication session is usable.
  Future<bool> isLoggedIn() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isLoggedIn',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

  /// Clears the saved GitHub authentication session.
  Future<void> logout() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'logout',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Builds the HTTP Authorization header for the current GitHub token.
  Future<String?> getAuthorizationHeader() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAuthorizationHeader',
        args: const <String, Object?>{},
      ),
    );
    return value == null ? null : value as String;
  }

}

class GeneratedPreferencesModelConfigManagerCoreProxy {
  const GeneratedPreferencesModelConfigManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Ensures the default API provider and fixed local provider exist.
  Future<void> initializeIfNeeded() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeIfNeeded',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Observes the ordered provider id list.
  Future<List<String>> providerListFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'providerListFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Observes the ordered provider id list.
  Stream<List<String>> providerListFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'providerListFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => item as String).toList(growable: false));
  }

  /// Reads the ordered provider id list.
  Future<List<String>> getProviderIds() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getProviderIds',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Observes all provider profiles in persisted order.
  Future<List<ProviderProfile>> getProviderProfilesFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'getProviderProfilesFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => ProviderProfile.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Observes all provider profiles in persisted order.
  Stream<List<ProviderProfile>> getProviderProfilesFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'getProviderProfilesFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => ProviderProfile.fromJson(item as Map<String, Object?>)).toList(growable: false));
  }

  /// Reads all provider profiles in persisted order.
  Future<List<ProviderProfile>> getProviderProfiles() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getProviderProfiles',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => ProviderProfile.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Reads one provider profile by id.
  Future<ProviderProfile> getProviderProfile({required String providerId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getProviderProfile',
        args: <String, Object?>{'providerId': providerId},
      ),
    );
    return ProviderProfile.fromJson(value as Map<String, Object?>);
  }

  /// Builds summary rows for every configured provider model.
  Future<List<ProviderModelSummary>> getAllModelSummaries() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllModelSummaries',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => ProviderModelSummary.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Reads provider catalog entries from the built-in catalog.
  Future<List<ProviderCatalogEntry>> getProviderCatalogEntries() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getProviderCatalogEntries',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => ProviderCatalogEntry.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Creates a new provider profile and stores it in provider order.
  Future<String> createProvider({required String name, required String providerTypeId, required String endpoint}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createProvider',
        args: <String, Object?>{'name': name, 'providerTypeId': providerTypeId, 'endpoint': endpoint},
      ),
    );
    return value as String;
  }

  /// Replaces an existing provider profile after validation.
  Future<ProviderProfile> updateProviderProfile({required ProviderProfile provider}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateProviderProfile',
        args: <String, Object?>{'provider': provider.toJson()},
      ),
    );
    return ProviderProfile.fromJson(value as Map<String, Object?>);
  }

  /// Replaces the default provider profile and preserves its id.
  Future<ProviderProfile> replaceDefaultProviderProfile({required ProviderProfile provider}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'replaceDefaultProviderProfile',
        args: <String, Object?>{'provider': provider.toJson()},
      ),
    );
    return ProviderProfile.fromJson(value as Map<String, Object?>);
  }

  /// Deletes one provider profile and removes it from provider order.
  Future<void> deleteProvider({required String providerId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteProvider',
        args: <String, Object?>{'providerId': providerId},
      ),
    );
  }

  /// Creates a model profile under an existing provider.
  Future<String> createProviderModel({required String providerId, required String modelId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createProviderModel',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId},
      ),
    );
    return value as String;
  }

  /// Fetches provider models available for import into a provider profile.
  Future<List<AvailableProviderModel>> getAvailableProviderModels({required String providerId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAvailableProviderModels',
        args: <String, Object?>{'providerId': providerId},
      ),
    );
    return (value as List<Object?>).map((item) => AvailableProviderModel.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Adds a provider model using catalog or remote availability metadata.
  Future<String> addProviderModelFromAvailable({required String providerId, required String modelId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'addProviderModelFromAvailable',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId},
      ),
    );
    return value as String;
  }

  /// Replaces a model profile under an existing provider.
  Future<ModelProfile> updateModelProfile({required String providerId, required ModelProfile model}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateModelProfile',
        args: <String, Object?>{'providerId': providerId, 'model': model.toJson()},
      ),
    );
    return ModelProfile.fromJson(value as Map<String, Object?>);
  }

  /// Deletes one model profile from a provider.
  Future<void> deleteModel({required String providerId, required String modelId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteModel',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId},
      ),
    );
  }

  /// Reads one model profile from a provider.
  Future<ModelProfile> getModelProfile({required String providerId, required String modelId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getModelProfile',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId},
      ),
    );
    return ModelProfile.fromJson(value as Map<String, Object?>);
  }

  /// Resolves provider and model profile data into a runtime model config.
  Future<ResolvedModelConfig> getResolvedModelConfig({required String providerId, required String modelId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getResolvedModelConfig',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId},
      ),
    );
    return ResolvedModelConfig.fromJson(value as Map<String, Object?>);
  }

  /// Reads model parameters for one provider/model pair.
  Future<List<Object?>> getModelParametersForModel({required String providerId, required String modelId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getModelParametersForModel',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId},
      ),
    );
    return (value as List<Object?>).map((item) => item).toList(growable: false);
  }

  /// Updates model parameters for one provider/model pair.
  Future<void> updateParametersForModel({required String providerId, required String modelId, required List<Object?> parameters}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateParametersForModel',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId, 'parameters': parameters.map((item) => item).toList(growable: false)},
      ),
    );
  }

  /// Updates model capabilities for one provider/model pair.
  Future<ModelProfile> updateCapabilitiesForModel({required String providerId, required String modelId, required ModelCapabilities capabilities}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateCapabilitiesForModel',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId, 'capabilities': capabilities.toJson()},
      ),
    );
    return ModelProfile.fromJson(value as Map<String, Object?>);
  }

  /// Updates model context settings for one provider/model pair.
  Future<ModelProfile> updateContextForModel({required String providerId, required String modelId, required ModelContextSpec context}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateContextForModel',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId, 'context': context.toJson()},
      ),
    );
    return ModelProfile.fromJson(value as Map<String, Object?>);
  }

  /// Updates built-in tool settings for one provider/model pair.
  Future<ModelProfile> updateBuiltinToolsForModel({required String providerId, required String modelId, required List<ModelBuiltinTool> builtinTools}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateBuiltinToolsForModel',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId, 'builtinTools': builtinTools.map((item) => item.toJson()).toList(growable: false)},
      ),
    );
    return ModelProfile.fromJson(value as Map<String, Object?>);
  }

  /// Updates request settings for one provider/model pair.
  Future<ModelProfile> updateRequestForModel({required String providerId, required String modelId, required ModelRequestSpec request}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateRequestForModel',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId, 'request': request.toJson()},
      ),
    );
    return ModelProfile.fromJson(value as Map<String, Object?>);
  }

  /// Updates summary settings for one provider/model pair.
  Future<ModelProfile> updateSummaryForModel({required String providerId, required String modelId, required ModelSummarySettings summary}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateSummaryForModel',
        args: <String, Object?>{'providerId': providerId, 'modelId': modelId, 'summary': summary.toJson()},
      ),
    );
    return ModelProfile.fromJson(value as Map<String, Object?>);
  }

  /// Exports all provider profiles as formatted JSON.
  Future<String> exportAllProviders() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exportAllProviders',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

}

class GeneratedPreferencesPreferenceStorageManagerCoreProxy {
  const GeneratedPreferencesPreferenceStorageManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Reads one value from a named custom preference file.
  Future<String?> getPreference({required String fileName, required String key}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPreference',
        args: <String, Object?>{'fileName': fileName, 'key': key},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Reads selected values from a named custom preference file.
  Future<Map<String, String>> getPreferences({required String fileName, required List<String> keys}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPreferences',
        args: <String, Object?>{'fileName': fileName, 'keys': keys.map((item) => item).toList(growable: false)},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String));
  }

  /// Writes one value to a named custom preference file.
  Future<void> setPreference({required String fileName, required String key, required String value}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setPreference',
        args: <String, Object?>{'fileName': fileName, 'key': key, 'value': value},
      ),
    );
  }

  /// Writes multiple values to a named custom preference file.
  Future<void> setPreferences({required String fileName, required Map<String, String> values}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setPreferences',
        args: <String, Object?>{'fileName': fileName, 'values': values.map((key, value) => MapEntry(key, value))},
      ),
    );
  }

  /// Removes one key from a named custom preference file.
  Future<void> removePreference({required String fileName, required String key}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removePreference',
        args: <String, Object?>{'fileName': fileName, 'key': key},
      ),
    );
  }

  /// Removes selected keys from a named custom preference file.
  Future<void> removePreferences({required String fileName, required List<String> keys}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removePreferences',
        args: <String, Object?>{'fileName': fileName, 'keys': keys.map((item) => item).toList(growable: false)},
      ),
    );
  }

  /// Removes every key from a named custom preference file.
  Future<void> clearPreferences({required String fileName}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearPreferences',
        args: <String, Object?>{'fileName': fileName},
      ),
    );
  }

}

class GeneratedPreferencesPromptTagManagerCoreProxy {
  const GeneratedPreferencesPromptTagManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Observes the ordered list of prompt tag identifiers.
  Future<List<String>> tagListFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'tagListFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Observes the ordered list of prompt tag identifiers.
  Stream<List<String>> tagListFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'tagListFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => item as String).toList(growable: false));
  }

  /// Observes all prompt tags sorted by most recent update time.
  Future<List<PromptTag>> allTagsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'allTagsFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => PromptTag.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Observes all prompt tags sorted by most recent update time.
  Stream<List<PromptTag>> allTagsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'allTagsFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => PromptTag.fromJson(item as Map<String, Object?>)).toList(growable: false));
  }

  /// Observes one prompt tag by identifier.
  Future<PromptTag> getPromptTagFlowSnapshot({required String id}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'getPromptTagFlow',
        args: <String, Object?>{'id': id},
      ),
    );
    return PromptTag.fromJson(event.value as Map<String, Object?>);
  }

  /// Observes one prompt tag by identifier.
  Stream<PromptTag> getPromptTagFlowChanges({required String id}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'getPromptTagFlow', args: <String, Object?>{'id': id}))
        .map((event) => PromptTag.fromJson(event.value as Map<String, Object?>));
  }

  /// Creates a prompt tag and returns its generated identifier.
  Future<String> createPromptTag({required String name, required String description, required String promptContent, required TagType tagType}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createPromptTag',
        args: <String, Object?>{'name': name, 'description': description, 'promptContent': promptContent, 'tagType': tagType.toJson()},
      ),
    );
    return value as String;
  }

  /// Updates selected prompt tag fields and refreshes the update timestamp.
  Future<void> updatePromptTag({required String id, required String? name, required String? description, required String? promptContent, required TagType? tagType}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updatePromptTag',
        args: <String, Object?>{'id': id, 'name': name, 'description': description, 'promptContent': promptContent, 'tagType': tagType?.toJson()},
      ),
    );
  }

  /// Deletes a prompt tag and removes its stored fields.
  Future<void> deletePromptTag({required String id}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deletePromptTag',
        args: <String, Object?>{'id': id},
      ),
    );
  }

  /// Reads all prompt tags sorted by most recent update time.
  Future<List<PromptTag>> getAllTags() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllTags',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => PromptTag.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Reads prompt tags that match the requested tag type.
  Future<List<PromptTag>> getTagsByType({required TagType tagType}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getTagsByType',
        args: <String, Object?>{'tagType': tagType.toJson()},
      ),
    );
    return (value as List<Object?>).map((item) => PromptTag.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Finds a prompt tag whose prompt content matches the provided content.
  Future<PromptTag?> findTagWithSameContent({required String promptContent}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'findTagWithSameContent',
        args: <String, Object?>{'promptContent': promptContent},
      ),
    );
    return value == null ? null : PromptTag.fromJson(value as Map<String, Object?>);
  }

  /// Creates a prompt tag or returns an existing tag with identical prompt content.
  Future<String> createOrReusePromptTag({required String name, required String description, required String promptContent, required TagType tagType}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createOrReusePromptTag',
        args: <String, Object?>{'name': name, 'description': description, 'promptContent': promptContent, 'tagType': tagType.toJson()},
      ),
    );
    return value as String;
  }

  /// Removes built-in prompt tag records created by older preference schemas.
  Future<void> removeLegacyBuiltInTags() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removeLegacyBuiltInTags',
        args: const <String, Object?>{},
      ),
    );
  }

}

class GeneratedPreferencesSharedMemoryStoreManagerCoreProxy {
  const GeneratedPreferencesSharedMemoryStoreManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Observes the ordered list of shared memory store identifiers.
  Future<List<String>> sharedMemoryStoreListFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'sharedMemoryStoreListFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Observes the ordered list of shared memory store identifiers.
  Stream<List<String>> sharedMemoryStoreListFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'sharedMemoryStoreListFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => item as String).toList(growable: false));
  }

  /// Reads every shared memory store in persisted list order.
  Future<List<SharedMemoryStore>> getAllSharedMemoryStores() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllSharedMemoryStores',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => SharedMemoryStore.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Reads one shared memory store by identifier.
  Future<SharedMemoryStore> getSharedMemoryStore({required String id}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getSharedMemoryStore',
        args: <String, Object?>{'id': id},
      ),
    );
    return SharedMemoryStore.fromJson(value as Map<String, Object?>);
  }

  /// Observes one shared memory store by identifier.
  Future<SharedMemoryStore> getSharedMemoryStoreFlowSnapshot({required String id}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'getSharedMemoryStoreFlow',
        args: <String, Object?>{'id': id},
      ),
    );
    return SharedMemoryStore.fromJson(event.value as Map<String, Object?>);
  }

  /// Observes one shared memory store by identifier.
  Stream<SharedMemoryStore> getSharedMemoryStoreFlowChanges({required String id}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'getSharedMemoryStoreFlow', args: <String, Object?>{'id': id}))
        .map((event) => SharedMemoryStore.fromJson(event.value as Map<String, Object?>));
  }

  /// Creates a shared memory store with a generated identifier.
  Future<SharedMemoryStore> createSharedMemoryStore({required String name}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createSharedMemoryStore',
        args: <String, Object?>{'name': name},
      ),
    );
    return SharedMemoryStore.fromJson(value as Map<String, Object?>);
  }

  /// Creates a shared memory store with an explicit identifier.
  Future<SharedMemoryStore> createSharedMemoryStoreWithId({required String id, required String name}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createSharedMemoryStoreWithId',
        args: <String, Object?>{'id': id, 'name': name},
      ),
    );
    return SharedMemoryStore.fromJson(value as Map<String, Object?>);
  }

  /// Renames a shared memory store and refreshes its update timestamp.
  Future<SharedMemoryStore> renameSharedMemoryStore({required String id, required String name}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'renameSharedMemoryStore',
        args: <String, Object?>{'id': id, 'name': name},
      ),
    );
    return SharedMemoryStore.fromJson(value as Map<String, Object?>);
  }

  /// Deletes a shared memory store and returns whether an entry was removed.
  Future<bool> deleteSharedMemoryStore({required String id}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteSharedMemoryStore',
        args: <String, Object?>{'id': id},
      ),
    );
    return value as bool;
  }

}

class GeneratedPreferencesSkillVisibilityPreferencesCoreProxy {
  const GeneratedPreferencesSkillVisibilityPreferencesCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Reads whether a skill is visible to AI tool selection.
  Future<bool> isSkillVisibleToAi({required String skillName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isSkillVisibleToAi',
        args: <String, Object?>{'skillName': skillName},
      ),
    );
    return value as bool;
  }

  /// Persists whether a skill is visible to AI tool selection.
  Future<void> setSkillVisibleToAi({required String skillName, required bool visible}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setSkillVisibleToAi',
        args: <String, Object?>{'skillName': skillName, 'visible': visible},
      ),
    );
  }

}

class GeneratedPreferencesSttConfigManagerCoreProxy {
  const GeneratedPreferencesSttConfigManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Observes the ordered list of STT configuration ids.
  Future<List<String>> sttConfigListFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'sttConfigListFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Observes the ordered list of STT configuration ids.
  Stream<List<String>> sttConfigListFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'sttConfigListFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => item as String).toList(growable: false));
  }

  /// Reads every configured STT provider profile.
  Future<List<SttConfig>> getAllSttConfigs() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllSttConfigs',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => SttConfig.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Reads one STT configuration by id.
  Future<SttConfig> getSttConfig({required String id}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getSttConfig',
        args: <String, Object?>{'id': id},
      ),
    );
    return SttConfig.fromJson(value as Map<String, Object?>);
  }

  /// Reads the current STT configuration id.
  Future<String> getCurrentSttConfigId() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCurrentSttConfigId',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Reads the selected STT configuration id when a selection exists.
  Future<String?> getSelectedSttConfigId() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getSelectedSttConfigId',
        args: const <String, Object?>{},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Reads the current STT configuration.
  Future<SttConfig> getCurrentSttConfig() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCurrentSttConfig',
        args: const <String, Object?>{},
      ),
    );
    return SttConfig.fromJson(value as Map<String, Object?>);
  }

  /// Selects the current STT configuration by id.
  Future<String> setCurrentSttConfigId({required String id}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setCurrentSttConfigId',
        args: <String, Object?>{'id': id},
      ),
    );
    return value as String;
  }

  /// Creates and persists one STT provider configuration.
  Future<SttConfig> createSttConfig({required SttConfig config}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createSttConfig',
        args: <String, Object?>{'config': config.toJson()},
      ),
    );
    return SttConfig.fromJson(value as Map<String, Object?>);
  }

  /// Updates one existing STT provider configuration.
  Future<SttConfig> updateSttConfig({required SttConfig config}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateSttConfig',
        args: <String, Object?>{'config': config.toJson()},
      ),
    );
    return SttConfig.fromJson(value as Map<String, Object?>);
  }

  /// Deletes one STT provider configuration.
  Future<bool> deleteSttConfig({required String id}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteSttConfig',
        args: <String, Object?>{'id': id},
      ),
    );
    return value as bool;
  }

  /// Returns every built-in STT provider catalog entry.
  Future<List<SttProviderCatalogEntry>> getProviderCatalogEntries() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getProviderCatalogEntries',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => SttProviderCatalogEntry.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns models exposed by one STT provider type.
  Future<List<AvailableSttModel>> getAvailableSttModels({required String providerTypeId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAvailableSttModels',
        args: <String, Object?>{'providerTypeId': providerTypeId},
      ),
    );
    return (value as List<Object?>).map((item) => AvailableSttModel.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

}

class GeneratedPreferencesTtsConfigManagerCoreProxy {
  const GeneratedPreferencesTtsConfigManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Observes the ordered list of text-to-speech configuration identifiers.
  Future<List<String>> ttsConfigListFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'ttsConfigListFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Observes the ordered list of text-to-speech configuration identifiers.
  Stream<List<String>> ttsConfigListFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'ttsConfigListFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => item as String).toList(growable: false));
  }

  /// Observes the identifier of the currently selected text-to-speech configuration.
  Future<String> currentTtsConfigIdFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'currentTtsConfigIdFlow',
        args: const <String, Object?>{},
      ),
    );
    return event.value as String;
  }

  /// Observes the identifier of the currently selected text-to-speech configuration.
  Stream<String> currentTtsConfigIdFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'currentTtsConfigIdFlow', args: const <String, Object?>{}))
        .map((event) => event.value as String);
  }

  /// Reads the currently selected text-to-speech configuration identifier.
  Future<String> getCurrentTtsConfigId() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCurrentTtsConfigId',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Reads the currently selected text-to-speech configuration.
  Future<TtsConfig> getCurrentTtsConfig() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCurrentTtsConfig',
        args: const <String, Object?>{},
      ),
    );
    return TtsConfig.fromJson(value as Map<String, Object?>);
  }

  /// Selects the active text-to-speech configuration by identifier.
  Future<String> setCurrentTtsConfigId({required String id}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setCurrentTtsConfigId',
        args: <String, Object?>{'id': id},
      ),
    );
    return value as String;
  }

  /// Reads every configured text-to-speech provider or voice profile.
  Future<List<TtsConfig>> getAllTtsConfigs() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllTtsConfigs',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => TtsConfig.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Reads the built-in catalog of supported text-to-speech provider presets.
  Future<List<TtsProviderCatalogEntry>> getProviderCatalogEntries() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getProviderCatalogEntries',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => TtsProviderCatalogEntry.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Reads one text-to-speech configuration by identifier.
  Future<TtsConfig> getTtsConfig({required String id}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getTtsConfig',
        args: <String, Object?>{'id': id},
      ),
    );
    return TtsConfig.fromJson(value as Map<String, Object?>);
  }

  /// Observes one text-to-speech configuration by identifier.
  Future<TtsConfig> getTtsConfigFlowSnapshot({required String id}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'getTtsConfigFlow',
        args: <String, Object?>{'id': id},
      ),
    );
    return TtsConfig.fromJson(event.value as Map<String, Object?>);
  }

  /// Observes one text-to-speech configuration by identifier.
  Stream<TtsConfig> getTtsConfigFlowChanges({required String id}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'getTtsConfigFlow', args: <String, Object?>{'id': id}))
        .map((event) => TtsConfig.fromJson(event.value as Map<String, Object?>));
  }

  /// Creates a text-to-speech configuration and assigns store timestamps.
  Future<TtsConfig> createTtsConfig({required TtsConfig config}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createTtsConfig',
        args: <String, Object?>{'config': config.toJson()},
      ),
    );
    return TtsConfig.fromJson(value as Map<String, Object?>);
  }

  /// Lists available voices reported by a provider configuration.
  Future<List<AvailableTtsVoice>> getAvailableTtsVoices({required String providerConfigId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAvailableTtsVoices',
        args: <String, Object?>{'providerConfigId': providerConfigId},
      ),
    );
    return (value as List<Object?>).map((item) => AvailableTtsVoice.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Creates a voice configuration from one provider-reported voice entry.
  Future<TtsConfig> addTtsVoiceFromAvailable({required String providerConfigId, required String model, required String voice}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'addTtsVoiceFromAvailable',
        args: <String, Object?>{'providerConfigId': providerConfigId, 'model': model, 'voice': voice},
      ),
    );
    return TtsConfig.fromJson(value as Map<String, Object?>);
  }

  /// Creates a voice configuration from custom model and voice values.
  Future<TtsConfig> createCustomTtsVoice({required String providerConfigId, required String model, required String voice}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createCustomTtsVoice',
        args: <String, Object?>{'providerConfigId': providerConfigId, 'model': model, 'voice': voice},
      ),
    );
    return TtsConfig.fromJson(value as Map<String, Object?>);
  }

  /// Updates a text-to-speech configuration and preserves its creation timestamp.
  Future<TtsConfig> updateTtsConfig({required TtsConfig config}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateTtsConfig',
        args: <String, Object?>{'config': config.toJson()},
      ),
    );
    return TtsConfig.fromJson(value as Map<String, Object?>);
  }

  /// Deletes a text-to-speech configuration and reports whether it existed.
  Future<bool> deleteTtsConfig({required String id}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteTtsConfig',
        args: <String, Object?>{'id': id},
      ),
    );
    return value as bool;
  }

}

class GeneratedPreferencesUserPreferencesManagerCoreProxy {
  const GeneratedPreferencesUserPreferencesManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Initializes the user preference store.
  Future<void> initializeIfNeeded({required String defaultProfileName}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initializeIfNeeded',
        args: <String, Object?>{'_defaultProfileName': defaultProfileName},
      ),
    );
  }

  /// Observes the selected application language code.
  Future<String> appLanguageSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'appLanguage',
        args: const <String, Object?>{},
      ),
    );
    return event.value as String;
  }

  /// Observes the selected application language code.
  Stream<String> appLanguageChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'appLanguage', args: const <String, Object?>{}))
        .map((event) => event.value as String);
  }

  /// Saves the selected application language code.
  Future<void> saveAppLanguage({required String languageCode}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveAppLanguage',
        args: <String, Object?>{'languageCode': languageCode},
      ),
    );
  }

  /// Reads the selected application language code.
  Future<String> getCurrentLanguage() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCurrentLanguage',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

}

class GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy {
  const GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Starts a new mirrored conversation round and clears mirrored content.
  Future<void> startNewRound() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'startNewRound',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Replaces the mirrored content for the current conversation round.
  Future<void> updateContent({required String content}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateContent',
        args: <String, Object?>{'content': content},
      ),
    );
  }

  /// Appends text to the mirrored content for the current conversation round.
  Future<void> appendContent({required String content}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'appendContent',
        args: <String, Object?>{'content': content},
      ),
    );
  }

  /// Returns the mirrored content formatted for display.
  Future<String> getDisplayContent() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getDisplayContent',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the mirrored content for the current conversation round.
  Future<String> getCurrentRoundContent() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCurrentRoundContent',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

}

class GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy {
  const GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Resets the manager to the beginning of a new conversation.
  Future<void> initializeNewConversation() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'initialize_new_conversation',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Advances to the next conversation round and returns its index.
  Future<int> startNewRound() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'start_new_round',
        args: const <String, Object?>{},
      ),
    );
    return value as int;
  }

  /// Returns the current conversation round index.
  Future<int> getCurrentRound() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_current_round',
        args: const <String, Object?>{},
      ),
    );
    return value as int;
  }

  /// Clears the raw content for the current conversation round.
  Future<void> clearContent() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clear_content',
        args: const <String, Object?>{},
      ),
    );
  }

}

class GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy {
  const GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Reports whether the streaming JSON-to-XML converter has an open parameter tag.
  Future<bool> hasUnfinishedParam() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'hasUnfinishedParam',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

}

class GeneratedProvidersMarketStatsApiServiceCoreProxy {
  const GeneratedProvidersMarketStatsApiServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Loads the marketplace manifest.
  Future<MarketManifest> getManifest() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_manifest',
        args: const <String, Object?>{},
      ),
    );
    return MarketManifest.fromJson(value as Map<String, Object?>);
  }

  /// Loads a paginated all-entry marketplace list.
  Future<MarketListPage> getListPage({required String sort, required int page}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_list_page',
        args: <String, Object?>{'sort': sort, 'page': page},
      ),
    );
    return MarketListPage.fromJson(value as Map<String, Object?>);
  }

  /// Loads a paginated marketplace list filtered by type.
  Future<MarketListPage> getTypePage({required String type, required String sort, required int page}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_type_page',
        args: <String, Object?>{'r#type': type, 'sort': sort, 'page': page},
      ),
    );
    return MarketListPage.fromJson(value as Map<String, Object?>);
  }

  /// Loads a paginated marketplace list filtered by category.
  Future<MarketListPage> getCategoryPage({required String categoryId, required String sort, required int page}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_category_page',
        args: <String, Object?>{'category_id': categoryId, 'sort': sort, 'page': page},
      ),
    );
    return MarketListPage.fromJson(value as Map<String, Object?>);
  }

  /// Loads a paginated marketplace list filtered by type and category.
  Future<MarketListPage> getTypeCategoryPage({required String type, required String categoryId, required String sort, required int page}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_type_category_page',
        args: <String, Object?>{'r#type': type, 'category_id': categoryId, 'sort': sort, 'page': page},
      ),
    );
    return MarketListPage.fromJson(value as Map<String, Object?>);
  }

  /// Loads one static entries shard.
  Future<MarketEntriesShard> getEntriesShard({required String shard}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_entries_shard',
        args: <String, Object?>{'shard': shard},
      ),
    );
    return MarketEntriesShard.fromJson(value as Map<String, Object?>);
  }

  /// Loads one marketplace entry by id.
  Future<MarketEntrySummary> getEntryById({required String entryId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_entry_by_id',
        args: <String, Object?>{'entry_id': entryId},
      ),
    );
    return MarketEntrySummary.fromJson(value as Map<String, Object?>);
  }

  /// Loads a paginated comment page for an entry.
  Future<MarketCommentPage> getCommentsPage({required String entryId, required int page}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_comments_page',
        args: <String, Object?>{'entry_id': entryId, 'page': page},
      ),
    );
    return MarketCommentPage.fromJson(value as Map<String, Object?>);
  }

  /// Builds aggregate stats for marketplace entries of one type.
  Future<MarketTypeStatsResponse> getStats({required String type}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_stats',
        args: <String, Object?>{'r#type': type},
      ),
    );
    return MarketTypeStatsResponse.fromJson(value as Map<String, Object?>);
  }

  /// Returns the GitHub user associated with the configured token.
  Future<CoreOperitProvidersMarketMarketStatsApiServiceGitHubUser> getCurrentGithubUser() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_current_github_user',
        args: const <String, Object?>{},
      ),
    );
    return CoreOperitProvidersMarketMarketStatsApiServiceGitHubUser.fromJson(value as Map<String, Object?>);
  }

  /// Exchanges the configured GitHub token for a marketplace session.
  Future<MarketAuthInfo> exchangeGithubTokenForMarketSession() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exchange_github_token_for_market_session',
        args: const <String, Object?>{},
      ),
    );
    return MarketAuthInfo.fromJson(value as Map<String, Object?>);
  }

  /// Creates a comment on a marketplace entry.
  Future<String> createEntryComment({required String entryId, required String body}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'create_entry_comment',
        args: <String, Object?>{'entry_id': entryId, 'body': body},
      ),
    );
    return value as String;
  }

  /// Edits an existing marketplace comment.
  Future<void> editEntryComment({required String commentId, required String body}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'edit_entry_comment',
        args: <String, Object?>{'comment_id': commentId, 'body': body},
      ),
    );
  }

  /// Deletes an existing marketplace comment.
  Future<void> deleteEntryComment({required String commentId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'delete_entry_comment',
        args: <String, Object?>{'comment_id': commentId},
      ),
    );
  }

  /// Adds a positive reaction to a marketplace entry.
  Future<void> createEntryReaction({required String entryId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'create_entry_reaction',
        args: <String, Object?>{'entry_id': entryId},
      ),
    );
  }

  /// Loads authenticated marketplace notifications.
  Future<MarketNotificationsResponse> getNotifications({required int limit, required int offset, required String? since}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_notifications',
        args: <String, Object?>{'limit': limit, 'offset': offset, 'since': since},
      ),
    );
    return MarketNotificationsResponse.fromJson(value as Map<String, Object?>);
  }

  /// Loads entries owned by the authenticated user.
  Future<MarketMyEntriesResponse> getMyEntries() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_my_entries',
        args: const <String, Object?>{},
      ),
    );
    return MarketMyEntriesResponse.fromJson(value as Map<String, Object?>);
  }

  /// Loads authenticated user entries filtered by type.
  Future<MarketMyEntriesResponse> getMyEntriesByType({required String type}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_my_entries_by_type',
        args: <String, Object?>{'r#type': type},
      ),
    );
    return MarketMyEntriesResponse.fromJson(value as Map<String, Object?>);
  }

  /// Loads full data for one entry submitted by the authenticated publisher.
  Future<MarketEntrySummary> getMyEntryDetail({required String entryId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'get_my_entry_detail',
        args: <String, Object?>{'entry_id': entryId},
      ),
    );
    return MarketEntrySummary.fromJson(value as Map<String, Object?>);
  }

  /// Publishes a marketplace artifact entry.
  Future<MarketPublishResponse> publishArtifact({required String type, required String title, required String description, required String detail, required String categoryId, required bool allowPublicUpdates, required String version, required String formatVer, required String minAppVer, required String? maxAppVer, required String? changelog, required String projectId, required String runtimePackageId, required String assetKind, required String assetUrl, required String ghOwner, required String ghRepo, required String ghReleaseTag, required String assetName, required String sha256}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'publish_artifact',
        args: <String, Object?>{'r#type': type, 'title': title, 'description': description, 'detail': detail, 'category_id': categoryId, 'allow_public_updates': allowPublicUpdates, 'version': version, 'format_ver': formatVer, 'min_app_ver': minAppVer, 'max_app_ver': maxAppVer, 'changelog': changelog, 'project_id': projectId, 'runtime_package_id': runtimePackageId, 'asset_kind': assetKind, 'asset_url': assetUrl, 'gh_owner': ghOwner, 'gh_repo': ghRepo, 'gh_release_tag': ghReleaseTag, 'asset_name': assetName, 'sha256': sha256},
      ),
    );
    return MarketPublishResponse.fromJson(value as Map<String, Object?>);
  }

  /// Publishes a marketplace entry backed by a GitHub repository.
  Future<MarketPublishResponse> publishRepoEntry({required String type, required String title, required String description, required String detail, required String categoryId, required bool allowPublicUpdates, required String sourceUrl, required String refType, required String refName, required String installConfig, required String version, required String formatVer, required String minAppVer, required String? maxAppVer, required String? changelog}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'publish_repo_entry',
        args: <String, Object?>{'r#type': type, 'title': title, 'description': description, 'detail': detail, 'category_id': categoryId, 'allow_public_updates': allowPublicUpdates, 'source_url': sourceUrl, 'ref_type': refType, 'ref_name': refName, 'install_config': installConfig, 'version': version, 'format_ver': formatVer, 'min_app_ver': minAppVer, 'max_app_ver': maxAppVer, 'changelog': changelog},
      ),
    );
    return MarketPublishResponse.fromJson(value as Map<String, Object?>);
  }

  /// Updates mutable metadata for a marketplace entry.
  Future<MarketEntryUpdateResponse> updateEntry({required String entryId, required String? title, required String? description, required String? detail, required String? categoryId, required bool? allowPublicUpdates}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'update_entry',
        args: <String, Object?>{'entry_id': entryId, 'title': title, 'description': description, 'detail': detail, 'category_id': categoryId, 'allow_public_updates': allowPublicUpdates},
      ),
    );
    return MarketEntryUpdateResponse.fromJson(value as Map<String, Object?>);
  }

  /// Publishes a new artifact version for an existing marketplace entry.
  Future<MarketPublishResponse> publishArtifactVersion({required String entryId, required String version, required String formatVer, required String minAppVer, required String? maxAppVer, required String? changelog, required String projectId, required String runtimePackageId, required String assetKind, required String assetUrl, required String ghOwner, required String ghRepo, required String ghReleaseTag, required String assetName, required String sha256, required String? entryTitle, required String? entryDescription, required String? entryDetail, required String? entryCategoryId, required bool? entryAllowPublicUpdates}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'publish_artifact_version',
        args: <String, Object?>{'entry_id': entryId, 'version': version, 'format_ver': formatVer, 'min_app_ver': minAppVer, 'max_app_ver': maxAppVer, 'changelog': changelog, 'project_id': projectId, 'runtime_package_id': runtimePackageId, 'asset_kind': assetKind, 'asset_url': assetUrl, 'gh_owner': ghOwner, 'gh_repo': ghRepo, 'gh_release_tag': ghReleaseTag, 'asset_name': assetName, 'sha256': sha256, 'entry_title': entryTitle, 'entry_description': entryDescription, 'entry_detail': entryDetail, 'entry_category_id': entryCategoryId, 'entry_allow_public_updates': entryAllowPublicUpdates},
      ),
    );
    return MarketPublishResponse.fromJson(value as Map<String, Object?>);
  }

  /// Publishes a new repository version for an existing marketplace entry.
  Future<MarketPublishResponse> publishRepoVersion({required String entryId, required String version, required String formatVer, required String minAppVer, required String? maxAppVer, required String? changelog, required String refType, required String refName, required String installConfig, required String? entryTitle, required String? entryDescription, required String? entryDetail, required String? entryCategoryId, required bool? entryAllowPublicUpdates}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'publish_repo_version',
        args: <String, Object?>{'entry_id': entryId, 'version': version, 'format_ver': formatVer, 'min_app_ver': minAppVer, 'max_app_ver': maxAppVer, 'changelog': changelog, 'ref_type': refType, 'ref_name': refName, 'install_config': installConfig, 'entry_title': entryTitle, 'entry_description': entryDescription, 'entry_detail': entryDetail, 'entry_category_id': entryCategoryId, 'entry_allow_public_updates': entryAllowPublicUpdates},
      ),
    );
    return MarketPublishResponse.fromJson(value as Map<String, Object?>);
  }

  /// Downloads a marketplace asset by id.
  Future<Uint8List> downloadAsset({required String assetId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'download_asset',
        args: <String, Object?>{'asset_id': assetId},
      ),
    );
    return value as Uint8List;
  }

}

class GeneratedRepositoryChatHistoryManagerCoreProxy {
  const GeneratedRepositoryChatHistoryManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

}

class GeneratedRepositoryMemoryRepositoryCoreProxy {
  const GeneratedRepositoryMemoryRepositoryCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Searches memories by lexical relevance, folder, and creation-time filters.
  Future<List<Memory>> searchMemories({required String query, required String? folderPath, required double relevanceThreshold, required int? createdAtStartMs, required int? createdAtEndMs}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'searchMemories',
        args: <String, Object?>{'query': query, 'folderPath': folderPath, 'relevanceThreshold': relevanceThreshold, 'createdAtStartMs': createdAtStartMs, 'createdAtEndMs': createdAtEndMs},
      ),
    );
    return (value as List<Object?>).map((item) => Memory.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Finds the first memory with the exact normalized title.
  Future<Memory?> findMemoryByTitle({required String title}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'findMemoryByTitle',
        args: <String, Object?>{'title': title},
      ),
    );
    return value == null ? null : Memory.fromJson(value as Map<String, Object?>);
  }

  /// Finds all memories with the exact normalized title.
  Future<List<Memory>> findMemoriesByTitle({required String title}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'findMemoriesByTitle',
        args: <String, Object?>{'title': title},
      ),
    );
    return (value as List<Object?>).map((item) => Memory.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns memories stored under one normalized folder path.
  Future<List<Memory>> getMemoriesByFolderPath({required String folderPath}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getMemoriesByFolderPath',
        args: <String, Object?>{'folderPath': folderPath},
      ),
    );
    return (value as List<Object?>).map((item) => Memory.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Creates and stores a new memory with generated identity and timestamps.
  Future<Memory> createMemory({required String title, required String content, required String contentType, required String source, required String folderPath, required List<String>? tags}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createMemory',
        args: <String, Object?>{'title': title, 'content': content, 'contentType': contentType, 'source': source, 'folderPath': folderPath, 'tags': tags?.map((item) => item).toList(growable: false)},
      ),
    );
    return Memory.fromJson(value as Map<String, Object?>);
  }

  /// Saves a memory after normalizing identity, timestamps, and folder path.
  Future<Memory> saveMemory({required Memory memory}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'saveMemory',
        args: <String, Object?>{'memory': memory.toJson()},
      ),
    );
    return Memory.fromJson(value as Map<String, Object?>);
  }

  /// Adds a tag to a memory and returns the updated memory.
  Future<Memory?> addTagToMemory({required int memoryId, required String tagName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'addTagToMemory',
        args: <String, Object?>{'memoryId': memoryId, 'tagName': tagName},
      ),
    );
    return value == null ? null : Memory.fromJson(value as Map<String, Object?>);
  }

  /// Merges memories with matching source titles into one primary memory.
  Future<Memory?> mergeMemories({required List<String> sourceTitles, required String newTitle, required String newContent, required List<String> newTags, required String folderPath}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'mergeMemories',
        args: <String, Object?>{'sourceTitles': sourceTitles.map((item) => item).toList(growable: false), 'newTitle': newTitle, 'newContent': newContent, 'newTags': newTags.map((item) => item).toList(growable: false), 'folderPath': folderPath},
      ),
    );
    return value == null ? null : Memory.fromJson(value as Map<String, Object?>);
  }

  /// Updates one memory's content, scoring metadata, folder, and tags.
  Future<Memory> updateMemory({required int memoryId, required String newTitle, required String newContent, required String newContentType, required String newSource, required double newCredibility, required double newImportance, required String? newFolderPath, required List<String>? newTags}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateMemory',
        args: <String, Object?>{'memoryId': memoryId, 'newTitle': newTitle, 'newContent': newContent, 'newContentType': newContentType, 'newSource': newSource, 'newCredibility': newCredibility, 'newImportance': newImportance, 'newFolderPath': newFolderPath, 'newTags': newTags?.map((item) => item).toList(growable: false)},
      ),
    );
    return Memory.fromJson(value as Map<String, Object?>);
  }

  /// Deletes one memory and removes links that reference it.
  Future<bool> deleteMemory({required int memoryId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteMemory',
        args: <String, Object?>{'memoryId': memoryId},
      ),
    );
    return value as bool;
  }

  /// Moves selected memories into a normalized target folder.
  Future<bool> moveMemoriesToFolder({required Object? memoryIds, required String targetFolderPath}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'moveMemoriesToFolder',
        args: <String, Object?>{'memoryIds': memoryIds, 'targetFolderPath': targetFolderPath},
      ),
    );
    return value as bool;
  }

  /// Creates a typed weighted link between two existing memories.
  Future<MemoryLink> linkMemories({required int sourceMemoryId, required int targetMemoryId, required String type, required double weight, required String description}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'linkMemories',
        args: <String, Object?>{'sourceMemoryId': sourceMemoryId, 'targetMemoryId': targetMemoryId, 'type_': type, 'weight': weight, 'description': description},
      ),
    );
    return MemoryLink.fromJson(value as Map<String, Object?>);
  }

  /// Deletes one memory link by id.
  Future<bool> deleteLink({required int linkId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteLink',
        args: <String, Object?>{'linkId': linkId},
      ),
    );
    return value as bool;
  }

  /// Lists every folder path currently used by stored memories.
  Future<List<String>> getAllFolderPaths() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllFolderPaths',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Builds the current memory graph after removing dangling links.
  Future<MemoryGraph> getMemoryGraph() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getMemoryGraph',
        args: const <String, Object?>{},
      ),
    );
    return MemoryGraph.fromJson(value as Map<String, Object?>);
  }

  /// Exports user memories and their internal links as a portable JSON document.
  Future<String> exportMemoriesToJson() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exportMemoriesToJson',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Imports memories and links from a portable JSON document.
  Future<MemoryImportResult> importMemoriesFromJson({required String jsonString, required ImportStrategy strategy}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importMemoriesFromJson',
        args: <String, Object?>{'jsonString': jsonString, 'strategy': strategy.toJson()},
      ),
    );
    return MemoryImportResult.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedRepositoryRuntimeStorageRepositoryCoreProxy {
  const GeneratedRepositoryRuntimeStorageRepositoryCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Reads a UTF-8 text object from runtime storage.
  Future<String?> readText({required String path}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'readText',
        args: <String, Object?>{'path': path},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Reads a runtime storage object and returns its base64 representation.
  Future<String?> readBase64({required String path}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'readBase64',
        args: <String, Object?>{'path': path},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Writes UTF-8 text content to runtime storage.
  Future<void> writeText({required String path, required String content}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'writeText',
        args: <String, Object?>{'path': path, 'content': content},
      ),
    );
  }

  /// Decodes base64 content and writes the bytes to runtime storage.
  Future<void> writeBase64({required String path, required String base64Content}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'writeBase64',
        args: <String, Object?>{'path': path, 'base64Content': base64Content},
      ),
    );
  }

  /// Returns the runtime storage path for browser bookmark data.
  Future<String> webSessionBrowserBookmarksPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'webSessionBrowserBookmarksPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage path for browser history data.
  Future<String> webSessionBrowserHistoryPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'webSessionBrowserHistoryPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage path for browser download metadata.
  Future<String> webSessionBrowserDownloadsPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'webSessionBrowserDownloadsPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage directory path for downloaded browser files.
  Future<String> webSessionBrowserDownloadFilesDirPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'webSessionBrowserDownloadFilesDirPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage directory path for imported theme assets.
  Future<String> themeAssetsDirPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'themeAssetsDirPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage directory for generated share images.
  Future<String> shareImageDirPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'shareImageDirPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage directory for exported share images.
  Future<String> shareImageExportsDirPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'shareImageExportsDirPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage directory for staged workspace videos.
  Future<String> workspaceVideoDirPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'workspaceVideoDirPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage directory for Compose DSL selected files.
  Future<String> composeDslWebViewFilesDirPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'composeDslWebViewFilesDirPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage directory for materialized Link Access web assets.
  Future<String> linkAccessWebAssetsDirPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'linkAccessWebAssetsDirPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage path for the client log.
  Future<String> clientLogPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clientLogPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns the runtime storage path for userscript state data.
  Future<String> webSessionUserscriptsStatePath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'webSessionUserscriptsStatePath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

}

class GeneratedRepositoryUsageStatisticsStoreCoreProxy {
  const GeneratedRepositoryUsageStatisticsStoreCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Reads all recorded provider model requests ordered by creation time.
  Future<List<UsageRequestRecord>> getAllRequestRecords() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllRequestRecords',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => UsageRequestRecord.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Deletes every recorded provider model request.
  Future<void> clearAllRequestRecords() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearAllRequestRecords',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Records token usage for one provider model request.
  Future<UsageRequestRecord> recordProviderModelRequest({required String providerModel, required FunctionType functionType, required UsageRequestSource source, required String? chatId, required int inputTokens, required int outputTokens, required int cachedInputTokens}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'recordProviderModelRequest',
        args: <String, Object?>{'providerModel': providerModel, 'functionType': functionType.toJson(), 'source': source.toJson(), 'chatId': chatId, 'inputTokens': inputTokens, 'outputTokens': outputTokens, 'cachedInputTokens': cachedInputTokens},
      ),
    );
    return UsageRequestRecord.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedRuntimeRemoteLinkServiceCoreProxy {
  const GeneratedRuntimeRemoteLinkServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Selects the local Core as the destination for incoming Link requests.
  Future<void> selectLocalRoute() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'selectLocalRoute',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Selects one paired remote session as the destination for incoming Link requests.
  Future<void> selectPairedRemoteRoute({required String name}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'selectPairedRemoteRoute',
        args: <String, Object?>{'name': name},
      ),
    );
  }

  /// Returns the persisted runtime route, initializing it to the local runtime when absent.
  Future<LinkAccessRoutingConfig> currentRoute() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'currentRoute',
        args: const <String, Object?>{},
      ),
    );
    return LinkAccessRoutingConfig.fromJson(value as Map<String, Object?>);
  }

  /// Returns every paired remote session owned by the local runtime.
  Future<Map<String, PairedRemoteSessionRecord>> pairedRemoteSessions() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'pairedRemoteSessions',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, PairedRemoteSessionRecord.fromJson(value as Map<String, Object?>)));
  }

  /// Enables or disables automatic synchronization for one paired remote session.
  Future<LinkAccessAutoSyncConfig> setPairedRemoteAutoSync({required String name, required bool enabled}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setPairedRemoteAutoSync',
        args: <String, Object?>{'name': name, 'enabled': enabled},
      ),
    );
    return LinkAccessAutoSyncConfig.fromJson(value as Map<String, Object?>);
  }

  /// Returns the current automatic synchronization configuration owned by this runtime.
  Future<LinkAccessAutoSyncConfig> autoSyncConfig() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'autoSyncConfig',
        args: const <String, Object?>{},
      ),
    );
    return LinkAccessAutoSyncConfig.fromJson(value as Map<String, Object?>);
  }

  /// Starts the singleton runtime-owned automatic synchronization worker.
  Future<void> startAutoSync() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'startAutoSync',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Discovers Link-enabled runtimes through the native runtime transport.
  Future<List<RuntimeRemoteDiscoveredDevice>> discoverPairedRemotes({required int timeoutMs}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'discoverPairedRemotes',
        args: <String, Object?>{'timeoutMs': timeoutMs},
      ),
    );
    return (value as List<Object?>).map((item) => RuntimeRemoteDiscoveredDevice.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Runs automatic synchronization for every enabled paired remote endpoint.
  Future<void> syncConfiguredPairedRemotes() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'syncConfiguredPairedRemotes',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Removes one paired remote while preserving a valid route and auto-sync configuration.
  Future<void> removePairedRemote({required String name}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removePairedRemote',
        args: <String, Object?>{'name': name},
      ),
    );
  }

  /// Starts a runtime-owned outbound pairing and stores its confidential client state.
  Future<RuntimeRemotePairStartResult> startPairedRemote({required String baseUrl, required String tokenHash, required RemoteDeviceInfo clientDeviceInfo}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'startPairedRemote',
        args: <String, Object?>{'baseUrl': baseUrl, 'tokenHash': tokenHash, 'clientDeviceInfo': clientDeviceInfo.toJson()},
      ),
    );
    return RuntimeRemotePairStartResult.fromJson(value as Map<String, Object?>);
  }

  /// Completes a runtime-owned outbound pairing, stores its named session, and selects it.
  Future<PairedRemoteSessionRecord> finishPairedRemote({required String pairingId, required String pairingCode, required String name}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'finishPairedRemote',
        args: <String, Object?>{'pairingId': pairingId, 'pairingCode': pairingCode, 'name': name},
      ),
    );
    return PairedRemoteSessionRecord.fromJson(value as Map<String, Object?>);
  }

  /// Probes one named paired remote and verifies the persisted remote identity.
  Future<RuntimeRemoteProbeResult> probePairedRemote({required String name}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'probePairedRemote',
        args: <String, Object?>{'name': name},
      ),
    );
    return RuntimeRemoteProbeResult.fromJson(value as Map<String, Object?>);
  }

  /// Runs one complete two-way sync transaction with a named paired remote runtime.
  Future<RuntimeRemoteSyncResult> syncPairedRemote({required String name, required int limit}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'syncPairedRemote',
        args: <String, Object?>{'name': name, 'limit': limit},
      ),
    );
    return RuntimeRemoteSyncResult.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedServicesArchiveTransferManagerCoreProxy {
  const GeneratedServicesArchiveTransferManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Creates one host-owned target for a streamed archive upload of an exact byte length.
  Future<String> beginArchiveUpload({required int expectedByteLength}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'beginArchiveUpload',
        args: <String, Object?>{'expectedByteLength': expectedByteLength},
      ),
    );
    return value as String;
  }

  /// Writes one complete caller-owned input stream into an archive upload.
  Future<void> writeArchiveUpload({required String archiveId, required Stream<Uint8List> bytes}) async {
    final sink = await bridge.push(
      CorePushRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'writeArchiveUpload',
        args: <String, Object?>{'archiveId': archiveId},
      ),
    );
    try {
      await for (final item in bytes) {
        await sink.add(item);
      }
    } finally {
      await sink.close();
    }
  }

  /// Seals one uploaded archive after verifying its caller-declared byte length.
  Future<StagedArchive> completeArchiveUpload({required String archiveId, required int expectedByteLength}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'completeArchiveUpload',
        args: <String, Object?>{'archiveId': archiveId, 'expectedByteLength': expectedByteLength},
      ),
    );
    return StagedArchive.fromJson(value as Map<String, Object?>);
  }

  /// Removes one host-owned archive upload regardless of its sealed state.
  Future<void> discardArchiveUpload({required String archiveId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'discardArchiveUpload',
        args: <String, Object?>{'archiveId': archiveId},
      ),
    );
  }

}

class GeneratedServicesGitHubOAuthBrokerServiceCoreProxy {
  const GeneratedServicesGitHubOAuthBrokerServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Starts one broker transaction after an application prepares its completion destination.
  Future<GitHubOAuthBrokerLoginStart> startLogin({required String completionRedirectUri}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'startLogin',
        args: <String, Object?>{'completionRedirectUri': completionRedirectUri},
      ),
    );
    return GitHubOAuthBrokerLoginStart.fromJson(value as Map<String, Object?>);
  }

  /// Validates one browser completion URL, claims its transaction once, and persists the session.
  Future<GitHubOAuthBrokerLoginResult> completeLogin({required GitHubOAuthBrokerLoginCompletion completion}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'completeLogin',
        args: <String, Object?>{'completion': completion.toJson()},
      ),
    );
    return GitHubOAuthBrokerLoginResult.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedServicesLocalModelServiceCoreProxy {
  const GeneratedServicesLocalModelServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Returns model catalog entries with installed model and engine state.
  Future<List<LocalModelCatalogStatus>> getCatalogStatus() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getCatalogStatus',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => LocalModelCatalogStatus.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns the shared local model and engine registry snapshot.
  Future<LocalModelRegistrySnapshot> getRegistry() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getRegistry',
        args: const <String, Object?>{},
      ),
    );
    return LocalModelRegistrySnapshot.fromJson(value as Map<String, Object?>);
  }

  /// Returns the current local engine platform target.
  Future<LocalPlatformTarget> getPlatformTarget() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPlatformTarget',
        args: const <String, Object?>{},
      ),
    );
    return LocalPlatformTarget.fromJson(value as Map<String, Object?>);
  }

  /// Installs one model and its exact platform engine dependency.
  Future<LocalModelBundleInstallResult> installModel({required String modelId, required String version}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'installModel',
        args: <String, Object?>{'modelId': modelId, 'version': version},
      ),
    );
    return LocalModelBundleInstallResult.fromJson(value as Map<String, Object?>);
  }

  /// Returns every installation operation retained by this runtime process.
  Future<List<LocalModelInstallStatus>> getInstallStatuses() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getInstallStatuses',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => LocalModelInstallStatus.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns one installation operation by exact model id and version.
  Future<LocalModelInstallStatus?> getInstallStatus({required String modelId, required String version}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getInstallStatus',
        args: <String, Object?>{'modelId': modelId, 'version': version},
      ),
    );
    return value == null ? null : LocalModelInstallStatus.fromJson(value as Map<String, Object?>);
  }

  /// Requests cancellation for one active model and engine installation.
  Future<LocalModelInstallStatus> cancelInstall({required String modelId, required String version}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'cancelInstall',
        args: <String, Object?>{'modelId': modelId, 'version': version},
      ),
    );
    return LocalModelInstallStatus.fromJson(value as Map<String, Object?>);
  }

  /// Verifies one installed model and its exact platform engine.
  Future<LocalModelCatalogStatus> verifyModel({required String modelId, required String version}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'verifyModel',
        args: <String, Object?>{'modelId': modelId, 'version': version},
      ),
    );
    return LocalModelCatalogStatus.fromJson(value as Map<String, Object?>);
  }

  /// Deletes one installed or retained local model download from the asset repository.
  Future<void> deleteModel({required String modelId, required String version}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteModel',
        args: <String, Object?>{'modelId': modelId, 'version': version},
      ),
    );
  }

  /// Deletes one engine target that is not required by installed models.
  Future<void> deleteEngine({required String engineId, required String version}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteEngine',
        args: <String, Object?>{'engineId': engineId, 'version': version},
      ),
    );
  }

}

class GeneratedServicesLocalProviderServiceCoreProxy {
  const GeneratedServicesLocalProviderServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Transcribes audio with one exact LOCAL_MODEL provider model.
  Future<LocalSttResponse> transcribeAudio({required LocalModelSelection model, required String audioPath, required String? language}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'transcribeAudio',
        args: <String, Object?>{'model': model.toJson(), 'audioPath': audioPath, 'language': language},
      ),
    );
    return LocalSttResponse.fromJson(value as Map<String, Object?>);
  }

  /// Synthesizes WAV audio with one exact LOCAL_MODEL provider model.
  Future<LocalTtsResponse> synthesizeAudio({required LocalModelSelection model, required String text, required String voice, required double speed}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'synthesizeAudio',
        args: <String, Object?>{'model': model.toJson(), 'text': text, 'voice': voice, 'speed': speed},
      ),
    );
    return LocalTtsResponse.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedServicesRuntimeBrowserServiceCoreProxy {
  const GeneratedServicesRuntimeBrowserServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Lists browser sessions currently owned by the owner host.
  Future<List<RuntimeBrowserSessionInfo>> listBrowserSessions() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'listBrowserSessions',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => RuntimeBrowserSessionInfo.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Creates a browser session on the owner host.
  Future<RuntimeBrowserSessionInfo> createBrowserSession({required String initialUrl, required String? userAgent, required Map<String, String> headers}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createBrowserSession',
        args: <String, Object?>{'initialUrl': initialUrl, 'userAgent': userAgent, 'headers': headers.map((key, value) => MapEntry(key, value))},
      ),
    );
    return RuntimeBrowserSessionInfo.fromJson(value as Map<String, Object?>);
  }

  /// Updates browser session metadata on the owner host.
  Future<RuntimeBrowserSessionInfo> updateBrowserSession({required String sessionId, required String? userAgent, required Map<String, String> headers}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'updateBrowserSession',
        args: <String, Object?>{'sessionId': sessionId, 'userAgent': userAgent, 'headers': headers.map((key, value) => MapEntry(key, value))},
      ),
    );
    return RuntimeBrowserSessionInfo.fromJson(value as Map<String, Object?>);
  }

  /// Returns the shared event stream used by an attached browser controller.
  Future<RuntimeBrowserStreamEvent> browserSessionEventsSnapshot({required String sessionId}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'browserSessionEvents',
        args: <String, Object?>{'sessionId': sessionId},
      ),
    );
    return RuntimeBrowserStreamEvent.fromJson(event.value as Map<String, Object?>);
  }

  /// Returns the shared event stream used by an attached browser controller.
  Stream<RuntimeBrowserStreamEvent> browserSessionEventsChanges({required String sessionId}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'browserSessionEvents', args: <String, Object?>{'sessionId': sessionId}))
        .map((event) => RuntimeBrowserStreamEvent.fromJson(event.value as Map<String, Object?>));
  }

  /// Submits one semantic browser command to the owner host.
  Future<RuntimeBrowserCommandResult> submitBrowserCommand({required RuntimeBrowserCommand command}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'submitBrowserCommand',
        args: <String, Object?>{'command': command.toJson()},
      ),
    );
    return RuntimeBrowserCommandResult.fromJson(value as Map<String, Object?>);
  }

  /// Applies ordered compositor interactions supplied by one caller-owned input stream.
  Future<void> submitBrowserInteractions({required Stream<RuntimeBrowserCommand> commands}) async {
    final sink = await bridge.push(
      CorePushRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'submitBrowserInteractions',
        args: <String, Object?>{},
      ),
    );
    try {
      await for (final item in commands) {
        await sink.add(item);
      }
    } finally {
      await sink.close();
    }
  }

  /// Returns the latest browser session snapshot for a controller.
  Future<RuntimeBrowserSessionSnapshot> getBrowserSessionSnapshot({required String sessionId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getBrowserSessionSnapshot',
        args: <String, Object?>{'sessionId': sessionId},
      ),
    );
    return RuntimeBrowserSessionSnapshot.fromJson(value as Map<String, Object?>);
  }

  /// Publishes one browser session event from the owner host.
  Future<void> publishBrowserSessionEvent({required RuntimeBrowserSessionEvent event}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'publishBrowserSessionEvent',
        args: <String, Object?>{'event': event.toJson()},
      ),
    );
  }

  /// Explicitly closes a browser session on the owner host.
  Future<void> closeBrowserSession({required String sessionId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'closeBrowserSession',
        args: <String, Object?>{'sessionId': sessionId},
      ),
    );
  }

}

class GeneratedServicesRuntimeHostInfoServiceCoreProxy {
  const GeneratedServicesRuntimeHostInfoServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Returns the captured runtime host descriptor.
  Future<RuntimeHostDescriptor> runtimeHostDescriptor() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'runtimeHostDescriptor',
        args: const <String, Object?>{},
      ),
    );
    return RuntimeHostDescriptor.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedServicesRuntimeHostInteractionServiceCoreProxy {
  const GeneratedServicesRuntimeHostInteractionServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Sends one application notification through the active system-operation host.
  Future<void> sendSystemNotification({required String title, required String message, required String? chatId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'sendSystemNotification',
        args: <String, Object?>{'title': title, 'message': message, 'chatId': chatId},
      ),
    );
  }

  /// Responds to a pending owner-host interaction request.
  Future<void> respondOwnerHostInteraction({required String requestId, required RuntimeHostInteractionResponse response}) async {
    await bridge.callControl(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'respondOwnerHostInteraction',
        args: <String, Object?>{'requestId': requestId, 'response': response.toJson()},
      ),
    );
  }

  /// Acknowledges a non-blocking owner-host notification after it is consumed.
  Future<void> acknowledgeOwnerHostInteraction({required String requestId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'acknowledgeOwnerHostInteraction',
        args: <String, Object?>{'requestId': requestId},
      ),
    );
  }

  /// Creates an event stream for owner-host interaction requests of selected kinds.
  Future<RuntimeHostInteractionRequest> ownerHostInteractionEventsSnapshot({required List<RuntimeHostInteractionKind> kinds}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'ownerHostInteractionEvents',
        args: <String, Object?>{'kinds': kinds.map((item) => item.toJson()).toList(growable: false)},
      ),
    );
    return RuntimeHostInteractionRequest.fromJson(event.value as Map<String, Object?>);
  }

  /// Creates an event stream for owner-host interaction requests of selected kinds.
  Stream<RuntimeHostInteractionRequest> ownerHostInteractionEventsChanges({required List<RuntimeHostInteractionKind> kinds}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'ownerHostInteractionEvents', args: <String, Object?>{'kinds': kinds.map((item) => item.toJson()).toList(growable: false)}))
        .map((event) => RuntimeHostInteractionRequest.fromJson(event.value as Map<String, Object?>));
  }

}

class GeneratedServicesRuntimeTerminalServiceCoreProxy {
  const GeneratedServicesRuntimeTerminalServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Lists terminal sessions currently known by the host.
  Future<List<RuntimeTerminalSessionInfo>> listTerminalSessions() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'listTerminalSessions',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => RuntimeTerminalSessionInfo.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns a state flow of published terminal sessions.
  Future<List<RuntimeTerminalSessionInfo>> terminalSessionsFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'terminalSessionsFlow',
        args: const <String, Object?>{},
      ),
    );
    return (event.value as List<Object?>).map((item) => RuntimeTerminalSessionInfo.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns a state flow of published terminal sessions.
  Stream<List<RuntimeTerminalSessionInfo>> terminalSessionsFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'terminalSessionsFlow', args: const <String, Object?>{}))
        .map((event) => (event.value as List<Object?>).map((item) => RuntimeTerminalSessionInfo.fromJson(item as Map<String, Object?>)).toList(growable: false));
  }

  /// Returns the host-declared terminal type for manual PTY creation.
  Future<String> defaultTerminalType() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'defaultTerminalType',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns every terminal type that the active host exposes to users.
  Future<RuntimeTerminalInfo> terminalInfo() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'terminalInfo',
        args: const <String, Object?>{},
      ),
    );
    return RuntimeTerminalInfo.fromJson(value as Map<String, Object?>);
  }

  /// Starts a PTY terminal session and attaches its output stream.
  Future<String> startTerminalPty({required String sessionName, required String terminal, required String terminalType, required String workingDir, required int rows, required int cols}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'startTerminalPty',
        args: <String, Object?>{'sessionName': sessionName, 'terminal': terminal, 'terminalType': terminalType, 'workingDir': workingDir, 'rows': rows, 'cols': cols},
      ),
    );
    return value as String;
  }

  /// Returns the shared output stream for a PTY session.
  Future<Object?> terminalPtyOutputSnapshot({required String sessionId}) async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'terminalPtyOutput',
        args: <String, Object?>{'sessionId': sessionId},
      ),
    );
    return event.value;
  }

  /// Returns the shared output stream for a PTY session.
  Stream<Object?> terminalPtyOutputChanges({required String sessionId}) {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'terminalPtyOutput', args: <String, Object?>{'sessionId': sessionId}))
        .map((event) => event.value);
  }

  /// Writes base64-encoded bytes to a PTY session.
  Future<int> writeTerminalPty({required String sessionId, required String dataBase64}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'writeTerminalPty',
        args: <String, Object?>{'sessionId': sessionId, 'dataBase64': dataBase64},
      ),
    );
    return value as int;
  }

  /// Resizes a PTY session.
  Future<void> resizeTerminalPty({required String sessionId, required int rows, required int cols}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resizeTerminalPty',
        args: <String, Object?>{'sessionId': sessionId, 'rows': rows, 'cols': cols},
      ),
    );
  }

  /// Polls the exit code for a PTY session.
  Future<int?> pollTerminalPtyExit({required String sessionId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'pollTerminalPtyExit',
        args: <String, Object?>{'sessionId': sessionId},
      ),
    );
    return value == null ? null : value as int;
  }

  /// Closes a PTY session and removes its output stream.
  Future<void> closeTerminalPty({required String sessionId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'closeTerminalPty',
        args: <String, Object?>{'sessionId': sessionId},
      ),
    );
  }

  /// Sends text input to a terminal session.
  Future<int> inputTerminalSession({required String sessionId, required String input}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'inputTerminalSession',
        args: <String, Object?>{'sessionId': sessionId, 'input': input},
      ),
    );
    return value as int;
  }

  /// Reads the current screen contents for a terminal session.
  Future<RuntimeTerminalScreen> getTerminalSessionScreen({required String sessionId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getTerminalSessionScreen',
        args: <String, Object?>{'sessionId': sessionId},
      ),
    );
    return RuntimeTerminalScreen.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedServicesSnapshotImportManagerCoreProxy {
  const GeneratedServicesSnapshotImportManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Exports all raw runtime storage into a portable snapshot archive.
  Future<Uint8List> exportRawSnapshot() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'exportRawSnapshot',
        args: const <String, Object?>{},
      ),
    );
    return value as Uint8List;
  }

  /// Reads raw snapshot metadata from a sealed archive without changing runtime storage.
  Future<RawSnapshotManifest> inspectRawSnapshot({required StagedArchive archive}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'inspectRawSnapshot',
        args: <String, Object?>{'archive': archive.toJson()},
      ),
    );
    return RawSnapshotManifest.fromJson(value as Map<String, Object?>);
  }

  /// Restores one sealed raw snapshot after closing the active database handle.
  Future<void> restoreRawSnapshot({required StagedArchive archive}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'restoreRawSnapshot',
        args: <String, Object?>{'archive': archive.toJson()},
      ),
    );
  }

  /// Previews an Operit1 model-configuration snapshot from a sealed archive.
  Future<Operit1ModelConfigSnapshotPreview> inspectOperit1ModelConfigSnapshot({required StagedArchive archive}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'inspectOperit1ModelConfigSnapshot',
        args: <String, Object?>{'archive': archive.toJson()},
      ),
    );
    return Operit1ModelConfigSnapshotPreview.fromJson(value as Map<String, Object?>);
  }

  /// Previews an Operit1 full snapshot from a sealed archive.
  Future<Operit1SnapshotPreview> inspectOperit1Snapshot({required StagedArchive archive}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'inspectOperit1Snapshot',
        args: <String, Object?>{'archive': archive.toJson()},
      ),
    );
    return Operit1SnapshotPreview.fromJson(value as Map<String, Object?>);
  }

  /// Imports an Operit1 model configuration from a sealed archive into the selected profile.
  Future<Operit1ModelConfigImportResult> importOperit1ModelConfigSnapshot({required StagedArchive archive, required String configId, required String modelId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importOperit1ModelConfigSnapshot',
        args: <String, Object?>{'archive': archive.toJson(), 'configId': configId, 'modelId': modelId},
      ),
    );
    return Operit1ModelConfigImportResult.fromJson(value as Map<String, Object?>);
  }

  /// Imports one sealed Operit1 snapshot and publishes progress events.
  Future<Operit1SnapshotImportResult> importOperit1Snapshot({required StagedArchive archive}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importOperit1Snapshot',
        args: <String, Object?>{'archive': archive.toJson()},
      ),
    );
    return Operit1SnapshotImportResult.fromJson(value as Map<String, Object?>);
  }

  /// Observes the latest Operit1 snapshot import progress state.
  Future<Operit1SnapshotImportProgress> operit1SnapshotImportProgressFlowSnapshot() async {
    final event = await bridge.watchSnapshot(
      CoreWatchRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        propertyName: 'operit1SnapshotImportProgressFlow',
        args: const <String, Object?>{},
      ),
    );
    return Operit1SnapshotImportProgress.fromJson(event.value as Map<String, Object?>);
  }

  /// Observes the latest Operit1 snapshot import progress state.
  Stream<Operit1SnapshotImportProgress> operit1SnapshotImportProgressFlowChanges() {
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetPath: targetPath, propertyName: 'operit1SnapshotImportProgressFlow', args: const <String, Object?>{}))
        .map((event) => Operit1SnapshotImportProgress.fromJson(event.value as Map<String, Object?>));
  }

}

class GeneratedServicesSttRecognitionServiceCoreProxy {
  const GeneratedServicesSttRecognitionServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Transcribes one in-memory audio payload with the selected STT configuration.
  Future<SttRecognitionResult> transcribeCurrent({required Uint8List audioBytes, required String fileName, required String contentType, required String? language}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'transcribeCurrent',
        args: <String, Object?>{'audioBytes': audioBytes, 'fileName': fileName, 'contentType': contentType, 'language': language},
      ),
    );
    return SttRecognitionResult.fromJson(value as Map<String, Object?>);
  }

  /// Transcribes one in-memory audio payload with an explicit STT configuration id.
  Future<SttRecognitionResult> transcribeWithConfigId({required String configId, required Uint8List audioBytes, required String fileName, required String contentType, required String? language}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'transcribeWithConfigId',
        args: <String, Object?>{'configId': configId, 'audioBytes': audioBytes, 'fileName': fileName, 'contentType': contentType, 'language': language},
      ),
    );
    return SttRecognitionResult.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedServicesTtsPlaybackServiceCoreProxy {
  const GeneratedServicesTtsPlaybackServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Plays a generated speech file through the TTS playback host.
  Future<TtsPlaybackResult> playAudio({required String path}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'playAudio',
        args: <String, Object?>{'path': path},
      ),
    );
    return TtsPlaybackResult.fromJson(value as Map<String, Object?>);
  }

  /// Speaks text with the TTS configuration bound to a character card.
  Future<TtsHostPlaybackResult> speakForCharacter({required String characterCardId, required String text, required bool interrupt}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'speakForCharacter',
        args: <String, Object?>{'characterCardId': characterCardId, 'text': text, 'interrupt': interrupt},
      ),
    );
    return TtsHostPlaybackResult.fromJson(value as Map<String, Object?>);
  }

  /// Speaks text with a selected TTS configuration.
  Future<TtsHostPlaybackResult> speakWithConfig({required String ttsConfigId, required String text, required bool interrupt}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'speakWithConfig',
        args: <String, Object?>{'ttsConfigId': ttsConfigId, 'text': text, 'interrupt': interrupt},
      ),
    );
    return TtsHostPlaybackResult.fromJson(value as Map<String, Object?>);
  }

  /// Pauses host speech playback.
  Future<TtsHostPlaybackResult> pauseSpeech() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'pauseSpeech',
        args: const <String, Object?>{},
      ),
    );
    return TtsHostPlaybackResult.fromJson(value as Map<String, Object?>);
  }

  /// Resumes host speech playback.
  Future<TtsHostPlaybackResult> resumeSpeech() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resumeSpeech',
        args: const <String, Object?>{},
      ),
    );
    return TtsHostPlaybackResult.fromJson(value as Map<String, Object?>);
  }

  /// Stops host speech playback.
  Future<TtsHostPlaybackResult> stopSpeech() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'stopSpeech',
        args: const <String, Object?>{},
      ),
    );
    return TtsHostPlaybackResult.fromJson(value as Map<String, Object?>);
  }

  /// Reads current host speech playback state.
  Future<TtsHostPlaybackResult> speechState() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'speechState',
        args: const <String, Object?>{},
      ),
    );
    return TtsHostPlaybackResult.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedServicesTtsSynthesisServiceCoreProxy {
  const GeneratedServicesTtsSynthesisServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Synthesizes text with the TTS configuration bound to a character card.
  Future<TtsSynthesisResult> synthesizeForCharacter({required String characterCardId, required String text}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'synthesizeForCharacter',
        args: <String, Object?>{'characterCardId': characterCardId, 'text': text},
      ),
    );
    return TtsSynthesisResult.fromJson(value as Map<String, Object?>);
  }

  /// Synthesizes text with a selected TTS configuration.
  Future<TtsSynthesisResult> synthesizeWithConfig({required String ttsConfigId, required String text}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'synthesizeWithConfig',
        args: <String, Object?>{'ttsConfigId': ttsConfigId, 'text': text},
      ),
    );
    return TtsSynthesisResult.fromJson(value as Map<String, Object?>);
  }

}

class GeneratedServicesWorkspaceServiceCoreProxy {
  const GeneratedServicesWorkspaceServiceCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Lists files under a chat-bound workspace relative path.
  Future<List<WorkspaceFileEntry>> listWorkspaceFiles({required String chatId, required String relativePath}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'listWorkspaceFiles',
        args: <String, Object?>{'chatId': chatId, 'relativePath': relativePath},
      ),
    );
    return (value as List<Object?>).map((item) => WorkspaceFileEntry.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Lists directories that can be selected as workspace binding targets.
  Future<List<WorkspaceFileEntry>> listWorkspaceBindingDirectories({required String path}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'listWorkspaceBindingDirectories',
        args: <String, Object?>{'path': path},
      ),
    );
    return (value as List<Object?>).map((item) => WorkspaceFileEntry.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Reads a text file from a chat-bound workspace.
  Future<String> readWorkspaceTextFile({required String chatId, required String relativePath}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'readWorkspaceTextFile',
        args: <String, Object?>{'chatId': chatId, 'relativePath': relativePath},
      ),
    );
    return value as String;
  }

  /// Reads a binary file from a chat-bound workspace as base64.
  Future<WorkspaceFileBytes> readWorkspaceFileBytes({required String chatId, required String relativePath}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'readWorkspaceFileBytes',
        args: <String, Object?>{'chatId': chatId, 'relativePath': relativePath},
      ),
    );
    return WorkspaceFileBytes.fromJson(value as Map<String, Object?>);
  }

  /// Writes a text file into a chat-bound workspace.
  Future<void> writeWorkspaceTextFile({required String chatId, required String relativePath, required String content}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'writeWorkspaceTextFile',
        args: <String, Object?>{'chatId': chatId, 'relativePath': relativePath, 'content': content},
      ),
    );
  }

  /// Writes base64-decoded bytes into a chat-bound workspace file.
  Future<void> writeWorkspaceFileBytes({required String chatId, required String relativePath, required String base64Content}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'writeWorkspaceFileBytes',
        args: <String, Object?>{'chatId': chatId, 'relativePath': relativePath, 'base64Content': base64Content},
      ),
    );
  }

  /// Opens a chat-bound workspace file through the host file opener.
  Future<void> openWorkspaceFile({required String chatId, required String relativePath}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'openWorkspaceFile',
        args: <String, Object?>{'chatId': chatId, 'relativePath': relativePath},
      ),
    );
  }

  /// Builds the workspace-management summary for chat bindings and stored workspace folders.
  Future<WorkspaceManagementSummary> workspaceManagementSummary() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'workspaceManagementSummary',
        args: const <String, Object?>{},
      ),
    );
    return WorkspaceManagementSummary.fromJson(value as Map<String, Object?>);
  }

  /// Deletes workspace folders that are not bound to any chat.
  Future<int> deleteUnboundWorkspaces({required List<String> workspaceNames}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteUnboundWorkspaces',
        args: <String, Object?>{'workspaceNames': workspaceNames.map((item) => item).toList(growable: false)},
      ),
    );
    return value as int;
  }

}

class GeneratedApplicationAiToolHandlerCoreProxy {
  const GeneratedApplicationAiToolHandlerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Removes one registered tool and its visibility metadata.
  Future<void> unregisterTool({required String toolName}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'unregisterTool',
        args: <String, Object?>{'toolName': toolName},
      ),
    );
  }

  /// Removes all tools registered for an MCP server namespace.
  Future<int> unregisterMcpServerTools({required String serverName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'unregisterMcpServerTools',
        args: <String, Object?>{'serverName': serverName},
      ),
    );
    return value as int;
  }

  /// Removes the package registration created for an MCP server.
  Future<bool> unregisterMcpServerPackage({required String serverName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'unregisterMcpServerPackage',
        args: <String, Object?>{'serverName': serverName},
      ),
    );
    return value as bool;
  }

  /// Removes a tool lifecycle hook by hook id.
  Future<void> removeToolHook({required String hookId}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'removeToolHook',
        args: <String, Object?>{'hookId': hookId},
      ),
    );
  }

  /// Removes every registered tool lifecycle hook.
  Future<void> clearToolHooks() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'clearToolHooks',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Notifies hooks that a tool call has been requested.
  Future<void> notifyToolCallRequested({required Object? tool}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'notifyToolCallRequested',
        args: <String, Object?>{'tool': tool},
      ),
    );
  }

  /// Notifies hooks that permission was checked for a tool call.
  Future<void> notifyToolPermissionChecked({required Object? tool, required bool granted, required String? reason}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'notifyToolPermissionChecked',
        args: <String, Object?>{'tool': tool, 'granted': granted, 'reason': reason},
      ),
    );
  }

  /// Notifies hooks that tool execution is about to start.
  Future<void> notifyToolExecutionStarted({required Object? tool}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'notifyToolExecutionStarted',
        args: <String, Object?>{'tool': tool},
      ),
    );
  }

  /// Notifies hooks that tool execution returned a result.
  Future<void> notifyToolExecutionResult({required Object? tool, required Object? result}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'notifyToolExecutionResult',
        args: <String, Object?>{'tool': tool, 'result': result},
      ),
    );
  }

  /// Notifies hooks that tool execution failed before producing a normal result.
  Future<void> notifyToolExecutionError({required Object? tool, required String message}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'notifyToolExecutionError',
        args: <String, Object?>{'tool': tool, 'message': message},
      ),
    );
  }

  /// Notifies hooks that tool execution has fully finished.
  Future<void> notifyToolExecutionFinished({required Object? tool}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'notifyToolExecutionFinished',
        args: <String, Object?>{'tool': tool},
      ),
    );
  }

  /// Returns every registered tool name regardless of visibility.
  Future<List<String>> getAllToolNames() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAllToolNames',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Returns the host environment descriptor associated with this handler.
  Future<HostEnvironmentDescriptor> getHostEnvironmentDescriptor() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getHostEnvironmentDescriptor',
        args: const <String, Object?>{},
      ),
    );
    return HostEnvironmentDescriptor.fromJson(value as Map<String, Object?>);
  }

  /// Returns tool names that should be visible to normal callers.
  Future<List<String>> getPublicToolNames() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPublicToolNames',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Returns tool names reserved for internal runtime calls.
  Future<List<String>> getInternalToolNames() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getInternalToolNames',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Returns the configured visibility for one tool.
  Future<ToolRegistrationVisibility?> getToolVisibility({required String toolName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolVisibility',
        args: <String, Object?>{'toolName': toolName},
      ),
    );
    return value == null ? null : ToolRegistrationVisibility.fromJson(value);
  }

  /// Registers built-in public and internal tools once for this handler.
  Future<void> registerDefaultTools() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'registerDefaultTools',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Returns whether a tool executor is already registered.
  Future<bool> hasToolExecutor({required String toolName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'hasToolExecutor',
        args: <String, Object?>{'toolName': toolName},
      ),
    );
    return value as bool;
  }

  /// Ensures default or package tools are registered, then reports whether a tool exists.
  Future<bool> getToolExecutorOrActivate({required String toolName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolExecutorOrActivate',
        args: <String, Object?>{'toolName': toolName},
      ),
    );
    return value as bool;
  }

  /// Executes a tool through hooks, permissions, limits, and a resolved executor.
  Future<List<CoreOperitToolsConversationMarkupManagerToolResult>?> executeToolSafelyWithResolvedExecutor({required Object? tool}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'executeToolSafelyWithResolvedExecutor',
        args: <String, Object?>{'tool': tool},
      ),
    );
    return value == null ? null : (value as List<Object?>).map((item) => CoreOperitToolsConversationMarkupManagerToolResult.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Resolves and executes a tool request through the registered tool chain.
  Future<CoreOperitToolsConversationMarkupManagerToolResult> executeTool({required CoreOperitToolsToolExecutionManagerAiTool tool}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'executeTool',
        args: <String, Object?>{'tool': tool.toJson()},
      ),
    );
    return CoreOperitToolsConversationMarkupManagerToolResult.fromJson(value as Map<String, Object?>);
  }

  /// Clears registered tools and runtime-only handler state.
  Future<void> reset() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'reset',
        args: const <String, Object?>{},
      ),
    );
  }

}

class GeneratedApplicationMcpRepositoryCoreProxy {
  const GeneratedApplicationMcpRepositoryCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Installs an MCP server from a repository URL for Flutter bridge callers.
  Future<String> installMcpServerWithObjectForFlutter({required String pluginId, required String repoUrl, required String name, required String description, required String mcpConfig}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'installMCPServerWithObjectForFlutter',
        args: <String, Object?>{'pluginId': pluginId, 'repoUrl': repoUrl, 'name': name, 'description': description, 'mcpConfig': mcpConfig},
      ),
    );
    return value as String;
  }

  /// Installs an MCP server from a local zip for Flutter bridge callers.
  Future<String> installMcpServerFromZipForFlutter({required String pluginId, required String zipPath, required String name, required String description, required String mcpConfig}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'installMCPServerFromZipForFlutter',
        args: <String, Object?>{'pluginId': pluginId, 'zipPath': zipPath, 'name': name, 'description': description, 'mcpConfig': mcpConfig},
      ),
    );
    return value as String;
  }

  /// Generates a concise plugin description from available MCP tool descriptions.
  Future<String> generatePluginDescription({required String pluginId, required String pluginName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'generatePluginDescription',
        args: <String, Object?>{'pluginId': pluginId, 'pluginName': pluginName},
      ),
    );
    return value as String;
  }

}

class GeneratedApplicationSkillRepositoryCoreProxy {
  const GeneratedApplicationSkillRepositoryCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Returns the directory where user-installed skills are stored.
  Future<String> getSkillsDirectoryPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getSkillsDirectoryPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Returns all valid installed skill packages.
  Future<Map<String, SkillPackage>> getAvailableSkillPackages() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAvailableSkillPackages',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, SkillPackage.fromJson(value as Map<String, Object?>)));
  }

  /// Returns valid installed skill packages together with scan errors.
  Future<Object?> getAvailableSkillPackagesSnapshot() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAvailableSkillPackagesSnapshot',
        args: const <String, Object?>{},
      ),
    );
    return value;
  }

  /// Returns skill directory scan errors keyed by directory name.
  Future<Map<String, String>> getSkillLoadErrors() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getSkillLoadErrors',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String));
  }

  /// Lists bundled external skills that are available for installation.
  Future<List<BundledExternalSkillCandidate>> getBundledExternalSkillCandidates() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getBundledExternalSkillCandidates',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => BundledExternalSkillCandidate.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Installs one bundled external skill.
  Future<SkillPackage> importBundledExternalSkill({required String skillName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importBundledExternalSkill',
        args: <String, Object?>{'skillName': skillName},
      ),
    );
    return SkillPackage.fromJson(value as Map<String, Object?>);
  }

  /// Returns installed skill packages that are visible to AI package activation.
  Future<Map<String, SkillPackage>> getAiVisibleSkillPackages() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAiVisibleSkillPackages',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, SkillPackage.fromJson(value as Map<String, Object?>)));
  }

  /// Reads the SKILL.md content for one installed skill.
  Future<String?> readSkillContent({required String skillName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'readSkillContent',
        args: <String, Object?>{'skillName': skillName},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Deletes one installed skill directory.
  Future<bool> deleteSkill({required String skillName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deleteSkill',
        args: <String, Object?>{'skillName': skillName},
      ),
    );
    return value as bool;
  }

  /// Returns whether one skill is visible to AI package activation.
  Future<bool> isSkillVisibleToAi({required String skillName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isSkillVisibleToAi',
        args: <String, Object?>{'skillName': skillName},
      ),
    );
    return value as bool;
  }

  /// Sets whether one skill is visible to AI package activation.
  Future<void> setSkillVisibleToAi({required String skillName, required bool visible}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setSkillVisibleToAi',
        args: <String, Object?>{'skillName': skillName, 'visible': visible},
      ),
    );
  }

  /// Installs the quick plugin creator skill and marks it visible to AI.
  Future<SkillPackage> ensureQuickPluginCreatorSkillVisible() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'ensureQuickPluginCreatorSkillVisible',
        args: const <String, Object?>{},
      ),
    );
    return SkillPackage.fromJson(value as Map<String, Object?>);
  }

  /// Imports a skill from a zip archive by searching for SKILL.md.
  Future<String> importSkillFromZip({required Object? zipFile}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importSkillFromZip',
        args: <String, Object?>{'zipFile': zipFile},
      ),
    );
    return value as String;
  }

  /// Imports a skill from a zip archive using an optional subdirectory inside the zip.
  Future<String> importSkillFromZipWithSubDir({required Object? zipFile, required String? subDirPathInZip}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importSkillFromZipWithSubDir',
        args: <String, Object?>{'zipFile': zipFile, 'subDirPathInZip': subDirPathInZip},
      ),
    );
    return value as String;
  }

  /// Downloads a GitHub repository zip and imports a skill from it.
  Future<String> importSkillFromGitHubRepo({required String repoUrl}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importSkillFromGitHubRepo',
        args: <String, Object?>{'repoUrl': repoUrl},
      ),
    );
    return value as String;
  }

  /// Creates a skill directly from text content and copied attachment files.
  Future<String> importSkillFromDirectInput({required String skillId, required String description, required String content, required Object? attachmentPaths}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importSkillFromDirectInput',
        args: <String, Object?>{'skillId': skillId, 'description': description, 'content': content, 'attachmentPaths': attachmentPaths},
      ),
    );
    return value as String;
  }

}

class GeneratedApplicationUserMarkdownRepositoryCoreProxy {
  const GeneratedApplicationUserMarkdownRepositoryCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Reads the user markdown file after ensuring it exists.
  Future<String> readUserMarkdown() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'readUserMarkdown',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Writes normalized content to the user markdown file.
  Future<void> writeUserMarkdown({required String content}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'writeUserMarkdown',
        args: <String, Object?>{'content': content},
      ),
    );
  }

}

class GeneratedApplicationInputMenuToggleBridgeCoreProxy {
  const GeneratedApplicationInputMenuToggleBridgeCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Generated proxy for `createToggleDefinitions`.
  Future<List<InputMenuToggleDefinitionSnapshot>> createToggleDefinitions({required String? chatId, required Map<String, bool> featureStates, required String? runtime}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createToggleDefinitions',
        args: <String, Object?>{'chatId': chatId, 'featureStates': featureStates.map((key, value) => MapEntry(key, value)), 'runtime': runtime},
      ),
    );
    return (value as List<Object?>).map((item) => InputMenuToggleDefinitionSnapshot.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Builds input-menu toggle definitions for Flutter from active tool package hooks.
  Future<List<InputMenuToggleDefinitionSnapshot>> createToggleDefinitionsForFlutter({required String? chatId, required Map<String, bool> featureStates, required String? runtime}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'createToggleDefinitionsForFlutter',
        args: <String, Object?>{'chatId': chatId, 'featureStates': featureStates.map((key, value) => MapEntry(key, value)), 'runtime': runtime},
      ),
    );
    return (value as List<Object?>).map((item) => InputMenuToggleDefinitionSnapshot.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Generated proxy for `triggerToggle`.
  Future<bool> triggerToggle({required String toggleId, required String? chatId, required String? runtime}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'triggerToggle',
        args: <String, Object?>{'toggleId': toggleId, 'chatId': chatId, 'runtime': runtime},
      ),
    );
    return value as bool;
  }

  /// Runs a Flutter-requested input-menu toggle hook by toggle identifier.
  Future<bool> triggerToggleForFlutter({required String toggleId, required String? chatId, required String? runtime}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'triggerToggleForFlutter',
        args: <String, Object?>{'toggleId': toggleId, 'chatId': chatId, 'runtime': runtime},
      ),
    );
    return value as bool;
  }

  /// Reports the input-menu toggle change counter for Flutter watchers.
  Future<int> changeVersion() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'changeVersion',
        args: const <String, Object?>{},
      ),
    );
    return value as int;
  }

}

class GeneratedApplicationPackageManagerCoreProxy {
  const GeneratedApplicationPackageManagerCoreProxy(this.bridge, this.targetPath);

  final OperitRuntimeBridge bridge;

  final CoreObjectPath targetPath;

  /// Marks a package as active for the current prompt session.
  Future<bool> activatePackage({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'activatePackage',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as bool;
  }

  /// Releases one explicitly owned ToolPkg execution engine.
  Future<void> releaseToolPkgExecutionEngine({required String contextKey, required String containerPackageName}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'releaseToolPkgExecutionEngine',
        args: <String, Object?>{'contextKey': contextKey, 'containerPackageName': containerPackageName},
      ),
    );
  }

  /// Acquires one explicit owner lease for a ToolPkg execution engine.
  Future<void> acquireToolPkgExecutionEngine({required String contextKey, required String containerPackageName}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'acquireToolPkgExecutionEngine',
        args: <String, Object?>{'contextKey': contextKey, 'containerPackageName': containerPackageName},
      ),
    );
  }

  /// Executes a Compose DSL render script through the ToolPkg engine for a context.
  Future<String?> executeToolPkgComposeDslScript({required String contextKey, required String containerPackageName, required String script, required Map<String, Object?> runtimeOptions, required Map<String, String> envOverrides}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'executeToolPkgComposeDslScript',
        args: <String, Object?>{'contextKey': contextKey, 'containerPackageName': containerPackageName, 'script': script, 'runtimeOptions': runtimeOptions.map((key, value) => MapEntry(key, value)), 'envOverrides': envOverrides.map((key, value) => MapEntry(key, value))},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Dispatches a Compose DSL action through the ToolPkg engine for a context.
  Future<List<String>> dispatchToolPkgComposeDslActionEvents({required String contextKey, required String containerPackageName, required String actionId, required Object? payload, required Map<String, Object?> runtimeOptions, required Map<String, String> envOverrides}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'dispatchToolPkgComposeDslActionEvents',
        args: <String, Object?>{'contextKey': contextKey, 'containerPackageName': containerPackageName, 'actionId': actionId, 'payload': payload, 'runtimeOptions': runtimeOptions.map((key, value) => MapEntry(key, value)), 'envOverrides': envOverrides.map((key, value) => MapEntry(key, value))},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Returns whether a package has been activated for the current prompt session.
  Future<bool> isPackageActivated({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isPackageActivated',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as bool;
  }

  /// Activates a package and returns its system prompt contribution.
  Future<String> usePackage({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'usePackage',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as String;
  }

  /// Executes the built-in package activation tool.
  Future<CoreOperitToolsConversationMarkupManagerToolResult> executeUsePackageTool({required String toolName, required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'executeUsePackageTool',
        args: <String, Object?>{'toolName': toolName, 'packageName': packageName},
      ),
    );
    return CoreOperitToolsConversationMarkupManagerToolResult.fromJson(value as Map<String, Object?>);
  }

  /// Returns package names enabled in preferences after applying disabled package records.
  Future<List<String>> getEnabledPackageNames() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getEnabledPackageNames',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Returns whether a package is enabled and not disabled by ToolPkg subpackage state.
  Future<bool> isPackageEnabled({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isPackageEnabled',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as bool;
  }

  /// Returns package names currently active in the prompt session.
  Future<List<String>> getActivePackageNames() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getActivePackageNames',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => item as String).toList(growable: false);
  }

  /// Enables a package and loads its tools into available package state.
  Future<String> enablePackage({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'enablePackage',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as String;
  }

  /// Disables a package and removes its tools from active package state.
  Future<String> disablePackage({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'disablePackage',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as String;
  }

  /// Deletes an external package from storage and package state.
  Future<bool> deletePackage({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'deletePackage',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as bool;
  }

  /// Enables a ToolPkg container through the normal package enable flow.
  Future<String> enableToolPkgContainer({required String containerPackageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'enableToolPkgContainer',
        args: <String, Object?>{'containerPackageName': containerPackageName},
      ),
    );
    return value as String;
  }

  /// Disables a ToolPkg container through the normal package disable flow.
  Future<String> disableToolPkgContainer({required String containerPackageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'disableToolPkgContainer',
        args: <String, Object?>{'containerPackageName': containerPackageName},
      ),
    );
    return value as String;
  }

  /// Returns whether a package name belongs to a ToolPkg container runtime.
  Future<bool> isToolPkgContainer({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isToolPkgContainer',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as bool;
  }

  /// Returns whether a package name is a ToolPkg subpackage.
  Future<bool> isToolPkgSubpackage({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isToolPkgSubpackage',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as bool;
  }

  /// Returns whether a package is visible as a top-level package.
  Future<bool> isTopLevelPackage({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isTopLevelPackage',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as bool;
  }

  /// Returns available packages excluding ToolPkg subpackages.
  Future<Map<String, ToolPackage>> getTopLevelAvailablePackages() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getTopLevelAvailablePackages',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, ToolPackage.fromJson(value as Map<String, Object?>)));
  }

  /// Returns packages that can be executed directly as tools.
  Future<Map<String, ToolPackage>> getExecutableAvailablePackages() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getExecutableAvailablePackages',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, ToolPackage.fromJson(value as Map<String, Object?>)));
  }

  /// Returns localized details for all registered ToolPkg containers.
  Future<List<ToolPkgContainerDetails>> getToolPkgPluginContainerDetails({required bool useEnglish}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgPluginContainerDetails',
        args: <String, Object?>{'useEnglish': useEnglish},
      ),
    );
    return (value as List<Object?>).map((item) => ToolPkgContainerDetails.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns ToolPkg container runtimes that are currently enabled.
  Future<List<ToolPkgContainerRuntime>> getEnabledToolPkgContainerRuntimes() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getEnabledToolPkgContainerRuntimes',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => ToolPkgContainerRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns all registered ToolPkg container runtimes.
  Future<List<ToolPkgContainerRuntime>> getToolPkgContainerRuntimes() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgContainerRuntimes',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => ToolPkgContainerRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns localized details for a ToolPkg container.
  Future<ToolPkgContainerDetails?> getToolPkgContainerDetails({required String packageName, required bool useEnglish}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgContainerDetails',
        args: <String, Object?>{'packageName': packageName, 'useEnglish': useEnglish},
      ),
    );
    return value == null ? null : ToolPkgContainerDetails.fromJson(value as Map<String, Object?>);
  }

  /// Returns UI routes exposed by ToolPkg modules for one runtime target.
  Future<List<ToolPkgUiRoute>> getToolPkgUiRoutes({required String runtime, required bool useEnglish}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgUiRoutes',
        args: <String, Object?>{'runtime': runtime, 'useEnglish': useEnglish},
      ),
    );
    return (value as List<Object?>).map((item) => ToolPkgUiRoute.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns desktop widgets exposed by enabled ToolPkg containers.
  Future<List<ToolPkgDesktopWidget>> getToolPkgDesktopWidgets({required bool useEnglish}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgDesktopWidgets',
        args: <String, Object?>{'useEnglish': useEnglish},
      ),
    );
    return (value as List<Object?>).map((item) => ToolPkgDesktopWidget.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns navigation entries exposed by enabled ToolPkg containers.
  Future<List<ToolPkgNavigationEntry>> getToolPkgNavigationEntries({required bool useEnglish}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgNavigationEntries',
        args: <String, Object?>{'useEnglish': useEnglish},
      ),
    );
    return (value as List<Object?>).map((item) => ToolPkgNavigationEntry.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Returns workspace templates exposed by enabled ToolPkg containers.
  Future<List<ToolPkgWorkspaceTemplate>> getToolPkgWorkspaceTemplates({required bool useEnglish}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgWorkspaceTemplates',
        args: <String, Object?>{'useEnglish': useEnglish},
      ),
    );
    return (value as List<Object?>).map((item) => ToolPkgWorkspaceTemplate.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Imports a ToolPkg workspace template into a destination directory.
  Future<ToolPkgWorkspaceTemplateImportResult> importToolPkgWorkspaceTemplate({required String containerPackageName, required String templateId, required String destinationDir}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importToolPkgWorkspaceTemplate',
        args: <String, Object?>{'containerPackageName': containerPackageName, 'templateId': templateId, 'destinationDir': destinationDir},
      ),
    );
    return ToolPkgWorkspaceTemplateImportResult.fromJson(value as Map<String, Object?>);
  }

  /// Updates the enabled state for a ToolPkg subpackage.
  Future<bool> setToolPkgSubpackageEnabled({required String subpackagePackageName, required bool enabled}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setToolPkgSubpackageEnabled',
        args: <String, Object?>{'subpackagePackageName': subpackagePackageName, 'enabled': enabled},
      ),
    );
    return value as bool;
  }

  /// Resolves the best package name for a ToolPkg subpackage id.
  Future<String?> findPreferredPackageNameForSubpackageId({required String subpackageId, required bool preferEnabled}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'findPreferredPackageNameForSubpackageId',
        args: <String, Object?>{'subpackageId': subpackageId, 'preferEnabled': preferEnabled},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Runs a ToolPkg navigation entry action hook.
  Future<String?> runToolPkgNavigationEntryAction({required String containerPackageName, required String entryId, required String functionName, required String? inlineFunctionSource, required Object? eventPayload}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'runToolPkgNavigationEntryAction',
        args: <String, Object?>{'containerPackageName': containerPackageName, 'entryId': entryId, 'functionName': functionName, 'inlineFunctionSource': inlineFunctionSource, 'eventPayload': eventPayload},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Lists bundled external standalone packages that are not currently loaded.
  Future<List<BundledExternalPackageCandidate>> getBundledExternalPackageCandidates() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getBundledExternalPackageCandidates',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => BundledExternalPackageCandidate.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Lists bundled external ToolPkg containers that are not currently loaded.
  Future<List<ToolPkgContainerRuntime>> getBundledExternalToolPkgContainerRuntimes() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getBundledExternalToolPkgContainerRuntimes',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => ToolPkgContainerRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Imports a bundled external standalone package into local package storage.
  Future<String> importBundledExternalPackage({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importBundledExternalPackage',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value as String;
  }

  /// Imports a bundled external ToolPkg container into local package storage.
  Future<String> importBundledExternalToolPkgContainer({required String containerPackageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'importBundledExternalToolPkgContainer',
        args: <String, Object?>{'containerPackageName': containerPackageName},
      ),
    );
    return value as String;
  }

  /// Returns the runtime metadata for a ToolPkg container.
  Future<ToolPkgContainerRuntime?> getToolPkgContainerRuntime({required String containerPackageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgContainerRuntime',
        args: <String, Object?>{'containerPackageName': containerPackageName},
      ),
    );
    return value == null ? null : ToolPkgContainerRuntime.fromJson(value as Map<String, Object?>);
  }

  /// Resolves ToolPkg subpackage runtime metadata by package name.
  Future<ToolPkgSubpackageRuntime?> resolveToolPkgSubpackageRuntimeInternal({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'resolveToolPkgSubpackageRuntimeInternal',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value == null ? null : ToolPkgSubpackageRuntime.fromJson(value as Map<String, Object?>);
  }

  /// Returns package tools with active state applied.
  Future<ToolPackage?> getEffectivePackageTools({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getEffectivePackageTools',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value == null ? null : ToolPackage.fromJson(value as Map<String, Object?>);
  }

  /// Returns raw package tool metadata by package name.
  Future<ToolPackage?> getPackageTools({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPackageTools',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value == null ? null : ToolPackage.fromJson(value as Map<String, Object?>);
  }

  /// Returns the first tool script associated with a package.
  Future<String?> getPackageScript({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPackageScript',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Returns the active state id selected for a package.
  Future<String?> getActivePackageStateId({required String packageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getActivePackageStateId',
        args: <String, Object?>{'packageName': packageName},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Returns all package definitions currently available to the manager.
  Future<Map<String, ToolPackage>> getAvailablePackages() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getAvailablePackages',
        args: const <String, Object?>{},
      ),
    );
    return (value as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, ToolPackage.fromJson(value as Map<String, Object?>)));
  }

  /// Removes an MCP server package from package manager state.
  Future<bool> unregisterMcpServerPackage({required String serverName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'unregisterMCPServerPackage',
        args: <String, Object?>{'serverName': serverName},
      ),
    );
    return value as bool;
  }

  /// Registers or replaces an available package definition.
  Future<void> setAvailablePackage({required String packageName, required ToolPackage toolPackage}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setAvailablePackage',
        args: <String, Object?>{'packageName': packageName, 'toolPackage': toolPackage.toJson()},
      ),
    );
  }

  /// Registers a loaded ToolPkg container and its subpackages.
  Future<bool> registerToolPkg({required ToolPkgLoadResult loadResult}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'registerToolPkg',
        args: <String, Object?>{'loadResult': loadResult.toJson()},
      ),
    );
    return value as bool;
  }

  /// Returns the external package storage path as display text.
  Future<String> getExternalPackagesPath() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getExternalPackagesPath',
        args: const <String, Object?>{},
      ),
    );
    return value as String;
  }

  /// Scans built-in, bundled external, and external package sources.
  Future<void> loadAvailablePackages() async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'loadAvailablePackages',
        args: const <String, Object?>{},
      ),
    );
  }

  /// Returns whether the ToolPkg protection secret is configured.
  Future<bool> isToolPkgProtectionSecretConfigured() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'isToolPkgProtectionSecretConfigured',
        args: const <String, Object?>{},
      ),
    );
    return value as bool;
  }

  /// Protects a local JS or ToolPkg artifact before marketplace upload.
  Future<Uint8List> protectArtifactFile({required String sourcePath, required bool isToolPkg, required String packageId, required String version, required List<String> author, required bool minifyArtifact}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'protectArtifactFile',
        args: <String, Object?>{'sourcePath': sourcePath, 'isToolPkg': isToolPkg, 'packageId': packageId, 'version': version, 'author': author.map((item) => item).toList(growable: false), 'minifyArtifact': minifyArtifact},
      ),
    );
    return value as Uint8List;
  }

  /// Returns package sources that can be exported or published.
  Future<List<PublishablePackageSource>> getPublishablePackageSources() async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getPublishablePackageSources',
        args: const <String, Object?>{},
      ),
    );
    return (value as List<Object?>).map((item) => PublishablePackageSource.fromJson(item as Map<String, Object?>)).toList(growable: false);
  }

  /// Imports a package file from external storage into package storage.
  Future<String> addPackageFileFromExternalStorage({required String filePath}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'addPackageFileFromExternalStorage',
        args: <String, Object?>{'filePath': filePath},
      ),
    );
    return value as String;
  }

  /// Installs one signed marketplace ToolPkg as a locally authenticated package archive.
  Future<String> addMarketToolPkgFileFromExternalStorage({required String filePath, required String expectedMarketAssetSha256}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'addMarketToolPkgFileFromExternalStorage',
        args: <String, Object?>{'filePath': filePath, 'expectedMarketAssetSha256': expectedMarketAssetSha256},
      ),
    );
    return value as String;
  }

  /// Persists the complete enabled package name list.
  Future<void> setEnabledPackageNames({required Object? packageNames}) async {
    await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'setEnabledPackageNames',
        args: <String, Object?>{'packageNames': packageNames},
      ),
    );
  }

  /// Returns the main script for an enabled ToolPkg container.
  Future<String?> getToolPkgMainScriptInternal({required String containerPackageName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgMainScriptInternal',
        args: <String, Object?>{'containerPackageName': containerPackageName},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Reads a text resource from a ToolPkg container or subpackage.
  Future<String?> readToolPkgTextResource({required String packageNameOrSubpackageId, required String resourcePath, required bool preferEnabledContainer}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'readToolPkgTextResource',
        args: <String, Object?>{'packageNameOrSubpackageId': packageNameOrSubpackageId, 'resourcePath': resourcePath, 'preferEnabledContainer': preferEnabledContainer},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Calls one declared ToolPkg WASM export.
  Future<JsToolPkgWasmResult> callToolPkgWasm({required JsToolPkgWasmRequest request, required bool preferEnabledContainer}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'callToolPkgWasm',
        args: <String, Object?>{'request': request.toJson(), 'preferEnabledContainer': preferEnabledContainer},
      ),
    );
    return JsToolPkgWasmResult.fromJson(value as Map<String, Object?>);
  }

  /// Copies a ToolPkg resource selected by subpackage id to a file.
  Future<bool> copyToolPkgResourceToFileBySubpackageId({required String subpackageId, required String resourceKey, required Object? destinationFile, required bool preferEnabledContainer}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'copyToolPkgResourceToFileBySubpackageId',
        args: <String, Object?>{'subpackageId': subpackageId, 'resourceKey': resourceKey, 'destinationFile': destinationFile, 'preferEnabledContainer': preferEnabledContainer},
      ),
    );
    return value as bool;
  }

  /// Copies a ToolPkg resource from a container to a file.
  Future<bool> copyToolPkgResourceToFile({required String containerPackageName, required String resourceKey, required Object? destinationFile}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'copyToolPkgResourceToFile',
        args: <String, Object?>{'containerPackageName': containerPackageName, 'resourceKey': resourceKey, 'destinationFile': destinationFile},
      ),
    );
    return value as bool;
  }

  /// Returns the output file name declared for a ToolPkg resource.
  Future<String?> getToolPkgResourceOutputFileName({required String packageNameOrSubpackageId, required String resourceKey, required bool preferEnabledContainer}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgResourceOutputFileName',
        args: <String, Object?>{'packageNameOrSubpackageId': packageNameOrSubpackageId, 'resourceKey': resourceKey, 'preferEnabledContainer': preferEnabledContainer},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Returns Compose DSL script text selected through a ToolPkg subpackage id.
  Future<String?> getToolPkgComposeDslScriptBySubpackageId({required String subpackageId, required String? uiModuleId, required bool preferEnabledContainer}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgComposeDslScriptBySubpackageId',
        args: <String, Object?>{'subpackageId': subpackageId, 'uiModuleId': uiModuleId, 'preferEnabledContainer': preferEnabledContainer},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Returns Compose DSL script text for a ToolPkg UI module.
  Future<String?> getToolPkgComposeDslScript({required String containerPackageName, required String? uiModuleId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgComposeDslScript',
        args: <String, Object?>{'containerPackageName': containerPackageName, 'uiModuleId': uiModuleId},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Returns the Compose DSL screen path for a ToolPkg UI module.
  Future<String?> getToolPkgComposeDslScreenPath({required String containerPackageName, required String? uiModuleId}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'getToolPkgComposeDslScreenPath',
        args: <String, Object?>{'containerPackageName': containerPackageName, 'uiModuleId': uiModuleId},
      ),
    );
    return value == null ? null : value as String;
  }

  /// Activates an MCP server package and returns its system prompt contribution.
  Future<String> useMcpServer({required String serverName}) async {
    final value = await bridge.call(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetPath: targetPath,
        methodName: 'useMCPServer',
        args: <String, Object?>{'serverName': serverName},
      ),
    );
    return value as String;
  }

}

