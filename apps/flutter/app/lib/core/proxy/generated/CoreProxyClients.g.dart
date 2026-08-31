// GENERATED CODE - DO NOT MODIFY BY HAND

import 'dart:async';
import 'dart:typed_data';

import '../../bridge/OperitRuntimeBridge.dart';
import '../../link/CoreLinkCodec.dart';
import '../../link/CoreLinkProtocol.dart';
import 'CoreProxyModels.g.dart';

String _coreProxyRequestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';
Map<String, Object?> _coreProxyArgs(Map<String, Object?> args, Map<String, Object?> objectArgs) => <String, Object?>{...objectArgs, ...args};

class GeneratedCoreProxyClients {
  const GeneratedCoreProxyClients(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `application`.
  GeneratedApplicationCoreProxy get application => GeneratedApplicationCoreProxy._(bridge, 0);
  /// Returns a generated proxy client for `chatRuntimeHolderMain`.
  GeneratedChatRuntimeHolderMainCoreProxy get chatRuntimeHolderMain => GeneratedChatRuntimeHolderMainCoreProxy._(bridge, 7);
  /// Returns a generated proxy client for `linkAccess.linkAccessStore`.
  GeneratedLinkAccessLinkAccessStoreCoreProxy get linkAccessLinkAccessStore => GeneratedLinkAccessLinkAccessStoreCoreProxy._(bridge, 8);
  /// Returns a generated proxy client for `permissions.mcp.mCPManager`.
  GeneratedPermissionsMcpManagerCoreProxy get permissionsMcpManager => GeneratedPermissionsMcpManagerCoreProxy._(bridge, 9);
  /// Returns a generated proxy client for `permissions.mcp_runtime.mCPLocalServer`.
  GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy get permissionsMcpRuntimeMcpLocalServer => GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy._(bridge, 10);
  /// Returns a generated proxy client for `permissions.mcp_runtime.plugins.mCPBridge`.
  GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy get permissionsMcpRuntimePluginsMcpBridge => GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy._(bridge, 11);
  /// Returns a generated proxy client for `permissions.toolPermissionSystem`.
  GeneratedPermissionsToolPermissionSystemCoreProxy get permissionsToolPermissionSystem => GeneratedPermissionsToolPermissionSystemCoreProxy._(bridge, 12);
  /// Returns a generated proxy client for `preferences.activePromptManager`.
  GeneratedPreferencesActivePromptManagerCoreProxy get preferencesActivePromptManager => GeneratedPreferencesActivePromptManagerCoreProxy._(bridge, 13);
  /// Returns a generated proxy client for `preferences.apiPreferences`.
  GeneratedPreferencesApiPreferencesCoreProxy get preferencesApiPreferences => GeneratedPreferencesApiPreferencesCoreProxy._(bridge, 14);
  /// Returns a generated proxy client for `preferences.characterCardManager`.
  GeneratedPreferencesCharacterCardManagerCoreProxy get preferencesCharacterCardManager => GeneratedPreferencesCharacterCardManagerCoreProxy._(bridge, 15);
  /// Returns a generated proxy client for `preferences.characterCardToolAccessResolver`.
  GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy get preferencesCharacterCardToolAccessResolver => GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy._(bridge, 16);
  /// Returns a generated proxy client for `preferences.characterGroupCardManager`.
  GeneratedPreferencesCharacterGroupCardManagerCoreProxy get preferencesCharacterGroupCardManager => GeneratedPreferencesCharacterGroupCardManagerCoreProxy._(bridge, 17);
  /// Returns a generated proxy client for `preferences.envPreferences`.
  GeneratedPreferencesEnvPreferencesCoreProxy get preferencesEnvPreferences => GeneratedPreferencesEnvPreferencesCoreProxy._(bridge, 18);
  /// Returns a generated proxy client for `preferences.functionalConfigManager`.
  GeneratedPreferencesFunctionalConfigManagerCoreProxy get preferencesFunctionalConfigManager => GeneratedPreferencesFunctionalConfigManagerCoreProxy._(bridge, 19);
  /// Returns a generated proxy client for `preferences.gitHubAuthPreferences`.
  GeneratedPreferencesGitHubAuthPreferencesCoreProxy get preferencesGitHubAuthPreferences => GeneratedPreferencesGitHubAuthPreferencesCoreProxy._(bridge, 20);
  /// Returns a generated proxy client for `preferences.modelConfigManager`.
  GeneratedPreferencesModelConfigManagerCoreProxy get preferencesModelConfigManager => GeneratedPreferencesModelConfigManagerCoreProxy._(bridge, 21);
  /// Returns a generated proxy client for `preferences.preferenceStorageManager`.
  GeneratedPreferencesPreferenceStorageManagerCoreProxy get preferencesPreferenceStorageManager => GeneratedPreferencesPreferenceStorageManagerCoreProxy._(bridge, 22);
  /// Returns a generated proxy client for `preferences.promptTagManager`.
  GeneratedPreferencesPromptTagManagerCoreProxy get preferencesPromptTagManager => GeneratedPreferencesPromptTagManagerCoreProxy._(bridge, 23);
  /// Returns a generated proxy client for `preferences.sharedMemoryStoreManager`.
  GeneratedPreferencesSharedMemoryStoreManagerCoreProxy get preferencesSharedMemoryStoreManager => GeneratedPreferencesSharedMemoryStoreManagerCoreProxy._(bridge, 24);
  /// Returns a generated proxy client for `preferences.skillVisibilityPreferences`.
  GeneratedPreferencesSkillVisibilityPreferencesCoreProxy get preferencesSkillVisibilityPreferences => GeneratedPreferencesSkillVisibilityPreferencesCoreProxy._(bridge, 25);
  /// Returns a generated proxy client for `preferences.sttConfigManager`.
  GeneratedPreferencesSttConfigManagerCoreProxy get preferencesSttConfigManager => GeneratedPreferencesSttConfigManagerCoreProxy._(bridge, 26);
  /// Returns a generated proxy client for `preferences.ttsConfigManager`.
  GeneratedPreferencesTtsConfigManagerCoreProxy get preferencesTtsConfigManager => GeneratedPreferencesTtsConfigManagerCoreProxy._(bridge, 27);
  /// Returns a generated proxy client for `preferences.userPreferencesManager`.
  GeneratedPreferencesUserPreferencesManagerCoreProxy get preferencesUserPreferencesManager => GeneratedPreferencesUserPreferencesManagerCoreProxy._(bridge, 28);
  /// Returns a generated proxy client for `providers.chat.conversationRoundManagerMirror`.
  GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy get providersChatConversationRoundManagerMirror => GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy._(bridge, 29);
  /// Returns a generated proxy client for `providers.chat.enhance.conversationRoundManager`.
  GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy get providersChatEnhanceConversationRoundManager => GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy._(bridge, 30);
  /// Returns a generated proxy client for `providers.chat.llmprovider.streamingJsonXmlConverter`.
  GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy get providersChatLlmproviderStreamingJsonXmlConverter => GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy._(bridge, 31);
  /// Returns a generated proxy client for `providers.market.marketStatsApiService`.
  GeneratedProvidersMarketStatsApiServiceCoreProxy get providersMarketStatsApiService => GeneratedProvidersMarketStatsApiServiceCoreProxy._(bridge, 32);
  /// Returns a generated proxy client for `repository.chatHistoryManager`.
  GeneratedRepositoryChatHistoryManagerCoreProxy get repositoryChatHistoryManager => GeneratedRepositoryChatHistoryManagerCoreProxy._(bridge, 33);
  /// Returns a generated proxy client for `repository.memoryRepository`.
  GeneratedRepositoryMemoryRepositoryCoreProxy get repositoryMemoryRepository => GeneratedRepositoryMemoryRepositoryCoreProxy._(bridge, 34);
  /// Returns the generated proxy for one memory owner.
  GeneratedRepositoryMemoryRepositoryCoreProxy repositoryMemoryRepositoryForOwner(String ownerKey) => GeneratedRepositoryMemoryRepositoryCoreProxy._(bridge, 34, objectArgs: <String, Object?>{'__core_instance_id': ownerKey});
  /// Returns a generated proxy client for `repository.runtimeStorageRepository`.
  GeneratedRepositoryRuntimeStorageRepositoryCoreProxy get repositoryRuntimeStorageRepository => GeneratedRepositoryRuntimeStorageRepositoryCoreProxy._(bridge, 35);
  /// Returns a generated proxy client for `repository.usageStatisticsStore`.
  GeneratedRepositoryUsageStatisticsStoreCoreProxy get repositoryUsageStatisticsStore => GeneratedRepositoryUsageStatisticsStoreCoreProxy._(bridge, 36);
  /// Returns a generated proxy client for `server.coreNodeRouter`.
  GeneratedServerCoreNodeRouterCoreProxy get serverCoreNodeRouter => GeneratedServerCoreNodeRouterCoreProxy._(bridge, 37);
  /// Returns a generated proxy client for `server.runtimeRemoteLinkService`.
  GeneratedServerRuntimeRemoteLinkServiceCoreProxy get serverRuntimeRemoteLinkService => GeneratedServerRuntimeRemoteLinkServiceCoreProxy._(bridge, 38);
  /// Returns a generated proxy client for `services.archiveTransferManager`.
  GeneratedServicesArchiveTransferManagerCoreProxy get servicesArchiveTransferManager => GeneratedServicesArchiveTransferManagerCoreProxy._(bridge, 39);
  /// Returns a generated proxy client for `services.gitHubOAuthBrokerService`.
  GeneratedServicesGitHubOAuthBrokerServiceCoreProxy get servicesGitHubOAuthBrokerService => GeneratedServicesGitHubOAuthBrokerServiceCoreProxy._(bridge, 40);
  /// Returns a generated proxy client for `services.localModelService`.
  GeneratedServicesLocalModelServiceCoreProxy get servicesLocalModelService => GeneratedServicesLocalModelServiceCoreProxy._(bridge, 41);
  /// Returns a generated proxy client for `services.localProviderService`.
  GeneratedServicesLocalProviderServiceCoreProxy get servicesLocalProviderService => GeneratedServicesLocalProviderServiceCoreProxy._(bridge, 42);
  /// Returns a generated proxy client for `services.runtimeBrowserService`.
  GeneratedServicesRuntimeBrowserServiceCoreProxy get servicesRuntimeBrowserService => GeneratedServicesRuntimeBrowserServiceCoreProxy._(bridge, 43);
  /// Returns a generated proxy client for `services.runtimeHostInfoService`.
  GeneratedServicesRuntimeHostInfoServiceCoreProxy get servicesRuntimeHostInfoService => GeneratedServicesRuntimeHostInfoServiceCoreProxy._(bridge, 44);
  /// Returns a generated proxy client for `services.runtimeHostInteractionService`.
  GeneratedServicesRuntimeHostInteractionServiceCoreProxy get servicesRuntimeHostInteractionService => GeneratedServicesRuntimeHostInteractionServiceCoreProxy._(bridge, 45);
  /// Returns a generated proxy client for `services.runtimeTerminalService`.
  GeneratedServicesRuntimeTerminalServiceCoreProxy get servicesRuntimeTerminalService => GeneratedServicesRuntimeTerminalServiceCoreProxy._(bridge, 46);
  /// Returns a generated proxy client for `services.snapshotImportManager`.
  GeneratedServicesSnapshotImportManagerCoreProxy get servicesSnapshotImportManager => GeneratedServicesSnapshotImportManagerCoreProxy._(bridge, 47);
  /// Returns a generated proxy client for `services.sttRecognitionService`.
  GeneratedServicesSttRecognitionServiceCoreProxy get servicesSttRecognitionService => GeneratedServicesSttRecognitionServiceCoreProxy._(bridge, 48);
  /// Returns a generated proxy client for `services.syncBlobTransferManager`.
  GeneratedServicesSyncBlobTransferManagerCoreProxy get servicesSyncBlobTransferManager => GeneratedServicesSyncBlobTransferManagerCoreProxy._(bridge, 49);
  /// Returns a generated proxy client for `services.ttsPlaybackService`.
  GeneratedServicesTtsPlaybackServiceCoreProxy get servicesTtsPlaybackService => GeneratedServicesTtsPlaybackServiceCoreProxy._(bridge, 50);
  /// Returns a generated proxy client for `services.ttsSynthesisService`.
  GeneratedServicesTtsSynthesisServiceCoreProxy get servicesTtsSynthesisService => GeneratedServicesTtsSynthesisServiceCoreProxy._(bridge, 51);
  /// Returns a generated proxy client for `services.workspaceService`.
  GeneratedServicesWorkspaceServiceCoreProxy get servicesWorkspaceService => GeneratedServicesWorkspaceServiceCoreProxy._(bridge, 52);
  /// Returns the generated proxy namespace for `linkAccess`.
  GeneratedLinkAccessCoreProxyNamespace get linkAccess => GeneratedLinkAccessCoreProxyNamespace._(bridge);
  /// Returns the generated proxy namespace for `permissions`.
  GeneratedPermissionsCoreProxyNamespace get permissions => GeneratedPermissionsCoreProxyNamespace._(bridge);
  /// Returns the generated proxy namespace for `preferences`.
  GeneratedPreferencesCoreProxyNamespace get preferences => GeneratedPreferencesCoreProxyNamespace._(bridge);
  /// Returns the generated proxy namespace for `providers`.
  GeneratedProvidersCoreProxyNamespace get providers => GeneratedProvidersCoreProxyNamespace._(bridge);
  /// Returns the generated proxy namespace for `repository`.
  GeneratedRepositoryCoreProxyNamespace get repository => GeneratedRepositoryCoreProxyNamespace._(bridge);
  /// Returns the generated proxy namespace for `server`.
  GeneratedServerCoreProxyNamespace get server => GeneratedServerCoreProxyNamespace._(bridge);
  /// Returns the generated proxy namespace for `services`.
  GeneratedServicesCoreProxyNamespace get services => GeneratedServicesCoreProxyNamespace._(bridge);
}

class GeneratedApplicationCoreProxyNamespace {
  const GeneratedApplicationCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

}

class GeneratedLinkAccessCoreProxyNamespace {
  const GeneratedLinkAccessCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `linkAccess.linkAccessStore`.
  GeneratedLinkAccessLinkAccessStoreCoreProxy get linkAccessStore => GeneratedLinkAccessLinkAccessStoreCoreProxy._(bridge, 8);
}

class GeneratedPermissionsCoreProxyNamespace {
  const GeneratedPermissionsCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns the generated proxy namespace for `permissions.mcp`.
  GeneratedPermissionsMcpCoreProxyNamespace get mcp => GeneratedPermissionsMcpCoreProxyNamespace._(bridge);
  /// Returns the generated proxy namespace for `permissions.mcp_runtime`.
  GeneratedPermissionsMcpRuntimeCoreProxyNamespace get mcpRuntime => GeneratedPermissionsMcpRuntimeCoreProxyNamespace._(bridge);
  /// Returns a generated proxy client for `permissions.toolPermissionSystem`.
  GeneratedPermissionsToolPermissionSystemCoreProxy get toolPermissionSystem => GeneratedPermissionsToolPermissionSystemCoreProxy._(bridge, 12);
}

class GeneratedPermissionsMcpCoreProxyNamespace {
  const GeneratedPermissionsMcpCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `permissions.mcp.mCPManager`.
  GeneratedPermissionsMcpManagerCoreProxy get mcpManager => GeneratedPermissionsMcpManagerCoreProxy._(bridge, 9);
}

class GeneratedPermissionsMcpRuntimeCoreProxyNamespace {
  const GeneratedPermissionsMcpRuntimeCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `permissions.mcp_runtime.mCPLocalServer`.
  GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy get mcpLocalServer => GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy._(bridge, 10);
  /// Returns the generated proxy namespace for `permissions.mcp_runtime.plugins`.
  GeneratedPermissionsMcpRuntimePluginsCoreProxyNamespace get plugins => GeneratedPermissionsMcpRuntimePluginsCoreProxyNamespace._(bridge);
}

class GeneratedPermissionsMcpRuntimePluginsCoreProxyNamespace {
  const GeneratedPermissionsMcpRuntimePluginsCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `permissions.mcp_runtime.plugins.mCPBridge`.
  GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy get mcpBridge => GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy._(bridge, 11);
}

class GeneratedPreferencesCoreProxyNamespace {
  const GeneratedPreferencesCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `preferences.activePromptManager`.
  GeneratedPreferencesActivePromptManagerCoreProxy get activePromptManager => GeneratedPreferencesActivePromptManagerCoreProxy._(bridge, 13);
  /// Returns a generated proxy client for `preferences.apiPreferences`.
  GeneratedPreferencesApiPreferencesCoreProxy get apiPreferences => GeneratedPreferencesApiPreferencesCoreProxy._(bridge, 14);
  /// Returns a generated proxy client for `preferences.characterCardManager`.
  GeneratedPreferencesCharacterCardManagerCoreProxy get characterCardManager => GeneratedPreferencesCharacterCardManagerCoreProxy._(bridge, 15);
  /// Returns a generated proxy client for `preferences.characterCardToolAccessResolver`.
  GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy get characterCardToolAccessResolver => GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy._(bridge, 16);
  /// Returns a generated proxy client for `preferences.characterGroupCardManager`.
  GeneratedPreferencesCharacterGroupCardManagerCoreProxy get characterGroupCardManager => GeneratedPreferencesCharacterGroupCardManagerCoreProxy._(bridge, 17);
  /// Returns a generated proxy client for `preferences.envPreferences`.
  GeneratedPreferencesEnvPreferencesCoreProxy get envPreferences => GeneratedPreferencesEnvPreferencesCoreProxy._(bridge, 18);
  /// Returns a generated proxy client for `preferences.functionalConfigManager`.
  GeneratedPreferencesFunctionalConfigManagerCoreProxy get functionalConfigManager => GeneratedPreferencesFunctionalConfigManagerCoreProxy._(bridge, 19);
  /// Returns a generated proxy client for `preferences.gitHubAuthPreferences`.
  GeneratedPreferencesGitHubAuthPreferencesCoreProxy get gitHubAuthPreferences => GeneratedPreferencesGitHubAuthPreferencesCoreProxy._(bridge, 20);
  /// Returns a generated proxy client for `preferences.modelConfigManager`.
  GeneratedPreferencesModelConfigManagerCoreProxy get modelConfigManager => GeneratedPreferencesModelConfigManagerCoreProxy._(bridge, 21);
  /// Returns a generated proxy client for `preferences.preferenceStorageManager`.
  GeneratedPreferencesPreferenceStorageManagerCoreProxy get preferenceStorageManager => GeneratedPreferencesPreferenceStorageManagerCoreProxy._(bridge, 22);
  /// Returns a generated proxy client for `preferences.promptTagManager`.
  GeneratedPreferencesPromptTagManagerCoreProxy get promptTagManager => GeneratedPreferencesPromptTagManagerCoreProxy._(bridge, 23);
  /// Returns a generated proxy client for `preferences.sharedMemoryStoreManager`.
  GeneratedPreferencesSharedMemoryStoreManagerCoreProxy get sharedMemoryStoreManager => GeneratedPreferencesSharedMemoryStoreManagerCoreProxy._(bridge, 24);
  /// Returns a generated proxy client for `preferences.skillVisibilityPreferences`.
  GeneratedPreferencesSkillVisibilityPreferencesCoreProxy get skillVisibilityPreferences => GeneratedPreferencesSkillVisibilityPreferencesCoreProxy._(bridge, 25);
  /// Returns a generated proxy client for `preferences.sttConfigManager`.
  GeneratedPreferencesSttConfigManagerCoreProxy get sttConfigManager => GeneratedPreferencesSttConfigManagerCoreProxy._(bridge, 26);
  /// Returns a generated proxy client for `preferences.ttsConfigManager`.
  GeneratedPreferencesTtsConfigManagerCoreProxy get ttsConfigManager => GeneratedPreferencesTtsConfigManagerCoreProxy._(bridge, 27);
  /// Returns a generated proxy client for `preferences.userPreferencesManager`.
  GeneratedPreferencesUserPreferencesManagerCoreProxy get userPreferencesManager => GeneratedPreferencesUserPreferencesManagerCoreProxy._(bridge, 28);
}

class GeneratedProvidersCoreProxyNamespace {
  const GeneratedProvidersCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns the generated proxy namespace for `providers.chat`.
  GeneratedProvidersChatCoreProxyNamespace get chat => GeneratedProvidersChatCoreProxyNamespace._(bridge);
  /// Returns the generated proxy namespace for `providers.market`.
  GeneratedProvidersMarketCoreProxyNamespace get market => GeneratedProvidersMarketCoreProxyNamespace._(bridge);
}

class GeneratedProvidersChatCoreProxyNamespace {
  const GeneratedProvidersChatCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `providers.chat.conversationRoundManagerMirror`.
  GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy get conversationRoundManagerMirror => GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy._(bridge, 29);
  /// Returns the generated proxy namespace for `providers.chat.enhance`.
  GeneratedProvidersChatEnhanceCoreProxyNamespace get enhance => GeneratedProvidersChatEnhanceCoreProxyNamespace._(bridge);
  /// Returns the generated proxy namespace for `providers.chat.llmprovider`.
  GeneratedProvidersChatLlmproviderCoreProxyNamespace get llmprovider => GeneratedProvidersChatLlmproviderCoreProxyNamespace._(bridge);
}

class GeneratedProvidersChatEnhanceCoreProxyNamespace {
  const GeneratedProvidersChatEnhanceCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `providers.chat.enhance.conversationRoundManager`.
  GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy get conversationRoundManager => GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy._(bridge, 30);
}

class GeneratedProvidersChatLlmproviderCoreProxyNamespace {
  const GeneratedProvidersChatLlmproviderCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `providers.chat.llmprovider.streamingJsonXmlConverter`.
  GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy get streamingJsonXmlConverter => GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy._(bridge, 31);
}

class GeneratedProvidersMarketCoreProxyNamespace {
  const GeneratedProvidersMarketCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `providers.market.marketStatsApiService`.
  GeneratedProvidersMarketStatsApiServiceCoreProxy get marketStatsApiService => GeneratedProvidersMarketStatsApiServiceCoreProxy._(bridge, 32);
}

class GeneratedRepositoryCoreProxyNamespace {
  const GeneratedRepositoryCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `repository.chatHistoryManager`.
  GeneratedRepositoryChatHistoryManagerCoreProxy get chatHistoryManager => GeneratedRepositoryChatHistoryManagerCoreProxy._(bridge, 33);
  /// Returns a generated proxy client for `repository.memoryRepository`.
  GeneratedRepositoryMemoryRepositoryCoreProxy get memoryRepository => GeneratedRepositoryMemoryRepositoryCoreProxy._(bridge, 34);
  /// Returns a generated proxy client for `repository.runtimeStorageRepository`.
  GeneratedRepositoryRuntimeStorageRepositoryCoreProxy get runtimeStorageRepository => GeneratedRepositoryRuntimeStorageRepositoryCoreProxy._(bridge, 35);
  /// Returns a generated proxy client for `repository.usageStatisticsStore`.
  GeneratedRepositoryUsageStatisticsStoreCoreProxy get usageStatisticsStore => GeneratedRepositoryUsageStatisticsStoreCoreProxy._(bridge, 36);
}

class GeneratedServerCoreProxyNamespace {
  const GeneratedServerCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `server.coreNodeRouter`.
  GeneratedServerCoreNodeRouterCoreProxy get coreNodeRouter => GeneratedServerCoreNodeRouterCoreProxy._(bridge, 37);
  /// Returns a generated proxy client for `server.runtimeRemoteLinkService`.
  GeneratedServerRuntimeRemoteLinkServiceCoreProxy get runtimeRemoteLinkService => GeneratedServerRuntimeRemoteLinkServiceCoreProxy._(bridge, 38);
}

class GeneratedServicesCoreProxyNamespace {
  const GeneratedServicesCoreProxyNamespace._(this.bridge);

  final OperitRuntimeBridge bridge;

  /// Returns a generated proxy client for `services.archiveTransferManager`.
  GeneratedServicesArchiveTransferManagerCoreProxy get archiveTransferManager => GeneratedServicesArchiveTransferManagerCoreProxy._(bridge, 39);
  /// Returns a generated proxy client for `services.gitHubOAuthBrokerService`.
  GeneratedServicesGitHubOAuthBrokerServiceCoreProxy get gitHubOAuthBrokerService => GeneratedServicesGitHubOAuthBrokerServiceCoreProxy._(bridge, 40);
  /// Returns a generated proxy client for `services.localModelService`.
  GeneratedServicesLocalModelServiceCoreProxy get localModelService => GeneratedServicesLocalModelServiceCoreProxy._(bridge, 41);
  /// Returns a generated proxy client for `services.localProviderService`.
  GeneratedServicesLocalProviderServiceCoreProxy get localProviderService => GeneratedServicesLocalProviderServiceCoreProxy._(bridge, 42);
  /// Returns a generated proxy client for `services.runtimeBrowserService`.
  GeneratedServicesRuntimeBrowserServiceCoreProxy get runtimeBrowserService => GeneratedServicesRuntimeBrowserServiceCoreProxy._(bridge, 43);
  /// Returns a generated proxy client for `services.runtimeHostInfoService`.
  GeneratedServicesRuntimeHostInfoServiceCoreProxy get runtimeHostInfoService => GeneratedServicesRuntimeHostInfoServiceCoreProxy._(bridge, 44);
  /// Returns a generated proxy client for `services.runtimeHostInteractionService`.
  GeneratedServicesRuntimeHostInteractionServiceCoreProxy get runtimeHostInteractionService => GeneratedServicesRuntimeHostInteractionServiceCoreProxy._(bridge, 45);
  /// Returns a generated proxy client for `services.runtimeTerminalService`.
  GeneratedServicesRuntimeTerminalServiceCoreProxy get runtimeTerminalService => GeneratedServicesRuntimeTerminalServiceCoreProxy._(bridge, 46);
  /// Returns a generated proxy client for `services.snapshotImportManager`.
  GeneratedServicesSnapshotImportManagerCoreProxy get snapshotImportManager => GeneratedServicesSnapshotImportManagerCoreProxy._(bridge, 47);
  /// Returns a generated proxy client for `services.sttRecognitionService`.
  GeneratedServicesSttRecognitionServiceCoreProxy get sttRecognitionService => GeneratedServicesSttRecognitionServiceCoreProxy._(bridge, 48);
  /// Returns a generated proxy client for `services.syncBlobTransferManager`.
  GeneratedServicesSyncBlobTransferManagerCoreProxy get syncBlobTransferManager => GeneratedServicesSyncBlobTransferManagerCoreProxy._(bridge, 49);
  /// Returns a generated proxy client for `services.ttsPlaybackService`.
  GeneratedServicesTtsPlaybackServiceCoreProxy get ttsPlaybackService => GeneratedServicesTtsPlaybackServiceCoreProxy._(bridge, 50);
  /// Returns a generated proxy client for `services.ttsSynthesisService`.
  GeneratedServicesTtsSynthesisServiceCoreProxy get ttsSynthesisService => GeneratedServicesTtsSynthesisServiceCoreProxy._(bridge, 51);
  /// Returns a generated proxy client for `services.workspaceService`.
  GeneratedServicesWorkspaceServiceCoreProxy get workspaceService => GeneratedServicesWorkspaceServiceCoreProxy._(bridge, 52);
}

class GeneratedApplicationCoreProxy {
  const GeneratedApplicationCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Initializes persistent stores, prompt managers, tool handlers, plugins, and runtime events.
  Future<void> onCreate() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'onCreate',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Delivers one normalized host event to registered ToolPkg host-event hooks.
  Future<Object?> ingestRuntimeEvent({required RuntimeEvent event}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'ingestRuntimeEvent',
        args: _coreProxyArgs(<String, Object?>{'event': event.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Applies host-specific OpenMP environment setup before runtime services start.
  Future<void> configureOpenMpEnvironment() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'configureOpenMpEnvironment',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Ensures background work infrastructure is available for runtime tasks.
  Future<void> ensureWorkManagerInitialized() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'ensureWorkManagerInitialized',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Registers JSON serialization rules used by generated bridge payloads.
  Future<void> initializeJsonSerializer() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'initializeJsonSerializer',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Initializes application language resources before user-facing services are created.
  Future<void> initializeAppLanguage() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'initializeAppLanguage',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Initializes platform permission preference state used by Android-facing tools.
  Future<void> initAndroidPermissionPreferences() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'initAndroidPermissionPreferences',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Touches database-backed services early so schema setup happens during startup.
  Future<void> preloadDatabase() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'preloadDatabase',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Starts deployed MCP plugins according to the configured startup timeout.
  Future<void> initMcpPlugins() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'initMcpPlugins',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns the initialized tool handler owned by this runtime.
  GeneratedApplicationAiToolHandlerCoreProxy aiToolHandler() {
    return GeneratedApplicationAiToolHandlerCoreProxy._(bridge, 1, objectArgs: const <String, Object?>{});
  }

  /// Creates an MCP repository with this runtime's host and tool support.
  GeneratedApplicationMcpRepositoryCoreProxy mcpRepository() {
    return GeneratedApplicationMcpRepositoryCoreProxy._(bridge, 3, objectArgs: const <String, Object?>{});
  }

  /// Creates a skill repository with this runtime's host and tool support.
  GeneratedApplicationSkillRepositoryCoreProxy skillRepository() {
    return GeneratedApplicationSkillRepositoryCoreProxy._(bridge, 5, objectArgs: const <String, Object?>{});
  }

  /// Creates a user-markdown repository using this runtime's configured storage host.
  GeneratedApplicationUserMarkdownRepositoryCoreProxy userMarkdownRepository({required String ownerKey}) {
    return GeneratedApplicationUserMarkdownRepositoryCoreProxy._(bridge, 6, objectArgs: <String, Object?>{'__core_factory_arg_0': ownerKey});
  }

  /// Creates an input menu bridge backed by this application's tool package runtime.
  GeneratedApplicationInputMenuToggleBridgeCoreProxy inputMenuToggleBridge() {
    return GeneratedApplicationInputMenuToggleBridgeCoreProxy._(bridge, 2, objectArgs: const <String, Object?>{});
  }

  /// Returns the shared package manager owned by the initialized tool handler.
  GeneratedApplicationPackageManagerCoreProxy packageManager() {
    return GeneratedApplicationPackageManagerCoreProxy._(bridge, 4, objectArgs: const <String, Object?>{});
  }

  /// Returns package names enabled in this application runtime.
  Future<List<String>> activePackageNames() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'active_package_names',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<String>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Tests one model connection using this application's provider runtime.
  Future<ModelConnectionTestReport> testModelConnection({required String providerId, required String modelId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'test_model_connection',
        args: _coreProxyArgs(<String, Object?>{'provider_id': providerId, 'model_id': modelId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ModelConnectionTestReport>(responseBytes, decode: (reader) => ModelConnectionTestReport.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the Cargo package version compiled into the runtime crate.
  Future<String> coreVersion() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'coreVersion',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns structured in-memory application log entries.
  Future<Object?> logEntries() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'logEntries',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the application log file as text.
  Future<String> logText() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'logText',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the package-manager log file as text.
  Future<String> packageLogText() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'packageLogText',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the active application log file path.
  Future<String> logFilePath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'logFilePath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the active package-manager log file path.
  Future<String> packageLogFilePath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'packageLogFilePath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the user-visible Operit root directory path.
  Future<String> operitRootPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'operitRootPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the directory used for exported user artifacts.
  Future<String> exportsPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exportsPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the directory used for files removed during clean-on-exit maintenance.
  Future<String> cleanOnExitPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'cleanOnExitPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Clears the current runtime log files.
  Future<void> resetLogs() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resetLogs',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Combines sync clocks from key-value/object stores and SQL chat storage.
  Future<Object?> syncClock() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'syncClock',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Lists compacted sync operations newer than the provided device clock.
  Future<Object?> syncOperationsSince({required Object? clock, required List<String> domains, required int limit}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'syncOperationsSince',
        args: _coreProxyArgs(<String, Object?>{'clock': clock, 'domains': domains.map((item) => item).toList(growable: false), 'limit': limit}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Applies incoming sync operations to their owning persistent stores.
  Future<Object?> syncApplyOperations({required Object? operations}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'syncApplyOperations',
        args: _coreProxyArgs(<String, Object?>{'operations': operations}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Applies one route-transported Binding operation without moving persistent sync clocks.
  Future<Object?> syncApplyImmediateBindingOperation({required SyncOperation operation}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'syncApplyImmediateBindingOperation',
        args: _coreProxyArgs(<String, Object?>{'operation': operation.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reports whether one complete verified synchronization blob exists locally.
  Future<bool> syncBlobExists({required String contentHash, required int size}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'syncBlobExists',
        args: _coreProxyArgs(<String, Object?>{'contentHash': contentHash, 'size': size}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads one bounded synchronization blob chunk for transfer to another CoreNode.
  Future<Uint8List> syncReadBlobChunk({required String contentHash, required int offset, required int length}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'syncReadBlobChunk',
        args: _coreProxyArgs(<String, Object?>{'contentHash': contentHash, 'offset': offset, 'length': length}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Uint8List>(responseBytes, decode: (reader) => reader.readBytes(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Runs one CLI-style Core command through the generated application proxy.
  Future<Object?> runCoreCommand({required List<String> args}) {
    return bridge.callApplication('runCoreCommand', args: <String, Object?>{'args': args});
  }

}

class GeneratedApplicationAiToolHandlerCoreProxy {
  const GeneratedApplicationAiToolHandlerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Removes one registered tool and its visibility metadata.
  Future<void> unregisterTool({required String toolName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'unregisterTool',
        args: _coreProxyArgs(<String, Object?>{'toolName': toolName}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes all tools registered for an MCP server namespace.
  Future<int> unregisterMcpServerTools({required String serverName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'unregisterMcpServerTools',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Removes the package registration created for an MCP server.
  Future<bool> unregisterMcpServerPackage({required String serverName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'unregisterMcpServerPackage',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Removes a tool lifecycle hook by hook id.
  Future<void> removeToolHook({required String hookId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removeToolHook',
        args: _coreProxyArgs(<String, Object?>{'hookId': hookId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes every registered tool lifecycle hook.
  Future<void> clearToolHooks() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearToolHooks',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Notifies hooks that a tool call has been requested.
  Future<void> notifyToolCallRequested({required Object? tool}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'notifyToolCallRequested',
        args: _coreProxyArgs(<String, Object?>{'tool': tool}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Notifies hooks that permission was checked for a tool call.
  Future<void> notifyToolPermissionChecked({required Object? tool, required bool granted, required String? reason}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'notifyToolPermissionChecked',
        args: _coreProxyArgs(<String, Object?>{'tool': tool, 'granted': granted, 'reason': reason}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Notifies hooks that tool execution is about to start.
  Future<void> notifyToolExecutionStarted({required Object? tool}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'notifyToolExecutionStarted',
        args: _coreProxyArgs(<String, Object?>{'tool': tool}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Notifies hooks that tool execution returned a result.
  Future<void> notifyToolExecutionResult({required Object? tool, required Object? result}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'notifyToolExecutionResult',
        args: _coreProxyArgs(<String, Object?>{'tool': tool, 'result': result}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Notifies hooks that tool execution failed before producing a normal result.
  Future<void> notifyToolExecutionError({required Object? tool, required String message}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'notifyToolExecutionError',
        args: _coreProxyArgs(<String, Object?>{'tool': tool, 'message': message}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Notifies hooks that tool execution has fully finished.
  Future<void> notifyToolExecutionFinished({required Object? tool}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'notifyToolExecutionFinished',
        args: _coreProxyArgs(<String, Object?>{'tool': tool}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns every registered tool name regardless of visibility.
  Future<List<String>> getAllToolNames() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllToolNames',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<String>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the host environment descriptor associated with this handler.
  Future<HostEnvironmentDescriptor> getHostEnvironmentDescriptor() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getHostEnvironmentDescriptor',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<HostEnvironmentDescriptor>(responseBytes, decode: (reader) => HostEnvironmentDescriptor.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns tool names that should be visible to normal callers.
  Future<List<String>> getPublicToolNames() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPublicToolNames',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<String>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns tool names reserved for internal runtime calls.
  Future<List<String>> getInternalToolNames() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getInternalToolNames',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<String>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the configured visibility for one tool.
  Future<ToolRegistrationVisibility?> getToolVisibility({required String toolName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolVisibility',
        args: _coreProxyArgs(<String, Object?>{'toolName': toolName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ToolRegistrationVisibility?>(responseBytes, decode: (reader) => reader.readNullable<ToolRegistrationVisibility>(() => ToolRegistrationVisibility.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Registers built-in public and internal tools once for this handler.
  Future<void> registerDefaultTools() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'registerDefaultTools',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns whether a tool executor is already registered.
  Future<bool> hasToolExecutor({required String toolName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'hasToolExecutor',
        args: _coreProxyArgs(<String, Object?>{'toolName': toolName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Ensures default or package tools are registered, then reports whether a tool exists.
  Future<bool> getToolExecutorOrActivate({required String toolName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolExecutorOrActivate',
        args: _coreProxyArgs(<String, Object?>{'toolName': toolName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Executes an AI-originated tool through permission checks and a resolved executor.
  Future<List<CoreOperitToolsConversationMarkupManagerToolResult>?> executeToolSafelyWithResolvedExecutor({required Object? tool}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'executeToolSafelyWithResolvedExecutor',
        args: _coreProxyArgs(<String, Object?>{'tool': tool}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<CoreOperitToolsConversationMarkupManagerToolResult>?>(responseBytes, decode: (reader) => reader.readNullable<List<CoreOperitToolsConversationMarkupManagerToolResult>>(() => (() { final length = reader.readArrayLength(); return List<CoreOperitToolsConversationMarkupManagerToolResult>.generate(length, (_) => CoreOperitToolsConversationMarkupManagerToolResult.fromMessagePack(reader), growable: false); })()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Executes a non-AI tool directly through a resolved executor.
  Future<List<CoreOperitToolsConversationMarkupManagerToolResult>?> executeToolDirectlyWithResolvedExecutor({required Object? tool}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'executeToolDirectlyWithResolvedExecutor',
        args: _coreProxyArgs(<String, Object?>{'tool': tool}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<CoreOperitToolsConversationMarkupManagerToolResult>?>(responseBytes, decode: (reader) => reader.readNullable<List<CoreOperitToolsConversationMarkupManagerToolResult>>(() => (() { final length = reader.readArrayLength(); return List<CoreOperitToolsConversationMarkupManagerToolResult>.generate(length, (_) => CoreOperitToolsConversationMarkupManagerToolResult.fromMessagePack(reader), growable: false); })()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Resolves and directly executes a non-AI tool request through the registered tool chain.
  Future<CoreOperitToolsConversationMarkupManagerToolResult> executeTool({required CoreOperitToolsToolExecutionManagerAiTool tool}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'executeTool',
        args: _coreProxyArgs(<String, Object?>{'tool': tool.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreOperitToolsConversationMarkupManagerToolResult>(responseBytes, decode: (reader) => CoreOperitToolsConversationMarkupManagerToolResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Clears registered tools and runtime-only handler state.
  Future<void> reset() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'reset',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedApplicationInputMenuToggleBridgeCoreProxy {
  const GeneratedApplicationInputMenuToggleBridgeCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Generated proxy for `createToggleDefinitions`.
  Future<List<InputMenuToggleDefinitionSnapshot>> createToggleDefinitions({required String? chatId, required Map<String, bool> featureStates, required String? runtime}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createToggleDefinitions',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'featureStates': featureStates.map((key, value) => MapEntry(key, value)), 'runtime': runtime}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<InputMenuToggleDefinitionSnapshot>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<InputMenuToggleDefinitionSnapshot>.generate(length, (_) => InputMenuToggleDefinitionSnapshot.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Builds input-menu toggle definitions for Flutter from active tool package hooks.
  Future<List<InputMenuToggleDefinitionSnapshot>> createToggleDefinitionsForFlutter({required String? chatId, required Map<String, bool> featureStates, required String? runtime}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createToggleDefinitionsForFlutter',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'featureStates': featureStates.map((key, value) => MapEntry(key, value)), 'runtime': runtime}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<InputMenuToggleDefinitionSnapshot>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<InputMenuToggleDefinitionSnapshot>.generate(length, (_) => InputMenuToggleDefinitionSnapshot.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Generated proxy for `triggerToggle`.
  Future<bool> triggerToggle({required String toggleId, required String? chatId, required String? runtime}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'triggerToggle',
        args: _coreProxyArgs(<String, Object?>{'toggleId': toggleId, 'chatId': chatId, 'runtime': runtime}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Runs a Flutter-requested input-menu toggle hook by toggle identifier.
  Future<bool> triggerToggleForFlutter({required String toggleId, required String? chatId, required String? runtime}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'triggerToggleForFlutter',
        args: _coreProxyArgs(<String, Object?>{'toggleId': toggleId, 'chatId': chatId, 'runtime': runtime}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reports the input-menu toggle change counter for Flutter watchers.
  Future<int> changeVersion() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'changeVersion',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedApplicationMcpRepositoryCoreProxy {
  const GeneratedApplicationMcpRepositoryCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Installs an MCP server from a repository URL for Flutter bridge callers.
  Future<String> installMcpServerWithObjectForFlutter({required String pluginId, required String repoUrl, required String name, required String description, required String mcpConfig}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'installMCPServerWithObjectForFlutter',
        args: _coreProxyArgs(<String, Object?>{'pluginId': pluginId, 'repoUrl': repoUrl, 'name': name, 'description': description, 'mcpConfig': mcpConfig}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Installs an MCP server from a local zip for Flutter bridge callers.
  Future<String> installMcpServerFromZipForFlutter({required String pluginId, required String zipPath, required String name, required String description, required String mcpConfig}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'installMCPServerFromZipForFlutter',
        args: _coreProxyArgs(<String, Object?>{'pluginId': pluginId, 'zipPath': zipPath, 'name': name, 'description': description, 'mcpConfig': mcpConfig}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Generates a concise plugin description from available MCP tool descriptions.
  Future<String> generatePluginDescription({required String pluginId, required String pluginName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'generatePluginDescription',
        args: _coreProxyArgs(<String, Object?>{'pluginId': pluginId, 'pluginName': pluginName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedApplicationPackageManagerCoreProxy {
  const GeneratedApplicationPackageManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Marks a package as active for the current prompt session.
  Future<bool> activatePackage({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'activatePackage',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Releases one explicitly owned ToolPkg execution engine.
  Future<void> releaseToolPkgExecutionEngine({required String contextKey, required String containerPackageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'releaseToolPkgExecutionEngine',
        args: _coreProxyArgs(<String, Object?>{'contextKey': contextKey, 'containerPackageName': containerPackageName}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Acquires one explicit owner lease for a ToolPkg execution engine.
  Future<void> acquireToolPkgExecutionEngine({required String contextKey, required String containerPackageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'acquireToolPkgExecutionEngine',
        args: _coreProxyArgs(<String, Object?>{'contextKey': contextKey, 'containerPackageName': containerPackageName}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Executes a Compose DSL render through the host-owned asynchronous JavaScript boundary.
  Future<String?> executeToolPkgComposeDslScript({required String contextKey, required String containerPackageName, required String script, required Map<String, Object?> runtimeOptions, required Map<String, String> envOverrides}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'executeToolPkgComposeDslScript',
        args: _coreProxyArgs(<String, Object?>{'contextKey': contextKey, 'containerPackageName': containerPackageName, 'script': script, 'runtimeOptions': runtimeOptions.map((key, value) => MapEntry(key, value)), 'envOverrides': envOverrides.map((key, value) => MapEntry(key, value))}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Dispatches a Compose DSL action and immediately streams intermediate render events.
  Stream<String> dispatchToolPkgComposeDslActionEvents({required String contextKey, required String containerPackageName, required String actionId, required Object? payload, required Map<String, Object?> runtimeOptions, required Map<String, String> envOverrides}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'dispatchToolPkgComposeDslActionEvents', args: _coreProxyArgs(<String, Object?>{'contextKey': contextKey, 'containerPackageName': containerPackageName, 'actionId': actionId, 'payload': payload, 'runtimeOptions': runtimeOptions.map((key, value) => MapEntry(key, value)), 'envOverrides': envOverrides.map((key, value) => MapEntry(key, value))}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<String>(event, decode: (valueBytes) => decodeCoreLink<String>(valueBytes, decode: (reader) => reader.readString(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns whether a package has been activated for the current prompt session.
  Future<bool> isPackageActivated({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isPackageActivated',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Activates a package and returns its system prompt contribution.
  Future<String> usePackage({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'usePackage',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Executes the built-in package activation tool.
  Future<CoreOperitToolsConversationMarkupManagerToolResult> executeUsePackageTool({required String toolName, required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'executeUsePackageTool',
        args: _coreProxyArgs(<String, Object?>{'toolName': toolName, 'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreOperitToolsConversationMarkupManagerToolResult>(responseBytes, decode: (reader) => CoreOperitToolsConversationMarkupManagerToolResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns package names enabled in preferences after applying disabled package records.
  Future<List<String>> getEnabledPackageNames() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getEnabledPackageNames',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<String>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether a package is enabled and not disabled by ToolPkg subpackage state.
  Future<bool> isPackageEnabled({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isPackageEnabled',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns package names currently active in the prompt session.
  Future<List<String>> getActivePackageNames() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getActivePackageNames',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<String>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Enables a package and loads its tools into available package state.
  Future<String> enablePackage({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'enablePackage',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Disables a package and removes its tools from active package state.
  Future<String> disablePackage({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'disablePackage',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes an external package from storage and package state.
  Future<bool> deletePackage({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deletePackage',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Enables a ToolPkg container through the normal package enable flow.
  Future<String> enableToolPkgContainer({required String containerPackageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'enableToolPkgContainer',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Disables a ToolPkg container through the normal package disable flow.
  Future<String> disableToolPkgContainer({required String containerPackageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'disableToolPkgContainer',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether a package name belongs to a ToolPkg container runtime.
  Future<bool> isToolPkgContainer({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isToolPkgContainer',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether a package name is a ToolPkg subpackage.
  Future<bool> isToolPkgSubpackage({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isToolPkgSubpackage',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether a package is visible as a top-level package.
  Future<bool> isTopLevelPackage({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isTopLevelPackage',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns available packages excluding ToolPkg subpackages.
  Future<Map<String, ToolPackage>> getTopLevelAvailablePackages() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getTopLevelAvailablePackages',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, ToolPackage>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, ToolPackage>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = ToolPackage.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns packages that can be executed directly as tools.
  Future<Map<String, ToolPackage>> getExecutableAvailablePackages() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getExecutableAvailablePackages',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, ToolPackage>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, ToolPackage>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = ToolPackage.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns localized details for all registered ToolPkg containers.
  Future<List<ToolPkgContainerDetails>> getToolPkgPluginContainerDetails({required bool useEnglish}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgPluginContainerDetails',
        args: _coreProxyArgs(<String, Object?>{'useEnglish': useEnglish}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ToolPkgContainerDetails>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ToolPkgContainerDetails>.generate(length, (_) => ToolPkgContainerDetails.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns ToolPkg container runtimes that are currently enabled.
  Future<List<ToolPkgContainerRuntime>> getEnabledToolPkgContainerRuntimes() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getEnabledToolPkgContainerRuntimes',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ToolPkgContainerRuntime>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ToolPkgContainerRuntime>.generate(length, (_) => ToolPkgContainerRuntime.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns all registered ToolPkg container runtimes.
  Future<List<ToolPkgContainerRuntime>> getToolPkgContainerRuntimes() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgContainerRuntimes',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ToolPkgContainerRuntime>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ToolPkgContainerRuntime>.generate(length, (_) => ToolPkgContainerRuntime.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns localized details for a ToolPkg container.
  Future<ToolPkgContainerDetails?> getToolPkgContainerDetails({required String packageName, required bool useEnglish}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgContainerDetails',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName, 'useEnglish': useEnglish}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ToolPkgContainerDetails?>(responseBytes, decode: (reader) => reader.readNullable<ToolPkgContainerDetails>(() => ToolPkgContainerDetails.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns UI routes exposed by ToolPkg modules for one runtime target.
  Future<List<ToolPkgUiRoute>> getToolPkgUiRoutes({required String runtime, required bool useEnglish}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgUiRoutes',
        args: _coreProxyArgs(<String, Object?>{'runtime': runtime, 'useEnglish': useEnglish}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ToolPkgUiRoute>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ToolPkgUiRoute>.generate(length, (_) => ToolPkgUiRoute.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns desktop widgets exposed by enabled ToolPkg containers.
  Future<List<ToolPkgDesktopWidget>> getToolPkgDesktopWidgets({required bool useEnglish}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgDesktopWidgets',
        args: _coreProxyArgs(<String, Object?>{'useEnglish': useEnglish}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ToolPkgDesktopWidget>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ToolPkgDesktopWidget>.generate(length, (_) => ToolPkgDesktopWidget.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns navigation entries exposed by enabled ToolPkg containers.
  Future<List<ToolPkgNavigationEntry>> getToolPkgNavigationEntries({required bool useEnglish}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgNavigationEntries',
        args: _coreProxyArgs(<String, Object?>{'useEnglish': useEnglish}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ToolPkgNavigationEntry>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ToolPkgNavigationEntry>.generate(length, (_) => ToolPkgNavigationEntry.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns workspace templates exposed by enabled ToolPkg containers.
  Future<List<ToolPkgWorkspaceTemplate>> getToolPkgWorkspaceTemplates({required bool useEnglish}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgWorkspaceTemplates',
        args: _coreProxyArgs(<String, Object?>{'useEnglish': useEnglish}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ToolPkgWorkspaceTemplate>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ToolPkgWorkspaceTemplate>.generate(length, (_) => ToolPkgWorkspaceTemplate.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports a ToolPkg workspace template into a destination directory.
  Future<ToolPkgWorkspaceTemplateImportResult> importToolPkgWorkspaceTemplate({required String containerPackageName, required String templateId, required String destinationDir}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importToolPkgWorkspaceTemplate',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName, 'templateId': templateId, 'destinationDir': destinationDir}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ToolPkgWorkspaceTemplateImportResult>(responseBytes, decode: (reader) => ToolPkgWorkspaceTemplateImportResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates the enabled state for a ToolPkg subpackage.
  Future<bool> setToolPkgSubpackageEnabled({required String subpackagePackageName, required bool enabled}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setToolPkgSubpackageEnabled',
        args: _coreProxyArgs(<String, Object?>{'subpackagePackageName': subpackagePackageName, 'enabled': enabled}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Resolves the best package name for a ToolPkg subpackage id.
  Future<String?> findPreferredPackageNameForSubpackageId({required String subpackageId, required bool preferEnabled}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'findPreferredPackageNameForSubpackageId',
        args: _coreProxyArgs(<String, Object?>{'subpackageId': subpackageId, 'preferEnabled': preferEnabled}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Runs a ToolPkg navigation entry action hook.
  Future<String?> runToolPkgNavigationEntryAction({required String containerPackageName, required String entryId, required String functionName, required String? inlineFunctionSource, required Object? eventPayload}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'runToolPkgNavigationEntryAction',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName, 'entryId': entryId, 'functionName': functionName, 'inlineFunctionSource': inlineFunctionSource, 'eventPayload': eventPayload}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Lists bundled external standalone packages that are not currently loaded.
  Future<List<BundledExternalPackageCandidate>> getBundledExternalPackageCandidates() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getBundledExternalPackageCandidates',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<BundledExternalPackageCandidate>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<BundledExternalPackageCandidate>.generate(length, (_) => BundledExternalPackageCandidate.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Lists bundled external ToolPkg containers that are not currently loaded.
  Future<List<ToolPkgContainerRuntime>> getBundledExternalToolPkgContainerRuntimes() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getBundledExternalToolPkgContainerRuntimes',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ToolPkgContainerRuntime>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ToolPkgContainerRuntime>.generate(length, (_) => ToolPkgContainerRuntime.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports a bundled external standalone package into local package storage.
  Future<String> importBundledExternalPackage({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importBundledExternalPackage',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports a bundled external ToolPkg container into local package storage.
  Future<String> importBundledExternalToolPkgContainer({required String containerPackageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importBundledExternalToolPkgContainer',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime metadata for a ToolPkg container.
  Future<ToolPkgContainerRuntime?> getToolPkgContainerRuntime({required String containerPackageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgContainerRuntime',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ToolPkgContainerRuntime?>(responseBytes, decode: (reader) => reader.readNullable<ToolPkgContainerRuntime>(() => ToolPkgContainerRuntime.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Resolves ToolPkg subpackage runtime metadata by package name.
  Future<ToolPkgSubpackageRuntime?> resolveToolPkgSubpackageRuntimeInternal({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resolveToolPkgSubpackageRuntimeInternal',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ToolPkgSubpackageRuntime?>(responseBytes, decode: (reader) => reader.readNullable<ToolPkgSubpackageRuntime>(() => ToolPkgSubpackageRuntime.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns package tools with active state applied.
  Future<ToolPackage?> getEffectivePackageTools({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getEffectivePackageTools',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ToolPackage?>(responseBytes, decode: (reader) => reader.readNullable<ToolPackage>(() => ToolPackage.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns raw package tool metadata by package name.
  Future<ToolPackage?> getPackageTools({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPackageTools',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ToolPackage?>(responseBytes, decode: (reader) => reader.readNullable<ToolPackage>(() => ToolPackage.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the first tool script associated with a package.
  Future<String?> getPackageScript({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPackageScript',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the active state id selected for a package.
  Future<String?> getActivePackageStateId({required String packageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getActivePackageStateId',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns all package definitions currently available to the manager.
  Future<Map<String, ToolPackage>> getAvailablePackages() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAvailablePackages',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, ToolPackage>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, ToolPackage>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = ToolPackage.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns MCP server packages registered with the package manager.
  Future<Map<String, McpServerConfig>> getAvailableServerPackages() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAvailableServerPackages',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, McpServerConfig>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, McpServerConfig>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = McpServerConfig.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Removes an MCP server package from package manager state.
  Future<bool> unregisterMcpServerPackage({required String serverName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'unregisterMCPServerPackage',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Registers or replaces an available package definition.
  Future<void> setAvailablePackage({required String packageName, required ToolPackage toolPackage}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setAvailablePackage',
        args: _coreProxyArgs(<String, Object?>{'packageName': packageName, 'toolPackage': toolPackage.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Registers a loaded ToolPkg container and its subpackages.
  Future<bool> registerToolPkg({required ToolPkgLoadResult loadResult}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'registerToolPkg',
        args: _coreProxyArgs(<String, Object?>{'loadResult': loadResult.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the external package storage path as display text.
  Future<String> getExternalPackagesPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getExternalPackagesPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Scans built-in, bundled external, and external package sources.
  Future<void> loadAvailablePackages() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'loadAvailablePackages',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns whether the ToolPkg protection secret is configured.
  Future<bool> isToolPkgProtectionSecretConfigured() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isToolPkgProtectionSecretConfigured',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Protects a local JS or ToolPkg artifact before marketplace upload.
  Future<Uint8List> protectArtifactFile({required String sourcePath, required bool isToolPkg, required String packageId, required String version, required List<String> author, required bool minifyArtifact}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'protectArtifactFile',
        args: _coreProxyArgs(<String, Object?>{'sourcePath': sourcePath, 'isToolPkg': isToolPkg, 'packageId': packageId, 'version': version, 'author': author.map((item) => item).toList(growable: false), 'minifyArtifact': minifyArtifact}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Uint8List>(responseBytes, decode: (reader) => reader.readBytes(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns package sources that can be exported or published.
  Future<List<PublishablePackageSource>> getPublishablePackageSources() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPublishablePackageSources',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<PublishablePackageSource>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<PublishablePackageSource>.generate(length, (_) => PublishablePackageSource.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports a package file from external storage into package storage.
  Future<String> addPackageFileFromExternalStorage({required String filePath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'addPackageFileFromExternalStorage',
        args: _coreProxyArgs(<String, Object?>{'filePath': filePath}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports one marketplace artifact from bytes after validating its declared digest.
  Future<String> addMarketArtifactBytes({required Uint8List bytes, required String fileName, required String expectedSha256}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'addMarketArtifactBytes',
        args: _coreProxyArgs(<String, Object?>{'bytes': bytes, 'fileName': fileName, 'expectedSha256': expectedSha256}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Installs one signed marketplace ToolPkg as a locally authenticated package archive.
  Future<String> addMarketToolPkgFileFromExternalStorage({required String filePath, required String expectedMarketAssetSha256}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'addMarketToolPkgFileFromExternalStorage',
        args: _coreProxyArgs(<String, Object?>{'filePath': filePath, 'expectedMarketAssetSha256': expectedMarketAssetSha256}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Persists the complete enabled package name list.
  Future<void> setEnabledPackageNames({required Object? packageNames}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setEnabledPackageNames',
        args: _coreProxyArgs(<String, Object?>{'packageNames': packageNames}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Registers or replaces an MCP server package definition.
  Future<void> setAvailableServerPackage({required String serverName, required McpServerConfig serverConfig}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setAvailableServerPackage',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName, 'serverConfig': serverConfig.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns the main script for an enabled ToolPkg container.
  Future<String?> getToolPkgMainScriptInternal({required String containerPackageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgMainScriptInternal',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the registered main script for runtime-owned ToolPkg IPC dispatch.
  Future<String?> getRegisteredToolPkgMainScript({required String containerPackageName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getRegisteredToolPkgMainScript',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads a text resource from a ToolPkg container or subpackage.
  Future<String?> readToolPkgTextResource({required String packageNameOrSubpackageId, required String resourcePath, required bool preferEnabledContainer}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'readToolPkgTextResource',
        args: _coreProxyArgs(<String, Object?>{'packageNameOrSubpackageId': packageNameOrSubpackageId, 'resourcePath': resourcePath, 'preferEnabledContainer': preferEnabledContainer}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Calls one declared ToolPkg WASM export.
  Future<JsToolPkgWasmResult> callToolPkgWasm({required JsToolPkgWasmRequest request, required bool preferEnabledContainer}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'callToolPkgWasm',
        args: _coreProxyArgs(<String, Object?>{'request': request.toJson(), 'preferEnabledContainer': preferEnabledContainer}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<JsToolPkgWasmResult>(responseBytes, decode: (reader) => JsToolPkgWasmResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Copies a ToolPkg resource selected by subpackage id to a file.
  Future<bool> copyToolPkgResourceToFileBySubpackageId({required String subpackageId, required String resourceKey, required Object? destinationFile, required bool preferEnabledContainer}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'copyToolPkgResourceToFileBySubpackageId',
        args: _coreProxyArgs(<String, Object?>{'subpackageId': subpackageId, 'resourceKey': resourceKey, 'destinationFile': destinationFile, 'preferEnabledContainer': preferEnabledContainer}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Copies a ToolPkg resource from a container to a file.
  Future<bool> copyToolPkgResourceToFile({required String containerPackageName, required String resourceKey, required Object? destinationFile}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'copyToolPkgResourceToFile',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName, 'resourceKey': resourceKey, 'destinationFile': destinationFile}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the output file name declared for a ToolPkg resource.
  Future<String?> getToolPkgResourceOutputFileName({required String packageNameOrSubpackageId, required String resourceKey, required bool preferEnabledContainer}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgResourceOutputFileName',
        args: _coreProxyArgs(<String, Object?>{'packageNameOrSubpackageId': packageNameOrSubpackageId, 'resourceKey': resourceKey, 'preferEnabledContainer': preferEnabledContainer}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns Compose DSL script text selected through a ToolPkg subpackage id.
  Future<String?> getToolPkgComposeDslScriptBySubpackageId({required String subpackageId, required String? uiModuleId, required bool preferEnabledContainer}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgComposeDslScriptBySubpackageId',
        args: _coreProxyArgs(<String, Object?>{'subpackageId': subpackageId, 'uiModuleId': uiModuleId, 'preferEnabledContainer': preferEnabledContainer}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns Compose DSL script text for a ToolPkg UI module.
  Future<String?> getToolPkgComposeDslScript({required String containerPackageName, required String? uiModuleId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgComposeDslScript',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName, 'uiModuleId': uiModuleId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the Compose DSL screen path for a ToolPkg UI module.
  Future<String?> getToolPkgComposeDslScreenPath({required String containerPackageName, required String? uiModuleId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgComposeDslScreenPath',
        args: _coreProxyArgs(<String, Object?>{'containerPackageName': containerPackageName, 'uiModuleId': uiModuleId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Activates an MCP server package and returns its system prompt contribution.
  Future<String> useMcpServer({required String serverName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'useMCPServer',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedApplicationSkillRepositoryCoreProxy {
  const GeneratedApplicationSkillRepositoryCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Returns the directory where user-installed skills are stored.
  Future<String> getSkillsDirectoryPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getSkillsDirectoryPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns all valid installed skill packages.
  Future<Map<String, SkillPackage>> getAvailableSkillPackages() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAvailableSkillPackages',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, SkillPackage>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, SkillPackage>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = SkillPackage.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns valid installed skill packages together with scan errors.
  Future<Object?> getAvailableSkillPackagesSnapshot() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAvailableSkillPackagesSnapshot',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns skill directory scan errors keyed by directory name.
  Future<Map<String, String>> getSkillLoadErrors() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getSkillLoadErrors',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, String>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, String>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = reader.readString(); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Lists bundled external skills that are available for installation.
  Future<List<BundledExternalSkillCandidate>> getBundledExternalSkillCandidates() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getBundledExternalSkillCandidates',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<BundledExternalSkillCandidate>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<BundledExternalSkillCandidate>.generate(length, (_) => BundledExternalSkillCandidate.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Installs one bundled external skill.
  Future<SkillPackage> importBundledExternalSkill({required String skillName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importBundledExternalSkill',
        args: _coreProxyArgs(<String, Object?>{'skillName': skillName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SkillPackage>(responseBytes, decode: (reader) => SkillPackage.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns installed skill packages that are visible to AI package activation.
  Future<Map<String, SkillPackage>> getAiVisibleSkillPackages() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAiVisibleSkillPackages',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, SkillPackage>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, SkillPackage>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = SkillPackage.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the SKILL.md content for one installed skill.
  Future<String?> readSkillContent({required String skillName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'readSkillContent',
        args: _coreProxyArgs(<String, Object?>{'skillName': skillName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes one installed skill directory.
  Future<bool> deleteSkill({required String skillName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteSkill',
        args: _coreProxyArgs(<String, Object?>{'skillName': skillName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether one skill is visible to AI package activation.
  Future<bool> isSkillVisibleToAi({required String skillName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isSkillVisibleToAi',
        args: _coreProxyArgs(<String, Object?>{'skillName': skillName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Sets whether one skill is visible to AI package activation.
  Future<void> setSkillVisibleToAi({required String skillName, required bool visible}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setSkillVisibleToAi',
        args: _coreProxyArgs(<String, Object?>{'skillName': skillName, 'visible': visible}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Installs the quick plugin creator skill and marks it visible to AI.
  Future<SkillPackage> ensureQuickPluginCreatorSkillVisible() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'ensureQuickPluginCreatorSkillVisible',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SkillPackage>(responseBytes, decode: (reader) => SkillPackage.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports a skill from a zip archive by searching for SKILL.md.
  Future<String> importSkillFromZip({required Object? zipFile}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importSkillFromZip',
        args: _coreProxyArgs(<String, Object?>{'zipFile': zipFile}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports a skill from a zip archive using an optional subdirectory inside the zip.
  Future<String> importSkillFromZipWithSubDir({required Object? zipFile, required String? subDirPathInZip}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importSkillFromZipWithSubDir',
        args: _coreProxyArgs(<String, Object?>{'zipFile': zipFile, 'subDirPathInZip': subDirPathInZip}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Downloads a GitHub repository zip and imports a skill from it.
  Future<String> importSkillFromGitHubRepo({required String repoUrl}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importSkillFromGitHubRepo',
        args: _coreProxyArgs(<String, Object?>{'repoUrl': repoUrl}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a skill directly from text content and copied attachment files.
  Future<String> importSkillFromDirectInput({required String skillId, required String description, required String content, required Object? attachmentPaths}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importSkillFromDirectInput',
        args: _coreProxyArgs(<String, Object?>{'skillId': skillId, 'description': description, 'content': content, 'attachmentPaths': attachmentPaths}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedApplicationUserMarkdownRepositoryCoreProxy {
  const GeneratedApplicationUserMarkdownRepositoryCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Reads the user markdown file after ensuring it exists.
  Future<String> readUserMarkdown() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'readUserMarkdown',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Writes normalized content to the user markdown file.
  Future<void> writeUserMarkdown({required String content}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'writeUserMarkdown',
        args: _coreProxyArgs(<String, Object?>{'content': content}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedChatRuntimeHolderMainCoreProxy {
  const GeneratedChatRuntimeHolderMainCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Dispatches chat input change notifications from host-owned input widgets.
  Future<void> dispatchChatInputChanged({required String? chatIdOverride, required String messageText, required int selectionStart, required int selectionEnd, required int attachmentCount}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'dispatchChatInputChanged',
        args: _coreProxyArgs(<String, Object?>{'chatIdOverride': chatIdOverride, 'messageText': messageText, 'selectionStart': selectionStart, 'selectionEnd': selectionEnd, 'attachmentCount': attachmentCount}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Dispatches submit_requested and returns the ToolPkg decision for the host input widget.
  Future<Object?> dispatchChatInputSubmitRequested({required String? chatIdOverride, required String messageText, required int selectionStart, required int selectionEnd, required int attachmentCount}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'dispatchChatInputSubmitRequested',
        args: _coreProxyArgs(<String, Object?>{'chatIdOverride': chatIdOverride, 'messageText': messageText, 'selectionStart': selectionStart, 'selectionEnd': selectionEnd, 'attachmentCount': attachmentCount}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Sends a user-authored message through the active chat runtime.
  Future<void> sendUserMessage({required PromptFunctionType promptFunctionType, required String? roleCardIdOverride, required String? chatIdOverride, required String messageText, required String? proxySenderNameOverride, required String? chatProviderIdOverride, required String? chatModelIdOverride, required List<AttachmentInfo> attachments, required ChatMessage? replyToMessage, required ChatTurnOptions turnOptions}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'sendUserMessage',
        args: _coreProxyArgs(<String, Object?>{'promptFunctionType': promptFunctionType.toJson(), 'roleCardIdOverride': roleCardIdOverride, 'chatIdOverride': chatIdOverride, 'messageText': messageText, 'proxySenderNameOverride': proxySenderNameOverride, 'chatProviderIdOverride': chatProviderIdOverride, 'chatModelIdOverride': chatModelIdOverride, 'attachments': attachments.map((item) => item.toJson()).toList(growable: false), 'replyToMessage': replyToMessage?.toJson(), 'turnOptions': turnOptions.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Resumes an AI round on the CoreNode that already owns the chat Binding.
  Future<void> resume({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resume',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Marks the source chat as paused while route synchronization is in progress.
  Future<void> beforeChangeRoute({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'beforeChangeRoute',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Resumes the target chat after the route change reaches the selected Core.
  Future<void> afterChangeRoute({required String chatId, required CoreRouteResumeContext resumeContext}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'afterChangeRoute',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'resumeContext': resumeContext.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Cancels message generation for a specific chat id.
  Future<void> cancelMessage({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'cancelMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Adds one message to the queue owned by a specific chat.
  Future<void> enqueuePendingQueueMessage({required String chatId, required String messageText}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'enqueuePendingQueueMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageText': messageText}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Deletes one queued message from a specific chat.
  Future<void> deletePendingQueueMessage({required String chatId, required int messageId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deletePendingQueueMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageId': messageId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes one queued message for editing or explicit user delivery.
  Future<PendingQueueMessageItem?> takePendingQueueMessage({required String chatId, required int messageId, required bool suppressNextAutoDequeue}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'takePendingQueueMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageId': messageId, 'suppressNextAutoDequeue': suppressNextAutoDequeue}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<PendingQueueMessageItem?>(responseBytes, decode: (reader) => reader.readNullable<PendingQueueMessageItem>(() => PendingQueueMessageItem.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Clears a manual-send suppression after that message is not delivered.
  Future<void> clearPendingQueueAutoDequeueSuppression({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearPendingQueueAutoDequeueSuppression',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Atomically removes the next queued message after a chat becomes ready.
  Future<PendingQueueMessageItem?> takeNextPendingQueueMessageIfReady({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'takeNextPendingQueueMessageIfReady',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<PendingQueueMessageItem?>(responseBytes, decode: (reader) => reader.readNullable<PendingQueueMessageItem>(() => PendingQueueMessageItem.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Inserts a rejected queued message back at the front of its chat queue.
  Future<void> restorePendingQueueMessage({required String chatId, required PendingQueueMessageItem message}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'restorePendingQueueMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'message': message.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Updates whether a chat's pending-message queue is expanded in the UI.
  Future<void> setPendingQueueExpanded({required String chatId, required bool isExpanded}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setPendingQueueExpanded',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'isExpanded': isExpanded}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Splits markdown content into stable render events for the client.
  Future<List<MarkdownStreamEvent>> splitMarkdownContent({required String content}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'splitMarkdownContent',
        args: _coreProxyArgs(<String, Object?>{'content': content}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<MarkdownStreamEvent>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<MarkdownStreamEvent>.generate(length, (_) => MarkdownStreamEvent.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Renders one XML block through registered ToolPkg XML render hooks.
  Future<Object?> renderToolPkgXml({required String tagName, required String xmlContent}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'renderToolPkgXml',
        args: _coreProxyArgs(<String, Object?>{'tagName': tagName, 'xmlContent': xmlContent}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a new chat and makes it available through chat history state.
  Future<void> createNewChat({required String? characterCardName, required String? group, required bool inheritGroupFromCurrent, required bool setAsCurrentChat, required String? characterGroupId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createNewChat',
        args: _coreProxyArgs(<String, Object?>{'characterCardName': characterCardName, 'group': group, 'inheritGroupFromCurrent': inheritGroupFromCurrent, 'setAsCurrentChat': setAsCurrentChat, 'characterGroupId': characterGroupId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Switches the active chat and refreshes its runtime state.
  Future<void> switchChat({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'switchChat',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Switches the local runtime selection without writing the global chat selection.
  Future<void> switchChatLocal({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'switchChatLocal',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Changes the active character card target used when new chat turns are sent.
  Future<void> switchActiveCharacterCardTarget({required String characterCardId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'switchActiveCharacterCardTarget',
        args: _coreProxyArgs(<String, Object?>{'characterCardId': characterCardId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Changes the active character group target used when new group chat turns are sent.
  Future<void> switchActiveCharacterGroupTarget({required String characterGroupId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'switchActiveCharacterGroupTarget',
        args: _coreProxyArgs(<String, Object?>{'characterGroupId': characterGroupId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Updates the character card binding stored on an existing chat.
  Future<void> updateChatCharacterCard({required String chatId, required String? characterCardName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateChatCharacterCard',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'characterCardName': characterCardName}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Updates the character group binding stored on an existing chat.
  Future<void> updateChatCharacterGroup({required String chatId, required String? characterGroupId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateChatCharacterGroup',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'characterGroupId': characterGroupId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Synchronizes the current runtime chat id to the global chat selection.
  Future<void> syncCurrentChatIdToGlobal() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'syncCurrentChatIdToGlobal',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Deletes a chat history and updates current chat selection.
  Future<bool> deleteChatHistory({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteChatHistory',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes one message from an explicit chat by message timestamp.
  Future<void> deleteMessage({required String chatId, required int messageTimestamp}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageTimestamp': messageTimestamp}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Deletes multiple messages from an explicit chat by message timestamps.
  Future<bool> deleteMessages({required String chatId, required List<int> messageTimestamps}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteMessages',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageTimestamps': messageTimestamps.map((item) => item).toList(growable: false)}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Replaces the content of one message and refreshes the stable context window.
  Future<bool> updateMessage({required String chatId, required int messageTimestamp, required String editedContent}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageTimestamp': messageTimestamp, 'editedContent': editedContent}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes the selected message and every following message in an explicit chat.
  Future<bool> deleteMessagesFrom({required String chatId, required int messageTimestamp}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteMessagesFrom',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageTimestamp': messageTimestamp}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes one alternate response variant from a message timestamp.
  Future<void> deleteMessageVariant({required int timestamp, required int variantIndex}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteMessageVariant',
        args: _coreProxyArgs(<String, Object?>{'timestamp': timestamp, 'variantIndex': variantIndex}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Creates a branch chat from the current conversation at an optional message timestamp.
  Future<void> createBranch({required int? upToMessageTimestamp}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createBranch',
        args: _coreProxyArgs(<String, Object?>{'upToMessageTimestamp': upToMessageTimestamp}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Generates and inserts a summary message around the selected user or AI message.
  Future<bool> insertSummary({required ChatMessage message}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'insertSummary',
        args: _coreProxyArgs(<String, Object?>{'message': message.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns branch chats that were derived from the requested parent chat.
  Future<List<ChatHistory>> getBranches({required String parentChatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getBranches',
        args: _coreProxyArgs(<String, Object?>{'parentChatId': parentChatId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ChatHistory>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ChatHistory>.generate(length, (_) => ChatHistory.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates whether a chat is locked against destructive changes.
  Future<void> updateChatLocked({required String chatId, required bool locked}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateChatLocked',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'locked': locked}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Updates whether a chat is pinned in chat history ordering.
  Future<void> updateChatPinned({required String chatId, required bool pinned}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateChatPinned',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'pinned': pinned}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Applies a reordered chat list and optionally moves the active item into a group.
  Future<void> updateChatOrderAndGroup({required List<ChatHistoryListItem> reorderedHistories, required ChatHistoryListItem movedItem, required String? targetGroup}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateChatOrderAndGroup',
        args: _coreProxyArgs(<String, Object?>{'reorderedHistories': reorderedHistories.map((item) => item.toJson()).toList(growable: false), 'movedItem': movedItem.toJson(), 'targetGroup': targetGroup}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes every message from the currently selected chat.
  Future<void> clearCurrentChat() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearCurrentChat',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Serializes all chat histories into a JSON archive string.
  Future<String> exportChatHistoriesToJson() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exportChatHistoriesToJson',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports chat histories from a JSON archive string.
  Future<ChatImportResult> importChatHistoriesFromJson({required String jsonString}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importChatHistoriesFromJson',
        args: _coreProxyArgs(<String, Object?>{'jsonString': jsonString}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ChatImportResult>(responseBytes, decode: (reader) => ChatImportResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates the stored title of a chat history.
  Future<void> updateChatTitle({required String chatId, required String title}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateChatTitle',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'title': title}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Binds a chat to an existing workspace path.
  Future<void> bindChatToWorkspace({required String chatId, required String workspace}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'bindChatToWorkspace',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'workspace': workspace}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Creates the default workspace directory for a chat and returns its path.
  Future<String> createAndGetDefaultWorkspace({required String chatId, required String? projectType}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createAndGetDefaultWorkspace',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'projectType': projectType}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates the default workspace for a chat and stores the workspace binding.
  Future<String> createAndBindDefaultWorkspace({required String chatId, required String? projectType}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createAndBindDefaultWorkspace',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'projectType': projectType}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Removes the workspace binding from a chat without deleting workspace files.
  Future<void> unbindChatFromWorkspace({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'unbindChatFromWorkspace',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Renames the workspace binding and chat title together.
  Future<void> renameWorkspaceAndChat({required String chatId, required String newWorkspace, required String newTitle}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'renameWorkspaceAndChat',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'newWorkspace': newWorkspace, 'newTitle': newTitle}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Shows file changes that would be applied when rewinding before one message timestamp.
  Future<List<WorkspaceFileChange>> previewWorkspaceChangesForMessage({required String chatId, required int messageTimestamp}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'previewWorkspaceChangesForMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageTimestamp': messageTimestamp}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<WorkspaceFileChange>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<WorkspaceFileChange>.generate(length, (_) => WorkspaceFileChange.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Restores the bound workspace to the snapshot before one message timestamp.
  Future<bool> rewindWorkspaceForMessage({required String chatId, required int messageTimestamp}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'rewindWorkspaceForMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageTimestamp': messageTimestamp}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Rolls an explicit chat back to a prior message timestamp.
  Future<String?> rollbackToMessage({required String chatId, required int messageTimestamp}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'rollbackToMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageTimestamp': messageTimestamp}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Rewinds a user message and sends edited content as a new turn.
  Future<bool> rewindAndResendMessage({required String chatId, required int messageTimestamp, required String editedContent}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'rewindAndResendMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageTimestamp': messageTimestamp, 'editedContent': editedContent}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Regenerates one AI message in place while preserving the surrounding chat history.
  Future<void> regenerateSingleAiMessage({required String chatId, required int messageTimestamp}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'regenerateSingleAiMessage',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'messageTimestamp': messageTimestamp}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Clears the token counters associated with the current chat service.
  Future<void> resetTokenStatistics() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resetTokenStatistics',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Recomputes cumulative token statistics for the current chat and service.
  Future<void> updateCumulativeStatistics() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateCumulativeStatistics',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Adds a file, pasted text, package, screen capture, notification capture, or location capture as an attachment.
  Future<void> handleAttachment({required String filePath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'handleAttachment',
        args: _coreProxyArgs(<String, Object?>{'_filePath': filePath}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes one attachment by its stored file path.
  Future<void> removeAttachment({required String filePath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removeAttachment',
        args: _coreProxyArgs(<String, Object?>{'_filePath': filePath}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes every pending attachment from the chat input.
  Future<void> clearAttachments() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearAttachments',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns chat ids that currently have active streaming turns.
  Future<List<String>> activeStreamingChatIds() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'activeStreamingChatIds',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<String>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the state flow of chat ids that currently have active streaming turns.
  Stream<List<String>> activeStreamingChatIdsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'activeStreamingChatIdsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<String>>(event, decode: (valueBytes) => decodeCoreLink<List<String>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns the state flow of processing states keyed by chat id.
  Stream<Map<String, InputProcessingState>> inputProcessingStateByChatIdFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'inputProcessingStateByChatIdFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<String, InputProcessingState>>(event, decode: (valueBytes) => decodeCoreLink<Map<String, InputProcessingState>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, InputProcessingState>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = InputProcessingState.fromMessagePack(reader); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns transient toast messages emitted by chat input actions.
  Stream<String?> toastEventFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'toastEventFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<String?>(event, decode: (valueBytes) => decodeCoreLink<String?>(valueBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Clears the current transient toast event after the UI has consumed it.
  Future<void> clearToastEvent() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearToastEvent',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns the processing state for the currently selected chat.
  Future<InputProcessingState> currentChatInputProcessingState() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'currentChatInputProcessingState',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<InputProcessingState>(responseBytes, decode: (reader) => InputProcessingState.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether the currently selected chat is actively streaming.
  Future<bool> currentChatIsLoading() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'currentChatIsLoading',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether older messages exist beyond the current display window.
  Future<bool> hasOlderDisplayHistory() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'hasOlderDisplayHistory',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether newer messages exist beyond the current display window.
  Future<bool> hasNewerDisplayHistory() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'hasNewerDisplayHistory',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether the display-window loader is currently fetching messages.
  Future<bool> isLoadingDisplayWindow() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isLoadingDisplayWindow',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the state flow of tool invocation counts keyed by chat id.
  Stream<Map<String, int>> currentTurnToolInvocationCountByChatIdFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'currentTurnToolInvocationCountByChatIdFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<String, int>>(event, decode: (valueBytes) => decodeCoreLink<Map<String, int>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, int>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = reader.readInt(); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns the in-memory messages for the current chat.
  Future<List<ChatMessage>> chatHistory() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'chatHistory',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ChatMessage>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ChatMessage>.generate(length, (_) => ChatMessage.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the state flow of the currently selected chat id.
  Stream<String?> currentChatIdFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'currentChatIdFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<String?>(event, decode: (valueBytes) => decodeCoreLink<String?>(valueBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns a current snapshot of all persisted chat histories.
  Future<List<ChatHistory>> chatHistories() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'chatHistories',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ChatHistory>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ChatHistory>.generate(length, (_) => ChatHistory.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the state flow of all persisted chat histories.
  Stream<List<ChatHistory>> chatHistoriesFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'chatHistoriesFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<ChatHistory>>(event, decode: (valueBytes) => decodeCoreLink<List<ChatHistory>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ChatHistory>.generate(length, (_) => ChatHistory.fromMessagePack(reader), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns chat history list items prepared for grouped history UI.
  Stream<List<ChatHistoryListItem>> chatHistoryListItemsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'chatHistoryListItemsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<ChatHistoryListItem>>(event, decode: (valueBytes) => decodeCoreLink<List<ChatHistoryListItem>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ChatHistoryListItem>.generate(length, (_) => ChatHistoryListItem.fromMessagePack(reader), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns messages from the Core selected by Binding for one explicit chat.
  Stream<List<ChatMessage>> chatMessagesFlow({required String chatId}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'chatMessagesFlow', args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<ChatMessage>>(event, decode: (valueBytes) => decodeCoreLink<List<ChatMessage>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ChatMessage>.generate(length, (_) => ChatMessage.fromMessagePack(reader), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Builds a routed diagnostic chat message flow with one embedded response stream.
  Stream<List<ChatMessage>> routeProbeChatMessagesFlow({required String chatId, required String streamText}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'routeProbeChatMessagesFlow', args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'streamText': streamText}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<ChatMessage>>(event, decode: (valueBytes) => decodeCoreLink<List<ChatMessage>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ChatMessage>.generate(length, (_) => ChatMessage.fromMessagePack(reader), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns messages from this local Core for one explicit chat.
  Stream<List<ChatMessage>> localChatMessagesFlow({required String chatId}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'localChatMessagesFlow', args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<ChatMessage>>(event, decode: (valueBytes) => decodeCoreLink<List<ChatMessage>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ChatMessage>.generate(length, (_) => ChatMessage.fromMessagePack(reader), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns runtime state from the Core selected by Binding for one explicit chat.
  Stream<ChatState> chatStateFlow({required String chatId}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'chatStateFlow', args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<ChatState>(event, decode: (valueBytes) => decodeCoreLink<ChatState>(valueBytes, decode: (reader) => ChatState.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns runtime state from this local Core for one explicit chat.
  Stream<ChatState> localChatStateFlow({required String chatId}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'localChatStateFlow', args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<ChatState>(event, decode: (valueBytes) => decodeCoreLink<ChatState>(valueBytes, decode: (reader) => ChatState.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns whether the chat history selector should be visible.
  Future<bool> showChatHistorySelector() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'showChatHistorySelector',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns a snapshot of pending input attachments.
  Future<List<AttachmentInfo>> attachments() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'attachments',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<AttachmentInfo>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<AttachmentInfo>.generate(length, (_) => AttachmentInfo.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the current context window size state flow.
  Stream<int> currentWindowSizeFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'currentWindowSizeFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<int>(event, decode: (valueBytes) => decodeCoreLink<int>(valueBytes, decode: (reader) => reader.readInt(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns the cumulative input token count state flow.
  Stream<int> inputTokenCountFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'inputTokenCountFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<int>(event, decode: (valueBytes) => decodeCoreLink<int>(valueBytes, decode: (reader) => reader.readInt(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns the cumulative output token count state flow.
  Stream<int> outputTokenCountFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'outputTokenCountFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<int>(event, decode: (valueBytes) => decodeCoreLink<int>(valueBytes, decode: (reader) => reader.readInt(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns whether this chat core has finished delegate initialization.
  Future<bool> isInitialized() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isInitialized',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reloads chat messages using the display-window strategy for the requested chat.
  Future<void> reloadChatMessagesSmart({required String chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'reloadChatMessagesSmart',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Loads older messages into the current chat display window.
  Future<void> loadOlderMessagesForCurrentChat() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'loadOlderMessagesForCurrentChat',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Loads newer messages into the current chat display window.
  Future<void> loadNewerMessagesForCurrentChat() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'loadNewerMessagesForCurrentChat',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Moves the current chat display window to the latest messages.
  Future<void> showLatestMessagesForCurrentChat() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'showLatestMessagesForCurrentChat',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Searches a chat and returns lightweight message previews for navigation.
  Future<List<ChatMessageLocatorPreview>> loadChatMessageLocatorPreviews({required String chatId, required String query}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'loadChatMessageLocatorPreviews',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'query': query}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ChatMessageLocatorPreview>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ChatMessageLocatorPreview>.generate(length, (_) => ChatMessageLocatorPreview.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Marks or unmarks one message as a favorite by message timestamp.
  Future<void> setMessageFavorite({required int timestamp, required bool isFavorite}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setMessageFavorite',
        args: _coreProxyArgs(<String, Object?>{'timestamp': timestamp, 'isFavorite': isFavorite}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedLinkAccessLinkAccessStoreCoreProxy {
  const GeneratedLinkAccessLinkAccessStoreCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Initializes and returns the runtime's persisted Link device identity.
  Future<LinkAccessIdentity> initializeIdentity({required RemoteDeviceInfo deviceInfo}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'initializeIdentity',
        args: _coreProxyArgs(<String, Object?>{'deviceInfo': deviceInfo.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LinkAccessIdentity>(responseBytes, decode: (reader) => LinkAccessIdentity.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates and returns the runtime's Link device information for its stable identity.
  Future<LinkAccessIdentity> updateIdentityDeviceInfo({required RemoteDeviceInfo deviceInfo}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateIdentityDeviceInfo',
        args: _coreProxyArgs(<String, Object?>{'deviceInfo': deviceInfo.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LinkAccessIdentity>(responseBytes, decode: (reader) => LinkAccessIdentity.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns every accepted inbound session owned by this runtime.
  Future<Map<String, AcceptedRemoteSessionRecord>> inboundSessions() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'inboundSessions',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, AcceptedRemoteSessionRecord>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, AcceptedRemoteSessionRecord>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = AcceptedRemoteSessionRecord.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Observes every accepted inbound session owned by this runtime.
  Stream<Map<String, AcceptedRemoteSessionRecord>> inboundSessionsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'inboundSessionsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<String, AcceptedRemoteSessionRecord>>(event, decode: (valueBytes) => decodeCoreLink<Map<String, AcceptedRemoteSessionRecord>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, AcceptedRemoteSessionRecord>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = AcceptedRemoteSessionRecord.fromMessagePack(reader); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Persists one accepted inbound session owned by this runtime.
  Future<void> saveInboundSession({required String sessionId, required AcceptedRemoteSessionRecord record}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveInboundSession',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId, 'record': record.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes one accepted inbound session owned by this runtime.
  Future<void> removeInboundSession({required String sessionId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removeInboundSession',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns every named outbound session owned by this runtime.
  Future<Map<String, PairedRemoteSessionRecord>> outboundSessions() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'outboundSessions',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, PairedRemoteSessionRecord>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, PairedRemoteSessionRecord>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = PairedRemoteSessionRecord.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Observes every named outbound session owned by this runtime.
  Stream<Map<String, PairedRemoteSessionRecord>> outboundSessionsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'outboundSessionsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<String, PairedRemoteSessionRecord>>(event, decode: (valueBytes) => decodeCoreLink<Map<String, PairedRemoteSessionRecord>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, PairedRemoteSessionRecord>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = PairedRemoteSessionRecord.fromMessagePack(reader); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Persists one named outbound session owned by this runtime.
  Future<void> saveOutboundSession({required String name, required PairedRemoteSessionRecord record}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveOutboundSession',
        args: _coreProxyArgs(<String, Object?>{'name': name, 'record': record.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Writes synchronized device profiles for every stored pairing endpoint.
  Future<void> syncPairedDeviceProfiles() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'syncPairedDeviceProfiles',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes one named outbound session owned by this runtime.
  Future<void> removeOutboundSession({required String name}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removeOutboundSession',
        args: _coreProxyArgs(<String, Object?>{'name': name}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns every pending pairing owned by this runtime.
  Future<Map<String, RemotePairingCodeRecord>> pendingPairings() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'pendingPairings',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, RemotePairingCodeRecord>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, RemotePairingCodeRecord>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = RemotePairingCodeRecord.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Persists one pending pairing owned by this runtime.
  Future<void> savePendingPairing({required RemotePairingCodeRecord record}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'savePendingPairing',
        args: _coreProxyArgs(<String, Object?>{'record': record.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes one pending pairing owned by this runtime.
  Future<void> removePendingPairing({required String pairingId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removePendingPairing',
        args: _coreProxyArgs(<String, Object?>{'pairingId': pairingId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns every pending outbound pairing initiated by this runtime.
  Future<Map<String, PendingOutboundPairingRecord>> pendingOutboundPairings() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'pendingOutboundPairings',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, PendingOutboundPairingRecord>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, PendingOutboundPairingRecord>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = PendingOutboundPairingRecord.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Persists one pending outbound pairing initiated by this runtime.
  Future<void> savePendingOutboundPairing({required String pairingId, required PendingOutboundPairingRecord record}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'savePendingOutboundPairing',
        args: _coreProxyArgs(<String, Object?>{'pairingId': pairingId, 'record': record.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes one pending outbound pairing after it has completed or been cancelled.
  Future<void> removePendingOutboundPairing({required String pairingId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removePendingOutboundPairing',
        args: _coreProxyArgs(<String, Object?>{'pairingId': pairingId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Persists the active Link Access host configuration for this runtime.
  Future<void> saveHostConfig({required LinkAccessHostConfig config}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveHostConfig',
        args: _coreProxyArgs(<String, Object?>{'config': config.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Initializes and returns the active Link Access host configuration.
  Future<LinkAccessHostConfig> initializeHostConfig() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'initializeHostConfig',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LinkAccessHostConfig>(responseBytes, decode: (reader) => LinkAccessHostConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the active Link Access host configuration for this runtime.
  Future<LinkAccessHostConfig> hostConfig() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'hostConfig',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LinkAccessHostConfig>(responseBytes, decode: (reader) => LinkAccessHostConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPermissionsMcpManagerCoreProxy {
  const GeneratedPermissionsMcpManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Returns whether a server name has been registered.
  Future<bool> isServerRegistered({required String serverName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isServerRegistered',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns all registered MCP server configurations.
  Future<Map<String, McpServerConfig>> getRegisteredServers() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getRegisteredServers',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, McpServerConfig>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, McpServerConfig>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = McpServerConfig.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the most recent connection failure detail for a server.
  Future<String?> getLastConnectionFailureReason({required String serverName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getLastConnectionFailureReason',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Registers or replaces an MCP server configuration.
  Future<void> registerServer({required String serverName, required McpServerConfig serverConfig}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'registerServer',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName, 'serverConfig': serverConfig.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Registers an MCP server from an endpoint URL and description.
  Future<void> registerServerFromEndpoint({required String serverName, required String endpoint, required String description}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'registerServerFromEndpoint',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName, 'endpoint': endpoint, 'description': description}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes an MCP server and disconnects its cached client.
  Future<void> unregisterServer({required String serverName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'unregisterServer',
        args: _coreProxyArgs(<String, Object?>{'serverName': serverName}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Disconnects all cached MCP bridge clients.
  Future<void> shutdown() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'shutdown',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy {
  const GeneratedPermissionsMcpRuntimeMcpLocalServerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Reloads MCP server and plugin configuration from the runtime store.
  Future<void> reloadConfigurations() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'reloadConfigurations',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Rewrites the MCP configuration file after loading and sanitizing it.
  Future<void> saveMcpConfig() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveMCPConfig',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Rewrites the persisted MCP server status file after loading it.
  Future<void> saveServerStatus() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveServerStatus',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Adds or replaces a command-based MCP server entry in the local config.
  Future<void> addOrUpdateMcpServer({required String serverId, required String command, required List<String> args, required Map<String, String> env, required bool disabled, required List<String> autoApprove}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'addOrUpdateMCPServer',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId, 'command': command, 'args': args.map((item) => item).toList(growable: false), 'env': env.map((key, value) => MapEntry(key, value)), 'disabled': disabled, 'autoApprove': autoApprove.map((item) => item).toList(growable: false)}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Adds or replaces a complete MCP server config after validation.
  Future<void> addOrUpdateMcpServerConfig({required String serverId, required ServerConfig serverConfig}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'addOrUpdateMCPServerConfig',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId, 'serverConfig': serverConfig.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes an MCP server config, metadata, status, and local plugin directory.
  Future<void> removeMcpServer({required String serverId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removeMCPServer',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Imports MCP server entries from a JSON config payload and returns the inserted count.
  Future<int> mergeConfigFromJson({required String jsonConfig}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'mergeConfigFromJson',
        args: _coreProxyArgs(<String, Object?>{'jsonConfig': jsonConfig}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the absolute path of the MCP configuration file.
  Future<String> getConfigFilePath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getConfigFilePath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the directory used for local MCP plugin runtime files.
  Future<String> getConfigDirectory() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getConfigDirectory',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns one MCP server config by id.
  Future<ServerConfig?> getMcpServer({required String serverId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getMCPServer',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ServerConfig?>(responseBytes, decode: (reader) => reader.readNullable<ServerConfig>(() => ServerConfig.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns every configured MCP server keyed by server id.
  Future<Map<String, ServerConfig>> getAllMcpServers() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllMCPServers',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, ServerConfig>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, ServerConfig>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = ServerConfig.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Adds or replaces display metadata for an installed MCP plugin.
  Future<void> addOrUpdatePluginMetadata({required String pluginId, required PluginMetadata metadata}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'addOrUpdatePluginMetadata',
        args: _coreProxyArgs(<String, Object?>{'pluginId': pluginId, 'metadata': metadata.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes display metadata for an MCP plugin.
  Future<void> removePluginMetadata({required String pluginId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removePluginMetadata',
        args: _coreProxyArgs(<String, Object?>{'pluginId': pluginId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns display metadata for one MCP plugin.
  Future<PluginMetadata?> getPluginMetadata({required String pluginId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPluginMetadata',
        args: _coreProxyArgs(<String, Object?>{'pluginId': pluginId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<PluginMetadata?>(responseBytes, decode: (reader) => reader.readNullable<PluginMetadata>(() => PluginMetadata.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns all MCP plugin metadata keyed by plugin id.
  Future<Map<String, PluginMetadata>> getAllPluginMetadata() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllPluginMetadata',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, PluginMetadata>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, PluginMetadata>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = PluginMetadata.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates runtime status, cached tool metadata, and timestamps for one MCP server.
  Future<void> updateServerStatus({required String serverId, required String? errorMessage, required List<CachedToolInfo>? cachedTools, required int? lastStartTime, required int? lastStopTime}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateServerStatus',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId, 'errorMessage': errorMessage, 'cachedTools': cachedTools?.map((item) => item.toJson()).toList(growable: false), 'lastStartTime': lastStartTime, 'lastStopTime': lastStopTime}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Stores the latest discovered tools for an MCP server.
  Future<void> cacheServerTools({required String serverId, required List<CachedToolInfo> tools}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'cacheServerTools',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId, 'tools': tools.map((item) => item.toJson()).toList(growable: false)}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns cached tool metadata for an MCP server.
  Future<List<CachedToolInfo>?> getCachedTools({required String serverId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCachedTools',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<CachedToolInfo>?>(responseBytes, decode: (reader) => reader.readNullable<List<CachedToolInfo>>(() => (() { final length = reader.readArrayLength(); return List<CachedToolInfo>.generate(length, (_) => CachedToolInfo.fromMessagePack(reader), growable: false); })()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether an MCP server has non-empty tool cache newer than one day.
  Future<bool> hasValidToolCache({required String serverId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'hasValidToolCache',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Removes runtime status information for one MCP server.
  Future<void> removeServerStatus({required String serverId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removeServerStatus',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns runtime status information for one MCP server.
  Future<ServerStatus?> getServerStatus({required String serverId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getServerStatus',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ServerStatus?>(responseBytes, decode: (reader) => reader.readNullable<ServerStatus>(() => ServerStatus.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns runtime status information for every known MCP server.
  Future<Map<String, ServerStatus>> getAllServerStatus() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllServerStatus',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, ServerStatus>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, ServerStatus>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = ServerStatus.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether the last status timestamps indicate that the server is running.
  Future<bool> isServerLikelyRunning({required String serverId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isServerLikelyRunning',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns whether a configured MCP server is enabled.
  Future<bool> isServerEnabled({required String serverId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isServerEnabled',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Enables or disables a configured MCP server.
  Future<void> setServerEnabled({required String serverId, required bool enabled}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setServerEnabled',
        args: _coreProxyArgs(<String, Object?>{'serverId': serverId, 'enabled': enabled}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns the runtime directory used by an installed MCP plugin.
  Future<String> getPluginRuntimeDirectory({required String pluginId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPluginRuntimeDirectory',
        args: _coreProxyArgs(<String, Object?>{'pluginId': pluginId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Exports one plugin server config as a pretty JSON document.
  Future<String> getPluginConfig({required String pluginId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPluginConfig',
        args: _coreProxyArgs(<String, Object?>{'pluginId': pluginId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Saves one plugin server config from either a full MCP config JSON or a server JSON.
  Future<bool> savePluginConfig({required String pluginId, required String configJson}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'savePluginConfig',
        args: _coreProxyArgs(<String, Object?>{'pluginId': pluginId, 'configJson': configJson}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Exports MCP config and server status as one JSON document.
  Future<String> exportConfigAsJson() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exportConfigAsJson',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports MCP config and server status from an exported JSON document.
  Future<bool> importConfigFromJson({required String json}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importConfigFromJson',
        args: _coreProxyArgs(<String, Object?>{'json': json}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy {
  const GeneratedPermissionsMcpRuntimePluginsMcpBridgeCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Registers a local MCP service process with command, arguments, environment, and working directory.
  Future<Object?> registerMcpService({required String name, required String command, required List<String> args, required String? description, required Map<String, String> env, required String? cwd}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'registerMcpService',
        args: _coreProxyArgs(<String, Object?>{'name': name, 'command': command, 'args': args.map((item) => item).toList(growable: false), 'description': description, 'env': env.map((key, value) => MapEntry(key, value)), 'cwd': cwd}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Registers a remote MCP service endpoint with optional authentication headers.
  Future<Object?> registerRemoteMcpService({required String name, required String endpoint, required String? connectionType, required String? description, required String? bearerToken, required Map<String, String> headers}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'registerRemoteMcpService',
        args: _coreProxyArgs(<String, Object?>{'name': name, 'endpoint': endpoint, 'connectionType': connectionType, 'description': description, 'bearerToken': bearerToken, 'headers': headers.map((key, value) => MapEntry(key, value))}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Unregisters an MCP service and stops its active process or session.
  Future<Object?> unregisterMcpService({required String name}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'unregisterMcpService',
        args: _coreProxyArgs(<String, Object?>{'name': name}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Lists registered MCP services with active state and discovered tools.
  Future<Object?> listMcpServices({required String? serviceName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'listMcpServices',
        args: _coreProxyArgs(<String, Object?>{'serviceName': serviceName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Stops an active MCP service without removing its registration.
  Future<Object?> unspawnMcpService({required String name}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'unspawnMcpService',
        args: _coreProxyArgs(<String, Object?>{'name': name}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Stores a tool list for a service when it is not currently active.
  Future<Object?> cacheTools({required String serviceName, required List<Object?> tools}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'cacheTools',
        args: _coreProxyArgs(<String, Object?>{'serviceName': serviceName, 'tools': tools.map((item) => item).toList(growable: false)}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Lists tools from an active service or its cached tool metadata.
  Future<Object?> listTools({required String serviceName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'listTools',
        args: _coreProxyArgs(<String, Object?>{'serviceName': serviceName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Calls one tool on an active local or remote MCP service.
  Future<Object?> callTool({required String serviceName, required String method, required Object? params}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'callTool',
        args: _coreProxyArgs(<String, Object?>{'serviceName': serviceName, 'method': method, 'params': params}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns logs or the latest startup error for an MCP service.
  Future<Object?> getServiceLogs({required String serviceName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getServiceLogs',
        args: _coreProxyArgs(<String, Object?>{'serviceName': serviceName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Stops all active MCP services and clears bridge registrations and cached tools.
  Future<Object?> resetBridge() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resetBridge',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Object?>(responseBytes, decode: (reader) => reader.readValue(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPermissionsToolPermissionSystemCoreProxy {
  const GeneratedPermissionsToolPermissionSystemCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Removes the active interactive permission requester.
  Future<void> clearAsyncPermissionRequester() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearAsyncPermissionRequester',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Clears approvals that were granted for the current runtime session.
  Future<void> clearSessionApprovals() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearSessionApprovals',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves the global AI permission mode used to allow or block tool effects.
  Future<void> saveAiPermissionMode({required AiPermissionMode mode}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveAiPermissionMode',
        args: _coreProxyArgs(<String, Object?>{'mode': mode.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Reads the global AI permission mode used to allow or block tool effects.
  Future<AiPermissionMode> getAiPermissionMode() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAiPermissionMode',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<AiPermissionMode>(responseBytes, decode: (reader) => AiPermissionMode.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Builds the description shown to the requester for a tool invocation.
  Future<String> getOperationDescription({required Object? tool}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getOperationDescription',
        args: _coreProxyArgs(<String, Object?>{'tool': tool}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Asynchronously requests approval for a package tool invocation.
  Future<bool> checkPackageToolApprovalAsync({required Object? tool}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'checkPackageToolApprovalAsync',
        args: _coreProxyArgs(<String, Object?>{'tool': tool}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Asynchronously requests approval for a tool invocation that can escape the sandbox boundary.
  Future<bool> checkSandboxEscapeApprovalAsync({required Object? tool}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'checkSandboxEscapeApprovalAsync',
        args: _coreProxyArgs(<String, Object?>{'tool': tool}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Refreshes permission request state exposed to front-end observers.
  Future<bool> refreshPermissionRequestState() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'refreshPermissionRequestState',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesActivePromptManagerCoreProxy {
  const GeneratedPreferencesActivePromptManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Returns the active prompt state derived from selected group and character card.
  Stream<ActivePrompt> activePromptFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'activePromptFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<ActivePrompt>(event, decode: (valueBytes) => decodeCoreLink<ActivePrompt>(valueBytes, decode: (reader) => ActivePrompt.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Reads the current active prompt snapshot.
  Future<ActivePrompt> getActivePrompt() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getActivePrompt',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ActivePrompt>(responseBytes, decode: (reader) => ActivePrompt.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Stores the active prompt and clears the opposite prompt target.
  Future<void> setActivePrompt({required ActivePrompt prompt}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setActivePrompt',
        args: _coreProxyArgs(<String, Object?>{'prompt': prompt.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Activates a character group or card based on chat binding metadata.
  Future<void> activateForChatBinding({required String? characterCardName, required String? characterGroupId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'activateForChatBinding',
        args: _coreProxyArgs(<String, Object?>{'characterCardName': characterCardName, 'characterGroupId': characterGroupId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns the character card id that should be used for the next send operation.
  Future<String> resolveActiveCardIdForSend() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resolveActiveCardIdForSend',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesApiPreferencesCoreProxy {
  const GeneratedPreferencesApiPreferencesCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Adds token counts to the cumulative total for one provider/model.
  Future<void> updateTokensForProviderModel({required String providerModel, required int inputTokens, required int outputTokens, required int cachedInputTokens}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateTokensForProviderModel',
        args: _coreProxyArgs(<String, Object?>{'providerModel': providerModel, 'inputTokens': inputTokens, 'outputTokens': outputTokens, 'cachedInputTokens': cachedInputTokens}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Reads uncached input token count for one provider/model.
  Future<int> getInputTokensForProviderModel({required String providerModel}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getInputTokensForProviderModel',
        args: _coreProxyArgs(<String, Object?>{'providerModel': providerModel}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads cached input token count for one provider/model.
  Future<int> getCachedInputTokensForProviderModel({required String providerModel}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCachedInputTokensForProviderModel',
        args: _coreProxyArgs(<String, Object?>{'providerModel': providerModel}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads output token count for one provider/model.
  Future<int> getOutputTokensForProviderModel({required String providerModel}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getOutputTokensForProviderModel',
        args: _coreProxyArgs(<String, Object?>{'providerModel': providerModel}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads all provider/model token counters.
  Future<Map<String, List<int>>> getAllProviderModelTokens() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllProviderModelTokens',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, List<int>>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, List<int>>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = (() { final length = reader.readArrayLength(); return List<int>.generate(length, (_) => reader.readInt(), growable: false); })(); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Observes all provider/model token counters.
  Stream<Map<String, List<int>>> allProviderModelTokensFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'allProviderModelTokensFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<String, List<int>>>(event, decode: (valueBytes) => decodeCoreLink<Map<String, List<int>>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, List<int>>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = (() { final length = reader.readArrayLength(); return List<int>.generate(length, (_) => reader.readInt(), growable: false); })(); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Clears every provider/model token counter.
  Future<void> resetAllProviderModelTokenCounts() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resetAllProviderModelTokenCounts',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Clears token counters for one provider/model.
  Future<void> resetProviderModelTokenCounts({required String providerModel}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resetProviderModelTokenCounts',
        args: _coreProxyArgs(<String, Object?>{'providerModel': providerModel}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Observes whether thinking mode is enabled.
  Stream<bool> enableThinkingModeFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'enableThinkingModeFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<bool>(event, decode: (valueBytes) => decodeCoreLink<bool>(valueBytes, decode: (reader) => reader.readBool(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes the persisted feature toggle map.
  Stream<Map<String, bool>> featureTogglesFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'featureTogglesFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<String, bool>>(event, decode: (valueBytes) => decodeCoreLink<Map<String, bool>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, bool>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = reader.readBool(); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes one feature toggle with an explicit default.
  Stream<bool> featureToggleFlow({required String featureKey, required bool defaultValue}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'featureToggleFlow', args: _coreProxyArgs(<String, Object?>{'featureKey': featureKey, 'defaultValue': defaultValue}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<bool>(event, decode: (valueBytes) => decodeCoreLink<bool>(valueBytes, decode: (reader) => reader.readBool(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes the current thinking quality level.
  Stream<int> thinkingQualityLevelFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'thinkingQualityLevelFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<int>(event, decode: (valueBytes) => decodeCoreLink<int>(valueBytes, decode: (reader) => reader.readInt(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes whether memory auto-update is enabled.
  Stream<bool> enableMemoryAutoUpdateFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'enableMemoryAutoUpdateFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<bool>(event, decode: (valueBytes) => decodeCoreLink<bool>(valueBytes, decode: (reader) => reader.readBool(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes whether AI tools are enabled.
  Stream<bool> enableToolsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'enableToolsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<bool>(event, decode: (valueBytes) => decodeCoreLink<bool>(valueBytes, decode: (reader) => reader.readBool(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes per-tool prompt visibility settings.
  Stream<Map<String, bool>> toolPromptVisibilityFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'toolPromptVisibilityFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<String, bool>>(event, decode: (valueBytes) => decodeCoreLink<Map<String, bool>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, bool>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = reader.readBool(); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes whether streaming output is disabled.
  Stream<bool> disableStreamOutputFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'disableStreamOutputFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<bool>(event, decode: (valueBytes) => decodeCoreLink<bool>(valueBytes, decode: (reader) => reader.readBool(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes whether user preference descriptions are hidden from prompts.
  Stream<bool> disableUserPreferenceDescriptionFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'disableUserPreferenceDescriptionFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<bool>(event, decode: (valueBytes) => decodeCoreLink<bool>(valueBytes, decode: (reader) => reader.readBool(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes the image-history turn limit.
  Stream<int> maxImageHistoryUserTurnsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'maxImageHistoryUserTurnsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<int>(event, decode: (valueBytes) => decodeCoreLink<int>(valueBytes, decode: (reader) => reader.readInt(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes the media-history turn limit.
  Stream<int> maxMediaHistoryUserTurnsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'maxMediaHistoryUserTurnsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<int>(event, decode: (valueBytes) => decodeCoreLink<int>(valueBytes, decode: (reader) => reader.readInt(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes the MCP startup timeout in seconds.
  Stream<int> mcpStartupTimeoutSecondsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'mcpStartupTimeoutSecondsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<int>(event, decode: (valueBytes) => decodeCoreLink<int>(valueBytes, decode: (reader) => reader.readInt(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
  Stream<int> toolPkgPreHookTimeoutSecondsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'toolPkgPreHookTimeoutSecondsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<int>(event, decode: (valueBytes) => decodeCoreLink<int>(valueBytes, decode: (reader) => reader.readInt(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Saves the thinking-mode toggle.
  Future<void> saveEnableThinkingMode({required bool isEnabled}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveEnableThinkingMode',
        args: _coreProxyArgs(<String, Object?>{'isEnabled': isEnabled}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves one feature toggle value.
  Future<void> saveFeatureToggle({required String featureKey, required bool isEnabled}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveFeatureToggle',
        args: _coreProxyArgs(<String, Object?>{'featureKey': featureKey, 'isEnabled': isEnabled}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves the thinking quality level.
  Future<void> saveThinkingQualityLevel({required int level}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveThinkingQualityLevel',
        args: _coreProxyArgs(<String, Object?>{'level': level}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves the memory auto-update toggle.
  Future<void> saveEnableMemoryAutoUpdate({required bool isEnabled}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveEnableMemoryAutoUpdate',
        args: _coreProxyArgs(<String, Object?>{'isEnabled': isEnabled}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves the global tools toggle.
  Future<void> saveEnableTools({required bool isEnabled}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveEnableTools',
        args: _coreProxyArgs(<String, Object?>{'isEnabled': isEnabled}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves prompt visibility for one tool.
  Future<void> saveToolPromptVisibility({required String toolName, required bool isVisible}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveToolPromptVisibility',
        args: _coreProxyArgs(<String, Object?>{'toolName': toolName, 'isVisible': isVisible}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Replaces the full tool prompt visibility map.
  Future<void> saveToolPromptVisibilityMap({required Map<String, bool> visibilityMap}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveToolPromptVisibilityMap',
        args: _coreProxyArgs(<String, Object?>{'visibilityMap': visibilityMap.map((key, value) => MapEntry(key, value))}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Reads the full tool prompt visibility map.
  Future<Map<String, bool>> getToolPromptVisibilityMap() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPromptVisibilityMap',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, bool>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, bool>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = reader.readBool(); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Saves whether streaming output should be disabled.
  Future<void> saveDisableStreamOutput({required bool isDisabled}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveDisableStreamOutput',
        args: _coreProxyArgs(<String, Object?>{'isDisabled': isDisabled}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves whether user preference descriptions should be hidden from prompts.
  Future<void> saveDisableUserPreferenceDescription({required bool isDisabled}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveDisableUserPreferenceDescription',
        args: _coreProxyArgs(<String, Object?>{'isDisabled': isDisabled}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves image and media history turn limits.
  Future<void> updateMediaHistorySettings({required int maxImageHistoryUserTurns, required int maxMediaHistoryUserTurns}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateMediaHistorySettings',
        args: _coreProxyArgs(<String, Object?>{'maxImageHistoryUserTurns': maxImageHistoryUserTurns, 'maxMediaHistoryUserTurns': maxMediaHistoryUserTurns}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves the MCP startup timeout in seconds.
  Future<void> saveMcpStartupTimeoutSeconds({required int seconds}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveMcpStartupTimeoutSeconds',
        args: _coreProxyArgs(<String, Object?>{'seconds': seconds}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Saves the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
  Future<void> saveToolPkgPreHookTimeoutSeconds({required int seconds}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveToolPkgPreHookTimeoutSeconds',
        args: _coreProxyArgs(<String, Object?>{'seconds': seconds}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Reads the MCP startup timeout in seconds.
  Future<int> getMcpStartupTimeoutSeconds() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getMcpStartupTimeoutSeconds',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the total timeout for one ToolPkg pre-hook dispatch chain in seconds.
  Future<int> getToolPkgPreHookTimeoutSeconds() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getToolPkgPreHookTimeoutSeconds',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates thinking settings in one edit transaction.
  Future<void> updateThinkingSettings({required bool? enableThinkingMode, required int? thinkingQualityLevel}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateThinkingSettings',
        args: _coreProxyArgs(<String, Object?>{'enableThinkingMode': enableThinkingMode, 'thinkingQualityLevel': thinkingQualityLevel}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedPreferencesCharacterCardManagerCoreProxy {
  const GeneratedPreferencesCharacterCardManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Returns the ordered list of character card ids as a preference flow.
  Stream<List<String>> characterCardListFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'characterCardListFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<String>>(event, decode: (valueBytes) => decodeCoreLink<List<String>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns the active character card id as a preference flow.
  Stream<String?> observeActiveCharacterCardId() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'observeActiveCharacterCardId', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<String?>(event, decode: (valueBytes) => decodeCoreLink<String?>(valueBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns a character card preference flow for one card id.
  Stream<CharacterCard> getCharacterCardFlow({required String id}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'getCharacterCardFlow', args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<CharacterCard>(event, decode: (valueBytes) => decodeCoreLink<CharacterCard>(valueBytes, decode: (reader) => CharacterCard.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Reads one character card snapshot by id.
  Future<CharacterCard> getCharacterCard({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCharacterCard',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CharacterCard>(responseBytes, decode: (reader) => CharacterCard.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a character card and returns the stored id.
  Future<String> createCharacterCard({required CharacterCard card}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createCharacterCard',
        args: _coreProxyArgs(<String, Object?>{'card': card.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates all stored fields for an existing character card.
  Future<void> updateCharacterCard({required CharacterCard card}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateCharacterCard',
        args: _coreProxyArgs(<String, Object?>{'card': card.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Deletes a non-default character card and clears it from the active selection.
  Future<void> deleteCharacterCard({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteCharacterCard',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Sets the active character card id used by active prompt resolution.
  Future<void> setActiveCharacterCard({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setActiveCharacterCard',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Clears the active character card selection.
  Future<void> clearActiveCharacterCard() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearActiveCharacterCard',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns every character card sorted with the default card first.
  Future<List<CharacterCard>> getAllCharacterCards() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllCharacterCards',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<CharacterCard>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<CharacterCard>.generate(length, (_) => CharacterCard.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Finds a character card by its display name.
  Future<CharacterCard?> findCharacterCardByName({required String name}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'findCharacterCardByName',
        args: _coreProxyArgs(<String, Object?>{'name': name}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CharacterCard?>(responseBytes, decode: (reader) => reader.readNullable<CharacterCard>(() => CharacterCard.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Recreates the default character card with the built-in default content.
  Future<void> resetDefaultCharacterCard() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resetDefaultCharacterCard',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Combines character card content and attached tags into a prompt string.
  Future<String> combinePrompts({required String characterCardId, required List<String> additionalTagIds, required PromptFunctionType promptFunctionType}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'combinePrompts',
        args: _coreProxyArgs(<String, Object?>{'characterCardId': characterCardId, 'additionalTagIds': additionalTagIds.map((item) => item).toList(growable: false), 'promptFunctionType': promptFunctionType.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Exports all character cards and prompt tags as backup JSON.
  Future<String> exportAllCharacterCardsToBackupContent() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exportAllCharacterCardsToBackupContent',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports character cards and prompt tags from backup JSON.
  Future<CharacterCardImportResult> importAllCharacterCardsFromBackupContent({required String jsonContent}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importAllCharacterCardsFromBackupContent',
        args: _coreProxyArgs(<String, Object?>{'jsonContent': jsonContent}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CharacterCardImportResult>(responseBytes, decode: (reader) => CharacterCardImportResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a character card from Tavern card JSON.
  Future<String> createCharacterCardFromTavernJson({required String jsonString}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createCharacterCardFromTavernJson',
        args: _coreProxyArgs(<String, Object?>{'jsonString': jsonString}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Exports one character card as Tavern-compatible JSON.
  Future<String> exportCharacterCardToTavernJson({required String characterCardId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exportCharacterCardToTavernJson',
        args: _coreProxyArgs(<String, Object?>{'characterCardId': characterCardId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy {
  const GeneratedPreferencesCharacterCardToolAccessResolverCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Returns the canonical built-in tool options used by character-card access control.
  Future<List<ManageableToolPrompt>> getManageableBuiltinToolOptions({required bool useEnglish}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getManageableBuiltinToolOptions',
        args: _coreProxyArgs(<String, Object?>{'useEnglish': useEnglish}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ManageableToolPrompt>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ManageableToolPrompt>.generate(length, (_) => ManageableToolPrompt.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesCharacterGroupCardManagerCoreProxy {
  const GeneratedPreferencesCharacterGroupCardManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Observes the ordered list of stored character group identifiers.
  Stream<List<String>> characterGroupCardListFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'characterGroupCardListFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<String>>(event, decode: (valueBytes) => decodeCoreLink<List<String>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes the currently selected character group identifier.
  Stream<String?> observeActiveCharacterGroupId() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'observeActiveCharacterGroupId', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<String?>(event, decode: (valueBytes) => decodeCoreLink<String?>(valueBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes all stored character groups sorted by most recent update time.
  Stream<List<CharacterGroupCard>> allCharacterGroupCardsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'allCharacterGroupCardsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<CharacterGroupCard>>(event, decode: (valueBytes) => decodeCoreLink<List<CharacterGroupCard>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<CharacterGroupCard>.generate(length, (_) => CharacterGroupCard.fromMessagePack(reader), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes one character group by identifier.
  Stream<CharacterGroupCard?> getCharacterGroupCardFlow({required String id}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'getCharacterGroupCardFlow', args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<CharacterGroupCard?>(event, decode: (valueBytes) => decodeCoreLink<CharacterGroupCard?>(valueBytes, decode: (reader) => reader.readNullable<CharacterGroupCard>(() => CharacterGroupCard.fromMessagePack(reader)), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes the full character group selected as active.
  Stream<CharacterGroupCard?> activeCharacterGroupCardFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'activeCharacterGroupCardFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<CharacterGroupCard?>(event, decode: (valueBytes) => decodeCoreLink<CharacterGroupCard?>(valueBytes, decode: (reader) => reader.readNullable<CharacterGroupCard>(() => CharacterGroupCard.fromMessagePack(reader)), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Creates a character group and assigns timestamps plus an id when required.
  Future<String> createCharacterGroupCard({required CharacterGroupCard group}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createCharacterGroupCard',
        args: _coreProxyArgs(<String, Object?>{'group': group.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates an existing character group and refreshes its update timestamp.
  Future<void> updateCharacterGroupCard({required CharacterGroupCard group}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateCharacterGroupCard',
        args: _coreProxyArgs(<String, Object?>{'group': group.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Deletes a character group and clears it from active selection when selected.
  Future<void> deleteCharacterGroupCard({required String groupId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteCharacterGroupCard',
        args: _coreProxyArgs(<String, Object?>{'groupId': groupId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Selects a character group as active or clears the active selection.
  Future<void> setActiveCharacterGroupCard({required String? groupId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setActiveCharacterGroupCard',
        args: _coreProxyArgs(<String, Object?>{'groupId': groupId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Reads one character group by identifier.
  Future<CharacterGroupCard?> getCharacterGroupCard({required String groupId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCharacterGroupCard',
        args: _coreProxyArgs(<String, Object?>{'groupId': groupId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CharacterGroupCard?>(responseBytes, decode: (reader) => reader.readNullable<CharacterGroupCard>(() => CharacterGroupCard.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads every stored character group sorted by most recent update time.
  Future<List<CharacterGroupCard>> getAllCharacterGroupCards() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllCharacterGroupCards',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<CharacterGroupCard>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<CharacterGroupCard>.generate(length, (_) => CharacterGroupCard.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Duplicates a character group and returns the newly created group id.
  Future<String?> duplicateCharacterGroupCard({required String sourceGroupId, required String? newName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'duplicateCharacterGroupCard',
        args: _coreProxyArgs(<String, Object?>{'sourceGroupId': sourceGroupId, 'newName': newName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Copies external bindings owned by one character group to another group.
  Future<void> cloneBindingsFromCharacterGroup({required String sourceGroupId, required String targetGroupId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'cloneBindingsFromCharacterGroup',
        args: _coreProxyArgs(<String, Object?>{'_sourceGroupId': sourceGroupId, '_targetGroupId': targetGroupId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Exports all character groups as pretty-printed backup JSON.
  Future<String> exportAllCharacterGroupsToBackupContent() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exportAllCharacterGroupsToBackupContent',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports character groups from backup JSON and reports create, update, and skip counts.
  Future<CharacterGroupImportResult> importAllCharacterGroupsFromBackupContent({required String jsonContent}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importAllCharacterGroupsFromBackupContent',
        args: _coreProxyArgs(<String, Object?>{'jsonContent': jsonContent}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CharacterGroupImportResult>(responseBytes, decode: (reader) => CharacterGroupImportResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesEnvPreferencesCoreProxy {
  const GeneratedPreferencesEnvPreferencesCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Reads an environment value from persistent preferences or the process environment.
  Future<String?> getEnv({required String key}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getEnv',
        args: _coreProxyArgs(<String, Object?>{'key': key}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Stores an environment value in persistent preferences.
  Future<void> setEnv({required String key, required String value}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setEnv',
        args: _coreProxyArgs(<String, Object?>{'key': key, 'value': value}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes a persisted environment value by key.
  Future<void> removeEnv({required String key}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removeEnv',
        args: _coreProxyArgs(<String, Object?>{'key': key}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns all persisted environment values keyed by variable name.
  Future<Map<String, String>> getAllEnv() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllEnv',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, String>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, String>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = reader.readString(); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Replaces the persisted environment values with the provided map.
  Future<void> setAllEnv({required Map<String, String> variables}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setAllEnv',
        args: _coreProxyArgs(<String, Object?>{'variables': variables.map((key, value) => MapEntry(key, value))}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedPreferencesFunctionalConfigManagerCoreProxy {
  const GeneratedPreferencesFunctionalConfigManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Observes the full mapping from runtime functions to provider model bindings.
  Stream<Map<FunctionType, FunctionModelBinding>> functionModelBindingFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'functionModelBindingFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<FunctionType, FunctionModelBinding>>(event, decode: (valueBytes) => decodeCoreLink<Map<FunctionType, FunctionModelBinding>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <FunctionType, FunctionModelBinding>{}; for (var index = 0; index < length; index += 1) { result[FunctionType.fromMessagePack(reader)] = FunctionModelBinding.fromMessagePack(reader); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Saves the complete function-to-model binding map.
  Future<void> saveFunctionModelBinding({required Map<FunctionType, FunctionModelBinding> binding}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveFunctionModelBinding',
        args: _coreProxyArgs(<String, Object?>{'binding': binding.map((key, value) => MapEntry(key.toJson(), value.toJson()))}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Reads the model binding currently assigned to one runtime function.
  Future<FunctionModelBinding> getModelBindingForFunction({required FunctionType functionType}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getModelBindingForFunction',
        args: _coreProxyArgs(<String, Object?>{'functionType': functionType.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<FunctionModelBinding>(responseBytes, decode: (reader) => FunctionModelBinding.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Assigns one runtime function to the specified provider and model.
  Future<void> setModelForFunction({required FunctionType functionType, required String providerId, required String modelId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setModelForFunction',
        args: _coreProxyArgs(<String, Object?>{'functionType': functionType.toJson(), 'providerId': providerId, 'modelId': modelId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Restores one runtime function to the default provider and model.
  Future<void> resetFunctionConfig({required FunctionType functionType}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resetFunctionConfig',
        args: _coreProxyArgs(<String, Object?>{'functionType': functionType.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Restores every runtime function to the default provider and model map.
  Future<void> resetAllFunctionConfigs() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resetAllFunctionConfigs',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedPreferencesGitHubAuthPreferencesCoreProxy {
  const GeneratedPreferencesGitHubAuthPreferencesCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Saves a GitHub login session including token metadata and user profile data.
  Future<void> saveAuthInfo({required String accessToken, required String tokenType, required Object? userInfo, required String? grantedScope}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveAuthInfo',
        args: _coreProxyArgs(<String, Object?>{'accessToken': accessToken, 'tokenType': tokenType, 'userInfo': userInfo, 'grantedScope': grantedScope}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Updates the saved GitHub access token without replacing the saved user profile.
  Future<void> updateAccessToken({required String accessToken, required String tokenType, required String? grantedScope}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateAccessToken',
        args: _coreProxyArgs(<String, Object?>{'accessToken': accessToken, 'tokenType': tokenType, 'grantedScope': grantedScope}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns the saved GitHub access token when the stored session is current.
  Future<String?> getCurrentAccessToken() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCurrentAccessToken',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the saved GitHub user profile when the stored session is current.
  Future<CoreDataPreferencesGitHubAuthPreferencesGitHubUser?> getCurrentUserInfo() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCurrentUserInfo',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreDataPreferencesGitHubAuthPreferencesGitHubUser?>(responseBytes, decode: (reader) => reader.readNullable<CoreDataPreferencesGitHubAuthPreferencesGitHubUser>(() => CoreDataPreferencesGitHubAuthPreferencesGitHubUser.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reports whether the saved GitHub authentication session is usable.
  Future<bool> isLoggedIn() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isLoggedIn',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Clears the saved GitHub authentication session.
  Future<void> logout() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'logout',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Builds the HTTP Authorization header for the current GitHub token.
  Future<String?> getAuthorizationHeader() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAuthorizationHeader',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesModelConfigManagerCoreProxy {
  const GeneratedPreferencesModelConfigManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Observes the ordered provider id list.
  Stream<List<String>> providerListFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'providerListFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<String>>(event, decode: (valueBytes) => decodeCoreLink<List<String>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Reads the ordered provider id list.
  Future<List<String>> getProviderIds() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getProviderIds',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<String>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Observes all provider profiles in persisted order.
  Stream<List<ProviderProfile>> getProviderProfilesFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'getProviderProfilesFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<ProviderProfile>>(event, decode: (valueBytes) => decodeCoreLink<List<ProviderProfile>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ProviderProfile>.generate(length, (_) => ProviderProfile.fromMessagePack(reader), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Reads all provider profiles in persisted order.
  Future<List<ProviderProfile>> getProviderProfiles() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getProviderProfiles',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ProviderProfile>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ProviderProfile>.generate(length, (_) => ProviderProfile.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads one provider profile by id.
  Future<ProviderProfile> getProviderProfile({required String providerId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getProviderProfile',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ProviderProfile>(responseBytes, decode: (reader) => ProviderProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Builds summary rows for every configured provider model.
  Future<List<ProviderModelSummary>> getAllModelSummaries() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllModelSummaries',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ProviderModelSummary>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ProviderModelSummary>.generate(length, (_) => ProviderModelSummary.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads provider catalog entries from the built-in catalog.
  Future<List<ProviderCatalogEntry>> getProviderCatalogEntries() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getProviderCatalogEntries',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<ProviderCatalogEntry>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<ProviderCatalogEntry>.generate(length, (_) => ProviderCatalogEntry.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a new provider profile and stores it in provider order.
  Future<String> createProvider({required String name, required String providerTypeId, required String endpoint}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createProvider',
        args: _coreProxyArgs(<String, Object?>{'name': name, 'providerTypeId': providerTypeId, 'endpoint': endpoint}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Replaces an existing provider profile after validation.
  Future<ProviderProfile> updateProviderProfile({required ProviderProfile provider}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateProviderProfile',
        args: _coreProxyArgs(<String, Object?>{'provider': provider.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ProviderProfile>(responseBytes, decode: (reader) => ProviderProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Replaces the default provider profile and preserves its id.
  Future<ProviderProfile> replaceDefaultProviderProfile({required ProviderProfile provider}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'replaceDefaultProviderProfile',
        args: _coreProxyArgs(<String, Object?>{'provider': provider.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ProviderProfile>(responseBytes, decode: (reader) => ProviderProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes one provider profile and removes it from provider order.
  Future<void> deleteProvider({required String providerId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteProvider',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Creates a model profile under an existing provider.
  Future<String> createProviderModel({required String providerId, required String modelId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createProviderModel',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Fetches provider models available for import into a provider profile.
  Future<List<AvailableProviderModel>> getAvailableProviderModels({required String providerId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAvailableProviderModels',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<AvailableProviderModel>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<AvailableProviderModel>.generate(length, (_) => AvailableProviderModel.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Adds a provider model using catalog or remote availability metadata.
  Future<String> addProviderModelFromAvailable({required String providerId, required String modelId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'addProviderModelFromAvailable',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Replaces a model profile under an existing provider.
  Future<ModelProfile> updateModelProfile({required String providerId, required ModelProfile model}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateModelProfile',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'model': model.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ModelProfile>(responseBytes, decode: (reader) => ModelProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes one model profile from a provider.
  Future<void> deleteModel({required String providerId, required String modelId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteModel',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Reads one model profile from a provider.
  Future<ModelProfile> getModelProfile({required String providerId, required String modelId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getModelProfile',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ModelProfile>(responseBytes, decode: (reader) => ModelProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Resolves provider and model profile data into a runtime model config.
  Future<ResolvedModelConfig> getResolvedModelConfig({required String providerId, required String modelId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getResolvedModelConfig',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ResolvedModelConfig>(responseBytes, decode: (reader) => ResolvedModelConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads model parameters for one provider/model pair.
  Future<List<Object?>> getModelParametersForModel({required String providerId, required String modelId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getModelParametersForModel',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<Object?>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<Object?>.generate(length, (_) => reader.readValue(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates model parameters for one provider/model pair.
  Future<void> updateParametersForModel({required String providerId, required String modelId, required List<Object?> parameters}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateParametersForModel',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId, 'parameters': parameters.map((item) => item).toList(growable: false)}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Updates model capabilities for one provider/model pair.
  Future<ModelProfile> updateCapabilitiesForModel({required String providerId, required String modelId, required ModelCapabilities capabilities}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateCapabilitiesForModel',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId, 'capabilities': capabilities.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ModelProfile>(responseBytes, decode: (reader) => ModelProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates model context settings for one provider/model pair.
  Future<ModelProfile> updateContextForModel({required String providerId, required String modelId, required ModelContextSpec context}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateContextForModel',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId, 'context': context.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ModelProfile>(responseBytes, decode: (reader) => ModelProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates built-in tool settings for one provider/model pair.
  Future<ModelProfile> updateBuiltinToolsForModel({required String providerId, required String modelId, required List<ModelBuiltinTool> builtinTools}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateBuiltinToolsForModel',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId, 'builtinTools': builtinTools.map((item) => item.toJson()).toList(growable: false)}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ModelProfile>(responseBytes, decode: (reader) => ModelProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates request settings for one provider/model pair.
  Future<ModelProfile> updateRequestForModel({required String providerId, required String modelId, required ModelRequestSpec request}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateRequestForModel',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId, 'request': request.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ModelProfile>(responseBytes, decode: (reader) => ModelProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates summary settings for one provider/model pair.
  Future<ModelProfile> updateSummaryForModel({required String providerId, required String modelId, required ModelSummarySettings summary}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateSummaryForModel',
        args: _coreProxyArgs(<String, Object?>{'providerId': providerId, 'modelId': modelId, 'summary': summary.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ModelProfile>(responseBytes, decode: (reader) => ModelProfile.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Exports all provider profiles as formatted JSON.
  Future<String> exportAllProviders() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exportAllProviders',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports provider profiles from the JSON format emitted by exportAllProviders.
  Future<ModelConfigImportResult> importAllProvidersFromBackupContent({required String jsonContent}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importAllProvidersFromBackupContent',
        args: _coreProxyArgs(<String, Object?>{'jsonContent': jsonContent}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<ModelConfigImportResult>(responseBytes, decode: (reader) => ModelConfigImportResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesPreferenceStorageManagerCoreProxy {
  const GeneratedPreferencesPreferenceStorageManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Reads one value from a named custom preference file.
  Future<String?> getPreference({required String fileName, required String key}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPreference',
        args: _coreProxyArgs(<String, Object?>{'fileName': fileName, 'key': key}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads selected values from a named custom preference file.
  Future<Map<String, String>> getPreferences({required String fileName, required List<String> keys}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPreferences',
        args: _coreProxyArgs(<String, Object?>{'fileName': fileName, 'keys': keys.map((item) => item).toList(growable: false)}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, String>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, String>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = reader.readString(); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Writes one value to a named custom preference file.
  Future<void> setPreference({required String fileName, required String key, required String value}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setPreference',
        args: _coreProxyArgs(<String, Object?>{'fileName': fileName, 'key': key, 'value': value}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Writes multiple values to a named custom preference file.
  Future<void> setPreferences({required String fileName, required Map<String, String> values}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setPreferences',
        args: _coreProxyArgs(<String, Object?>{'fileName': fileName, 'values': values.map((key, value) => MapEntry(key, value))}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes one key from a named custom preference file.
  Future<void> removePreference({required String fileName, required String key}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removePreference',
        args: _coreProxyArgs(<String, Object?>{'fileName': fileName, 'key': key}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes selected keys from a named custom preference file.
  Future<void> removePreferences({required String fileName, required List<String> keys}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removePreferences',
        args: _coreProxyArgs(<String, Object?>{'fileName': fileName, 'keys': keys.map((item) => item).toList(growable: false)}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes every key from a named custom preference file.
  Future<void> clearPreferences({required String fileName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearPreferences',
        args: _coreProxyArgs(<String, Object?>{'fileName': fileName}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedPreferencesPromptTagManagerCoreProxy {
  const GeneratedPreferencesPromptTagManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Observes the ordered list of prompt tag identifiers.
  Stream<List<String>> tagListFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'tagListFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<String>>(event, decode: (valueBytes) => decodeCoreLink<List<String>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes all prompt tags sorted by most recent update time.
  Stream<List<PromptTag>> allTagsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'allTagsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<PromptTag>>(event, decode: (valueBytes) => decodeCoreLink<List<PromptTag>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<PromptTag>.generate(length, (_) => PromptTag.fromMessagePack(reader), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes one prompt tag by identifier.
  Stream<PromptTag> getPromptTagFlow({required String id}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'getPromptTagFlow', args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<PromptTag>(event, decode: (valueBytes) => decodeCoreLink<PromptTag>(valueBytes, decode: (reader) => PromptTag.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Creates a prompt tag and returns its generated identifier.
  Future<String> createPromptTag({required String name, required String description, required String promptContent, required TagType tagType}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createPromptTag',
        args: _coreProxyArgs(<String, Object?>{'name': name, 'description': description, 'promptContent': promptContent, 'tagType': tagType.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates selected prompt tag fields and refreshes the update timestamp.
  Future<void> updatePromptTag({required String id, required String? name, required String? description, required String? promptContent, required TagType? tagType}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updatePromptTag',
        args: _coreProxyArgs(<String, Object?>{'id': id, 'name': name, 'description': description, 'promptContent': promptContent, 'tagType': tagType?.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Deletes a prompt tag and removes its stored fields.
  Future<void> deletePromptTag({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deletePromptTag',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Reads all prompt tags sorted by most recent update time.
  Future<List<PromptTag>> getAllTags() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllTags',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<PromptTag>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<PromptTag>.generate(length, (_) => PromptTag.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads prompt tags that match the requested tag type.
  Future<List<PromptTag>> getTagsByType({required TagType tagType}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getTagsByType',
        args: _coreProxyArgs(<String, Object?>{'tagType': tagType.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<PromptTag>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<PromptTag>.generate(length, (_) => PromptTag.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Finds a prompt tag whose prompt content matches the provided content.
  Future<PromptTag?> findTagWithSameContent({required String promptContent}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'findTagWithSameContent',
        args: _coreProxyArgs(<String, Object?>{'promptContent': promptContent}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<PromptTag?>(responseBytes, decode: (reader) => reader.readNullable<PromptTag>(() => PromptTag.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a prompt tag or returns an existing tag with identical prompt content.
  Future<String> createOrReusePromptTag({required String name, required String description, required String promptContent, required TagType tagType}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createOrReusePromptTag',
        args: _coreProxyArgs(<String, Object?>{'name': name, 'description': description, 'promptContent': promptContent, 'tagType': tagType.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesSharedMemoryStoreManagerCoreProxy {
  const GeneratedPreferencesSharedMemoryStoreManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Observes the ordered list of shared memory store identifiers.
  Stream<List<String>> sharedMemoryStoreListFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'sharedMemoryStoreListFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<String>>(event, decode: (valueBytes) => decodeCoreLink<List<String>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Reads every shared memory store in persisted list order.
  Future<List<SharedMemoryStore>> getAllSharedMemoryStores() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllSharedMemoryStores',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<SharedMemoryStore>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<SharedMemoryStore>.generate(length, (_) => SharedMemoryStore.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads one shared memory store by identifier.
  Future<SharedMemoryStore> getSharedMemoryStore({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getSharedMemoryStore',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SharedMemoryStore>(responseBytes, decode: (reader) => SharedMemoryStore.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Observes one shared memory store by identifier.
  Stream<SharedMemoryStore> getSharedMemoryStoreFlow({required String id}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'getSharedMemoryStoreFlow', args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<SharedMemoryStore>(event, decode: (valueBytes) => decodeCoreLink<SharedMemoryStore>(valueBytes, decode: (reader) => SharedMemoryStore.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Creates a shared memory store with a generated identifier.
  Future<SharedMemoryStore> createSharedMemoryStore({required String name}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createSharedMemoryStore',
        args: _coreProxyArgs(<String, Object?>{'name': name}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SharedMemoryStore>(responseBytes, decode: (reader) => SharedMemoryStore.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a shared memory store with an explicit identifier.
  Future<SharedMemoryStore> createSharedMemoryStoreWithId({required String id, required String name}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createSharedMemoryStoreWithId',
        args: _coreProxyArgs(<String, Object?>{'id': id, 'name': name}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SharedMemoryStore>(responseBytes, decode: (reader) => SharedMemoryStore.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Renames a shared memory store and refreshes its update timestamp.
  Future<SharedMemoryStore> renameSharedMemoryStore({required String id, required String name}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'renameSharedMemoryStore',
        args: _coreProxyArgs(<String, Object?>{'id': id, 'name': name}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SharedMemoryStore>(responseBytes, decode: (reader) => SharedMemoryStore.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes a shared memory store and returns whether an entry was removed.
  Future<bool> deleteSharedMemoryStore({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteSharedMemoryStore',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesSkillVisibilityPreferencesCoreProxy {
  const GeneratedPreferencesSkillVisibilityPreferencesCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Reads whether a skill is visible to AI tool selection.
  Future<bool> isSkillVisibleToAi({required String skillName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'isSkillVisibleToAi',
        args: _coreProxyArgs(<String, Object?>{'skillName': skillName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Persists whether a skill is visible to AI tool selection.
  Future<void> setSkillVisibleToAi({required String skillName, required bool visible}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setSkillVisibleToAi',
        args: _coreProxyArgs(<String, Object?>{'skillName': skillName, 'visible': visible}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedPreferencesSttConfigManagerCoreProxy {
  const GeneratedPreferencesSttConfigManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Observes the ordered list of STT configuration ids.
  Stream<List<String>> sttConfigListFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'sttConfigListFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<String>>(event, decode: (valueBytes) => decodeCoreLink<List<String>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Reads every configured STT provider profile.
  Future<List<SttConfig>> getAllSttConfigs() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllSttConfigs',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<SttConfig>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<SttConfig>.generate(length, (_) => SttConfig.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads one STT configuration by id.
  Future<SttConfig> getSttConfig({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getSttConfig',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SttConfig>(responseBytes, decode: (reader) => SttConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the current STT configuration id.
  Future<String> getCurrentSttConfigId() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCurrentSttConfigId',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the selected STT configuration id when a selection exists.
  Future<String?> getSelectedSttConfigId() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getSelectedSttConfigId',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the current STT configuration.
  Future<SttConfig> getCurrentSttConfig() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCurrentSttConfig',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SttConfig>(responseBytes, decode: (reader) => SttConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Selects the current STT configuration by id.
  Future<String> setCurrentSttConfigId({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setCurrentSttConfigId',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates and persists one STT provider configuration.
  Future<SttConfig> createSttConfig({required SttConfig config}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createSttConfig',
        args: _coreProxyArgs(<String, Object?>{'config': config.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SttConfig>(responseBytes, decode: (reader) => SttConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates one existing STT provider configuration.
  Future<SttConfig> updateSttConfig({required SttConfig config}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateSttConfig',
        args: _coreProxyArgs(<String, Object?>{'config': config.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SttConfig>(responseBytes, decode: (reader) => SttConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes one STT provider configuration.
  Future<bool> deleteSttConfig({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteSttConfig',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns every built-in STT provider catalog entry.
  Future<List<SttProviderCatalogEntry>> getProviderCatalogEntries() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getProviderCatalogEntries',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<SttProviderCatalogEntry>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<SttProviderCatalogEntry>.generate(length, (_) => SttProviderCatalogEntry.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns models exposed by one STT provider type.
  Future<List<AvailableSttModel>> getAvailableSttModels({required String providerTypeId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAvailableSttModels',
        args: _coreProxyArgs(<String, Object?>{'providerTypeId': providerTypeId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<AvailableSttModel>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<AvailableSttModel>.generate(length, (_) => AvailableSttModel.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesTtsConfigManagerCoreProxy {
  const GeneratedPreferencesTtsConfigManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Observes the ordered list of text-to-speech configuration identifiers.
  Stream<List<String>> ttsConfigListFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'ttsConfigListFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<String>>(event, decode: (valueBytes) => decodeCoreLink<List<String>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes the identifier of the currently selected text-to-speech configuration.
  Stream<String> currentTtsConfigIdFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'currentTtsConfigIdFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<String>(event, decode: (valueBytes) => decodeCoreLink<String>(valueBytes, decode: (reader) => reader.readString(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Reads the currently selected text-to-speech configuration identifier.
  Future<String> getCurrentTtsConfigId() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCurrentTtsConfigId',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the currently selected text-to-speech configuration.
  Future<TtsConfig> getCurrentTtsConfig() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCurrentTtsConfig',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsConfig>(responseBytes, decode: (reader) => TtsConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Selects the active text-to-speech configuration by identifier.
  Future<String> setCurrentTtsConfigId({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setCurrentTtsConfigId',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads every configured text-to-speech provider or voice profile.
  Future<List<TtsConfig>> getAllTtsConfigs() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllTtsConfigs',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<TtsConfig>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<TtsConfig>.generate(length, (_) => TtsConfig.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the built-in catalog of supported text-to-speech provider presets.
  Future<List<TtsProviderCatalogEntry>> getProviderCatalogEntries() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getProviderCatalogEntries',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<TtsProviderCatalogEntry>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<TtsProviderCatalogEntry>.generate(length, (_) => TtsProviderCatalogEntry.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads one text-to-speech configuration by identifier.
  Future<TtsConfig> getTtsConfig({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getTtsConfig',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsConfig>(responseBytes, decode: (reader) => TtsConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Observes one text-to-speech configuration by identifier.
  Stream<TtsConfig> getTtsConfigFlow({required String id}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'getTtsConfigFlow', args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<TtsConfig>(event, decode: (valueBytes) => decodeCoreLink<TtsConfig>(valueBytes, decode: (reader) => TtsConfig.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Creates a text-to-speech configuration and assigns store timestamps.
  Future<TtsConfig> createTtsConfig({required TtsConfig config}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createTtsConfig',
        args: _coreProxyArgs(<String, Object?>{'config': config.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsConfig>(responseBytes, decode: (reader) => TtsConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Lists available voices reported by a provider configuration.
  Future<List<AvailableTtsVoice>> getAvailableTtsVoices({required String providerConfigId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAvailableTtsVoices',
        args: _coreProxyArgs(<String, Object?>{'providerConfigId': providerConfigId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<AvailableTtsVoice>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<AvailableTtsVoice>.generate(length, (_) => AvailableTtsVoice.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a voice configuration from one provider-reported voice entry.
  Future<TtsConfig> addTtsVoiceFromAvailable({required String providerConfigId, required String model, required String voice}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'addTtsVoiceFromAvailable',
        args: _coreProxyArgs(<String, Object?>{'providerConfigId': providerConfigId, 'model': model, 'voice': voice}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsConfig>(responseBytes, decode: (reader) => TtsConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a voice configuration from custom model and voice values.
  Future<TtsConfig> createCustomTtsVoice({required String providerConfigId, required String model, required String voice}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createCustomTtsVoice',
        args: _coreProxyArgs(<String, Object?>{'providerConfigId': providerConfigId, 'model': model, 'voice': voice}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsConfig>(responseBytes, decode: (reader) => TtsConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates a text-to-speech configuration and preserves its creation timestamp.
  Future<TtsConfig> updateTtsConfig({required TtsConfig config}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateTtsConfig',
        args: _coreProxyArgs(<String, Object?>{'config': config.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsConfig>(responseBytes, decode: (reader) => TtsConfig.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes a text-to-speech configuration and reports whether it existed.
  Future<bool> deleteTtsConfig({required String id}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteTtsConfig',
        args: _coreProxyArgs(<String, Object?>{'id': id}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedPreferencesUserPreferencesManagerCoreProxy {
  const GeneratedPreferencesUserPreferencesManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Observes the selected application language code.
  Stream<String> appLanguage() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'appLanguage', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<String>(event, decode: (valueBytes) => decodeCoreLink<String>(valueBytes, decode: (reader) => reader.readString(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Saves the selected application language code.
  Future<void> saveAppLanguage({required String languageCode}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveAppLanguage',
        args: _coreProxyArgs(<String, Object?>{'languageCode': languageCode}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Reads the selected application language code.
  Future<String> getCurrentLanguage() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCurrentLanguage',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy {
  const GeneratedProvidersChatConversationRoundManagerMirrorCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Starts a new mirrored conversation round and clears mirrored content.
  Future<void> startNewRound() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'startNewRound',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Replaces the mirrored content for the current conversation round.
  Future<void> updateContent({required String content}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateContent',
        args: _coreProxyArgs(<String, Object?>{'content': content}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Appends text to the mirrored content for the current conversation round.
  Future<void> appendContent({required String content}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'appendContent',
        args: _coreProxyArgs(<String, Object?>{'content': content}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns the mirrored content formatted for display.
  Future<String> getDisplayContent() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getDisplayContent',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the mirrored content for the current conversation round.
  Future<String> getCurrentRoundContent() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCurrentRoundContent',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy {
  const GeneratedProvidersChatEnhanceConversationRoundManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Resets the manager to the beginning of a new conversation.
  Future<void> initializeNewConversation() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'initialize_new_conversation',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Advances to the next conversation round and returns its index.
  Future<int> startNewRound() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'start_new_round',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the current conversation round index.
  Future<int> getCurrentRound() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_current_round',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Clears the raw content for the current conversation round.
  Future<void> clearContent() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clear_content',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy {
  const GeneratedProvidersChatLlmproviderStreamingJsonXmlConverterCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Reports whether the streaming JSON-to-XML converter has an open parameter tag.
  Future<bool> hasUnfinishedParam() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'hasUnfinishedParam',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedProvidersMarketStatsApiServiceCoreProxy {
  const GeneratedProvidersMarketStatsApiServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Loads the marketplace manifest.
  Future<MarketManifest> getManifest() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_manifest',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketManifest>(responseBytes, decode: (reader) => MarketManifest.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads a paginated all-entry marketplace list.
  Future<MarketListPage> getListPage({required String sort, required int page}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_list_page',
        args: _coreProxyArgs(<String, Object?>{'sort': sort, 'page': page}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketListPage>(responseBytes, decode: (reader) => MarketListPage.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads a paginated marketplace list filtered by type.
  Future<MarketListPage> getTypePage({required String type, required String sort, required int page}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_type_page',
        args: _coreProxyArgs(<String, Object?>{'r#type': type, 'sort': sort, 'page': page}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketListPage>(responseBytes, decode: (reader) => MarketListPage.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads a paginated marketplace list filtered by category.
  Future<MarketListPage> getCategoryPage({required String categoryId, required String sort, required int page}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_category_page',
        args: _coreProxyArgs(<String, Object?>{'category_id': categoryId, 'sort': sort, 'page': page}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketListPage>(responseBytes, decode: (reader) => MarketListPage.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads a paginated marketplace list filtered by type and category.
  Future<MarketListPage> getTypeCategoryPage({required String type, required String categoryId, required String sort, required int page}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_type_category_page',
        args: _coreProxyArgs(<String, Object?>{'r#type': type, 'category_id': categoryId, 'sort': sort, 'page': page}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketListPage>(responseBytes, decode: (reader) => MarketListPage.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads one static entries shard.
  Future<MarketEntriesShard> getEntriesShard({required String shard}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_entries_shard',
        args: _coreProxyArgs(<String, Object?>{'shard': shard}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketEntriesShard>(responseBytes, decode: (reader) => MarketEntriesShard.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads one marketplace entry by id.
  Future<MarketEntrySummary> getEntryById({required String entryId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_entry_by_id',
        args: _coreProxyArgs(<String, Object?>{'entry_id': entryId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketEntrySummary>(responseBytes, decode: (reader) => MarketEntrySummary.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads a paginated comment page for an entry.
  Future<MarketCommentPage> getCommentsPage({required String entryId, required int page}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_comments_page',
        args: _coreProxyArgs(<String, Object?>{'entry_id': entryId, 'page': page}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketCommentPage>(responseBytes, decode: (reader) => MarketCommentPage.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Builds aggregate stats for marketplace entries of one type.
  Future<MarketTypeStatsResponse> getStats({required String type}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_stats',
        args: _coreProxyArgs(<String, Object?>{'r#type': type}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketTypeStatsResponse>(responseBytes, decode: (reader) => MarketTypeStatsResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the GitHub user associated with the configured token.
  Future<CoreOperitProvidersMarketMarketStatsApiServiceGitHubUser> getCurrentGithubUser() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_current_github_user',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreOperitProvidersMarketMarketStatsApiServiceGitHubUser>(responseBytes, decode: (reader) => CoreOperitProvidersMarketMarketStatsApiServiceGitHubUser.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Exchanges the configured GitHub token for a marketplace session.
  Future<MarketAuthInfo> exchangeGithubTokenForMarketSession() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exchange_github_token_for_market_session',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketAuthInfo>(responseBytes, decode: (reader) => MarketAuthInfo.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a comment on a marketplace entry.
  Future<String> createEntryComment({required String entryId, required String body}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'create_entry_comment',
        args: _coreProxyArgs(<String, Object?>{'entry_id': entryId, 'body': body}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Edits an existing marketplace comment.
  Future<void> editEntryComment({required String commentId, required String body}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'edit_entry_comment',
        args: _coreProxyArgs(<String, Object?>{'comment_id': commentId, 'body': body}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Deletes an existing marketplace comment.
  Future<void> deleteEntryComment({required String commentId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'delete_entry_comment',
        args: _coreProxyArgs(<String, Object?>{'comment_id': commentId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Adds a positive reaction to a marketplace entry.
  Future<void> createEntryReaction({required String entryId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'create_entry_reaction',
        args: _coreProxyArgs(<String, Object?>{'entry_id': entryId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Loads authenticated marketplace notifications.
  Future<MarketNotificationsResponse> getNotifications({required int limit, required int offset, required String? since}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_notifications',
        args: _coreProxyArgs(<String, Object?>{'limit': limit, 'offset': offset, 'since': since}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketNotificationsResponse>(responseBytes, decode: (reader) => MarketNotificationsResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads entries owned by the authenticated user.
  Future<MarketMyEntriesResponse> getMyEntries() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_my_entries',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketMyEntriesResponse>(responseBytes, decode: (reader) => MarketMyEntriesResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads authenticated user entries filtered by type.
  Future<MarketMyEntriesResponse> getMyEntriesByType({required String type}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_my_entries_by_type',
        args: _coreProxyArgs(<String, Object?>{'r#type': type}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketMyEntriesResponse>(responseBytes, decode: (reader) => MarketMyEntriesResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Loads full data for one entry submitted by the authenticated publisher.
  Future<MarketEntrySummary> getMyEntryDetail({required String entryId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'get_my_entry_detail',
        args: _coreProxyArgs(<String, Object?>{'entry_id': entryId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketEntrySummary>(responseBytes, decode: (reader) => MarketEntrySummary.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Publishes a marketplace artifact entry.
  Future<MarketPublishResponse> publishArtifact({required String type, required String title, required String description, required String detail, required String categoryId, required bool allowPublicUpdates, required String version, required String formatVer, required String minAppVer, required String? maxAppVer, required String? changelog, required String projectId, required String runtimePackageId, required String assetKind, required String assetUrl, required String ghOwner, required String ghRepo, required String ghReleaseTag, required String assetName, required String sha256}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'publish_artifact',
        args: _coreProxyArgs(<String, Object?>{'r#type': type, 'title': title, 'description': description, 'detail': detail, 'category_id': categoryId, 'allow_public_updates': allowPublicUpdates, 'version': version, 'format_ver': formatVer, 'min_app_ver': minAppVer, 'max_app_ver': maxAppVer, 'changelog': changelog, 'project_id': projectId, 'runtime_package_id': runtimePackageId, 'asset_kind': assetKind, 'asset_url': assetUrl, 'gh_owner': ghOwner, 'gh_repo': ghRepo, 'gh_release_tag': ghReleaseTag, 'asset_name': assetName, 'sha256': sha256}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketPublishResponse>(responseBytes, decode: (reader) => MarketPublishResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Publishes a marketplace entry backed by a GitHub repository.
  Future<MarketPublishResponse> publishRepoEntry({required String type, required String title, required String description, required String detail, required String categoryId, required bool allowPublicUpdates, required String sourceUrl, required String refType, required String refName, required String installConfig, required String version, required String formatVer, required String minAppVer, required String? maxAppVer, required String? changelog}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'publish_repo_entry',
        args: _coreProxyArgs(<String, Object?>{'r#type': type, 'title': title, 'description': description, 'detail': detail, 'category_id': categoryId, 'allow_public_updates': allowPublicUpdates, 'source_url': sourceUrl, 'ref_type': refType, 'ref_name': refName, 'install_config': installConfig, 'version': version, 'format_ver': formatVer, 'min_app_ver': minAppVer, 'max_app_ver': maxAppVer, 'changelog': changelog}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketPublishResponse>(responseBytes, decode: (reader) => MarketPublishResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates mutable metadata for a marketplace entry.
  Future<MarketEntryUpdateResponse> updateEntry({required String entryId, required String? title, required String? description, required String? detail, required String? categoryId, required bool? allowPublicUpdates}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'update_entry',
        args: _coreProxyArgs(<String, Object?>{'entry_id': entryId, 'title': title, 'description': description, 'detail': detail, 'category_id': categoryId, 'allow_public_updates': allowPublicUpdates}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketEntryUpdateResponse>(responseBytes, decode: (reader) => MarketEntryUpdateResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Publishes a new artifact version for an existing marketplace entry.
  Future<MarketPublishResponse> publishArtifactVersion({required String entryId, required String version, required String formatVer, required String minAppVer, required String? maxAppVer, required String? changelog, required String projectId, required String runtimePackageId, required String assetKind, required String assetUrl, required String ghOwner, required String ghRepo, required String ghReleaseTag, required String assetName, required String sha256, required String? entryTitle, required String? entryDescription, required String? entryDetail, required String? entryCategoryId, required bool? entryAllowPublicUpdates}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'publish_artifact_version',
        args: _coreProxyArgs(<String, Object?>{'entry_id': entryId, 'version': version, 'format_ver': formatVer, 'min_app_ver': minAppVer, 'max_app_ver': maxAppVer, 'changelog': changelog, 'project_id': projectId, 'runtime_package_id': runtimePackageId, 'asset_kind': assetKind, 'asset_url': assetUrl, 'gh_owner': ghOwner, 'gh_repo': ghRepo, 'gh_release_tag': ghReleaseTag, 'asset_name': assetName, 'sha256': sha256, 'entry_title': entryTitle, 'entry_description': entryDescription, 'entry_detail': entryDetail, 'entry_category_id': entryCategoryId, 'entry_allow_public_updates': entryAllowPublicUpdates}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketPublishResponse>(responseBytes, decode: (reader) => MarketPublishResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Publishes a new repository version for an existing marketplace entry.
  Future<MarketPublishResponse> publishRepoVersion({required String entryId, required String version, required String formatVer, required String minAppVer, required String? maxAppVer, required String? changelog, required String refType, required String refName, required String installConfig, required String? entryTitle, required String? entryDescription, required String? entryDetail, required String? entryCategoryId, required bool? entryAllowPublicUpdates}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'publish_repo_version',
        args: _coreProxyArgs(<String, Object?>{'entry_id': entryId, 'version': version, 'format_ver': formatVer, 'min_app_ver': minAppVer, 'max_app_ver': maxAppVer, 'changelog': changelog, 'ref_type': refType, 'ref_name': refName, 'install_config': installConfig, 'entry_title': entryTitle, 'entry_description': entryDescription, 'entry_detail': entryDetail, 'entry_category_id': entryCategoryId, 'entry_allow_public_updates': entryAllowPublicUpdates}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MarketPublishResponse>(responseBytes, decode: (reader) => MarketPublishResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Downloads a marketplace asset by id.
  Future<Uint8List> downloadAsset({required String assetId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'download_asset',
        args: _coreProxyArgs(<String, Object?>{'asset_id': assetId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Uint8List>(responseBytes, decode: (reader) => reader.readBytes(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedRepositoryChatHistoryManagerCoreProxy {
  const GeneratedRepositoryChatHistoryManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

}

class GeneratedRepositoryMemoryRepositoryCoreProxy {
  const GeneratedRepositoryMemoryRepositoryCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Searches memories by lexical relevance, folder, and creation-time filters.
  Future<List<Memory>> searchMemories({required String query, required String? folderPath, required double relevanceThreshold, required int? createdAtStartMs, required int? createdAtEndMs}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'searchMemories',
        args: _coreProxyArgs(<String, Object?>{'query': query, 'folderPath': folderPath, 'relevanceThreshold': relevanceThreshold, 'createdAtStartMs': createdAtStartMs, 'createdAtEndMs': createdAtEndMs}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<Memory>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<Memory>.generate(length, (_) => Memory.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Finds the first memory with the exact normalized title.
  Future<Memory?> findMemoryByTitle({required String title}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'findMemoryByTitle',
        args: _coreProxyArgs(<String, Object?>{'title': title}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Memory?>(responseBytes, decode: (reader) => reader.readNullable<Memory>(() => Memory.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Finds all memories with the exact normalized title.
  Future<List<Memory>> findMemoriesByTitle({required String title}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'findMemoriesByTitle',
        args: _coreProxyArgs(<String, Object?>{'title': title}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<Memory>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<Memory>.generate(length, (_) => Memory.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns memories stored under one normalized folder path.
  Future<List<Memory>> getMemoriesByFolderPath({required String folderPath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getMemoriesByFolderPath',
        args: _coreProxyArgs(<String, Object?>{'folderPath': folderPath}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<Memory>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<Memory>.generate(length, (_) => Memory.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates and stores a new memory with generated identity and timestamps.
  Future<Memory> createMemory({required String title, required String content, required String contentType, required String source, required String folderPath, required List<String>? tags}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createMemory',
        args: _coreProxyArgs(<String, Object?>{'title': title, 'content': content, 'contentType': contentType, 'source': source, 'folderPath': folderPath, 'tags': tags?.map((item) => item).toList(growable: false)}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Memory>(responseBytes, decode: (reader) => Memory.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Saves a memory after normalizing identity, timestamps, and folder path.
  Future<Memory> saveMemory({required Memory memory}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'saveMemory',
        args: _coreProxyArgs(<String, Object?>{'memory': memory.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Memory>(responseBytes, decode: (reader) => Memory.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Adds a tag to a memory and returns the updated memory.
  Future<Memory?> addTagToMemory({required int memoryId, required String tagName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'addTagToMemory',
        args: _coreProxyArgs(<String, Object?>{'memoryId': memoryId, 'tagName': tagName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Memory?>(responseBytes, decode: (reader) => reader.readNullable<Memory>(() => Memory.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Merges memories with matching source titles into one primary memory.
  Future<Memory?> mergeMemories({required List<String> sourceTitles, required String newTitle, required String newContent, required List<String> newTags, required String folderPath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'mergeMemories',
        args: _coreProxyArgs(<String, Object?>{'sourceTitles': sourceTitles.map((item) => item).toList(growable: false), 'newTitle': newTitle, 'newContent': newContent, 'newTags': newTags.map((item) => item).toList(growable: false), 'folderPath': folderPath}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Memory?>(responseBytes, decode: (reader) => reader.readNullable<Memory>(() => Memory.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates one memory's content, scoring metadata, folder, and tags.
  Future<Memory> updateMemory({required int memoryId, required String newTitle, required String newContent, required String newContentType, required String newSource, required double newCredibility, required double newImportance, required String? newFolderPath, required List<String>? newTags}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateMemory',
        args: _coreProxyArgs(<String, Object?>{'memoryId': memoryId, 'newTitle': newTitle, 'newContent': newContent, 'newContentType': newContentType, 'newSource': newSource, 'newCredibility': newCredibility, 'newImportance': newImportance, 'newFolderPath': newFolderPath, 'newTags': newTags?.map((item) => item).toList(growable: false)}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Memory>(responseBytes, decode: (reader) => Memory.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes one memory and removes links that reference it.
  Future<bool> deleteMemory({required int memoryId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteMemory',
        args: _coreProxyArgs(<String, Object?>{'memoryId': memoryId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Moves selected memories into a normalized target folder.
  Future<bool> moveMemoriesToFolder({required Object? memoryIds, required String targetFolderPath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'moveMemoriesToFolder',
        args: _coreProxyArgs(<String, Object?>{'memoryIds': memoryIds, 'targetFolderPath': targetFolderPath}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a typed weighted link between two existing memories.
  Future<MemoryLink> linkMemories({required int sourceMemoryId, required int targetMemoryId, required String type, required double weight, required String description}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'linkMemories',
        args: _coreProxyArgs(<String, Object?>{'sourceMemoryId': sourceMemoryId, 'targetMemoryId': targetMemoryId, 'type_': type, 'weight': weight, 'description': description}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MemoryLink>(responseBytes, decode: (reader) => MemoryLink.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes one memory link by id.
  Future<bool> deleteLink({required int linkId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteLink',
        args: _coreProxyArgs(<String, Object?>{'linkId': linkId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Lists every folder path currently used by stored memories.
  Future<List<String>> getAllFolderPaths() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllFolderPaths',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<String>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<String>.generate(length, (_) => reader.readString(), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Builds the current memory graph after removing dangling links.
  Future<MemoryGraph> getMemoryGraph() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getMemoryGraph',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MemoryGraph>(responseBytes, decode: (reader) => MemoryGraph.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Exports user memories and their internal links as a portable JSON document.
  Future<String> exportMemoriesToJson() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exportMemoriesToJson',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports memories and links from a portable JSON document.
  Future<MemoryImportResult> importMemoriesFromJson({required String jsonString, required ImportStrategy strategy}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importMemoriesFromJson',
        args: _coreProxyArgs(<String, Object?>{'jsonString': jsonString, 'strategy': strategy.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<MemoryImportResult>(responseBytes, decode: (reader) => MemoryImportResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedRepositoryRuntimeStorageRepositoryCoreProxy {
  const GeneratedRepositoryRuntimeStorageRepositoryCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Reads a UTF-8 text object from runtime storage.
  Future<String?> readText({required String path}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'readText',
        args: _coreProxyArgs(<String, Object?>{'path': path}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads a runtime storage object and returns its base64 representation.
  Future<String?> readBase64({required String path}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'readBase64',
        args: _coreProxyArgs(<String, Object?>{'path': path}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String?>(responseBytes, decode: (reader) => reader.readNullable<String>(() => reader.readString()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Writes UTF-8 text content to runtime storage.
  Future<void> writeText({required String path, required String content}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'writeText',
        args: _coreProxyArgs(<String, Object?>{'path': path, 'content': content}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Appends UTF-8 text content to runtime storage.
  Future<void> appendText({required String path, required String content}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'appendText',
        args: _coreProxyArgs(<String, Object?>{'path': path, 'content': content}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Decodes base64 content and writes the bytes to runtime storage.
  Future<void> writeBase64({required String path, required String base64Content}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'writeBase64',
        args: _coreProxyArgs(<String, Object?>{'path': path, 'base64Content': base64Content}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Deletes one runtime storage object using its registered synchronization scope.
  Future<void> delete({required String path}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'delete',
        args: _coreProxyArgs(<String, Object?>{'path': path}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Returns the runtime storage path for browser bookmark data.
  Future<String> webSessionBrowserBookmarksPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'webSessionBrowserBookmarksPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage path for browser history data.
  Future<String> webSessionBrowserHistoryPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'webSessionBrowserHistoryPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage path for browser download metadata.
  Future<String> webSessionBrowserDownloadsPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'webSessionBrowserDownloadsPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage directory for character avatar assets.
  Future<String> characterAvatarsDirPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'characterAvatarsDirPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage directory path for downloaded browser files.
  Future<String> webSessionBrowserDownloadFilesDirPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'webSessionBrowserDownloadFilesDirPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage directory path for imported theme assets.
  Future<String> themeAssetsDirPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'themeAssetsDirPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage directory for generated share images.
  Future<String> shareImageDirPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'shareImageDirPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage directory for exported share images.
  Future<String> shareImageExportsDirPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'shareImageExportsDirPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage directory for staged workspace videos.
  Future<String> workspaceVideoDirPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'workspaceVideoDirPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage directory for Compose DSL selected files.
  Future<String> composeDslWebViewFilesDirPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'composeDslWebViewFilesDirPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage directory for materialized Link Access web assets.
  Future<String> linkAccessWebAssetsDirPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'linkAccessWebAssetsDirPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage path for the client log.
  Future<String> clientLogPath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clientLogPath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the runtime storage path for userscript state data.
  Future<String> webSessionUserscriptsStatePath() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'webSessionUserscriptsStatePath',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedRepositoryUsageStatisticsStoreCoreProxy {
  const GeneratedRepositoryUsageStatisticsStoreCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Reads all recorded provider model requests ordered by creation time.
  Future<List<UsageRequestRecord>> getAllRequestRecords() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getAllRequestRecords',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<UsageRequestRecord>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<UsageRequestRecord>.generate(length, (_) => UsageRequestRecord.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes every recorded provider model request.
  Future<void> clearAllRequestRecords() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'clearAllRequestRecords',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Records token usage for one provider model request.
  Future<UsageRequestRecord> recordProviderModelRequest({required String providerModel, required FunctionType functionType, required UsageRequestSource source, required String? chatId, required int inputTokens, required int outputTokens, required int cachedInputTokens}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'recordProviderModelRequest',
        args: _coreProxyArgs(<String, Object?>{'providerModel': providerModel, 'functionType': functionType.toJson(), 'source': source.toJson(), 'chatId': chatId, 'inputTokens': inputTokens, 'outputTokens': outputTokens, 'cachedInputTokens': cachedInputTokens}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<UsageRequestRecord>(responseBytes, decode: (reader) => UsageRequestRecord.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServerCoreNodeRouterCoreProxy {
  const GeneratedServerCoreNodeRouterCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Returns the stable identity of the CoreNode that owns this router.
  Future<String> localNodeId() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'localNodeId',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Resolves one generated Core schema through the owning local runtime.
  Future<int?> objectIdForSchema({required String schema}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'objectIdForSchema',
        args: _coreProxyArgs(<String, Object?>{'schema': schema}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int?>(responseBytes, decode: (reader) => reader.readNullable<int>(() => reader.readInt()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reports whether the active Peer Link graph currently proves one device reachable.
  Future<bool> nodeIsReachable({required String targetNodeId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'nodeIsReachable',
        args: _coreProxyArgs(<String, Object?>{'targetNodeId': targetNodeId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServerRuntimeRemoteLinkServiceCoreProxy {
  const GeneratedServerRuntimeRemoteLinkServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Returns the converged Space membership owned by this CoreNode.
  Future<CoreSpace> deviceSpace() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deviceSpace',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreSpace>(responseBytes, decode: (reader) => CoreSpace.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the synchronized device metadata and direct-connection graph.
  Future<RuntimeDeviceSpaceTopology> deviceSpaceTopology() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deviceSpaceTopology',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeDeviceSpaceTopology>(responseBytes, decode: (reader) => RuntimeDeviceSpaceTopology.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Publishes the active user identity name as synchronized device metadata.
  Future<RuntimeDeviceSpaceDevice> updateCurrentDeviceUserName({required String userName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateCurrentDeviceUserName',
        args: _coreProxyArgs(<String, Object?>{'userName': userName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeDeviceSpaceDevice>(responseBytes, decode: (reader) => RuntimeDeviceSpaceDevice.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Adopts Space membership received through an explicit authenticated join.
  Future<CoreSpace> adoptDeviceSpace({required CoreSpace space}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'adoptDeviceSpace',
        args: _coreProxyArgs(<String, Object?>{'space': space.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreSpace>(responseBytes, decode: (reader) => CoreSpace.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Records one directly paired device's current Space projection.
  Future<CoreSpace> observePairedDeviceSpace({required String deviceId, required CoreSpace space}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'observePairedDeviceSpace',
        args: _coreProxyArgs(<String, Object?>{'deviceId': deviceId, 'space': space.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreSpace>(responseBytes, decode: (reader) => CoreSpace.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Renames the current Space and returns its new synchronized identity.
  Future<CoreSpace> renameDeviceSpace({required String spaceName}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'renameDeviceSpace',
        args: _coreProxyArgs(<String, Object?>{'spaceName': spaceName}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreSpace>(responseBytes, decode: (reader) => CoreSpace.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Leaves the current device space while preserving all direct pairing records.
  Future<CoreSpace> leaveDeviceSpace() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'leaveDeviceSpace',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreSpace>(responseBytes, decode: (reader) => CoreSpace.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Joins the Space exposed by one directly paired CoreNode.
  Future<CoreSpace> joinPairedDeviceSpace({required String name}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'joinPairedDeviceSpace',
        args: _coreProxyArgs(<String, Object?>{'name': name}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<CoreSpace>(responseBytes, decode: (reader) => CoreSpace.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads paired devices with inbound and outbound records merged by device id.
  Future<Map<String, RuntimePairedDevice>> pairedDevicesSnapshot() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'pairedDevicesSnapshot',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Map<String, RuntimePairedDevice>>(responseBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, RuntimePairedDevice>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = RuntimePairedDevice.fromMessagePack(reader); } return result; })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Observes paired devices after merging both connection directions by device id.
  Stream<Map<String, RuntimePairedDevice>> pairedDevicesFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'pairedDevicesFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<String, RuntimePairedDevice>>(event, decode: (valueBytes) => decodeCoreLink<Map<String, RuntimePairedDevice>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, RuntimePairedDevice>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = RuntimePairedDevice.fromMessagePack(reader); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Observes paired device statuses from paired records and Peer Links.
  Stream<Map<String, RuntimePairedDeviceStatus>> pairedDeviceStatusesFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'pairedDeviceStatusesFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Map<String, RuntimePairedDeviceStatus>>(event, decode: (valueBytes) => decodeCoreLink<Map<String, RuntimePairedDeviceStatus>>(valueBytes, decode: (reader) => (() { final length = reader.readMapLength(); final result = <String, RuntimePairedDeviceStatus>{}; for (var index = 0; index < length; index += 1) { result[reader.readString()] = RuntimePairedDeviceStatus.fromMessagePack(reader); } return result; })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns whether one paired device currently has an active Peer Link.
  Future<bool> pairedDeviceOnline({required String deviceId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'pairedDeviceOnline',
        args: _coreProxyArgs(<String, Object?>{'deviceId': deviceId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<bool>(responseBytes, decode: (reader) => reader.readBool(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Commits a chat Binding change, installs it on the target Core, and resumes there.
  Future<void> requestChangeRoute({required String chatId, required String targetNodeId, required CoreRouteResumeContext resumeContext}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'requestChangeRoute',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'targetNodeId': targetNodeId, 'resumeContext': resumeContext.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Resolves the persisted pairing and reports revocation or Space removal explicitly.
  Future<RuntimePairedDeviceStatus> pairedDeviceStatus({required String deviceId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'pairedDeviceStatus',
        args: _coreProxyArgs(<String, Object?>{'deviceId': deviceId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimePairedDeviceStatus>(responseBytes, decode: (reader) => RuntimePairedDeviceStatus.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Disconnects one directly adjacent device while preserving pairing records.
  Future<void> disconnectDeviceSpaceConnection({required String deviceId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'disconnectDeviceSpaceConnection',
        args: _coreProxyArgs(<String, Object?>{'deviceId': deviceId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Removes every local pairing record associated with one device.
  Future<void> removePairedDevice({required String deviceId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'removePairedDevice',
        args: _coreProxyArgs(<String, Object?>{'deviceId': deviceId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Starts the singleton persistent synchronization worker for direct Space peers.
  Future<void> startSpaceSync() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'startSpaceSync',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Stops the persistent synchronization worker owned by this CoreNode.
  Future<void> stopSpaceSync() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'stopSpaceSync',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Discovers nearby Spaces and groups their directly connectable CoreNodes.
  Future<List<RuntimeRemoteDiscoveredSpace>> discoverSpaces({required int timeoutMs}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'discoverSpaces',
        args: _coreProxyArgs(<String, Object?>{'timeoutMs': timeoutMs}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<RuntimeRemoteDiscoveredSpace>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<RuntimeRemoteDiscoveredSpace>.generate(length, (_) => RuntimeRemoteDiscoveredSpace.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Starts a runtime-owned outbound pairing and stores its confidential client state.
  Future<RuntimeRemotePairStartResult> startPairedRemote({required String baseUrl, required String tokenHash, required RemoteDeviceInfo clientDeviceInfo}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'startPairedRemote',
        args: _coreProxyArgs(<String, Object?>{'baseUrl': baseUrl, 'tokenHash': tokenHash, 'clientDeviceInfo': clientDeviceInfo.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeRemotePairStartResult>(responseBytes, decode: (reader) => RuntimeRemotePairStartResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Completes a runtime-owned outbound pairing and stores its named direct connection.
  Future<PairedRemoteSessionRecord> finishPairedRemote({required String pairingId, required String pairingCode, required String name}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'finishPairedRemote',
        args: _coreProxyArgs(<String, Object?>{'pairingId': pairingId, 'pairingCode': pairingCode, 'name': name}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<PairedRemoteSessionRecord>(responseBytes, decode: (reader) => PairedRemoteSessionRecord.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Persists the explicit carrier selected for one named outbound session.
  Future<PairedRemoteSessionRecord> setPairedRemoteTransport({required String name, required LinkTransportPreference transport}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'setPairedRemoteTransport',
        args: _coreProxyArgs(<String, Object?>{'name': name, 'transport': transport.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<PairedRemoteSessionRecord>(responseBytes, decode: (reader) => PairedRemoteSessionRecord.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServicesArchiveTransferManagerCoreProxy {
  const GeneratedServicesArchiveTransferManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Creates one host-owned target for a streamed archive upload of an exact byte length.
  Future<String> beginArchiveUpload({required int expectedByteLength}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'beginArchiveUpload',
        args: _coreProxyArgs(<String, Object?>{'expectedByteLength': expectedByteLength}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Writes one complete caller-owned input stream into an archive upload.
  Future<void> writeArchiveUpload({required String archiveId, required Stream<Uint8List> bytes}) async {
    final sink = await bridge.push(
      CorePushRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'writeArchiveUpload',
        args: _coreProxyArgs(<String, Object?>{'archiveId': archiveId}, objectArgs),
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
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'completeArchiveUpload',
        args: _coreProxyArgs(<String, Object?>{'archiveId': archiveId, 'expectedByteLength': expectedByteLength}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<StagedArchive>(responseBytes, decode: (reader) => StagedArchive.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Removes one host-owned archive upload regardless of its sealed state.
  Future<void> discardArchiveUpload({required String archiveId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'discardArchiveUpload',
        args: _coreProxyArgs(<String, Object?>{'archiveId': archiveId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedServicesGitHubOAuthBrokerServiceCoreProxy {
  const GeneratedServicesGitHubOAuthBrokerServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Starts one broker transaction after an application prepares its completion destination.
  Future<GitHubOAuthBrokerLoginStart> startLogin({required String completionRedirectUri}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'startLogin',
        args: _coreProxyArgs(<String, Object?>{'completionRedirectUri': completionRedirectUri}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<GitHubOAuthBrokerLoginStart>(responseBytes, decode: (reader) => GitHubOAuthBrokerLoginStart.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Validates one browser completion URL, claims its transaction once, and persists the session.
  Future<GitHubOAuthBrokerLoginResult> completeLogin({required GitHubOAuthBrokerLoginCompletion completion}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'completeLogin',
        args: _coreProxyArgs(<String, Object?>{'completion': completion.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<GitHubOAuthBrokerLoginResult>(responseBytes, decode: (reader) => GitHubOAuthBrokerLoginResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServicesLocalModelServiceCoreProxy {
  const GeneratedServicesLocalModelServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Returns model catalog entries with installed model and engine state.
  Future<List<LocalModelCatalogStatus>> getCatalogStatus() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getCatalogStatus',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<LocalModelCatalogStatus>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<LocalModelCatalogStatus>.generate(length, (_) => LocalModelCatalogStatus.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the shared local model and engine registry snapshot.
  Future<LocalModelRegistrySnapshot> getRegistry() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getRegistry',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LocalModelRegistrySnapshot>(responseBytes, decode: (reader) => LocalModelRegistrySnapshot.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the current local engine platform target.
  Future<LocalPlatformTarget> getPlatformTarget() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getPlatformTarget',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LocalPlatformTarget>(responseBytes, decode: (reader) => LocalPlatformTarget.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Installs one model and its exact platform engine dependency.
  Future<LocalModelBundleInstallResult> installModel({required String modelId, required String version}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'installModel',
        args: _coreProxyArgs(<String, Object?>{'modelId': modelId, 'version': version}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LocalModelBundleInstallResult>(responseBytes, decode: (reader) => LocalModelBundleInstallResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns every installation operation retained by this runtime process.
  Future<List<LocalModelInstallStatus>> getInstallStatuses() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getInstallStatuses',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<LocalModelInstallStatus>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<LocalModelInstallStatus>.generate(length, (_) => LocalModelInstallStatus.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns one installation operation by exact model id and version.
  Future<LocalModelInstallStatus?> getInstallStatus({required String modelId, required String version}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getInstallStatus',
        args: _coreProxyArgs(<String, Object?>{'modelId': modelId, 'version': version}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LocalModelInstallStatus?>(responseBytes, decode: (reader) => reader.readNullable<LocalModelInstallStatus>(() => LocalModelInstallStatus.fromMessagePack(reader)), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Requests cancellation for one active model and engine installation.
  Future<LocalModelInstallStatus> cancelInstall({required String modelId, required String version}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'cancelInstall',
        args: _coreProxyArgs(<String, Object?>{'modelId': modelId, 'version': version}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LocalModelInstallStatus>(responseBytes, decode: (reader) => LocalModelInstallStatus.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Verifies one installed model and its exact platform engine.
  Future<LocalModelCatalogStatus> verifyModel({required String modelId, required String version}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'verifyModel',
        args: _coreProxyArgs(<String, Object?>{'modelId': modelId, 'version': version}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LocalModelCatalogStatus>(responseBytes, decode: (reader) => LocalModelCatalogStatus.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes one installed or retained local model download from the asset repository.
  Future<void> deleteModel({required String modelId, required String version}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteModel',
        args: _coreProxyArgs(<String, Object?>{'modelId': modelId, 'version': version}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Deletes one engine target that is not required by installed models.
  Future<void> deleteEngine({required String engineId, required String version}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteEngine',
        args: _coreProxyArgs(<String, Object?>{'engineId': engineId, 'version': version}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedServicesLocalProviderServiceCoreProxy {
  const GeneratedServicesLocalProviderServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Transcribes audio with one exact LOCAL_MODEL provider model.
  Future<LocalSttResponse> transcribeAudio({required LocalModelSelection model, required String audioPath, required String? language}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'transcribeAudio',
        args: _coreProxyArgs(<String, Object?>{'model': model.toJson(), 'audioPath': audioPath, 'language': language}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LocalSttResponse>(responseBytes, decode: (reader) => LocalSttResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Synthesizes WAV audio with one exact LOCAL_MODEL provider model.
  Future<LocalTtsResponse> synthesizeAudio({required LocalModelSelection model, required String text, required String voice, required double speed}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'synthesizeAudio',
        args: _coreProxyArgs(<String, Object?>{'model': model.toJson(), 'text': text, 'voice': voice, 'speed': speed}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<LocalTtsResponse>(responseBytes, decode: (reader) => LocalTtsResponse.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServicesRuntimeBrowserServiceCoreProxy {
  const GeneratedServicesRuntimeBrowserServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Lists browser sessions currently owned by the owner host.
  Future<List<RuntimeBrowserSessionInfo>> listBrowserSessions() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'listBrowserSessions',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<RuntimeBrowserSessionInfo>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<RuntimeBrowserSessionInfo>.generate(length, (_) => RuntimeBrowserSessionInfo.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Creates a browser session on the owner host.
  Future<RuntimeBrowserSessionInfo> createBrowserSession({required String initialUrl, required String? userAgent, required Map<String, String> headers}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'createBrowserSession',
        args: _coreProxyArgs(<String, Object?>{'initialUrl': initialUrl, 'userAgent': userAgent, 'headers': headers.map((key, value) => MapEntry(key, value))}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeBrowserSessionInfo>(responseBytes, decode: (reader) => RuntimeBrowserSessionInfo.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Updates browser session metadata on the owner host.
  Future<RuntimeBrowserSessionInfo> updateBrowserSession({required String sessionId, required String? userAgent, required Map<String, String> headers}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'updateBrowserSession',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId, 'userAgent': userAgent, 'headers': headers.map((key, value) => MapEntry(key, value))}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeBrowserSessionInfo>(responseBytes, decode: (reader) => RuntimeBrowserSessionInfo.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the shared event stream used by an attached browser controller.
  Stream<RuntimeBrowserStreamEvent> browserSessionEvents({required String sessionId}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'browserSessionEvents', args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<RuntimeBrowserStreamEvent>(event, decode: (valueBytes) => decodeCoreLink<RuntimeBrowserStreamEvent>(valueBytes, decode: (reader) => RuntimeBrowserStreamEvent.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Submits one semantic browser command to the owner host.
  Future<RuntimeBrowserCommandResult> submitBrowserCommand({required RuntimeBrowserCommand command}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'submitBrowserCommand',
        args: _coreProxyArgs(<String, Object?>{'command': command.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeBrowserCommandResult>(responseBytes, decode: (reader) => RuntimeBrowserCommandResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Applies ordered compositor interactions supplied by one caller-owned input stream.
  Future<void> submitBrowserInteractions({required Stream<RuntimeBrowserCommand> commands}) async {
    final sink = await bridge.push(
      CorePushRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'submitBrowserInteractions',
        args: _coreProxyArgs(<String, Object?>{}, objectArgs),
      ),
    );
    try {
      await for (final item in commands) {
        await sink.add(item.toJson());
      }
    } finally {
      await sink.close();
    }
  }

  /// Returns the latest browser session snapshot for a controller.
  Future<RuntimeBrowserSessionSnapshot> getBrowserSessionSnapshot({required String sessionId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getBrowserSessionSnapshot',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeBrowserSessionSnapshot>(responseBytes, decode: (reader) => RuntimeBrowserSessionSnapshot.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Publishes one browser session event from the owner host.
  Future<void> publishBrowserSessionEvent({required RuntimeBrowserSessionEvent event}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'publishBrowserSessionEvent',
        args: _coreProxyArgs(<String, Object?>{'event': event.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Explicitly closes a browser session on the owner host.
  Future<void> closeBrowserSession({required String sessionId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'closeBrowserSession',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

}

class GeneratedServicesRuntimeHostInfoServiceCoreProxy {
  const GeneratedServicesRuntimeHostInfoServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Returns the captured runtime host descriptor.
  Future<RuntimeHostDescriptor> runtimeHostDescriptor() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'runtimeHostDescriptor',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeHostDescriptor>(responseBytes, decode: (reader) => RuntimeHostDescriptor.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServicesRuntimeHostInteractionServiceCoreProxy {
  const GeneratedServicesRuntimeHostInteractionServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Sends one application notification through the active system-operation host.
  Future<void> sendSystemNotification({required String title, required String message, required String? chatId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'sendSystemNotification',
        args: _coreProxyArgs(<String, Object?>{'title': title, 'message': message, 'chatId': chatId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Responds to a pending owner-host interaction request.
  Future<void> respondOwnerHostInteraction({required String requestId, required RuntimeHostInteractionResponse response}) async {
    final responseBytes = await bridge.callControlBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'respondOwnerHostInteraction',
        args: _coreProxyArgs(<String, Object?>{'requestId': requestId, 'response': response.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Acknowledges a non-blocking owner-host notification after it is consumed.
  Future<void> acknowledgeOwnerHostInteraction({required String requestId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'acknowledgeOwnerHostInteraction',
        args: _coreProxyArgs(<String, Object?>{'requestId': requestId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Creates an event stream for owner-host interaction requests of selected kinds.
  Stream<RuntimeHostInteractionRequest> ownerHostInteractionEvents({required List<RuntimeHostInteractionKind> kinds}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'ownerHostInteractionEvents', args: _coreProxyArgs(<String, Object?>{'kinds': kinds.map((item) => item.toJson()).toList(growable: false)}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<RuntimeHostInteractionRequest>(event, decode: (valueBytes) => decodeCoreLink<RuntimeHostInteractionRequest>(valueBytes, decode: (reader) => RuntimeHostInteractionRequest.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

}

class GeneratedServicesRuntimeTerminalServiceCoreProxy {
  const GeneratedServicesRuntimeTerminalServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Lists terminal sessions currently known by the host.
  Future<List<RuntimeTerminalSessionInfo>> listTerminalSessions() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'listTerminalSessions',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<RuntimeTerminalSessionInfo>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<RuntimeTerminalSessionInfo>.generate(length, (_) => RuntimeTerminalSessionInfo.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns a state flow of published terminal sessions.
  Stream<List<RuntimeTerminalSessionInfo>> terminalSessionsFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'terminalSessionsFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<List<RuntimeTerminalSessionInfo>>(event, decode: (valueBytes) => decodeCoreLink<List<RuntimeTerminalSessionInfo>>(valueBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<RuntimeTerminalSessionInfo>.generate(length, (_) => RuntimeTerminalSessionInfo.fromMessagePack(reader), growable: false); })(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Returns the host-declared terminal type for manual PTY creation.
  Future<String> defaultTerminalType() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'defaultTerminalType',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns every terminal type that the active host exposes to users.
  Future<RuntimeTerminalInfo> terminalInfo() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'terminalInfo',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeTerminalInfo>(responseBytes, decode: (reader) => RuntimeTerminalInfo.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Starts a PTY terminal session and attaches its output stream.
  Future<String> startTerminalPty({required String sessionName, required String terminal, required String terminalType, required String workingDir, required int rows, required int cols}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'startTerminalPty',
        args: _coreProxyArgs(<String, Object?>{'sessionName': sessionName, 'terminal': terminal, 'terminalType': terminalType, 'workingDir': workingDir, 'rows': rows, 'cols': cols}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Returns the shared output stream for a PTY session.
  Stream<String> terminalPtyOutput({required String sessionId}) {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'terminalPtyOutput', args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<String>(event, decode: (valueBytes) => decodeCoreLink<String>(valueBytes, decode: (reader) => reader.readString(), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

  /// Writes base64-encoded bytes to a PTY session.
  Future<int> writeTerminalPty({required String sessionId, required String dataBase64}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'writeTerminalPty',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId, 'dataBase64': dataBase64}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Resizes a PTY session.
  Future<void> resizeTerminalPty({required String sessionId, required int rows, required int cols}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resizeTerminalPty',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId, 'rows': rows, 'cols': cols}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Polls the exit code for a PTY session.
  Future<int?> pollTerminalPtyExit({required String sessionId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'pollTerminalPtyExit',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int?>(responseBytes, decode: (reader) => reader.readNullable<int>(() => reader.readInt()), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Closes a PTY session and removes its output stream.
  Future<void> closeTerminalPty({required String sessionId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'closeTerminalPty',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Sends text input to a terminal session.
  Future<int> inputTerminalSession({required String sessionId, required String input}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'inputTerminalSession',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId, 'input': input}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads the current screen contents for a terminal session.
  Future<RuntimeTerminalScreen> getTerminalSessionScreen({required String sessionId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'getTerminalSessionScreen',
        args: _coreProxyArgs(<String, Object?>{'sessionId': sessionId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RuntimeTerminalScreen>(responseBytes, decode: (reader) => RuntimeTerminalScreen.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServicesSnapshotImportManagerCoreProxy {
  const GeneratedServicesSnapshotImportManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Exports all raw runtime storage into a portable snapshot archive.
  Future<Uint8List> exportRawSnapshot() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'exportRawSnapshot',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Uint8List>(responseBytes, decode: (reader) => reader.readBytes(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads raw snapshot metadata from a sealed archive without changing runtime storage.
  Future<RawSnapshotManifest> inspectRawSnapshot({required StagedArchive archive}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'inspectRawSnapshot',
        args: _coreProxyArgs(<String, Object?>{'archive': archive.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<RawSnapshotManifest>(responseBytes, decode: (reader) => RawSnapshotManifest.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Restores one sealed raw snapshot after closing the active database handle.
  Future<void> restoreRawSnapshot({required StagedArchive archive}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'restoreRawSnapshot',
        args: _coreProxyArgs(<String, Object?>{'archive': archive.toJson()}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Previews an Operit1 model-configuration snapshot from a sealed archive.
  Future<Operit1ModelConfigSnapshotPreview> inspectOperit1ModelConfigSnapshot({required StagedArchive archive}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'inspectOperit1ModelConfigSnapshot',
        args: _coreProxyArgs(<String, Object?>{'archive': archive.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Operit1ModelConfigSnapshotPreview>(responseBytes, decode: (reader) => Operit1ModelConfigSnapshotPreview.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Previews an Operit1 full snapshot from a sealed archive.
  Future<Operit1SnapshotPreview> inspectOperit1Snapshot({required StagedArchive archive}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'inspectOperit1Snapshot',
        args: _coreProxyArgs(<String, Object?>{'archive': archive.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Operit1SnapshotPreview>(responseBytes, decode: (reader) => Operit1SnapshotPreview.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports an Operit1 model configuration from a sealed archive into the selected profile.
  Future<Operit1ModelConfigImportResult> importOperit1ModelConfigSnapshot({required StagedArchive archive, required String configId, required String modelId}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importOperit1ModelConfigSnapshot',
        args: _coreProxyArgs(<String, Object?>{'archive': archive.toJson(), 'configId': configId, 'modelId': modelId}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Operit1ModelConfigImportResult>(responseBytes, decode: (reader) => Operit1ModelConfigImportResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Imports one sealed Operit1 snapshot and publishes progress events.
  Future<Operit1SnapshotImportResult> importOperit1Snapshot({required StagedArchive archive}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'importOperit1Snapshot',
        args: _coreProxyArgs(<String, Object?>{'archive': archive.toJson()}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<Operit1SnapshotImportResult>(responseBytes, decode: (reader) => Operit1SnapshotImportResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Observes the latest Operit1 snapshot import progress state.
  Stream<Operit1SnapshotImportProgress> operit1SnapshotImportProgressFlow() {
    final eventValueDecoder = CoreLinkEventValueDecoder();
    return bridge
        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: 'operit1SnapshotImportProgressFlow', args: _coreProxyArgs(const <String, Object?>{}, objectArgs)))
        .where((event) => event.kind != 'Completed')
        .map((event) {
          return eventValueDecoder.decode<Operit1SnapshotImportProgress>(event, decode: (valueBytes) => decodeCoreLink<Operit1SnapshotImportProgress>(valueBytes, decode: (reader) => Operit1SnapshotImportProgress.fromMessagePack(reader), targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));
        });
  }

}

class GeneratedServicesSttRecognitionServiceCoreProxy {
  const GeneratedServicesSttRecognitionServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Transcribes one in-memory audio payload with the selected STT configuration.
  Future<SttRecognitionResult> transcribeCurrent({required Uint8List audioBytes, required String fileName, required String contentType, required String? language}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'transcribeCurrent',
        args: _coreProxyArgs(<String, Object?>{'audioBytes': audioBytes, 'fileName': fileName, 'contentType': contentType, 'language': language}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SttRecognitionResult>(responseBytes, decode: (reader) => SttRecognitionResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Transcribes one in-memory audio payload with an explicit STT configuration id.
  Future<SttRecognitionResult> transcribeWithConfigId({required String configId, required Uint8List audioBytes, required String fileName, required String contentType, required String? language}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'transcribeWithConfigId',
        args: _coreProxyArgs(<String, Object?>{'configId': configId, 'audioBytes': audioBytes, 'fileName': fileName, 'contentType': contentType, 'language': language}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<SttRecognitionResult>(responseBytes, decode: (reader) => SttRecognitionResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServicesSyncBlobTransferManagerCoreProxy {
  const GeneratedServicesSyncBlobTransferManagerCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Receives and verifies one complete content-addressed blob through a push stream.
  Future<void> syncReceiveBlob({required String contentHash, required int size, required Stream<Uint8List> chunks}) async {
    final sink = await bridge.push(
      CorePushRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'syncReceiveBlob',
        args: _coreProxyArgs(<String, Object?>{'contentHash': contentHash, 'size': size}, objectArgs),
      ),
    );
    try {
      await for (final item in chunks) {
        await sink.add(item);
      }
    } finally {
      await sink.close();
    }
  }

}

class GeneratedServicesTtsPlaybackServiceCoreProxy {
  const GeneratedServicesTtsPlaybackServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Plays a generated speech file through the TTS playback host.
  Future<TtsPlaybackResult> playAudio({required String path}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'playAudio',
        args: _coreProxyArgs(<String, Object?>{'path': path}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsPlaybackResult>(responseBytes, decode: (reader) => TtsPlaybackResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Speaks text with the TTS configuration bound to a character card.
  Future<TtsHostPlaybackResult> speakForCharacter({required String characterCardId, required String text, required bool interrupt}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'speakForCharacter',
        args: _coreProxyArgs(<String, Object?>{'characterCardId': characterCardId, 'text': text, 'interrupt': interrupt}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsHostPlaybackResult>(responseBytes, decode: (reader) => TtsHostPlaybackResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Speaks text with a selected TTS configuration.
  Future<TtsHostPlaybackResult> speakWithConfig({required String ttsConfigId, required String text, required bool interrupt}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'speakWithConfig',
        args: _coreProxyArgs(<String, Object?>{'ttsConfigId': ttsConfigId, 'text': text, 'interrupt': interrupt}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsHostPlaybackResult>(responseBytes, decode: (reader) => TtsHostPlaybackResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Pauses host speech playback.
  Future<TtsHostPlaybackResult> pauseSpeech() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'pauseSpeech',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsHostPlaybackResult>(responseBytes, decode: (reader) => TtsHostPlaybackResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Resumes host speech playback.
  Future<TtsHostPlaybackResult> resumeSpeech() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'resumeSpeech',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsHostPlaybackResult>(responseBytes, decode: (reader) => TtsHostPlaybackResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Stops host speech playback.
  Future<TtsHostPlaybackResult> stopSpeech() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'stopSpeech',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsHostPlaybackResult>(responseBytes, decode: (reader) => TtsHostPlaybackResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads current host speech playback state.
  Future<TtsHostPlaybackResult> speechState() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'speechState',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsHostPlaybackResult>(responseBytes, decode: (reader) => TtsHostPlaybackResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServicesTtsSynthesisServiceCoreProxy {
  const GeneratedServicesTtsSynthesisServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Synthesizes text with the TTS configuration bound to a character card.
  Future<TtsSynthesisResult> synthesizeForCharacter({required String characterCardId, required String text}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'synthesizeForCharacter',
        args: _coreProxyArgs(<String, Object?>{'characterCardId': characterCardId, 'text': text}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsSynthesisResult>(responseBytes, decode: (reader) => TtsSynthesisResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Synthesizes text with a selected TTS configuration.
  Future<TtsSynthesisResult> synthesizeWithConfig({required String ttsConfigId, required String text}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'synthesizeWithConfig',
        args: _coreProxyArgs(<String, Object?>{'ttsConfigId': ttsConfigId, 'text': text}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<TtsSynthesisResult>(responseBytes, decode: (reader) => TtsSynthesisResult.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

class GeneratedServicesWorkspaceServiceCoreProxy {
  const GeneratedServicesWorkspaceServiceCoreProxy._(this.bridge, this.objectId, {this.objectArgs = const <String, Object?>{}});

  final OperitRuntimeBridge bridge;

  final int objectId;

  final Map<String, Object?> objectArgs;

  /// Lists files under a chat-bound workspace relative path.
  Future<List<WorkspaceFileEntry>> listWorkspaceFiles({required String chatId, required String relativePath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'listWorkspaceFiles',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'relativePath': relativePath}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<WorkspaceFileEntry>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<WorkspaceFileEntry>.generate(length, (_) => WorkspaceFileEntry.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Lists directories that can be selected as workspace binding targets.
  Future<List<WorkspaceFileEntry>> listWorkspaceBindingDirectories({required String path}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'listWorkspaceBindingDirectories',
        args: _coreProxyArgs(<String, Object?>{'path': path}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<List<WorkspaceFileEntry>>(responseBytes, decode: (reader) => (() { final length = reader.readArrayLength(); return List<WorkspaceFileEntry>.generate(length, (_) => WorkspaceFileEntry.fromMessagePack(reader), growable: false); })(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads a text file from a chat-bound workspace.
  Future<String> readWorkspaceTextFile({required String chatId, required String relativePath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'readWorkspaceTextFile',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'relativePath': relativePath}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<String>(responseBytes, decode: (reader) => reader.readString(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Reads a binary file from a chat-bound workspace as base64.
  Future<WorkspaceFileBytes> readWorkspaceFileBytes({required String chatId, required String relativePath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'readWorkspaceFileBytes',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'relativePath': relativePath}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<WorkspaceFileBytes>(responseBytes, decode: (reader) => WorkspaceFileBytes.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Writes a text file into a chat-bound workspace.
  Future<void> writeWorkspaceTextFile({required String chatId, required String relativePath, required String content}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'writeWorkspaceTextFile',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'relativePath': relativePath, 'content': content}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Writes base64-decoded bytes into a chat-bound workspace file.
  Future<void> writeWorkspaceFileBytes({required String chatId, required String relativePath, required String base64Content}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'writeWorkspaceFileBytes',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'relativePath': relativePath, 'base64Content': base64Content}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Opens a chat-bound workspace file through the host file opener.
  Future<void> openWorkspaceFile({required String chatId, required String relativePath}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'openWorkspaceFile',
        args: _coreProxyArgs(<String, Object?>{'chatId': chatId, 'relativePath': relativePath}, objectArgs),
      ),
    );
    decodeNativeCoreVoidResult(responseBytes);
  }

  /// Builds the workspace-management summary for chat bindings and stored workspace folders.
  Future<WorkspaceManagementSummary> workspaceManagementSummary() async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'workspaceManagementSummary',
        args: _coreProxyArgs(const <String, Object?>{}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<WorkspaceManagementSummary>(responseBytes, decode: (reader) => WorkspaceManagementSummary.fromMessagePack(reader), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

  /// Deletes workspace folders that are not bound to any chat.
  Future<int> deleteUnboundWorkspaces({required List<String> workspaceNames}) async {
    final responseBytes = await bridge.callBytes(
      CoreCallRequest(
        requestId: _coreProxyRequestId(),
        targetObjectId: objectId,
        methodName: 'deleteUnboundWorkspaces',
        args: _coreProxyArgs(<String, Object?>{'workspaceNames': workspaceNames.map((item) => item).toList(growable: false)}, objectArgs),
      ),
    );
    return decodeNativeCoreResult<int>(responseBytes, decode: (reader) => reader.readInt(), targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);
  }

}

