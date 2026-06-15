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
  String get settings => 'Settings';

  @override
  String get packageManager => 'Package manager';

  @override
  String get market => 'Market';

  @override
  String get addAttachment => 'Add attachment';

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
  String get toolApprovalAlwaysAllow => 'Always allow';

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
  String get settingsCategoryCharactersTitle => 'Characters & Memory';

  @override
  String get settingsCategoryCharactersSubtitle => 'Cards, groups, bindings';

  @override
  String get settingsCategoryCharactersDescription =>
      'Manage character cards, groups, active roles, and role-level model, memory, and tool bindings.';

  @override
  String get settingsCategoryToolsTitle => 'Tools & Extensions';

  @override
  String get settingsCategoryToolsSubtitle =>
      'Permissions, packages, skills, MCP';

  @override
  String get settingsCategoryToolsDescription =>
      'Control which tools Operit can call, and manage plugins, tool packages, skills, and MCP servers.';

  @override
  String get settingsCategoryWorkspaceTitle => 'Workspace & Browser';

  @override
  String get settingsCategoryWorkspaceSubtitle => 'Files, terminal, browser';

  @override
  String get settingsCategoryWorkspaceDescription =>
      'Manage default workspaces, terminal sessions, browser mode, scripts, and web automation.';

  @override
  String get settingsCategoryAppearanceTitle => 'Appearance & Interaction';

  @override
  String get settingsCategoryAppearanceSubtitle => 'Theme and language';

  @override
  String get settingsCategoryAppearanceDescription =>
      'Adjust the client theme and current localization display.';

  @override
  String get settingsCategoryDataTitle => 'Data & Diagnostics';

  @override
  String get settingsCategoryDataSubtitle => 'History, backup, logs';

  @override
  String get settingsCategoryDataDescription =>
      'Manage chat history, backup and restore, token statistics, host capabilities, logs, and updates.';

  @override
  String get settingsCategoryAccessLinksTitle => 'Access & Links';

  @override
  String get settingsCategoryAccessLinksSubtitle => 'Runtime, Web Access';

  @override
  String get settingsCategoryAccessLinksDescription =>
      'Manage the runtime connection, local web access entry, bind address, and access token for this client.';

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
      'No tags available. Create tags in prompt/tag management, then bind them to character cards.';

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
  String get settingsCharactersToolAccessRequiresUsePackage =>
      'Selecting packages, skills, or MCP also requires allowing the built-in use_package tool.';

  @override
  String get settingsCharactersToolAccessEmptyBuiltin =>
      'No built-in tools available.';

  @override
  String get settingsCharactersToolAccessEmptyPackages =>
      'No packages available.';

  @override
  String get settingsCharactersToolAccessEmptySkills => 'No skills available.';

  @override
  String get settingsCharactersToolAccessEmptyMcp =>
      'No MCP servers available.';

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
  String get settingsCharactersEditUserMarkdown => 'Edit USER.md';

  @override
  String settingsCharactersUserMarkdownTitle(String profileName) {
    return '$profileName\'s USER.md';
  }

  @override
  String get settingsCharactersUserMarkdownSaved => 'USER.md saved';

  @override
  String get settingsCharactersMemoryAutoUpdate =>
      'Auto-update memory preferences';

  @override
  String get settingsCharactersMemoryAutoUpdateDescription =>
      'Allow AI to update user preferences and long-term memory from conversations.';

  @override
  String get settingsCharactersPreferenceDescription =>
      'Send user preferences to model';

  @override
  String get settingsCharactersPreferenceDescriptionSubtitle =>
      'Include the active preference profile in chat prompts.';

  @override
  String get settingsCharactersCardsSection => 'Character cards';

  @override
  String get settingsCharactersGroupsSection => 'Groups';

  @override
  String settingsCharactersGroupMembers(int count) {
    return '$count members';
  }

  @override
  String get settingsToolsPermissionMode => 'Tool permission mode';

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
  String get settingsToolsOverrides => 'Per-tool permission records';

  @override
  String get settingsToolsToolGroups => 'Per-tool permissions';

  @override
  String get settingsToolsToolGroupsDescription =>
      'Keep Ask for normal use. Put trusted tools in Always allow, and risky or unwanted tools in Always forbid.';

  @override
  String get settingsToolsAlwaysAllow => 'Always allow';

  @override
  String get settingsToolsAlwaysAllowDescription =>
      'These tools run without asking again.';

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
  String get settingsRuntimeConnection => 'Runtime connection';

  @override
  String get settingsRuntimeConnectionDescription =>
      'All core calls from this client use the runtime selected here. The Web Access entry follows the same runtime.';

  @override
  String get settingsRuntimeUseLocal => 'Use local runtime';

  @override
  String get settingsRuntimeLocalTitle => 'Local runtime';

  @override
  String get settingsRuntimeLocalDescription =>
      'Use the core built into this device. No connection details are required.';

  @override
  String get settingsRuntimeRemoteTitle => 'Remote runtime';

  @override
  String get settingsRuntimeRemoteDescription =>
      'Manage paired remote cores and switch to a specific remote from the list.';

  @override
  String get settingsRuntimeTesting => 'Testing runtime…';

  @override
  String get settingsRuntimePairRemote => 'Pair new remote';

  @override
  String get settingsRuntimeNoPairedRemote => 'No paired remote core yet.';

  @override
  String get settingsRuntimePairToken => 'Pairing token';

  @override
  String get settingsRuntimePairCode => 'Pairing code';

  @override
  String get settingsRuntimeStartPairing => 'Start pairing';

  @override
  String get settingsRuntimeFinishPairing => 'Finish pairing';

  @override
  String get settingsRuntimeBaseUrl => 'Remote address';

  @override
  String get settingsRuntimeTestCurrent => 'Test current runtime';

  @override
  String get settingsRuntimeSwitchedLocal => 'Switched to local runtime.';

  @override
  String get settingsRuntimeSwitchedRemote => 'Switched to remote runtime.';

  @override
  String settingsRuntimeTestResult(String version) {
    return 'Runtime available: $version';
  }

  @override
  String settingsRuntimeTestFailed(String error) {
    return 'Runtime test failed: $error';
  }

  @override
  String get settingsRuntimeRemoteDisconnected => 'Remote runtime disconnected';

  @override
  String settingsRuntimeRemoteDisconnectedMessage(String error) {
    return 'Switched to local core.\n\n$error';
  }

  @override
  String get settingsWebAccessService => 'Web access service';

  @override
  String get settingsWebAccessServiceDescription =>
      'When enabled, this client starts a local HTTP entry. Browsers can access core with the token.';

  @override
  String get settingsWebAccessEnable => 'Enable web access';

  @override
  String get settingsWebAccessBindAddress => 'Bind address';

  @override
  String get settingsWebAccessToken => 'Access token';

  @override
  String get settingsWebAccessRotateToken => 'Rotate token';

  @override
  String get settingsWebAccessCopyToken => 'Copy token';

  @override
  String get settingsWebAccessAccessUrl => 'Address';

  @override
  String get settingsWebAccessLocalUrl => 'Local';

  @override
  String get settingsWebAccessPairingUrl => 'Pairing';

  @override
  String get settingsWebAccessPairingUrlLocalOnly => 'Local only';

  @override
  String get settingsWebAccessPairingUrlUnavailable => 'No LAN address found';

  @override
  String get settingsWebAccessCopyUrl => 'Copy URL';

  @override
  String get settingsWebAccessOpenUrl => 'Open';

  @override
  String get settingsWebAccessRunning => 'Running';

  @override
  String get settingsWebAccessStopped => 'Stopped';

  @override
  String get settingsWebAccessSaved => 'Web access settings saved.';

  @override
  String get settingsWebAccessTokenCopied => 'Access token copied.';

  @override
  String get settingsWebAccessUrlCopied => 'Access URL copied.';

  @override
  String get settingsWebAccessPairedClients => 'Paired clients';

  @override
  String get settingsWebAccessNoPairedClients =>
      'No client has paired with this device yet.';

  @override
  String get settingsWebAccessPairedDeleted => 'Paired client deleted.';

  @override
  String get settingsWebAccessPairingRequest => 'Remote pairing request';

  @override
  String settingsWebAccessPairingRequestMessage(String code, String client) {
    return 'Pairing code: $code\nClient: $client';
  }

  @override
  String get settingsWebAccessInvalidBindAddress =>
      'Bind address must be host:port.';

  @override
  String settingsWebAccessStartFailed(String error) {
    return 'Failed to start web access: $error';
  }

  @override
  String settingsWebAccessStopFailed(String error) {
    return 'Failed to stop web access: $error';
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
  String get settingsDataRuntimeSection => 'Runtime';

  @override
  String get settingsDataCoreVersion => 'Core version';

  @override
  String get settingsDataTokenSection => 'Token statistics';

  @override
  String get settingsDataInputTokens => 'Input tokens';

  @override
  String get settingsDataOutputTokens => 'Output tokens';

  @override
  String get settingsDataRefreshTokenStats => 'Refresh cumulative statistics';

  @override
  String get settingsDataResetTokenStats => 'Reset token statistics';

  @override
  String get settingsDataBackupSection => 'Backup';

  @override
  String get settingsDataChatHistoriesBackup => 'Chat history backup';

  @override
  String get settingsDataChatHistoriesBackupDescription =>
      'Copy all chats and messages as JSON. Import updates or creates chats by chat ID.';

  @override
  String get settingsDataCharacterCardsBackup => 'Character card backup';

  @override
  String get settingsDataCharacterCardsBackupDescription =>
      'Copy all character cards and referenced tags as JSON. Import updates or creates items by original ID.';

  @override
  String get settingsDataCharacterGroupsBackup => 'Group backup';

  @override
  String get settingsDataCharacterGroupsBackupDescription =>
      'Copy all groups as JSON. Import keeps member references and ordering.';

  @override
  String get settingsDataModelConfigsBackup => 'Model config backup';

  @override
  String get settingsDataModelConfigsBackupDescription =>
      'Copy all model configs as JSON. Import updates or creates items by config ID, including model parameters and API key pools.';

  @override
  String settingsDataBackupCount(int count) {
    return '$count items';
  }

  @override
  String get settingsDataCopyBackupJson => 'Copy backup JSON';

  @override
  String get settingsDataImportBackupJson => 'Import backup JSON';

  @override
  String get settingsDataBackupJsonInput => 'Backup JSON content';

  @override
  String settingsDataBackupCopied(String name) {
    return 'Copied backup JSON for “$name”.';
  }

  @override
  String settingsDataBackupImportResult(
    int newCount,
    int updatedCount,
    int skippedCount,
  ) {
    return 'Import complete: $newCount new, $updatedCount updated, $skippedCount skipped.';
  }

  @override
  String settingsDataBackupImportError(String error) {
    return 'Backup import failed: $error';
  }

  @override
  String settingsDataBackupCopyError(String error) {
    return 'Backup copy failed: $error';
  }

  @override
  String get settingsDataExportRawSnapshot => 'Export raw snapshot';

  @override
  String get settingsDataExportRawSnapshotDescription =>
      'Generate the current data snapshot from runtime and show its byte size.';

  @override
  String settingsDataSnapshotBytes(int bytes) {
    return 'Snapshot generated: $bytes bytes';
  }
}
