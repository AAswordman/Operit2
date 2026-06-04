import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'generated/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations? of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations);
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('zh'),
  ];

  /// No description provided for @askOperitHint.
  ///
  /// In en, this message translates to:
  /// **'Ask Operit'**
  String get askOperitHint;

  /// No description provided for @aiChat.
  ///
  /// In en, this message translates to:
  /// **'AI Chat'**
  String get aiChat;

  /// No description provided for @fullscreenInput.
  ///
  /// In en, this message translates to:
  /// **'Fullscreen input'**
  String get fullscreenInput;

  /// No description provided for @settings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settings;

  /// No description provided for @addAttachment.
  ///
  /// In en, this message translates to:
  /// **'Add attachment'**
  String get addAttachment;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @send.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get send;

  /// No description provided for @model.
  ///
  /// In en, this message translates to:
  /// **'Model'**
  String get model;

  /// No description provided for @processingInput.
  ///
  /// In en, this message translates to:
  /// **'Processing input...'**
  String get processingInput;

  /// No description provided for @processingMessage.
  ///
  /// In en, this message translates to:
  /// **'Processing message...'**
  String get processingMessage;

  /// No description provided for @connectingAiService.
  ///
  /// In en, this message translates to:
  /// **'Connecting to AI service...'**
  String get connectingAiService;

  /// No description provided for @receivingAiResponse.
  ///
  /// In en, this message translates to:
  /// **'Receiving AI response...'**
  String get receivingAiResponse;

  /// No description provided for @receivingToolResultAiResponse.
  ///
  /// In en, this message translates to:
  /// **'Receiving AI response after tool execution...'**
  String get receivingToolResultAiResponse;

  /// No description provided for @roleResponsePlannerPlanning.
  ///
  /// In en, this message translates to:
  /// **'Planning group speaking order...'**
  String get roleResponsePlannerPlanning;

  /// No description provided for @roleResponsePlannerMemberReplying.
  ///
  /// In en, this message translates to:
  /// **'Generating a reply from \"{memberName}\"...'**
  String roleResponsePlannerMemberReplying(String memberName);

  /// No description provided for @roleResponsePlannerFailed.
  ///
  /// In en, this message translates to:
  /// **'Group planning failed'**
  String get roleResponsePlannerFailed;

  /// No description provided for @summarizingMemories.
  ///
  /// In en, this message translates to:
  /// **'Summarizing memories...'**
  String get summarizingMemories;

  /// No description provided for @executingPlan.
  ///
  /// In en, this message translates to:
  /// **'Executing plan...'**
  String get executingPlan;

  /// No description provided for @executingTool.
  ///
  /// In en, this message translates to:
  /// **'Executing tool: {toolName}'**
  String executingTool(String toolName);

  /// No description provided for @processingToolResult.
  ///
  /// In en, this message translates to:
  /// **'Processing tool result: {toolName}'**
  String processingToolResult(String toolName);

  /// No description provided for @toolRunning.
  ///
  /// In en, this message translates to:
  /// **'Tool running...'**
  String get toolRunning;

  /// No description provided for @toolRunningWithName.
  ///
  /// In en, this message translates to:
  /// **'{toolName}: Tool running...'**
  String toolRunningWithName(String toolName);

  /// No description provided for @toolStatusWithName.
  ///
  /// In en, this message translates to:
  /// **'{toolName}: {message}'**
  String toolStatusWithName(String toolName, String message);

  /// No description provided for @close.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get close;

  /// No description provided for @create.
  ///
  /// In en, this message translates to:
  /// **'Create'**
  String get create;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @delete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get delete;

  /// No description provided for @search.
  ///
  /// In en, this message translates to:
  /// **'Search'**
  String get search;

  /// No description provided for @loading.
  ///
  /// In en, this message translates to:
  /// **'Loading'**
  String get loading;

  /// No description provided for @toolApprovalTitle.
  ///
  /// In en, this message translates to:
  /// **'Tool permission request'**
  String get toolApprovalTitle;

  /// No description provided for @toolApprovalToolLabel.
  ///
  /// In en, this message translates to:
  /// **'Tool'**
  String get toolApprovalToolLabel;

  /// No description provided for @toolApprovalActionLabel.
  ///
  /// In en, this message translates to:
  /// **'Action'**
  String get toolApprovalActionLabel;

  /// No description provided for @toolApprovalDeny.
  ///
  /// In en, this message translates to:
  /// **'Deny'**
  String get toolApprovalDeny;

  /// No description provided for @toolApprovalAllowOnce.
  ///
  /// In en, this message translates to:
  /// **'Allow once'**
  String get toolApprovalAllowOnce;

  /// No description provided for @toolApprovalAlwaysAllow.
  ///
  /// In en, this message translates to:
  /// **'Always allow'**
  String get toolApprovalAlwaysAllow;

  /// No description provided for @createGroupTitle.
  ///
  /// In en, this message translates to:
  /// **'New group'**
  String get createGroupTitle;

  /// No description provided for @groupNameLabel.
  ///
  /// In en, this message translates to:
  /// **'Group name'**
  String get groupNameLabel;

  /// No description provided for @renameConversationTitle.
  ///
  /// In en, this message translates to:
  /// **'Edit title'**
  String get renameConversationTitle;

  /// No description provided for @newTitleLabel.
  ///
  /// In en, this message translates to:
  /// **'New title'**
  String get newTitleLabel;

  /// No description provided for @deleteConversationTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete conversation?'**
  String get deleteConversationTitle;

  /// No description provided for @deleteConversationMessage.
  ///
  /// In en, this message translates to:
  /// **'Delete \"{title}\"?'**
  String deleteConversationMessage(String title);

  /// No description provided for @chatHistory.
  ///
  /// In en, this message translates to:
  /// **'Chat history'**
  String get chatHistory;

  /// No description provided for @editTitle.
  ///
  /// In en, this message translates to:
  /// **'Edit title'**
  String get editTitle;

  /// No description provided for @moveUp.
  ///
  /// In en, this message translates to:
  /// **'Move up'**
  String get moveUp;

  /// No description provided for @moveDown.
  ///
  /// In en, this message translates to:
  /// **'Move down'**
  String get moveDown;

  /// No description provided for @pin.
  ///
  /// In en, this message translates to:
  /// **'Pin'**
  String get pin;

  /// No description provided for @unpin.
  ///
  /// In en, this message translates to:
  /// **'Unpin'**
  String get unpin;

  /// No description provided for @lock.
  ///
  /// In en, this message translates to:
  /// **'Lock'**
  String get lock;

  /// No description provided for @unlock.
  ///
  /// In en, this message translates to:
  /// **'Unlock'**
  String get unlock;

  /// No description provided for @messageLocatorTitle.
  ///
  /// In en, this message translates to:
  /// **'Message locator'**
  String get messageLocatorTitle;

  /// No description provided for @messageLocatorCurrent.
  ///
  /// In en, this message translates to:
  /// **'Current {current} / {total}'**
  String messageLocatorCurrent(int current, int total);

  /// No description provided for @messageLocatorSearchHint.
  ///
  /// In en, this message translates to:
  /// **'Search message content'**
  String get messageLocatorSearchHint;

  /// No description provided for @messageLocatorInstruction.
  ///
  /// In en, this message translates to:
  /// **'Scroll the list or search to jump to a message'**
  String get messageLocatorInstruction;

  /// No description provided for @messageLocatorResultCount.
  ///
  /// In en, this message translates to:
  /// **'{count} results'**
  String messageLocatorResultCount(int count);

  /// No description provided for @messageLocatorNoMatches.
  ///
  /// In en, this message translates to:
  /// **'No matching messages'**
  String get messageLocatorNoMatches;

  /// No description provided for @messageSenderUser.
  ///
  /// In en, this message translates to:
  /// **'User'**
  String get messageSenderUser;

  /// No description provided for @messageSenderSummary.
  ///
  /// In en, this message translates to:
  /// **'Summary'**
  String get messageSenderSummary;

  /// No description provided for @messageSenderSystem.
  ///
  /// In en, this message translates to:
  /// **'System'**
  String get messageSenderSystem;

  /// No description provided for @messageSenderThinking.
  ///
  /// In en, this message translates to:
  /// **'Thinking'**
  String get messageSenderThinking;

  /// No description provided for @messageSenderOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get messageSenderOther;

  /// No description provided for @hiddenUserMessage.
  ///
  /// In en, this message translates to:
  /// **'Hidden user message'**
  String get hiddenUserMessage;

  /// No description provided for @workspaceSetupTitle.
  ///
  /// In en, this message translates to:
  /// **'Set up workspace'**
  String get workspaceSetupTitle;

  /// No description provided for @workspaceSetupSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Provide a dedicated file environment for your AI projects'**
  String get workspaceSetupSubtitle;

  /// No description provided for @workspaceCreateDefaultTitle.
  ///
  /// In en, this message translates to:
  /// **'Create default'**
  String get workspaceCreateDefaultTitle;

  /// No description provided for @workspaceCreateDefaultDescription.
  ///
  /// In en, this message translates to:
  /// **'Create a new workspace in the app'**
  String get workspaceCreateDefaultDescription;

  /// No description provided for @workspaceBindExistingTitle.
  ///
  /// In en, this message translates to:
  /// **'Choose existing'**
  String get workspaceBindExistingTitle;

  /// No description provided for @workspaceBindExistingDescription.
  ///
  /// In en, this message translates to:
  /// **'Select a folder from this device'**
  String get workspaceBindExistingDescription;

  /// No description provided for @workspaceProjectTypeDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Choose project type'**
  String get workspaceProjectTypeDialogTitle;

  /// No description provided for @workspaceProjectTypeDialogDescription.
  ///
  /// In en, this message translates to:
  /// **'Choose the default workspace type to create'**
  String get workspaceProjectTypeDialogDescription;

  /// No description provided for @workspaceBindDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Choose existing workspace'**
  String get workspaceBindDialogTitle;

  /// No description provided for @workspacePathLabel.
  ///
  /// In en, this message translates to:
  /// **'Selected workspace'**
  String get workspacePathLabel;

  /// No description provided for @workspaceEnvLabel.
  ///
  /// In en, this message translates to:
  /// **'Workspace environment'**
  String get workspaceEnvLabel;

  /// No description provided for @optionalHint.
  ///
  /// In en, this message translates to:
  /// **'Optional'**
  String get optionalHint;

  /// No description provided for @workspacePathRequired.
  ///
  /// In en, this message translates to:
  /// **'Select a workspace folder'**
  String get workspacePathRequired;

  /// No description provided for @bind.
  ///
  /// In en, this message translates to:
  /// **'Bind'**
  String get bind;

  /// No description provided for @workspaceProjectBlankTitle.
  ///
  /// In en, this message translates to:
  /// **'Blank workspace'**
  String get workspaceProjectBlankTitle;

  /// No description provided for @workspaceProjectBlankDescription.
  ///
  /// In en, this message translates to:
  /// **'Create an empty workspace directory without template files'**
  String get workspaceProjectBlankDescription;

  /// No description provided for @workspaceProjectOfficeTitle.
  ///
  /// In en, this message translates to:
  /// **'Office documents'**
  String get workspaceProjectOfficeTitle;

  /// No description provided for @workspaceProjectOfficeDescription.
  ///
  /// In en, this message translates to:
  /// **'For document editing, file processing, and general office tasks'**
  String get workspaceProjectOfficeDescription;

  /// No description provided for @workspaceProjectWebTitle.
  ///
  /// In en, this message translates to:
  /// **'Web project'**
  String get workspaceProjectWebTitle;

  /// No description provided for @workspaceProjectWebDescription.
  ///
  /// In en, this message translates to:
  /// **'For web development with HTML/CSS/JavaScript and an automatic local server'**
  String get workspaceProjectWebDescription;

  /// No description provided for @workspaceProjectAndroidTitle.
  ///
  /// In en, this message translates to:
  /// **'Android project'**
  String get workspaceProjectAndroidTitle;

  /// No description provided for @workspaceProjectAndroidDescription.
  ///
  /// In en, this message translates to:
  /// **'For Android engineering with common Gradle task shortcuts'**
  String get workspaceProjectAndroidDescription;

  /// No description provided for @workspaceProjectFlutterTitle.
  ///
  /// In en, this message translates to:
  /// **'Flutter project'**
  String get workspaceProjectFlutterTitle;

  /// No description provided for @workspaceProjectFlutterDescription.
  ///
  /// In en, this message translates to:
  /// **'For Flutter cross-platform development with a stable app template and common commands'**
  String get workspaceProjectFlutterDescription;

  /// No description provided for @workspaceProjectNodeTitle.
  ///
  /// In en, this message translates to:
  /// **'Node.js project'**
  String get workspaceProjectNodeTitle;

  /// No description provided for @workspaceProjectNodeDescription.
  ///
  /// In en, this message translates to:
  /// **'For Node.js backend development with npm command shortcuts'**
  String get workspaceProjectNodeDescription;

  /// No description provided for @workspaceProjectTypeScriptTitle.
  ///
  /// In en, this message translates to:
  /// **'TypeScript project'**
  String get workspaceProjectTypeScriptTitle;

  /// No description provided for @workspaceProjectTypeScriptDescription.
  ///
  /// In en, this message translates to:
  /// **'TypeScript + pnpm with type-safe development and tsc watch'**
  String get workspaceProjectTypeScriptDescription;

  /// No description provided for @workspaceProjectPythonTitle.
  ///
  /// In en, this message translates to:
  /// **'Python project'**
  String get workspaceProjectPythonTitle;

  /// No description provided for @workspaceProjectPythonDescription.
  ///
  /// In en, this message translates to:
  /// **'For Python development with pip and an HTTP server'**
  String get workspaceProjectPythonDescription;

  /// No description provided for @workspaceProjectJavaTitle.
  ///
  /// In en, this message translates to:
  /// **'Java project'**
  String get workspaceProjectJavaTitle;

  /// No description provided for @workspaceProjectJavaDescription.
  ///
  /// In en, this message translates to:
  /// **'For Java development with Gradle and Maven builds'**
  String get workspaceProjectJavaDescription;

  /// No description provided for @workspaceProjectGoTitle.
  ///
  /// In en, this message translates to:
  /// **'Go project'**
  String get workspaceProjectGoTitle;

  /// No description provided for @workspaceProjectGoDescription.
  ///
  /// In en, this message translates to:
  /// **'For Go development with go mod and build commands'**
  String get workspaceProjectGoDescription;

  /// No description provided for @version.
  ///
  /// In en, this message translates to:
  /// **'Version'**
  String get version;

  /// No description provided for @author.
  ///
  /// In en, this message translates to:
  /// **'Author'**
  String get author;

  /// No description provided for @entry.
  ///
  /// In en, this message translates to:
  /// **'Entry'**
  String get entry;

  /// No description provided for @source.
  ///
  /// In en, this message translates to:
  /// **'Source'**
  String get source;

  /// No description provided for @category.
  ///
  /// In en, this message translates to:
  /// **'Category'**
  String get category;

  /// No description provided for @defaultStatus.
  ///
  /// In en, this message translates to:
  /// **'Default status'**
  String get defaultStatus;

  /// No description provided for @builtIn.
  ///
  /// In en, this message translates to:
  /// **'Built-in'**
  String get builtIn;

  /// No description provided for @external.
  ///
  /// In en, this message translates to:
  /// **'External'**
  String get external;

  /// No description provided for @enabledByDefault.
  ///
  /// In en, this message translates to:
  /// **'Enabled by default'**
  String get enabledByDefault;

  /// No description provided for @disabledByDefault.
  ///
  /// In en, this message translates to:
  /// **'Disabled by default'**
  String get disabledByDefault;

  /// No description provided for @toolPkgResources.
  ///
  /// In en, this message translates to:
  /// **'ToolPkg resources'**
  String get toolPkgResources;

  /// No description provided for @resourcesCount.
  ///
  /// In en, this message translates to:
  /// **'Resources {count}'**
  String resourcesCount(int count);

  /// No description provided for @uiModulesCount.
  ///
  /// In en, this message translates to:
  /// **'UI modules {count}'**
  String uiModulesCount(int count);

  /// No description provided for @navigationEntriesCount.
  ///
  /// In en, this message translates to:
  /// **'Navigation entries {count}'**
  String navigationEntriesCount(int count);

  /// No description provided for @desktopWidgetsCount.
  ///
  /// In en, this message translates to:
  /// **'Desktop widgets {count}'**
  String desktopWidgetsCount(int count);

  /// No description provided for @workflowTemplatesCount.
  ///
  /// In en, this message translates to:
  /// **'Workflow templates {count}'**
  String workflowTemplatesCount(int count);

  /// No description provided for @workspaceTemplatesCount.
  ///
  /// In en, this message translates to:
  /// **'Workspace templates {count}'**
  String workspaceTemplatesCount(int count);

  /// No description provided for @pluginConfiguration.
  ///
  /// In en, this message translates to:
  /// **'Plugin configuration'**
  String get pluginConfiguration;

  /// No description provided for @subpackages.
  ///
  /// In en, this message translates to:
  /// **'Subpackages'**
  String get subpackages;

  /// No description provided for @toolPkgNoSubpackages.
  ///
  /// In en, this message translates to:
  /// **'This ToolPkg declares no subpackages'**
  String get toolPkgNoSubpackages;

  /// No description provided for @subpackageToolCount.
  ///
  /// In en, this message translates to:
  /// **'{packageName} · {count} tools'**
  String subpackageToolCount(String packageName, int count);

  /// No description provided for @workflowTemplates.
  ///
  /// In en, this message translates to:
  /// **'Workflow templates'**
  String get workflowTemplates;

  /// No description provided for @workspaceTemplates.
  ///
  /// In en, this message translates to:
  /// **'Workspace templates'**
  String get workspaceTemplates;

  /// No description provided for @disable.
  ///
  /// In en, this message translates to:
  /// **'Disable'**
  String get disable;

  /// No description provided for @enable.
  ///
  /// In en, this message translates to:
  /// **'Enable'**
  String get enable;

  /// No description provided for @environmentVariables.
  ///
  /// In en, this message translates to:
  /// **'Environment variables'**
  String get environmentVariables;

  /// No description provided for @required.
  ///
  /// In en, this message translates to:
  /// **'Required'**
  String get required;

  /// No description provided for @states.
  ///
  /// In en, this message translates to:
  /// **'States'**
  String get states;

  /// No description provided for @stateToolSummary.
  ///
  /// In en, this message translates to:
  /// **'{condition} · {toolCount} tools · excludes {excludeCount}'**
  String stateToolSummary(String condition, int toolCount, int excludeCount);

  /// No description provided for @inherit.
  ///
  /// In en, this message translates to:
  /// **'Inherit'**
  String get inherit;

  /// No description provided for @tools.
  ///
  /// In en, this message translates to:
  /// **'Tools'**
  String get tools;

  /// No description provided for @packageNoTools.
  ///
  /// In en, this message translates to:
  /// **'This package declares no tools'**
  String get packageNoTools;

  /// No description provided for @permissionsTitle.
  ///
  /// In en, this message translates to:
  /// **'Permissions'**
  String get permissionsTitle;

  /// No description provided for @clear.
  ///
  /// In en, this message translates to:
  /// **'Clear'**
  String get clear;

  /// No description provided for @noPermissionRecords.
  ///
  /// In en, this message translates to:
  /// **'No permission records yet'**
  String get noPermissionRecords;

  /// No description provided for @allow.
  ///
  /// In en, this message translates to:
  /// **'Allow'**
  String get allow;

  /// No description provided for @deny.
  ///
  /// In en, this message translates to:
  /// **'Deny'**
  String get deny;

  /// No description provided for @camera.
  ///
  /// In en, this message translates to:
  /// **'Camera'**
  String get camera;

  /// No description provided for @microphone.
  ///
  /// In en, this message translates to:
  /// **'Microphone'**
  String get microphone;

  /// No description provided for @protectedMedia.
  ///
  /// In en, this message translates to:
  /// **'Protected media'**
  String get protectedMedia;

  /// No description provided for @midiDevice.
  ///
  /// In en, this message translates to:
  /// **'MIDI device'**
  String get midiDevice;

  /// No description provided for @browserPermissionRequestTitle.
  ///
  /// In en, this message translates to:
  /// **'Website permission request'**
  String get browserPermissionRequestTitle;

  /// No description provided for @history.
  ///
  /// In en, this message translates to:
  /// **'History'**
  String get history;

  /// No description provided for @bookmarks.
  ///
  /// In en, this message translates to:
  /// **'Bookmarks'**
  String get bookmarks;

  /// No description provided for @downloads.
  ///
  /// In en, this message translates to:
  /// **'Downloads'**
  String get downloads;

  /// No description provided for @scripts.
  ///
  /// In en, this message translates to:
  /// **'Scripts'**
  String get scripts;

  /// No description provided for @zoom.
  ///
  /// In en, this message translates to:
  /// **'Zoom'**
  String get zoom;

  /// No description provided for @zoomIn.
  ///
  /// In en, this message translates to:
  /// **'Zoom in'**
  String get zoomIn;

  /// No description provided for @zoomOut.
  ///
  /// In en, this message translates to:
  /// **'Zoom out'**
  String get zoomOut;

  /// No description provided for @desktopMode.
  ///
  /// In en, this message translates to:
  /// **'Desktop mode'**
  String get desktopMode;

  /// No description provided for @clearLocalStorage.
  ///
  /// In en, this message translates to:
  /// **'Clear local storage'**
  String get clearLocalStorage;

  /// No description provided for @searchHistory.
  ///
  /// In en, this message translates to:
  /// **'Search history'**
  String get searchHistory;

  /// No description provided for @noDownloadTasks.
  ///
  /// In en, this message translates to:
  /// **'No download tasks yet'**
  String get noDownloadTasks;

  /// No description provided for @openFile.
  ///
  /// In en, this message translates to:
  /// **'Open file'**
  String get openFile;

  /// No description provided for @openLocation.
  ///
  /// In en, this message translates to:
  /// **'Open location'**
  String get openLocation;

  /// No description provided for @retry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get retry;

  /// No description provided for @removeRecord.
  ///
  /// In en, this message translates to:
  /// **'Remove record'**
  String get removeRecord;

  /// No description provided for @pending.
  ///
  /// In en, this message translates to:
  /// **'Pending'**
  String get pending;

  /// No description provided for @completed.
  ///
  /// In en, this message translates to:
  /// **'Completed'**
  String get completed;

  /// No description provided for @failed.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get failed;

  /// No description provided for @back.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get back;

  /// No description provided for @forward.
  ///
  /// In en, this message translates to:
  /// **'Forward'**
  String get forward;

  /// No description provided for @stop.
  ///
  /// In en, this message translates to:
  /// **'Stop'**
  String get stop;

  /// No description provided for @refresh.
  ///
  /// In en, this message translates to:
  /// **'Refresh'**
  String get refresh;

  /// No description provided for @home.
  ///
  /// In en, this message translates to:
  /// **'Home'**
  String get home;

  /// No description provided for @newTab.
  ///
  /// In en, this message translates to:
  /// **'New tab'**
  String get newTab;

  /// No description provided for @openExternalApplication.
  ///
  /// In en, this message translates to:
  /// **'Open external application'**
  String get openExternalApplication;

  /// No description provided for @open.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get open;

  /// No description provided for @ok.
  ///
  /// In en, this message translates to:
  /// **'OK'**
  String get ok;

  /// No description provided for @webPage.
  ///
  /// In en, this message translates to:
  /// **'Web page'**
  String get webPage;

  /// No description provided for @tabs.
  ///
  /// In en, this message translates to:
  /// **'Tabs'**
  String get tabs;

  /// No description provided for @noBookmarks.
  ///
  /// In en, this message translates to:
  /// **'No bookmarks yet'**
  String get noBookmarks;

  /// No description provided for @removeBookmark.
  ///
  /// In en, this message translates to:
  /// **'Remove bookmark'**
  String get removeBookmark;

  /// No description provided for @addBookmark.
  ///
  /// In en, this message translates to:
  /// **'Add bookmark'**
  String get addBookmark;

  /// No description provided for @menu.
  ///
  /// In en, this message translates to:
  /// **'Menu'**
  String get menu;

  /// No description provided for @siteData.
  ///
  /// In en, this message translates to:
  /// **'Site data'**
  String get siteData;

  /// No description provided for @clearAllWebViewCookies.
  ///
  /// In en, this message translates to:
  /// **'Clear all WebView cookies'**
  String get clearAllWebViewCookies;

  /// No description provided for @clearCookies.
  ///
  /// In en, this message translates to:
  /// **'Clear cookies'**
  String get clearCookies;

  /// No description provided for @noData.
  ///
  /// In en, this message translates to:
  /// **'No data'**
  String get noData;

  /// No description provided for @local.
  ///
  /// In en, this message translates to:
  /// **'Local'**
  String get local;

  /// No description provided for @pageLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Page load failed'**
  String get pageLoadFailed;

  /// No description provided for @pause.
  ///
  /// In en, this message translates to:
  /// **'Pause'**
  String get pause;

  /// No description provided for @resume.
  ///
  /// In en, this message translates to:
  /// **'Resume'**
  String get resume;

  /// No description provided for @paused.
  ///
  /// In en, this message translates to:
  /// **'Paused'**
  String get paused;

  /// No description provided for @cancelled.
  ///
  /// In en, this message translates to:
  /// **'Cancelled'**
  String get cancelled;

  /// No description provided for @downloading.
  ///
  /// In en, this message translates to:
  /// **'Downloading'**
  String get downloading;

  /// No description provided for @savedTo.
  ///
  /// In en, this message translates to:
  /// **'Saved to {path}'**
  String savedTo(String path);

  /// No description provided for @sslCertificateError.
  ///
  /// In en, this message translates to:
  /// **'SSL certificate error'**
  String get sslCertificateError;

  /// No description provided for @edit.
  ///
  /// In en, this message translates to:
  /// **'Edit'**
  String get edit;

  /// No description provided for @files.
  ///
  /// In en, this message translates to:
  /// **'Files'**
  String get files;

  /// No description provided for @terminal.
  ///
  /// In en, this message translates to:
  /// **'Terminal'**
  String get terminal;

  /// No description provided for @browser.
  ///
  /// In en, this message translates to:
  /// **'Browser'**
  String get browser;

  /// No description provided for @filePreview.
  ///
  /// In en, this message translates to:
  /// **'File preview'**
  String get filePreview;

  /// No description provided for @workspaceBoundTitle.
  ///
  /// In en, this message translates to:
  /// **'Bound workspace'**
  String get workspaceBoundTitle;

  /// No description provided for @selectFile.
  ///
  /// In en, this message translates to:
  /// **'Select file'**
  String get selectFile;

  /// No description provided for @selectFileDescription.
  ///
  /// In en, this message translates to:
  /// **'Select a file from the workspace to view, edit, or send to AI'**
  String get selectFileDescription;

  /// No description provided for @openTerminal.
  ///
  /// In en, this message translates to:
  /// **'Open terminal'**
  String get openTerminal;

  /// No description provided for @openTerminalDescription.
  ///
  /// In en, this message translates to:
  /// **'Enter the command line for the current workspace'**
  String get openTerminalDescription;

  /// No description provided for @openBrowser.
  ///
  /// In en, this message translates to:
  /// **'Open browser'**
  String get openBrowser;

  /// No description provided for @openBrowserDescription.
  ///
  /// In en, this message translates to:
  /// **'Open a full browser session, project preview, and web automation'**
  String get openBrowserDescription;

  /// No description provided for @noWorkspaceBound.
  ///
  /// In en, this message translates to:
  /// **'This conversation has no bound workspace.'**
  String get noWorkspaceBound;

  /// No description provided for @terminalSessionPlaceholder.
  ///
  /// In en, this message translates to:
  /// **'The current workspace terminal session will appear here.'**
  String get terminalSessionPlaceholder;

  /// No description provided for @emptyFolder.
  ///
  /// In en, this message translates to:
  /// **'This folder is empty'**
  String get emptyFolder;

  /// No description provided for @imagePreview.
  ///
  /// In en, this message translates to:
  /// **'Image preview'**
  String get imagePreview;

  /// No description provided for @audioPreview.
  ///
  /// In en, this message translates to:
  /// **'Audio preview'**
  String get audioPreview;

  /// No description provided for @videoPreview.
  ///
  /// In en, this message translates to:
  /// **'Video preview'**
  String get videoPreview;

  /// No description provided for @pdfPreview.
  ///
  /// In en, this message translates to:
  /// **'PDF preview'**
  String get pdfPreview;

  /// No description provided for @wordPreview.
  ///
  /// In en, this message translates to:
  /// **'Word preview'**
  String get wordPreview;

  /// No description provided for @spreadsheetPreview.
  ///
  /// In en, this message translates to:
  /// **'Spreadsheet preview'**
  String get spreadsheetPreview;

  /// No description provided for @presentationPreview.
  ///
  /// In en, this message translates to:
  /// **'Presentation preview'**
  String get presentationPreview;

  /// No description provided for @webPagePreview.
  ///
  /// In en, this message translates to:
  /// **'Web page preview'**
  String get webPagePreview;

  /// No description provided for @markdownPreview.
  ///
  /// In en, this message translates to:
  /// **'Markdown preview'**
  String get markdownPreview;

  /// No description provided for @textPreview.
  ///
  /// In en, this message translates to:
  /// **'Text preview'**
  String get textPreview;

  /// No description provided for @file.
  ///
  /// In en, this message translates to:
  /// **'File'**
  String get file;

  /// No description provided for @unsupportedReadOnlyPreview.
  ///
  /// In en, this message translates to:
  /// **'This file is not a built-in read-only preview type.'**
  String get unsupportedReadOnlyPreview;

  /// No description provided for @cannotPreview.
  ///
  /// In en, this message translates to:
  /// **'Cannot preview'**
  String get cannotPreview;

  /// No description provided for @openProjectInFullBrowser.
  ///
  /// In en, this message translates to:
  /// **'Open project in full browser'**
  String get openProjectInFullBrowser;

  /// No description provided for @openInBrowser.
  ///
  /// In en, this message translates to:
  /// **'Open in browser'**
  String get openInBrowser;

  /// No description provided for @emptySpreadsheet.
  ///
  /// In en, this message translates to:
  /// **'Spreadsheet is empty'**
  String get emptySpreadsheet;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'zh'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
