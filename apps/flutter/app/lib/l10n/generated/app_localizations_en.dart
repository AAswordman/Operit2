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
}
