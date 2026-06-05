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

  /// No description provided for @packageManager.
  ///
  /// In en, this message translates to:
  /// **'Package manager'**
  String get packageManager;

  /// No description provided for @market.
  ///
  /// In en, this message translates to:
  /// **'Market'**
  String get market;

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

  /// No description provided for @settingsCategoryModelTitle.
  ///
  /// In en, this message translates to:
  /// **'Models & AI'**
  String get settingsCategoryModelTitle;

  /// No description provided for @settingsCategoryModelSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Models, keys, context'**
  String get settingsCategoryModelSubtitle;

  /// No description provided for @settingsCategoryModelDescription.
  ///
  /// In en, this message translates to:
  /// **'Configure model connections, choose the chat model, and manage thinking, context, and multimodal abilities.'**
  String get settingsCategoryModelDescription;

  /// No description provided for @settingsCategoryCharactersTitle.
  ///
  /// In en, this message translates to:
  /// **'Characters & Memory'**
  String get settingsCategoryCharactersTitle;

  /// No description provided for @settingsCategoryCharactersSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Cards, groups, bindings'**
  String get settingsCategoryCharactersSubtitle;

  /// No description provided for @settingsCategoryCharactersDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage character cards, character groups, active roles, and role-level model, memory, and tool bindings.'**
  String get settingsCategoryCharactersDescription;

  /// No description provided for @settingsCategoryToolsTitle.
  ///
  /// In en, this message translates to:
  /// **'Tools & Extensions'**
  String get settingsCategoryToolsTitle;

  /// No description provided for @settingsCategoryToolsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Permissions, packages, skills, MCP'**
  String get settingsCategoryToolsSubtitle;

  /// No description provided for @settingsCategoryToolsDescription.
  ///
  /// In en, this message translates to:
  /// **'Control which tools Operit can call, and manage plugins, tool packages, skills, and MCP servers.'**
  String get settingsCategoryToolsDescription;

  /// No description provided for @settingsCategoryWorkspaceTitle.
  ///
  /// In en, this message translates to:
  /// **'Workspace & Browser'**
  String get settingsCategoryWorkspaceTitle;

  /// No description provided for @settingsCategoryWorkspaceSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Files, terminal, browser'**
  String get settingsCategoryWorkspaceSubtitle;

  /// No description provided for @settingsCategoryWorkspaceDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage default workspaces, terminal sessions, browser mode, scripts, and web automation.'**
  String get settingsCategoryWorkspaceDescription;

  /// No description provided for @settingsCategoryAppearanceTitle.
  ///
  /// In en, this message translates to:
  /// **'Appearance & Interaction'**
  String get settingsCategoryAppearanceTitle;

  /// No description provided for @settingsCategoryAppearanceSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Theme and language'**
  String get settingsCategoryAppearanceSubtitle;

  /// No description provided for @settingsCategoryAppearanceDescription.
  ///
  /// In en, this message translates to:
  /// **'Adjust the client theme and current localization display.'**
  String get settingsCategoryAppearanceDescription;

  /// No description provided for @settingsCategoryDataTitle.
  ///
  /// In en, this message translates to:
  /// **'Data & Diagnostics'**
  String get settingsCategoryDataTitle;

  /// No description provided for @settingsCategoryDataSubtitle.
  ///
  /// In en, this message translates to:
  /// **'History, backup, logs'**
  String get settingsCategoryDataSubtitle;

  /// No description provided for @settingsCategoryDataDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage chat history, backup and restore, token statistics, host capabilities, logs, and updates.'**
  String get settingsCategoryDataDescription;

  /// No description provided for @settingsComingSoon.
  ///
  /// In en, this message translates to:
  /// **'This area will continue connecting existing runtime capabilities. Models, characters, and tools are being completed first.'**
  String get settingsComingSoon;

  /// No description provided for @settingsAdvanced.
  ///
  /// In en, this message translates to:
  /// **'Advanced settings'**
  String get settingsAdvanced;

  /// No description provided for @settingsActive.
  ///
  /// In en, this message translates to:
  /// **'Active'**
  String get settingsActive;

  /// No description provided for @settingsActivate.
  ///
  /// In en, this message translates to:
  /// **'Activate'**
  String get settingsActivate;

  /// No description provided for @settingsModelCurrentSection.
  ///
  /// In en, this message translates to:
  /// **'Current chat model'**
  String get settingsModelCurrentSection;

  /// No description provided for @settingsModelCurrentChatModel.
  ///
  /// In en, this message translates to:
  /// **'Chat uses'**
  String get settingsModelCurrentChatModel;

  /// No description provided for @settingsChatThinkingMode.
  ///
  /// In en, this message translates to:
  /// **'Thinking mode'**
  String get settingsChatThinkingMode;

  /// No description provided for @settingsChatThinkingModeDescription.
  ///
  /// In en, this message translates to:
  /// **'Let supported models produce steadier reasoning.'**
  String get settingsChatThinkingModeDescription;

  /// No description provided for @settingsChatStreamOutput.
  ///
  /// In en, this message translates to:
  /// **'Stream output'**
  String get settingsChatStreamOutput;

  /// No description provided for @settingsChatStreamOutputDescription.
  ///
  /// In en, this message translates to:
  /// **'Show generated replies progressively.'**
  String get settingsChatStreamOutputDescription;

  /// No description provided for @settingsModelProfilesSection.
  ///
  /// In en, this message translates to:
  /// **'Model profiles'**
  String get settingsModelProfilesSection;

  /// No description provided for @settingsModelFunctionMappingsSection.
  ///
  /// In en, this message translates to:
  /// **'Function model assignment'**
  String get settingsModelFunctionMappingsSection;

  /// No description provided for @settingsModelFunctionMappingsDescription.
  ///
  /// In en, this message translates to:
  /// **'Choose the model profile and concrete model used by chat, summary, memory, image recognition, and other functions.'**
  String get settingsModelFunctionMappingsDescription;

  /// No description provided for @settingsModelFunctionMappingsReset.
  ///
  /// In en, this message translates to:
  /// **'Reset all'**
  String get settingsModelFunctionMappingsReset;

  /// No description provided for @settingsModelFunctionMappingsChange.
  ///
  /// In en, this message translates to:
  /// **'Change'**
  String get settingsModelFunctionMappingsChange;

  /// No description provided for @settingsModelFunctionMappingsSelect.
  ///
  /// In en, this message translates to:
  /// **'Select {name} model'**
  String settingsModelFunctionMappingsSelect(String name);

  /// No description provided for @settingsModelFunctionMappingsCurrent.
  ///
  /// In en, this message translates to:
  /// **'{configName} · {modelName}'**
  String settingsModelFunctionMappingsCurrent(
    String configName,
    String modelName,
  );

  /// No description provided for @settingsModelChatAutoGlmWarning.
  ///
  /// In en, this message translates to:
  /// **'AutoGLM cannot be used as the main chat model. Chat and UI control use separate model assignments; choose another large model.'**
  String get settingsModelChatAutoGlmWarning;

  /// No description provided for @settingsModelFunctionChat.
  ///
  /// In en, this message translates to:
  /// **'Chat'**
  String get settingsModelFunctionChat;

  /// No description provided for @settingsModelFunctionChatDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used for main conversation replies.'**
  String get settingsModelFunctionChatDescription;

  /// No description provided for @settingsModelFunctionSummary.
  ///
  /// In en, this message translates to:
  /// **'Summary'**
  String get settingsModelFunctionSummary;

  /// No description provided for @settingsModelFunctionSummaryDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used for long-context automatic summaries.'**
  String get settingsModelFunctionSummaryDescription;

  /// No description provided for @settingsModelFunctionMemory.
  ///
  /// In en, this message translates to:
  /// **'Memory'**
  String get settingsModelFunctionMemory;

  /// No description provided for @settingsModelFunctionMemoryDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used to extract, organize, and update memories.'**
  String get settingsModelFunctionMemoryDescription;

  /// No description provided for @settingsModelFunctionUiController.
  ///
  /// In en, this message translates to:
  /// **'UI control'**
  String get settingsModelFunctionUiController;

  /// No description provided for @settingsModelFunctionUiControllerDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used for interface control and lightweight action planning.'**
  String get settingsModelFunctionUiControllerDescription;

  /// No description provided for @settingsModelFunctionTranslation.
  ///
  /// In en, this message translates to:
  /// **'Translation'**
  String get settingsModelFunctionTranslation;

  /// No description provided for @settingsModelFunctionTranslationDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used to translate text and localized content.'**
  String get settingsModelFunctionTranslationDescription;

  /// No description provided for @settingsModelFunctionGrep.
  ///
  /// In en, this message translates to:
  /// **'Text search'**
  String get settingsModelFunctionGrep;

  /// No description provided for @settingsModelFunctionGrepDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used to filter search results and judge text matches.'**
  String get settingsModelFunctionGrepDescription;

  /// No description provided for @settingsModelFunctionRoleResponsePlanner.
  ///
  /// In en, this message translates to:
  /// **'Group reply planner'**
  String get settingsModelFunctionRoleResponsePlanner;

  /// No description provided for @settingsModelFunctionRoleResponsePlannerDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used to plan speaking roles and order in character group conversations.'**
  String get settingsModelFunctionRoleResponsePlannerDescription;

  /// No description provided for @settingsModelFunctionImageRecognition.
  ///
  /// In en, this message translates to:
  /// **'Image recognition'**
  String get settingsModelFunctionImageRecognition;

  /// No description provided for @settingsModelFunctionImageRecognitionDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used to understand images and extract image content.'**
  String get settingsModelFunctionImageRecognitionDescription;

  /// No description provided for @settingsModelFunctionAudioRecognition.
  ///
  /// In en, this message translates to:
  /// **'Audio recognition'**
  String get settingsModelFunctionAudioRecognition;

  /// No description provided for @settingsModelFunctionAudioRecognitionDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used to understand audio and extract audio content.'**
  String get settingsModelFunctionAudioRecognitionDescription;

  /// No description provided for @settingsModelFunctionVideoRecognition.
  ///
  /// In en, this message translates to:
  /// **'Video recognition'**
  String get settingsModelFunctionVideoRecognition;

  /// No description provided for @settingsModelFunctionVideoRecognitionDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used to understand video and extract video content.'**
  String get settingsModelFunctionVideoRecognitionDescription;

  /// No description provided for @settingsModelFunctionImageUnsupported.
  ///
  /// In en, this message translates to:
  /// **'The selected model profile has direct image input disabled.'**
  String get settingsModelFunctionImageUnsupported;

  /// No description provided for @settingsModelFunctionAudioUnsupported.
  ///
  /// In en, this message translates to:
  /// **'The selected model profile has direct audio input disabled.'**
  String get settingsModelFunctionAudioUnsupported;

  /// No description provided for @settingsModelFunctionVideoUnsupported.
  ///
  /// In en, this message translates to:
  /// **'The selected model profile has direct video input disabled.'**
  String get settingsModelFunctionVideoUnsupported;

  /// No description provided for @settingsModelCreateProfile.
  ///
  /// In en, this message translates to:
  /// **'New model profile'**
  String get settingsModelCreateProfile;

  /// No description provided for @settingsModelEditProfile.
  ///
  /// In en, this message translates to:
  /// **'Edit model profile'**
  String get settingsModelEditProfile;

  /// No description provided for @settingsModelProfileName.
  ///
  /// In en, this message translates to:
  /// **'Profile name'**
  String get settingsModelProfileName;

  /// No description provided for @settingsModelApiEndpoint.
  ///
  /// In en, this message translates to:
  /// **'API endpoint'**
  String get settingsModelApiEndpoint;

  /// No description provided for @settingsModelModelNames.
  ///
  /// In en, this message translates to:
  /// **'Model names'**
  String get settingsModelModelNames;

  /// No description provided for @settingsModelApiKey.
  ///
  /// In en, this message translates to:
  /// **'API key'**
  String get settingsModelApiKey;

  /// No description provided for @settingsModelApiKeyPool.
  ///
  /// In en, this message translates to:
  /// **'API key pool'**
  String get settingsModelApiKeyPool;

  /// No description provided for @settingsModelApiKeyPoolDescription.
  ///
  /// In en, this message translates to:
  /// **'Prepare multiple keys for one model profile so runtime can rotate between them.'**
  String get settingsModelApiKeyPoolDescription;

  /// No description provided for @settingsModelApiKeyPoolCount.
  ///
  /// In en, this message translates to:
  /// **'{count} keys'**
  String settingsModelApiKeyPoolCount(int count);

  /// No description provided for @settingsModelApiKeyPoolEmpty.
  ///
  /// In en, this message translates to:
  /// **'No keys yet. Add keys and this profile will use the key pool.'**
  String get settingsModelApiKeyPoolEmpty;

  /// No description provided for @settingsModelAddApiKey.
  ///
  /// In en, this message translates to:
  /// **'Add key'**
  String get settingsModelAddApiKey;

  /// No description provided for @settingsModelEditApiKey.
  ///
  /// In en, this message translates to:
  /// **'Edit key'**
  String get settingsModelEditApiKey;

  /// No description provided for @settingsModelApiKeyName.
  ///
  /// In en, this message translates to:
  /// **'Key name'**
  String get settingsModelApiKeyName;

  /// No description provided for @settingsModelApiKeyEnabled.
  ///
  /// In en, this message translates to:
  /// **'Enable this key'**
  String get settingsModelApiKeyEnabled;

  /// No description provided for @settingsModelProviderId.
  ///
  /// In en, this message translates to:
  /// **'Provider ID'**
  String get settingsModelProviderId;

  /// No description provided for @settingsModelMaxTokens.
  ///
  /// In en, this message translates to:
  /// **'Max tokens'**
  String get settingsModelMaxTokens;

  /// No description provided for @settingsModelMaxTokensDescription.
  ///
  /// In en, this message translates to:
  /// **'Limit how many tokens one response may generate.'**
  String get settingsModelMaxTokensDescription;

  /// No description provided for @settingsModelTemperature.
  ///
  /// In en, this message translates to:
  /// **'Temperature'**
  String get settingsModelTemperature;

  /// No description provided for @settingsModelTemperatureDescription.
  ///
  /// In en, this message translates to:
  /// **'Controls randomness. Lower is steadier, higher is more varied.'**
  String get settingsModelTemperatureDescription;

  /// No description provided for @settingsModelTopP.
  ///
  /// In en, this message translates to:
  /// **'Top-p'**
  String get settingsModelTopP;

  /// No description provided for @settingsModelTopPDescription.
  ///
  /// In en, this message translates to:
  /// **'Sample only from the cumulative Top-p probability range.'**
  String get settingsModelTopPDescription;

  /// No description provided for @settingsModelTopK.
  ///
  /// In en, this message translates to:
  /// **'Top-k'**
  String get settingsModelTopK;

  /// No description provided for @settingsModelTopKDescription.
  ///
  /// In en, this message translates to:
  /// **'Sample from the K most likely candidate tokens. 0 disables it.'**
  String get settingsModelTopKDescription;

  /// No description provided for @settingsModelPresencePenalty.
  ///
  /// In en, this message translates to:
  /// **'Presence penalty'**
  String get settingsModelPresencePenalty;

  /// No description provided for @settingsModelPresencePenaltyDescription.
  ///
  /// In en, this message translates to:
  /// **'Encourages new topics and reduces reuse of existing content.'**
  String get settingsModelPresencePenaltyDescription;

  /// No description provided for @settingsModelFrequencyPenalty.
  ///
  /// In en, this message translates to:
  /// **'Frequency penalty'**
  String get settingsModelFrequencyPenalty;

  /// No description provided for @settingsModelFrequencyPenaltyDescription.
  ///
  /// In en, this message translates to:
  /// **'Penalizes repeated tokens by frequency.'**
  String get settingsModelFrequencyPenaltyDescription;

  /// No description provided for @settingsModelRepetitionPenalty.
  ///
  /// In en, this message translates to:
  /// **'Repetition penalty'**
  String get settingsModelRepetitionPenalty;

  /// No description provided for @settingsModelRepetitionPenaltyDescription.
  ///
  /// In en, this message translates to:
  /// **'Further reduces repeated output. 1.0 means no penalty.'**
  String get settingsModelRepetitionPenaltyDescription;

  /// No description provided for @settingsModelRequestLimit.
  ///
  /// In en, this message translates to:
  /// **'Requests per minute'**
  String get settingsModelRequestLimit;

  /// No description provided for @settingsModelMaxConcurrent.
  ///
  /// In en, this message translates to:
  /// **'Max concurrent requests'**
  String get settingsModelMaxConcurrent;

  /// No description provided for @settingsModelContextLength.
  ///
  /// In en, this message translates to:
  /// **'Context length'**
  String get settingsModelContextLength;

  /// No description provided for @settingsModelMaxContextLength.
  ///
  /// In en, this message translates to:
  /// **'Max context length'**
  String get settingsModelMaxContextLength;

  /// No description provided for @settingsModelMaxContextMode.
  ///
  /// In en, this message translates to:
  /// **'Max context mode'**
  String get settingsModelMaxContextMode;

  /// No description provided for @settingsModelSummaryThreshold.
  ///
  /// In en, this message translates to:
  /// **'Summary token threshold'**
  String get settingsModelSummaryThreshold;

  /// No description provided for @settingsModelSummaryByMessageCount.
  ///
  /// In en, this message translates to:
  /// **'Summarize by message count'**
  String get settingsModelSummaryByMessageCount;

  /// No description provided for @settingsModelSummaryMessageCount.
  ///
  /// In en, this message translates to:
  /// **'Summary message threshold'**
  String get settingsModelSummaryMessageCount;

  /// No description provided for @settingsModelCustomHeaders.
  ///
  /// In en, this message translates to:
  /// **'Custom headers'**
  String get settingsModelCustomHeaders;

  /// No description provided for @settingsModelCustomParameters.
  ///
  /// In en, this message translates to:
  /// **'Custom parameters JSON'**
  String get settingsModelCustomParameters;

  /// No description provided for @settingsModelToolCall.
  ///
  /// In en, this message translates to:
  /// **'Tool calling'**
  String get settingsModelToolCall;

  /// No description provided for @settingsModelToolCallDescription.
  ///
  /// In en, this message translates to:
  /// **'Allow the model to use structured tool calls.'**
  String get settingsModelToolCallDescription;

  /// No description provided for @settingsModelDirectImage.
  ///
  /// In en, this message translates to:
  /// **'Direct image input'**
  String get settingsModelDirectImage;

  /// No description provided for @settingsModelDirectImageDescription.
  ///
  /// In en, this message translates to:
  /// **'Send images directly to models that support image input.'**
  String get settingsModelDirectImageDescription;

  /// No description provided for @settingsModelDirectAudio.
  ///
  /// In en, this message translates to:
  /// **'Direct audio input'**
  String get settingsModelDirectAudio;

  /// No description provided for @settingsModelDirectAudioDescription.
  ///
  /// In en, this message translates to:
  /// **'Send audio directly to models that support audio input.'**
  String get settingsModelDirectAudioDescription;

  /// No description provided for @settingsModelDirectVideo.
  ///
  /// In en, this message translates to:
  /// **'Direct video input'**
  String get settingsModelDirectVideo;

  /// No description provided for @settingsModelDirectVideoDescription.
  ///
  /// In en, this message translates to:
  /// **'Send video directly to models that support video input.'**
  String get settingsModelDirectVideoDescription;

  /// No description provided for @settingsModelGoogleSearch.
  ///
  /// In en, this message translates to:
  /// **'Google Search'**
  String get settingsModelGoogleSearch;

  /// No description provided for @settingsModelGoogleSearchDescription.
  ///
  /// In en, this message translates to:
  /// **'Enable provider-side search capability.'**
  String get settingsModelGoogleSearchDescription;

  /// No description provided for @settingsModelContext.
  ///
  /// In en, this message translates to:
  /// **'Context window'**
  String get settingsModelContext;

  /// No description provided for @settingsModelSummary.
  ///
  /// In en, this message translates to:
  /// **'Auto summary'**
  String get settingsModelSummary;

  /// No description provided for @settingsModelMediaHistory.
  ///
  /// In en, this message translates to:
  /// **'Media history'**
  String get settingsModelMediaHistory;

  /// No description provided for @settingsModelConnectionTestSection.
  ///
  /// In en, this message translates to:
  /// **'Connection test'**
  String get settingsModelConnectionTestSection;

  /// No description provided for @settingsModelRunConnectionTest.
  ///
  /// In en, this message translates to:
  /// **'Test current model'**
  String get settingsModelRunConnectionTest;

  /// No description provided for @settingsModelTestingConnection.
  ///
  /// In en, this message translates to:
  /// **'Testing current model connection…'**
  String get settingsModelTestingConnection;

  /// No description provided for @settingsModelTestedModel.
  ///
  /// In en, this message translates to:
  /// **'Tested model'**
  String get settingsModelTestedModel;

  /// No description provided for @settingsModelConnectionTestPassed.
  ///
  /// In en, this message translates to:
  /// **'All checks passed'**
  String get settingsModelConnectionTestPassed;

  /// No description provided for @settingsModelConnectionTestFailed.
  ///
  /// In en, this message translates to:
  /// **'Some checks failed'**
  String get settingsModelConnectionTestFailed;

  /// No description provided for @settingsModelCapabilitiesApplied.
  ///
  /// In en, this message translates to:
  /// **'Model capability switches were updated from the test result.'**
  String get settingsModelCapabilitiesApplied;

  /// No description provided for @settingsModelCapabilitiesNeedChat.
  ///
  /// In en, this message translates to:
  /// **'Chat test did not pass, so model capability switches were not updated.'**
  String get settingsModelCapabilitiesNeedChat;

  /// No description provided for @settingsModelConnectionTestError.
  ///
  /// In en, this message translates to:
  /// **'Connection test failed: {error}'**
  String settingsModelConnectionTestError(String error);

  /// No description provided for @settingsModelTestItemChat.
  ///
  /// In en, this message translates to:
  /// **'Chat'**
  String get settingsModelTestItemChat;

  /// No description provided for @settingsModelTestItemToolCall.
  ///
  /// In en, this message translates to:
  /// **'Tool call'**
  String get settingsModelTestItemToolCall;

  /// No description provided for @settingsModelTestItemImage.
  ///
  /// In en, this message translates to:
  /// **'Image'**
  String get settingsModelTestItemImage;

  /// No description provided for @settingsModelTestItemAudio.
  ///
  /// In en, this message translates to:
  /// **'Audio'**
  String get settingsModelTestItemAudio;

  /// No description provided for @settingsModelTestItemVideo.
  ///
  /// In en, this message translates to:
  /// **'Video'**
  String get settingsModelTestItemVideo;

  /// No description provided for @settingsModelTestItemUnknown.
  ///
  /// In en, this message translates to:
  /// **'Unknown item'**
  String get settingsModelTestItemUnknown;

  /// No description provided for @settingsCharactersCreateCard.
  ///
  /// In en, this message translates to:
  /// **'New character card'**
  String get settingsCharactersCreateCard;

  /// No description provided for @settingsCharactersEditCard.
  ///
  /// In en, this message translates to:
  /// **'Edit character card'**
  String get settingsCharactersEditCard;

  /// No description provided for @settingsCharactersCardName.
  ///
  /// In en, this message translates to:
  /// **'Character name'**
  String get settingsCharactersCardName;

  /// No description provided for @settingsCharactersCreateGroup.
  ///
  /// In en, this message translates to:
  /// **'New character group'**
  String get settingsCharactersCreateGroup;

  /// No description provided for @settingsCharactersEditGroup.
  ///
  /// In en, this message translates to:
  /// **'Edit character group'**
  String get settingsCharactersEditGroup;

  /// No description provided for @settingsCharactersGroupName.
  ///
  /// In en, this message translates to:
  /// **'Group name'**
  String get settingsCharactersGroupName;

  /// No description provided for @settingsCharactersDescription.
  ///
  /// In en, this message translates to:
  /// **'Description'**
  String get settingsCharactersDescription;

  /// No description provided for @settingsCharactersCharacterSetting.
  ///
  /// In en, this message translates to:
  /// **'Character setting'**
  String get settingsCharactersCharacterSetting;

  /// No description provided for @settingsCharactersOpeningStatement.
  ///
  /// In en, this message translates to:
  /// **'Opening statement'**
  String get settingsCharactersOpeningStatement;

  /// No description provided for @settingsCharactersOtherContentChat.
  ///
  /// In en, this message translates to:
  /// **'Extra chat content'**
  String get settingsCharactersOtherContentChat;

  /// No description provided for @settingsCharactersOtherContentVoice.
  ///
  /// In en, this message translates to:
  /// **'Extra voice content'**
  String get settingsCharactersOtherContentVoice;

  /// No description provided for @settingsCharactersAdvancedPrompt.
  ///
  /// In en, this message translates to:
  /// **'Advanced custom prompt'**
  String get settingsCharactersAdvancedPrompt;

  /// No description provided for @settingsCharactersMarks.
  ///
  /// In en, this message translates to:
  /// **'Notes'**
  String get settingsCharactersMarks;

  /// No description provided for @settingsCharactersTags.
  ///
  /// In en, this message translates to:
  /// **'Tags'**
  String get settingsCharactersTags;

  /// No description provided for @settingsCharactersNoTags.
  ///
  /// In en, this message translates to:
  /// **'No tags available. Create tags in prompt/tag management, then bind them to character cards.'**
  String get settingsCharactersNoTags;

  /// No description provided for @settingsCharactersImportJson.
  ///
  /// In en, this message translates to:
  /// **'Import JSON'**
  String get settingsCharactersImportJson;

  /// No description provided for @settingsCharactersCopyJson.
  ///
  /// In en, this message translates to:
  /// **'Copy JSON'**
  String get settingsCharactersCopyJson;

  /// No description provided for @settingsCharactersImportTavernJson.
  ///
  /// In en, this message translates to:
  /// **'Import Tavern JSON'**
  String get settingsCharactersImportTavernJson;

  /// No description provided for @settingsCharactersCopyTavernJson.
  ///
  /// In en, this message translates to:
  /// **'Copy Tavern JSON'**
  String get settingsCharactersCopyTavernJson;

  /// No description provided for @settingsCharactersJsonInput.
  ///
  /// In en, this message translates to:
  /// **'JSON content'**
  String get settingsCharactersJsonInput;

  /// No description provided for @settingsCharactersTavernJsonInput.
  ///
  /// In en, this message translates to:
  /// **'Tavern JSON content'**
  String get settingsCharactersTavernJsonInput;

  /// No description provided for @settingsCharactersJsonCopied.
  ///
  /// In en, this message translates to:
  /// **'Copied JSON for “{name}”.'**
  String settingsCharactersJsonCopied(String name);

  /// No description provided for @settingsCharactersTavernJsonCopied.
  ///
  /// In en, this message translates to:
  /// **'Copied Tavern JSON for “{name}”.'**
  String settingsCharactersTavernJsonCopied(String name);

  /// No description provided for @settingsCharactersImportCardJson.
  ///
  /// In en, this message translates to:
  /// **'Import character card JSON'**
  String get settingsCharactersImportCardJson;

  /// No description provided for @settingsCharactersImportCardJsonDone.
  ///
  /// In en, this message translates to:
  /// **'Character card imported.'**
  String get settingsCharactersImportCardJsonDone;

  /// No description provided for @settingsCharactersImportTavernJsonDone.
  ///
  /// In en, this message translates to:
  /// **'Tavern character card imported.'**
  String get settingsCharactersImportTavernJsonDone;

  /// No description provided for @settingsCharactersImportGroupJson.
  ///
  /// In en, this message translates to:
  /// **'Import character group JSON'**
  String get settingsCharactersImportGroupJson;

  /// No description provided for @settingsCharactersImportGroupJsonDone.
  ///
  /// In en, this message translates to:
  /// **'Character group imported.'**
  String get settingsCharactersImportGroupJsonDone;

  /// No description provided for @settingsCharactersImportJsonError.
  ///
  /// In en, this message translates to:
  /// **'JSON import failed: {error}'**
  String settingsCharactersImportJsonError(String error);

  /// No description provided for @settingsCharactersImportTavernJsonError.
  ///
  /// In en, this message translates to:
  /// **'Tavern JSON import failed: {error}'**
  String settingsCharactersImportTavernJsonError(String error);

  /// No description provided for @settingsCharactersTavernJsonCopyError.
  ///
  /// In en, this message translates to:
  /// **'Tavern JSON copy failed: {error}'**
  String settingsCharactersTavernJsonCopyError(String error);

  /// No description provided for @settingsCharactersTagsSection.
  ///
  /// In en, this message translates to:
  /// **'Tags'**
  String get settingsCharactersTagsSection;

  /// No description provided for @settingsCharactersCreateTag.
  ///
  /// In en, this message translates to:
  /// **'New tag'**
  String get settingsCharactersCreateTag;

  /// No description provided for @settingsCharactersEditTag.
  ///
  /// In en, this message translates to:
  /// **'Edit tag'**
  String get settingsCharactersEditTag;

  /// No description provided for @settingsCharactersDeleteTag.
  ///
  /// In en, this message translates to:
  /// **'Delete tag'**
  String get settingsCharactersDeleteTag;

  /// No description provided for @settingsCharactersDeleteTagMessage.
  ///
  /// In en, this message translates to:
  /// **'Delete “{name}”?'**
  String settingsCharactersDeleteTagMessage(String name);

  /// No description provided for @settingsCharactersTagName.
  ///
  /// In en, this message translates to:
  /// **'Tag name'**
  String get settingsCharactersTagName;

  /// No description provided for @settingsCharactersTagDescription.
  ///
  /// In en, this message translates to:
  /// **'Tag description'**
  String get settingsCharactersTagDescription;

  /// No description provided for @settingsCharactersTagPromptContent.
  ///
  /// In en, this message translates to:
  /// **'Prompt content'**
  String get settingsCharactersTagPromptContent;

  /// No description provided for @settingsCharactersChatModelBindingMode.
  ///
  /// In en, this message translates to:
  /// **'Chat model binding mode'**
  String get settingsCharactersChatModelBindingMode;

  /// No description provided for @settingsCharactersChatModelConfigId.
  ///
  /// In en, this message translates to:
  /// **'Chat model config ID'**
  String get settingsCharactersChatModelConfigId;

  /// No description provided for @settingsCharactersChatModelIndex.
  ///
  /// In en, this message translates to:
  /// **'Chat model index'**
  String get settingsCharactersChatModelIndex;

  /// No description provided for @settingsCharactersMemoryBindingMode.
  ///
  /// In en, this message translates to:
  /// **'Memory binding mode'**
  String get settingsCharactersMemoryBindingMode;

  /// No description provided for @settingsCharactersMemoryProfileId.
  ///
  /// In en, this message translates to:
  /// **'Memory profile ID'**
  String get settingsCharactersMemoryProfileId;

  /// No description provided for @settingsCharactersToolAccess.
  ///
  /// In en, this message translates to:
  /// **'Enable character tool allowlist'**
  String get settingsCharactersToolAccess;

  /// No description provided for @settingsCharactersChatModelFollowGlobal.
  ///
  /// In en, this message translates to:
  /// **'Follow global model'**
  String get settingsCharactersChatModelFollowGlobal;

  /// No description provided for @settingsCharactersChatModelFixedConfig.
  ///
  /// In en, this message translates to:
  /// **'Use fixed model config'**
  String get settingsCharactersChatModelFixedConfig;

  /// No description provided for @settingsCharactersChatModelConfig.
  ///
  /// In en, this message translates to:
  /// **'Model config'**
  String get settingsCharactersChatModelConfig;

  /// No description provided for @settingsCharactersMemoryProfileFollowGlobal.
  ///
  /// In en, this message translates to:
  /// **'Follow global memory'**
  String get settingsCharactersMemoryProfileFollowGlobal;

  /// No description provided for @settingsCharactersMemoryProfileFixedProfile.
  ///
  /// In en, this message translates to:
  /// **'Use fixed memory profile'**
  String get settingsCharactersMemoryProfileFixedProfile;

  /// No description provided for @settingsCharactersMemoryProfile.
  ///
  /// In en, this message translates to:
  /// **'Memory profile'**
  String get settingsCharactersMemoryProfile;

  /// No description provided for @settingsCharactersToolAccessFollowGlobal.
  ///
  /// In en, this message translates to:
  /// **'Follow global tool permissions'**
  String get settingsCharactersToolAccessFollowGlobal;

  /// No description provided for @settingsCharactersToolAccessEmpty.
  ///
  /// In en, this message translates to:
  /// **'Enabled with no selected tools'**
  String get settingsCharactersToolAccessEmpty;

  /// No description provided for @settingsCharactersToolAccessSummaryCounts.
  ///
  /// In en, this message translates to:
  /// **'Built-in {builtinCount} · packages {packageCount} · skills {skillCount} · MCP {mcpCount}'**
  String settingsCharactersToolAccessSummaryCounts(
    int builtinCount,
    int packageCount,
    int skillCount,
    int mcpCount,
  );

  /// No description provided for @settingsCharactersToolAccessConfigure.
  ///
  /// In en, this message translates to:
  /// **'Configure tool allowlist'**
  String get settingsCharactersToolAccessConfigure;

  /// No description provided for @settingsCharactersToolAccessRequiresUsePackage.
  ///
  /// In en, this message translates to:
  /// **'Selecting packages, skills, or MCP also requires allowing the built-in use_package tool.'**
  String get settingsCharactersToolAccessRequiresUsePackage;

  /// No description provided for @settingsCharactersToolAccessEmptyBuiltin.
  ///
  /// In en, this message translates to:
  /// **'No built-in tools available.'**
  String get settingsCharactersToolAccessEmptyBuiltin;

  /// No description provided for @settingsCharactersToolAccessEmptyPackages.
  ///
  /// In en, this message translates to:
  /// **'No packages available.'**
  String get settingsCharactersToolAccessEmptyPackages;

  /// No description provided for @settingsCharactersToolAccessEmptySkills.
  ///
  /// In en, this message translates to:
  /// **'No skills available.'**
  String get settingsCharactersToolAccessEmptySkills;

  /// No description provided for @settingsCharactersToolAccessEmptyMcp.
  ///
  /// In en, this message translates to:
  /// **'No MCP servers available.'**
  String get settingsCharactersToolAccessEmptyMcp;

  /// No description provided for @settingsCharactersBuiltinTools.
  ///
  /// In en, this message translates to:
  /// **'Allowed built-in tools'**
  String get settingsCharactersBuiltinTools;

  /// No description provided for @settingsCharactersAllowedPackages.
  ///
  /// In en, this message translates to:
  /// **'Allowed packages'**
  String get settingsCharactersAllowedPackages;

  /// No description provided for @settingsCharactersAllowedSkills.
  ///
  /// In en, this message translates to:
  /// **'Allowed skills'**
  String get settingsCharactersAllowedSkills;

  /// No description provided for @settingsCharactersAllowedMcpServers.
  ///
  /// In en, this message translates to:
  /// **'Allowed MCP servers'**
  String get settingsCharactersAllowedMcpServers;

  /// No description provided for @settingsCharactersGroupMembersTitle.
  ///
  /// In en, this message translates to:
  /// **'Group characters'**
  String get settingsCharactersGroupMembersTitle;

  /// No description provided for @settingsCharactersPreferenceProfilesSection.
  ///
  /// In en, this message translates to:
  /// **'User preferences & memory'**
  String get settingsCharactersPreferenceProfilesSection;

  /// No description provided for @settingsCharactersCreatePreferenceProfile.
  ///
  /// In en, this message translates to:
  /// **'New user preference profile'**
  String get settingsCharactersCreatePreferenceProfile;

  /// No description provided for @settingsCharactersEditPreferenceProfile.
  ///
  /// In en, this message translates to:
  /// **'Edit user preference profile'**
  String get settingsCharactersEditPreferenceProfile;

  /// No description provided for @settingsCharactersPreferenceProfileName.
  ///
  /// In en, this message translates to:
  /// **'Profile name'**
  String get settingsCharactersPreferenceProfileName;

  /// No description provided for @settingsCharactersPreferenceBirthDate.
  ///
  /// In en, this message translates to:
  /// **'Birth date timestamp'**
  String get settingsCharactersPreferenceBirthDate;

  /// No description provided for @settingsCharactersPreferenceGender.
  ///
  /// In en, this message translates to:
  /// **'Gender'**
  String get settingsCharactersPreferenceGender;

  /// No description provided for @settingsCharactersPreferencePersonality.
  ///
  /// In en, this message translates to:
  /// **'Personality'**
  String get settingsCharactersPreferencePersonality;

  /// No description provided for @settingsCharactersPreferenceIdentity.
  ///
  /// In en, this message translates to:
  /// **'Identity'**
  String get settingsCharactersPreferenceIdentity;

  /// No description provided for @settingsCharactersPreferenceOccupation.
  ///
  /// In en, this message translates to:
  /// **'Occupation'**
  String get settingsCharactersPreferenceOccupation;

  /// No description provided for @settingsCharactersPreferenceAiStyle.
  ///
  /// In en, this message translates to:
  /// **'AI interaction style'**
  String get settingsCharactersPreferenceAiStyle;

  /// No description provided for @settingsCharactersMemoryAutoUpdate.
  ///
  /// In en, this message translates to:
  /// **'Auto-update memory preferences'**
  String get settingsCharactersMemoryAutoUpdate;

  /// No description provided for @settingsCharactersMemoryAutoUpdateDescription.
  ///
  /// In en, this message translates to:
  /// **'Allow AI to update user preferences and long-term memory from conversations.'**
  String get settingsCharactersMemoryAutoUpdateDescription;

  /// No description provided for @settingsCharactersPreferenceDescription.
  ///
  /// In en, this message translates to:
  /// **'Send user preferences to model'**
  String get settingsCharactersPreferenceDescription;

  /// No description provided for @settingsCharactersPreferenceDescriptionSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Include the active preference profile in chat prompts.'**
  String get settingsCharactersPreferenceDescriptionSubtitle;

  /// No description provided for @settingsCharactersPreferenceLocksSection.
  ///
  /// In en, this message translates to:
  /// **'Preference field locks'**
  String get settingsCharactersPreferenceLocksSection;

  /// No description provided for @settingsCharactersPreferenceLockDescription.
  ///
  /// In en, this message translates to:
  /// **'When locked, automatic memory updates will not rewrite this field.'**
  String get settingsCharactersPreferenceLockDescription;

  /// No description provided for @settingsCharactersCardsSection.
  ///
  /// In en, this message translates to:
  /// **'Character cards'**
  String get settingsCharactersCardsSection;

  /// No description provided for @settingsCharactersGroupsSection.
  ///
  /// In en, this message translates to:
  /// **'Character groups'**
  String get settingsCharactersGroupsSection;

  /// No description provided for @settingsCharactersGroupMembers.
  ///
  /// In en, this message translates to:
  /// **'{count} members'**
  String settingsCharactersGroupMembers(int count);

  /// No description provided for @settingsToolsPermissionMode.
  ///
  /// In en, this message translates to:
  /// **'Tool permission mode'**
  String get settingsToolsPermissionMode;

  /// No description provided for @settingsToolsAsk.
  ///
  /// In en, this message translates to:
  /// **'Ask'**
  String get settingsToolsAsk;

  /// No description provided for @settingsToolsExtensions.
  ///
  /// In en, this message translates to:
  /// **'Extension management'**
  String get settingsToolsExtensions;

  /// No description provided for @settingsToolsPlugins.
  ///
  /// In en, this message translates to:
  /// **'Plugins'**
  String get settingsToolsPlugins;

  /// No description provided for @settingsToolsPluginsDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage ToolPkg plugin containers and UI extensions.'**
  String get settingsToolsPluginsDescription;

  /// No description provided for @settingsToolsPackages.
  ///
  /// In en, this message translates to:
  /// **'Tool packages'**
  String get settingsToolsPackages;

  /// No description provided for @settingsToolsPackagesDescription.
  ///
  /// In en, this message translates to:
  /// **'Enable, disable, and inspect built-in or external tool packages.'**
  String get settingsToolsPackagesDescription;

  /// No description provided for @settingsToolsSkills.
  ///
  /// In en, this message translates to:
  /// **'Skills'**
  String get settingsToolsSkills;

  /// No description provided for @settingsToolsSkillsDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage skill package visibility and imports.'**
  String get settingsToolsSkillsDescription;

  /// No description provided for @settingsToolsMcp.
  ///
  /// In en, this message translates to:
  /// **'MCP servers'**
  String get settingsToolsMcp;

  /// No description provided for @settingsToolsMcpDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage MCP configuration. Startup wait is {seconds} seconds.'**
  String settingsToolsMcpDescription(int seconds);

  /// No description provided for @settingsToolsOverrides.
  ///
  /// In en, this message translates to:
  /// **'Per-tool permission records'**
  String get settingsToolsOverrides;

  /// No description provided for @settingsToolsToolGroups.
  ///
  /// In en, this message translates to:
  /// **'Per-tool permissions'**
  String get settingsToolsToolGroups;

  /// No description provided for @settingsToolsToolGroupsDescription.
  ///
  /// In en, this message translates to:
  /// **'Keep Ask for normal use. Put trusted tools in Always allow, and risky or unwanted tools in Always forbid.'**
  String get settingsToolsToolGroupsDescription;

  /// No description provided for @settingsToolsAlwaysAllow.
  ///
  /// In en, this message translates to:
  /// **'Always allow'**
  String get settingsToolsAlwaysAllow;

  /// No description provided for @settingsToolsAlwaysAllowDescription.
  ///
  /// In en, this message translates to:
  /// **'These tools run without asking again.'**
  String get settingsToolsAlwaysAllowDescription;

  /// No description provided for @settingsToolsAlwaysForbid.
  ///
  /// In en, this message translates to:
  /// **'Always forbid'**
  String get settingsToolsAlwaysForbid;

  /// No description provided for @settingsToolsAlwaysForbidDescription.
  ///
  /// In en, this message translates to:
  /// **'AI will not call these tools.'**
  String get settingsToolsAlwaysForbidDescription;

  /// No description provided for @settingsToolsAddTool.
  ///
  /// In en, this message translates to:
  /// **'Add tool'**
  String get settingsToolsAddTool;

  /// No description provided for @settingsToolsAddAllowTool.
  ///
  /// In en, this message translates to:
  /// **'Add allowed tool'**
  String get settingsToolsAddAllowTool;

  /// No description provided for @settingsToolsAddForbidTool.
  ///
  /// In en, this message translates to:
  /// **'Add forbidden tool'**
  String get settingsToolsAddForbidTool;

  /// No description provided for @settingsToolsSearchTools.
  ///
  /// In en, this message translates to:
  /// **'Search tools'**
  String get settingsToolsSearchTools;

  /// No description provided for @settingsToolsNoToolsInGroup.
  ///
  /// In en, this message translates to:
  /// **'No tools in this group.'**
  String get settingsToolsNoToolsInGroup;

  /// No description provided for @settingsToolsMcpStartupTimeout.
  ///
  /// In en, this message translates to:
  /// **'MCP startup timeout'**
  String get settingsToolsMcpStartupTimeout;

  /// No description provided for @settingsToolsMcpStartupTimeoutSeconds.
  ///
  /// In en, this message translates to:
  /// **'Wait seconds'**
  String get settingsToolsMcpStartupTimeoutSeconds;

  /// No description provided for @settingsWorkspaceCurrentDesign.
  ///
  /// In en, this message translates to:
  /// **'Current workspace structure'**
  String get settingsWorkspaceCurrentDesign;

  /// No description provided for @settingsWorkspaceCurrentDesignDescription.
  ///
  /// In en, this message translates to:
  /// **'Workspaces are bound to chats. Terminal sessions and browser sessions are global sessions shown flat inside the workspace.'**
  String get settingsWorkspaceCurrentDesignDescription;

  /// No description provided for @settingsWorkspaceOpenChat.
  ///
  /// In en, this message translates to:
  /// **'Return to chat workspace'**
  String get settingsWorkspaceOpenChat;

  /// No description provided for @settingsWorkspaceOpenChatDescription.
  ///
  /// In en, this message translates to:
  /// **'Open files, terminals, browser, and web automation on the right side of chat.'**
  String get settingsWorkspaceOpenChatDescription;

  /// No description provided for @settingsWorkspaceContains.
  ///
  /// In en, this message translates to:
  /// **'Workspace contains'**
  String get settingsWorkspaceContains;

  /// No description provided for @settingsWorkspacePerChat.
  ///
  /// In en, this message translates to:
  /// **'Bound per chat'**
  String get settingsWorkspacePerChat;

  /// No description provided for @settingsWorkspaceGlobalSessions.
  ///
  /// In en, this message translates to:
  /// **'Global terminal sessions'**
  String get settingsWorkspaceGlobalSessions;

  /// No description provided for @settingsWorkspaceBrowserSessions.
  ///
  /// In en, this message translates to:
  /// **'Browser and WebVisit sessions'**
  String get settingsWorkspaceBrowserSessions;

  /// No description provided for @settingsAppearanceThemeSection.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get settingsAppearanceThemeSection;

  /// No description provided for @settingsAppearanceThemeMode.
  ///
  /// In en, this message translates to:
  /// **'Current mode'**
  String get settingsAppearanceThemeMode;

  /// No description provided for @settingsAppearanceThemeTarget.
  ///
  /// In en, this message translates to:
  /// **'Theme save target'**
  String get settingsAppearanceThemeTarget;

  /// No description provided for @settingsAppearanceThemeTargetGlobal.
  ///
  /// In en, this message translates to:
  /// **'Global'**
  String get settingsAppearanceThemeTargetGlobal;

  /// No description provided for @settingsAppearanceThemeTargetCharacter.
  ///
  /// In en, this message translates to:
  /// **'Current character: {name}'**
  String settingsAppearanceThemeTargetCharacter(Object name);

  /// No description provided for @settingsAppearanceThemeTargetGroup.
  ///
  /// In en, this message translates to:
  /// **'Current group: {name}'**
  String settingsAppearanceThemeTargetGroup(Object name);

  /// No description provided for @settingsAppearanceThemeSystem.
  ///
  /// In en, this message translates to:
  /// **'System'**
  String get settingsAppearanceThemeSystem;

  /// No description provided for @settingsAppearanceThemeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get settingsAppearanceThemeLight;

  /// No description provided for @settingsAppearanceThemeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get settingsAppearanceThemeDark;

  /// No description provided for @settingsAppearanceColorSection.
  ///
  /// In en, this message translates to:
  /// **'Theme color'**
  String get settingsAppearanceColorSection;

  /// No description provided for @settingsAppearanceColorDescription.
  ///
  /// In en, this message translates to:
  /// **'Choose a simple color preset. System bars and current app chrome follow the theme automatically.'**
  String get settingsAppearanceColorDescription;

  /// No description provided for @settingsAppearanceColorDefault.
  ///
  /// In en, this message translates to:
  /// **'Default'**
  String get settingsAppearanceColorDefault;

  /// No description provided for @settingsAppearanceColorSky.
  ///
  /// In en, this message translates to:
  /// **'Sky'**
  String get settingsAppearanceColorSky;

  /// No description provided for @settingsAppearanceColorMatcha.
  ///
  /// In en, this message translates to:
  /// **'Matcha'**
  String get settingsAppearanceColorMatcha;

  /// No description provided for @settingsAppearanceColorEmber.
  ///
  /// In en, this message translates to:
  /// **'Ember'**
  String get settingsAppearanceColorEmber;

  /// No description provided for @settingsAppearanceColorRose.
  ///
  /// In en, this message translates to:
  /// **'Rose'**
  String get settingsAppearanceColorRose;

  /// No description provided for @settingsAppearanceColorCustom.
  ///
  /// In en, this message translates to:
  /// **'Custom colors'**
  String get settingsAppearanceColorCustom;

  /// No description provided for @settingsAppearanceCustomColorsTitle.
  ///
  /// In en, this message translates to:
  /// **'Custom theme colors'**
  String get settingsAppearanceCustomColorsTitle;

  /// No description provided for @settingsAppearancePrimaryColor.
  ///
  /// In en, this message translates to:
  /// **'Primary color'**
  String get settingsAppearancePrimaryColor;

  /// No description provided for @settingsAppearanceSecondaryColor.
  ///
  /// In en, this message translates to:
  /// **'Secondary color'**
  String get settingsAppearanceSecondaryColor;

  /// No description provided for @settingsAppearanceHexColorHint.
  ///
  /// In en, this message translates to:
  /// **'#RRGGBB'**
  String get settingsAppearanceHexColorHint;

  /// No description provided for @settingsAppearanceHexColorInvalid.
  ///
  /// In en, this message translates to:
  /// **'Enter a color in #RRGGBB format'**
  String get settingsAppearanceHexColorInvalid;

  /// No description provided for @settingsAppearanceBackgroundSection.
  ///
  /// In en, this message translates to:
  /// **'Background'**
  String get settingsAppearanceBackgroundSection;

  /// No description provided for @settingsAppearanceBackgroundDescription.
  ///
  /// In en, this message translates to:
  /// **'Choose a local image or video as the app background. App surfaces and system bars follow the theme automatically.'**
  String get settingsAppearanceBackgroundDescription;

  /// No description provided for @settingsAppearanceBackgroundImage.
  ///
  /// In en, this message translates to:
  /// **'Background media'**
  String get settingsAppearanceBackgroundImage;

  /// No description provided for @settingsAppearanceBackgroundNone.
  ///
  /// In en, this message translates to:
  /// **'None selected'**
  String get settingsAppearanceBackgroundNone;

  /// No description provided for @settingsAppearanceBackgroundChooseImage.
  ///
  /// In en, this message translates to:
  /// **'Choose image'**
  String get settingsAppearanceBackgroundChooseImage;

  /// No description provided for @settingsAppearanceBackgroundChooseVideo.
  ///
  /// In en, this message translates to:
  /// **'Choose video'**
  String get settingsAppearanceBackgroundChooseVideo;

  /// No description provided for @settingsAppearanceBackgroundDisable.
  ///
  /// In en, this message translates to:
  /// **'Disable background'**
  String get settingsAppearanceBackgroundDisable;

  /// No description provided for @settingsAppearanceBackgroundEnabled.
  ///
  /// In en, this message translates to:
  /// **'Enable background'**
  String get settingsAppearanceBackgroundEnabled;

  /// No description provided for @settingsAppearanceBackgroundOpacity.
  ///
  /// In en, this message translates to:
  /// **'Background opacity'**
  String get settingsAppearanceBackgroundOpacity;

  /// No description provided for @settingsAppearanceBackgroundBlur.
  ///
  /// In en, this message translates to:
  /// **'Blur background'**
  String get settingsAppearanceBackgroundBlur;

  /// No description provided for @settingsAppearanceBackgroundBlurRadius.
  ///
  /// In en, this message translates to:
  /// **'Blur strength'**
  String get settingsAppearanceBackgroundBlurRadius;

  /// No description provided for @settingsAppearanceBackgroundVideoMuted.
  ///
  /// In en, this message translates to:
  /// **'Mute video background'**
  String get settingsAppearanceBackgroundVideoMuted;

  /// No description provided for @settingsAppearanceBackgroundVideoLoop.
  ///
  /// In en, this message translates to:
  /// **'Loop video background'**
  String get settingsAppearanceBackgroundVideoLoop;

  /// No description provided for @settingsAppearanceTextSection.
  ///
  /// In en, this message translates to:
  /// **'Text'**
  String get settingsAppearanceTextSection;

  /// No description provided for @settingsAppearanceFontFamily.
  ///
  /// In en, this message translates to:
  /// **'Font'**
  String get settingsAppearanceFontFamily;

  /// No description provided for @settingsAppearanceFontDefault.
  ///
  /// In en, this message translates to:
  /// **'Default'**
  String get settingsAppearanceFontDefault;

  /// No description provided for @settingsAppearanceCustomFont.
  ///
  /// In en, this message translates to:
  /// **'Custom font'**
  String get settingsAppearanceCustomFont;

  /// No description provided for @settingsAppearanceFontCustom.
  ///
  /// In en, this message translates to:
  /// **'Custom'**
  String get settingsAppearanceFontCustom;

  /// No description provided for @settingsAppearanceChooseCustomFont.
  ///
  /// In en, this message translates to:
  /// **'Choose custom font'**
  String get settingsAppearanceChooseCustomFont;

  /// No description provided for @settingsAppearanceClearCustomFont.
  ///
  /// In en, this message translates to:
  /// **'Clear custom font'**
  String get settingsAppearanceClearCustomFont;

  /// No description provided for @settingsAppearanceFontSerif.
  ///
  /// In en, this message translates to:
  /// **'Serif'**
  String get settingsAppearanceFontSerif;

  /// No description provided for @settingsAppearanceFontMonospace.
  ///
  /// In en, this message translates to:
  /// **'Mono'**
  String get settingsAppearanceFontMonospace;

  /// No description provided for @settingsAppearanceFontScale.
  ///
  /// In en, this message translates to:
  /// **'Font size'**
  String get settingsAppearanceFontScale;

  /// No description provided for @settingsAppearanceAvatarSection.
  ///
  /// In en, this message translates to:
  /// **'Avatars'**
  String get settingsAppearanceAvatarSection;

  /// No description provided for @settingsAppearanceUserAvatar.
  ///
  /// In en, this message translates to:
  /// **'User avatar'**
  String get settingsAppearanceUserAvatar;

  /// No description provided for @settingsAppearanceAiAvatar.
  ///
  /// In en, this message translates to:
  /// **'AI avatar'**
  String get settingsAppearanceAiAvatar;

  /// No description provided for @settingsAppearanceAvatarDefault.
  ///
  /// In en, this message translates to:
  /// **'Default avatar'**
  String get settingsAppearanceAvatarDefault;

  /// No description provided for @settingsAppearanceAvatarShape.
  ///
  /// In en, this message translates to:
  /// **'Avatar shape'**
  String get settingsAppearanceAvatarShape;

  /// No description provided for @settingsAppearanceAvatarShapeCircle.
  ///
  /// In en, this message translates to:
  /// **'Circle'**
  String get settingsAppearanceAvatarShapeCircle;

  /// No description provided for @settingsAppearanceAvatarShapeSquare.
  ///
  /// In en, this message translates to:
  /// **'Square'**
  String get settingsAppearanceAvatarShapeSquare;

  /// No description provided for @settingsAppearanceChooseUserAvatar.
  ///
  /// In en, this message translates to:
  /// **'Choose user avatar'**
  String get settingsAppearanceChooseUserAvatar;

  /// No description provided for @settingsAppearanceChooseAiAvatar.
  ///
  /// In en, this message translates to:
  /// **'Choose AI avatar'**
  String get settingsAppearanceChooseAiAvatar;

  /// No description provided for @settingsAppearanceClearUserAvatar.
  ///
  /// In en, this message translates to:
  /// **'Clear user avatar'**
  String get settingsAppearanceClearUserAvatar;

  /// No description provided for @settingsAppearanceClearAiAvatar.
  ///
  /// In en, this message translates to:
  /// **'Clear AI avatar'**
  String get settingsAppearanceClearAiAvatar;

  /// No description provided for @settingsAppearanceChatDisplaySection.
  ///
  /// In en, this message translates to:
  /// **'Chat display'**
  String get settingsAppearanceChatDisplaySection;

  /// No description provided for @settingsAppearanceMessageStyle.
  ///
  /// In en, this message translates to:
  /// **'Message style'**
  String get settingsAppearanceMessageStyle;

  /// No description provided for @settingsAppearanceMessageStyleClean.
  ///
  /// In en, this message translates to:
  /// **'Command'**
  String get settingsAppearanceMessageStyleClean;

  /// No description provided for @settingsAppearanceMessageStyleCard.
  ///
  /// In en, this message translates to:
  /// **'Bubble'**
  String get settingsAppearanceMessageStyleCard;

  /// No description provided for @settingsAppearanceMessageColors.
  ///
  /// In en, this message translates to:
  /// **'Message colors'**
  String get settingsAppearanceMessageColors;

  /// No description provided for @settingsAppearanceMessageColorsTheme.
  ///
  /// In en, this message translates to:
  /// **'Follow theme'**
  String get settingsAppearanceMessageColorsTheme;

  /// No description provided for @settingsAppearanceMessageColorsSky.
  ///
  /// In en, this message translates to:
  /// **'Clean blue'**
  String get settingsAppearanceMessageColorsSky;

  /// No description provided for @settingsAppearanceMessageColorsMatcha.
  ///
  /// In en, this message translates to:
  /// **'Matcha'**
  String get settingsAppearanceMessageColorsMatcha;

  /// No description provided for @settingsAppearanceMessageColorsInk.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get settingsAppearanceMessageColorsInk;

  /// No description provided for @settingsAppearanceMessageColorsCustom.
  ///
  /// In en, this message translates to:
  /// **'Custom message colors'**
  String get settingsAppearanceMessageColorsCustom;

  /// No description provided for @settingsAppearanceCustomMessageColorsTitle.
  ///
  /// In en, this message translates to:
  /// **'Custom message colors'**
  String get settingsAppearanceCustomMessageColorsTitle;

  /// No description provided for @settingsAppearanceCursorUserBubbleColor.
  ///
  /// In en, this message translates to:
  /// **'Command user bubble'**
  String get settingsAppearanceCursorUserBubbleColor;

  /// No description provided for @settingsAppearanceUserBubbleColor.
  ///
  /// In en, this message translates to:
  /// **'User bubble'**
  String get settingsAppearanceUserBubbleColor;

  /// No description provided for @settingsAppearanceAiBubbleColor.
  ///
  /// In en, this message translates to:
  /// **'AI bubble'**
  String get settingsAppearanceAiBubbleColor;

  /// No description provided for @settingsAppearanceUserTextColor.
  ///
  /// In en, this message translates to:
  /// **'User text'**
  String get settingsAppearanceUserTextColor;

  /// No description provided for @settingsAppearanceAiTextColor.
  ///
  /// In en, this message translates to:
  /// **'AI text'**
  String get settingsAppearanceAiTextColor;

  /// No description provided for @settingsAppearanceMessageSurface.
  ///
  /// In en, this message translates to:
  /// **'Global texture'**
  String get settingsAppearanceMessageSurface;

  /// No description provided for @settingsAppearanceMessageSurfaceNormal.
  ///
  /// In en, this message translates to:
  /// **'Normal'**
  String get settingsAppearanceMessageSurfaceNormal;

  /// No description provided for @settingsAppearanceMessageSurfaceTransparent.
  ///
  /// In en, this message translates to:
  /// **'Transparent'**
  String get settingsAppearanceMessageSurfaceTransparent;

  /// No description provided for @settingsAppearanceUserBubbleFont.
  ///
  /// In en, this message translates to:
  /// **'User bubble font'**
  String get settingsAppearanceUserBubbleFont;

  /// No description provided for @settingsAppearanceAiBubbleFont.
  ///
  /// In en, this message translates to:
  /// **'AI bubble font'**
  String get settingsAppearanceAiBubbleFont;

  /// No description provided for @settingsAppearanceAdjustUserBubbleFont.
  ///
  /// In en, this message translates to:
  /// **'Adjust user bubble font'**
  String get settingsAppearanceAdjustUserBubbleFont;

  /// No description provided for @settingsAppearanceAdjustAiBubbleFont.
  ///
  /// In en, this message translates to:
  /// **'Adjust AI bubble font'**
  String get settingsAppearanceAdjustAiBubbleFont;

  /// No description provided for @settingsAppearanceEnableBubbleFont.
  ///
  /// In en, this message translates to:
  /// **'Enable bubble-specific font'**
  String get settingsAppearanceEnableBubbleFont;

  /// No description provided for @settingsAppearanceUserBubbleImage.
  ///
  /// In en, this message translates to:
  /// **'User bubble image'**
  String get settingsAppearanceUserBubbleImage;

  /// No description provided for @settingsAppearanceAiBubbleImage.
  ///
  /// In en, this message translates to:
  /// **'AI bubble image'**
  String get settingsAppearanceAiBubbleImage;

  /// No description provided for @settingsAppearanceChooseUserBubbleImage.
  ///
  /// In en, this message translates to:
  /// **'Choose user bubble'**
  String get settingsAppearanceChooseUserBubbleImage;

  /// No description provided for @settingsAppearanceChooseAiBubbleImage.
  ///
  /// In en, this message translates to:
  /// **'Choose AI bubble'**
  String get settingsAppearanceChooseAiBubbleImage;

  /// No description provided for @settingsAppearanceClearUserBubbleImage.
  ///
  /// In en, this message translates to:
  /// **'Clear user bubble'**
  String get settingsAppearanceClearUserBubbleImage;

  /// No description provided for @settingsAppearanceClearAiBubbleImage.
  ///
  /// In en, this message translates to:
  /// **'Clear AI bubble'**
  String get settingsAppearanceClearAiBubbleImage;

  /// No description provided for @settingsAppearanceBubbleImageRenderMode.
  ///
  /// In en, this message translates to:
  /// **'Bubble image mode'**
  String get settingsAppearanceBubbleImageRenderMode;

  /// No description provided for @settingsAppearanceBubbleImageTiledNineSlice.
  ///
  /// In en, this message translates to:
  /// **'Tiled 9-slice'**
  String get settingsAppearanceBubbleImageTiledNineSlice;

  /// No description provided for @settingsAppearanceBubbleImageNinePatch.
  ///
  /// In en, this message translates to:
  /// **'Stretch 9-patch'**
  String get settingsAppearanceBubbleImageNinePatch;

  /// No description provided for @settingsAppearanceBubbleImageAdjustUser.
  ///
  /// In en, this message translates to:
  /// **'Adjust user bubble image'**
  String get settingsAppearanceBubbleImageAdjustUser;

  /// No description provided for @settingsAppearanceBubbleImageAdjustAi.
  ///
  /// In en, this message translates to:
  /// **'Adjust AI bubble image'**
  String get settingsAppearanceBubbleImageAdjustAi;

  /// No description provided for @settingsAppearanceBubbleImagePreview.
  ///
  /// In en, this message translates to:
  /// **'Preview'**
  String get settingsAppearanceBubbleImagePreview;

  /// No description provided for @settingsAppearanceBubbleImagePreviewText.
  ///
  /// In en, this message translates to:
  /// **'Bubble preview with 9-slice guides'**
  String get settingsAppearanceBubbleImagePreviewText;

  /// No description provided for @settingsAppearanceBubbleImageCrop.
  ///
  /// In en, this message translates to:
  /// **'Crop'**
  String get settingsAppearanceBubbleImageCrop;

  /// No description provided for @settingsAppearanceBubbleImageRepeat.
  ///
  /// In en, this message translates to:
  /// **'Repeat region'**
  String get settingsAppearanceBubbleImageRepeat;

  /// No description provided for @settingsAppearanceBubbleImageScale.
  ///
  /// In en, this message translates to:
  /// **'Image scale'**
  String get settingsAppearanceBubbleImageScale;

  /// No description provided for @settingsAppearanceBubbleImageCropLeft.
  ///
  /// In en, this message translates to:
  /// **'Crop left'**
  String get settingsAppearanceBubbleImageCropLeft;

  /// No description provided for @settingsAppearanceBubbleImageCropTop.
  ///
  /// In en, this message translates to:
  /// **'Crop top'**
  String get settingsAppearanceBubbleImageCropTop;

  /// No description provided for @settingsAppearanceBubbleImageCropRight.
  ///
  /// In en, this message translates to:
  /// **'Crop right'**
  String get settingsAppearanceBubbleImageCropRight;

  /// No description provided for @settingsAppearanceBubbleImageCropBottom.
  ///
  /// In en, this message translates to:
  /// **'Crop bottom'**
  String get settingsAppearanceBubbleImageCropBottom;

  /// No description provided for @settingsAppearanceBubbleImageRepeatStart.
  ///
  /// In en, this message translates to:
  /// **'Repeat X start'**
  String get settingsAppearanceBubbleImageRepeatStart;

  /// No description provided for @settingsAppearanceBubbleImageRepeatEnd.
  ///
  /// In en, this message translates to:
  /// **'Repeat X end'**
  String get settingsAppearanceBubbleImageRepeatEnd;

  /// No description provided for @settingsAppearanceBubbleImageRepeatYStart.
  ///
  /// In en, this message translates to:
  /// **'Repeat Y start'**
  String get settingsAppearanceBubbleImageRepeatYStart;

  /// No description provided for @settingsAppearanceBubbleImageRepeatYEnd.
  ///
  /// In en, this message translates to:
  /// **'Repeat Y end'**
  String get settingsAppearanceBubbleImageRepeatYEnd;

  /// No description provided for @settingsAppearanceMessageDensity.
  ///
  /// In en, this message translates to:
  /// **'Message spacing'**
  String get settingsAppearanceMessageDensity;

  /// No description provided for @settingsAppearanceMessageDensityComfortable.
  ///
  /// In en, this message translates to:
  /// **'Comfortable'**
  String get settingsAppearanceMessageDensityComfortable;

  /// No description provided for @settingsAppearanceMessageDensityCompact.
  ///
  /// In en, this message translates to:
  /// **'Compact'**
  String get settingsAppearanceMessageDensityCompact;

  /// No description provided for @settingsAppearanceWideLayout.
  ///
  /// In en, this message translates to:
  /// **'Use wider chat layout'**
  String get settingsAppearanceWideLayout;

  /// No description provided for @settingsAppearanceRoundedMessages.
  ///
  /// In en, this message translates to:
  /// **'Rounded message cards'**
  String get settingsAppearanceRoundedMessages;

  /// No description provided for @settingsAppearanceShowAvatars.
  ///
  /// In en, this message translates to:
  /// **'Show message avatars'**
  String get settingsAppearanceShowAvatars;

  /// No description provided for @settingsAppearanceShowThinkingProcess.
  ///
  /// In en, this message translates to:
  /// **'Show thinking process'**
  String get settingsAppearanceShowThinkingProcess;

  /// No description provided for @settingsAppearanceShowRoleName.
  ///
  /// In en, this message translates to:
  /// **'Show role name'**
  String get settingsAppearanceShowRoleName;

  /// No description provided for @settingsAppearanceShowUserName.
  ///
  /// In en, this message translates to:
  /// **'Show user name'**
  String get settingsAppearanceShowUserName;

  /// No description provided for @settingsAppearanceShowModelName.
  ///
  /// In en, this message translates to:
  /// **'Show model name'**
  String get settingsAppearanceShowModelName;

  /// No description provided for @settingsAppearanceShowModelProvider.
  ///
  /// In en, this message translates to:
  /// **'Show model provider'**
  String get settingsAppearanceShowModelProvider;

  /// No description provided for @settingsAppearanceShowMessageTokenStats.
  ///
  /// In en, this message translates to:
  /// **'Show token stats'**
  String get settingsAppearanceShowMessageTokenStats;

  /// No description provided for @settingsAppearanceShowMessageTimingStats.
  ///
  /// In en, this message translates to:
  /// **'Show timing stats'**
  String get settingsAppearanceShowMessageTimingStats;

  /// No description provided for @settingsAppearanceShowMessageTimestamp.
  ///
  /// In en, this message translates to:
  /// **'Show message time'**
  String get settingsAppearanceShowMessageTimestamp;

  /// No description provided for @settingsAppearanceShowInputProcessingStatus.
  ///
  /// In en, this message translates to:
  /// **'Show input processing status'**
  String get settingsAppearanceShowInputProcessingStatus;

  /// No description provided for @settingsAppearanceResetTheme.
  ///
  /// In en, this message translates to:
  /// **'Reset theme settings'**
  String get settingsAppearanceResetTheme;

  /// No description provided for @settingsAppearanceLanguageSection.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get settingsAppearanceLanguageSection;

  /// No description provided for @settingsAppearanceLanguage.
  ///
  /// In en, this message translates to:
  /// **'Current language'**
  String get settingsAppearanceLanguage;

  /// No description provided for @settingsAppearanceLanguageDescription.
  ///
  /// In en, this message translates to:
  /// **'Language follows the localization configuration loaded at app startup.'**
  String get settingsAppearanceLanguageDescription;

  /// No description provided for @settingsDataRuntimeSection.
  ///
  /// In en, this message translates to:
  /// **'Runtime'**
  String get settingsDataRuntimeSection;

  /// No description provided for @settingsDataCoreVersion.
  ///
  /// In en, this message translates to:
  /// **'Core version'**
  String get settingsDataCoreVersion;

  /// No description provided for @settingsDataTokenSection.
  ///
  /// In en, this message translates to:
  /// **'Token statistics'**
  String get settingsDataTokenSection;

  /// No description provided for @settingsDataInputTokens.
  ///
  /// In en, this message translates to:
  /// **'Input tokens'**
  String get settingsDataInputTokens;

  /// No description provided for @settingsDataOutputTokens.
  ///
  /// In en, this message translates to:
  /// **'Output tokens'**
  String get settingsDataOutputTokens;

  /// No description provided for @settingsDataRefreshTokenStats.
  ///
  /// In en, this message translates to:
  /// **'Refresh cumulative statistics'**
  String get settingsDataRefreshTokenStats;

  /// No description provided for @settingsDataResetTokenStats.
  ///
  /// In en, this message translates to:
  /// **'Reset token statistics'**
  String get settingsDataResetTokenStats;

  /// No description provided for @settingsDataBackupSection.
  ///
  /// In en, this message translates to:
  /// **'Backup'**
  String get settingsDataBackupSection;

  /// No description provided for @settingsDataChatHistoriesBackup.
  ///
  /// In en, this message translates to:
  /// **'Chat history backup'**
  String get settingsDataChatHistoriesBackup;

  /// No description provided for @settingsDataChatHistoriesBackupDescription.
  ///
  /// In en, this message translates to:
  /// **'Copy all chats and messages as JSON. Import updates or creates chats by chat ID.'**
  String get settingsDataChatHistoriesBackupDescription;

  /// No description provided for @settingsDataCharacterCardsBackup.
  ///
  /// In en, this message translates to:
  /// **'Character card backup'**
  String get settingsDataCharacterCardsBackup;

  /// No description provided for @settingsDataCharacterCardsBackupDescription.
  ///
  /// In en, this message translates to:
  /// **'Copy all character cards and referenced tags as JSON. Import updates or creates items by original ID.'**
  String get settingsDataCharacterCardsBackupDescription;

  /// No description provided for @settingsDataCharacterGroupsBackup.
  ///
  /// In en, this message translates to:
  /// **'Character group backup'**
  String get settingsDataCharacterGroupsBackup;

  /// No description provided for @settingsDataCharacterGroupsBackupDescription.
  ///
  /// In en, this message translates to:
  /// **'Copy all character groups as JSON. Import keeps member references and ordering.'**
  String get settingsDataCharacterGroupsBackupDescription;

  /// No description provided for @settingsDataModelConfigsBackup.
  ///
  /// In en, this message translates to:
  /// **'Model config backup'**
  String get settingsDataModelConfigsBackup;

  /// No description provided for @settingsDataModelConfigsBackupDescription.
  ///
  /// In en, this message translates to:
  /// **'Copy all model configs as JSON. Import updates or creates items by config ID, including model parameters and API key pools.'**
  String get settingsDataModelConfigsBackupDescription;

  /// No description provided for @settingsDataBackupCount.
  ///
  /// In en, this message translates to:
  /// **'{count} items'**
  String settingsDataBackupCount(int count);

  /// No description provided for @settingsDataCopyBackupJson.
  ///
  /// In en, this message translates to:
  /// **'Copy backup JSON'**
  String get settingsDataCopyBackupJson;

  /// No description provided for @settingsDataImportBackupJson.
  ///
  /// In en, this message translates to:
  /// **'Import backup JSON'**
  String get settingsDataImportBackupJson;

  /// No description provided for @settingsDataBackupJsonInput.
  ///
  /// In en, this message translates to:
  /// **'Backup JSON content'**
  String get settingsDataBackupJsonInput;

  /// No description provided for @settingsDataBackupCopied.
  ///
  /// In en, this message translates to:
  /// **'Copied backup JSON for “{name}”.'**
  String settingsDataBackupCopied(String name);

  /// No description provided for @settingsDataBackupImportResult.
  ///
  /// In en, this message translates to:
  /// **'Import complete: {newCount} new, {updatedCount} updated, {skippedCount} skipped.'**
  String settingsDataBackupImportResult(
    int newCount,
    int updatedCount,
    int skippedCount,
  );

  /// No description provided for @settingsDataBackupImportError.
  ///
  /// In en, this message translates to:
  /// **'Backup import failed: {error}'**
  String settingsDataBackupImportError(String error);

  /// No description provided for @settingsDataBackupCopyError.
  ///
  /// In en, this message translates to:
  /// **'Backup copy failed: {error}'**
  String settingsDataBackupCopyError(String error);

  /// No description provided for @settingsDataExportRawSnapshot.
  ///
  /// In en, this message translates to:
  /// **'Export raw snapshot'**
  String get settingsDataExportRawSnapshot;

  /// No description provided for @settingsDataExportRawSnapshotDescription.
  ///
  /// In en, this message translates to:
  /// **'Generate the current data snapshot from runtime and show its byte size.'**
  String get settingsDataExportRawSnapshotDescription;

  /// No description provided for @settingsDataSnapshotBytes.
  ///
  /// In en, this message translates to:
  /// **'Snapshot generated: {bytes} bytes'**
  String settingsDataSnapshotBytes(int bytes);
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
