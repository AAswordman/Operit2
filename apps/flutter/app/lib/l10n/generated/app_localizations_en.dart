// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get askOperitHint => 'Ask Operit';

  @override
  String get aiChat => 'AI Chat';

  @override
  String get fullscreenInput => 'Fullscreen input';

  @override
  String get expandInput => 'Expand input';

  @override
  String get collapseInput => 'Collapse input';

  @override
  String get settings => 'Settings';

  @override
  String get packageManager => 'Package manager';

  @override
  String get market => 'Market';

  @override
  String get addAttachment => 'Add attachment';

  @override
  String get attachmentPhoto => 'Photo';

  @override
  String get attachmentCamera => 'Camera';

  @override
  String get attachmentMemory => 'Memory';

  @override
  String get attachmentFile => 'File';

  @override
  String get attachmentScreenContent => 'Screen content';

  @override
  String get attachmentNotifications => 'Current notifications';

  @override
  String get attachmentLocation => 'Current location';

  @override
  String get attachmentPackage => 'Package';

  @override
  String get attachmentPackageSelectTitle => 'Select package';

  @override
  String get attachmentPackageEmpty => 'No available packages';

  @override
  String get attachmentPackageSearchPlaceholder =>
      'Search package name or description';

  @override
  String get attachmentPackageSearchEmpty => 'No matching packages';

  @override
  String get attachmentPackageKindPackage => 'Package';

  @override
  String get attachmentPackageKindSkill => 'Skill';

  @override
  String get attachmentPackageKindMcp => 'MCP';

  @override
  String get attachmentCameraUnavailable =>
      'Camera capture is not available in the Flutter client';

  @override
  String get attachmentMemoryUnavailable =>
      'Memory folder selection is not available in the Flutter client';

  @override
  String get clearSearch => 'Clear search';

  @override
  String chatPendingQueueTitle(int count) {
    return 'Queued messages ($count)';
  }

  @override
  String get chatQueueAddMessage => 'Queue message';

  @override
  String get chatQueueAdded => 'Added to queue';

  @override
  String get chatPleaseCreateNewChat => 'Please create a chat';

  @override
  String get cancel => 'Cancel';

  @override
  String get send => 'Send';

  @override
  String get model => 'Model';

  @override
  String get processingInput => 'Processing input...';

  @override
  String get processingMessage => 'Processing message...';

  @override
  String get connectingAiService => 'Connecting to AI service...';

  @override
  String get receivingAiResponse => 'Receiving AI response...';

  @override
  String get receivingToolResultAiResponse =>
      'Receiving AI response after tool execution...';

  @override
  String get roleResponsePlannerPlanning => 'Planning group speaking order...';

  @override
  String roleResponsePlannerMemberReplying(String memberName) {
    return 'Generating a reply from \"$memberName\"...';
  }

  @override
  String get roleResponsePlannerFailed => 'Group planning failed';

  @override
  String get summarizingMemories => 'Summarizing memories...';

  @override
  String get executingPlan => 'Executing plan...';

  @override
  String executingTool(String toolName) {
    return 'Executing tool: $toolName';
  }

  @override
  String processingToolResult(String toolName) {
    return 'Processing tool result: $toolName';
  }

  @override
  String get statusWarningAiErrorSummary => 'The AI made an error';

  @override
  String get statusWarningAiErrorDetailTitle => 'AI Error Reason';

  @override
  String get toolRunning => 'Tool running...';

  @override
  String toolRunningWithName(String toolName) {
    return '$toolName: Tool running...';
  }

  @override
  String toolStatusWithName(String toolName, String message) {
    return '$toolName: $message';
  }

  @override
  String get close => 'Close';

  @override
  String get create => 'Create';

  @override
  String get save => 'Save';

  @override
  String get delete => 'Delete';

  @override
  String get search => 'Search';

  @override
  String get loading => 'Loading';

  @override
  String get toolApprovalTitle => 'Tool permission request';

  @override
  String get toolApprovalToolLabel => 'Tool';

  @override
  String get toolApprovalActionLabel => 'Action';

  @override
  String get toolApprovalDeny => 'Deny';

  @override
  String get toolApprovalAllowOnce => 'Allow once';

  @override
  String get toolApprovalAlwaysAllow => 'Always allow in this session';

  @override
  String get createGroupTitle => 'New group';

  @override
  String get groupNameLabel => 'Group name';

  @override
  String get renameConversationTitle => 'Edit title';

  @override
  String get newTitleLabel => 'New title';

  @override
  String get deleteConversationTitle => 'Delete conversation?';

  @override
  String deleteConversationMessage(String title) {
    return 'Delete \"$title\"?';
  }

  @override
  String get chatHistory => 'Chat history';

  @override
  String get editTitle => 'Edit title';

  @override
  String get moveUp => 'Move up';

  @override
  String get moveDown => 'Move down';

  @override
  String get pin => 'Pin';

  @override
  String get unpin => 'Unpin';

  @override
  String get lock => 'Lock';

  @override
  String get unlock => 'Unlock';

  @override
  String get chatLockedCannotDelete =>
      'This chat is locked and cannot be deleted';

  @override
  String get messageLocatorTitle => 'Message locator';

  @override
  String messageLocatorCurrent(int current, int total) {
    return 'Current $current / $total';
  }

  @override
  String get messageLocatorSearchHint => 'Search message content';

  @override
  String get messageLocatorInstruction =>
      'Scroll the list or search to jump to a message';

  @override
  String messageLocatorResultCount(int count) {
    return '$count results';
  }

  @override
  String get messageLocatorNoMatches => 'No matching messages';

  @override
  String get messageSenderUser => 'User';

  @override
  String get messageSenderSummary => 'Summary';

  @override
  String get messageSenderSystem => 'System';

  @override
  String get messageSenderThinking => 'Thinking';

  @override
  String get thinkingProcess => 'Thinking Process';

  @override
  String thinkingToolsGroupTitleWithCount(int count) {
    return 'Thinking & Tool Calls ($count)';
  }

  @override
  String toolsGroupTitleWithCount(int count) {
    return 'Tool Calls ($count)';
  }

  @override
  String get messageSenderOther => 'Other';

  @override
  String get hiddenUserMessage => 'Hidden user message';

  @override
  String get workspaceSetupTitle => 'Set up workspace';

  @override
  String get workspaceSetupSubtitle =>
      'Provide a dedicated file environment for your AI projects';

  @override
  String get workspaceCreateDefaultTitle => 'Create default';

  @override
  String get workspaceCreateDefaultDescription =>
      'Create a new workspace in the app';

  @override
  String get workspaceBindExistingTitle => 'Choose existing';

  @override
  String get workspaceBindExistingDescription =>
      'Select a folder from this device';

  @override
  String get workspaceProjectTypeDialogTitle => 'Choose project type';

  @override
  String get workspaceProjectTypeDialogDescription =>
      'Choose the default workspace type to create';

  @override
  String get workspaceBindDialogTitle => 'Choose existing workspace';

  @override
  String get workspacePathLabel => 'Selected workspace';

  @override
  String get workspaceEnvLabel => 'Workspace environment';

  @override
  String get optionalHint => 'Optional';

  @override
  String get workspacePathRequired => 'Select a workspace folder';

  @override
  String get bind => 'Bind';

  @override
  String get workspaceProjectBlankTitle => 'Blank workspace';

  @override
  String get workspaceProjectBlankDescription =>
      'Create an empty workspace directory without template files';

  @override
  String get workspaceProjectOfficeTitle => 'Office documents';

  @override
  String get workspaceProjectOfficeDescription =>
      'For document editing, file processing, and general office tasks';

  @override
  String get workspaceProjectWebTitle => 'Web project';

  @override
  String get workspaceProjectWebDescription =>
      'For web development with HTML/CSS/JavaScript and an automatic local server';

  @override
  String get workspaceProjectAndroidTitle => 'Android project';

  @override
  String get workspaceProjectAndroidDescription =>
      'For Android engineering with common Gradle task shortcuts';

  @override
  String get workspaceProjectFlutterTitle => 'Flutter project';

  @override
  String get workspaceProjectFlutterDescription =>
      'For Flutter cross-platform development with a stable app template and common commands';

  @override
  String get workspaceProjectNodeTitle => 'Node.js project';

  @override
  String get workspaceProjectNodeDescription =>
      'For Node.js backend development with npm command shortcuts';

  @override
  String get workspaceProjectTypeScriptTitle => 'TypeScript project';

  @override
  String get workspaceProjectTypeScriptDescription =>
      'TypeScript + pnpm with type-safe development and tsc watch';

  @override
  String get workspaceProjectPythonTitle => 'Python project';

  @override
  String get workspaceProjectPythonDescription =>
      'For Python development with pip and an HTTP server';

  @override
  String get workspaceProjectJavaTitle => 'Java project';

  @override
  String get workspaceProjectJavaDescription =>
      'For Java development with Gradle and Maven builds';

  @override
  String get workspaceProjectGoTitle => 'Go project';

  @override
  String get workspaceProjectGoDescription =>
      'For Go development with go mod and build commands';

  @override
  String get version => 'Version';

  @override
  String get author => 'Author';

  @override
  String get entry => 'Entry';

  @override
  String get source => 'Source';

  @override
  String get category => 'Category';

  @override
  String get defaultStatus => 'Default status';

  @override
  String get builtIn => 'Built-in';

  @override
  String get external => 'External';

  @override
  String get enabledByDefault => 'Enabled by default';

  @override
  String get disabledByDefault => 'Disabled by default';

  @override
  String get toolPkgResources => 'ToolPkg resources';

  @override
  String resourcesCount(int count) {
    return 'Resources $count';
  }

  @override
  String uiModulesCount(int count) {
    return 'UI modules $count';
  }

  @override
  String navigationEntriesCount(int count) {
    return 'Navigation entries $count';
  }

  @override
  String desktopWidgetsCount(int count) {
    return 'Desktop widgets $count';
  }

  @override
  String workflowTemplatesCount(int count) {
    return 'Workflow templates $count';
  }

  @override
  String workspaceTemplatesCount(int count) {
    return 'Workspace templates $count';
  }

  @override
  String get pluginConfiguration => 'Plugin configuration';

  @override
  String get subpackages => 'Subpackages';

  @override
  String get toolPkgNoSubpackages => 'This ToolPkg declares no subpackages';

  @override
  String subpackageToolCount(String packageName, int count) {
    return '$packageName · $count tools';
  }

  @override
  String get workflowTemplates => 'Workflow templates';

  @override
  String get workspaceTemplates => 'Workspace templates';

  @override
  String get disable => 'Disable';

  @override
  String get enable => 'Enable';

  @override
  String get environmentVariables => 'Environment variables';

  @override
  String get required => 'Required';

  @override
  String get states => 'States';

  @override
  String stateToolSummary(String condition, int toolCount, int excludeCount) {
    return '$condition · $toolCount tools · excludes $excludeCount';
  }

  @override
  String get inherit => 'Inherit';

  @override
  String get tools => 'Tools';

  @override
  String get packageNoTools => 'This package declares no tools';

  @override
  String get permissionsTitle => 'Permissions';

  @override
  String get clear => 'Clear';

  @override
  String get noPermissionRecords => 'No permission records yet';

  @override
  String get allow => 'Allow';

  @override
  String get deny => 'Deny';

  @override
  String get camera => 'Camera';

  @override
  String get microphone => 'Microphone';

  @override
  String get protectedMedia => 'Protected media';

  @override
  String get midiDevice => 'MIDI device';

  @override
  String get browserPermissionRequestTitle => 'Website permission request';

  @override
  String chatSpeechInputFailed(Object error) {
    return 'Speech input failed: $error';
  }

  @override
  String get chatSpeechInputConfigurationRequired =>
      'Select a speech recognition configuration in Settings > Voice & Recognition before using speech input.';

  @override
  String get chatSpeechNoTextRecognized => 'No speech recognized.';

  @override
  String get history => 'History';

  @override
  String get bookmarks => 'Bookmarks';

  @override
  String get downloads => 'Downloads';

  @override
  String get scripts => 'Scripts';

  @override
  String get zoom => 'Zoom';

  @override
  String get zoomIn => 'Zoom in';

  @override
  String get zoomOut => 'Zoom out';

  @override
  String get desktopMode => 'Desktop mode';

  @override
  String get clearLocalStorage => 'Clear local storage';

  @override
  String get searchHistory => 'Search history';

  @override
  String get noDownloadTasks => 'No download tasks yet';

  @override
  String get openFile => 'Open file';

  @override
  String get openLocation => 'Open location';

  @override
  String get retry => 'Retry';

  @override
  String get removeRecord => 'Remove record';

  @override
  String get pending => 'Pending';

  @override
  String get completed => 'Completed';

  @override
  String get failed => 'Failed';

  @override
  String get back => 'Back';

  @override
  String get forward => 'Forward';

  @override
  String get stop => 'Stop';

  @override
  String get refresh => 'Refresh';

  @override
  String get home => 'Home';

  @override
  String get newTab => 'New tab';

  @override
  String get openExternalApplication => 'Open external application';

  @override
  String get open => 'Open';

  @override
  String get ok => 'OK';

  @override
  String get webPage => 'Web page';

  @override
  String get tabs => 'Tabs';

  @override
  String get noBookmarks => 'No bookmarks yet';

  @override
  String get removeBookmark => 'Remove bookmark';

  @override
  String get addBookmark => 'Add bookmark';

  @override
  String get menu => 'Menu';

  @override
  String get siteData => 'Site data';

  @override
  String get clearAllWebViewCookies => 'Clear all WebView cookies';

  @override
  String get clearCookies => 'Clear cookies';

  @override
  String get noData => 'No data';

  @override
  String get local => 'Local';

  @override
  String get pageLoadFailed => 'Page load failed';

  @override
  String get pause => 'Pause';

  @override
  String get resume => 'Resume';

  @override
  String get paused => 'Paused';

  @override
  String get cancelled => 'Cancelled';

  @override
  String get downloading => 'Downloading';

  @override
  String savedTo(String path) {
    return 'Saved to $path';
  }

  @override
  String get sslCertificateError => 'SSL certificate error';

  @override
  String get edit => 'Edit';

  @override
  String get files => 'Files';

  @override
  String get terminal => 'Terminal';

  @override
  String get browser => 'Browser';

  @override
  String get filePreview => 'File preview';

  @override
  String get workspaceBoundTitle => 'Bound workspace';

  @override
  String get selectFile => 'Select file';

  @override
  String get selectFileDescription =>
      'Select a file from the workspace to view, edit, or send to AI';

  @override
  String get openTerminal => 'Open terminal';

  @override
  String get openTerminalDescription =>
      'Enter the command line for the current workspace';

  @override
  String get openBrowser => 'Open browser';

  @override
  String get openBrowserDescription =>
      'Open a full browser session, project preview, and web automation';

  @override
  String get noWorkspaceBound => 'This conversation has no bound workspace.';

  @override
  String get terminalSessionPlaceholder =>
      'The current workspace terminal session will appear here.';

  @override
  String get emptyFolder => 'This folder is empty';

  @override
  String get imagePreview => 'Image preview';

  @override
  String get audioPreview => 'Audio preview';

  @override
  String get videoPreview => 'Video preview';

  @override
  String get pdfPreview => 'PDF preview';

  @override
  String get wordPreview => 'Word preview';

  @override
  String get spreadsheetPreview => 'Spreadsheet preview';

  @override
  String get presentationPreview => 'Presentation preview';

  @override
  String get webPagePreview => 'Web page preview';

  @override
  String get markdownPreview => 'Markdown preview';

  @override
  String get textPreview => 'Text preview';

  @override
  String get file => 'File';

  @override
  String get unsupportedReadOnlyPreview =>
      'This file is not a built-in read-only preview type.';

  @override
  String get cannotPreview => 'Cannot preview';

  @override
  String get openProjectInFullBrowser => 'Open project in full browser';

  @override
  String get openInBrowser => 'Open in browser';

  @override
  String get emptySpreadsheet => 'Spreadsheet is empty';

  @override
  String get settingsCategoryModelTitle => 'Models & AI';

  @override
  String get settingsCategoryModelSubtitle => 'Models, keys, context';

  @override
  String get settingsCategoryModelDescription =>
      'Configure model connections, choose the chat model, and manage thinking, context, and multimodal abilities.';

  @override
  String get settingsCategoryLocalModelsTitle => 'Local Models';

  @override
  String get settingsCategoryLocalModelsSubtitle =>
      'Downloads, engines, STT / TTS';

  @override
  String get settingsCategoryLocalModelsDescription =>
      'Manage local models and inference engines installed on demand.';

  @override
  String get settingsCategoryCharactersTitle => 'Characters & Memory';

  @override
  String get settingsCategoryCharactersSubtitle => 'Cards, groups, bindings';

  @override
  String get settingsCategoryCharactersDescription =>
      'Manage character cards, groups, active roles, and role-level model, memory, and tool bindings.';

  @override
  String get settingsCategoryToolsTitle => 'Tools & Permissions';

  @override
  String get settingsCategoryToolsSubtitle =>
      'AI capability, system authorization, extensions';

  @override
  String get settingsCategoryToolsDescription =>
      'Set AI read-only, read-write, or full access, and review the current device system authorization status.';

  @override
  String get settingsCategoryWorkspaceTitle => 'Workspace & Browser';

  @override
  String get settingsCategoryWorkspaceSubtitle => 'Files, terminal, browser';

  @override
  String get settingsCategoryWorkspaceDescription =>
      'Manage default workspaces, terminal sessions, browser mode, scripts, and web automation.';

  @override
  String get settingsCategoryGlobalBehaviorTitle => 'Global Behavior Settings';

  @override
  String get settingsCategoryGlobalBehaviorSubtitle =>
      'Input processing and interaction';

  @override
  String get settingsCategoryGlobalBehaviorDescription =>
      'Configure input and interaction behavior that does not vary by character card.';

  @override
  String get settingsCategoryAppearanceTitle => 'Appearance & Interaction';

  @override
  String get settingsCategoryAppearanceSubtitle => 'Theme and language';

  @override
  String get settingsCategoryAppearanceDescription =>
      'Adjust the client theme and current localization display.';

  @override
  String get settingsCategoryDataTitle => 'Data & Backup';

  @override
  String get settingsCategoryDataSubtitle => 'Backup, restore, stats';

  @override
  String get settingsCategoryDataDescription =>
      'Back up chats, characters, and model settings, restore backup content, and view data statistics.';

  @override
  String get settingsCategoryAccessLinksTitle => 'Devices & Access';

  @override
  String get settingsCategoryAccessLinksSubtitle => 'Connect, sync, access';

  @override
  String get settingsCategoryAccessLinksDescription =>
      'Connect another device, sync data, or allow browsers to access this device.';

  @override
  String get settingsCategoryGroupAssistant => 'AI & Creation';

  @override
  String get settingsCategoryGroupWorkspace => 'Workspace & Automation';

  @override
  String get settingsCategoryGroupExperience => 'Display & Interaction';

  @override
  String get settingsCategoryGroupSystem => 'Data & System';

  @override
  String get settingsGlobalBehaviorChatInputSection => 'Chat input';

  @override
  String get settingsGlobalBehaviorLongPastedTextAsAttachment =>
      'Convert long pasted text to a file';

  @override
  String get settingsGlobalBehaviorLongPastedTextThreshold =>
      'Conversion threshold';

  @override
  String settingsGlobalBehaviorLongPastedTextThresholdValue(int count) {
    return '$count characters';
  }

  @override
  String get settingsComingSoon =>
      'This area will continue connecting existing runtime capabilities. Models, characters, and tools are being completed first.';

  @override
  String get settingsAdvanced => 'Advanced settings';

  @override
  String get settingsActive => 'Active';

  @override
  String get settingsActivate => 'Activate';

  @override
  String get settingsModelCurrentSection => 'Current chat model';

  @override
  String get settingsModelCurrentChatModel => 'Chat uses';

  @override
  String get settingsModelCurrentActive => 'Active';

  @override
  String get settingsModelSetCurrentActive => 'Set active';

  @override
  String get settingsChatThinkingMode => 'Thinking mode';

  @override
  String get settingsChatThinkingModeDescription =>
      'Let supported models produce steadier reasoning.';

  @override
  String get settingsChatStreamOutput => 'Stream output';

  @override
  String get settingsChatStreamOutputDescription =>
      'Show generated replies progressively.';

  @override
  String get agentModelSelectorThinkingSettings => 'Thinking settings';

  @override
  String get agentModelSelectorThinkingSettingsDescription =>
      'Configure reasoning behavior for the current chat model.';

  @override
  String get agentModelSelectorThinkingQuality => 'Reasoning depth';

  @override
  String get agentModelSelectorThinkingQualityDescription =>
      'Available only in thinking mode. Higher levels reason more deeply; level 1 is automatic.';

  @override
  String agentModelSelectorMaxModeDescription(
    String enabledLength,
    String disabledLength,
  ) {
    return 'Max context mode uses a ${enabledLength}k context window when enabled and ${disabledLength}k when disabled.';
  }

  @override
  String get agentModelSelectorModelConfiguration => 'Model configuration';

  @override
  String get agentModelSelectorModelConfigurationDescription =>
      'Choose a configured model here, or use Manage configuration below to add or edit one.';

  @override
  String get agentModelSelectorModel => 'Model';

  @override
  String get agentModelSelectorNoModels => 'No models available';

  @override
  String get agentModelSelectorManageConfiguration => 'Manage configuration';

  @override
  String agentModelSelectorModelCount(int count) {
    return '$count models';
  }

  @override
  String get agentModelSelectorOn => 'On';

  @override
  String get agentModelSelectorOff => 'Off';

  @override
  String get settingsModelProfilesSection => 'Model profiles';

  @override
  String get settingsModelFunctionMappingsSection =>
      'Function model assignment';

  @override
  String get settingsModelFunctionMappingsDescription =>
      'Choose the model profile and concrete model used by chat, summary, memory, image recognition, and other functions.';

  @override
  String get settingsModelFunctionMappingsReset => 'Reset all';

  @override
  String get settingsModelFunctionMappingsChange => 'Change';

  @override
  String settingsModelFunctionMappingsSelect(String name) {
    return 'Select $name model';
  }

  @override
  String settingsModelFunctionMappingsCurrent(
    String configName,
    String modelName,
  ) {
    return '$configName · $modelName';
  }

  @override
  String settingsModelFunctionMappingsMissing(
    String providerId,
    String modelId,
  ) {
    return 'Bound model does not exist: $providerId · $modelId';
  }

  @override
  String settingsModelDeleteBlocked(String functions) {
    return 'This model is used by these functions. Change their model assignments first: $functions';
  }

  @override
  String settingsModelDeleteProviderBlocked(String functions) {
    return 'Models under this provider are used by these functions. Change their model assignments first: $functions';
  }

  @override
  String settingsModelDeleteProviderConfirm(String name, int count) {
    return 'Delete provider “$name”? This will also delete its $count models.';
  }

  @override
  String get settingsModelDeleteProviderConfirmAction => 'Delete provider';

  @override
  String get settingsTtsDeleteProvider => 'Delete TTS provider';

  @override
  String settingsTtsDeleteProviderConfirm(String name, int count) {
    return 'Delete TTS provider “$name” and its $count voice configurations?';
  }

  @override
  String settingsTtsDeleteProviderFailed(String error) {
    return 'Failed to delete TTS provider: $error';
  }

  @override
  String get settingsTtsCurrentConfigCannotDelete =>
      'The TTS configuration currently in use cannot be deleted.';

  @override
  String get settingsTtsConfigUsedByCharacter =>
      'This TTS configuration is used by a character card and cannot be deleted.';

  @override
  String get settingsModelChatAutoGlmWarning =>
      'AutoGLM cannot be used as the main chat model. Chat and UI control use separate model assignments; choose another large model.';

  @override
  String get settingsModelFunctionChat => 'Chat';

  @override
  String get settingsModelFunctionChatDescription =>
      'Model used for main conversation replies.';

  @override
  String get settingsModelFunctionSummary => 'Summary';

  @override
  String get settingsModelFunctionSummaryDescription =>
      'Model used for long-context automatic summaries.';

  @override
  String get settingsModelFunctionTitleGeneration => 'Conversation title';

  @override
  String get settingsModelFunctionTitleGenerationDescription =>
      'Model used to summarize the first message and attachments into a conversation title.';

  @override
  String get settingsModelFunctionMemory => 'Memory';

  @override
  String get settingsModelFunctionMemoryDescription =>
      'Model used to extract, organize, and update memories.';

  @override
  String get settingsModelFunctionUiController => 'UI control';

  @override
  String get settingsModelFunctionUiControllerDescription =>
      'Model used for interface control and lightweight action planning.';

  @override
  String get settingsModelFunctionTranslation => 'Translation';

  @override
  String get settingsModelFunctionTranslationDescription =>
      'Model used to translate text and localized content.';

  @override
  String get settingsModelFunctionGrep => 'Text search';

  @override
  String get settingsModelFunctionGrepDescription =>
      'Model used to filter search results and judge text matches.';

  @override
  String get settingsModelFunctionRoleResponsePlanner => 'Group reply planner';

  @override
  String get settingsModelFunctionRoleResponsePlannerDescription =>
      'Model used to plan speaking roles and order in group conversations.';

  @override
  String get settingsModelFunctionImageRecognition => 'Image recognition';

  @override
  String get settingsModelFunctionImageRecognitionDescription =>
      'Model used to understand images and extract image content.';

  @override
  String get settingsModelFunctionAudioRecognition => 'Audio recognition';

  @override
  String get settingsModelFunctionAudioRecognitionDescription =>
      'Model used to understand audio and extract audio content.';

  @override
  String get settingsModelFunctionVideoRecognition => 'Video recognition';

  @override
  String get settingsModelFunctionVideoRecognitionDescription =>
      'Model used to understand video and extract video content.';

  @override
  String get settingsModelFunctionImageUnsupported =>
      'The selected model profile has direct image input disabled.';

  @override
  String get settingsModelFunctionAudioUnsupported =>
      'The selected model profile has direct audio input disabled.';

  @override
  String get settingsModelFunctionVideoUnsupported =>
      'The selected model profile has direct video input disabled.';

  @override
  String get settingsModelCreateProfile => 'New model profile';

  @override
  String get settingsModelEditProfile => 'Edit model profile';

  @override
  String get settingsModelProfileName => 'Profile name';

  @override
  String get settingsModelApiEndpoint => 'API endpoint';

  @override
  String get settingsModelModelNames => 'Model names';

  @override
  String get settingsModelApiKey => 'API key';

  @override
  String get settingsModelProviderTypeOpenaiCodex => 'ChatGPT Codex';

  @override
  String get settingsModelCodexTitle => 'ChatGPT / Codex sign-in';

  @override
  String get settingsModelCodexDescription =>
      'Use Codex included with your ChatGPT plan. No API key or relay server is required.';

  @override
  String get settingsModelCodexSignIn => 'Sign in with ChatGPT';

  @override
  String get settingsModelCodexConnected => 'Signed in';

  @override
  String get settingsModelCodexSignedOut => 'Signed out';

  @override
  String get settingsModelCodexWaiting => 'Waiting for approval';

  @override
  String get settingsModelCodexExpired => 'Code expired';

  @override
  String get settingsModelCodexChecking => 'Checking';

  @override
  String get settingsModelCodexCodeHelp =>
      'Enter this one-time code on the ChatGPT page. It expires in 15 minutes.';

  @override
  String get settingsModelCodexOpenBrowser => 'Open ChatGPT page';

  @override
  String get settingsModelCodexOpenFailed =>
      'Could not open the ChatGPT page. Try the button again.';

  @override
  String get settingsModelCodexLogout => 'Sign out';

  @override
  String get settingsModelCodexRequired =>
      'Sign in with ChatGPT before saving this connection.';

  @override
  String get settingsModelCodexPlan => 'Plan';

  @override
  String get settingsModelApiKeyPool => 'API key pool';

  @override
  String get settingsModelApiKeyPoolDescription =>
      'Prepare multiple keys for one model profile so runtime can rotate between them.';

  @override
  String settingsModelApiKeyPoolCount(int count) {
    return '$count keys';
  }

  @override
  String get settingsModelApiKeyPoolEmpty =>
      'No keys yet. Add keys and this profile will use the key pool.';

  @override
  String get settingsModelAddApiKey => 'Add key';

  @override
  String get settingsModelEditApiKey => 'Edit key';

  @override
  String get settingsModelApiKeyName => 'Key name';

  @override
  String get settingsModelApiKeyEnabled => 'Enable this key';

  @override
  String get settingsModelProviderId => 'Provider ID';

  @override
  String get settingsModelProvidersSection => 'Providers';

  @override
  String get settingsModelProviderType => 'Provider type';

  @override
  String settingsModelProviderTypeOption(String name, String original) {
    return '$name ($original)';
  }

  @override
  String get settingsModelProviderTypeOpenai => 'OpenAI';

  @override
  String get settingsModelProviderTypeOpenaiResponses => 'OpenAI Responses';

  @override
  String get settingsModelProviderTypeOpenaiResponsesGeneric =>
      'OpenAI Responses compatible';

  @override
  String get settingsModelProviderTypeOpenaiGeneric => 'OpenAI compatible';

  @override
  String get settingsModelProviderTypeAnthropic => 'Anthropic';

  @override
  String get settingsModelProviderTypeAnthropicGeneric =>
      'Anthropic compatible';

  @override
  String get settingsModelProviderTypeGoogle => 'Google Gemini';

  @override
  String get settingsModelProviderTypeGeminiGeneric => 'Gemini compatible';

  @override
  String get settingsModelProviderTypeBaidu => 'Baidu';

  @override
  String get settingsModelProviderTypeAliyun => 'Aliyun';

  @override
  String get settingsModelProviderTypeXunfei => 'Xunfei';

  @override
  String get settingsModelProviderTypeZhipu => 'Zhipu AI';

  @override
  String get settingsModelProviderTypeBaichuan => 'Baichuan';

  @override
  String get settingsModelProviderTypeMoonshot => 'Moonshot';

  @override
  String get settingsModelProviderTypeMimo => 'MiMo';

  @override
  String get settingsModelProviderTypeDeepseek => 'DeepSeek';

  @override
  String get settingsModelProviderTypeMistral => 'Mistral';

  @override
  String get settingsModelProviderTypeSiliconflow => 'SiliconFlow';

  @override
  String get settingsModelProviderTypeIflow => 'iFlow';

  @override
  String get settingsModelProviderTypeOpenrouter => 'OpenRouter';

  @override
  String get settingsModelProviderTypeFourRouter => '4Router';

  @override
  String get settingsModelProviderTypeNousPortal => 'Nous Portal';

  @override
  String get settingsModelProviderTypeInfiniai => 'InfiniAI';

  @override
  String get settingsModelProviderTypeAlipayBailing => 'Alipay Bailing';

  @override
  String get settingsModelProviderTypeDoubao => 'Doubao';

  @override
  String get settingsModelProviderTypeNvidia => 'NVIDIA';

  @override
  String get settingsModelProviderTypeLmstudio => 'LM Studio';

  @override
  String get settingsModelProviderTypeOllama => 'Ollama';

  @override
  String get settingsModelProviderTypeOpenaiLocal => 'OpenAI Local';

  @override
  String get settingsModelProviderTypeLocalModel => 'Local model';

  @override
  String localModelsLoadFailed(Object error) {
    return 'Failed to load local model status: $error';
  }

  @override
  String localModelsOperationFailed(Object error) {
    return 'Local model operation failed: $error';
  }

  @override
  String get localModelsCatalog => 'Model catalog';

  @override
  String get localModelsCategorySpeechToText => 'Speech-to-text models';

  @override
  String get localModelsCategoryTextToSpeech => 'Text-to-speech models';

  @override
  String get localModelsCategoryChat => 'LLM models';

  @override
  String get localModelsCategoryEmbedding => 'Embedding models';

  @override
  String get localModelsInstalledEngines => 'Installed engines';

  @override
  String get localModelsNoInstalledEngines =>
      'No local inference engine is installed on this platform.';

  @override
  String get localModelsDeleteModelTitle => 'Delete local model';

  @override
  String localModelsDeleteModelMessage(Object modelName) {
    return 'Delete model files for $modelName?';
  }

  @override
  String get localModelsDeleteEngineTitle => 'Delete local engine';

  @override
  String localModelsDeleteEngineMessage(Object engineName, Object version) {
    return 'Delete $engineName $version?';
  }

  @override
  String get localModelsCancelling => 'Pausing';

  @override
  String localModelsDownloadPaused(Object downloaded, Object total) {
    return 'Paused: $downloaded / $total';
  }

  @override
  String get localModelsDownloadInstalling => 'Download complete, installing';

  @override
  String localModelsDownloading(Object downloaded, Object total) {
    return 'Downloading: $downloaded / $total';
  }

  @override
  String localModelsLicense(Object license) {
    return 'License: $license';
  }

  @override
  String get localModelsPlatformCompatible => 'Platform compatible';

  @override
  String get localModelsPlatformIncompatible => 'Platform incompatible';

  @override
  String get localModelsModelInstalled => 'Model installed';

  @override
  String get localModelsModelNotInstalled => 'Model not installed';

  @override
  String get localModelsEngineInstalled => 'Engine installed';

  @override
  String get localModelsEngineNotInstalled => 'Engine not installed';

  @override
  String get localModelsVerifyModelAndEngine => 'Verify model and engine';

  @override
  String get localModelsDeleteModel => 'Delete model';

  @override
  String get localModelsPauseDownload => 'Pause download';

  @override
  String get localModelsDeleteDownload => 'Delete download';

  @override
  String get localModelsResumeDownload => 'Resume';

  @override
  String get localModelsInstalling => 'Installing';

  @override
  String get localModelsInstall => 'Install';

  @override
  String get localModelsDeleteEngine => 'Delete engine';

  @override
  String get localModelDescriptionSherpaOnnxStreamingStt =>
      'Streaming bilingual Chinese and English speech recognition.';

  @override
  String get localModelDescriptionSherpaOnnxVitsAishell3 =>
      'Local Chinese multi-speaker speech synthesis.';

  @override
  String get localModelDescriptionSherpaOnnxVitsZhLl =>
      'Local Chinese five-speaker speech synthesis.';

  @override
  String get localModelDescriptionSherpaOnnxMatchaBaker =>
      'Local Chinese single-speaker Matcha speech synthesis.';

  @override
  String get localModelDescriptionSherpaOnnxKittenNano =>
      'Local English eight-speaker KittenTTS speech synthesis.';

  @override
  String get localModelDescriptionSherpaOnnxWebParaformer =>
      'Browser packaged Chinese and English Paraformer speech recognition.';

  @override
  String get localModelDescriptionSherpaOnnxWebVitsPiper =>
      'Browser packaged English multi-speaker VITS speech synthesis.';

  @override
  String get settingsModelProviderTypeMnn => 'MNN';

  @override
  String get settingsModelProviderTypeLlamaCpp => 'llama.cpp';

  @override
  String get settingsModelProviderTypePpinfra => 'PPInfra';

  @override
  String get settingsModelProviderTypeNovita => 'Novita AI';

  @override
  String get settingsModelProviderTypeOther => 'Other';

  @override
  String get settingsModelEditModelSettings => 'Model settings';

  @override
  String get settingsModelCreateProvider => 'Create provider';

  @override
  String get settingsModelEditProvider => 'Edit provider';

  @override
  String get settingsModelAddModel => 'Add model';

  @override
  String get settingsModelAddModelShort => 'Add';

  @override
  String get settingsModelCustomModel => 'Custom model';

  @override
  String get settingsModelModelId => 'Model ID';

  @override
  String get settingsModelDuplicateModelId =>
      'This model has already been added to this provider.';

  @override
  String get settingsModelMaxTokens => 'Max tokens';

  @override
  String get settingsModelMaxTokensDescription =>
      'Limit how many tokens one response may generate.';

  @override
  String get settingsModelTemperature => 'Temperature';

  @override
  String get settingsModelTemperatureDescription =>
      'Controls randomness. Lower is steadier, higher is more varied.';

  @override
  String get settingsModelTopP => 'Top-p';

  @override
  String get settingsModelTopPDescription =>
      'Sample only from the cumulative Top-p probability range.';

  @override
  String get settingsModelTopK => 'Top-k';

  @override
  String get settingsModelTopKDescription =>
      'Sample from the K most likely candidate tokens. 0 disables it.';

  @override
  String get settingsModelPresencePenalty => 'Presence penalty';

  @override
  String get settingsModelPresencePenaltyDescription =>
      'Encourages new topics and reduces reuse of existing content.';

  @override
  String get settingsModelFrequencyPenalty => 'Frequency penalty';

  @override
  String get settingsModelFrequencyPenaltyDescription =>
      'Penalizes repeated tokens by frequency.';

  @override
  String get settingsModelRepetitionPenalty => 'Repetition penalty';

  @override
  String get settingsModelRepetitionPenaltyDescription =>
      'Further reduces repeated output. 1.0 means no penalty.';

  @override
  String get settingsModelRequestLimit => 'Requests per minute';

  @override
  String get settingsModelMaxConcurrent => 'Max concurrent requests';

  @override
  String get settingsModelContextLength => 'Context length';

  @override
  String get settingsModelMaxContextLength => 'Max context length';

  @override
  String get settingsModelMaxContextLengthInvalid =>
      'Enter a max context length greater than 0';

  @override
  String get settingsModelMaxContextMode => 'Max context mode';

  @override
  String get settingsModelSummaryThreshold => 'Summary token threshold';

  @override
  String get settingsModelSummaryByMessageCount => 'Summarize by message count';

  @override
  String get settingsModelSummaryMessageCount => 'Summary message threshold';

  @override
  String get settingsModelCustomHeaders => 'Custom headers';

  @override
  String get settingsModelCustomParameters => 'Custom parameters JSON';

  @override
  String get settingsModelToolCall => 'Tool calling';

  @override
  String get settingsModelToolCallDescription =>
      'Allow the model to use structured tool calls.';

  @override
  String get settingsModelDirectImage => 'Direct image input';

  @override
  String get settingsModelDirectImageDescription =>
      'Send images directly to models that support image input.';

  @override
  String get settingsModelDirectAudio => 'Direct audio input';

  @override
  String get settingsModelDirectAudioDescription =>
      'Send audio directly to models that support audio input.';

  @override
  String get settingsModelDirectVideo => 'Direct video input';

  @override
  String get settingsModelDirectVideoDescription =>
      'Send video directly to models that support video input.';

  @override
  String get settingsModelGoogleSearch => 'Google Search';

  @override
  String get settingsModelGoogleSearchDescription =>
      'Enable provider-side search capability.';

  @override
  String get settingsModelContext => 'Context window';

  @override
  String get settingsModelSummary => 'Auto summary';

  @override
  String get settingsModelMediaHistory => 'Media history';

  @override
  String get settingsModelCapabilities => 'Capabilities';

  @override
  String get settingsModelBuiltinTools => 'Built-in tools';

  @override
  String get settingsModelBuiltinToolExclusive =>
      'Turns off external tool calling when enabled';

  @override
  String get settingsModelConnectionTestSection => 'Connection test';

  @override
  String get settingsModelRunConnectionTest => 'Test current model';

  @override
  String get settingsModelTestModel => 'Test model';

  @override
  String get settingsModelTestingConnection =>
      'Testing current model connection…';

  @override
  String get settingsModelTestedModel => 'Tested model';

  @override
  String get settingsModelConnectionTestPassed => 'All checks passed';

  @override
  String get settingsModelConnectionTestFailed => 'Some checks failed';

  @override
  String get settingsModelCapabilitiesApplied =>
      'Model capability switches were updated from the test result.';

  @override
  String get settingsModelCapabilitiesNeedChat =>
      'Chat test did not pass, so model capability switches were not updated.';

  @override
  String settingsModelConnectionTestError(String error) {
    return 'Connection test failed: $error';
  }

  @override
  String get settingsModelTestItemChat => 'Chat';

  @override
  String get settingsModelTestItemToolCall => 'Tool call';

  @override
  String get settingsModelTestItemImage => 'Image';

  @override
  String get settingsModelTestItemAudio => 'Audio';

  @override
  String get settingsModelTestItemVideo => 'Video';

  @override
  String get settingsModelTestItemUnknown => 'Unknown item';

  @override
  String get settingsCharactersCreateCard => 'New character card';

  @override
  String get settingsCharactersEditCard => 'Edit character card';

  @override
  String get settingsCharactersCardName => 'Character name';

  @override
  String get settingsCharactersCreateGroup => 'New group';

  @override
  String get settingsCharactersEditGroup => 'Edit group';

  @override
  String get settingsCharactersGroupName => 'Group name';

  @override
  String get settingsCharactersDescription => 'Description';

  @override
  String get settingsCharactersCharacterSetting => 'Character setting';

  @override
  String get settingsCharactersOpeningStatement => 'Opening statement';

  @override
  String get settingsCharactersOtherContentChat => 'Extra chat content';

  @override
  String get settingsCharactersOtherContentVoice => 'Extra voice content';

  @override
  String get settingsCharactersAdvancedPrompt => 'Advanced custom prompt';

  @override
  String get settingsCharactersMarks => 'Notes';

  @override
  String get settingsCharactersTags => 'Tags';

  @override
  String get settingsCharactersNoTags =>
      'No tags available. Create one in tag management, then bind it to this character card.';

  @override
  String get settingsCharactersImport => 'Import';

  @override
  String get settingsCharactersExport => 'Export';

  @override
  String get settingsCharactersImportJson => 'Import JSON';

  @override
  String get settingsCharactersCopyJson => 'Copy JSON';

  @override
  String get settingsCharactersImportTavernJson => 'Import Tavern JSON';

  @override
  String get settingsCharactersCopyTavernJson => 'Copy Tavern JSON';

  @override
  String get settingsCharactersJsonInput => 'JSON content';

  @override
  String get settingsCharactersTavernJsonInput => 'Tavern JSON content';

  @override
  String settingsCharactersJsonCopied(String name) {
    return 'Copied JSON for “$name”.';
  }

  @override
  String settingsCharactersTavernJsonCopied(String name) {
    return 'Copied Tavern JSON for “$name”.';
  }

  @override
  String get settingsCharactersImportCardJson => 'Import character card JSON';

  @override
  String get settingsCharactersImportCardJsonDone => 'Character card imported.';

  @override
  String get settingsCharactersImportTavernJsonDone =>
      'Tavern character card imported.';

  @override
  String get settingsCharactersImportGroupJson => 'Import group JSON';

  @override
  String get settingsCharactersImportGroupJsonDone => 'Group imported.';

  @override
  String settingsCharactersImportJsonError(String error) {
    return 'JSON import failed: $error';
  }

  @override
  String settingsCharactersImportTavernJsonError(String error) {
    return 'Tavern JSON import failed: $error';
  }

  @override
  String settingsCharactersTavernJsonCopyError(String error) {
    return 'Tavern JSON copy failed: $error';
  }

  @override
  String get settingsCharactersTagsSection => 'Tags';

  @override
  String get settingsCharactersManageTags => 'Manage tags';

  @override
  String get settingsCharactersCreateTag => 'New tag';

  @override
  String get settingsCharactersEditTag => 'Edit tag';

  @override
  String get settingsCharactersDeleteTag => 'Delete tag';

  @override
  String settingsCharactersDeleteTagMessage(String name) {
    return 'Delete “$name”?';
  }

  @override
  String get settingsCharactersTagName => 'Tag name';

  @override
  String get settingsCharactersTagDescription => 'Tag description';

  @override
  String get settingsCharactersTagPromptContent => 'Prompt content';

  @override
  String get settingsCharactersChatModelBindingMode =>
      'Chat model binding mode';

  @override
  String get settingsCharactersChatModelConfigId => 'Chat model config ID';

  @override
  String get settingsCharactersChatModelIndex => 'Chat model index';

  @override
  String get settingsCharactersToolAccess => 'Tool permission mode';

  @override
  String get settingsCharactersChatModelFollowGlobal => 'Follow global model';

  @override
  String get settingsCharactersChatModelFixedConfig => 'Use fixed model config';

  @override
  String get settingsCharactersChatModelConfig => 'Model config';

  @override
  String get settingsCharactersToolAccessFollowGlobal =>
      'Follow global tool permissions';

  @override
  String get settingsCharactersToolAccessCustom =>
      'Custom character tool permissions';

  @override
  String get settingsCharactersToolAccessEmpty =>
      'Enabled with no selected tools';

  @override
  String settingsCharactersToolAccessSummaryCounts(
    int builtinCount,
    int packageCount,
    int skillCount,
    int mcpCount,
  ) {
    return 'Built-in $builtinCount · packages $packageCount · skills $skillCount · MCP $mcpCount';
  }

  @override
  String get settingsCharactersToolAccessConfigure =>
      'Configure tool allowlist';

  @override
  String get settingsCharactersToolAccessTitle => 'Custom Allowed Tools';

  @override
  String get settingsCharactersToolAccessTabBuiltin => 'Built-ins';

  @override
  String get settingsCharactersToolAccessTabPackage => 'Packages';

  @override
  String get settingsCharactersToolAccessTabSkill => 'Skill';

  @override
  String get settingsCharactersToolAccessTabMcp => 'MCP';

  @override
  String get settingsCharactersToolAccessSearchPlaceholder =>
      'Search name, description, or ID';

  @override
  String get settingsCharactersToolAccessEmptySearch =>
      'No matching tools found';

  @override
  String get settingsCharactersToolAccessRequiresUsePackage =>
      'Selecting packages, skills, or MCP also requires allowing the built-in use_package tool.';

  @override
  String get settingsCharactersToolAccessEmptyBuiltin =>
      'No built-in tools are available for configuration';

  @override
  String get settingsCharactersToolAccessEmptyPackages =>
      'No globally available packages right now';

  @override
  String get settingsCharactersToolAccessEmptySkills =>
      'No AI-visible skills are available right now';

  @override
  String get settingsCharactersToolAccessEmptyMcp =>
      'No enabled MCP servers are available right now';

  @override
  String get settingsCharactersBuiltinTools => 'Allowed built-in tools';

  @override
  String get settingsCharactersAllowedPackages => 'Allowed packages';

  @override
  String get settingsCharactersAllowedSkills => 'Allowed skills';

  @override
  String get settingsCharactersAllowedMcpServers => 'Allowed MCP servers';

  @override
  String get settingsCharactersGroupMembersTitle => 'Group characters';

  @override
  String get settingsCharactersOpenMemoryGraph => 'View memory graph';

  @override
  String settingsCharactersMemoryGraphTitle(String profileName) {
    return '$profileName\'s memory graph';
  }

  @override
  String get settingsCharactersMemoryGraphEmpty => 'No memory nodes yet';

  @override
  String settingsCharactersMemoryGraphStats(int nodes, int edges) {
    return '$nodes nodes · $edges links';
  }

  @override
  String get settingsCharactersMemoryGraphLink => 'Memory link';

  @override
  String get settingsCharactersEditUserMarkdown => 'Edit user profile';

  @override
  String settingsCharactersUserMarkdownTitle(String profileName) {
    return '$profileName\'s user profile';
  }

  @override
  String get settingsCharactersUserMarkdownSaved => 'User profile saved';

  @override
  String get settingsCharactersUserMarkdownContent => 'User profile content';

  @override
  String get settingsCharactersMemoryAutoUpdate => 'Auto-update memory stores';

  @override
  String get settingsCharactersMemoryAutoUpdateDescription =>
      'Allow AI to organize conversation info into memory stores.';

  @override
  String get settingsCharactersPreferenceDescription =>
      'Provide user profile to model';

  @override
  String get settingsCharactersPreferenceDescriptionSubtitle =>
      'Include the current user profile in chat prompts.';

  @override
  String get settingsCharactersCardsSection => 'Character cards';

  @override
  String get settingsCharactersGroupsSection => 'Groups';

  @override
  String settingsCharactersGroupMembers(int count) {
    return '$count members';
  }

  @override
  String get settingsToolsPermissionMode => 'AI capability mode';

  @override
  String get settingsToolsAsk => 'Ask';

  @override
  String get settingsToolsExtensions => 'Extension management';

  @override
  String get settingsToolsPlugins => 'Plugins';

  @override
  String get settingsToolsPluginsDescription =>
      'Manage ToolPkg plugin containers and UI extensions.';

  @override
  String get settingsToolsPackages => 'Tool packages';

  @override
  String get settingsToolsPackagesDescription =>
      'Enable, disable, and inspect built-in or external tool packages.';

  @override
  String get settingsToolsSkills => 'Skills';

  @override
  String get settingsToolsSkillsDescription =>
      'Manage skill package visibility and imports.';

  @override
  String get settingsToolsMcp => 'MCP servers';

  @override
  String settingsToolsMcpDescription(int seconds) {
    return 'Manage MCP configuration. Startup wait is $seconds seconds.';
  }

  @override
  String get settingsToolsOverrides => 'Tool records';

  @override
  String get settingsToolsToolGroups => 'Registered tools';

  @override
  String get settingsToolsToolGroupsDescription =>
      'Tools registered by the current runtime for AI use.';

  @override
  String get settingsToolsAlwaysAllow => 'Allowed in this session';

  @override
  String get settingsToolsAlwaysAllowDescription =>
      'These tools were approved for the current session.';

  @override
  String get settingsToolsAlwaysForbid => 'Always forbid';

  @override
  String get settingsToolsAlwaysForbidDescription =>
      'AI will not call these tools.';

  @override
  String get settingsToolsAddTool => 'Add tool';

  @override
  String get settingsToolsAddAllowTool => 'Add allowed tool';

  @override
  String get settingsToolsAddForbidTool => 'Add forbidden tool';

  @override
  String get settingsToolsSearchTools => 'Search tools';

  @override
  String get settingsToolsNoToolsInGroup => 'No tools in this group.';

  @override
  String get settingsToolsMcpStartupTimeout => 'MCP startup timeout';

  @override
  String get settingsToolsMcpStartupTimeoutSeconds => 'Wait seconds';

  @override
  String get settingsToolsToolPkgPreHookTimeout => 'ToolPkg pre-hook timeout';

  @override
  String settingsToolsToolPkgPreHookDescription(int seconds) {
    return 'One ToolPkg pre-hook chain has $seconds seconds in total.';
  }

  @override
  String get settingsToolsToolPkgPreHookTimeoutSeconds => 'Total seconds';

  @override
  String get settingsWorkspaceCurrentDesign => 'Current workspace structure';

  @override
  String get settingsWorkspaceCurrentDesignDescription =>
      'Workspaces are bound to chats. Terminal sessions and browser sessions are global sessions shown flat inside the workspace.';

  @override
  String get settingsWorkspaceOpenChat => 'Return to chat workspace';

  @override
  String get settingsWorkspaceOpenChatDescription =>
      'Open files, terminals, browser, and web automation on the right side of chat.';

  @override
  String get settingsWorkspaceContains => 'Workspace contains';

  @override
  String get settingsWorkspacePerChat => 'Bound per chat';

  @override
  String get settingsWorkspaceGlobalSessions => 'Global terminal sessions';

  @override
  String get settingsWorkspaceBrowserSessions =>
      'Browser and WebVisit sessions';

  @override
  String get settingsWorkspaceBoundOverview => 'Workspace binding overview';

  @override
  String get settingsWorkspaceBoundOverviewDescription =>
      'Workspace paths recorded by chat histories are used as the binding source.';

  @override
  String get settingsWorkspaceBoundChats => 'Bound chats';

  @override
  String get settingsWorkspaceInternalRoot => 'Internal workspace root';

  @override
  String get settingsWorkspaceExternalRoot => 'Legacy external workspace root';

  @override
  String get settingsWorkspaceUnboundTitle => 'Unbound workspaces';

  @override
  String get settingsWorkspaceUnboundSubtitle =>
      'These workspace folders are not used by any chat.';

  @override
  String get settingsWorkspaceNoUnbound => 'No unbound workspaces.';

  @override
  String settingsWorkspaceSelectedCount(int selected, int total) {
    return 'Selected $selected / $total';
  }

  @override
  String get settingsWorkspaceSelectAllCurrentList => 'Select all';

  @override
  String get settingsWorkspaceClearAll => 'Clear';

  @override
  String get settingsWorkspaceInternalStorage => 'Internal storage';

  @override
  String get settingsWorkspaceExternalStorage => 'External storage';

  @override
  String get settingsWorkspaceNotUsedByAnyChat => 'Not used by any chat';

  @override
  String settingsWorkspaceDeleteSelected(int count) {
    return 'Delete selected workspaces ($count)';
  }

  @override
  String get settingsWorkspaceConfirmDeleteTitle => 'Confirm delete';

  @override
  String settingsWorkspaceDeleteConfirmation(int count) {
    return 'Delete $count selected workspace folders?';
  }

  @override
  String settingsWorkspaceDeleted(int count) {
    return 'Deleted $count unbound workspaces.';
  }

  @override
  String settingsWorkspaceDeleteFailed(String error) {
    return 'Delete failed: $error';
  }

  @override
  String settingsWorkspaceLoadFailed(String error) {
    return 'Failed to load workspaces: $error';
  }

  @override
  String get settingsWorkspaceRefresh => 'Refresh';

  @override
  String get runtimeIdentity => 'Current identity';

  @override
  String get runtimeIdentityManage => 'Switch or manage identities';

  @override
  String get runtimeIdentitySheetTitle => 'Identities';

  @override
  String get runtimeIdentityCreate => 'New identity';

  @override
  String get runtimeIdentityCreateTitle => 'New identity';

  @override
  String get runtimeIdentityRename => 'Rename identity';

  @override
  String get runtimeIdentityRenameTitle => 'Rename identity';

  @override
  String get runtimeIdentityName => 'Name (optional)';

  @override
  String runtimeIdentitySwitchTitle(String identityName) {
    return 'Switch to $identityName?';
  }

  @override
  String get runtimeIdentitySwitchDescription =>
      'Each identity has separate chats, settings, device space, paired devices, and workspaces. Switching ends the current runtime; the selected identity is used the next time the app starts.';

  @override
  String get runtimeIdentitySwitchConfirm => 'Switch identity';

  @override
  String get runtimeIdentityCurrent => 'Current';

  @override
  String get settingsUserProfileTitle => 'User profile';

  @override
  String get settingsUserProfileSubtitle =>
      'Avatar, name, identities, and GitHub';

  @override
  String get settingsUserProfileDescription =>
      'Manage this profile and switch isolated identities.';

  @override
  String get settingsUserProfileUnnamed => 'Unnamed';

  @override
  String get settingsUserProfileNotLoggedIn => 'Not logged in';

  @override
  String get settingsUserProfileGitHubLoading => 'Loading GitHub account...';

  @override
  String settingsUserProfileGitHubAccount(String account) {
    return 'GitHub: @$account';
  }

  @override
  String settingsUserProfileGitHubStatusError(String error) {
    return 'GitHub status error: $error';
  }

  @override
  String get settingsUserProfileOverview => 'Profile';

  @override
  String get settingsUserProfileAvatar => 'Avatar';

  @override
  String get settingsUserProfileName => 'Name';

  @override
  String get settingsUserProfileChooseAvatar => 'Choose avatar';

  @override
  String get settingsUserProfileClearAvatar => 'Clear avatar';

  @override
  String get settingsUserProfileEditName => 'Edit name';

  @override
  String get settingsUserProfileIdentities => 'Identities';

  @override
  String get settingsUserProfileGitHub => 'GitHub account';

  @override
  String get settingsUserProfileGitHubDescription => 'Not logged in';

  @override
  String get settingsUserProfileLogin => 'Log in';

  @override
  String get settingsUserProfileLogout => 'Log out';

  @override
  String get settingsAppearanceAvatarCustom => 'Custom avatar';

  @override
  String get settingsRuntimeConnection => 'Current device space';

  @override
  String get settingsRuntimeConnectionDescription =>
      'Connection status for this device and its device space.';

  @override
  String get settingsRuntimeCurrentSpace => 'Current device space';

  @override
  String get settingsRuntimeRenameSpace => 'Rename device space';

  @override
  String get settingsRuntimeLeaveSpace => 'Leave device space';

  @override
  String get settingsRuntimeLeaveSpaceTitle =>
      'Leave the current device space?';

  @override
  String get settingsRuntimeLeaveSpaceDescription =>
      'This device will create a new single-device space. Business data and paired devices are preserved.';

  @override
  String get settingsRuntimeLeaveSpaceConfirm => 'Leave';

  @override
  String get settingsRuntimeSpaceName => 'Device space name';

  @override
  String settingsRuntimeSpaceId(String spaceId) {
    return 'Device space ID: $spaceId';
  }

  @override
  String settingsRuntimeSpaceDeviceCount(int count) {
    return '$count devices';
  }

  @override
  String get settingsRuntimeViewSpaceTopology => 'View device topology';

  @override
  String settingsRuntimeSpaceTopologyTitle(String spaceName) {
    return '$spaceName device topology';
  }

  @override
  String settingsRuntimeSpaceTopologySummary(
    int deviceCount,
    int connectionCount,
  ) {
    return '$deviceCount devices · $connectionCount direct connections';
  }

  @override
  String get settingsRuntimeDisconnectConnection => 'Disconnect';

  @override
  String get settingsRuntimeDisconnectConnectionTitle =>
      'Disconnect direct connection';

  @override
  String settingsRuntimeDisconnectConnectionMessage(String deviceName) {
    return 'Disconnect the direct connection to $deviceName? Pairing records will be kept.';
  }

  @override
  String get settingsRuntimeDisconnectConnectionFailed => 'Disconnect failed';

  @override
  String get settingsRuntimeCurrentDevice => 'Current device';

  @override
  String get settingsRuntimeRemoteTitle => 'Connected devices';

  @override
  String get settingsRuntimeRemoteDescription =>
      'Directly paired devices. Pairing establishes a connection; joining a device space enables shared data and routing.';

  @override
  String get settingsRuntimePairRemote => 'Connect another device';

  @override
  String get settingsRuntimeNoPairedRemote => 'No connected devices yet.';

  @override
  String get settingsRuntimeConnectionInitiatedByOtherDevice =>
      'Connection initiated by the other device';

  @override
  String get settingsRuntimePairToken => 'Connection token';

  @override
  String get settingsRuntimePairCode => 'Pairing code';

  @override
  String get settingsRuntimeStartPairing => 'Start connection';

  @override
  String get settingsRuntimeFinishPairing => 'Finish connection';

  @override
  String get settingsRuntimeBaseUrl => 'Device address';

  @override
  String settingsRuntimeConnectionFailed(String error) {
    return 'Device connection failed: $error';
  }

  @override
  String get settingsRuntimePairingRejected =>
      'A device connection was rejected';

  @override
  String get settingsRuntimePairedChecking => 'Checking';

  @override
  String get settingsRuntimePairedOnline => 'Online';

  @override
  String get settingsRuntimePairedOffline => 'Offline';

  @override
  String get settingsRuntimePairedInvalid => 'Pairing invalid';

  @override
  String get settingsRuntimePairedError => 'Status unavailable';

  @override
  String get settingsRuntimePairingRevokedTitle =>
      'Pairing revoked by the other device';

  @override
  String get settingsRuntimePairingRevokedMessage =>
      'The other device cancelled its pairing with this device. The local pairing record is no longer valid.';

  @override
  String get settingsRuntimePairingRevokedConfirm => 'Got it';

  @override
  String get settingsRuntimePairedRemovedFromSpace =>
      'Removed from device space';

  @override
  String get settingsRuntimeRemovedFromSpaceTitle =>
      'Removed from device space';

  @override
  String get settingsRuntimeRemovedFromSpaceMessage =>
      'The remote device removed this device from the space. Leave the current space and create a standalone device space now.';

  @override
  String get settingsRuntimeRemovedFromSpaceConfirm =>
      'Leave and create standalone space';

  @override
  String get settingsRuntimeJoinSpace => 'Join device space';

  @override
  String settingsRuntimeJoinSpaceTitle(String deviceName) {
    return 'Join $deviceName\'s device space?';
  }

  @override
  String get settingsRuntimeJoinSpaceDescription =>
      'The current device space will merge with the other one and use its name.';

  @override
  String get settingsRuntimePairingComplete => 'Device paired';

  @override
  String get settingsRuntimeDeviceInCurrentSpace =>
      'In the current device space';

  @override
  String get settingsRuntimeDiscoverSpaces => 'Discover device spaces';

  @override
  String get settingsRuntimeDiscoverSpacesDescription =>
      'Nearby devices are grouped by device space. Expand a device space to connect directly to one of its devices.';

  @override
  String settingsRuntimeDiscoveredSpaceSummary(
    int memberCount,
    int nearbyCount,
  ) {
    return '$memberCount devices total · $nearbyCount nearby';
  }

  @override
  String get settingsRuntimeScan => 'Scan';

  @override
  String get settingsRuntimeScanning => 'Scanning…';

  @override
  String get settingsRuntimeEnterManually => 'Enter manually';

  @override
  String get settingsRuntimeConnect => 'Connect';

  @override
  String get settingsRuntimeEnableDiscovery =>
      'Allow nearby devices to discover this device space';

  @override
  String get settingsRuntimeEnableDiscoveryDescription =>
      'Devices on the same network can find this device space and choose this device for a direct connection.';

  @override
  String settingsRuntimeEnableDiscoveryFailed(String error) {
    return 'Could not enable device space discovery: $error';
  }

  @override
  String settingsRuntimeDisableDiscoveryFailed(String error) {
    return 'Could not disable device space discovery: $error';
  }

  @override
  String get settingsRuntimeUsingLocal => 'Using: this device';

  @override
  String settingsRuntimeUsingRemote(String device) {
    return 'Using: $device';
  }

  @override
  String get settingsRuntimeRemoteInUseDescription =>
      'Chats and tools run on this connected device.';

  @override
  String get settingsWebAccessService => 'Allow access';

  @override
  String get settingsWebAccessServiceDescription =>
      'When enabled, browsers can access this device with an address and token.';

  @override
  String get settingsWebAccessEnable => 'Allow external access';

  @override
  String get settingsWebAccessPortMode => 'Port mode';

  @override
  String get settingsWebAccessPortAutomatic => 'Automatic';

  @override
  String get settingsWebAccessPortFixed => 'Fixed';

  @override
  String get settingsWebAccessPortAutomaticDescription =>
      'The app chooses a port automatically. No manual setup is needed.';

  @override
  String get settingsWebAccessPortFixedDescription =>
      'Only the port in the listen address is used.';

  @override
  String get settingsWebAccessBindAddress => 'Listen address';

  @override
  String get settingsWebAccessToken => 'Access token';

  @override
  String get settingsWebAccessRotateToken => 'Change token';

  @override
  String get settingsWebAccessCopyToken => 'Copy token';

  @override
  String get settingsWebAccessAccessUrl => 'Access address';

  @override
  String get settingsWebAccessLocalUrl => 'This device';

  @override
  String get settingsWebAccessPairingUrl => 'Pairing address';

  @override
  String get settingsWebAccessPairingUrlLocalOnly => 'This device only';

  @override
  String get settingsWebAccessPairingUrlUnavailable => 'No LAN address found';

  @override
  String get settingsWebAccessCopyUrl => 'Copy URL';

  @override
  String get settingsWebAccessOpenUrl => 'Open address';

  @override
  String get settingsWebAccessRunning => 'On';

  @override
  String get settingsWebAccessStopped => 'Off';

  @override
  String get settingsWebAccessSaved => 'Access settings saved.';

  @override
  String get settingsWebAccessTokenCopied => 'Access token copied.';

  @override
  String get settingsWebAccessUrlCopied => 'Access URL copied.';

  @override
  String get settingsWebAccessPairedClients => 'Authorized devices';

  @override
  String get settingsWebAccessNoPairedClients => 'No device is authorized yet.';

  @override
  String get settingsWebAccessPairedDeleted => 'Authorized device deleted.';

  @override
  String get settingsWebAccessPairingRequest => 'Pairing request';

  @override
  String settingsWebAccessPairingRequestMessage(String code, String client) {
    return 'Pairing code: $code\nDevice: $client';
  }

  @override
  String get settingsWebAccessInvalidBindAddress =>
      'Bind address must be host:port.';

  @override
  String settingsWebAccessStartFailed(String error) {
    return 'Failed to enable access: $error';
  }

  @override
  String settingsWebAccessStopFailed(String error) {
    return 'Failed to turn off access: $error';
  }

  @override
  String get settingsAppearanceThemeSection => 'Theme';

  @override
  String get settingsAppearanceThemeMode => 'Current mode';

  @override
  String get settingsAppearanceThemeTarget => 'Theme save target';

  @override
  String get settingsAppearanceThemeTargetGlobal => 'Global';

  @override
  String settingsAppearanceThemeTargetCharacter(Object name) {
    return 'Current character: $name';
  }

  @override
  String settingsAppearanceThemeTargetGroup(Object name) {
    return 'Current group: $name';
  }

  @override
  String get settingsAppearanceThemeSystem => 'System';

  @override
  String get settingsAppearanceThemeLight => 'Light';

  @override
  String get settingsAppearanceThemeDark => 'Dark';

  @override
  String get settingsAppearanceInputSection => 'Input';

  @override
  String get settingsAppearanceInputStyle => 'Input style';

  @override
  String get settingsAppearanceInputStyleClassic => 'Classic';

  @override
  String get settingsAppearanceInputStyleAgent => 'Agent';

  @override
  String get settingsAppearanceInputFloating => 'Floating input';

  @override
  String get settingsAppearanceColorSection => 'Theme color';

  @override
  String get settingsAppearanceColorDescription =>
      'Choose a simple color preset. System bars and current app chrome follow the theme automatically.';

  @override
  String get settingsAppearanceColorDefault => 'Default';

  @override
  String get settingsAppearanceColorSky => 'Sky';

  @override
  String get settingsAppearanceColorMatcha => 'Matcha';

  @override
  String get settingsAppearanceColorEmber => 'Ember';

  @override
  String get settingsAppearanceColorRose => 'Rose';

  @override
  String get settingsAppearanceColorCustom => 'Custom colors';

  @override
  String get settingsAppearanceCustomColorsTitle => 'Custom theme colors';

  @override
  String get settingsAppearancePrimaryColor => 'Primary color';

  @override
  String get settingsAppearanceSecondaryColor => 'Secondary color';

  @override
  String get settingsAppearanceHexColorHint => '#RRGGBB';

  @override
  String get settingsAppearanceHexColorInvalid =>
      'Enter a color in #RRGGBB format';

  @override
  String get settingsAppearanceBackgroundSection => 'Background';

  @override
  String get settingsAppearanceBackgroundDescription =>
      'Choose a local image or video as the app background. App surfaces and system bars follow the theme automatically.';

  @override
  String get settingsAppearanceBackgroundImage => 'Background media';

  @override
  String get settingsAppearanceBackgroundNone => 'None selected';

  @override
  String get settingsAppearanceBackgroundChooseImage => 'Choose image';

  @override
  String get settingsAppearanceBackgroundChooseVideo => 'Choose video';

  @override
  String get settingsAppearanceBackgroundDisable => 'Disable background';

  @override
  String get settingsAppearanceBackgroundEnabled => 'Enable background';

  @override
  String get settingsAppearanceBackgroundOpacity => 'Background opacity';

  @override
  String get settingsAppearanceBackgroundBlur => 'Blur background';

  @override
  String get settingsAppearanceBackgroundBlurRadius => 'Blur strength';

  @override
  String get settingsAppearanceBackgroundVideoMuted => 'Mute video background';

  @override
  String get settingsAppearanceBackgroundVideoLoop => 'Loop video background';

  @override
  String get settingsAppearanceTextSection => 'Text';

  @override
  String get settingsAppearanceFontFamily => 'Font';

  @override
  String get settingsAppearanceFontDefault => 'Default';

  @override
  String get settingsAppearanceCustomFont => 'Custom font';

  @override
  String get settingsAppearanceFontCustom => 'Custom';

  @override
  String get settingsAppearanceChooseCustomFont => 'Choose custom font';

  @override
  String get settingsAppearanceClearCustomFont => 'Clear custom font';

  @override
  String get settingsAppearanceFontSerif => 'Serif';

  @override
  String get settingsAppearanceFontMonospace => 'Mono';

  @override
  String get settingsAppearanceFontScale => 'Font size';

  @override
  String get settingsAppearanceAvatarSection => 'Avatars';

  @override
  String get settingsAppearanceUserAvatar => 'User avatar';

  @override
  String get settingsAppearanceAiAvatar => 'AI avatar';

  @override
  String get settingsAppearanceAvatarDefault => 'Default avatar';

  @override
  String get settingsAppearanceAvatarShape => 'Avatar shape';

  @override
  String get settingsAppearanceAvatarShapeCircle => 'Circle';

  @override
  String get settingsAppearanceAvatarShapeSquare => 'Square';

  @override
  String get settingsAppearanceChooseUserAvatar => 'Choose user avatar';

  @override
  String get settingsAppearanceChooseAiAvatar => 'Choose AI avatar';

  @override
  String get settingsAppearanceClearUserAvatar => 'Clear user avatar';

  @override
  String get settingsAppearanceClearAiAvatar => 'Clear AI avatar';

  @override
  String get settingsAppearanceChatDisplaySection => 'Chat display';

  @override
  String get settingsAppearanceMessageStyle => 'Message style';

  @override
  String get settingsAppearanceMessageStyleClean => 'Command';

  @override
  String get settingsAppearanceMessageStyleCard => 'Bubble';

  @override
  String get settingsAppearanceMessageColors => 'Message colors';

  @override
  String get settingsAppearanceMessageColorsTheme => 'Follow theme';

  @override
  String get settingsAppearanceMessageColorsSky => 'Clean blue';

  @override
  String get settingsAppearanceMessageColorsMatcha => 'Matcha';

  @override
  String get settingsAppearanceMessageColorsInk => 'Dark';

  @override
  String get settingsAppearanceMessageColorsCustom => 'Custom message colors';

  @override
  String get settingsAppearanceCustomMessageColorsTitle =>
      'Custom message colors';

  @override
  String get settingsAppearanceCursorUserBubbleColor => 'Command user bubble';

  @override
  String get settingsAppearanceUserBubbleColor => 'User bubble';

  @override
  String get settingsAppearanceAiBubbleColor => 'AI bubble';

  @override
  String get settingsAppearanceUserTextColor => 'User text';

  @override
  String get settingsAppearanceAiTextColor => 'AI text';

  @override
  String get settingsAppearanceMessageSurface => 'Global texture';

  @override
  String get settingsAppearanceMessageSurfaceNormal => 'Normal';

  @override
  String get settingsAppearanceMessageSurfaceTransparent => 'Transparent';

  @override
  String get settingsAppearanceUserBubbleFont => 'User bubble font';

  @override
  String get settingsAppearanceAiBubbleFont => 'AI bubble font';

  @override
  String get settingsAppearanceAdjustUserBubbleFont =>
      'Adjust user bubble font';

  @override
  String get settingsAppearanceAdjustAiBubbleFont => 'Adjust AI bubble font';

  @override
  String get settingsAppearanceEnableBubbleFont =>
      'Enable bubble-specific font';

  @override
  String get settingsAppearanceUserBubbleImage => 'User bubble image';

  @override
  String get settingsAppearanceAiBubbleImage => 'AI bubble image';

  @override
  String get settingsAppearanceChooseUserBubbleImage => 'Choose user bubble';

  @override
  String get settingsAppearanceChooseAiBubbleImage => 'Choose AI bubble';

  @override
  String get settingsAppearanceClearUserBubbleImage => 'Clear user bubble';

  @override
  String get settingsAppearanceClearAiBubbleImage => 'Clear AI bubble';

  @override
  String get settingsAppearanceBubbleImageRenderMode => 'Bubble image mode';

  @override
  String get settingsAppearanceBubbleImageTiledNineSlice => 'Tiled 9-slice';

  @override
  String get settingsAppearanceBubbleImageNinePatch => 'Stretch 9-patch';

  @override
  String get settingsAppearanceBubbleImageAdjustUser =>
      'Adjust user bubble image';

  @override
  String get settingsAppearanceBubbleImageAdjustAi => 'Adjust AI bubble image';

  @override
  String get settingsAppearanceBubbleImagePreview => 'Preview';

  @override
  String get settingsAppearanceBubbleImagePreviewText =>
      'Bubble preview with 9-slice guides';

  @override
  String get settingsAppearanceBubbleImageCrop => 'Crop';

  @override
  String get settingsAppearanceBubbleImageRepeat => 'Repeat region';

  @override
  String get settingsAppearanceBubbleImageScale => 'Image scale';

  @override
  String get settingsAppearanceBubbleImageCropLeft => 'Crop left';

  @override
  String get settingsAppearanceBubbleImageCropTop => 'Crop top';

  @override
  String get settingsAppearanceBubbleImageCropRight => 'Crop right';

  @override
  String get settingsAppearanceBubbleImageCropBottom => 'Crop bottom';

  @override
  String get settingsAppearanceBubbleImageRepeatStart => 'Repeat X start';

  @override
  String get settingsAppearanceBubbleImageRepeatEnd => 'Repeat X end';

  @override
  String get settingsAppearanceBubbleImageRepeatYStart => 'Repeat Y start';

  @override
  String get settingsAppearanceBubbleImageRepeatYEnd => 'Repeat Y end';

  @override
  String get settingsAppearanceMessageDensity => 'Message spacing';

  @override
  String get settingsAppearanceMessageDensityComfortable => 'Comfortable';

  @override
  String get settingsAppearanceMessageDensityCompact => 'Compact';

  @override
  String get settingsAppearanceWideLayout => 'Use wider chat layout';

  @override
  String get settingsAppearanceRoundedMessages => 'Rounded message cards';

  @override
  String get settingsAppearanceShowAvatars => 'Show message avatars';

  @override
  String get settingsAppearanceMessageDisplaySection => 'Message display';

  @override
  String get settingsAppearanceShowThinkingProcess => 'Show thinking process';

  @override
  String get settingsAppearanceShowRoleName => 'Show role name';

  @override
  String get settingsAppearanceShowUserName => 'Show user name';

  @override
  String get settingsAppearanceShowModelName => 'Show model name';

  @override
  String get settingsAppearanceShowModelProvider => 'Show model provider';

  @override
  String get settingsAppearanceShowMessageTokenStats => 'Show token stats';

  @override
  String get settingsAppearanceShowMessageTimingStats => 'Show timing stats';

  @override
  String get settingsAppearanceShowMessageTimestamp => 'Show message time';

  @override
  String get settingsAppearanceShowInputProcessingStatus =>
      'Show input processing status';

  @override
  String get settingsAppearanceResetTheme => 'Reset theme settings';

  @override
  String get settingsAppearanceLanguageSection => 'Language';

  @override
  String get settingsAppearanceLanguage => 'Current language';

  @override
  String get settingsAppearanceLanguageDescription =>
      'Language follows the localization configuration loaded at app startup.';

  @override
  String get settingsDataRuntimeSection => 'Data overview';

  @override
  String get settingsDataCoreVersion => 'Current version';

  @override
  String get settingsDataStorageSection => 'Storage location';

  @override
  String get settingsDataStorageDescription =>
      'Move runtime and workspace data into independently selected local folders.';

  @override
  String get settingsDataRuntimeRoot => 'Runtime root';

  @override
  String get settingsDataWorkspaceRoot => 'Workspace root';

  @override
  String get settingsDataChooseStorageRoots => 'Edit locations';

  @override
  String get settingsDataEditStorageRootsTitle => 'Edit storage locations';

  @override
  String get settingsDataStorageRootsRequired =>
      'Runtime root and workspace root are required.';

  @override
  String get settingsDataStorageConfirmTitle => 'Change storage location';

  @override
  String get settingsDataStorageConfirmMessage =>
      'Runtime and workspace data will be copied into the selected directories, and the app will use them after restart.';

  @override
  String get settingsDataStorageConfirmAction => 'Change location';

  @override
  String get settingsDataStorageChanged =>
      'Storage location changed. Restart the app to use it.';

  @override
  String settingsDataStorageChangeError(String error) {
    return 'Storage location change failed: $error';
  }

  @override
  String get settingsDataTokenSection => 'Usage statistics';

  @override
  String get settingsDataInputTokens => 'Input';

  @override
  String get settingsDataOutputTokens => 'Output';

  @override
  String get settingsDataOpenDetailedStats => 'View detailed statistics';

  @override
  String get settingsDataOpenDetailedStatsDescription =>
      'Open daily trends, input/output token changes, and usage breakdown by provider, model, and conversation.';

  @override
  String get settingsDataRefreshTokenStats => 'Refresh statistics';

  @override
  String get settingsDataResetTokenStats => 'Reset statistics';

  @override
  String get settingsDataDetailedStatsTitle => 'Detailed statistics';

  @override
  String get settingsDataDetailedStatsDescription =>
      'Statistics are calculated from dedicated model request records.';

  @override
  String get settingsDataDetailedStatsEmpty => 'No detailed usage records yet';

  @override
  String settingsDataDetailedStatsDateRange(String start, String end) {
    return '$start to $end';
  }

  @override
  String get settingsDataDetailedStatsSourceLabel => 'Model request records';

  @override
  String get settingsDataDetailedStatsSourceChat => 'Chat response';

  @override
  String get settingsDataDetailedStatsSourceToolResult =>
      'Tool-result response';

  @override
  String get settingsDataDetailedStatsSourceSummary => 'Summary generation';

  @override
  String get settingsDataDetailedStatsSourceTitleGeneration =>
      'Title generation';

  @override
  String get settingsDataDetailedStatsSourceMemory => 'Memory analysis';

  @override
  String get settingsDataDetailedStatsTotalRequests => 'Total requests';

  @override
  String get settingsDataDetailedStatsCachedInput => 'Cached input';

  @override
  String get settingsDataDetailedStatsActiveDays => 'Active days';

  @override
  String get settingsDataDetailedStatsChats => 'Conversations';

  @override
  String get settingsDataDetailedStatsProviders => 'Providers';

  @override
  String get settingsDataDetailedStatsModels => 'Models';

  @override
  String get settingsDataDetailedStatsDailyUsageTitle => 'Daily usage trend';

  @override
  String get settingsDataDetailedStatsDailyUsageSubtitle =>
      'Request count by day';

  @override
  String get settingsDataDetailedStatsRequestsSeries => 'Requests';

  @override
  String get settingsDataDetailedStatsInputOutputTitle =>
      'Input / output consumption trend';

  @override
  String get settingsDataDetailedStatsInputOutputSubtitle =>
      'Daily token changes for input and output';

  @override
  String get settingsDataDetailedStatsProviderPieTitle =>
      'Provider distribution';

  @override
  String get settingsDataDetailedStatsModelPieTitle => 'Model distribution';

  @override
  String get settingsDataDetailedStatsChatPieTitle =>
      'Conversation distribution';

  @override
  String get settingsDataDetailedStatsTotalTokens => 'Total tokens';

  @override
  String get settingsDataDetailedStatsTopRequestsTitle => 'Top requests';

  @override
  String get settingsDataDetailedStatsTopRequestsSubtitle =>
      'Highest single-request token consumption';

  @override
  String get settingsDataDetailedStatsTopChatsTitle => 'Top conversations';

  @override
  String get settingsDataDetailedStatsTopChatsSubtitle =>
      'Highest total token consumption by conversation';

  @override
  String get settingsDataDetailedStatsOther => 'Other';

  @override
  String settingsDataDetailedStatsInputOutputSummary(
    String input,
    String output,
    String chatTitle,
    String time,
  ) {
    return 'Input $input · Output $output · $chatTitle · $time';
  }

  @override
  String settingsDataDetailedStatsRequestModelSummary(
    int requests,
    int models,
  ) {
    return '$requests requests · $models models';
  }

  @override
  String get settingsDataDetailedStatsUnlabeledProvider => 'Unlabeled provider';

  @override
  String get settingsDataDetailedStatsUnlabeledModel => 'Unlabeled model';

  @override
  String get settingsDataDetailedStatsUntitledChat => 'Untitled conversation';

  @override
  String get settingsDataBackupSection => 'Backup & restore';

  @override
  String get settingsDataChatHistoriesBackup => 'Chat data';

  @override
  String get settingsDataChatHistoriesBackupDescription =>
      'Back up all chats and messages. Restore updates or creates chats by chat ID.';

  @override
  String get settingsDataCharacterCardsBackup => 'Character card data';

  @override
  String get settingsDataCharacterCardsBackupDescription =>
      'Back up all character cards and referenced tags. Restore updates or creates items by original ID.';

  @override
  String get settingsDataCharacterGroupsBackup => 'Group data';

  @override
  String get settingsDataCharacterGroupsBackupDescription =>
      'Back up all groups. Restore keeps member references and ordering.';

  @override
  String get settingsDataModelConfigsBackup => 'Model settings';

  @override
  String get settingsDataModelConfigsBackupDescription =>
      'Back up all model settings, including model parameters and API key pools.';

  @override
  String settingsDataBackupCount(int count) {
    return '$count items';
  }

  @override
  String get settingsDataCopyBackupJson => 'Copy backup';

  @override
  String get settingsDataImportBackupJson => 'Restore data';

  @override
  String get settingsDataBackupJsonInput => 'Restore content';

  @override
  String settingsDataBackupCopied(String name) {
    return 'Copied backup for “$name”.';
  }

  @override
  String settingsDataBackupImportResult(
    int newCount,
    int updatedCount,
    int skippedCount,
  ) {
    return 'Restore complete: $newCount new, $updatedCount updated, $skippedCount skipped.';
  }

  @override
  String settingsDataBackupImportError(String error) {
    return 'Restore failed: $error';
  }

  @override
  String settingsDataBackupCopyError(String error) {
    return 'Copy failed: $error';
  }

  @override
  String get settingsDataSnapshotBackupTitle => 'Full snapshot';

  @override
  String get settingsDataExportRawSnapshot => 'Export snapshot';

  @override
  String get settingsDataImportRawSnapshot => 'Restore snapshot';

  @override
  String get settingsDataExportRawSnapshotDescription =>
      'Pack chats, characters, model settings, and local files into one backup file. Restoring replaces current data with the backup.';

  @override
  String settingsDataSnapshotBytes(int bytes) {
    return 'Snapshot size: $bytes bytes';
  }

  @override
  String get settingsDataSnapshotImported => 'Snapshot restored.';

  @override
  String settingsDataSnapshotExportError(String error) {
    return 'Snapshot export failed: $error';
  }

  @override
  String settingsDataSnapshotImportError(String error) {
    return 'Snapshot restore failed: $error';
  }

  @override
  String get settingsDataSnapshotRestoreConfirmTitle => 'Restore full snapshot';

  @override
  String settingsDataSnapshotRestoreConfirmMessage(
    int formatVersion,
    int fileCount,
    String createdAt,
    int bytes,
  ) {
    return 'Restoring will replace the current runtime data.\nFormat version: $formatVersion\nFiles: $fileCount\nCreated: $createdAt\nSnapshot size: $bytes bytes';
  }

  @override
  String get settingsDataSnapshotRestoreConfirmAction => 'Restore';

  @override
  String get settingsDataImportOperit1Snapshot => 'Import from Operit1';

  @override
  String get settingsDataOperit1SnapshotImported =>
      'Operit1 snapshot imported.';

  @override
  String settingsDataOperit1SnapshotImportError(String error) {
    return 'Operit1 snapshot import failed: $error';
  }

  @override
  String settingsDataOperit1SnapshotImportConfirmMessage(
    String fileName,
    int formatVersion,
    String chatModelId,
    int chatCount,
    int messageCount,
    int fileCount,
    int byteCount,
  ) {
    return 'Import this Operit1 snapshot into the current Runtime.\nFile: $fileName\nFormat version: $formatVersion\nChat model: $chatModelId\nChats: $chatCount; messages: $messageCount\nResource files: $fileCount\nSnapshot size: $byteCount bytes';
  }

  @override
  String get settingsDataOperit1SnapshotImportAction => 'Import';

  @override
  String get settingsDataAdvancedBackupOptions => 'Advanced options';

  @override
  String get settingsDataAdvancedBackupOptionsDescription =>
      'Single-item JSON export and restore';

  @override
  String get onboardingIntroTagline => 'Make everyday tasks simpler from here';

  @override
  String get onboardingStart => 'Start';

  @override
  String get onboardingPleaseWait => 'Please wait';

  @override
  String get onboardingAgree => 'Agree';

  @override
  String get onboardingAgreementProgress => 'User agreement';

  @override
  String get onboardingSkip => 'Skip';

  @override
  String get onboardingPrevious => 'Previous page';

  @override
  String get onboardingAgreementTitle => 'User agreement and privacy policy';

  @override
  String get onboardingAgreementDescription =>
      'Please read this agreement. You can continue device setup after agreeing.';

  @override
  String onboardingAgreementVersion(String version) {
    return 'Version: $version';
  }

  @override
  String onboardingAgreementWait(int seconds) {
    return 'You can confirm after $seconds seconds.';
  }

  @override
  String get onboardingAgreementPlainTitle =>
      'Plain-language summary (not the legal version)';

  @override
  String get onboardingAgreementPlainIntro =>
      'Operit is an open-source client that runs on your device. We do not operate model inference services, host chat histories, or provide shared API keys. When you configure cloud models, voice, search, image generation, MCP, or other network features, data is sent directly to the relevant provider and is governed by that provider\'s terms and privacy policy. Local models run on your device.';

  @override
  String get onboardingAgreementPlainCapabilities =>
      'The app may use files, terminals, automation, system permissions, Root, ADB, and extensions. Check the operation, back up important data, and grant permissions carefully. Responsibility for device, data, account, or other loss caused by your actions, configuration, third-party services, or extensions is handled by the actual operator under applicable law.';

  @override
  String get onboardingAgreementPlainThirdParty =>
      'Copyright and responsibility for marketplace plugins, scripts, Skills, tool packages, and other third-party content belong to their authors or rights holders. Displaying or installing them does not mean Operit guarantees, endorses, or acquires rights to them.';

  @override
  String get onboardingAgreementLegalTitle => 'Formal legal agreement';

  @override
  String get onboardingAgreementLegalSection1 =>
      '1. Scope and agreement version\nThis agreement applies to officially released Operit clients and optional online features. By using this app, you confirm that you have read and agreed to the current version. The app records the version you accepted and will ask for confirmation again after a material change. Open-source licensing is governed by LGPLv3 in the repository-root LICENSE. This agreement does not exclude or reduce rights granted by applicable law or open-source licenses.';

  @override
  String get onboardingAgreementLegalSection2 =>
      '2. Product role and third-party services\nOperit does not provide large language model inference, shared API keys, chat-request relays, or cloud hosting of chat histories. You select, configure, and enable third-party services yourself and are responsible for judging the security, legality, and suitability of providers, models, endpoints, and extensions, and for protecting credentials.';

  @override
  String get onboardingAgreementLegalSection3 =>
      '3. Data handling and privacy\nChat histories, character cards, memories, model settings, and API keys are generally stored in app data on your device. When you export, back up, upload files, use a third-party network feature, or submit a request to an external HTTP service, related data is copied, transmitted, or disclosed according to your action. Features such as the marketplace, notices, update checks, GitHub login, and publishing access Operit, GitHub, or related third-party resources.';

  @override
  String get onboardingAgreementLegalSection4 =>
      '4. External deployment and operator responsibility\nExternal HTTP services, bots, automatic replies, and similar capabilities are enabled at your choice. If you make them available to other people or the public, you are the actual deployer or operator and are responsible for access control, user authorization, content safety, data protection, protection of minors, required notices, and other applicable obligations.';

  @override
  String get onboardingAgreementLegalSection5 =>
      '5. Lawful use and content responsibility\nYou must follow applicable laws, third-party service rules, and platform rules. Do not use this app, extensions, or configurations for illegal activity, infringement of rights, unauthorized access to systems or data, or distribution of illegal or harmful content. AI output may contain errors, omissions, or bias and is not medical, legal, financial, or other professional advice.';

  @override
  String get onboardingAgreementLegalSection6 =>
      '6. Software provided as is\nTo the extent permitted by applicable law, this software is provided as is and as available. Contributors make no express or implied warranty regarding continued availability, accuracy, security, merchantability, fitness for a particular purpose, or non-infringement of the software or third-party services.';

  @override
  String get onboardingAgreementLegalSection7 =>
      '7. Agreement updates and contact\nWe may update this agreement for functional, legal, or security requirements and provide the current version in the app. Updates that materially affect user rights take effect by increasing the agreement version and requesting confirmation again. Questions and feedback can be submitted through the project repository, in-app feedback entry, or published contact channels.';

  @override
  String get onboardingAgreementPrecedenceNote =>
      'The plain-language summary is provided only to aid understanding. If it differs from the formal Chinese agreement, the formal Chinese agreement takes precedence.';

  @override
  String get onboardingStorageRequired =>
      'Runtime and workspace folders are both required.';

  @override
  String get onboardingModelLoadFirst => 'Load the available models first.';

  @override
  String get onboardingModelSelectDefault => 'Select the default model.';

  @override
  String get onboardingImportSelectSnapshotError =>
      'Select an Operit 1 snapshot file.';

  @override
  String get onboardingRemoteCredentialsRequired =>
      'Remote address and access token are both required.';

  @override
  String get onboardingRemoteCodeRequired => 'Enter the one-time pairing code.';

  @override
  String get onboardingActionSaving => 'Saving';

  @override
  String get onboardingActionConfirm => 'Confirm';

  @override
  String get onboardingActionPreparing => 'Preparing';

  @override
  String get onboardingActionContinue => 'Continue';

  @override
  String get onboardingActionLoadingModels => 'Loading';

  @override
  String get onboardingActionReading => 'Reading';

  @override
  String get onboardingActionImporting => 'Importing';

  @override
  String get onboardingActionConnecting => 'Connecting';

  @override
  String get onboardingActionPairing => 'Pairing';

  @override
  String get onboardingActionStartPairing => 'Start pairing';

  @override
  String get onboardingActionFinishConnection => 'Finish connection';

  @override
  String get onboardingActionProcessing => 'Processing';

  @override
  String get onboardingActionComplete => 'Done';

  @override
  String get onboardingProgressStorage => 'Storage';

  @override
  String get onboardingProgressStartMode => 'Setup method';

  @override
  String get onboardingProgressModel => 'Model setup';

  @override
  String get onboardingProgressImport => 'Import';

  @override
  String get onboardingProgressRemote => 'Remote connection';

  @override
  String get onboardingProgressDeviceSpace => 'Device space';

  @override
  String get onboardingProgressPermissions => 'Permissions';

  @override
  String get onboardingProgressWelcome => 'Welcome';

  @override
  String get onboardingPreparingLocalRuntime => 'Preparing the local Runtime';

  @override
  String get onboardingModeTitle => 'Choose how to begin';

  @override
  String get onboardingModeDescription =>
      'Use quick setup for a new installation, import existing Operit 1 data, or connect directly to a remote Runtime.';

  @override
  String get onboardingModeQuickTitle => 'Quick setup';

  @override
  String get onboardingModeQuickSubtitle =>
      'Configure a model provider and finish the basic setup';

  @override
  String get onboardingModeImportTitle => 'Import from Operit 1';

  @override
  String get onboardingModeImportSubtitle =>
      'Bring over settings and data from Operit 1';

  @override
  String get onboardingModeRemoteTitle => 'Connect to another Runtime';

  @override
  String get onboardingModeRemoteSubtitle =>
      'Enter the remote address, access token, and one-time pairing code';

  @override
  String get onboardingModeDeviceSpaceTitle => 'Join an existing device space';

  @override
  String get onboardingModeDeviceSpaceSubtitle =>
      'Find a nearby device space and sync models and data after joining';

  @override
  String get onboardingDeviceSpaceDescription =>
      'Find a nearby device space and choose a device. After pairing, your models, settings, and data are synchronized before onboarding finishes.';

  @override
  String get onboardingRemoteDescription =>
      'Use this app as an independent client. After pairing, it switches to the remote Runtime.';

  @override
  String get onboardingRemoteAddress => 'Remote address';

  @override
  String get onboardingRemoteToken => 'Access token';

  @override
  String get onboardingRemotePairingStarted => 'Pairing started';

  @override
  String onboardingRemotePairingSummary(
    String platform,
    String model,
    String deviceId,
  ) {
    return 'Remote: $platform / $model\nDevice: $deviceId\nEnter the one-time code shown by the remote app.';
  }

  @override
  String get onboardingRemotePairingCode => 'One-time pairing code';

  @override
  String get onboardingRemoteBeforePairing =>
      'Start a pairing session with the remote address and access token.';

  @override
  String get onboardingRemoteAfterPairing =>
      'The code is used only for this confirmation. The remote Runtime will be saved as a connection option.';

  @override
  String get onboardingStorageTitle => 'Confirm storage on this device';

  @override
  String get onboardingStorageDescription =>
      'Runtime data and workspace data are stored separately. Enter paths or choose each folder.';

  @override
  String get onboardingStorageRuntimeFolder => 'Runtime data folder';

  @override
  String get onboardingStorageWorkspaceFolder => 'Workspace folder';

  @override
  String get onboardingStorageReading => 'Reading storage paths';

  @override
  String get onboardingStorageDetail =>
      'These folders store Runtime state and your workspace separately. The local Host mounts them after confirmation.';

  @override
  String onboardingStorageChooseFolder(String label) {
    return 'Choose $label';
  }

  @override
  String get onboardingImportTitle => 'Import from Operit 1';

  @override
  String get onboardingImportDescription =>
      'Choose an Operit 1 snapshot to migrate settings, chats, character cards, and resources to Operit 2.';

  @override
  String get onboardingImportReadingSnapshot => 'Reading snapshot';

  @override
  String get onboardingImportSelectSnapshot => 'Select snapshot file';

  @override
  String get onboardingImportDetected => 'Content ready to migrate';

  @override
  String get onboardingImportModelSettings => 'Model settings';

  @override
  String get onboardingImportChats => 'Chats';

  @override
  String get onboardingImportMessages => 'Messages';

  @override
  String get onboardingImportPreferences => 'Preference files';

  @override
  String get onboardingImportResources => 'Resource files';

  @override
  String get onboardingImportExternalResources => 'External resources';

  @override
  String onboardingImportDefaultModel(String modelId) {
    return 'Default chat model: $modelId';
  }

  @override
  String get onboardingImportInProgress =>
      'Importing the snapshot. Please wait.';

  @override
  String get onboardingImportReady =>
      'Continue to migrate the complete snapshot.';

  @override
  String get onboardingModelTitle => 'Finish model setup';

  @override
  String get onboardingModelDescription =>
      'Choose a model provider, enter the API key, load models, and select the default model.';

  @override
  String get onboardingModelProvider => 'Model provider';

  @override
  String get onboardingModelProviderRequired => 'Select a model provider.';

  @override
  String get onboardingModelContinueSetup => 'Continue setup';

  @override
  String get onboardingModelEndpoint => 'Service address';

  @override
  String get onboardingModelLoadingAvailable => 'Loading models';

  @override
  String get onboardingModelLoadAvailable => 'Load available models';

  @override
  String get onboardingModelDefault => 'Default model';

  @override
  String get onboardingPermissionsTitle =>
      'Grant only the permissions you need';

  @override
  String get onboardingPermissionsDescription =>
      'Choose permissions for the features you use. You may continue without granting any; features without permission will remain unavailable.';

  @override
  String get onboardingPermissionsEmptyTitle =>
      'No permissions need attention on this device';

  @override
  String get onboardingPermissionsEmptySubtitle =>
      'This environment has no system permissions to handle during setup.';

  @override
  String get onboardingPermissionsRefresh => 'Refresh permission status';

  @override
  String get onboardingPermissionOptional => 'Optional';

  @override
  String get onboardingFieldRequired => 'Required';

  @override
  String get onboardingPermissionGranted => 'Granted';

  @override
  String get onboardingPermissionGrant => 'Grant';

  @override
  String get onboardingPermissionNoAction => 'No action needed';

  @override
  String get networkErrorBadRequestTitle => 'Check the request settings';

  @override
  String get networkErrorBadRequestMessage =>
      'The service rejected the model-list request. Check that the service address matches the provider.';

  @override
  String get networkErrorUnauthorizedTitle => 'API key could not be verified';

  @override
  String get networkErrorUnauthorizedMessage =>
      'The provider rejected the API key. Enter the complete key and try loading the models again.';

  @override
  String get networkErrorForbiddenTitle => 'Access is not permitted';

  @override
  String get networkErrorForbiddenMessage =>
      'This key cannot access the provider API. Check the account permissions and model-service access.';

  @override
  String get networkErrorNotFoundTitle => 'Service address is unavailable';

  @override
  String get networkErrorNotFoundMessage =>
      'No model-list endpoint was found at this address. Check the address and path.';

  @override
  String get networkErrorRateLimitedTitle => 'Too many requests';

  @override
  String get networkErrorRateLimitedMessage =>
      'The provider is limiting requests. Wait a moment and try loading the models again.';

  @override
  String get networkErrorServerTitle => 'Provider service error';

  @override
  String get networkErrorServerMessage =>
      'The provider cannot process the model-list request right now. Try again later.';

  @override
  String get networkErrorModelListTitle => 'Could not load the model list';

  @override
  String get networkErrorModelListMessage =>
      'No model list was returned. Check the service address, API key, and network connection.';

  @override
  String get networkErrorConnectionTitle => 'Network connection failed';

  @override
  String get networkErrorConnectionMessage =>
      'Could not connect to the model provider. Check the network connection and service address.';

  @override
  String get networkErrorDuplicateModelTitle => 'Model already added';

  @override
  String networkErrorDuplicateModelMessage(
    String modelId,
    String providerName,
  ) {
    return 'Model “$modelId” is already added to provider “$providerName”.';
  }

  @override
  String get networkErrorDefaultTitle => 'Model setup failed';

  @override
  String get networkErrorDefaultMessage =>
      'An error occurred while loading models. Check the provider, service address, and API key.';

  @override
  String get onboardingRequirementWindowsAdminTitle => 'Administrator access';

  @override
  String get onboardingRequirementWindowsAdminDescription =>
      'Shows whether the Host is running as administrator. Elevation must be handled when the Host is launched.';

  @override
  String get onboardingRequirementAndroidFileManagementTitle => 'File access';

  @override
  String get onboardingRequirementAndroidFileManagementDescription =>
      'Allows the Host to read and write the Android shared-storage folders you choose.';

  @override
  String get onboardingRequirementAndroidNotificationsTitle => 'Notifications';

  @override
  String get onboardingRequirementAndroidNotificationsDescription =>
      'Allows the Host to show foreground-service status, task progress, and tool results.';

  @override
  String get onboardingRequirementAndroidAppListTitle => 'Installed apps';

  @override
  String get onboardingRequirementAndroidAppListDescription =>
      'Allows the Host to list, launch, and stop Android apps.';

  @override
  String get onboardingRequirementAndroidUsageStatsTitle => 'App usage access';

  @override
  String get onboardingRequirementAndroidUsageStatsDescription =>
      'Allows the Host to read how long apps have been used in the foreground.';

  @override
  String get onboardingRequirementAndroidWriteSettingsTitle =>
      'Change system settings';

  @override
  String get onboardingRequirementAndroidWriteSettingsDescription =>
      'Allows the Host to change supported Android system settings.';

  @override
  String get onboardingRequirementAndroidLocationTitle =>
      'Nearby-device location';

  @override
  String get onboardingRequirementAndroidLocationDescription =>
      'Allows location access required by some nearby-device discovery features.';

  @override
  String get onboardingRequirementAndroidBluetoothTitle => 'Bluetooth';

  @override
  String get onboardingRequirementAndroidBluetoothDescription =>
      'Allows Bluetooth scanning and connections to discover and use devices.';

  @override
  String get onboardingRequirementAndroidOverlayTitle =>
      'Display over other apps';

  @override
  String get onboardingRequirementAndroidOverlayDescription =>
      'Allows Operit to show an entry point over other apps.';

  @override
  String get onboardingRequirementAndroidBatteryOptimizationTitle =>
      'Keep long tasks running';

  @override
  String get onboardingRequirementAndroidBatteryOptimizationDescription =>
      'Excludes Operit from battery optimization so sync, collaboration, and long tasks can continue.';

  @override
  String get onboardingRequirementAndroidShizukuTitle => 'Shizuku';

  @override
  String get onboardingRequirementAndroidShizukuDescription =>
      'Optional. Start Shizuku or Sui first, then grant access to use supported Android system features.';

  @override
  String get onboardingRequirementAndroidRootTitle => 'Root';

  @override
  String get onboardingRequirementAndroidRootDescription =>
      'Optional. Grants access to Android system features that require Root.';

  @override
  String get onboardingRequirementOhosLocationTitle => 'Location access';

  @override
  String get onboardingRequirementOhosLocationDescription =>
      'Allows the Host to read device location and support nearby-device discovery that requires location access.';

  @override
  String get onboardingRequirementOhosBluetoothTitle => 'Bluetooth access';

  @override
  String get onboardingRequirementOhosBluetoothDescription =>
      'Allows scanning, connecting, reading, and writing with classic Bluetooth and BLE devices.';

  @override
  String get runtimeBootstrapPreparingAssets => 'Preparing local Runtime files';

  @override
  String get runtimeBootstrapInitializingCore => 'Starting local core services';

  @override
  String get runtimeBootstrapFailed => 'Could not start the local Runtime';

  @override
  String get runtimeBootstrapReady => 'Local Runtime is ready';

  @override
  String get runtimeBootstrapUnconfigured =>
      'Choose the Runtime data and workspace folders in the main window first';

  @override
  String get mainExitConfirm => 'Press back again to exit';

  @override
  String get messageMenuCopy => 'Copy message';

  @override
  String get messageMenuEditAndResend => 'Edit and resend';

  @override
  String get messageMenuRollback => 'Roll back to here';

  @override
  String get messageMenuRegenerate => 'Regenerate';

  @override
  String get messageMenuModifyMemory => 'Edit memory';

  @override
  String get messageMenuPlayVoice => 'Generate / play voice';

  @override
  String get messageMenuDeleteVariant => 'Delete current variant';

  @override
  String get messageMenuReply => 'Reply';

  @override
  String get messageMenuInsertSummary => 'Insert summary';

  @override
  String get messageMenuCreateBranch => 'Create branch';

  @override
  String get messageMenuInfo => 'Info';

  @override
  String get messageMenuMultiSelect => 'Select multiple';

  @override
  String get messageMenuDeleteConfirmTitle => 'Delete message?';

  @override
  String get messageMenuDeleteConfirmMessage => 'Delete this message?';

  @override
  String get messageMenuInfoTitle => 'Message info';

  @override
  String messageMenuSender(String value) {
    return 'Sender: $value';
  }

  @override
  String messageMenuTimestamp(String value) {
    return 'Timestamp: $value';
  }

  @override
  String messageMenuRole(String value) {
    return 'Role: $value';
  }

  @override
  String messageMenuModel(String value) {
    return 'Model: $value';
  }

  @override
  String messageMenuProvider(String value) {
    return 'Provider: $value';
  }

  @override
  String messageMenuInputTokens(String value) {
    return 'Input tokens: $value';
  }

  @override
  String messageMenuCachedInputTokens(String value) {
    return 'Cached input tokens: $value';
  }

  @override
  String messageMenuOutputTokens(String value) {
    return 'Output tokens: $value';
  }

  @override
  String messageMenuWaitDuration(String value) {
    return 'Wait time: $value ms';
  }

  @override
  String messageMenuOutputDuration(String value) {
    return 'Output time: $value ms';
  }

  @override
  String get messageMenuConfirm => 'OK';

  @override
  String get messageCopyTitle => 'Copy message';

  @override
  String get messageCopyPlainText => 'Plain text';

  @override
  String get messageCopyMarkdownSource => 'Markdown source';

  @override
  String messageCopyPlainTextConversionFailed(String error) {
    return 'Could not convert to plain text: $error';
  }

  @override
  String get messageCopyCopyPlainText => 'Copy plain text';

  @override
  String get messageCopyCopyMarkdownSource => 'Copy Markdown source';

  @override
  String messageCopyFailed(String error) {
    return 'Copy failed: $error';
  }

  @override
  String get messageCopyCompleted => 'Message copied to the clipboard';

  @override
  String get messageEditorTitle => 'Edit message';

  @override
  String get messageEditorMemoryTitle => 'Edit memory';

  @override
  String get messageEditorVisualMode => 'Visual';

  @override
  String get messageEditorPlainTextMode => 'Plain text';

  @override
  String get messageEditorSaveAndResend => 'Save and resend';

  @override
  String get messageEditorUpdateMemory => 'Update memory';

  @override
  String get messageEditorPlainTextContent => 'Plain-text content';

  @override
  String get messageEditorContentParts => 'Content parts';

  @override
  String get messageEditorAddText => 'Add text';

  @override
  String get messageEditorAddTag => 'Add tag';

  @override
  String get messageEditorTextLabel => 'Text';

  @override
  String get messageEditorTextHint => 'Enter text content';

  @override
  String get messageEditorTagTitle => 'Edit tag';

  @override
  String get messageEditorTagName => 'Tag name';

  @override
  String get messageEditorTagNameHint => 'For example, memory';

  @override
  String get messageEditorAttributes => 'Attributes (optional)';

  @override
  String get messageEditorAttributesHint => 'For example, type=\"note\"';

  @override
  String get messageEditorContent => 'Content';

  @override
  String get aboutDescription =>
      'An AI workspace for mobile and desktop with chats, workspaces, tools, plugins, MCP, remote connections, and web access.';

  @override
  String aboutVersion(String version) {
    return 'Version $version';
  }

  @override
  String get aboutProjectSection => 'Project';

  @override
  String get aboutSourceTitle => 'Source code';

  @override
  String get aboutDocumentationTitle => 'Documentation';

  @override
  String get aboutDocumentationSubtitle => 'README and command-line guide';

  @override
  String get aboutOpenSourceLicenses => 'Open-source licenses';

  @override
  String get aboutOpenSourceLicensesSubtitle => 'Operit2 uses AGPL-3.0';

  @override
  String get aboutContactSection => 'Contact';

  @override
  String get aboutMaintainer => 'Developer AAswordman';

  @override
  String get aboutCopyright => '© 2025 - 2026 Operit. All rights reserved.';

  @override
  String get chatTtsNoMatchingCharacter =>
      'No matching character is available for this message.';

  @override
  String chatTtsMatchingCharacterCount(String name) {
    return 'Expected one matching character card for $name.';
  }

  @override
  String get chatTtsEmptyMessage =>
      'This message has no content to turn into speech.';

  @override
  String chatTtsPlaybackFailed(String error) {
    return 'Could not generate or play speech: $error';
  }

  @override
  String chatCopyFailed(String error) {
    return 'Copy failed: $error';
  }

  @override
  String get chatConfirmDeleteSelectedTitle => 'Delete selected messages?';

  @override
  String chatConfirmDeleteSelectedMessage(int count) {
    return 'Delete $count selected messages?';
  }

  @override
  String get chatGeneratingShareImage => 'Creating share image...';

  @override
  String chatGenerateShareImageFailed(String error) {
    return 'Could not create share image: $error';
  }
}
