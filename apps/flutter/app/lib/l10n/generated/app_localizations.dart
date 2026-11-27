import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_fr.dart';
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
    Locale('fr'),
    Locale('zh'),
  ];

  /// No description provided for @terminalProfileSelect.
  ///
  /// In en, this message translates to:
  /// **'Choose terminal profile'**
  String get terminalProfileSelect;

  /// No description provided for @terminalProfilesUnavailable.
  ///
  /// In en, this message translates to:
  /// **'No terminals are available on this device.'**
  String get terminalProfilesUnavailable;

  /// No description provided for @terminalImplementationNative.
  ///
  /// In en, this message translates to:
  /// **'Native terminal'**
  String get terminalImplementationNative;

  /// No description provided for @terminalImplementationProot.
  ///
  /// In en, this message translates to:
  /// **'PRoot Linux terminal'**
  String get terminalImplementationProot;

  /// No description provided for @terminalImplementationAndroidSystem.
  ///
  /// In en, this message translates to:
  /// **'Android system terminal'**
  String get terminalImplementationAndroidSystem;

  /// No description provided for @terminalImplementationChroot.
  ///
  /// In en, this message translates to:
  /// **'Chroot (Root)'**
  String get terminalImplementationChroot;

  /// No description provided for @terminalImplementationTermux.
  ///
  /// In en, this message translates to:
  /// **'Termux'**
  String get terminalImplementationTermux;

  /// No description provided for @terminalProfileDirectoryMode.
  ///
  /// In en, this message translates to:
  /// **'Starting directory'**
  String get terminalProfileDirectoryMode;

  /// No description provided for @terminalProfileDirectoryHome.
  ///
  /// In en, this message translates to:
  /// **'Termux home'**
  String get terminalProfileDirectoryHome;

  /// No description provided for @terminalProfileDirectoryWorkspace.
  ///
  /// In en, this message translates to:
  /// **'Workspace (shared storage)'**
  String get terminalProfileDirectoryWorkspace;

  /// No description provided for @terminalProfileTermuxSetup.
  ///
  /// In en, this message translates to:
  /// **'Open Termux and run pkg install --no-install-recommends python. Set allow-external-apps = true in ~/.termux/termux.properties, then run termux-reload-settings. In Android settings, allow Operit to run commands in Termux.'**
  String get terminalProfileTermuxSetup;

  /// No description provided for @terminalImplementationQemu.
  ///
  /// In en, this message translates to:
  /// **'QEMU Linux emulated terminal'**
  String get terminalImplementationQemu;

  /// No description provided for @terminalImplementationArm64.
  ///
  /// In en, this message translates to:
  /// **'Linux ARM64 emulated terminal'**
  String get terminalImplementationArm64;

  /// No description provided for @terminalImplementationSystem.
  ///
  /// In en, this message translates to:
  /// **'System terminal'**
  String get terminalImplementationSystem;

  /// No description provided for @terminalImplementationBrowser.
  ///
  /// In en, this message translates to:
  /// **'Browser Linux VM'**
  String get terminalImplementationBrowser;

  /// No description provided for @terminalInterpreterShell.
  ///
  /// In en, this message translates to:
  /// **'Shell'**
  String get terminalInterpreterShell;

  /// No description provided for @terminalBuiltinMacNative.
  ///
  /// In en, this message translates to:
  /// **'macOS native {interpreter} terminal'**
  String terminalBuiltinMacNative(String interpreter);

  /// No description provided for @terminalBuiltinMacSandbox.
  ///
  /// In en, this message translates to:
  /// **'macOS sandbox {interpreter} terminal'**
  String terminalBuiltinMacSandbox(String interpreter);

  /// No description provided for @terminalBuiltinAndroidLinux.
  ///
  /// In en, this message translates to:
  /// **'Android PRoot Linux Bash terminal'**
  String get terminalBuiltinAndroidLinux;

  /// No description provided for @terminalBuiltinAndroidSystem.
  ///
  /// In en, this message translates to:
  /// **'Android system Shell terminal'**
  String get terminalBuiltinAndroidSystem;

  /// No description provided for @terminalBuiltinLinux.
  ///
  /// In en, this message translates to:
  /// **'Linux native Bash terminal'**
  String get terminalBuiltinLinux;

  /// No description provided for @terminalBuiltinPosix.
  ///
  /// In en, this message translates to:
  /// **'POSIX native Bash terminal'**
  String get terminalBuiltinPosix;

  /// No description provided for @terminalBuiltinPrivileged.
  ///
  /// In en, this message translates to:
  /// **'System privileged sh terminal'**
  String get terminalBuiltinPrivileged;

  /// No description provided for @terminalBuiltinPowerShellDownload.
  ///
  /// In en, this message translates to:
  /// **'PowerShell 7 terminal'**
  String get terminalBuiltinPowerShellDownload;

  /// No description provided for @terminalBuiltinPowerShell.
  ///
  /// In en, this message translates to:
  /// **'Windows bundled PowerShell 7 terminal'**
  String get terminalBuiltinPowerShell;

  /// No description provided for @terminalBuiltinWindowsBash.
  ///
  /// In en, this message translates to:
  /// **'Windows bundled Bash terminal'**
  String get terminalBuiltinWindowsBash;

  /// No description provided for @terminalBuiltinArm64.
  ///
  /// In en, this message translates to:
  /// **'Linux ARM64 emulated terminal'**
  String get terminalBuiltinArm64;

  /// No description provided for @terminalBuiltinIosSystem.
  ///
  /// In en, this message translates to:
  /// **'iOS system sh terminal'**
  String get terminalBuiltinIosSystem;

  /// No description provided for @terminalBuiltinOhosLinux.
  ///
  /// In en, this message translates to:
  /// **'OpenHarmony QEMU Linux emulated terminal'**
  String get terminalBuiltinOhosLinux;

  /// No description provided for @terminalBuiltinOhosSystem.
  ///
  /// In en, this message translates to:
  /// **'OpenHarmony system sh terminal'**
  String get terminalBuiltinOhosSystem;

  /// No description provided for @terminalBuiltinBrowser.
  ///
  /// In en, this message translates to:
  /// **'Browser Linux VM terminal'**
  String get terminalBuiltinBrowser;

  /// No description provided for @terminalProfilesTitle.
  ///
  /// In en, this message translates to:
  /// **'Terminal profiles'**
  String get terminalProfilesTitle;

  /// No description provided for @terminalProfilesDescription.
  ///
  /// In en, this message translates to:
  /// **'Add terminal profiles for this device and choose a default. New workspace and AI terminal sessions share this setting; existing sessions keep their original environment.'**
  String get terminalProfilesDescription;

  /// No description provided for @terminalProfileAdd.
  ///
  /// In en, this message translates to:
  /// **'Add terminal profile'**
  String get terminalProfileAdd;

  /// No description provided for @terminalProfileEdit.
  ///
  /// In en, this message translates to:
  /// **'Edit terminal profile'**
  String get terminalProfileEdit;

  /// No description provided for @terminalProfileDelete.
  ///
  /// In en, this message translates to:
  /// **'Delete terminal profile'**
  String get terminalProfileDelete;

  /// No description provided for @terminalProfileDeleteDescription.
  ///
  /// In en, this message translates to:
  /// **'This profile will no longer be available for new terminals. Running sessions stay open. Select another default before deleting the default profile.'**
  String get terminalProfileDeleteDescription;

  /// No description provided for @terminalProfileName.
  ///
  /// In en, this message translates to:
  /// **'Profile name'**
  String get terminalProfileName;

  /// No description provided for @terminalProfileNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter a profile name'**
  String get terminalProfileNameRequired;

  /// No description provided for @terminalProfileImplementation.
  ///
  /// In en, this message translates to:
  /// **'Terminal implementation and interpreter'**
  String get terminalProfileImplementation;

  /// No description provided for @terminalProfileRootfs.
  ///
  /// In en, this message translates to:
  /// **'Root filesystem directory'**
  String get terminalProfileRootfs;

  /// No description provided for @terminalProfileChrootRootfsHelp.
  ///
  /// In en, this message translates to:
  /// **'Requires Root authorization and the absolute path to a trusted Linux root filesystem containing /bin/bash.'**
  String get terminalProfileChrootRootfsHelp;

  /// No description provided for @terminalProfileRootfsRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter the root filesystem path.'**
  String get terminalProfileRootfsRequired;

  /// No description provided for @terminalProfileMounts.
  ///
  /// In en, this message translates to:
  /// **'Directory mounts'**
  String get terminalProfileMounts;

  /// No description provided for @terminalProfileAddMount.
  ///
  /// In en, this message translates to:
  /// **'Add mount'**
  String get terminalProfileAddMount;

  /// No description provided for @terminalProfileMountSource.
  ///
  /// In en, this message translates to:
  /// **'Host directory'**
  String get terminalProfileMountSource;

  /// No description provided for @terminalProfileMountTarget.
  ///
  /// In en, this message translates to:
  /// **'Guest directory'**
  String get terminalProfileMountTarget;

  /// No description provided for @terminalProfileMountPathRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter a directory path.'**
  String get terminalProfileMountPathRequired;

  /// No description provided for @terminalProfileRootfsHelp.
  ///
  /// In en, this message translates to:
  /// **'Enter an accessible absolute path to an extracted Linux root filesystem matching this device\'s architecture, such as Ubuntu. Android requires /bin/bash; OpenHarmony requires /bin/sh. Leave blank to use the built-in environment.'**
  String get terminalProfileRootfsHelp;

  /// No description provided for @terminalProfileExecutable.
  ///
  /// In en, this message translates to:
  /// **'Interpreter executable path'**
  String get terminalProfileExecutable;

  /// No description provided for @terminalProfileArguments.
  ///
  /// In en, this message translates to:
  /// **'Launcher arguments (JSON array)'**
  String get terminalProfileArguments;

  /// No description provided for @terminalProfileArgumentsHelp.
  ///
  /// In en, this message translates to:
  /// **'For example [\"--login\"]. Arguments precede the managed interactive-shell arguments. Enter an executable path when specifying launch options. PowerShell 5 and 7 can be added as separate profiles.'**
  String get terminalProfileArgumentsHelp;

  /// No description provided for @terminalProfileArgumentsError.
  ///
  /// In en, this message translates to:
  /// **'Enter a JSON array of strings, for example [\"--login\"]'**
  String get terminalProfileArgumentsError;

  /// No description provided for @terminalProfileNoOptions.
  ///
  /// In en, this message translates to:
  /// **'This terminal is managed by the platform and can be selected as the default. The host does not expose additional launch options.'**
  String get terminalProfileNoOptions;

  /// No description provided for @chatPromoteToRoot.
  ///
  /// In en, this message translates to:
  /// **'Promote to main conversation'**
  String get chatPromoteToRoot;

  /// No description provided for @workspaceUnsavedChangesTitle.
  ///
  /// In en, this message translates to:
  /// **'Unsaved changes'**
  String get workspaceUnsavedChangesTitle;

  /// No description provided for @workspaceUnsavedChangesMessage.
  ///
  /// In en, this message translates to:
  /// **'Save your changes before closing this file?'**
  String get workspaceUnsavedChangesMessage;

  /// No description provided for @workspaceDiscardChanges.
  ///
  /// In en, this message translates to:
  /// **'Discard changes'**
  String get workspaceDiscardChanges;

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

  /// No description provided for @noticeExpand.
  ///
  /// In en, this message translates to:
  /// **'Expand'**
  String get noticeExpand;

  /// No description provided for @noticeCollapse.
  ///
  /// In en, this message translates to:
  /// **'Collapse'**
  String get noticeCollapse;

  /// No description provided for @expandInput.
  ///
  /// In en, this message translates to:
  /// **'Expand input'**
  String get expandInput;

  /// No description provided for @collapseInput.
  ///
  /// In en, this message translates to:
  /// **'Collapse input'**
  String get collapseInput;

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

  /// No description provided for @attachmentPhoto.
  ///
  /// In en, this message translates to:
  /// **'Photo'**
  String get attachmentPhoto;

  /// No description provided for @attachmentCamera.
  ///
  /// In en, this message translates to:
  /// **'Camera'**
  String get attachmentCamera;

  /// No description provided for @attachmentFile.
  ///
  /// In en, this message translates to:
  /// **'File'**
  String get attachmentFile;

  /// No description provided for @attachmentScreenContent.
  ///
  /// In en, this message translates to:
  /// **'Screen content'**
  String get attachmentScreenContent;

  /// No description provided for @attachmentNotifications.
  ///
  /// In en, this message translates to:
  /// **'Current notifications'**
  String get attachmentNotifications;

  /// No description provided for @attachmentLocation.
  ///
  /// In en, this message translates to:
  /// **'Current location'**
  String get attachmentLocation;

  /// No description provided for @attachmentPackage.
  ///
  /// In en, this message translates to:
  /// **'Package'**
  String get attachmentPackage;

  /// No description provided for @attachmentPackageSelectTitle.
  ///
  /// In en, this message translates to:
  /// **'Select package'**
  String get attachmentPackageSelectTitle;

  /// No description provided for @attachmentPackageEmpty.
  ///
  /// In en, this message translates to:
  /// **'No available packages'**
  String get attachmentPackageEmpty;

  /// No description provided for @attachmentPackageSearchPlaceholder.
  ///
  /// In en, this message translates to:
  /// **'Search package name or description'**
  String get attachmentPackageSearchPlaceholder;

  /// No description provided for @attachmentPackageSearchEmpty.
  ///
  /// In en, this message translates to:
  /// **'No matching packages'**
  String get attachmentPackageSearchEmpty;

  /// No description provided for @attachmentPackageKindPackage.
  ///
  /// In en, this message translates to:
  /// **'Package'**
  String get attachmentPackageKindPackage;

  /// No description provided for @attachmentPackageKindSkill.
  ///
  /// In en, this message translates to:
  /// **'Skill'**
  String get attachmentPackageKindSkill;

  /// No description provided for @attachmentPackageKindMcp.
  ///
  /// In en, this message translates to:
  /// **'MCP'**
  String get attachmentPackageKindMcp;

  /// No description provided for @attachmentCameraUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Camera capture is not available in the Flutter client'**
  String get attachmentCameraUnavailable;

  /// No description provided for @clearSearch.
  ///
  /// In en, this message translates to:
  /// **'Clear search'**
  String get clearSearch;

  /// No description provided for @chatPendingQueueTitle.
  ///
  /// In en, this message translates to:
  /// **'Queued messages ({count})'**
  String chatPendingQueueTitle(int count);

  /// No description provided for @chatQueueAddMessage.
  ///
  /// In en, this message translates to:
  /// **'Queue message'**
  String get chatQueueAddMessage;

  /// No description provided for @chatQueueAdded.
  ///
  /// In en, this message translates to:
  /// **'Added to queue'**
  String get chatQueueAdded;

  /// No description provided for @chatSendOptions.
  ///
  /// In en, this message translates to:
  /// **'Choose how to send'**
  String get chatSendOptions;

  /// No description provided for @chatQueueAddMessageDescription.
  ///
  /// In en, this message translates to:
  /// **'Send after the current reply finishes (default)'**
  String get chatQueueAddMessageDescription;

  /// No description provided for @chatGuideCurrentTurn.
  ///
  /// In en, this message translates to:
  /// **'Guide current conversation'**
  String get chatGuideCurrentTurn;

  /// No description provided for @chatGuideCurrentTurnDescription.
  ///
  /// In en, this message translates to:
  /// **'Apply in a later model request without interrupting this turn'**
  String get chatGuideCurrentTurnDescription;

  /// No description provided for @chatInterruptAndSend.
  ///
  /// In en, this message translates to:
  /// **'Interrupt and send'**
  String get chatInterruptAndSend;

  /// No description provided for @chatInterruptAndSendDescription.
  ///
  /// In en, this message translates to:
  /// **'Stop the current reply and send this message now'**
  String get chatInterruptAndSendDescription;

  /// No description provided for @chatGuidanceAccepted.
  ///
  /// In en, this message translates to:
  /// **'Guidance added for a later model request'**
  String get chatGuidanceAccepted;

  /// No description provided for @chatGuidanceTextOnly.
  ///
  /// In en, this message translates to:
  /// **'Guidance supports text only. Remove attachments and the reply target first.'**
  String get chatGuidanceTextOnly;

  /// No description provided for @chatQueueWithdrawToEditor.
  ///
  /// In en, this message translates to:
  /// **'Withdraw to editor'**
  String get chatQueueWithdrawToEditor;

  /// No description provided for @chatPleaseCreateNewChat.
  ///
  /// In en, this message translates to:
  /// **'Please create a chat'**
  String get chatPleaseCreateNewChat;

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

  /// No description provided for @summarizingMemories.
  ///
  /// In en, this message translates to:
  /// **'Summarizing memories...'**
  String get summarizingMemories;

  /// No description provided for @summaryGenerating.
  ///
  /// In en, this message translates to:
  /// **'Generating summary...'**
  String get summaryGenerating;

  /// No description provided for @summaryStageCore.
  ///
  /// In en, this message translates to:
  /// **'Summarizing the core task...'**
  String get summaryStageCore;

  /// No description provided for @summaryStageConstraints.
  ///
  /// In en, this message translates to:
  /// **'Summarizing user requests and constraints...'**
  String get summaryStageConstraints;

  /// No description provided for @summaryStageFiles.
  ///
  /// In en, this message translates to:
  /// **'Summarizing key files and code...'**
  String get summaryStageFiles;

  /// No description provided for @summaryStageErrors.
  ///
  /// In en, this message translates to:
  /// **'Summarizing errors and fixes...'**
  String get summaryStageErrors;

  /// No description provided for @summaryStageInteraction.
  ///
  /// In en, this message translates to:
  /// **'Summarizing interaction and context...'**
  String get summaryStageInteraction;

  /// No description provided for @summaryStageNext.
  ///
  /// In en, this message translates to:
  /// **'Summarizing next steps...'**
  String get summaryStageNext;

  /// No description provided for @summaryStageFormatting.
  ///
  /// In en, this message translates to:
  /// **'Formatting the summary...'**
  String get summaryStageFormatting;

  /// No description provided for @chatCompressingHistory.
  ///
  /// In en, this message translates to:
  /// **'Compressing history records...'**
  String get chatCompressingHistory;

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

  /// No description provided for @statusWarningAiErrorSummary.
  ///
  /// In en, this message translates to:
  /// **'The AI made an error'**
  String get statusWarningAiErrorSummary;

  /// No description provided for @statusWarningAiErrorDetailTitle.
  ///
  /// In en, this message translates to:
  /// **'AI Error Reason'**
  String get statusWarningAiErrorDetailTitle;

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

  /// No description provided for @workspaceCloseOtherTabs.
  ///
  /// In en, this message translates to:
  /// **'Close Others'**
  String get workspaceCloseOtherTabs;

  /// No description provided for @workspaceDockDropTarget.
  ///
  /// In en, this message translates to:
  /// **'Drop into workspace'**
  String get workspaceDockDropTarget;

  /// No description provided for @workspaceOpenWith.
  ///
  /// In en, this message translates to:
  /// **'Open with'**
  String get workspaceOpenWith;

  /// No description provided for @workspaceBuiltInPreview.
  ///
  /// In en, this message translates to:
  /// **'Built-in preview'**
  String get workspaceBuiltInPreview;

  /// No description provided for @workspacePluginUnsavedChanges.
  ///
  /// In en, this message translates to:
  /// **'Save in the editor first, or discard the unsaved changes.'**
  String get workspacePluginUnsavedChanges;

  /// No description provided for @workspaceCloseTabsToRight.
  ///
  /// In en, this message translates to:
  /// **'Close to the Right'**
  String get workspaceCloseTabsToRight;

  /// No description provided for @workspaceCloseAllTabs.
  ///
  /// In en, this message translates to:
  /// **'Close All'**
  String get workspaceCloseAllTabs;

  /// No description provided for @commonExpand.
  ///
  /// In en, this message translates to:
  /// **'Expand'**
  String get commonExpand;

  /// No description provided for @commonCollapse.
  ///
  /// In en, this message translates to:
  /// **'Collapse'**
  String get commonCollapse;

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
  /// **'Always allow in this session'**
  String get toolApprovalAlwaysAllow;

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

  /// No description provided for @chatLockedCannotDelete.
  ///
  /// In en, this message translates to:
  /// **'This chat is locked and cannot be deleted'**
  String get chatLockedCannotDelete;

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

  /// No description provided for @messageLocatorSearchMessages.
  ///
  /// In en, this message translates to:
  /// **'Search messages'**
  String get messageLocatorSearchMessages;

  /// No description provided for @messageLocatorViewFavorites.
  ///
  /// In en, this message translates to:
  /// **'View favorites'**
  String get messageLocatorViewFavorites;

  /// No description provided for @messageLocatorAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get messageLocatorAll;

  /// No description provided for @messageLocatorFavorites.
  ///
  /// In en, this message translates to:
  /// **'Favorites'**
  String get messageLocatorFavorites;

  /// No description provided for @messageLocatorNoFavorites.
  ///
  /// In en, this message translates to:
  /// **'No favorite messages yet'**
  String get messageLocatorNoFavorites;

  /// No description provided for @messageLocatorLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not load messages. Please try again.'**
  String get messageLocatorLoadFailed;

  /// No description provided for @messageTransformFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to process message display: {error}'**
  String messageTransformFailed(String error);

  /// No description provided for @messageLocatorFavoriteFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not update favorite. Please try again.'**
  String get messageLocatorFavoriteFailed;

  /// No description provided for @messageLocatorAddFavorite.
  ///
  /// In en, this message translates to:
  /// **'Favorite message'**
  String get messageLocatorAddFavorite;

  /// No description provided for @messageLocatorRemoveFavorite.
  ///
  /// In en, this message translates to:
  /// **'Remove favorite'**
  String get messageLocatorRemoveFavorite;

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

  /// No description provided for @historyDialogSummary.
  ///
  /// In en, this message translates to:
  /// **'History Dialog Summary'**
  String get historyDialogSummary;

  /// No description provided for @confirmDeleteSummary.
  ///
  /// In en, this message translates to:
  /// **'Delete Summary'**
  String get confirmDeleteSummary;

  /// No description provided for @confirmDeleteSummaryMessage.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to delete this summary?'**
  String get confirmDeleteSummaryMessage;

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

  /// No description provided for @thinkingProcess.
  ///
  /// In en, this message translates to:
  /// **'Thinking Process'**
  String get thinkingProcess;

  /// No description provided for @thinkingToolsGroupTitleWithCount.
  ///
  /// In en, this message translates to:
  /// **'Thinking & Tool Calls ({count})'**
  String thinkingToolsGroupTitleWithCount(int count);

  /// No description provided for @toolsGroupTitleWithCount.
  ///
  /// In en, this message translates to:
  /// **'Tool Calls ({count})'**
  String toolsGroupTitleWithCount(int count);

  /// No description provided for @searchGroupTitle.
  ///
  /// In en, this message translates to:
  /// **'Search'**
  String get searchGroupTitle;

  /// No description provided for @searchSources.
  ///
  /// In en, this message translates to:
  /// **'Search Sources'**
  String get searchSources;

  /// No description provided for @thinkingSearchGroupTitle.
  ///
  /// In en, this message translates to:
  /// **'Thinking and searching'**
  String get thinkingSearchGroupTitle;

  /// No description provided for @thinkingSearchToolsGroupTitleWithCount.
  ///
  /// In en, this message translates to:
  /// **'Thinking, searching, and calling tools ({count})'**
  String thinkingSearchToolsGroupTitleWithCount(int count);

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

  /// No description provided for @autoTriggeredUserMessage.
  ///
  /// In en, this message translates to:
  /// **'AUTO'**
  String get autoTriggeredUserMessage;

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

  /// No description provided for @workspaceCreateTitle.
  ///
  /// In en, this message translates to:
  /// **'Create workspace'**
  String get workspaceCreateTitle;

  /// No description provided for @workspaceCreateDescription.
  ///
  /// In en, this message translates to:
  /// **'Create a named workspace'**
  String get workspaceCreateDescription;

  /// No description provided for @workspaceNameLabel.
  ///
  /// In en, this message translates to:
  /// **'Workspace name'**
  String get workspaceNameLabel;

  /// No description provided for @workspaceNameHint.
  ///
  /// In en, this message translates to:
  /// **'Enter a workspace name'**
  String get workspaceNameHint;

  /// No description provided for @workspaceBindExistingTitle.
  ///
  /// In en, this message translates to:
  /// **'Choose workspace'**
  String get workspaceBindExistingTitle;

  /// No description provided for @workspacePickLocalFolder.
  ///
  /// In en, this message translates to:
  /// **'Choose a folder on this device'**
  String get workspacePickLocalFolder;

  /// No description provided for @workspaceBindExistingDescription.
  ///
  /// In en, this message translates to:
  /// **'Bind an existing workspace'**
  String get workspaceBindExistingDescription;

  /// No description provided for @workspaceAddFolderTitle.
  ///
  /// In en, this message translates to:
  /// **'Add folder'**
  String get workspaceAddFolderTitle;

  /// No description provided for @workspaceAddFolderDescription.
  ///
  /// In en, this message translates to:
  /// **'Mount another folder into the current workspace'**
  String get workspaceAddFolderDescription;

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

  /// No description provided for @workspaceUnbindTitle.
  ///
  /// In en, this message translates to:
  /// **'Remove project from this chat'**
  String get workspaceUnbindTitle;

  /// No description provided for @workspaceUnbindDescription.
  ///
  /// In en, this message translates to:
  /// **'Detach this chat without deleting workspace files'**
  String get workspaceUnbindDescription;

  /// No description provided for @workspaceUnbindConfirmation.
  ///
  /// In en, this message translates to:
  /// **'Unbind the workspace from this chat? The workspace and its files will be kept, and other chats will not be affected. You can bind a workspace again later.'**
  String get workspaceUnbindConfirmation;

  /// No description provided for @workspaceUnbindFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to unbind workspace. Please try again.'**
  String get workspaceUnbindFailed;

  /// No description provided for @workspaceUnbindChanged.
  ///
  /// In en, this message translates to:
  /// **'The current chat or workspace has changed. Cancel and try again.'**
  String get workspaceUnbindChanged;

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

  /// No description provided for @packageConfigureEnvironmentVariables.
  ///
  /// In en, this message translates to:
  /// **'Configure Environment Variables'**
  String get packageConfigureEnvironmentVariables;

  /// No description provided for @packageNoEnvironmentVariables.
  ///
  /// In en, this message translates to:
  /// **'The currently enabled tool packages have not declared any environment variables.'**
  String get packageNoEnvironmentVariables;

  /// No description provided for @packageEnvironmentOptional.
  ///
  /// In en, this message translates to:
  /// **'Optional'**
  String get packageEnvironmentOptional;

  /// No description provided for @packageEnvironmentInputRequired.
  ///
  /// In en, this message translates to:
  /// **'Input value (required)'**
  String get packageEnvironmentInputRequired;

  /// No description provided for @packageEnvironmentInputOptional.
  ///
  /// In en, this message translates to:
  /// **'Input value (optional)'**
  String get packageEnvironmentInputOptional;

  /// No description provided for @packageEnvironmentDefault.
  ///
  /// In en, this message translates to:
  /// **'Default: {value}'**
  String packageEnvironmentDefault(String value);

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

  /// No description provided for @chatSpeechInputFailed.
  ///
  /// In en, this message translates to:
  /// **'Speech input failed: {error}'**
  String chatSpeechInputFailed(Object error);

  /// No description provided for @chatSpeechInputConfigurationRequired.
  ///
  /// In en, this message translates to:
  /// **'Select a speech recognition configuration in Settings > Models & AI before using speech input.'**
  String get chatSpeechInputConfigurationRequired;

  /// No description provided for @chatSpeechNoTextRecognized.
  ///
  /// In en, this message translates to:
  /// **'No speech recognized.'**
  String get chatSpeechNoTextRecognized;

  /// No description provided for @chatVoiceListening.
  ///
  /// In en, this message translates to:
  /// **'Listening…'**
  String get chatVoiceListening;

  /// No description provided for @chatVoiceStopListening.
  ///
  /// In en, this message translates to:
  /// **'Stop listening'**
  String get chatVoiceStopListening;

  /// No description provided for @chatVoiceThinking.
  ///
  /// In en, this message translates to:
  /// **'thinking'**
  String get chatVoiceThinking;

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

  /// No description provided for @browserDownloadsAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get browserDownloadsAll;

  /// No description provided for @browserDownloadStorageHint.
  ///
  /// In en, this message translates to:
  /// **'Files are kept in the browser\'s download folder on the browser device. Open copies the file into this workspace first. You can also save it to the workspace directly.'**
  String get browserDownloadStorageHint;

  /// No description provided for @browserDownloadWorkspaceTarget.
  ///
  /// In en, this message translates to:
  /// **'Save to this workspace:'**
  String get browserDownloadWorkspaceTarget;

  /// No description provided for @browserDownloadSaveToWorkspace.
  ///
  /// In en, this message translates to:
  /// **'Save to workspace'**
  String get browserDownloadSaveToWorkspace;

  /// No description provided for @browserDownloadInterrupted.
  ///
  /// In en, this message translates to:
  /// **'Interrupted · download again'**
  String get browserDownloadInterrupted;

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

  /// No description provided for @openInOtherApp.
  ///
  /// In en, this message translates to:
  /// **'Open in other app'**
  String get openInOtherApp;

  /// No description provided for @workspaceFileOpening.
  ///
  /// In en, this message translates to:
  /// **'Opening file…'**
  String get workspaceFileOpening;

  /// No description provided for @workspaceFileOpenTimedOut.
  ///
  /// In en, this message translates to:
  /// **'Opening the file timed out. Please try again.'**
  String get workspaceFileOpenTimedOut;

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

  /// No description provided for @browserExternalNavigationFailed.
  ///
  /// In en, this message translates to:
  /// **'Unable to open the link. No compatible application is installed, the link is invalid, or the system rejected the request.'**
  String get browserExternalNavigationFailed;

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

  /// No description provided for @settingsCategoryLocalModelsTitle.
  ///
  /// In en, this message translates to:
  /// **'Local Models'**
  String get settingsCategoryLocalModelsTitle;

  /// No description provided for @settingsCategoryLocalModelsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'LLMs, downloads, STT / TTS'**
  String get settingsCategoryLocalModelsSubtitle;

  /// No description provided for @settingsCategoryLocalModelsDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage local models and inference engines installed on demand.'**
  String get settingsCategoryLocalModelsDescription;

  /// No description provided for @settingsCategoryToolsTitle.
  ///
  /// In en, this message translates to:
  /// **'Tools & Permissions'**
  String get settingsCategoryToolsTitle;

  /// No description provided for @settingsCategoryToolsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'AI capability, system authorization, extensions'**
  String get settingsCategoryToolsSubtitle;

  /// No description provided for @settingsCategoryToolsDescription.
  ///
  /// In en, this message translates to:
  /// **'Set AI read-only, read-write, or full access, and review the current device system authorization status.'**
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

  /// No description provided for @settingsCategoryGlobalBehaviorTitle.
  ///
  /// In en, this message translates to:
  /// **'Global Behavior Settings'**
  String get settingsCategoryGlobalBehaviorTitle;

  /// No description provided for @settingsCategoryGlobalBehaviorSubtitle.
  ///
  /// In en, this message translates to:
  /// **'App language, chat layout and input'**
  String get settingsCategoryGlobalBehaviorSubtitle;

  /// No description provided for @settingsCategoryGlobalBehaviorDescription.
  ///
  /// In en, this message translates to:
  /// **'Choose the app language and configure application-wide chat layout, input and interaction behavior.'**
  String get settingsCategoryGlobalBehaviorDescription;

  /// No description provided for @settingsCategoryAppearanceTitle.
  ///
  /// In en, this message translates to:
  /// **'Appearance & Interaction'**
  String get settingsCategoryAppearanceTitle;

  /// No description provided for @settingsCategoryAppearanceSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Theme, bubbles, and interaction'**
  String get settingsCategoryAppearanceSubtitle;

  /// No description provided for @settingsCategoryAppearanceDescription.
  ///
  /// In en, this message translates to:
  /// **'Customize global theme, background media, message bubbles, and chat interaction.'**
  String get settingsCategoryAppearanceDescription;

  /// No description provided for @settingsCategoryDataTitle.
  ///
  /// In en, this message translates to:
  /// **'Data & Backup'**
  String get settingsCategoryDataTitle;

  /// No description provided for @settingsCategoryDataSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Backup, restore, storage'**
  String get settingsCategoryDataSubtitle;

  /// No description provided for @settingsCategoryDataDescription.
  ///
  /// In en, this message translates to:
  /// **'Back up and restore chats, plugin data, and model settings, and manage storage.'**
  String get settingsCategoryDataDescription;

  /// No description provided for @settingsUsageReportTitle.
  ///
  /// In en, this message translates to:
  /// **'Token Usage Report'**
  String get settingsUsageReportTitle;

  /// No description provided for @settingsUsageReportSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Model usage, cache, and cost'**
  String get settingsUsageReportSubtitle;

  /// No description provided for @settingsCategoryAccessLinksTitle.
  ///
  /// In en, this message translates to:
  /// **'Device Space'**
  String get settingsCategoryAccessLinksTitle;

  /// No description provided for @settingsCategoryAccessLinksSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Connect, sync, collaborate'**
  String get settingsCategoryAccessLinksSubtitle;

  /// No description provided for @settingsCategoryAccessLinksDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage this device space, connect devices, and share data and routes.'**
  String get settingsCategoryAccessLinksDescription;

  /// No description provided for @settingsCategoryGroupAssistant.
  ///
  /// In en, this message translates to:
  /// **'AI & Creation'**
  String get settingsCategoryGroupAssistant;

  /// No description provided for @settingsCategoryGroupWorkspace.
  ///
  /// In en, this message translates to:
  /// **'Workspace & Automation'**
  String get settingsCategoryGroupWorkspace;

  /// No description provided for @settingsCategoryGroupExperience.
  ///
  /// In en, this message translates to:
  /// **'Display & Interaction'**
  String get settingsCategoryGroupExperience;

  /// No description provided for @settingsCategoryGroupSystem.
  ///
  /// In en, this message translates to:
  /// **'Data & System'**
  String get settingsCategoryGroupSystem;

  /// No description provided for @settingsGlobalBehaviorChatInputSection.
  ///
  /// In en, this message translates to:
  /// **'Chat input'**
  String get settingsGlobalBehaviorChatInputSection;

  /// No description provided for @settingsGlobalBehaviorChatLayoutSection.
  ///
  /// In en, this message translates to:
  /// **'Chat layout'**
  String get settingsGlobalBehaviorChatLayoutSection;

  /// No description provided for @settingsGlobalBehaviorChatScrollNavigatorOnLeft.
  ///
  /// In en, this message translates to:
  /// **'Show message navigator on the left'**
  String get settingsGlobalBehaviorChatScrollNavigatorOnLeft;

  /// No description provided for @settingsGlobalBehaviorChatScrollNavigatorOnLeftDescription.
  ///
  /// In en, this message translates to:
  /// **'Mirror the navigator on the left side of the chat. It appears on the right by default.'**
  String get settingsGlobalBehaviorChatScrollNavigatorOnLeftDescription;

  /// No description provided for @settingsGlobalBehaviorEnterSendsMessage.
  ///
  /// In en, this message translates to:
  /// **'Press Enter to send'**
  String get settingsGlobalBehaviorEnterSendsMessage;

  /// No description provided for @settingsGlobalBehaviorEnterSendsMessageDescription.
  ///
  /// In en, this message translates to:
  /// **'When off, Enter inserts a newline. Ctrl+Enter and Enter in the expanded editor always insert a newline.'**
  String get settingsGlobalBehaviorEnterSendsMessageDescription;

  /// No description provided for @settingsGlobalBehaviorLongPastedTextAsAttachment.
  ///
  /// In en, this message translates to:
  /// **'Convert long pasted text to a file'**
  String get settingsGlobalBehaviorLongPastedTextAsAttachment;

  /// No description provided for @settingsGlobalBehaviorLongPastedTextThreshold.
  ///
  /// In en, this message translates to:
  /// **'Conversion threshold'**
  String get settingsGlobalBehaviorLongPastedTextThreshold;

  /// No description provided for @settingsGlobalBehaviorLongPastedTextThresholdValue.
  ///
  /// In en, this message translates to:
  /// **'{count} characters'**
  String settingsGlobalBehaviorLongPastedTextThresholdValue(int count);

  /// No description provided for @settingsComingSoon.
  ///
  /// In en, this message translates to:
  /// **'This area will continue connecting existing runtime capabilities. Model and tool settings are being completed first.'**
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

  /// No description provided for @settingsModelCurrentActive.
  ///
  /// In en, this message translates to:
  /// **'Active'**
  String get settingsModelCurrentActive;

  /// No description provided for @settingsModelMultimodalBadge.
  ///
  /// In en, this message translates to:
  /// **'Multimodal'**
  String get settingsModelMultimodalBadge;

  /// No description provided for @settingsModelNoModels.
  ///
  /// In en, this message translates to:
  /// **'No models yet'**
  String get settingsModelNoModels;

  /// No description provided for @settingsModelProviderModelCount.
  ///
  /// In en, this message translates to:
  /// **'{count} models'**
  String settingsModelProviderModelCount(int count);

  /// No description provided for @settingsModelSetCurrentActive.
  ///
  /// In en, this message translates to:
  /// **'Set active'**
  String get settingsModelSetCurrentActive;

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

  /// No description provided for @settingsModelFunctionFollowChat.
  ///
  /// In en, this message translates to:
  /// **'Follow chat'**
  String get settingsModelFunctionFollowChat;

  /// No description provided for @settingsModelFunctionFollowChatAll.
  ///
  /// In en, this message translates to:
  /// **'Follow all'**
  String get settingsModelFunctionFollowChatAll;

  /// No description provided for @settingsModelFunctionFollowChatHint.
  ///
  /// In en, this message translates to:
  /// **'Automatically uses the current chat model'**
  String get settingsModelFunctionFollowChatHint;

  /// No description provided for @settingsModelFunctionGroupBackground.
  ///
  /// In en, this message translates to:
  /// **'Background helpers'**
  String get settingsModelFunctionGroupBackground;

  /// No description provided for @settingsModelFunctionGroupMain.
  ///
  /// In en, this message translates to:
  /// **'Main chat model'**
  String get settingsModelFunctionGroupMain;

  /// No description provided for @settingsModelFunctionGroupMultimodal.
  ///
  /// In en, this message translates to:
  /// **'Multimodal recognition'**
  String get settingsModelFunctionGroupMultimodal;

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

  /// No description provided for @settingsModelFunctionMappingsMissing.
  ///
  /// In en, this message translates to:
  /// **'Bound model does not exist: {providerId} · {modelId}'**
  String settingsModelFunctionMappingsMissing(
    String providerId,
    String modelId,
  );

  /// No description provided for @settingsModelDeleteBlocked.
  ///
  /// In en, this message translates to:
  /// **'This model is used by these functions. Change their model assignments first: {functions}'**
  String settingsModelDeleteBlocked(String functions);

  /// No description provided for @settingsModelDeleteProviderBlocked.
  ///
  /// In en, this message translates to:
  /// **'Models under this provider are used by these functions. Change their model assignments first: {functions}'**
  String settingsModelDeleteProviderBlocked(String functions);

  /// No description provided for @settingsModelDeleteProviderConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete provider “{name}”? This will also delete its {count} models.'**
  String settingsModelDeleteProviderConfirm(String name, int count);

  /// No description provided for @settingsModelDeleteProviderConfirmAction.
  ///
  /// In en, this message translates to:
  /// **'Delete provider'**
  String get settingsModelDeleteProviderConfirmAction;

  /// No description provided for @settingsTtsDeleteProvider.
  ///
  /// In en, this message translates to:
  /// **'Delete TTS provider'**
  String get settingsTtsDeleteProvider;

  /// No description provided for @settingsTtsDeleteProviderConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete TTS provider “{name}” and its {count} voice configurations?'**
  String settingsTtsDeleteProviderConfirm(String name, int count);

  /// No description provided for @settingsTtsDeleteProviderFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to delete TTS provider: {error}'**
  String settingsTtsDeleteProviderFailed(String error);

  /// No description provided for @settingsTtsCurrentConfigCannotDelete.
  ///
  /// In en, this message translates to:
  /// **'The TTS configuration currently in use cannot be deleted.'**
  String get settingsTtsCurrentConfigCannotDelete;

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

  /// No description provided for @settingsModelFunctionTitleGeneration.
  ///
  /// In en, this message translates to:
  /// **'Conversation title'**
  String get settingsModelFunctionTitleGeneration;

  /// No description provided for @settingsModelFunctionTitleGenerationDescription.
  ///
  /// In en, this message translates to:
  /// **'Model used to summarize the first message and attachments into a conversation title.'**
  String get settingsModelFunctionTitleGenerationDescription;

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
  /// **'Model used to plan speaking roles and order in group conversations.'**
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

  /// No description provided for @settingsModelShowApiKey.
  ///
  /// In en, this message translates to:
  /// **'Show API key'**
  String get settingsModelShowApiKey;

  /// No description provided for @settingsModelHideApiKey.
  ///
  /// In en, this message translates to:
  /// **'Hide API key'**
  String get settingsModelHideApiKey;

  /// No description provided for @settingsModelCodexLogin.
  ///
  /// In en, this message translates to:
  /// **'ChatGPT login'**
  String get settingsModelCodexLogin;

  /// No description provided for @settingsModelCodexLoginTitle.
  ///
  /// In en, this message translates to:
  /// **'Sign in to ChatGPT Codex'**
  String get settingsModelCodexLoginTitle;

  /// No description provided for @settingsModelCodexLoginDescription.
  ///
  /// In en, this message translates to:
  /// **'OpenAI Codex uses a ChatGPT subscription sign-in instead of an API key.'**
  String get settingsModelCodexLoginDescription;

  /// No description provided for @settingsModelCodexSignedIn.
  ///
  /// In en, this message translates to:
  /// **'Signed in as {account}'**
  String settingsModelCodexSignedIn(String account);

  /// No description provided for @settingsModelCodexSignedOut.
  ///
  /// In en, this message translates to:
  /// **'Not signed in to ChatGPT'**
  String get settingsModelCodexSignedOut;

  /// No description provided for @settingsModelCodexBrowser.
  ///
  /// In en, this message translates to:
  /// **'Browser sign-in'**
  String get settingsModelCodexBrowser;

  /// No description provided for @settingsModelCodexDevice.
  ///
  /// In en, this message translates to:
  /// **'Device code'**
  String get settingsModelCodexDevice;

  /// No description provided for @settingsModelCodexWaiting.
  ///
  /// In en, this message translates to:
  /// **'Waiting for ChatGPT authorization…'**
  String get settingsModelCodexWaiting;

  /// No description provided for @settingsModelCodexDeviceInstructions.
  ///
  /// In en, this message translates to:
  /// **'Enter this code on the opened page, then return to Operit.'**
  String get settingsModelCodexDeviceInstructions;

  /// No description provided for @settingsModelCodexOpenPage.
  ///
  /// In en, this message translates to:
  /// **'Open verification page'**
  String get settingsModelCodexOpenPage;

  /// No description provided for @settingsModelCodexLogout.
  ///
  /// In en, this message translates to:
  /// **'Sign out'**
  String get settingsModelCodexLogout;

  /// No description provided for @settingsModelCodexLoginFailed.
  ///
  /// In en, this message translates to:
  /// **'ChatGPT sign-in failed: {error}'**
  String settingsModelCodexLoginFailed(String error);

  /// No description provided for @settingsModelApiKeyPool.
  ///
  /// In en, this message translates to:
  /// **'API key pool'**
  String get settingsModelApiKeyPool;

  /// No description provided for @settingsModelApiKeyPoolDescription.
  ///
  /// In en, this message translates to:
  /// **'Keep using a healthy key. Switch on authentication, quota, rate limit, or temporary failures; cooling keys are skipped.'**
  String get settingsModelApiKeyPoolDescription;

  /// No description provided for @settingsModelApiKeyPoolCount.
  ///
  /// In en, this message translates to:
  /// **'{count} keys'**
  String settingsModelApiKeyPoolCount(int count);

  /// No description provided for @settingsModelApiKeyPoolEmpty.
  ///
  /// In en, this message translates to:
  /// **'No keys yet. Add a key before enabling this pool.'**
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

  /// No description provided for @settingsModelPoolAddSingle.
  ///
  /// In en, this message translates to:
  /// **'Add current single key'**
  String get settingsModelPoolAddSingle;

  /// No description provided for @settingsModelPoolPaste.
  ///
  /// In en, this message translates to:
  /// **'Paste one key per line to add several at once.'**
  String get settingsModelPoolPaste;

  /// No description provided for @settingsModelPoolNoEligible.
  ///
  /// In en, this message translates to:
  /// **'Enable at least one nonempty key that is not marked unavailable.'**
  String get settingsModelPoolNoEligible;

  /// No description provided for @settingsModelPoolUntested.
  ///
  /// In en, this message translates to:
  /// **'Untested'**
  String get settingsModelPoolUntested;

  /// No description provided for @settingsModelPoolAvailable.
  ///
  /// In en, this message translates to:
  /// **'Available'**
  String get settingsModelPoolAvailable;

  /// No description provided for @settingsModelPoolUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Unavailable'**
  String get settingsModelPoolUnavailable;

  /// No description provided for @settingsModelPoolClearMarks.
  ///
  /// In en, this message translates to:
  /// **'Reset key health'**
  String get settingsModelPoolClearMarks;

  /// No description provided for @settingsModelPoolTest.
  ///
  /// In en, this message translates to:
  /// **'Test untested keys'**
  String get settingsModelPoolTest;

  /// No description provided for @settingsModelPoolNeedsModel.
  ///
  /// In en, this message translates to:
  /// **'Save the provider and add a model before testing keys.'**
  String get settingsModelPoolNeedsModel;

  /// No description provided for @settingsModelProviderId.
  ///
  /// In en, this message translates to:
  /// **'Provider ID'**
  String get settingsModelProviderId;

  /// No description provided for @settingsModelProvidersSection.
  ///
  /// In en, this message translates to:
  /// **'LLM providers'**
  String get settingsModelProvidersSection;

  /// No description provided for @settingsModelProviderType.
  ///
  /// In en, this message translates to:
  /// **'Provider type'**
  String get settingsModelProviderType;

  /// No description provided for @settingsModelProviderTypeOption.
  ///
  /// In en, this message translates to:
  /// **'{name} ({original})'**
  String settingsModelProviderTypeOption(String name, String original);

  /// No description provided for @settingsModelProviderTypeOpenai.
  ///
  /// In en, this message translates to:
  /// **'OpenAI'**
  String get settingsModelProviderTypeOpenai;

  /// No description provided for @settingsModelProviderTypeXai.
  ///
  /// In en, this message translates to:
  /// **'xAI'**
  String get settingsModelProviderTypeXai;

  /// No description provided for @settingsModelProviderTypeOpenaiResponses.
  ///
  /// In en, this message translates to:
  /// **'OpenAI Responses'**
  String get settingsModelProviderTypeOpenaiResponses;

  /// No description provided for @settingsModelProviderTypeOpenaiCodex.
  ///
  /// In en, this message translates to:
  /// **'OpenAI Codex'**
  String get settingsModelProviderTypeOpenaiCodex;

  /// No description provided for @settingsModelProviderTypeOpenaiResponsesGeneric.
  ///
  /// In en, this message translates to:
  /// **'OpenAI Responses compatible'**
  String get settingsModelProviderTypeOpenaiResponsesGeneric;

  /// No description provided for @settingsModelProviderTypeOpenaiGeneric.
  ///
  /// In en, this message translates to:
  /// **'OpenAI compatible'**
  String get settingsModelProviderTypeOpenaiGeneric;

  /// No description provided for @settingsModelProviderTypeAnthropic.
  ///
  /// In en, this message translates to:
  /// **'Anthropic'**
  String get settingsModelProviderTypeAnthropic;

  /// No description provided for @settingsModelProviderTypeAnthropicGeneric.
  ///
  /// In en, this message translates to:
  /// **'Anthropic compatible'**
  String get settingsModelProviderTypeAnthropicGeneric;

  /// No description provided for @settingsModelProviderTypeGoogle.
  ///
  /// In en, this message translates to:
  /// **'Google Gemini'**
  String get settingsModelProviderTypeGoogle;

  /// No description provided for @settingsModelProviderTypeGeminiGeneric.
  ///
  /// In en, this message translates to:
  /// **'Gemini compatible'**
  String get settingsModelProviderTypeGeminiGeneric;

  /// No description provided for @settingsModelProviderTypeBaidu.
  ///
  /// In en, this message translates to:
  /// **'Baidu'**
  String get settingsModelProviderTypeBaidu;

  /// No description provided for @settingsModelProviderTypeAliyun.
  ///
  /// In en, this message translates to:
  /// **'Aliyun'**
  String get settingsModelProviderTypeAliyun;

  /// No description provided for @settingsModelProviderTypeXunfei.
  ///
  /// In en, this message translates to:
  /// **'Xunfei'**
  String get settingsModelProviderTypeXunfei;

  /// No description provided for @settingsModelProviderTypeZhipu.
  ///
  /// In en, this message translates to:
  /// **'Zhipu AI'**
  String get settingsModelProviderTypeZhipu;

  /// No description provided for @settingsModelProviderTypeBaichuan.
  ///
  /// In en, this message translates to:
  /// **'Baichuan'**
  String get settingsModelProviderTypeBaichuan;

  /// No description provided for @settingsModelProviderTypeMoonshot.
  ///
  /// In en, this message translates to:
  /// **'Moonshot'**
  String get settingsModelProviderTypeMoonshot;

  /// No description provided for @settingsModelProviderTypeMimo.
  ///
  /// In en, this message translates to:
  /// **'MiMo'**
  String get settingsModelProviderTypeMimo;

  /// No description provided for @settingsModelProviderTypeDeepseek.
  ///
  /// In en, this message translates to:
  /// **'DeepSeek'**
  String get settingsModelProviderTypeDeepseek;

  /// No description provided for @settingsModelProviderTypeMistral.
  ///
  /// In en, this message translates to:
  /// **'Mistral'**
  String get settingsModelProviderTypeMistral;

  /// No description provided for @settingsModelProviderTypeSiliconflow.
  ///
  /// In en, this message translates to:
  /// **'SiliconFlow'**
  String get settingsModelProviderTypeSiliconflow;

  /// No description provided for @settingsModelProviderTypeOpenrouter.
  ///
  /// In en, this message translates to:
  /// **'OpenRouter'**
  String get settingsModelProviderTypeOpenrouter;

  /// No description provided for @settingsModelProviderTypeOpencode.
  ///
  /// In en, this message translates to:
  /// **'OpenCode'**
  String get settingsModelProviderTypeOpencode;

  /// No description provided for @settingsModelProviderTypeFourRouter.
  ///
  /// In en, this message translates to:
  /// **'4Router'**
  String get settingsModelProviderTypeFourRouter;

  /// No description provided for @settingsModelProviderTypeNousPortal.
  ///
  /// In en, this message translates to:
  /// **'Nous Portal'**
  String get settingsModelProviderTypeNousPortal;

  /// No description provided for @settingsModelProviderTypeInfiniai.
  ///
  /// In en, this message translates to:
  /// **'InfiniAI'**
  String get settingsModelProviderTypeInfiniai;

  /// No description provided for @settingsModelProviderTypeAlipayBailing.
  ///
  /// In en, this message translates to:
  /// **'Alipay Bailing'**
  String get settingsModelProviderTypeAlipayBailing;

  /// No description provided for @settingsModelProviderTypeDoubao.
  ///
  /// In en, this message translates to:
  /// **'Doubao'**
  String get settingsModelProviderTypeDoubao;

  /// No description provided for @settingsModelProviderTypeNvidia.
  ///
  /// In en, this message translates to:
  /// **'NVIDIA'**
  String get settingsModelProviderTypeNvidia;

  /// No description provided for @settingsModelProviderTypeLmstudio.
  ///
  /// In en, this message translates to:
  /// **'LM Studio'**
  String get settingsModelProviderTypeLmstudio;

  /// No description provided for @settingsModelProviderTypeOllama.
  ///
  /// In en, this message translates to:
  /// **'Ollama'**
  String get settingsModelProviderTypeOllama;

  /// No description provided for @settingsModelProviderTypeOpenaiLocal.
  ///
  /// In en, this message translates to:
  /// **'OpenAI Local'**
  String get settingsModelProviderTypeOpenaiLocal;

  /// No description provided for @settingsModelProviderTypeLocalModel.
  ///
  /// In en, this message translates to:
  /// **'Local model'**
  String get settingsModelProviderTypeLocalModel;

  /// No description provided for @settingsModelProviderTypeLocalGateway.
  ///
  /// In en, this message translates to:
  /// **'Local gateway'**
  String get settingsModelProviderTypeLocalGateway;

  /// No description provided for @settingsModelLocalGatewayHint.
  ///
  /// In en, this message translates to:
  /// **'Create this provider on the device running Magpie, CC Switch, or another gateway. Local addresses refer to that device; other devices reach it through route. Supports /v1/chat/completions, /v1/responses, and /v1/messages. Keep the gateway device online and allow remote execution.'**
  String get settingsModelLocalGatewayHint;

  /// No description provided for @localModelsLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to load local model status: {error}'**
  String localModelsLoadFailed(Object error);

  /// No description provided for @localModelsOperationFailed.
  ///
  /// In en, this message translates to:
  /// **'Local model operation failed: {error}'**
  String localModelsOperationFailed(Object error);

  /// No description provided for @localModelsAddModel.
  ///
  /// In en, this message translates to:
  /// **'Browse online'**
  String get localModelsAddModel;

  /// No description provided for @localModelsImportFromDevice.
  ///
  /// In en, this message translates to:
  /// **'Import from device'**
  String get localModelsImportFromDevice;

  /// No description provided for @localModelsImportGgufHint.
  ///
  /// In en, this message translates to:
  /// **'Choose one complete GGUF chat model for llama.cpp.'**
  String get localModelsImportGgufHint;

  /// No description provided for @localModelsImportMnnHint.
  ///
  /// In en, this message translates to:
  /// **'Select the extracted MNN model files together, including config.json and the weights.'**
  String get localModelsImportMnnHint;

  /// No description provided for @localModelsImportInvalidGguf.
  ///
  /// In en, this message translates to:
  /// **'Select a single .gguf file.'**
  String get localModelsImportInvalidGguf;

  /// No description provided for @localModelsImportMissingConfig.
  ///
  /// In en, this message translates to:
  /// **'The selected files are missing config.json.'**
  String get localModelsImportMissingConfig;

  /// No description provided for @localModelsImportFileSizeRequired.
  ///
  /// In en, this message translates to:
  /// **'Each selected file must be non-empty and report its size.'**
  String get localModelsImportFileSizeRequired;

  /// No description provided for @localModelsImportChangeFiles.
  ///
  /// In en, this message translates to:
  /// **'Choose again'**
  String get localModelsImportChangeFiles;

  /// No description provided for @localModelsImportFileCount.
  ///
  /// In en, this message translates to:
  /// **'{count} files selected'**
  String localModelsImportFileCount(int count);

  /// No description provided for @localModelsImportName.
  ///
  /// In en, this message translates to:
  /// **'Model name'**
  String get localModelsImportName;

  /// No description provided for @localModelsImportCopyNotice.
  ///
  /// In en, this message translates to:
  /// **'The files are copied into app storage and stay on this device.'**
  String get localModelsImportCopyNotice;

  /// No description provided for @localModelsImportCopying.
  ///
  /// In en, this message translates to:
  /// **'Copying model files…'**
  String get localModelsImportCopying;

  /// No description provided for @localModelsImportFinishing.
  ///
  /// In en, this message translates to:
  /// **'Checking and registering model…'**
  String get localModelsImportFinishing;

  /// No description provided for @localModelsImportComplete.
  ///
  /// In en, this message translates to:
  /// **'Model imported. Open Run and configure to try it.'**
  String get localModelsImportComplete;

  /// No description provided for @localModelsImportedFile.
  ///
  /// In en, this message translates to:
  /// **'Device import'**
  String get localModelsImportedFile;

  /// No description provided for @localModelsFilterDownloads.
  ///
  /// In en, this message translates to:
  /// **'Downloads'**
  String get localModelsFilterDownloads;

  /// No description provided for @localModelsAllTypes.
  ///
  /// In en, this message translates to:
  /// **'All types'**
  String get localModelsAllTypes;

  /// No description provided for @localModelsNoMatches.
  ///
  /// In en, this message translates to:
  /// **'No matching models'**
  String get localModelsNoMatches;

  /// No description provided for @localModelsResetFilters.
  ///
  /// In en, this message translates to:
  /// **'Reset filters'**
  String get localModelsResetFilters;

  /// No description provided for @localModelsDetails.
  ///
  /// In en, this message translates to:
  /// **'Model details'**
  String get localModelsDetails;

  /// No description provided for @localModelsMoreActions.
  ///
  /// In en, this message translates to:
  /// **'More actions'**
  String get localModelsMoreActions;

  /// No description provided for @localModelsResultCount.
  ///
  /// In en, this message translates to:
  /// **'Showing {count} of {total} models'**
  String localModelsResultCount(int count, int total);

  /// No description provided for @localModelsHubRepositoryTab.
  ///
  /// In en, this message translates to:
  /// **'Repository'**
  String get localModelsHubRepositoryTab;

  /// No description provided for @localModelsDownloadFailed.
  ///
  /// In en, this message translates to:
  /// **'Download failed'**
  String get localModelsDownloadFailed;

  /// No description provided for @localModelsDownloadPausedLabel.
  ///
  /// In en, this message translates to:
  /// **'Paused'**
  String get localModelsDownloadPausedLabel;

  /// No description provided for @localModelsCatalog.
  ///
  /// In en, this message translates to:
  /// **'Model catalog'**
  String get localModelsCatalog;

  /// No description provided for @localModelsCategorySpeechToText.
  ///
  /// In en, this message translates to:
  /// **'Speech-to-text models'**
  String get localModelsCategorySpeechToText;

  /// No description provided for @localModelsCategoryTextToSpeech.
  ///
  /// In en, this message translates to:
  /// **'Text-to-speech models'**
  String get localModelsCategoryTextToSpeech;

  /// No description provided for @localModelsCategoryChat.
  ///
  /// In en, this message translates to:
  /// **'LLM models'**
  String get localModelsCategoryChat;

  /// No description provided for @localModelsCategoryEmbedding.
  ///
  /// In en, this message translates to:
  /// **'Embedding models'**
  String get localModelsCategoryEmbedding;

  /// No description provided for @localModelsInstalledEngines.
  ///
  /// In en, this message translates to:
  /// **'Installed engines'**
  String get localModelsInstalledEngines;

  /// No description provided for @localModelsNoInstalledEngines.
  ///
  /// In en, this message translates to:
  /// **'No local inference engine is installed on this platform.'**
  String get localModelsNoInstalledEngines;

  /// No description provided for @localModelsDeleteModelTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete local model'**
  String get localModelsDeleteModelTitle;

  /// No description provided for @localModelsDeleteModelMessage.
  ///
  /// In en, this message translates to:
  /// **'Delete model files for {modelName}?'**
  String localModelsDeleteModelMessage(Object modelName);

  /// No description provided for @localModelsDeleteEngineTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete local engine'**
  String get localModelsDeleteEngineTitle;

  /// No description provided for @localModelsDeleteEngineMessage.
  ///
  /// In en, this message translates to:
  /// **'Delete {engineName} {version}?'**
  String localModelsDeleteEngineMessage(Object engineName, Object version);

  /// No description provided for @localModelsCancelling.
  ///
  /// In en, this message translates to:
  /// **'Pausing'**
  String get localModelsCancelling;

  /// No description provided for @localModelsDownloadPaused.
  ///
  /// In en, this message translates to:
  /// **'Paused: {downloaded} / {total}'**
  String localModelsDownloadPaused(Object downloaded, Object total);

  /// No description provided for @localModelsDownloadInstalling.
  ///
  /// In en, this message translates to:
  /// **'Download complete, installing'**
  String get localModelsDownloadInstalling;

  /// No description provided for @localModelsDownloading.
  ///
  /// In en, this message translates to:
  /// **'Downloading: {downloaded} / {total}'**
  String localModelsDownloading(Object downloaded, Object total);

  /// No description provided for @localModelsLicense.
  ///
  /// In en, this message translates to:
  /// **'License: {license}'**
  String localModelsLicense(Object license);

  /// No description provided for @localModelsPlatformCompatible.
  ///
  /// In en, this message translates to:
  /// **'Platform compatible'**
  String get localModelsPlatformCompatible;

  /// No description provided for @localModelsPlatformIncompatible.
  ///
  /// In en, this message translates to:
  /// **'Platform incompatible'**
  String get localModelsPlatformIncompatible;

  /// No description provided for @localModelsModelInstalled.
  ///
  /// In en, this message translates to:
  /// **'Model installed'**
  String get localModelsModelInstalled;

  /// No description provided for @localModelsModelNotInstalled.
  ///
  /// In en, this message translates to:
  /// **'Model not installed'**
  String get localModelsModelNotInstalled;

  /// No description provided for @localModelsEngineInstalled.
  ///
  /// In en, this message translates to:
  /// **'Engine installed'**
  String get localModelsEngineInstalled;

  /// No description provided for @localModelsEngineNotInstalled.
  ///
  /// In en, this message translates to:
  /// **'Engine not installed'**
  String get localModelsEngineNotInstalled;

  /// No description provided for @localModelsVerifyModelAndEngine.
  ///
  /// In en, this message translates to:
  /// **'Verify model and engine'**
  String get localModelsVerifyModelAndEngine;

  /// No description provided for @localModelsDeleteModel.
  ///
  /// In en, this message translates to:
  /// **'Delete model'**
  String get localModelsDeleteModel;

  /// No description provided for @localModelsPauseDownload.
  ///
  /// In en, this message translates to:
  /// **'Pause download'**
  String get localModelsPauseDownload;

  /// No description provided for @localModelsDeleteDownload.
  ///
  /// In en, this message translates to:
  /// **'Delete download'**
  String get localModelsDeleteDownload;

  /// No description provided for @localModelsResumeDownload.
  ///
  /// In en, this message translates to:
  /// **'Resume'**
  String get localModelsResumeDownload;

  /// No description provided for @localModelsInstalling.
  ///
  /// In en, this message translates to:
  /// **'Installing'**
  String get localModelsInstalling;

  /// No description provided for @localModelsInstall.
  ///
  /// In en, this message translates to:
  /// **'Install'**
  String get localModelsInstall;

  /// No description provided for @localModelsDeleteEngine.
  ///
  /// In en, this message translates to:
  /// **'Delete engine'**
  String get localModelsDeleteEngine;

  /// No description provided for @localModelsDownloadSource.
  ///
  /// In en, this message translates to:
  /// **'Download source'**
  String get localModelsDownloadSource;

  /// No description provided for @localModelsSourceHuggingFace.
  ///
  /// In en, this message translates to:
  /// **'Hugging Face'**
  String get localModelsSourceHuggingFace;

  /// No description provided for @localModelsSourceModelScope.
  ///
  /// In en, this message translates to:
  /// **'ModelScope'**
  String get localModelsSourceModelScope;

  /// No description provided for @localModelsSourceHfMirror.
  ///
  /// In en, this message translates to:
  /// **'HF-Mirror'**
  String get localModelsSourceHfMirror;

  /// No description provided for @localModelsSourceDirectHttp.
  ///
  /// In en, this message translates to:
  /// **'Direct HTTP'**
  String get localModelsSourceDirectHttp;

  /// No description provided for @localModelsExploreHub.
  ///
  /// In en, this message translates to:
  /// **'Explore / Import Models'**
  String get localModelsExploreHub;

  /// No description provided for @localModelsHubSearchHint.
  ///
  /// In en, this message translates to:
  /// **'Search models or enter owner/repo...'**
  String get localModelsHubSearchHint;

  /// No description provided for @localModelsHubImport.
  ///
  /// In en, this message translates to:
  /// **'Import'**
  String get localModelsHubImport;

  /// No description provided for @localModelsHubImporting.
  ///
  /// In en, this message translates to:
  /// **'Analyzing and importing model...'**
  String get localModelsHubImporting;

  /// No description provided for @localModelsHubImportSuccess.
  ///
  /// In en, this message translates to:
  /// **'Model successfully imported into catalog'**
  String get localModelsHubImportSuccess;

  /// No description provided for @localModelsRemoveFromCatalog.
  ///
  /// In en, this message translates to:
  /// **'Remove from catalog'**
  String get localModelsRemoveFromCatalog;

  /// No description provided for @localModelsCustomBadge.
  ///
  /// In en, this message translates to:
  /// **'Custom'**
  String get localModelsCustomBadge;

  /// No description provided for @localModelsHubSearching.
  ///
  /// In en, this message translates to:
  /// **'Searching...'**
  String get localModelsHubSearching;

  /// No description provided for @localModelsHubNoResults.
  ///
  /// In en, this message translates to:
  /// **'No matching model repositories found'**
  String get localModelsHubNoResults;

  /// No description provided for @localModelsHubDirectImportTitle.
  ///
  /// In en, this message translates to:
  /// **'Import by Repository'**
  String get localModelsHubDirectImportTitle;

  /// No description provided for @localModelsHubDirectImportPrompt.
  ///
  /// In en, this message translates to:
  /// **'Enter a Hugging Face or ModelScope repository (owner/repo) or URL:'**
  String get localModelsHubDirectImportPrompt;

  /// No description provided for @localModelDescriptionSherpaOnnxStreamingStt.
  ///
  /// In en, this message translates to:
  /// **'Streaming bilingual Chinese and English speech recognition.'**
  String get localModelDescriptionSherpaOnnxStreamingStt;

  /// No description provided for @localModelDescriptionSherpaOnnxVitsAishell3.
  ///
  /// In en, this message translates to:
  /// **'Local Chinese multi-speaker speech synthesis.'**
  String get localModelDescriptionSherpaOnnxVitsAishell3;

  /// No description provided for @localModelDescriptionSherpaOnnxVitsZhLl.
  ///
  /// In en, this message translates to:
  /// **'Local Chinese five-speaker speech synthesis.'**
  String get localModelDescriptionSherpaOnnxVitsZhLl;

  /// No description provided for @localModelDescriptionSherpaOnnxMatchaBaker.
  ///
  /// In en, this message translates to:
  /// **'Local Chinese single-speaker Matcha speech synthesis.'**
  String get localModelDescriptionSherpaOnnxMatchaBaker;

  /// No description provided for @localModelDescriptionSherpaOnnxKittenNano.
  ///
  /// In en, this message translates to:
  /// **'Local English eight-speaker KittenTTS speech synthesis.'**
  String get localModelDescriptionSherpaOnnxKittenNano;

  /// No description provided for @localModelDescriptionSherpaOnnxWebParaformer.
  ///
  /// In en, this message translates to:
  /// **'Browser packaged Chinese and English Paraformer speech recognition.'**
  String get localModelDescriptionSherpaOnnxWebParaformer;

  /// No description provided for @localModelDescriptionSherpaOnnxWebVitsPiper.
  ///
  /// In en, this message translates to:
  /// **'Browser packaged English multi-speaker VITS speech synthesis.'**
  String get localModelDescriptionSherpaOnnxWebVitsPiper;

  /// No description provided for @settingsModelProviderTypeMnn.
  ///
  /// In en, this message translates to:
  /// **'MNN'**
  String get settingsModelProviderTypeMnn;

  /// No description provided for @settingsModelProviderTypeLlamaCpp.
  ///
  /// In en, this message translates to:
  /// **'llama.cpp'**
  String get settingsModelProviderTypeLlamaCpp;

  /// No description provided for @settingsModelProviderTypePpinfra.
  ///
  /// In en, this message translates to:
  /// **'PPInfra'**
  String get settingsModelProviderTypePpinfra;

  /// No description provided for @settingsModelProviderTypeNovita.
  ///
  /// In en, this message translates to:
  /// **'Novita AI'**
  String get settingsModelProviderTypeNovita;

  /// No description provided for @settingsModelProviderTypeMinimax.
  ///
  /// In en, this message translates to:
  /// **'MiniMax'**
  String get settingsModelProviderTypeMinimax;

  /// No description provided for @settingsModelProviderTypeOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get settingsModelProviderTypeOther;

  /// No description provided for @settingsModelEditModelSettings.
  ///
  /// In en, this message translates to:
  /// **'Model settings'**
  String get settingsModelEditModelSettings;

  /// No description provided for @settingsModelCreateProvider.
  ///
  /// In en, this message translates to:
  /// **'Create provider'**
  String get settingsModelCreateProvider;

  /// No description provided for @settingsModelEditProvider.
  ///
  /// In en, this message translates to:
  /// **'Edit provider'**
  String get settingsModelEditProvider;

  /// No description provided for @settingsModelAddModel.
  ///
  /// In en, this message translates to:
  /// **'Add model'**
  String get settingsModelAddModel;

  /// No description provided for @settingsModelAddModelShort.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get settingsModelAddModelShort;

  /// No description provided for @settingsModelAvailableFetchedOnly.
  ///
  /// In en, this message translates to:
  /// **'Fetched only'**
  String get settingsModelAvailableFetchedOnly;

  /// No description provided for @settingsModelAvailableIncludeHistory.
  ///
  /// In en, this message translates to:
  /// **'Include history'**
  String get settingsModelAvailableIncludeHistory;

  /// No description provided for @settingsModelAvailableSourceFetched.
  ///
  /// In en, this message translates to:
  /// **'Fetched'**
  String get settingsModelAvailableSourceFetched;

  /// No description provided for @settingsModelAvailableSourceHistory.
  ///
  /// In en, this message translates to:
  /// **'History'**
  String get settingsModelAvailableSourceHistory;

  /// No description provided for @settingsModelAvailableEmptyFetched.
  ///
  /// In en, this message translates to:
  /// **'No fetched models. Choose Include history to see catalog IDs.'**
  String get settingsModelAvailableEmptyFetched;

  /// No description provided for @settingsModelUnrecognized.
  ///
  /// In en, this message translates to:
  /// **'Unrecognized'**
  String get settingsModelUnrecognized;

  /// No description provided for @settingsModelCatalogUpdatedAt.
  ///
  /// In en, this message translates to:
  /// **'Capabilities and context from models.dev, updated {time}'**
  String settingsModelCatalogUpdatedAt(String time);

  /// No description provided for @settingsModelCatalogUpdateFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not download model details: {error}'**
  String settingsModelCatalogUpdateFailed(String error);

  /// No description provided for @settingsModelCatalogUpdateFailedUsing.
  ///
  /// In en, this message translates to:
  /// **'Could not update model details, using those from {time}: {error}'**
  String settingsModelCatalogUpdateFailedUsing(String time, String error);

  /// No description provided for @settingsModelCatalogBundled.
  ///
  /// In en, this message translates to:
  /// **'Using bundled model details; online updates replace them automatically'**
  String get settingsModelCatalogBundled;

  /// No description provided for @settingsModelCatalogRetry.
  ///
  /// In en, this message translates to:
  /// **'Update'**
  String get settingsModelCatalogRetry;

  /// No description provided for @settingsModelCustomModel.
  ///
  /// In en, this message translates to:
  /// **'Custom model'**
  String get settingsModelCustomModel;

  /// No description provided for @settingsModelModelId.
  ///
  /// In en, this message translates to:
  /// **'Model ID'**
  String get settingsModelModelId;

  /// No description provided for @settingsModelDuplicateModelId.
  ///
  /// In en, this message translates to:
  /// **'This model has already been added to this provider.'**
  String get settingsModelDuplicateModelId;

  /// No description provided for @settingsModelDuplicateProviderName.
  ///
  /// In en, this message translates to:
  /// **'A provider with this name already exists.'**
  String get settingsModelDuplicateProviderName;

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

  /// No description provided for @settingsModelMaxContextLengthInvalid.
  ///
  /// In en, this message translates to:
  /// **'Enter a max context length greater than 0'**
  String get settingsModelMaxContextLengthInvalid;

  /// No description provided for @settingsModelSummaryThreshold.
  ///
  /// In en, this message translates to:
  /// **'Summarize at'**
  String get settingsModelSummaryThreshold;

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
  /// **'Context'**
  String get settingsModelContext;

  /// No description provided for @settingsModelSummary.
  ///
  /// In en, this message translates to:
  /// **'Auto summary'**
  String get settingsModelSummary;

  /// No description provided for @settingsModelThinking.
  ///
  /// In en, this message translates to:
  /// **'Thinking'**
  String get settingsModelThinking;

  /// No description provided for @thinkingLevelLabel.
  ///
  /// In en, this message translates to:
  /// **'Thinking'**
  String get thinkingLevelLabel;

  /// No description provided for @thinkingAlwaysOn.
  ///
  /// In en, this message translates to:
  /// **'Always thinking'**
  String get thinkingAlwaysOn;

  /// No description provided for @thinkingChip.
  ///
  /// In en, this message translates to:
  /// **'Thinking'**
  String get thinkingChip;

  /// No description provided for @settingsModelThinkingSourceDeclared.
  ///
  /// In en, this message translates to:
  /// **'Levels from your description'**
  String get settingsModelThinkingSourceDeclared;

  /// No description provided for @settingsModelThinkingSourceLocalRuntime.
  ///
  /// In en, this message translates to:
  /// **'Runs locally: thinking switches on or off, and the switch does nothing for a model that cannot think'**
  String get settingsModelThinkingSourceLocalRuntime;

  /// No description provided for @settingsModelThinkingUnknown.
  ///
  /// In en, this message translates to:
  /// **'The catalog has no thinking details for this model'**
  String get settingsModelThinkingUnknown;

  /// No description provided for @settingsModelThinkingNotAdjustable.
  ///
  /// In en, this message translates to:
  /// **'This API cannot change how this model thinks'**
  String get settingsModelThinkingNotAdjustable;

  /// No description provided for @settingsModelThinkingNotSupported.
  ///
  /// In en, this message translates to:
  /// **'This model does not think'**
  String get settingsModelThinkingNotSupported;

  /// No description provided for @settingsModelThinkingCarried.
  ///
  /// In en, this message translates to:
  /// **'Carried over from the level you set last'**
  String get settingsModelThinkingCarried;

  /// No description provided for @settingsModelThinkingDescribe.
  ///
  /// In en, this message translates to:
  /// **'Describe'**
  String get settingsModelThinkingDescribe;

  /// No description provided for @settingsModelThinkingUseCatalog.
  ///
  /// In en, this message translates to:
  /// **'Use catalog'**
  String get settingsModelThinkingUseCatalog;

  /// No description provided for @settingsModelThinkingDescribeTitle.
  ///
  /// In en, this message translates to:
  /// **'How this model thinks'**
  String get settingsModelThinkingDescribeTitle;

  /// No description provided for @settingsModelThinkingKindNone.
  ///
  /// In en, this message translates to:
  /// **'Does not think'**
  String get settingsModelThinkingKindNone;

  /// No description provided for @settingsModelThinkingKindAlways.
  ///
  /// In en, this message translates to:
  /// **'Always thinks, no control'**
  String get settingsModelThinkingKindAlways;

  /// No description provided for @settingsModelThinkingKindSwitch.
  ///
  /// In en, this message translates to:
  /// **'On or off'**
  String get settingsModelThinkingKindSwitch;

  /// No description provided for @settingsModelThinkingKindLevels.
  ///
  /// In en, this message translates to:
  /// **'Levels'**
  String get settingsModelThinkingKindLevels;

  /// No description provided for @settingsModelThinkingKindBudget.
  ///
  /// In en, this message translates to:
  /// **'Token budget'**
  String get settingsModelThinkingKindBudget;

  /// No description provided for @settingsModelThinkingCanTurnOff.
  ///
  /// In en, this message translates to:
  /// **'Can be turned off'**
  String get settingsModelThinkingCanTurnOff;

  /// No description provided for @settingsModelThinkingBudgetMin.
  ///
  /// In en, this message translates to:
  /// **'Minimum tokens'**
  String get settingsModelThinkingBudgetMin;

  /// No description provided for @settingsModelThinkingBudgetMax.
  ///
  /// In en, this message translates to:
  /// **'Maximum tokens'**
  String get settingsModelThinkingBudgetMax;

  /// No description provided for @settingsModelThinkingLevelsRequired.
  ///
  /// In en, this message translates to:
  /// **'Pick at least one level'**
  String get settingsModelThinkingLevelsRequired;

  /// No description provided for @settingsModelMediaHistory.
  ///
  /// In en, this message translates to:
  /// **'Media history'**
  String get settingsModelMediaHistory;

  /// No description provided for @settingsModelCapabilities.
  ///
  /// In en, this message translates to:
  /// **'Capabilities'**
  String get settingsModelCapabilities;

  /// No description provided for @settingsModelBuiltinTools.
  ///
  /// In en, this message translates to:
  /// **'Built-in tools'**
  String get settingsModelBuiltinTools;

  /// No description provided for @settingsModelBuiltinToolExclusive.
  ///
  /// In en, this message translates to:
  /// **'Turns off external tool calling when enabled'**
  String get settingsModelBuiltinToolExclusive;

  /// No description provided for @settingsModelRunConnectionTest.
  ///
  /// In en, this message translates to:
  /// **'Test current model'**
  String get settingsModelRunConnectionTest;

  /// No description provided for @settingsModelTestModel.
  ///
  /// In en, this message translates to:
  /// **'Test'**
  String get settingsModelTestModel;

  /// No description provided for @settingsModelCapabilityToolCall.
  ///
  /// In en, this message translates to:
  /// **'Tools'**
  String get settingsModelCapabilityToolCall;

  /// No description provided for @settingsModelCapabilityImage.
  ///
  /// In en, this message translates to:
  /// **'Image'**
  String get settingsModelCapabilityImage;

  /// No description provided for @settingsModelCapabilityAudio.
  ///
  /// In en, this message translates to:
  /// **'Audio'**
  String get settingsModelCapabilityAudio;

  /// No description provided for @settingsModelCapabilityVideo.
  ///
  /// In en, this message translates to:
  /// **'Video'**
  String get settingsModelCapabilityVideo;

  /// No description provided for @settingsModelClaudeCache.
  ///
  /// In en, this message translates to:
  /// **'Claude 1h cache'**
  String get settingsModelClaudeCache;

  /// No description provided for @settingsModelClaudeCacheDescription.
  ///
  /// In en, this message translates to:
  /// **'Sets a 1-hour TTL on explicit cache breakpoints; when off, the default TTL applies.'**
  String get settingsModelClaudeCacheDescription;

  /// No description provided for @settingsModelSummaryThresholdDescription.
  ///
  /// In en, this message translates to:
  /// **'Summarize when the context fills to this share. 0 turns summaries off.'**
  String get settingsModelSummaryThresholdDescription;

  /// No description provided for @settingsModelMoreActions.
  ///
  /// In en, this message translates to:
  /// **'More actions'**
  String get settingsModelMoreActions;

  /// No description provided for @settingsModelRestoreDefaults.
  ///
  /// In en, this message translates to:
  /// **'Restore defaults'**
  String get settingsModelRestoreDefaults;

  /// No description provided for @settingsModelDescribeThinking.
  ///
  /// In en, this message translates to:
  /// **'Describe thinking levels'**
  String get settingsModelDescribeThinking;

  /// No description provided for @settingsModelDeleteModel.
  ///
  /// In en, this message translates to:
  /// **'Delete model'**
  String get settingsModelDeleteModel;

  /// No description provided for @settingsModelSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not save: {error}'**
  String settingsModelSaveFailed(String error);

  /// No description provided for @settingsModelTestAll.
  ///
  /// In en, this message translates to:
  /// **'Test all'**
  String get settingsModelTestAll;

  /// No description provided for @settingsModelTestProgress.
  ///
  /// In en, this message translates to:
  /// **'Testing {done}/{total}'**
  String settingsModelTestProgress(int done, int total);

  /// No description provided for @settingsModelTestSummary.
  ///
  /// In en, this message translates to:
  /// **'{passed} passed, {failed} failed'**
  String settingsModelTestSummary(int passed, int failed);

  /// No description provided for @settingsModelTestSummaryWithRestricted.
  ///
  /// In en, this message translates to:
  /// **'{passed} passed, {failed} failed, {restricted} restricted'**
  String settingsModelTestSummaryWithRestricted(
    int passed,
    int failed,
    int restricted,
  );

  /// No description provided for @settingsModelTestAllPassed.
  ///
  /// In en, this message translates to:
  /// **'All {count} passed'**
  String settingsModelTestAllPassed(int count);

  /// No description provided for @settingsModelTestCouldNotRun.
  ///
  /// In en, this message translates to:
  /// **'The test could not run'**
  String get settingsModelTestCouldNotRun;

  /// No description provided for @settingsModelCopyError.
  ///
  /// In en, this message translates to:
  /// **'Copy error'**
  String get settingsModelCopyError;

  /// No description provided for @settingsModelSelect.
  ///
  /// In en, this message translates to:
  /// **'Select'**
  String get settingsModelSelect;

  /// No description provided for @settingsModelCheckAll.
  ///
  /// In en, this message translates to:
  /// **'Select all'**
  String get settingsModelCheckAll;

  /// No description provided for @settingsModelCheckedCount.
  ///
  /// In en, this message translates to:
  /// **'{count} selected'**
  String settingsModelCheckedCount(int count);

  /// No description provided for @settingsModelDeleteModelsTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete {count} models?'**
  String settingsModelDeleteModelsTitle(int count);

  /// No description provided for @settingsModelDeleteModelsMessage.
  ///
  /// In en, this message translates to:
  /// **'Their settings are deleted with them.'**
  String get settingsModelDeleteModelsMessage;

  /// No description provided for @settingsModelDeleteModelsKept.
  ///
  /// In en, this message translates to:
  /// **'{count} in use by functions will be kept: {names}'**
  String settingsModelDeleteModelsKept(int count, String names);

  /// No description provided for @settingsModelDeleteModelsBlocked.
  ///
  /// In en, this message translates to:
  /// **'The selected models are used by these functions. Change the function model assignments first: {functions}'**
  String settingsModelDeleteModelsBlocked(String functions);

  /// No description provided for @settingsModelDeletedCount.
  ///
  /// In en, this message translates to:
  /// **'Deleted {count} models'**
  String settingsModelDeletedCount(int count);

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

  /// No description provided for @settingsModelConnectionTestRestricted.
  ///
  /// In en, this message translates to:
  /// **'Test restricted'**
  String get settingsModelConnectionTestRestricted;

  /// No description provided for @settingsModelTestCreditRestricted.
  ///
  /// In en, this message translates to:
  /// **'Credit or output budget limit'**
  String get settingsModelTestCreditRestricted;

  /// No description provided for @settingsModelTestRateRestricted.
  ///
  /// In en, this message translates to:
  /// **'Upstream rate limit or cooldown'**
  String get settingsModelTestRateRestricted;

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

  /// No description provided for @settingsToolsPermissionMode.
  ///
  /// In en, this message translates to:
  /// **'AI capability mode'**
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
  /// **'Tool records'**
  String get settingsToolsOverrides;

  /// No description provided for @settingsToolsToolGroups.
  ///
  /// In en, this message translates to:
  /// **'Registered tools'**
  String get settingsToolsToolGroups;

  /// No description provided for @settingsToolsToolGroupsDescription.
  ///
  /// In en, this message translates to:
  /// **'Tools registered by the current runtime for AI use.'**
  String get settingsToolsToolGroupsDescription;

  /// No description provided for @settingsToolsAlwaysAllow.
  ///
  /// In en, this message translates to:
  /// **'Allowed in this session'**
  String get settingsToolsAlwaysAllow;

  /// No description provided for @settingsToolsAlwaysAllowDescription.
  ///
  /// In en, this message translates to:
  /// **'These tools were approved for the current session.'**
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

  /// No description provided for @settingsToolsToolPkgPreHookTimeout.
  ///
  /// In en, this message translates to:
  /// **'ToolPkg pre-hook timeout'**
  String get settingsToolsToolPkgPreHookTimeout;

  /// No description provided for @settingsToolsToolPkgPreHookDescription.
  ///
  /// In en, this message translates to:
  /// **'One ToolPkg pre-hook chain has {seconds} seconds in total.'**
  String settingsToolsToolPkgPreHookDescription(int seconds);

  /// No description provided for @settingsToolsToolPkgPreHookTimeoutSeconds.
  ///
  /// In en, this message translates to:
  /// **'Total seconds'**
  String get settingsToolsToolPkgPreHookTimeoutSeconds;

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

  /// No description provided for @settingsWorkspaceBoundOverview.
  ///
  /// In en, this message translates to:
  /// **'Workspace binding overview'**
  String get settingsWorkspaceBoundOverview;

  /// No description provided for @settingsWorkspaceBoundOverviewDescription.
  ///
  /// In en, this message translates to:
  /// **'Workspace paths recorded by chat histories are used as the binding source.'**
  String get settingsWorkspaceBoundOverviewDescription;

  /// No description provided for @settingsWorkspaceBoundChats.
  ///
  /// In en, this message translates to:
  /// **'Bound chats'**
  String get settingsWorkspaceBoundChats;

  /// No description provided for @settingsWorkspaceInternalRoot.
  ///
  /// In en, this message translates to:
  /// **'Internal workspace root'**
  String get settingsWorkspaceInternalRoot;

  /// No description provided for @settingsWorkspaceExternalRoot.
  ///
  /// In en, this message translates to:
  /// **'Legacy external workspace root'**
  String get settingsWorkspaceExternalRoot;

  /// No description provided for @settingsWorkspaceUnboundTitle.
  ///
  /// In en, this message translates to:
  /// **'Unbound workspaces'**
  String get settingsWorkspaceUnboundTitle;

  /// No description provided for @settingsWorkspaceUnboundSubtitle.
  ///
  /// In en, this message translates to:
  /// **'These workspace folders are not used by any chat.'**
  String get settingsWorkspaceUnboundSubtitle;

  /// No description provided for @settingsWorkspaceNoUnbound.
  ///
  /// In en, this message translates to:
  /// **'No unbound workspaces.'**
  String get settingsWorkspaceNoUnbound;

  /// No description provided for @settingsWorkspaceSelectedCount.
  ///
  /// In en, this message translates to:
  /// **'Selected {selected} / {total}'**
  String settingsWorkspaceSelectedCount(int selected, int total);

  /// No description provided for @settingsWorkspaceSelectAllCurrentList.
  ///
  /// In en, this message translates to:
  /// **'Select all'**
  String get settingsWorkspaceSelectAllCurrentList;

  /// No description provided for @settingsWorkspaceClearAll.
  ///
  /// In en, this message translates to:
  /// **'Clear'**
  String get settingsWorkspaceClearAll;

  /// No description provided for @settingsWorkspaceInternalStorage.
  ///
  /// In en, this message translates to:
  /// **'Internal storage'**
  String get settingsWorkspaceInternalStorage;

  /// No description provided for @settingsWorkspaceExternalStorage.
  ///
  /// In en, this message translates to:
  /// **'External storage'**
  String get settingsWorkspaceExternalStorage;

  /// No description provided for @settingsWorkspaceNotUsedByAnyChat.
  ///
  /// In en, this message translates to:
  /// **'Not used by any chat'**
  String get settingsWorkspaceNotUsedByAnyChat;

  /// No description provided for @settingsWorkspaceDeleteSelected.
  ///
  /// In en, this message translates to:
  /// **'Delete selected workspaces ({count})'**
  String settingsWorkspaceDeleteSelected(int count);

  /// No description provided for @settingsWorkspaceConfirmDeleteTitle.
  ///
  /// In en, this message translates to:
  /// **'Confirm delete'**
  String get settingsWorkspaceConfirmDeleteTitle;

  /// No description provided for @settingsWorkspaceDeleteConfirmation.
  ///
  /// In en, this message translates to:
  /// **'Delete {count} selected workspace folders?'**
  String settingsWorkspaceDeleteConfirmation(int count);

  /// No description provided for @settingsWorkspaceDeleted.
  ///
  /// In en, this message translates to:
  /// **'Deleted {count} unbound workspaces.'**
  String settingsWorkspaceDeleted(int count);

  /// No description provided for @settingsWorkspaceDeleteFailed.
  ///
  /// In en, this message translates to:
  /// **'Delete failed: {error}'**
  String settingsWorkspaceDeleteFailed(String error);

  /// No description provided for @settingsWorkspaceLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to load workspaces: {error}'**
  String settingsWorkspaceLoadFailed(String error);

  /// No description provided for @settingsWorkspaceRefresh.
  ///
  /// In en, this message translates to:
  /// **'Refresh'**
  String get settingsWorkspaceRefresh;

  /// No description provided for @runtimeIdentity.
  ///
  /// In en, this message translates to:
  /// **'Current identity'**
  String get runtimeIdentity;

  /// No description provided for @runtimeIdentityManage.
  ///
  /// In en, this message translates to:
  /// **'Switch or manage identities'**
  String get runtimeIdentityManage;

  /// No description provided for @runtimeIdentitySheetTitle.
  ///
  /// In en, this message translates to:
  /// **'Identities'**
  String get runtimeIdentitySheetTitle;

  /// No description provided for @runtimeIdentityCreate.
  ///
  /// In en, this message translates to:
  /// **'New identity'**
  String get runtimeIdentityCreate;

  /// No description provided for @runtimeIdentityCreateTitle.
  ///
  /// In en, this message translates to:
  /// **'New identity'**
  String get runtimeIdentityCreateTitle;

  /// No description provided for @runtimeIdentityRename.
  ///
  /// In en, this message translates to:
  /// **'Rename identity'**
  String get runtimeIdentityRename;

  /// No description provided for @runtimeIdentityRenameTitle.
  ///
  /// In en, this message translates to:
  /// **'Rename identity'**
  String get runtimeIdentityRenameTitle;

  /// No description provided for @runtimeIdentityName.
  ///
  /// In en, this message translates to:
  /// **'Name (optional)'**
  String get runtimeIdentityName;

  /// No description provided for @runtimeIdentitySwitchTitle.
  ///
  /// In en, this message translates to:
  /// **'Switch to {identityName}?'**
  String runtimeIdentitySwitchTitle(String identityName);

  /// No description provided for @runtimeIdentitySwitchDescription.
  ///
  /// In en, this message translates to:
  /// **'Each identity has separate chats, settings, device space, paired devices, and workspaces. Switching ends the current runtime; the selected identity is used the next time the app starts.'**
  String get runtimeIdentitySwitchDescription;

  /// No description provided for @runtimeIdentitySwitchConfirm.
  ///
  /// In en, this message translates to:
  /// **'Switch identity'**
  String get runtimeIdentitySwitchConfirm;

  /// No description provided for @runtimeIdentityActivate.
  ///
  /// In en, this message translates to:
  /// **'Switch'**
  String get runtimeIdentityActivate;

  /// No description provided for @runtimeIdentityActions.
  ///
  /// In en, this message translates to:
  /// **'Identity actions'**
  String get runtimeIdentityActions;

  /// No description provided for @runtimeIdentityDelete.
  ///
  /// In en, this message translates to:
  /// **'Delete identity'**
  String get runtimeIdentityDelete;

  /// No description provided for @runtimeIdentityDeleteTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete {identityName}?'**
  String runtimeIdentityDeleteTitle(String identityName);

  /// No description provided for @runtimeIdentityDeleteDescription.
  ///
  /// In en, this message translates to:
  /// **'This identity will be removed from the switcher on this device. Its local files will be kept.'**
  String get runtimeIdentityDeleteDescription;

  /// No description provided for @runtimeIdentityCurrent.
  ///
  /// In en, this message translates to:
  /// **'Current'**
  String get runtimeIdentityCurrent;

  /// No description provided for @settingsUserProfileTitle.
  ///
  /// In en, this message translates to:
  /// **'Identity management'**
  String get settingsUserProfileTitle;

  /// No description provided for @settingsUserProfileSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Avatar, name, identities, and GitHub'**
  String get settingsUserProfileSubtitle;

  /// No description provided for @settingsUserProfileDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage this profile and switch isolated identities.'**
  String get settingsUserProfileDescription;

  /// No description provided for @settingsUserProfileUnnamed.
  ///
  /// In en, this message translates to:
  /// **'Unnamed'**
  String get settingsUserProfileUnnamed;

  /// No description provided for @settingsUserProfileNotLoggedIn.
  ///
  /// In en, this message translates to:
  /// **'Not logged in'**
  String get settingsUserProfileNotLoggedIn;

  /// No description provided for @settingsUserProfileGitHubLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading GitHub account...'**
  String get settingsUserProfileGitHubLoading;

  /// No description provided for @settingsUserProfileGitHubAccount.
  ///
  /// In en, this message translates to:
  /// **'GitHub: @{account}'**
  String settingsUserProfileGitHubAccount(String account);

  /// No description provided for @settingsUserProfileGitHubStatusError.
  ///
  /// In en, this message translates to:
  /// **'GitHub status error: {error}'**
  String settingsUserProfileGitHubStatusError(String error);

  /// No description provided for @settingsUserProfileOverview.
  ///
  /// In en, this message translates to:
  /// **'Profile'**
  String get settingsUserProfileOverview;

  /// No description provided for @settingsUserProfileAvatar.
  ///
  /// In en, this message translates to:
  /// **'Avatar'**
  String get settingsUserProfileAvatar;

  /// No description provided for @settingsUserProfileName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get settingsUserProfileName;

  /// No description provided for @settingsUserProfileChooseAvatar.
  ///
  /// In en, this message translates to:
  /// **'Choose avatar'**
  String get settingsUserProfileChooseAvatar;

  /// No description provided for @settingsUserProfileClearAvatar.
  ///
  /// In en, this message translates to:
  /// **'Clear avatar'**
  String get settingsUserProfileClearAvatar;

  /// No description provided for @settingsUserProfileEditName.
  ///
  /// In en, this message translates to:
  /// **'Edit name'**
  String get settingsUserProfileEditName;

  /// No description provided for @settingsUserProfileIdentities.
  ///
  /// In en, this message translates to:
  /// **'Local identities'**
  String get settingsUserProfileIdentities;

  /// No description provided for @settingsUserProfileGitHub.
  ///
  /// In en, this message translates to:
  /// **'GitHub account'**
  String get settingsUserProfileGitHub;

  /// No description provided for @settingsUserProfileGitHubDescription.
  ///
  /// In en, this message translates to:
  /// **'Not logged in'**
  String get settingsUserProfileGitHubDescription;

  /// No description provided for @settingsUserProfileLogin.
  ///
  /// In en, this message translates to:
  /// **'Log in'**
  String get settingsUserProfileLogin;

  /// No description provided for @settingsUserProfileLogout.
  ///
  /// In en, this message translates to:
  /// **'Log out'**
  String get settingsUserProfileLogout;

  /// No description provided for @settingsAppearanceAvatarCustom.
  ///
  /// In en, this message translates to:
  /// **'Custom avatar'**
  String get settingsAppearanceAvatarCustom;

  /// No description provided for @settingsRuntimeConnection.
  ///
  /// In en, this message translates to:
  /// **'Current device space'**
  String get settingsRuntimeConnection;

  /// No description provided for @settingsRuntimeConnectionDescription.
  ///
  /// In en, this message translates to:
  /// **'Connection status for this device and its device space.'**
  String get settingsRuntimeConnectionDescription;

  /// No description provided for @settingsRuntimeCurrentSpace.
  ///
  /// In en, this message translates to:
  /// **'Current device space'**
  String get settingsRuntimeCurrentSpace;

  /// No description provided for @settingsRuntimeOverviewDescription.
  ///
  /// In en, this message translates to:
  /// **'Devices in the same space can connect and share data and routes.'**
  String get settingsRuntimeOverviewDescription;

  /// No description provided for @settingsRuntimeRenameSpace.
  ///
  /// In en, this message translates to:
  /// **'Rename device space'**
  String get settingsRuntimeRenameSpace;

  /// No description provided for @settingsRuntimeResetTransactions.
  ///
  /// In en, this message translates to:
  /// **'Reset transactions'**
  String get settingsRuntimeResetTransactions;

  /// No description provided for @settingsRuntimeResetTransactionsDone.
  ///
  /// In en, this message translates to:
  /// **'Sync transactions rebuilt from this device’s current content'**
  String get settingsRuntimeResetTransactionsDone;

  /// No description provided for @settingsRuntimeLeaveSpace.
  ///
  /// In en, this message translates to:
  /// **'Leave device space'**
  String get settingsRuntimeLeaveSpace;

  /// No description provided for @settingsRuntimeLeaveSpaceTitle.
  ///
  /// In en, this message translates to:
  /// **'Leave the current device space?'**
  String get settingsRuntimeLeaveSpaceTitle;

  /// No description provided for @settingsRuntimeLeaveSpaceDescription.
  ///
  /// In en, this message translates to:
  /// **'This device will create a new single-device space. Business data and paired devices are preserved.'**
  String get settingsRuntimeLeaveSpaceDescription;

  /// No description provided for @settingsRuntimeLeaveSpaceConfirm.
  ///
  /// In en, this message translates to:
  /// **'Leave'**
  String get settingsRuntimeLeaveSpaceConfirm;

  /// No description provided for @settingsRuntimeSpaceName.
  ///
  /// In en, this message translates to:
  /// **'Device space name'**
  String get settingsRuntimeSpaceName;

  /// No description provided for @settingsRuntimeSpaceId.
  ///
  /// In en, this message translates to:
  /// **'Device space ID: {spaceId}'**
  String settingsRuntimeSpaceId(String spaceId);

  /// No description provided for @settingsRuntimeSpaceDeviceCount.
  ///
  /// In en, this message translates to:
  /// **'{count} devices'**
  String settingsRuntimeSpaceDeviceCount(int count);

  /// No description provided for @settingsRuntimeViewSpaceTopology.
  ///
  /// In en, this message translates to:
  /// **'View device topology'**
  String get settingsRuntimeViewSpaceTopology;

  /// No description provided for @settingsRuntimeSpaceTopologyTitle.
  ///
  /// In en, this message translates to:
  /// **'{spaceName} device topology'**
  String settingsRuntimeSpaceTopologyTitle(String spaceName);

  /// No description provided for @settingsRuntimeSpaceTopologySummary.
  ///
  /// In en, this message translates to:
  /// **'{deviceCount} devices · {connectionCount} direct connections'**
  String settingsRuntimeSpaceTopologySummary(
    int deviceCount,
    int connectionCount,
  );

  /// No description provided for @settingsRuntimeDisconnectConnection.
  ///
  /// In en, this message translates to:
  /// **'Disconnect'**
  String get settingsRuntimeDisconnectConnection;

  /// No description provided for @settingsRuntimeDisconnectConnectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Disconnect direct connection'**
  String get settingsRuntimeDisconnectConnectionTitle;

  /// No description provided for @settingsRuntimeDisconnectConnectionMessage.
  ///
  /// In en, this message translates to:
  /// **'Disconnect the direct connection to {deviceName}? Pairing records will be kept.'**
  String settingsRuntimeDisconnectConnectionMessage(String deviceName);

  /// No description provided for @settingsRuntimeDisconnectConnectionFailed.
  ///
  /// In en, this message translates to:
  /// **'Disconnect failed'**
  String get settingsRuntimeDisconnectConnectionFailed;

  /// No description provided for @settingsRuntimeCurrentDevice.
  ///
  /// In en, this message translates to:
  /// **'Current device'**
  String get settingsRuntimeCurrentDevice;

  /// No description provided for @settingsRuntimeRemoteTitle.
  ///
  /// In en, this message translates to:
  /// **'Connected devices'**
  String get settingsRuntimeRemoteTitle;

  /// No description provided for @settingsRuntimeRemoteDescription.
  ///
  /// In en, this message translates to:
  /// **'Directly paired devices. Pairing establishes a connection; joining a device space enables shared data and routing.'**
  String get settingsRuntimeRemoteDescription;

  /// No description provided for @settingsRuntimePairRemote.
  ///
  /// In en, this message translates to:
  /// **'Connect another device'**
  String get settingsRuntimePairRemote;

  /// No description provided for @settingsRuntimeDiscoveredEdges.
  ///
  /// In en, this message translates to:
  /// **'ESP32 devices'**
  String get settingsRuntimeDiscoveredEdges;

  /// No description provided for @settingsRuntimeEdgeEndpointRequired.
  ///
  /// In en, this message translates to:
  /// **'Edge endpoint is required'**
  String get settingsRuntimeEdgeEndpointRequired;

  /// No description provided for @settingsRuntimePairingCodeRequired.
  ///
  /// In en, this message translates to:
  /// **'Pairing code is required'**
  String get settingsRuntimePairingCodeRequired;

  /// No description provided for @settingsRuntimeNoPairedRemote.
  ///
  /// In en, this message translates to:
  /// **'No connected devices yet.'**
  String get settingsRuntimeNoPairedRemote;

  /// No description provided for @settingsRuntimeConnectionInitiatedByOtherDevice.
  ///
  /// In en, this message translates to:
  /// **'Connection initiated by the other device'**
  String get settingsRuntimeConnectionInitiatedByOtherDevice;

  /// No description provided for @settingsRuntimePairCode.
  ///
  /// In en, this message translates to:
  /// **'Pairing code'**
  String get settingsRuntimePairCode;

  /// No description provided for @settingsRuntimeStartPairing.
  ///
  /// In en, this message translates to:
  /// **'Start connection'**
  String get settingsRuntimeStartPairing;

  /// No description provided for @settingsRuntimeFinishPairing.
  ///
  /// In en, this message translates to:
  /// **'Finish connection'**
  String get settingsRuntimeFinishPairing;

  /// No description provided for @settingsRuntimeBaseUrl.
  ///
  /// In en, this message translates to:
  /// **'Device address'**
  String get settingsRuntimeBaseUrl;

  /// No description provided for @settingsRuntimeTcpAddressHint.
  ///
  /// In en, this message translates to:
  /// **'host:port, for example 192.168.1.10:37195'**
  String get settingsRuntimeTcpAddressHint;

  /// No description provided for @settingsRuntimeTransportAuto.
  ///
  /// In en, this message translates to:
  /// **'Auto (recommended)'**
  String get settingsRuntimeTransportAuto;

  /// No description provided for @settingsRuntimeTransportAutoHint.
  ///
  /// In en, this message translates to:
  /// **'Enter host:port; the carrier (HTTP/WebSocket/TCP) is detected automatically'**
  String get settingsRuntimeTransportAutoHint;

  /// No description provided for @settingsRuntimeConnectionFailed.
  ///
  /// In en, this message translates to:
  /// **'Device connection failed: {error}'**
  String settingsRuntimeConnectionFailed(String error);

  /// No description provided for @settingsRuntimePairingRejected.
  ///
  /// In en, this message translates to:
  /// **'A device connection was rejected'**
  String get settingsRuntimePairingRejected;

  /// No description provided for @settingsRuntimePairedChecking.
  ///
  /// In en, this message translates to:
  /// **'Checking'**
  String get settingsRuntimePairedChecking;

  /// No description provided for @settingsRuntimePairedOnline.
  ///
  /// In en, this message translates to:
  /// **'Online'**
  String get settingsRuntimePairedOnline;

  /// No description provided for @settingsRuntimePairedOffline.
  ///
  /// In en, this message translates to:
  /// **'Offline'**
  String get settingsRuntimePairedOffline;

  /// No description provided for @settingsRuntimePairedInvalid.
  ///
  /// In en, this message translates to:
  /// **'Pairing invalid'**
  String get settingsRuntimePairedInvalid;

  /// No description provided for @settingsRuntimePairedError.
  ///
  /// In en, this message translates to:
  /// **'Status unavailable'**
  String get settingsRuntimePairedError;

  /// No description provided for @settingsRuntimePairingRevokedTitle.
  ///
  /// In en, this message translates to:
  /// **'Pairing revoked by the other device'**
  String get settingsRuntimePairingRevokedTitle;

  /// No description provided for @settingsRuntimePairingRevokedMessage.
  ///
  /// In en, this message translates to:
  /// **'The other device cancelled its pairing with this device. The local pairing record is no longer valid.'**
  String get settingsRuntimePairingRevokedMessage;

  /// No description provided for @settingsRuntimePairingRevokedConfirm.
  ///
  /// In en, this message translates to:
  /// **'Got it'**
  String get settingsRuntimePairingRevokedConfirm;

  /// No description provided for @settingsRuntimePairedRemovedFromSpace.
  ///
  /// In en, this message translates to:
  /// **'Removed from device space'**
  String get settingsRuntimePairedRemovedFromSpace;

  /// No description provided for @settingsRuntimeRemovedFromSpaceTitle.
  ///
  /// In en, this message translates to:
  /// **'Removed from device space'**
  String get settingsRuntimeRemovedFromSpaceTitle;

  /// No description provided for @settingsRuntimeRemovedFromSpaceMessage.
  ///
  /// In en, this message translates to:
  /// **'The remote device removed this device from the space. Leave the current space and create a standalone device space now.'**
  String get settingsRuntimeRemovedFromSpaceMessage;

  /// No description provided for @settingsRuntimeRemovedFromSpaceConfirm.
  ///
  /// In en, this message translates to:
  /// **'Leave and create standalone space'**
  String get settingsRuntimeRemovedFromSpaceConfirm;

  /// No description provided for @settingsRuntimeJoinSpace.
  ///
  /// In en, this message translates to:
  /// **'Join device space'**
  String get settingsRuntimeJoinSpace;

  /// No description provided for @settingsRuntimeJoinSpaceTitle.
  ///
  /// In en, this message translates to:
  /// **'Join {deviceName}\'s device space?'**
  String settingsRuntimeJoinSpaceTitle(String deviceName);

  /// No description provided for @settingsRuntimeJoinSpaceDescription.
  ///
  /// In en, this message translates to:
  /// **'The current device space will merge with the other one and use its name.'**
  String get settingsRuntimeJoinSpaceDescription;

  /// No description provided for @settingsRuntimePairingComplete.
  ///
  /// In en, this message translates to:
  /// **'Device paired'**
  String get settingsRuntimePairingComplete;

  /// No description provided for @settingsRuntimeDeviceInCurrentSpace.
  ///
  /// In en, this message translates to:
  /// **'In the current device space'**
  String get settingsRuntimeDeviceInCurrentSpace;

  /// No description provided for @settingsRuntimeDiscoverSpaces.
  ///
  /// In en, this message translates to:
  /// **'Discover device spaces'**
  String get settingsRuntimeDiscoverSpaces;

  /// No description provided for @settingsRuntimeDiscoverSpacesDescription.
  ///
  /// In en, this message translates to:
  /// **'Nearby devices are grouped by device space. Expand a device space to connect directly to one of its devices.'**
  String get settingsRuntimeDiscoverSpacesDescription;

  /// No description provided for @settingsRuntimeWebPairDescription.
  ///
  /// In en, this message translates to:
  /// **'Enter another device\'s address and confirm the pairing code to connect this browser as an active pairing client.'**
  String get settingsRuntimeWebPairDescription;

  /// No description provided for @settingsRuntimeDiscoveredSpaceSummary.
  ///
  /// In en, this message translates to:
  /// **'{memberCount} devices total · {nearbyCount} nearby'**
  String settingsRuntimeDiscoveredSpaceSummary(
    int memberCount,
    int nearbyCount,
  );

  /// No description provided for @settingsRuntimeScan.
  ///
  /// In en, this message translates to:
  /// **'Scan'**
  String get settingsRuntimeScan;

  /// No description provided for @settingsRuntimeScanning.
  ///
  /// In en, this message translates to:
  /// **'Scanning…'**
  String get settingsRuntimeScanning;

  /// No description provided for @settingsRuntimeEnterManually.
  ///
  /// In en, this message translates to:
  /// **'Enter manually'**
  String get settingsRuntimeEnterManually;

  /// No description provided for @settingsRuntimeConnect.
  ///
  /// In en, this message translates to:
  /// **'Connect'**
  String get settingsRuntimeConnect;

  /// No description provided for @settingsRuntimeEnableDiscovery.
  ///
  /// In en, this message translates to:
  /// **'Allow nearby devices to discover this device space'**
  String get settingsRuntimeEnableDiscovery;

  /// No description provided for @settingsRuntimeEnableDiscoveryDescription.
  ///
  /// In en, this message translates to:
  /// **'Devices on the same network can find this device space and choose this device for a direct connection.'**
  String get settingsRuntimeEnableDiscoveryDescription;

  /// No description provided for @settingsRuntimeEnableDiscoveryFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not enable device space discovery: {error}'**
  String settingsRuntimeEnableDiscoveryFailed(String error);

  /// No description provided for @settingsRuntimeDisableDiscoveryFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not disable device space discovery: {error}'**
  String settingsRuntimeDisableDiscoveryFailed(String error);

  /// No description provided for @settingsRuntimeUsingLocal.
  ///
  /// In en, this message translates to:
  /// **'Using: this device'**
  String get settingsRuntimeUsingLocal;

  /// No description provided for @settingsRuntimeUsingRemote.
  ///
  /// In en, this message translates to:
  /// **'Using: {device}'**
  String settingsRuntimeUsingRemote(String device);

  /// No description provided for @settingsRuntimeRemoteInUseDescription.
  ///
  /// In en, this message translates to:
  /// **'Chats and tools run on this connected device.'**
  String get settingsRuntimeRemoteInUseDescription;

  /// No description provided for @settingsWebAccessService.
  ///
  /// In en, this message translates to:
  /// **'Allow access'**
  String get settingsWebAccessService;

  /// No description provided for @settingsWebAccessServiceDescription.
  ///
  /// In en, this message translates to:
  /// **'When enabled, browsers can access this device with an address and token.'**
  String get settingsWebAccessServiceDescription;

  /// No description provided for @settingsWebAccessEnable.
  ///
  /// In en, this message translates to:
  /// **'Allow external access'**
  String get settingsWebAccessEnable;

  /// No description provided for @settingsWebAccessPortMode.
  ///
  /// In en, this message translates to:
  /// **'Port mode'**
  String get settingsWebAccessPortMode;

  /// No description provided for @settingsWebAccessPortAutomatic.
  ///
  /// In en, this message translates to:
  /// **'Automatic'**
  String get settingsWebAccessPortAutomatic;

  /// No description provided for @settingsWebAccessPortFixed.
  ///
  /// In en, this message translates to:
  /// **'Fixed'**
  String get settingsWebAccessPortFixed;

  /// No description provided for @settingsWebAccessPortAutomaticDescription.
  ///
  /// In en, this message translates to:
  /// **'The app chooses a port automatically. No manual setup is needed.'**
  String get settingsWebAccessPortAutomaticDescription;

  /// No description provided for @settingsWebAccessPortFixedDescription.
  ///
  /// In en, this message translates to:
  /// **'Only the port in the listen address is used.'**
  String get settingsWebAccessPortFixedDescription;

  /// No description provided for @settingsWebAccessBindAddress.
  ///
  /// In en, this message translates to:
  /// **'Listen address'**
  String get settingsWebAccessBindAddress;

  /// No description provided for @settingsWebAccessToken.
  ///
  /// In en, this message translates to:
  /// **'Access token'**
  String get settingsWebAccessToken;

  /// No description provided for @settingsWebAccessRotateToken.
  ///
  /// In en, this message translates to:
  /// **'Change token'**
  String get settingsWebAccessRotateToken;

  /// No description provided for @settingsWebAccessCopyToken.
  ///
  /// In en, this message translates to:
  /// **'Copy token'**
  String get settingsWebAccessCopyToken;

  /// No description provided for @settingsWebAccessAccessUrl.
  ///
  /// In en, this message translates to:
  /// **'Access address'**
  String get settingsWebAccessAccessUrl;

  /// No description provided for @settingsWebAccessLocalUrl.
  ///
  /// In en, this message translates to:
  /// **'This device'**
  String get settingsWebAccessLocalUrl;

  /// No description provided for @settingsWebAccessPairingUrl.
  ///
  /// In en, this message translates to:
  /// **'Pairing address'**
  String get settingsWebAccessPairingUrl;

  /// No description provided for @settingsWebAccessPairingUrlLocalOnly.
  ///
  /// In en, this message translates to:
  /// **'This device only'**
  String get settingsWebAccessPairingUrlLocalOnly;

  /// No description provided for @settingsWebAccessPairingUrlUnavailable.
  ///
  /// In en, this message translates to:
  /// **'No LAN address found'**
  String get settingsWebAccessPairingUrlUnavailable;

  /// No description provided for @settingsWebAccessCopyUrl.
  ///
  /// In en, this message translates to:
  /// **'Copy URL'**
  String get settingsWebAccessCopyUrl;

  /// No description provided for @settingsWebAccessOpenUrl.
  ///
  /// In en, this message translates to:
  /// **'Open address'**
  String get settingsWebAccessOpenUrl;

  /// No description provided for @settingsWebAccessRunning.
  ///
  /// In en, this message translates to:
  /// **'On'**
  String get settingsWebAccessRunning;

  /// No description provided for @settingsWebAccessStopped.
  ///
  /// In en, this message translates to:
  /// **'Off'**
  String get settingsWebAccessStopped;

  /// No description provided for @settingsWebAccessSaved.
  ///
  /// In en, this message translates to:
  /// **'Access settings saved.'**
  String get settingsWebAccessSaved;

  /// No description provided for @settingsWebAccessTokenCopied.
  ///
  /// In en, this message translates to:
  /// **'Access token copied.'**
  String get settingsWebAccessTokenCopied;

  /// No description provided for @settingsWebAccessUrlCopied.
  ///
  /// In en, this message translates to:
  /// **'Access URL copied.'**
  String get settingsWebAccessUrlCopied;

  /// No description provided for @settingsWebAccessPairedClients.
  ///
  /// In en, this message translates to:
  /// **'Authorized devices'**
  String get settingsWebAccessPairedClients;

  /// No description provided for @settingsWebAccessNoPairedClients.
  ///
  /// In en, this message translates to:
  /// **'No device is authorized yet.'**
  String get settingsWebAccessNoPairedClients;

  /// No description provided for @settingsWebAccessPairedDeleted.
  ///
  /// In en, this message translates to:
  /// **'Authorized device deleted.'**
  String get settingsWebAccessPairedDeleted;

  /// No description provided for @settingsWebAccessPairingRequest.
  ///
  /// In en, this message translates to:
  /// **'Pairing request'**
  String get settingsWebAccessPairingRequest;

  /// No description provided for @settingsWebAccessPairingRequestMessage.
  ///
  /// In en, this message translates to:
  /// **'Pairing code: {code}\nDevice: {client}'**
  String settingsWebAccessPairingRequestMessage(String code, String client);

  /// No description provided for @settingsWebAccessInvalidBindAddress.
  ///
  /// In en, this message translates to:
  /// **'Bind address must be host:port.'**
  String get settingsWebAccessInvalidBindAddress;

  /// No description provided for @settingsWebAccessStartFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to enable access: {error}'**
  String settingsWebAccessStartFailed(String error);

  /// No description provided for @settingsWebAccessStopFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to turn off access: {error}'**
  String settingsWebAccessStopFailed(String error);

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

  /// No description provided for @settingsAppearanceInputSection.
  ///
  /// In en, this message translates to:
  /// **'Input'**
  String get settingsAppearanceInputSection;

  /// No description provided for @settingsAppearanceInputStyle.
  ///
  /// In en, this message translates to:
  /// **'Input style'**
  String get settingsAppearanceInputStyle;

  /// No description provided for @settingsAppearanceInputStyleClassic.
  ///
  /// In en, this message translates to:
  /// **'Classic'**
  String get settingsAppearanceInputStyleClassic;

  /// No description provided for @settingsAppearanceInputStyleAgent.
  ///
  /// In en, this message translates to:
  /// **'Agent'**
  String get settingsAppearanceInputStyleAgent;

  /// No description provided for @settingsAppearanceInputFloating.
  ///
  /// In en, this message translates to:
  /// **'Floating input'**
  String get settingsAppearanceInputFloating;

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

  /// No description provided for @settingsAppearanceNormalFontSection.
  ///
  /// In en, this message translates to:
  /// **'Body font'**
  String get settingsAppearanceNormalFontSection;

  /// No description provided for @settingsAppearanceFontsSection.
  ///
  /// In en, this message translates to:
  /// **'Fonts'**
  String get settingsAppearanceFontsSection;

  /// No description provided for @settingsAppearanceCodeFontSection.
  ///
  /// In en, this message translates to:
  /// **'Code font'**
  String get settingsAppearanceCodeFontSection;

  /// No description provided for @settingsAppearanceCodeFontDescription.
  ///
  /// In en, this message translates to:
  /// **'Used by code blocks, inline code, diffs, file previews and the terminal'**
  String get settingsAppearanceCodeFontDescription;

  /// No description provided for @settingsAppearanceCodeFontBuiltin.
  ///
  /// In en, this message translates to:
  /// **'Built-in mono'**
  String get settingsAppearanceCodeFontBuiltin;

  /// No description provided for @settingsAppearanceChatBodyFontSection.
  ///
  /// In en, this message translates to:
  /// **'Chat text font'**
  String get settingsAppearanceChatBodyFontSection;

  /// No description provided for @settingsAppearanceChatBodyFontFollow.
  ///
  /// In en, this message translates to:
  /// **'Follows the body font'**
  String get settingsAppearanceChatBodyFontFollow;

  /// No description provided for @settingsAppearanceChatBodySplitSides.
  ///
  /// In en, this message translates to:
  /// **'Set user and AI separately'**
  String get settingsAppearanceChatBodySplitSides;

  /// No description provided for @settingsAppearanceChatBodyAdjust.
  ///
  /// In en, this message translates to:
  /// **'Adjust'**
  String get settingsAppearanceChatBodyAdjust;

  /// No description provided for @settingsAppearanceInterfaceScale.
  ///
  /// In en, this message translates to:
  /// **'Interface scale'**
  String get settingsAppearanceInterfaceScale;

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

  /// No description provided for @settingsAppearanceUserMessageStyle.
  ///
  /// In en, this message translates to:
  /// **'User message style'**
  String get settingsAppearanceUserMessageStyle;

  /// No description provided for @settingsAppearanceAiMessageStyle.
  ///
  /// In en, this message translates to:
  /// **'AI message style'**
  String get settingsAppearanceAiMessageStyle;

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
  /// **'Live preview'**
  String get settingsAppearanceBubbleImagePreview;

  /// No description provided for @settingsAppearanceBubbleImagePreviewText.
  ///
  /// In en, this message translates to:
  /// **'This is a bubble preview message.\nCheck the corners, edges, and center at their rendered size.'**
  String get settingsAppearanceBubbleImagePreviewText;

  /// No description provided for @settingsAppearanceBubbleImageSource.
  ///
  /// In en, this message translates to:
  /// **'Original'**
  String get settingsAppearanceBubbleImageSource;

  /// No description provided for @settingsAppearanceBubbleImageSourceHint.
  ///
  /// In en, this message translates to:
  /// **'Original · White: crop boundary. Blue: nine-slice grid. Pinch to zoom and drag to inspect.'**
  String get settingsAppearanceBubbleImageSourceHint;

  /// No description provided for @settingsAppearanceBubbleImageFitSource.
  ///
  /// In en, this message translates to:
  /// **'Fit original image'**
  String get settingsAppearanceBubbleImageFitSource;

  /// No description provided for @settingsAppearanceBubbleImageGrid.
  ///
  /// In en, this message translates to:
  /// **'9-slice'**
  String get settingsAppearanceBubbleImageGrid;

  /// No description provided for @settingsAppearanceBubbleImageGridHint.
  ///
  /// In en, this message translates to:
  /// **'Two horizontal and two vertical cuts form nine regions. Corners stay fixed; edges tile along one axis and the center tiles along both. Stretch mode stretches those regions instead. Boundaries are relative to the cropped image.'**
  String get settingsAppearanceBubbleImageGridHint;

  /// No description provided for @settingsAppearanceBubbleImageImportFailed.
  ///
  /// In en, this message translates to:
  /// **'Bubble image was not applied: {error}'**
  String settingsAppearanceBubbleImageImportFailed(String error);

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

  /// No description provided for @settingsAppearanceChatHorizontalPadding.
  ///
  /// In en, this message translates to:
  /// **'Chat content side padding'**
  String get settingsAppearanceChatHorizontalPadding;

  /// No description provided for @settingsAppearanceChatHorizontalPaddingEdge.
  ///
  /// In en, this message translates to:
  /// **'Edge to edge'**
  String get settingsAppearanceChatHorizontalPaddingEdge;

  /// No description provided for @settingsAppearanceChatHorizontalPaddingDetail.
  ///
  /// In en, this message translates to:
  /// **'Smaller values widen the transcript; bubble and composer insets are unaffected.'**
  String get settingsAppearanceChatHorizontalPaddingDetail;

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

  /// No description provided for @settingsAppearanceMessageDisplaySection.
  ///
  /// In en, this message translates to:
  /// **'Message display'**
  String get settingsAppearanceMessageDisplaySection;

  /// No description provided for @settingsAppearanceShowThinkingProcess.
  ///
  /// In en, this message translates to:
  /// **'Show thinking process'**
  String get settingsAppearanceShowThinkingProcess;

  /// No description provided for @settingsAppearanceShowRoleName.
  ///
  /// In en, this message translates to:
  /// **'Show custom responder names'**
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

  /// No description provided for @settingsAppLanguageTitle.
  ///
  /// In en, this message translates to:
  /// **'App language'**
  String get settingsAppLanguageTitle;

  /// No description provided for @settingsAppLanguageSystem.
  ///
  /// In en, this message translates to:
  /// **'Follow system'**
  String get settingsAppLanguageSystem;

  /// No description provided for @settingsAppLanguageDescription.
  ///
  /// In en, this message translates to:
  /// **'Changes apply immediately. No restart required.'**
  String get settingsAppLanguageDescription;

  /// No description provided for @settingsAppLanguageSaveError.
  ///
  /// In en, this message translates to:
  /// **'Could not save app language: {error}'**
  String settingsAppLanguageSaveError(String error);

  /// No description provided for @settingsAppearanceBubblesTab.
  ///
  /// In en, this message translates to:
  /// **'Messages & Bubbles'**
  String get settingsAppearanceBubblesTab;

  /// No description provided for @settingsAppearanceInteractionTab.
  ///
  /// In en, this message translates to:
  /// **'Interaction & Feed'**
  String get settingsAppearanceInteractionTab;

  /// No description provided for @settingsAppearanceBubbleStyleSection.
  ///
  /// In en, this message translates to:
  /// **'Layout & Style'**
  String get settingsAppearanceBubbleStyleSection;

  /// No description provided for @settingsAppearanceBubbleFontSection.
  ///
  /// In en, this message translates to:
  /// **'Bubble Fonts'**
  String get settingsAppearanceBubbleFontSection;

  /// No description provided for @settingsAppearanceBubbleImageSection.
  ///
  /// In en, this message translates to:
  /// **'Bubble Images'**
  String get settingsAppearanceBubbleImageSection;

  /// No description provided for @settingsAppearanceInteractionStatusSection.
  ///
  /// In en, this message translates to:
  /// **'Process & Status'**
  String get settingsAppearanceInteractionStatusSection;

  /// No description provided for @settingsAppearanceBackgroundEffectsSection.
  ///
  /// In en, this message translates to:
  /// **'Visual Effects'**
  String get settingsAppearanceBackgroundEffectsSection;

  /// No description provided for @settingsAppearanceBackgroundVideoSection.
  ///
  /// In en, this message translates to:
  /// **'Video Playback'**
  String get settingsAppearanceBackgroundVideoSection;

  /// No description provided for @settingsAppearanceLivePreviewTitle.
  ///
  /// In en, this message translates to:
  /// **'Live Preview'**
  String get settingsAppearanceLivePreviewTitle;

  /// No description provided for @settingsAppearanceLivePreviewUserSample.
  ///
  /// In en, this message translates to:
  /// **'Help me organize today\'s work plan.'**
  String get settingsAppearanceLivePreviewUserSample;

  /// No description provided for @settingsAppearanceLivePreviewAiSample.
  ///
  /// In en, this message translates to:
  /// **'Sure! I\'ve organized your core task list and we\'re ready to start.'**
  String get settingsAppearanceLivePreviewAiSample;

  /// No description provided for @settingsAppearanceLivePreviewThinkingSample.
  ///
  /// In en, this message translates to:
  /// **'Analyzing context and schedule priorities...'**
  String get settingsAppearanceLivePreviewThinkingSample;

  /// No description provided for @settingsDataLogsSection.
  ///
  /// In en, this message translates to:
  /// **'Diagnostic logs'**
  String get settingsDataLogsSection;

  /// No description provided for @settingsDataLogsDescription.
  ///
  /// In en, this message translates to:
  /// **'Export the selected Core and this client’s logs as one time-ordered .log file, marked [CORE] / [CLIENT]. Logs may contain sensitive data; review before sharing.'**
  String get settingsDataLogsDescription;

  /// No description provided for @settingsDataExportLogs.
  ///
  /// In en, this message translates to:
  /// **'Export diagnostic logs'**
  String get settingsDataExportLogs;

  /// No description provided for @settingsDataLogsExporting.
  ///
  /// In en, this message translates to:
  /// **'Exporting logs…'**
  String get settingsDataLogsExporting;

  /// No description provided for @settingsDataLogsExportError.
  ///
  /// In en, this message translates to:
  /// **'Log export failed: {error}'**
  String settingsDataLogsExportError(String error);

  /// No description provided for @settingsDataLogsExportPartial.
  ///
  /// In en, this message translates to:
  /// **'Saved to {path}. One log source could not be read; see the warnings in the file.'**
  String settingsDataLogsExportPartial(String path);

  /// No description provided for @settingsDataOverviewLoadError.
  ///
  /// In en, this message translates to:
  /// **'The data overview could not be loaded: {error}. You can still export diagnostic logs.'**
  String settingsDataOverviewLoadError(String error);

  /// No description provided for @settingsDataRuntimeSection.
  ///
  /// In en, this message translates to:
  /// **'Data overview'**
  String get settingsDataRuntimeSection;

  /// No description provided for @settingsDataCoreVersion.
  ///
  /// In en, this message translates to:
  /// **'Current version'**
  String get settingsDataCoreVersion;

  /// No description provided for @settingsDataStorageSection.
  ///
  /// In en, this message translates to:
  /// **'Storage location'**
  String get settingsDataStorageSection;

  /// No description provided for @settingsDataStorageDescription.
  ///
  /// In en, this message translates to:
  /// **'Move runtime and workspace data into independently selected local folders.'**
  String get settingsDataStorageDescription;

  /// No description provided for @settingsDataRuntimeRoot.
  ///
  /// In en, this message translates to:
  /// **'Runtime root'**
  String get settingsDataRuntimeRoot;

  /// No description provided for @settingsDataWorkspaceRoot.
  ///
  /// In en, this message translates to:
  /// **'Workspace root'**
  String get settingsDataWorkspaceRoot;

  /// No description provided for @settingsDataChooseStorageRoots.
  ///
  /// In en, this message translates to:
  /// **'Edit locations'**
  String get settingsDataChooseStorageRoots;

  /// No description provided for @settingsDataEditStorageRootsTitle.
  ///
  /// In en, this message translates to:
  /// **'Edit storage locations'**
  String get settingsDataEditStorageRootsTitle;

  /// No description provided for @settingsDataStorageRootsRequired.
  ///
  /// In en, this message translates to:
  /// **'Runtime root and workspace root are required.'**
  String get settingsDataStorageRootsRequired;

  /// No description provided for @storageDirectoryTermuxUnsupported.
  ///
  /// In en, this message translates to:
  /// **'Termux\'s document provider is not supported for local storage paths. Choose a directory in shared storage (such as Download or Documents) instead of Termux\'s private directory.'**
  String get storageDirectoryTermuxUnsupported;

  /// No description provided for @storageDirectoryProviderUnsupported.
  ///
  /// In en, this message translates to:
  /// **'The selected directory cannot be resolved to a supported local storage path. Choose a directory in shared storage (such as Download or Documents), or enter a local path that this app can access.'**
  String get storageDirectoryProviderUnsupported;

  /// No description provided for @storageDirectorySelectionFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not select a directory: {error}'**
  String storageDirectorySelectionFailed(String error);

  /// No description provided for @settingsDataStorageConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Change storage location'**
  String get settingsDataStorageConfirmTitle;

  /// No description provided for @settingsDataStorageConfirmMessage.
  ///
  /// In en, this message translates to:
  /// **'Runtime and workspace data will be copied into the selected directories, and the app will use them after restart.'**
  String get settingsDataStorageConfirmMessage;

  /// No description provided for @settingsDataStorageConfirmAction.
  ///
  /// In en, this message translates to:
  /// **'Change location'**
  String get settingsDataStorageConfirmAction;

  /// No description provided for @settingsDataStorageChanged.
  ///
  /// In en, this message translates to:
  /// **'Storage location changed. Restart the app to use it.'**
  String get settingsDataStorageChanged;

  /// No description provided for @settingsDataStorageChangeError.
  ///
  /// In en, this message translates to:
  /// **'Storage location change failed: {error}'**
  String settingsDataStorageChangeError(String error);

  /// No description provided for @settingsDataInputTokens.
  ///
  /// In en, this message translates to:
  /// **'Input'**
  String get settingsDataInputTokens;

  /// No description provided for @settingsDataOutputTokens.
  ///
  /// In en, this message translates to:
  /// **'Output'**
  String get settingsDataOutputTokens;

  /// No description provided for @settingsDataResetTokenStats.
  ///
  /// In en, this message translates to:
  /// **'Reset statistics'**
  String get settingsDataResetTokenStats;

  /// No description provided for @settingsUsageReportResetConfirmation.
  ///
  /// In en, this message translates to:
  /// **'This permanently deletes all saved model usage records. Your conversations are kept.'**
  String get settingsUsageReportResetConfirmation;

  /// No description provided for @settingsDataDetailedStatsDescription.
  ///
  /// In en, this message translates to:
  /// **'Statistics are calculated from dedicated model request records.'**
  String get settingsDataDetailedStatsDescription;

  /// No description provided for @settingsDataDetailedStatsEmpty.
  ///
  /// In en, this message translates to:
  /// **'No detailed usage records yet'**
  String get settingsDataDetailedStatsEmpty;

  /// No description provided for @settingsDataDetailedStatsDateRange.
  ///
  /// In en, this message translates to:
  /// **'{start} to {end}'**
  String settingsDataDetailedStatsDateRange(String start, String end);

  /// No description provided for @settingsDataDetailedStatsSourceLabel.
  ///
  /// In en, this message translates to:
  /// **'Model request records'**
  String get settingsDataDetailedStatsSourceLabel;

  /// No description provided for @settingsDataDetailedStatsHistoricalTotal.
  ///
  /// In en, this message translates to:
  /// **'Historical total'**
  String get settingsDataDetailedStatsHistoricalTotal;

  /// No description provided for @settingsDataDetailedStatsRangeAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get settingsDataDetailedStatsRangeAll;

  /// No description provided for @settingsDataDetailedStatsRangeLast7Days.
  ///
  /// In en, this message translates to:
  /// **'7 days'**
  String get settingsDataDetailedStatsRangeLast7Days;

  /// No description provided for @settingsDataDetailedStatsRangeLast30Days.
  ///
  /// In en, this message translates to:
  /// **'30 days'**
  String get settingsDataDetailedStatsRangeLast30Days;

  /// No description provided for @settingsDataDetailedStatsRangeCustom.
  ///
  /// In en, this message translates to:
  /// **'Custom'**
  String get settingsDataDetailedStatsRangeCustom;

  /// No description provided for @settingsDataDetailedStatsCustomRangePickerTitle.
  ///
  /// In en, this message translates to:
  /// **'Select statistics date range'**
  String get settingsDataDetailedStatsCustomRangePickerTitle;

  /// No description provided for @settingsDataDetailedStatsNoDataInRange.
  ///
  /// In en, this message translates to:
  /// **'No data in this range'**
  String get settingsDataDetailedStatsNoDataInRange;

  /// No description provided for @settingsDataDetailedStatsNoRankRows.
  ///
  /// In en, this message translates to:
  /// **'No ranking data in this range'**
  String get settingsDataDetailedStatsNoRankRows;

  /// No description provided for @settingsDataDetailedStatsFunctionModels.
  ///
  /// In en, this message translates to:
  /// **'Function models'**
  String get settingsDataDetailedStatsFunctionModels;

  /// No description provided for @settingsDataDetailedStatsFunctionModelPieTitle.
  ///
  /// In en, this message translates to:
  /// **'Function model distribution'**
  String get settingsDataDetailedStatsFunctionModelPieTitle;

  /// No description provided for @settingsDataDetailedStatsTopFunctionModelsTitle.
  ///
  /// In en, this message translates to:
  /// **'Function model ranking'**
  String get settingsDataDetailedStatsTopFunctionModelsTitle;

  /// No description provided for @settingsDataDetailedStatsTopFunctionModelsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Highest total token consumption by function model'**
  String get settingsDataDetailedStatsTopFunctionModelsSubtitle;

  /// No description provided for @settingsDataDetailedStatsSourceChat.
  ///
  /// In en, this message translates to:
  /// **'Chat response'**
  String get settingsDataDetailedStatsSourceChat;

  /// No description provided for @settingsDataDetailedStatsSourceToolResult.
  ///
  /// In en, this message translates to:
  /// **'Tool-result response'**
  String get settingsDataDetailedStatsSourceToolResult;

  /// No description provided for @settingsDataDetailedStatsSourceSummary.
  ///
  /// In en, this message translates to:
  /// **'Summary generation'**
  String get settingsDataDetailedStatsSourceSummary;

  /// No description provided for @settingsDataDetailedStatsSourceTitleGeneration.
  ///
  /// In en, this message translates to:
  /// **'Title generation'**
  String get settingsDataDetailedStatsSourceTitleGeneration;

  /// No description provided for @settingsDataDetailedStatsTotalRequests.
  ///
  /// In en, this message translates to:
  /// **'Total requests'**
  String get settingsDataDetailedStatsTotalRequests;

  /// No description provided for @settingsDataDetailedStatsCachedInput.
  ///
  /// In en, this message translates to:
  /// **'Cached input'**
  String get settingsDataDetailedStatsCachedInput;

  /// No description provided for @settingsDataDetailedStatsActiveDays.
  ///
  /// In en, this message translates to:
  /// **'Active days'**
  String get settingsDataDetailedStatsActiveDays;

  /// No description provided for @settingsDataDetailedStatsChats.
  ///
  /// In en, this message translates to:
  /// **'Conversations'**
  String get settingsDataDetailedStatsChats;

  /// No description provided for @settingsDataDetailedStatsProviders.
  ///
  /// In en, this message translates to:
  /// **'Providers'**
  String get settingsDataDetailedStatsProviders;

  /// No description provided for @settingsDataDetailedStatsModels.
  ///
  /// In en, this message translates to:
  /// **'Models'**
  String get settingsDataDetailedStatsModels;

  /// No description provided for @settingsDataDetailedStatsUnknownTokenRecords.
  ///
  /// In en, this message translates to:
  /// **'Records with unknown tokens'**
  String get settingsDataDetailedStatsUnknownTokenRecords;

  /// No description provided for @settingsDataDetailedStatsHeatmapTitle.
  ///
  /// In en, this message translates to:
  /// **'Usage heatmap'**
  String get settingsDataDetailedStatsHeatmapTitle;

  /// No description provided for @settingsDataDetailedStatsHeatmapSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Daily token intensity across the selected range'**
  String get settingsDataDetailedStatsHeatmapSubtitle;

  /// No description provided for @settingsDataDetailedStatsHeatmapTapHint.
  ///
  /// In en, this message translates to:
  /// **'Tap a cell to view details'**
  String get settingsDataDetailedStatsHeatmapTapHint;

  /// No description provided for @settingsDataDetailedStatsHeatmapDayDetail.
  ///
  /// In en, this message translates to:
  /// **'{date} used {tokens} tokens · {requests} requests'**
  String settingsDataDetailedStatsHeatmapDayDetail(
    String date,
    String tokens,
    String requests,
  );

  /// No description provided for @settingsDataDetailedStatsHeatmapLow.
  ///
  /// In en, this message translates to:
  /// **'Less'**
  String get settingsDataDetailedStatsHeatmapLow;

  /// No description provided for @settingsDataDetailedStatsHeatmapHigh.
  ///
  /// In en, this message translates to:
  /// **'More'**
  String get settingsDataDetailedStatsHeatmapHigh;

  /// No description provided for @settingsDataDetailedStatsDailyUsageTitle.
  ///
  /// In en, this message translates to:
  /// **'Daily usage trend'**
  String get settingsDataDetailedStatsDailyUsageTitle;

  /// No description provided for @settingsDataDetailedStatsDailyUsageSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Request count by day'**
  String get settingsDataDetailedStatsDailyUsageSubtitle;

  /// No description provided for @settingsDataDetailedStatsRequestsSeries.
  ///
  /// In en, this message translates to:
  /// **'Requests'**
  String get settingsDataDetailedStatsRequestsSeries;

  /// No description provided for @settingsDataDetailedStatsInputOutputTitle.
  ///
  /// In en, this message translates to:
  /// **'Input / output consumption trend'**
  String get settingsDataDetailedStatsInputOutputTitle;

  /// No description provided for @settingsDataDetailedStatsInputOutputSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Daily token changes for input and output'**
  String get settingsDataDetailedStatsInputOutputSubtitle;

  /// No description provided for @settingsDataDetailedStatsProviderPieTitle.
  ///
  /// In en, this message translates to:
  /// **'Provider distribution'**
  String get settingsDataDetailedStatsProviderPieTitle;

  /// No description provided for @settingsDataDetailedStatsModelPieTitle.
  ///
  /// In en, this message translates to:
  /// **'Model distribution'**
  String get settingsDataDetailedStatsModelPieTitle;

  /// No description provided for @settingsDataDetailedStatsChatPieTitle.
  ///
  /// In en, this message translates to:
  /// **'Conversation distribution'**
  String get settingsDataDetailedStatsChatPieTitle;

  /// No description provided for @settingsDataDetailedStatsTotalTokens.
  ///
  /// In en, this message translates to:
  /// **'Total tokens'**
  String get settingsDataDetailedStatsTotalTokens;

  /// No description provided for @settingsDataDetailedStatsTopRequestsTitle.
  ///
  /// In en, this message translates to:
  /// **'Single-request peaks'**
  String get settingsDataDetailedStatsTopRequestsTitle;

  /// No description provided for @settingsDataDetailedStatsTopRequestsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Highest token consumption from individual calls'**
  String get settingsDataDetailedStatsTopRequestsSubtitle;

  /// No description provided for @settingsDataDetailedStatsTopChatsTitle.
  ///
  /// In en, this message translates to:
  /// **'Top conversations'**
  String get settingsDataDetailedStatsTopChatsTitle;

  /// No description provided for @settingsDataDetailedStatsTopChatsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Highest total token consumption by conversation'**
  String get settingsDataDetailedStatsTopChatsSubtitle;

  /// No description provided for @settingsDataDetailedStatsOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get settingsDataDetailedStatsOther;

  /// No description provided for @settingsDataDetailedStatsInputOutputSummary.
  ///
  /// In en, this message translates to:
  /// **'Input {input} · Output {output} · {chatTitle} · {time}'**
  String settingsDataDetailedStatsInputOutputSummary(
    String input,
    String output,
    String chatTitle,
    String time,
  );

  /// No description provided for @settingsDataDetailedStatsRequestModelSummary.
  ///
  /// In en, this message translates to:
  /// **'{requests} requests · {models} models'**
  String settingsDataDetailedStatsRequestModelSummary(int requests, int models);

  /// No description provided for @settingsDataDetailedStatsRequestCountSummary.
  ///
  /// In en, this message translates to:
  /// **'{requests} requests'**
  String settingsDataDetailedStatsRequestCountSummary(int requests);

  /// No description provided for @settingsDataDetailedStatsUnlabeledProvider.
  ///
  /// In en, this message translates to:
  /// **'Unlabeled provider'**
  String get settingsDataDetailedStatsUnlabeledProvider;

  /// No description provided for @settingsDataDetailedStatsUnlabeledModel.
  ///
  /// In en, this message translates to:
  /// **'Unlabeled model'**
  String get settingsDataDetailedStatsUnlabeledModel;

  /// No description provided for @settingsDataDetailedStatsUntitledChat.
  ///
  /// In en, this message translates to:
  /// **'Untitled conversation'**
  String get settingsDataDetailedStatsUntitledChat;

  /// No description provided for @settingsDataBackupSection.
  ///
  /// In en, this message translates to:
  /// **'Backup & restore'**
  String get settingsDataBackupSection;

  /// No description provided for @settingsDataChatHistoriesBackup.
  ///
  /// In en, this message translates to:
  /// **'Chat data'**
  String get settingsDataChatHistoriesBackup;

  /// No description provided for @settingsDataChatHistoriesBackupDescription.
  ///
  /// In en, this message translates to:
  /// **'Back up all chats and messages. Restore updates or creates chats by chat ID.'**
  String get settingsDataChatHistoriesBackupDescription;

  /// No description provided for @settingsDataModelConfigsBackup.
  ///
  /// In en, this message translates to:
  /// **'Model settings'**
  String get settingsDataModelConfigsBackup;

  /// No description provided for @settingsDataModelConfigsBackupDescription.
  ///
  /// In en, this message translates to:
  /// **'Back up all model settings, including model parameters and API key pools.'**
  String get settingsDataModelConfigsBackupDescription;

  /// No description provided for @settingsDataBackupCount.
  ///
  /// In en, this message translates to:
  /// **'{count} items'**
  String settingsDataBackupCount(int count);

  /// No description provided for @settingsDataExportBackupJson.
  ///
  /// In en, this message translates to:
  /// **'Export backup'**
  String get settingsDataExportBackupJson;

  /// No description provided for @settingsDataImportBackupJson.
  ///
  /// In en, this message translates to:
  /// **'Restore data'**
  String get settingsDataImportBackupJson;

  /// No description provided for @settingsDataBackupImportResult.
  ///
  /// In en, this message translates to:
  /// **'Restore complete: {newCount} new, {updatedCount} updated, {skippedCount} skipped.'**
  String settingsDataBackupImportResult(
    int newCount,
    int updatedCount,
    int skippedCount,
  );

  /// No description provided for @settingsDataBackupImportError.
  ///
  /// In en, this message translates to:
  /// **'Restore failed: {error}'**
  String settingsDataBackupImportError(String error);

  /// No description provided for @settingsDataBackupExportError.
  ///
  /// In en, this message translates to:
  /// **'Export failed: {error}'**
  String settingsDataBackupExportError(String error);

  /// No description provided for @settingsDataSnapshotBackupTitle.
  ///
  /// In en, this message translates to:
  /// **'Full snapshot'**
  String get settingsDataSnapshotBackupTitle;

  /// No description provided for @settingsDataExportRawSnapshot.
  ///
  /// In en, this message translates to:
  /// **'Export full snapshot'**
  String get settingsDataExportRawSnapshot;

  /// No description provided for @settingsDataImportRawSnapshot.
  ///
  /// In en, this message translates to:
  /// **'Restore a full snapshot'**
  String get settingsDataImportRawSnapshot;

  /// No description provided for @settingsDataExportRawSnapshotDescription.
  ///
  /// In en, this message translates to:
  /// **'Package chats, plugin data, model settings, and local files as an .opsnapshot file.'**
  String get settingsDataExportRawSnapshotDescription;

  /// No description provided for @settingsDataDatabaseBackup.
  ///
  /// In en, this message translates to:
  /// **'Database backup'**
  String get settingsDataDatabaseBackup;

  /// No description provided for @settingsDataDatabaseBackupDescription.
  ///
  /// In en, this message translates to:
  /// **'Save a copy of the database daily and keep the 10 most recent automatic backups. Attachments and workspace files are stored separately.'**
  String get settingsDataDatabaseBackupDescription;

  /// No description provided for @settingsDataBackupScheduled.
  ///
  /// In en, this message translates to:
  /// **'Daily backup is on'**
  String get settingsDataBackupScheduled;

  /// No description provided for @settingsDataBackupUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Choose an accessible folder to turn on daily backups'**
  String get settingsDataBackupUnavailable;

  /// No description provided for @settingsDataBackupNow.
  ///
  /// In en, this message translates to:
  /// **'Back up now'**
  String get settingsDataBackupNow;

  /// No description provided for @settingsDataBackupSaved.
  ///
  /// In en, this message translates to:
  /// **'Database backup saved.'**
  String get settingsDataBackupSaved;

  /// No description provided for @settingsDataRecoverySection.
  ///
  /// In en, this message translates to:
  /// **'Restore data'**
  String get settingsDataRecoverySection;

  /// No description provided for @settingsDataRestoreDatabaseDescription.
  ///
  /// In en, this message translates to:
  /// **'Choose a dated database backup. The restored data becomes the source of truth for sync with other devices.'**
  String get settingsDataRestoreDatabaseDescription;

  /// No description provided for @settingsDataRestoreArchiveDescription.
  ///
  /// In en, this message translates to:
  /// **'Replace current runtime data with an Operit2 full snapshot and restart the app.'**
  String get settingsDataRestoreArchiveDescription;

  /// No description provided for @settingsDataImportSection.
  ///
  /// In en, this message translates to:
  /// **'Import data'**
  String get settingsDataImportSection;

  /// No description provided for @settingsDataImportOperit1Description.
  ///
  /// In en, this message translates to:
  /// **'Preview an Operit1 snapshot before migrating chats, model settings, and files.'**
  String get settingsDataImportOperit1Description;

  /// No description provided for @settingsDataImportChatJson.
  ///
  /// In en, this message translates to:
  /// **'Import chat JSON'**
  String get settingsDataImportChatJson;

  /// No description provided for @settingsDataImportChatJsonDescription.
  ///
  /// In en, this message translates to:
  /// **'Add chats and update existing chats with matching IDs.'**
  String get settingsDataImportChatJsonDescription;

  /// No description provided for @settingsDataImportModelJson.
  ///
  /// In en, this message translates to:
  /// **'Import model settings JSON'**
  String get settingsDataImportModelJson;

  /// No description provided for @settingsDataImportModelJsonDescription.
  ///
  /// In en, this message translates to:
  /// **'Add model provider settings and update existing providers with matching IDs.'**
  String get settingsDataImportModelJsonDescription;

  /// No description provided for @settingsDataImportAction.
  ///
  /// In en, this message translates to:
  /// **'Import'**
  String get settingsDataImportAction;

  /// No description provided for @settingsDataReadingSnapshot.
  ///
  /// In en, this message translates to:
  /// **'Reading snapshot'**
  String get settingsDataReadingSnapshot;

  /// No description provided for @settingsDataPreparingPreview.
  ///
  /// In en, this message translates to:
  /// **'Preparing preview'**
  String get settingsDataPreparingPreview;

  /// No description provided for @settingsDataOperit1ImportResult.
  ///
  /// In en, this message translates to:
  /// **'Imported {chatCount} chats and {messageCount} messages.'**
  String settingsDataOperit1ImportResult(int chatCount, int messageCount);

  /// No description provided for @settingsDataExportSection.
  ///
  /// In en, this message translates to:
  /// **'Export files'**
  String get settingsDataExportSection;

  /// No description provided for @settingsDataExportChatJson.
  ///
  /// In en, this message translates to:
  /// **'Export chat JSON'**
  String get settingsDataExportChatJson;

  /// No description provided for @settingsDataExportModelJson.
  ///
  /// In en, this message translates to:
  /// **'Export model settings JSON'**
  String get settingsDataExportModelJson;

  /// No description provided for @settingsDataSnapshotBytes.
  ///
  /// In en, this message translates to:
  /// **'Snapshot size: {bytes} bytes'**
  String settingsDataSnapshotBytes(int bytes);

  /// No description provided for @settingsDataSnapshotImported.
  ///
  /// In en, this message translates to:
  /// **'Snapshot restored.'**
  String get settingsDataSnapshotImported;

  /// No description provided for @settingsDataSnapshotExportError.
  ///
  /// In en, this message translates to:
  /// **'Snapshot export failed: {error}'**
  String settingsDataSnapshotExportError(String error);

  /// No description provided for @settingsDataSnapshotImportError.
  ///
  /// In en, this message translates to:
  /// **'Snapshot restore failed: {error}'**
  String settingsDataSnapshotImportError(String error);

  /// No description provided for @settingsDataSnapshotRestoreConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Restore full snapshot'**
  String get settingsDataSnapshotRestoreConfirmTitle;

  /// No description provided for @settingsDataSnapshotRestoreConfirmMessage.
  ///
  /// In en, this message translates to:
  /// **'Restoring will replace the current runtime data.\nFormat version: {formatVersion}\nFiles: {fileCount}\nCreated: {createdAt}\nSnapshot size: {bytes} bytes'**
  String settingsDataSnapshotRestoreConfirmMessage(
    int formatVersion,
    int fileCount,
    String createdAt,
    int bytes,
  );

  /// No description provided for @settingsDataSnapshotRestoreConfirmAction.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get settingsDataSnapshotRestoreConfirmAction;

  /// No description provided for @settingsDataImportOperit1Snapshot.
  ///
  /// In en, this message translates to:
  /// **'Import from Operit1'**
  String get settingsDataImportOperit1Snapshot;

  /// No description provided for @settingsDataOperit1SnapshotImported.
  ///
  /// In en, this message translates to:
  /// **'Operit1 snapshot imported.'**
  String get settingsDataOperit1SnapshotImported;

  /// No description provided for @settingsDataOperit1SnapshotImportError.
  ///
  /// In en, this message translates to:
  /// **'Operit1 snapshot import failed: {error}'**
  String settingsDataOperit1SnapshotImportError(String error);

  /// No description provided for @settingsDataOperit1SnapshotImportConfirmMessage.
  ///
  /// In en, this message translates to:
  /// **'Import this Operit1 snapshot into the current Runtime.\nFile: {fileName}\nFormat version: {formatVersion}\nChat model: {chatModelId}\nChats: {chatCount}; messages: {messageCount}\nResource files: {fileCount}\nSnapshot size: {byteCount} bytes'**
  String settingsDataOperit1SnapshotImportConfirmMessage(
    String fileName,
    int formatVersion,
    String chatModelId,
    int chatCount,
    int messageCount,
    int fileCount,
    int byteCount,
  );

  /// No description provided for @settingsDataOperit1SnapshotImportAction.
  ///
  /// In en, this message translates to:
  /// **'Import'**
  String get settingsDataOperit1SnapshotImportAction;

  /// No description provided for @settingsDataAdvancedBackupOptions.
  ///
  /// In en, this message translates to:
  /// **'Advanced options'**
  String get settingsDataAdvancedBackupOptions;

  /// No description provided for @settingsDataAdvancedBackupOptionsDescription.
  ///
  /// In en, this message translates to:
  /// **'Single-item JSON export and restore'**
  String get settingsDataAdvancedBackupOptionsDescription;

  /// No description provided for @settingsRuntimeNetworkControl.
  ///
  /// In en, this message translates to:
  /// **'Network control'**
  String get settingsRuntimeNetworkControl;

  /// No description provided for @settingsRuntimeNetworkControlDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage device identities and permissions, and review activity'**
  String get settingsRuntimeNetworkControlDescription;

  /// No description provided for @settingsRuntimeIdentitiesAndPermissions.
  ///
  /// In en, this message translates to:
  /// **'Identities & permissions'**
  String get settingsRuntimeIdentitiesAndPermissions;

  /// No description provided for @settingsRuntimeControlInitialize.
  ///
  /// In en, this message translates to:
  /// **'Initialize control'**
  String get settingsRuntimeControlInitialize;

  /// No description provided for @settingsRuntimeControlRoles.
  ///
  /// In en, this message translates to:
  /// **'Roles'**
  String get settingsRuntimeControlRoles;

  /// No description provided for @settingsRuntimeControlIdentities.
  ///
  /// In en, this message translates to:
  /// **'Identities'**
  String get settingsRuntimeControlIdentities;

  /// No description provided for @settingsRuntimeControlAuditAccepted.
  ///
  /// In en, this message translates to:
  /// **'Applied'**
  String get settingsRuntimeControlAuditAccepted;

  /// No description provided for @settingsRuntimeControlAuditRejected.
  ///
  /// In en, this message translates to:
  /// **'Rejected'**
  String get settingsRuntimeControlAuditRejected;

  /// No description provided for @settingsRuntimeControlAuditIssuer.
  ///
  /// In en, this message translates to:
  /// **'Requested by'**
  String get settingsRuntimeControlAuditIssuer;

  /// No description provided for @settingsRuntimeControlAuditDetails.
  ///
  /// In en, this message translates to:
  /// **'Technical details'**
  String get settingsRuntimeControlAuditDetails;

  /// No description provided for @settingsRuntimeControlAuditBootstrap.
  ///
  /// In en, this message translates to:
  /// **'Initialize space permissions'**
  String get settingsRuntimeControlAuditBootstrap;

  /// No description provided for @settingsRuntimeControlAuditAdmit.
  ///
  /// In en, this message translates to:
  /// **'Admit device'**
  String get settingsRuntimeControlAuditAdmit;

  /// No description provided for @settingsRuntimeControlAuditMerge.
  ///
  /// In en, this message translates to:
  /// **'Merge device spaces'**
  String get settingsRuntimeControlAuditMerge;

  /// No description provided for @settingsRuntimeControlAuditPolicy.
  ///
  /// In en, this message translates to:
  /// **'Update permission policy'**
  String get settingsRuntimeControlAuditPolicy;

  /// No description provided for @settingsRuntimeControlAudit.
  ///
  /// In en, this message translates to:
  /// **'Activity'**
  String get settingsRuntimeControlAudit;

  /// No description provided for @settingsRuntimeControlAddRole.
  ///
  /// In en, this message translates to:
  /// **'New identity'**
  String get settingsRuntimeControlAddRole;

  /// No description provided for @settingsRuntimeControlGrantRole.
  ///
  /// In en, this message translates to:
  /// **'Grant role'**
  String get settingsRuntimeControlGrantRole;

  /// No description provided for @settingsRuntimeControlAddIdentity.
  ///
  /// In en, this message translates to:
  /// **'Add identity'**
  String get settingsRuntimeControlAddIdentity;

  /// No description provided for @settingsRuntimeControlBindDevice.
  ///
  /// In en, this message translates to:
  /// **'Bind device'**
  String get settingsRuntimeControlBindDevice;

  /// No description provided for @settingsRuntimeControlRoleName.
  ///
  /// In en, this message translates to:
  /// **'Identity name'**
  String get settingsRuntimeControlRoleName;

  /// No description provided for @settingsRuntimeControlCapabilities.
  ///
  /// In en, this message translates to:
  /// **'Actions this identity can perform'**
  String get settingsRuntimeControlCapabilities;

  /// No description provided for @settingsRuntimeControlManagedDevices.
  ///
  /// In en, this message translates to:
  /// **'Devices this role can manage (empty = all)'**
  String get settingsRuntimeControlManagedDevices;

  /// No description provided for @settingsRuntimeControlIdentityName.
  ///
  /// In en, this message translates to:
  /// **'Identity name'**
  String get settingsRuntimeControlIdentityName;

  /// No description provided for @settingsRuntimeControlDeviceId.
  ///
  /// In en, this message translates to:
  /// **'Device ID'**
  String get settingsRuntimeControlDeviceId;

  /// No description provided for @settingsRuntimeControlNoAudit.
  ///
  /// In en, this message translates to:
  /// **'No activity yet. Identity and device changes will appear here.'**
  String get settingsRuntimeControlNoAudit;

  /// No description provided for @settingsRuntimeControlAuditCount.
  ///
  /// In en, this message translates to:
  /// **'{count} records'**
  String settingsRuntimeControlAuditCount(int count);

  /// No description provided for @settingsRuntimeControlNoBindings.
  ///
  /// In en, this message translates to:
  /// **'No device bindings'**
  String get settingsRuntimeControlNoBindings;

  /// No description provided for @settingsRuntimeControlGranted.
  ///
  /// In en, this message translates to:
  /// **'Granted'**
  String get settingsRuntimeControlGranted;

  /// No description provided for @settingsRuntimeControlRevoked.
  ///
  /// In en, this message translates to:
  /// **'Revoked'**
  String get settingsRuntimeControlRevoked;

  /// No description provided for @settingsRuntimeControlRemoveDevice.
  ///
  /// In en, this message translates to:
  /// **'Remove device'**
  String get settingsRuntimeControlRemoveDevice;

  /// No description provided for @settingsRuntimeControlDisconnectDevice.
  ///
  /// In en, this message translates to:
  /// **'Isolate device'**
  String get settingsRuntimeControlDisconnectDevice;

  /// No description provided for @settingsRuntimeControlRemoved.
  ///
  /// In en, this message translates to:
  /// **'Removed'**
  String get settingsRuntimeControlRemoved;

  /// No description provided for @settingsRuntimeControlDisconnected.
  ///
  /// In en, this message translates to:
  /// **'Isolated'**
  String get settingsRuntimeControlDisconnected;

  /// No description provided for @settingsRuntimeControlAdmitDevice.
  ///
  /// In en, this message translates to:
  /// **'Lift isolation'**
  String get settingsRuntimeControlAdmitDevice;

  /// No description provided for @settingsRuntimeControlAllDevices.
  ///
  /// In en, this message translates to:
  /// **'All devices'**
  String get settingsRuntimeControlAllDevices;

  /// No description provided for @settingsRuntimeControlManages.
  ///
  /// In en, this message translates to:
  /// **'Manages'**
  String get settingsRuntimeControlManages;

  /// No description provided for @settingsRuntimeControlLocalIdentity.
  ///
  /// In en, this message translates to:
  /// **'This device'**
  String get settingsRuntimeControlLocalIdentity;

  /// No description provided for @settingsRuntimeControlCurrentIdentity.
  ///
  /// In en, this message translates to:
  /// **'Current identity'**
  String get settingsRuntimeControlCurrentIdentity;

  /// No description provided for @settingsRuntimeControlCurrentCapabilities.
  ///
  /// In en, this message translates to:
  /// **'Permissions'**
  String get settingsRuntimeControlCurrentCapabilities;

  /// No description provided for @settingsRuntimeControlManageIdentities.
  ///
  /// In en, this message translates to:
  /// **'Manage identities'**
  String get settingsRuntimeControlManageIdentities;

  /// No description provided for @settingsRuntimeControlAssignIdentity.
  ///
  /// In en, this message translates to:
  /// **'Set device identity'**
  String get settingsRuntimeControlAssignIdentity;

  /// No description provided for @settingsRuntimeControlClearIdentity.
  ///
  /// In en, this message translates to:
  /// **'Reset to default identity'**
  String get settingsRuntimeControlClearIdentity;

  /// No description provided for @settingsRuntimeControlDeviceAdmitted.
  ///
  /// In en, this message translates to:
  /// **'Isolation lifted; waiting for the device to reconnect'**
  String get settingsRuntimeControlDeviceAdmitted;

  /// No description provided for @settingsRuntimeControlIdentityResetDone.
  ///
  /// In en, this message translates to:
  /// **'Identity reset to the default user'**
  String get settingsRuntimeControlIdentityResetDone;

  /// No description provided for @settingsRuntimeControlIdentityDefinitions.
  ///
  /// In en, this message translates to:
  /// **'Identity permissions'**
  String get settingsRuntimeControlIdentityDefinitions;

  /// No description provided for @settingsRuntimeControlIdentityCount.
  ///
  /// In en, this message translates to:
  /// **'{count} identities'**
  String settingsRuntimeControlIdentityCount(int count);

  /// No description provided for @settingsRuntimeControlIdentityAssignment.
  ///
  /// In en, this message translates to:
  /// **'Identity assignment'**
  String get settingsRuntimeControlIdentityAssignment;

  /// No description provided for @settingsRuntimeControlNoIdentity.
  ///
  /// In en, this message translates to:
  /// **'No identity assigned'**
  String get settingsRuntimeControlNoIdentity;

  /// No description provided for @settingsRuntimeControlOnline.
  ///
  /// In en, this message translates to:
  /// **'Online'**
  String get settingsRuntimeControlOnline;

  /// No description provided for @settingsRuntimeControlOffline.
  ///
  /// In en, this message translates to:
  /// **'Offline'**
  String get settingsRuntimeControlOffline;

  /// No description provided for @settingsRuntimeControlCapabilityAll.
  ///
  /// In en, this message translates to:
  /// **'Everything'**
  String get settingsRuntimeControlCapabilityAll;

  /// No description provided for @settingsRuntimeControlCapabilityAuditRead.
  ///
  /// In en, this message translates to:
  /// **'View audit history'**
  String get settingsRuntimeControlCapabilityAuditRead;

  /// No description provided for @settingsRuntimeControlCapabilityNetworkRelay.
  ///
  /// In en, this message translates to:
  /// **'Relay network traffic'**
  String get settingsRuntimeControlCapabilityNetworkRelay;

  /// No description provided for @settingsRuntimeControlCapabilityStorageProvide.
  ///
  /// In en, this message translates to:
  /// **'Store shared data replicas'**
  String get settingsRuntimeControlCapabilityStorageProvide;

  /// No description provided for @settingsRuntimeControlCapabilityRuntimeExecute.
  ///
  /// In en, this message translates to:
  /// **'Host Core execution'**
  String get settingsRuntimeControlCapabilityRuntimeExecute;

  /// No description provided for @settingsRuntimeControlCapabilityViewDevices.
  ///
  /// In en, this message translates to:
  /// **'View devices and current identities'**
  String get settingsRuntimeControlCapabilityViewDevices;

  /// No description provided for @settingsRuntimeControlCapabilityApproval.
  ///
  /// In en, this message translates to:
  /// **'Approve authorizations'**
  String get settingsRuntimeControlCapabilityApproval;

  /// No description provided for @chatRoutePermissionDenied.
  ///
  /// In en, this message translates to:
  /// **'Permission denied: target device {targetNodeId} lacks the {capability} capability.'**
  String chatRoutePermissionDenied(Object targetNodeId, Object capability);

  /// No description provided for @settingsRuntimeControlRoleAdministrator.
  ///
  /// In en, this message translates to:
  /// **'Administrator'**
  String get settingsRuntimeControlRoleAdministrator;

  /// No description provided for @settingsRuntimeControlRoleRelay.
  ///
  /// In en, this message translates to:
  /// **'Relay node'**
  String get settingsRuntimeControlRoleRelay;

  /// No description provided for @settingsRuntimeControlRoleEdge.
  ///
  /// In en, this message translates to:
  /// **'Edge node'**
  String get settingsRuntimeControlRoleEdge;

  /// No description provided for @settingsRuntimeControlRoleUser.
  ///
  /// In en, this message translates to:
  /// **'User'**
  String get settingsRuntimeControlRoleUser;

  /// No description provided for @pluginLoadingTitle.
  ///
  /// In en, this message translates to:
  /// **'Loading'**
  String get pluginLoadingTitle;

  /// No description provided for @pluginLoadingSkip.
  ///
  /// In en, this message translates to:
  /// **'Skip'**
  String get pluginLoadingSkip;

  /// No description provided for @pluginLoadingCollapse.
  ///
  /// In en, this message translates to:
  /// **'Collapse'**
  String get pluginLoadingCollapse;

  /// No description provided for @pluginLoadingStatus.
  ///
  /// In en, this message translates to:
  /// **'Started: {started} / {total}'**
  String pluginLoadingStatus(int started, int total);

  /// No description provided for @pluginLoadingCurrentTask.
  ///
  /// In en, this message translates to:
  /// **'Loading {task}'**
  String pluginLoadingCurrentTask(String task);

  /// No description provided for @pluginLoadingSuccess.
  ///
  /// In en, this message translates to:
  /// **'Loaded'**
  String get pluginLoadingSuccess;

  /// No description provided for @pluginLoadingFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get pluginLoadingFailed;

  /// No description provided for @pluginLoadingWaiting.
  ///
  /// In en, this message translates to:
  /// **'Waiting'**
  String get pluginLoadingWaiting;

  /// No description provided for @pluginLoadingKindPackage.
  ///
  /// In en, this message translates to:
  /// **'Package'**
  String get pluginLoadingKindPackage;

  /// No description provided for @pluginLoadingKindPlugin.
  ///
  /// In en, this message translates to:
  /// **'Plugin'**
  String get pluginLoadingKindPlugin;

  /// No description provided for @pluginLoadingKindMcp.
  ///
  /// In en, this message translates to:
  /// **'MCP'**
  String get pluginLoadingKindMcp;

  /// No description provided for @pluginLoadingCompleteSuccess.
  ///
  /// In en, this message translates to:
  /// **'All packages and plugins loaded'**
  String get pluginLoadingCompleteSuccess;

  /// No description provided for @pluginLoadingCompleteWithFailures.
  ///
  /// In en, this message translates to:
  /// **'Some packages or plugins failed to load'**
  String get pluginLoadingCompleteWithFailures;

  /// No description provided for @pluginLoadingNoLogs.
  ///
  /// In en, this message translates to:
  /// **'No logs'**
  String get pluginLoadingNoLogs;

  /// No description provided for @chatRouteLocal.
  ///
  /// In en, this message translates to:
  /// **'Local'**
  String get chatRouteLocal;

  /// No description provided for @chatRouteRemote.
  ///
  /// In en, this message translates to:
  /// **'Remote {platform}'**
  String chatRouteRemote(String platform);

  /// No description provided for @chatRouteRemoteOffline.
  ///
  /// In en, this message translates to:
  /// **'Remote {platform} offline'**
  String chatRouteRemoteOffline(String platform);

  /// No description provided for @chatRouteLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading execution device'**
  String get chatRouteLoading;

  /// No description provided for @chatRouteUnknown.
  ///
  /// In en, this message translates to:
  /// **'Execution device status unknown'**
  String get chatRouteUnknown;

  /// No description provided for @chatRouteClaimHostPrompt.
  ///
  /// In en, this message translates to:
  /// **'The device host for this chat was not found. Switch it to this device?'**
  String get chatRouteClaimHostPrompt;

  /// No description provided for @chatRouteClaimHostYes.
  ///
  /// In en, this message translates to:
  /// **'Yes'**
  String get chatRouteClaimHostYes;

  /// No description provided for @chatRouteClaimHostNo.
  ///
  /// In en, this message translates to:
  /// **'No'**
  String get chatRouteClaimHostNo;

  /// No description provided for @edgePairingCodeTitle.
  ///
  /// In en, this message translates to:
  /// **'Enter the pairing code for {deviceName}'**
  String edgePairingCodeTitle(String deviceName);

  /// No description provided for @edgePairingCodeHint.
  ///
  /// In en, this message translates to:
  /// **'Read the six-digit code shown by the device.'**
  String get edgePairingCodeHint;

  /// No description provided for @chatMessageResponse.
  ///
  /// In en, this message translates to:
  /// **'Response'**
  String get chatMessageResponse;

  /// No description provided for @chatMessagePrompt.
  ///
  /// In en, this message translates to:
  /// **'Prompt'**
  String get chatMessagePrompt;

  /// No description provided for @chatMessagePromptBy.
  ///
  /// In en, this message translates to:
  /// **'Prompt by {sender}'**
  String chatMessagePromptBy(String sender);

  /// No description provided for @chatMessageTokensTooltip.
  ///
  /// In en, this message translates to:
  /// **'Tokens: ↑{input} · ↓{output}'**
  String chatMessageTokensTooltip(String input, String output);

  /// No description provided for @chatMessageTokensCachedTooltip.
  ///
  /// In en, this message translates to:
  /// **'Tokens: ↑{input} (cached: {cached}) · ↓{output}'**
  String chatMessageTokensCachedTooltip(
    String input,
    String cached,
    String output,
  );

  /// No description provided for @chatMessageTimingTooltip.
  ///
  /// In en, this message translates to:
  /// **'Timing: {wait} wait · {output} output'**
  String chatMessageTimingTooltip(String wait, String output);

  /// No description provided for @chatMessageTimeTooltip.
  ///
  /// In en, this message translates to:
  /// **'Time: {time}'**
  String chatMessageTimeTooltip(String time);

  /// No description provided for @chatMessageInfoDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Message Details'**
  String get chatMessageInfoDialogTitle;

  /// No description provided for @chatMessageDeleteConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Confirm delete'**
  String get chatMessageDeleteConfirmTitle;

  /// No description provided for @chatMessageDeleteConfirmMessage.
  ///
  /// In en, this message translates to:
  /// **'Delete this message?'**
  String get chatMessageDeleteConfirmMessage;

  /// No description provided for @chatMessageSender.
  ///
  /// In en, this message translates to:
  /// **'Sender: {sender}'**
  String chatMessageSender(String sender);

  /// No description provided for @chatMessageTimestamp.
  ///
  /// In en, this message translates to:
  /// **'Timestamp: {timestamp}'**
  String chatMessageTimestamp(String timestamp);

  /// No description provided for @chatMessageRole.
  ///
  /// In en, this message translates to:
  /// **'Responder: {roleName}'**
  String chatMessageRole(String roleName);

  /// No description provided for @chatMessageModel.
  ///
  /// In en, this message translates to:
  /// **'Model: {modelName}'**
  String chatMessageModel(String modelName);

  /// No description provided for @chatMessageProvider.
  ///
  /// In en, this message translates to:
  /// **'Provider: {provider}'**
  String chatMessageProvider(String provider);

  /// No description provided for @chatMessageTokensInput.
  ///
  /// In en, this message translates to:
  /// **'Input tokens: {count}'**
  String chatMessageTokensInput(String count);

  /// No description provided for @chatMessageTokensCached.
  ///
  /// In en, this message translates to:
  /// **'Cached input tokens: {count}'**
  String chatMessageTokensCached(String count);

  /// No description provided for @chatMessageTokensOutput.
  ///
  /// In en, this message translates to:
  /// **'Output tokens: {count}'**
  String chatMessageTokensOutput(String count);

  /// No description provided for @chatMessageWaitDuration.
  ///
  /// In en, this message translates to:
  /// **'Wait duration: {duration}'**
  String chatMessageWaitDuration(String duration);

  /// No description provided for @chatMessageOutputDuration.
  ///
  /// In en, this message translates to:
  /// **'Output duration: {duration}'**
  String chatMessageOutputDuration(String duration);

  /// No description provided for @chatMessageTokenSpeed.
  ///
  /// In en, this message translates to:
  /// **'Token speed: {speed}'**
  String chatMessageTokenSpeed(String speed);

  /// No description provided for @chatMessageCacheShort.
  ///
  /// In en, this message translates to:
  /// **'cache {count}'**
  String chatMessageCacheShort(String count);

  /// No description provided for @chatMessageCacheHitRate.
  ///
  /// In en, this message translates to:
  /// **'Cache hit rate: {rate}'**
  String chatMessageCacheHitRate(String rate);

  /// No description provided for @settingsPeerPortModeHelp.
  ///
  /// In en, this message translates to:
  /// **'By default, the system assigns an available port and LAN discovery advertises it. Fixed mode uses the entered listen address and port and reports an error if the port is busy.'**
  String get settingsPeerPortModeHelp;

  /// No description provided for @settingsPeerPortMode.
  ///
  /// In en, this message translates to:
  /// **'Port allocation'**
  String get settingsPeerPortMode;

  /// No description provided for @settingsPeerPortModeFixed.
  ///
  /// In en, this message translates to:
  /// **'Fixed listen address and port'**
  String get settingsPeerPortModeFixed;

  /// No description provided for @settingsPeerPortModeAutomatic.
  ///
  /// In en, this message translates to:
  /// **'Automatic port'**
  String get settingsPeerPortModeAutomatic;

  /// No description provided for @settingsPeerNetworkDiscoveryHelp.
  ///
  /// In en, this message translates to:
  /// **'For discovery across routed networks, configure a shared directory and group key. To host the directory, leave the address empty and set a group key.'**
  String get settingsPeerNetworkDiscoveryHelp;

  /// No description provided for @settingsPeerDiscoveryDirectory.
  ///
  /// In en, this message translates to:
  /// **'Discovery directory (optional)'**
  String get settingsPeerDiscoveryDirectory;

  /// No description provided for @settingsPeerDiscoveryGroup.
  ///
  /// In en, this message translates to:
  /// **'Discovery group key'**
  String get settingsPeerDiscoveryGroup;

  /// No description provided for @settingsPeerDiscoveryGenerateKey.
  ///
  /// In en, this message translates to:
  /// **'Generate new group key'**
  String get settingsPeerDiscoveryGenerateKey;

  /// No description provided for @settingsPeerDiscoveryCopyKey.
  ///
  /// In en, this message translates to:
  /// **'Copy group key'**
  String get settingsPeerDiscoveryCopyKey;

  /// No description provided for @settingsPeerDiscoveryPort.
  ///
  /// In en, this message translates to:
  /// **'Directory listener port'**
  String get settingsPeerDiscoveryPort;

  /// No description provided for @settingsPeerDiscoveryPortHelp.
  ///
  /// In en, this message translates to:
  /// **'Only used on the directory host. Other devices use the port in the directory URL.'**
  String get settingsPeerDiscoveryPortHelp;

  /// No description provided for @settingsPeerNetworkDiscoveryInvalid.
  ///
  /// In en, this message translates to:
  /// **'Check the discovery settings: use an HTTP(S) origin without a path, a shared key of 16–128 bytes, and a nonzero port when hosting the directory.'**
  String get settingsPeerNetworkDiscoveryInvalid;

  /// No description provided for @settingsPeerAdvanced.
  ///
  /// In en, this message translates to:
  /// **'Advanced options'**
  String get settingsPeerAdvanced;

  /// No description provided for @settingsPeerDiscoveryUnavailable.
  ///
  /// In en, this message translates to:
  /// **'This Host cannot advertise listeners on the local network. Direct connections remain available for supported transports.'**
  String get settingsPeerDiscoveryUnavailable;

  /// No description provided for @settingsPeerTransportUnavailable.
  ///
  /// In en, this message translates to:
  /// **'{transport} listening is not supported by this Host.'**
  String settingsPeerTransportUnavailable(String transport);

  /// No description provided for @settingsPeerTransportHelp.
  ///
  /// In en, this message translates to:
  /// **'Listener transports (multiple selections). HTTP, WebSocket and TCP share one listening port and can be combined freely. Bluetooth requires platform support. Clear all selections and disable discovery to stop listening. Changing settings interrupts existing connections. Disabling discovery alone does not stop direct connections.'**
  String get settingsPeerTransportHelp;

  /// No description provided for @settingsPeerBindAddress.
  ///
  /// In en, this message translates to:
  /// **'Listen IP address'**
  String get settingsPeerBindAddress;

  /// No description provided for @settingsPeerBindAddressHelp.
  ///
  /// In en, this message translates to:
  /// **'Example: 0.0.0.0 for LAN access; IPv6 addresses are also supported.'**
  String get settingsPeerBindAddressHelp;

  /// No description provided for @settingsPeerBindPort.
  ///
  /// In en, this message translates to:
  /// **'Listen port'**
  String get settingsPeerBindPort;

  /// No description provided for @settingsPeerBindPortHelp.
  ///
  /// In en, this message translates to:
  /// **'The port used by fixed listener mode.'**
  String get settingsPeerBindPortHelp;

  /// No description provided for @settingsPeerBindPortAutomaticHelp.
  ///
  /// In en, this message translates to:
  /// **'The system assigns an available port. The actual port appears after startup.'**
  String get settingsPeerBindPortAutomaticHelp;

  /// No description provided for @settingsPeerDiscoveryNeedsTransport.
  ///
  /// In en, this message translates to:
  /// **'Select at least one transport before enabling discovery.'**
  String get settingsPeerDiscoveryNeedsTransport;

  /// No description provided for @settingsPeerAddressRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter a listen IP address.'**
  String get settingsPeerAddressRequired;

  /// No description provided for @settingsPeerPortRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter a port from 1 to 65535.'**
  String get settingsPeerPortRequired;

  /// No description provided for @settingsPeerApplied.
  ///
  /// In en, this message translates to:
  /// **'Listener settings applied.'**
  String get settingsPeerApplied;

  /// No description provided for @settingsPeerRestoreFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to restore previous listeners: {error}'**
  String settingsPeerRestoreFailed(String error);

  /// No description provided for @settingsPeerCodeInstructions.
  ///
  /// In en, this message translates to:
  /// **'Enter this code on the other device to finish pairing. It expires in five minutes. Do not share it with strangers.'**
  String get settingsPeerCodeInstructions;

  /// No description provided for @settingsPeerSixDigitCode.
  ///
  /// In en, this message translates to:
  /// **'Enter the six-digit pairing code shown on the receiving device.'**
  String get settingsPeerSixDigitCode;

  /// No description provided for @spaceJoinRequests.
  ///
  /// In en, this message translates to:
  /// **'Space join requests'**
  String get spaceJoinRequests;

  /// No description provided for @spaceJoinReviewer.
  ///
  /// In en, this message translates to:
  /// **'Reviewer: {name}'**
  String spaceJoinReviewer(String name);

  /// No description provided for @spaceJoinWaitingReviewer.
  ///
  /// In en, this message translates to:
  /// **'Waiting for an authorized reviewer to come online'**
  String get spaceJoinWaitingReviewer;

  /// No description provided for @spaceJoinPending.
  ///
  /// In en, this message translates to:
  /// **'Waiting for approval'**
  String get spaceJoinPending;

  /// No description provided for @spaceJoinApproving.
  ///
  /// In en, this message translates to:
  /// **'Decision is being processed'**
  String get spaceJoinApproving;

  /// No description provided for @spaceJoinApproved.
  ///
  /// In en, this message translates to:
  /// **'Approved · completing join'**
  String get spaceJoinApproved;

  /// No description provided for @spaceJoinJoined.
  ///
  /// In en, this message translates to:
  /// **'Joined'**
  String get spaceJoinJoined;

  /// No description provided for @spaceJoinRejected.
  ///
  /// In en, this message translates to:
  /// **'Rejected'**
  String get spaceJoinRejected;

  /// No description provided for @spaceJoinCancelled.
  ///
  /// In en, this message translates to:
  /// **'Cancelled'**
  String get spaceJoinCancelled;

  /// No description provided for @spaceJoinExpired.
  ///
  /// In en, this message translates to:
  /// **'Expired'**
  String get spaceJoinExpired;

  /// No description provided for @spaceJoinSubmit.
  ///
  /// In en, this message translates to:
  /// **'Submit request'**
  String get spaceJoinSubmit;

  /// No description provided for @spaceJoinApprovalTitle.
  ///
  /// In en, this message translates to:
  /// **'Space join request'**
  String get spaceJoinApprovalTitle;

  /// No description provided for @spaceJoinApprovalDescription.
  ///
  /// In en, this message translates to:
  /// **'{name} wants to join “{space}”. No Space data is shared before approval.'**
  String spaceJoinApprovalDescription(String name, String space);

  /// No description provided for @spaceJoinApprove.
  ///
  /// In en, this message translates to:
  /// **'Approve'**
  String get spaceJoinApprove;

  /// No description provided for @spaceJoinReject.
  ///
  /// In en, this message translates to:
  /// **'Reject'**
  String get spaceJoinReject;

  /// No description provided for @spaceJoinRetry.
  ///
  /// In en, this message translates to:
  /// **'Submit again'**
  String get spaceJoinRetry;

  /// No description provided for @spaceJoinKeepWaiting.
  ///
  /// In en, this message translates to:
  /// **'Keep waiting in background'**
  String get spaceJoinKeepWaiting;

  /// No description provided for @spaceJoinCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel request'**
  String get spaceJoinCancel;

  /// No description provided for @spaceJoinCancelFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not confirm cancellation. Check the connection or try again later.'**
  String get spaceJoinCancelFailed;

  /// No description provided for @spaceJoinRefreshingFailed.
  ///
  /// In en, this message translates to:
  /// **'Cannot reach the device right now. The request is saved and will retry.'**
  String get spaceJoinRefreshingFailed;

  /// No description provided for @spaceJoinAssignmentChanged.
  ///
  /// In en, this message translates to:
  /// **'This request has been transferred or already processed.'**
  String get spaceJoinAssignmentChanged;

  /// No description provided for @spaceJoinProgressTitle.
  ///
  /// In en, this message translates to:
  /// **'Space join request'**
  String get spaceJoinProgressTitle;

  /// No description provided for @spaceJoinSending.
  ///
  /// In en, this message translates to:
  /// **'Submitting request…'**
  String get spaceJoinSending;

  /// No description provided for @spaceJoinSubmitFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not submit the request. Check the connection and try again.'**
  String get spaceJoinSubmitFailed;

  /// No description provided for @spaceJoinNoDataYet.
  ///
  /// In en, this message translates to:
  /// **'Your current Space remains unchanged until approval. You can close this window and keep waiting in the background.'**
  String get spaceJoinNoDataYet;

  /// No description provided for @deviceSpaceAddDevice.
  ///
  /// In en, this message translates to:
  /// **'Add device'**
  String get deviceSpaceAddDevice;

  /// No description provided for @deviceSpaceDevices.
  ///
  /// In en, this message translates to:
  /// **'Devices'**
  String get deviceSpaceDevices;

  /// No description provided for @deviceSpaceMemberNotPaired.
  ///
  /// In en, this message translates to:
  /// **'Space member · not paired'**
  String get deviceSpaceMemberNotPaired;

  /// No description provided for @deviceSpaceMemberPaired.
  ///
  /// In en, this message translates to:
  /// **'Space member · paired'**
  String get deviceSpaceMemberPaired;

  /// No description provided for @deviceSpacePairedOnly.
  ///
  /// In en, this message translates to:
  /// **'Paired'**
  String get deviceSpacePairedOnly;

  /// No description provided for @deviceSpaceDirectLink.
  ///
  /// In en, this message translates to:
  /// **'Direct'**
  String get deviceSpaceDirectLink;

  /// No description provided for @deviceSpaceRelayHops.
  ///
  /// In en, this message translates to:
  /// **'Relayed · {hops} hops'**
  String deviceSpaceRelayHops(int hops);

  /// No description provided for @deviceSpaceRelayPath.
  ///
  /// In en, this message translates to:
  /// **'Path'**
  String get deviceSpaceRelayPath;

  /// No description provided for @deviceSpaceLocalDevice.
  ///
  /// In en, this message translates to:
  /// **'This device'**
  String get deviceSpaceLocalDevice;

  /// No description provided for @deviceSpaceManageDevice.
  ///
  /// In en, this message translates to:
  /// **'Manage device'**
  String get deviceSpaceManageDevice;

  /// No description provided for @deviceSpaceConnectDirect.
  ///
  /// In en, this message translates to:
  /// **'Connect directly'**
  String get deviceSpaceConnectDirect;

  /// No description provided for @deviceSpaceConnectDirectHint.
  ///
  /// In en, this message translates to:
  /// **'This device is already a Space member. Pairing it adds a direct link without joining again; pairing shows a six-digit code on the other device for you to enter.'**
  String get deviceSpaceConnectDirectHint;

  /// No description provided for @deviceSpaceUpdateEndpoint.
  ///
  /// In en, this message translates to:
  /// **'Update direct address'**
  String get deviceSpaceUpdateEndpoint;

  /// No description provided for @deviceSpaceUpdateEndpointLabel.
  ///
  /// In en, this message translates to:
  /// **'Direct address'**
  String get deviceSpaceUpdateEndpointLabel;

  /// No description provided for @deviceSpaceUpdateEndpointHint.
  ///
  /// In en, this message translates to:
  /// **'e.g. ws://host:port/link or host:port — use the peer\'s current listen port'**
  String get deviceSpaceUpdateEndpointHint;

  /// No description provided for @deviceSpaceUpdateEndpointSaved.
  ///
  /// In en, this message translates to:
  /// **'Direct address updated'**
  String get deviceSpaceUpdateEndpointSaved;

  /// No description provided for @deviceSpaceDirectLinkEstablished.
  ///
  /// In en, this message translates to:
  /// **'Direct link established: {device}'**
  String deviceSpaceDirectLinkEstablished(String device);

  /// No description provided for @deviceSpaceUnpair.
  ///
  /// In en, this message translates to:
  /// **'Unpair device'**
  String get deviceSpaceUnpair;

  /// No description provided for @deviceSpaceMore.
  ///
  /// In en, this message translates to:
  /// **'More device actions'**
  String get deviceSpaceMore;

  /// No description provided for @deviceSpaceConnectionSettings.
  ///
  /// In en, this message translates to:
  /// **'Connection settings'**
  String get deviceSpaceConnectionSettings;

  /// No description provided for @deviceSpaceViewRequest.
  ///
  /// In en, this message translates to:
  /// **'View request'**
  String get deviceSpaceViewRequest;

  /// No description provided for @devicePickerNearby.
  ///
  /// In en, this message translates to:
  /// **'Nearby devices'**
  String get devicePickerNearby;

  /// No description provided for @devicePickerDiscoveryFailed.
  ///
  /// In en, this message translates to:
  /// **'Nearby device discovery failed: {error}'**
  String devicePickerDiscoveryFailed(String error);

  /// No description provided for @devicePickerEmpty.
  ///
  /// In en, this message translates to:
  /// **'No nearby devices found'**
  String get devicePickerEmpty;

  /// No description provided for @devicePickerHint.
  ///
  /// In en, this message translates to:
  /// **'Connect both devices to the same network and enable discovery on the other device.'**
  String get devicePickerHint;

  /// No description provided for @devicePickerManual.
  ///
  /// In en, this message translates to:
  /// **'Connect using an address'**
  String get devicePickerManual;

  /// No description provided for @deviceSpaceMyRequests.
  ///
  /// In en, this message translates to:
  /// **'My requests'**
  String get deviceSpaceMyRequests;

  /// No description provided for @deviceSpaceMyReviews.
  ///
  /// In en, this message translates to:
  /// **'Awaiting my review'**
  String get deviceSpaceMyReviews;

  /// No description provided for @deviceSpaceNoRequests.
  ///
  /// In en, this message translates to:
  /// **'No requests'**
  String get deviceSpaceNoRequests;

  /// No description provided for @workspaceUndo.
  ///
  /// In en, this message translates to:
  /// **'Undo'**
  String get workspaceUndo;

  /// No description provided for @workspaceRedo.
  ///
  /// In en, this message translates to:
  /// **'Redo'**
  String get workspaceRedo;

  /// No description provided for @workspaceInsertSymbol.
  ///
  /// In en, this message translates to:
  /// **'Insert {symbol}'**
  String workspaceInsertSymbol(String symbol);

  /// No description provided for @settingsAppearanceThemeConfig.
  ///
  /// In en, this message translates to:
  /// **'Theme configuration'**
  String get settingsAppearanceThemeConfig;

  /// No description provided for @settingsAppearanceThemeConfigCustom.
  ///
  /// In en, this message translates to:
  /// **'Unbound appearance'**
  String get settingsAppearanceThemeConfigCustom;

  /// No description provided for @settingsAppearanceThemeConfigCreate.
  ///
  /// In en, this message translates to:
  /// **'New theme configuration'**
  String get settingsAppearanceThemeConfigCreate;

  /// No description provided for @settingsAppearanceThemeConfigRename.
  ///
  /// In en, this message translates to:
  /// **'Rename theme configuration'**
  String get settingsAppearanceThemeConfigRename;

  /// No description provided for @settingsAppearanceThemeConfigDelete.
  ///
  /// In en, this message translates to:
  /// **'Delete an inactive theme configuration'**
  String get settingsAppearanceThemeConfigDelete;

  /// No description provided for @settingsAppearanceThemeConfigName.
  ///
  /// In en, this message translates to:
  /// **'Theme name'**
  String get settingsAppearanceThemeConfigName;

  /// No description provided for @settingsAppearanceThemeConfigNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter a theme name'**
  String get settingsAppearanceThemeConfigNameRequired;

  /// No description provided for @settingsAppearanceThemeConfigCustomHint.
  ///
  /// In en, this message translates to:
  /// **'Create a theme to save your appearance. Tap a card to switch; choose Edit to customize.'**
  String get settingsAppearanceThemeConfigCustomHint;

  /// No description provided for @settingsAppearanceThemeConfigEditingHint.
  ///
  /// In en, this message translates to:
  /// **'Tap a card to switch themes. Mode shortcuts change only the current theme. Edits save automatically.'**
  String get settingsAppearanceThemeConfigEditingHint;

  /// No description provided for @settingsAppearanceThemeConfigFailed.
  ///
  /// In en, this message translates to:
  /// **'Theme operation failed: {detail}'**
  String settingsAppearanceThemeConfigFailed(String detail);

  /// No description provided for @settingsRuntimeControlEditRole.
  ///
  /// In en, this message translates to:
  /// **'Edit role'**
  String get settingsRuntimeControlEditRole;

  /// No description provided for @settingsRuntimeControlDeleteRole.
  ///
  /// In en, this message translates to:
  /// **'Delete role'**
  String get settingsRuntimeControlDeleteRole;

  /// No description provided for @settingsRuntimeControlDeleteRoleTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete role {roleName}?'**
  String settingsRuntimeControlDeleteRoleTitle(String roleName);

  /// No description provided for @settingsRuntimeControlDeleteRoleMessage.
  ///
  /// In en, this message translates to:
  /// **'Devices holding this role fall back to the default user identity.'**
  String get settingsRuntimeControlDeleteRoleMessage;

  /// No description provided for @settingsAppearanceThemeConfigSource.
  ///
  /// In en, this message translates to:
  /// **'Create from'**
  String get settingsAppearanceThemeConfigSource;

  /// No description provided for @settingsAppearanceThemeConfigFromDefaults.
  ///
  /// In en, this message translates to:
  /// **'Default appearance'**
  String get settingsAppearanceThemeConfigFromDefaults;

  /// No description provided for @settingsAppearanceThemeConfigFromCurrent.
  ///
  /// In en, this message translates to:
  /// **'Current appearance'**
  String get settingsAppearanceThemeConfigFromCurrent;

  /// No description provided for @settingsAppearanceThemeConfigInherit.
  ///
  /// In en, this message translates to:
  /// **'Inherit: {name}'**
  String settingsAppearanceThemeConfigInherit(String name);

  /// No description provided for @settingsAppearanceThemeConfigFromTemplate.
  ///
  /// In en, this message translates to:
  /// **'Template: {name}'**
  String settingsAppearanceThemeConfigFromTemplate(String name);

  /// No description provided for @settingsAppearanceThemeConfigDefaultsHint.
  ///
  /// In en, this message translates to:
  /// **'Start with the app defaults'**
  String get settingsAppearanceThemeConfigDefaultsHint;

  /// No description provided for @settingsAppearanceThemeConfigCurrentHint.
  ///
  /// In en, this message translates to:
  /// **'Copy your current appearance'**
  String get settingsAppearanceThemeConfigCurrentHint;

  /// No description provided for @settingsAppearanceThemeConfigSavedSources.
  ///
  /// In en, this message translates to:
  /// **'Inherit a saved configuration'**
  String get settingsAppearanceThemeConfigSavedSources;

  /// No description provided for @settingsAppearanceThemeConfigPluginSources.
  ///
  /// In en, this message translates to:
  /// **'Use a plugin template'**
  String get settingsAppearanceThemeConfigPluginSources;

  /// No description provided for @settingsAppearanceThemeConfigSourceHint.
  ///
  /// In en, this message translates to:
  /// **'Templates come from installed plugins. New configurations can be edited independently and use existing appearance sync.'**
  String get settingsAppearanceThemeConfigSourceHint;

  /// No description provided for @settingsAppearanceLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Unable to load appearance'**
  String get settingsAppearanceLoadFailed;

  /// No description provided for @settingsAppearanceRepairHint.
  ///
  /// In en, this message translates to:
  /// **'Your saved appearance has not been replaced. Retry after repairing its resources, or reset appearance explicitly. Reset keeps your avatar and saved configurations and selects custom appearance.'**
  String get settingsAppearanceRepairHint;

  /// No description provided for @settingsAppearanceAssetBackgroundFailed.
  ///
  /// In en, this message translates to:
  /// **'The background image could not be loaded. Using the default background ({reference}).'**
  String settingsAppearanceAssetBackgroundFailed(String reference);

  /// No description provided for @settingsAppearanceAssetVideoFailed.
  ///
  /// In en, this message translates to:
  /// **'The background video could not be played. Using the default background ({reference}).'**
  String settingsAppearanceAssetVideoFailed(String reference);

  /// No description provided for @settingsAppearanceAssetBubbleFailed.
  ///
  /// In en, this message translates to:
  /// **'The bubble image could not be loaded. Using the default bubble color ({reference}).'**
  String settingsAppearanceAssetBubbleFailed(String reference);

  /// No description provided for @settingsAppearanceAssetPreviousRetained.
  ///
  /// In en, this message translates to:
  /// **'This theme resource could not be loaded. Keeping the previous image or media ({reference}); your other theme settings still apply.'**
  String settingsAppearanceAssetPreviousRetained(String reference);

  /// No description provided for @settingsAppearanceAssetRetry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get settingsAppearanceAssetRetry;

  /// No description provided for @settingsAppearanceAssetReimportHint.
  ///
  /// In en, this message translates to:
  /// **'If retrying does not help, choose and import a readable image or playable video again.'**
  String get settingsAppearanceAssetReimportHint;

  /// No description provided for @settingsAppearanceAssetImportFailed.
  ///
  /// In en, this message translates to:
  /// **'The background resource was not applied: {reason}. Please choose a readable image or playable video.'**
  String settingsAppearanceAssetImportFailed(String reason);

  /// No description provided for @settingsAppearanceRetry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get settingsAppearanceRetry;

  /// No description provided for @settingsAppearanceResetAfterError.
  ///
  /// In en, this message translates to:
  /// **'Reset appearance'**
  String get settingsAppearanceResetAfterError;

  /// No description provided for @backgroundWorkIdle.
  ///
  /// In en, this message translates to:
  /// **'No work is running'**
  String get backgroundWorkIdle;

  /// No description provided for @backgroundWorkRunningCount.
  ///
  /// In en, this message translates to:
  /// **'work items running'**
  String get backgroundWorkRunningCount;

  /// No description provided for @backgroundWorkWaiting.
  ///
  /// In en, this message translates to:
  /// **'Waiting for your confirmation'**
  String get backgroundWorkWaiting;

  /// No description provided for @backgroundWorkWaitingCount.
  ///
  /// In en, this message translates to:
  /// **'awaiting confirmation'**
  String get backgroundWorkWaitingCount;

  /// No description provided for @backgroundWorkCompleted.
  ///
  /// In en, this message translates to:
  /// **'Completed'**
  String get backgroundWorkCompleted;

  /// No description provided for @backgroundWorkFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed — open to view details'**
  String get backgroundWorkFailed;

  /// No description provided for @backgroundWorkCancelled.
  ///
  /// In en, this message translates to:
  /// **'Stopped'**
  String get backgroundWorkCancelled;

  /// No description provided for @backgroundWorkProcessing.
  ///
  /// In en, this message translates to:
  /// **'Preparing'**
  String get backgroundWorkProcessing;

  /// No description provided for @backgroundWorkConnecting.
  ///
  /// In en, this message translates to:
  /// **'Connecting'**
  String get backgroundWorkConnecting;

  /// No description provided for @backgroundWorkReceiving.
  ///
  /// In en, this message translates to:
  /// **'Writing a response'**
  String get backgroundWorkReceiving;

  /// No description provided for @backgroundWorkTool.
  ///
  /// In en, this message translates to:
  /// **'Running a tool'**
  String get backgroundWorkTool;

  /// No description provided for @backgroundWorkToolResult.
  ///
  /// In en, this message translates to:
  /// **'Processing tool results'**
  String get backgroundWorkToolResult;

  /// No description provided for @backgroundWorkSummarizing.
  ///
  /// In en, this message translates to:
  /// **'Organizing context'**
  String get backgroundWorkSummarizing;

  /// No description provided for @backgroundWorkPlan.
  ///
  /// In en, this message translates to:
  /// **'Executing a plan'**
  String get backgroundWorkPlan;

  /// No description provided for @backgroundWorkOpen.
  ///
  /// In en, this message translates to:
  /// **'View conversation'**
  String get backgroundWorkOpen;

  /// No description provided for @backgroundWorkReview.
  ///
  /// In en, this message translates to:
  /// **'Review'**
  String get backgroundWorkReview;

  /// No description provided for @backgroundWorkStop.
  ///
  /// In en, this message translates to:
  /// **'Stop'**
  String get backgroundWorkStop;

  /// No description provided for @localModelsDeviceOnly.
  ///
  /// In en, this message translates to:
  /// **'Stored on this device'**
  String get localModelsDeviceOnly;

  /// No description provided for @localModelsDeviceOnlyDescription.
  ///
  /// In en, this message translates to:
  /// **'Weights, download progress and installation records stay on this device and are excluded from Space sync. Chats can route inference to the device with the model.'**
  String get localModelsDeviceOnlyDescription;

  /// No description provided for @localModelsRunChat.
  ///
  /// In en, this message translates to:
  /// **'Run and configure'**
  String get localModelsRunChat;

  /// No description provided for @localModelsUseForChat.
  ///
  /// In en, this message translates to:
  /// **'Use for chat'**
  String get localModelsUseForChat;

  /// No description provided for @localModelsChatActivated.
  ///
  /// In en, this message translates to:
  /// **'Selected as the chat model'**
  String get localModelsChatActivated;

  /// No description provided for @localModelsPrompt.
  ///
  /// In en, this message translates to:
  /// **'Enter a prompt'**
  String get localModelsPrompt;

  /// No description provided for @localModelsGenerate.
  ///
  /// In en, this message translates to:
  /// **'Generate'**
  String get localModelsGenerate;

  /// No description provided for @localModelsGenerating.
  ///
  /// In en, this message translates to:
  /// **'Generating…'**
  String get localModelsGenerating;

  /// No description provided for @localModelsThreads.
  ///
  /// In en, this message translates to:
  /// **'CPU threads'**
  String get localModelsThreads;

  /// No description provided for @localModelsContextSize.
  ///
  /// In en, this message translates to:
  /// **'Context size (tokens)'**
  String get localModelsContextSize;

  /// No description provided for @localModelsBatchSize.
  ///
  /// In en, this message translates to:
  /// **'Batch size'**
  String get localModelsBatchSize;

  /// No description provided for @localModelsUnload.
  ///
  /// In en, this message translates to:
  /// **'Unload model'**
  String get localModelsUnload;

  /// No description provided for @localModelsCpuBackend.
  ///
  /// In en, this message translates to:
  /// **'On-device CPU inference'**
  String get localModelsCpuBackend;

  /// No description provided for @localModelsLlmDescription.
  ///
  /// In en, this message translates to:
  /// **'Download a GGUF model for llama.cpp or an MNN model bundle. After installation, try it here or select it for chat.'**
  String get localModelsLlmDescription;

  /// No description provided for @localModelsInvalidSettings.
  ///
  /// In en, this message translates to:
  /// **'Check the thread count, context size and batch size.'**
  String get localModelsInvalidSettings;

  /// No description provided for @localModelsFilterAll.
  ///
  /// In en, this message translates to:
  /// **'All models'**
  String get localModelsFilterAll;

  /// No description provided for @localModelsFilterInstalled.
  ///
  /// In en, this message translates to:
  /// **'Installed only'**
  String get localModelsFilterInstalled;

  /// No description provided for @localModelsSearchCatalog.
  ///
  /// In en, this message translates to:
  /// **'Search models, tags or engines'**
  String get localModelsSearchCatalog;

  /// No description provided for @deviceSpaceSyncPeer.
  ///
  /// In en, this message translates to:
  /// **'a space device'**
  String get deviceSpaceSyncPeer;

  /// No description provided for @deviceSpaceSyncActive.
  ///
  /// In en, this message translates to:
  /// **'Syncing with {device}'**
  String deviceSpaceSyncActive(String device);

  /// No description provided for @deviceSpaceSyncCompleted.
  ///
  /// In en, this message translates to:
  /// **'Synced with {device}'**
  String deviceSpaceSyncCompleted(String device);

  /// No description provided for @deviceSpaceSyncChecking.
  ///
  /// In en, this message translates to:
  /// **'Checking for changes'**
  String get deviceSpaceSyncChecking;

  /// No description provided for @deviceSpaceSyncRecords.
  ///
  /// In en, this message translates to:
  /// **'Exchanging records'**
  String get deviceSpaceSyncRecords;

  /// No description provided for @deviceSpaceSyncRecordProgress.
  ///
  /// In en, this message translates to:
  /// **'{completed} / {total} records · {percent}%'**
  String deviceSpaceSyncRecordProgress(int completed, int total, int percent);

  /// No description provided for @deviceSpaceSyncPendingCount.
  ///
  /// In en, this message translates to:
  /// **'{count} records pending application'**
  String deviceSpaceSyncPendingCount(int count);

  /// No description provided for @deviceSpaceSyncCounting.
  ///
  /// In en, this message translates to:
  /// **'Counting total · {count} records processed'**
  String deviceSpaceSyncCounting(int count);

  /// No description provided for @deviceSpaceSyncFiles.
  ///
  /// In en, this message translates to:
  /// **'Transferring and verifying files'**
  String get deviceSpaceSyncFiles;

  /// No description provided for @deviceSpaceSyncRetrying.
  ///
  /// In en, this message translates to:
  /// **'Sync interrupted. Retrying automatically.'**
  String get deviceSpaceSyncRetrying;

  /// No description provided for @deviceSpaceSyncPending.
  ///
  /// In en, this message translates to:
  /// **'Some data is still pending.'**
  String get deviceSpaceSyncPending;

  /// No description provided for @deviceSpaceSyncCurrentFile.
  ///
  /// In en, this message translates to:
  /// **'Current file: {done} / {total}'**
  String deviceSpaceSyncCurrentFile(String done, String total);

  /// No description provided for @deviceSpaceSyncTransferred.
  ///
  /// In en, this message translates to:
  /// **'{count} records exchanged · {bytes} transferred'**
  String deviceSpaceSyncTransferred(int count, String bytes);

  /// No description provided for @deviceSpaceSyncBackground.
  ///
  /// In en, this message translates to:
  /// **'You can keep using the app while syncing continues.'**
  String get deviceSpaceSyncBackground;

  /// No description provided for @runtimeActivityTitle.
  ///
  /// In en, this message translates to:
  /// **'Background tasks'**
  String get runtimeActivityTitle;

  /// No description provided for @runtimeActivitySyncing.
  ///
  /// In en, this message translates to:
  /// **'Syncing'**
  String get runtimeActivitySyncing;

  /// No description provided for @runtimeActivitySyncCompleted.
  ///
  /// In en, this message translates to:
  /// **'Sync complete'**
  String get runtimeActivitySyncCompleted;

  /// No description provided for @runtimeActivityWaiting.
  ///
  /// In en, this message translates to:
  /// **'Waiting to retry'**
  String get runtimeActivityWaiting;

  /// No description provided for @runtimeActivityExpand.
  ///
  /// In en, this message translates to:
  /// **'Show background tasks'**
  String get runtimeActivityExpand;

  /// No description provided for @runtimeActivityLoadingPackages.
  ///
  /// In en, this message translates to:
  /// **'Loading extensions'**
  String get runtimeActivityLoadingPackages;

  /// No description provided for @peerQrTitle.
  ///
  /// In en, this message translates to:
  /// **'QR connection'**
  String get peerQrTitle;

  /// No description provided for @peerQrScan.
  ///
  /// In en, this message translates to:
  /// **'Scan code'**
  String get peerQrScan;

  /// No description provided for @peerQrShow.
  ///
  /// In en, this message translates to:
  /// **'My code'**
  String get peerQrShow;

  /// No description provided for @peerQrShowHint.
  ///
  /// In en, this message translates to:
  /// **'Scan this code on another device to connect to this device.'**
  String get peerQrShowHint;

  /// No description provided for @peerQrScanHint.
  ///
  /// In en, this message translates to:
  /// **'Scan the Operit connection code on the other device.'**
  String get peerQrScanHint;

  /// No description provided for @peerQrTrustHint.
  ///
  /// In en, this message translates to:
  /// **'This QR code is valid for 5 minutes. Share it only with trusted devices. Confirming joins directly; this device automatically approves when it has administrator approval permissions.'**
  String get peerQrTrustHint;

  /// No description provided for @peerQrConfirmJoin.
  ///
  /// In en, this message translates to:
  /// **'Confirm join'**
  String get peerQrConfirmJoin;

  /// No description provided for @peerQrConfirmJoinHint.
  ///
  /// In en, this message translates to:
  /// **'Join this device’s space? Confirm to connect and submit the join request without entering a pairing code.'**
  String get peerQrConfirmJoinHint;

  /// No description provided for @peerQrListenerRequired.
  ///
  /// In en, this message translates to:
  /// **'Enable receiving connections in connection settings and connect to a network first.'**
  String get peerQrListenerRequired;

  /// No description provided for @peerQrCopy.
  ///
  /// In en, this message translates to:
  /// **'Copy connection info'**
  String get peerQrCopy;

  /// No description provided for @peerQrCopied.
  ///
  /// In en, this message translates to:
  /// **'Copied'**
  String get peerQrCopied;

  /// No description provided for @peerQrImport.
  ///
  /// In en, this message translates to:
  /// **'Import QR image'**
  String get peerQrImport;

  /// No description provided for @peerQrPaste.
  ///
  /// In en, this message translates to:
  /// **'Or paste connection info'**
  String get peerQrPaste;

  /// No description provided for @peerQrRead.
  ///
  /// In en, this message translates to:
  /// **'Read connection info'**
  String get peerQrRead;

  /// No description provided for @peerQrWrongDevice.
  ///
  /// In en, this message translates to:
  /// **'This code belongs to a different device. Scan the code for the selected device.'**
  String get peerQrWrongDevice;

  /// No description provided for @peerPairTimeoutHint.
  ///
  /// In en, this message translates to:
  /// **'Connection timed out: could not reach the other device. A common cause is the other device’s firewall blocking incoming connections. Try initiating the connection from the other device, or allow Operit through the other device’s firewall (especially on public networks).'**
  String get peerPairTimeoutHint;

  /// No description provided for @peerQrImageInvalid.
  ///
  /// In en, this message translates to:
  /// **'No QR code could be read. Choose a clear QR image smaller than 16 MB.'**
  String get peerQrImageInvalid;

  /// No description provided for @peerQrCameraUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Camera scanning is unavailable. Import a QR image or paste connection info instead.'**
  String get peerQrCameraUnavailable;

  /// No description provided for @peerQrCameraPermission.
  ///
  /// In en, this message translates to:
  /// **'Camera access is required. Allow it in system settings, or import a QR image.'**
  String get peerQrCameraPermission;

  /// No description provided for @peerQrDevice.
  ///
  /// In en, this message translates to:
  /// **'Connect device'**
  String get peerQrDevice;

  /// No description provided for @peerQrScanAgain.
  ///
  /// In en, this message translates to:
  /// **'Scan another code'**
  String get peerQrScanAgain;

  /// No description provided for @peerQrMethodsHint.
  ///
  /// In en, this message translates to:
  /// **'Can’t find the device? Connect with a QR code.'**
  String get peerQrMethodsHint;

  /// No description provided for @toolParametersLabel.
  ///
  /// In en, this message translates to:
  /// **'Parameters'**
  String get toolParametersLabel;

  /// No description provided for @toolResultLabel.
  ///
  /// In en, this message translates to:
  /// **'Result'**
  String get toolResultLabel;

  /// No description provided for @toolGroupThinkingCount.
  ///
  /// In en, this message translates to:
  /// **'{count} thinking'**
  String toolGroupThinkingCount(int count);

  /// No description provided for @toolGroupCallCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{count} call} other{{count} calls}}'**
  String toolGroupCallCount(int count);

  /// No description provided for @toolGroupSearchCount.
  ///
  /// In en, this message translates to:
  /// **'{count} search'**
  String toolGroupSearchCount(int count);

  /// No description provided for @toolEditedFileCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{Edited {count} file} other{Edited {count} files}}'**
  String toolEditedFileCount(int count);

  /// No description provided for @toolViewChanges.
  ///
  /// In en, this message translates to:
  /// **'View changes'**
  String get toolViewChanges;

  /// No description provided for @toolViewFileDiff.
  ///
  /// In en, this message translates to:
  /// **'View diff for {fileName}'**
  String toolViewFileDiff(String fileName);

  /// No description provided for @toolGroupCompleted.
  ///
  /// In en, this message translates to:
  /// **'Completed'**
  String get toolGroupCompleted;

  /// No description provided for @toolGroupExecuting.
  ///
  /// In en, this message translates to:
  /// **'Executing'**
  String get toolGroupExecuting;

  /// No description provided for @settingsDataTimelineTitle.
  ///
  /// In en, this message translates to:
  /// **'Model usage over time'**
  String get settingsDataTimelineTitle;

  /// No description provided for @settingsDataTimelineHint.
  ///
  /// In en, this message translates to:
  /// **'One line per model. Pinch or scroll to zoom, drag to move, and tap to inspect a time bucket.'**
  String get settingsDataTimelineHint;

  /// No description provided for @settingsDataTimelineHours.
  ///
  /// In en, this message translates to:
  /// **'{hours}h'**
  String settingsDataTimelineHours(int hours);

  /// No description provided for @settingsDataTimelineBucket.
  ///
  /// In en, this message translates to:
  /// **'{minutes} min buckets'**
  String settingsDataTimelineBucket(int minutes);

  /// No description provided for @settingsDataTimelineSecondsBucket.
  ///
  /// In en, this message translates to:
  /// **'{seconds} sec buckets'**
  String settingsDataTimelineSecondsBucket(int seconds);

  /// No description provided for @settingsDataTimelineEarlier.
  ///
  /// In en, this message translates to:
  /// **'Earlier'**
  String get settingsDataTimelineEarlier;

  /// No description provided for @settingsDataTimelineLater.
  ///
  /// In en, this message translates to:
  /// **'Later'**
  String get settingsDataTimelineLater;

  /// No description provided for @settingsDataTimelineRecent.
  ///
  /// In en, this message translates to:
  /// **'Recent 3h'**
  String get settingsDataTimelineRecent;

  /// No description provided for @settingsDataTimelineRange.
  ///
  /// In en, this message translates to:
  /// **'Visible time range'**
  String get settingsDataTimelineRange;

  /// No description provided for @settingsDataTimelineSelectModel.
  ///
  /// In en, this message translates to:
  /// **'Select a model below to display its usage'**
  String get settingsDataTimelineSelectModel;

  /// No description provided for @settingsDataTimelineSpend.
  ///
  /// In en, this message translates to:
  /// **'Spending (estimated)'**
  String get settingsDataTimelineSpend;

  /// No description provided for @settingsDataTimelineCacheRate.
  ///
  /// In en, this message translates to:
  /// **'Cache hit rate'**
  String get settingsDataTimelineCacheRate;

  /// No description provided for @settingsDataTimelineSpendHint.
  ///
  /// In en, this message translates to:
  /// **'Estimated using current model prices. Shares the time range above.'**
  String get settingsDataTimelineSpendHint;

  /// No description provided for @settingsDataTimelineCacheRateHint.
  ///
  /// In en, this message translates to:
  /// **'Cached input / total input per time bucket. Idle periods have no rate.'**
  String get settingsDataTimelineCacheRateHint;

  /// No description provided for @settingsDataTimelineNoPrices.
  ///
  /// In en, this message translates to:
  /// **'No matching model prices'**
  String get settingsDataTimelineNoPrices;

  /// No description provided for @settingsDataTimelinePricesFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not load model prices. Refresh to retry.'**
  String get settingsDataTimelinePricesFailed;

  /// No description provided for @settingsDataTimelineUnpriced.
  ///
  /// In en, this message translates to:
  /// **'{count} requests have unknown cost'**
  String settingsDataTimelineUnpriced(int count);

  /// No description provided for @settingsDataTimelineKnownSpend.
  ///
  /// In en, this message translates to:
  /// **'Known spending (estimated)'**
  String get settingsDataTimelineKnownSpend;

  /// No description provided for @settingsDataDetailedStatsRangeLastDay.
  ///
  /// In en, this message translates to:
  /// **'Last 24 hours'**
  String get settingsDataDetailedStatsRangeLastDay;

  /// No description provided for @settingsDataTimelineAverageInput.
  ///
  /// In en, this message translates to:
  /// **'Average input per request'**
  String get settingsDataTimelineAverageInput;

  /// No description provided for @settingsDataTimelineAverageInputHint.
  ///
  /// In en, this message translates to:
  /// **'Input tokens / requests per bucket, to spot growing context sizes. Idle periods are gaps.'**
  String get settingsDataTimelineAverageInputHint;

  /// No description provided for @settingsDataTimelineCacheSavings.
  ///
  /// In en, this message translates to:
  /// **'Cache savings (estimated)'**
  String get settingsDataTimelineCacheSavings;

  /// No description provided for @settingsDataTimelineCacheSavingsHint.
  ///
  /// In en, this message translates to:
  /// **'At current prices, cached input cost compared with regular input. Per-request billing saves 0; negative values mean a cache surcharge.'**
  String get settingsDataTimelineCacheSavingsHint;

  /// No description provided for @settingsDataTimelineTokensPerRequest.
  ///
  /// In en, this message translates to:
  /// **'{tokens} tokens / request'**
  String settingsDataTimelineTokensPerRequest(String tokens);

  /// No description provided for @settingsDataTimelineUnknownSavings.
  ///
  /// In en, this message translates to:
  /// **'Savings unknown for {count} requests'**
  String settingsDataTimelineUnknownSavings(int count);

  /// No description provided for @contextStatsSummaryLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Unable to load context statistics'**
  String get contextStatsSummaryLoadFailed;

  /// No description provided for @contextStatsDevicesLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Unable to load device information'**
  String get contextStatsDevicesLoadFailed;

  /// No description provided for @settingsAppearanceThemeLibrary.
  ///
  /// In en, this message translates to:
  /// **'Your themes'**
  String get settingsAppearanceThemeLibrary;

  /// No description provided for @settingsAppearanceThemeLibraryHint.
  ///
  /// In en, this message translates to:
  /// **'Choose a look. Make it yours.'**
  String get settingsAppearanceThemeLibraryHint;

  /// No description provided for @settingsAppearanceThemeActive.
  ///
  /// In en, this message translates to:
  /// **'In use'**
  String get settingsAppearanceThemeActive;

  /// No description provided for @settingsAppearanceThemeApply.
  ///
  /// In en, this message translates to:
  /// **'Use this theme'**
  String get settingsAppearanceThemeApply;

  /// No description provided for @settingsAppearanceThemeMore.
  ///
  /// In en, this message translates to:
  /// **'More theme actions'**
  String get settingsAppearanceThemeMore;

  /// No description provided for @settingsAppearanceThemeDuplicate.
  ///
  /// In en, this message translates to:
  /// **'Create a copy'**
  String get settingsAppearanceThemeDuplicate;

  /// No description provided for @settingsAppearanceThemeUnboundHint.
  ///
  /// In en, this message translates to:
  /// **'Keep the current look and edit without binding a saved theme.'**
  String get settingsAppearanceThemeUnboundHint;

  /// No description provided for @settingsAppearanceThemeDeleteTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete theme'**
  String get settingsAppearanceThemeDeleteTitle;

  /// No description provided for @settingsAppearanceThemeDeleteConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete “{name}”? This cannot be undone.'**
  String settingsAppearanceThemeDeleteConfirm(String name);

  /// No description provided for @settingsAppearancePreviewCollapse.
  ///
  /// In en, this message translates to:
  /// **'Collapse'**
  String get settingsAppearancePreviewCollapse;

  /// No description provided for @settingsAppearancePreviewExpand.
  ///
  /// In en, this message translates to:
  /// **'Expand'**
  String get settingsAppearancePreviewExpand;

  /// No description provided for @settingsAppearanceThemeNew.
  ///
  /// In en, this message translates to:
  /// **'New theme'**
  String get settingsAppearanceThemeNew;

  /// No description provided for @settingsAppearanceDeviceFonts.
  ///
  /// In en, this message translates to:
  /// **'Device fonts'**
  String get settingsAppearanceDeviceFonts;

  /// No description provided for @settingsAppearanceChooseDeviceFont.
  ///
  /// In en, this message translates to:
  /// **'Choose device font'**
  String get settingsAppearanceChooseDeviceFont;

  /// No description provided for @settingsAppearanceNoDeviceFonts.
  ///
  /// In en, this message translates to:
  /// **'No matching device fonts'**
  String get settingsAppearanceNoDeviceFonts;

  /// No description provided for @settingsAppearanceDeviceFontPreview.
  ///
  /// In en, this message translates to:
  /// **'The quick brown fox · 0123456789\n字体预览：你好，世界'**
  String get settingsAppearanceDeviceFontPreview;

  /// No description provided for @workspaceFileRename.
  ///
  /// In en, this message translates to:
  /// **'Rename'**
  String get workspaceFileRename;

  /// No description provided for @workspaceFileCopy.
  ///
  /// In en, this message translates to:
  /// **'Copy'**
  String get workspaceFileCopy;

  /// No description provided for @workspaceFileCut.
  ///
  /// In en, this message translates to:
  /// **'Cut'**
  String get workspaceFileCut;

  /// No description provided for @workspaceFilePaste.
  ///
  /// In en, this message translates to:
  /// **'Paste'**
  String get workspaceFilePaste;

  /// No description provided for @workspaceFileName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get workspaceFileName;

  /// No description provided for @workspaceFileInvalidName.
  ///
  /// In en, this message translates to:
  /// **'Enter a valid name without path separators.'**
  String get workspaceFileInvalidName;

  /// No description provided for @workspaceDeleteFolderTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete folder?'**
  String get workspaceDeleteFolderTitle;

  /// No description provided for @workspaceDeleteFolderMessage.
  ///
  /// In en, this message translates to:
  /// **'Delete “{name}” and all its contents? This cannot be undone.'**
  String workspaceDeleteFolderMessage(String name);

  /// No description provided for @workspaceFileOperationFailed.
  ///
  /// In en, this message translates to:
  /// **'File operation failed: {error}'**
  String workspaceFileOperationFailed(String error);

  /// No description provided for @workspaceFileCopyName.
  ///
  /// In en, this message translates to:
  /// **'{name} (copy {index})'**
  String workspaceFileCopyName(String name, int index);

  /// No description provided for @workspaceFileSaving.
  ///
  /// In en, this message translates to:
  /// **'This file is being saved. Please try again when saving finishes.'**
  String get workspaceFileSaving;

  /// No description provided for @chatBackupChooseTitle.
  ///
  /// In en, this message translates to:
  /// **'Choose your backup directory'**
  String get chatBackupChooseTitle;

  /// No description provided for @chatBackupTitle.
  ///
  /// In en, this message translates to:
  /// **'Daily chat backup'**
  String get chatBackupTitle;

  /// No description provided for @chatBackupDescription.
  ///
  /// In en, this message translates to:
  /// **'Copy the complete SQLite database file daily while the app is running and keep the latest 10 automatic backups. Includes all data stored in this database. External attachments, workspaces and separate configuration files are not copied.'**
  String get chatBackupDescription;

  /// No description provided for @chatRestoreTitle.
  ///
  /// In en, this message translates to:
  /// **'Restore from a database backup'**
  String get chatRestoreTitle;

  /// No description provided for @chatRestoreDescription.
  ///
  /// In en, this message translates to:
  /// **'Open the folder containing your backups and choose the date you want to restore from.'**
  String get chatRestoreDescription;

  /// No description provided for @chatRestoreChooseFolder.
  ///
  /// In en, this message translates to:
  /// **'Choose backup source'**
  String get chatRestoreChooseFolder;

  /// No description provided for @chatRestoreRefresh.
  ///
  /// In en, this message translates to:
  /// **'Refresh list'**
  String get chatRestoreRefresh;

  /// No description provided for @chatRestoreSelected.
  ///
  /// In en, this message translates to:
  /// **'Restore selected backup'**
  String get chatRestoreSelected;

  /// No description provided for @chatRestoreAction.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get chatRestoreAction;

  /// No description provided for @chatRestoreCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get chatRestoreCancel;

  /// No description provided for @chatRestoreEmpty.
  ///
  /// In en, this message translates to:
  /// **'This folder contains no database backups.'**
  String get chatRestoreEmpty;

  /// No description provided for @chatRestoreConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Restore this database backup?'**
  String get chatRestoreConfirmTitle;

  /// No description provided for @chatRestoreConfirmDescription.
  ///
  /// In en, this message translates to:
  /// **'Replace the current database file with {name}. This replaces all data stored in this database. Stop running conversations first. Restoring establishes a new synchronization baseline that replaces older data on other devices.'**
  String chatRestoreConfirmDescription(String name);

  /// No description provided for @chatRestoreSuccess.
  ///
  /// In en, this message translates to:
  /// **'Database restored. The next sync will use this copy as the source of truth.'**
  String get chatRestoreSuccess;

  /// No description provided for @chatRestoreOnboardingTitle.
  ///
  /// In en, this message translates to:
  /// **'Restore from a previous backup'**
  String get chatRestoreOnboardingTitle;

  /// No description provided for @chatRestoreOnboardingDescription.
  ///
  /// In en, this message translates to:
  /// **'Recover chats saved in an external folder before reinstalling or uninstalling'**
  String get chatRestoreOnboardingDescription;

  /// No description provided for @chatBackupExternalDescription.
  ///
  /// In en, this message translates to:
  /// **'Choose a folder outside the app so backups remain after uninstall. On iPhone and iPad, choose iCloud Drive or another external file provider.'**
  String get chatBackupExternalDescription;

  /// No description provided for @chatBackupChoose.
  ///
  /// In en, this message translates to:
  /// **'Choose backup directory'**
  String get chatBackupChoose;

  /// No description provided for @chatBackupLater.
  ///
  /// In en, this message translates to:
  /// **'Set up later'**
  String get chatBackupLater;

  /// No description provided for @chatBackupChange.
  ///
  /// In en, this message translates to:
  /// **'Change backup directory'**
  String get chatBackupChange;

  /// No description provided for @chatBackupMissing.
  ///
  /// In en, this message translates to:
  /// **'The backup directory is not configured, no longer exists or cannot be accessed. Select a directory to enable daily chat backups.'**
  String get chatBackupMissing;

  /// No description provided for @chatBackupDirectoryUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The selected folder cannot be used. Create a new directory inside an accessible folder, such as Documents or Downloads, and select that directory.'**
  String get chatBackupDirectoryUnavailable;

  /// No description provided for @chatBackupReady.
  ///
  /// In en, this message translates to:
  /// **'The backup directory is ready. Daily backups are checked at startup, on resume and while the app is running.'**
  String get chatBackupReady;

  /// No description provided for @chatBackupNow.
  ///
  /// In en, this message translates to:
  /// **'Back up chats now'**
  String get chatBackupNow;

  /// No description provided for @chatBackupNever.
  ///
  /// In en, this message translates to:
  /// **'No successful backup yet'**
  String get chatBackupNever;

  /// No description provided for @chatBackupLastSuccess.
  ///
  /// In en, this message translates to:
  /// **'Last successful backup: {time}'**
  String chatBackupLastSuccess(String time);

  /// No description provided for @terminalDistributionsTitle.
  ///
  /// In en, this message translates to:
  /// **'Download Linux distributions'**
  String get terminalDistributionsTitle;

  /// No description provided for @terminalDistributionDownload.
  ///
  /// In en, this message translates to:
  /// **'Download and install'**
  String get terminalDistributionDownload;

  /// No description provided for @terminalDistributionInstallAnother.
  ///
  /// In en, this message translates to:
  /// **'Install another'**
  String get terminalDistributionInstallAnother;

  /// No description provided for @terminalInstallQueued.
  ///
  /// In en, this message translates to:
  /// **'Waiting to install'**
  String get terminalInstallQueued;

  /// No description provided for @terminalInstallConnecting.
  ///
  /// In en, this message translates to:
  /// **'Testing download mirrors…'**
  String get terminalInstallConnecting;

  /// No description provided for @terminalInstallCancelling.
  ///
  /// In en, this message translates to:
  /// **'Cancelling…'**
  String get terminalInstallCancelling;

  /// No description provided for @terminalInstallErrorDetails.
  ///
  /// In en, this message translates to:
  /// **'Error details'**
  String get terminalInstallErrorDetails;

  /// No description provided for @terminalInstallDownloading.
  ///
  /// In en, this message translates to:
  /// **'Downloading root filesystem'**
  String get terminalInstallDownloading;

  /// No description provided for @terminalInstallVerifying.
  ///
  /// In en, this message translates to:
  /// **'Verifying SHA-256'**
  String get terminalInstallVerifying;

  /// No description provided for @terminalInstallExtracting.
  ///
  /// In en, this message translates to:
  /// **'Installing root filesystem'**
  String get terminalInstallExtracting;

  /// No description provided for @terminalInstallCompleted.
  ///
  /// In en, this message translates to:
  /// **'Installed · available in terminal profiles'**
  String get terminalInstallCompleted;

  /// No description provided for @terminalInstallCancelled.
  ///
  /// In en, this message translates to:
  /// **'Installation cancelled'**
  String get terminalInstallCancelled;

  /// No description provided for @terminalInstallFailed.
  ///
  /// In en, this message translates to:
  /// **'Installation failed'**
  String get terminalInstallFailed;

  /// No description provided for @terminalProfileWslDistribution.
  ///
  /// In en, this message translates to:
  /// **'WSL distribution'**
  String get terminalProfileWslDistribution;

  /// No description provided for @terminalProfileWslDistributionRequired.
  ///
  /// In en, this message translates to:
  /// **'Select a WSL distribution.'**
  String get terminalProfileWslDistributionRequired;

  /// No description provided for @terminalProfileWslHelp.
  ///
  /// In en, this message translates to:
  /// **'Install a WSL distribution on the Windows device running Core, then reopen this dialog.'**
  String get terminalProfileWslHelp;

  /// No description provided for @appUpdateUnavailableTitle.
  ///
  /// In en, this message translates to:
  /// **'Update temporarily unavailable'**
  String get appUpdateUnavailableTitle;

  /// No description provided for @appUpdatePlanning.
  ///
  /// In en, this message translates to:
  /// **'Checking for updates and selecting a download source…'**
  String get appUpdatePlanning;

  /// No description provided for @appUpdateSelectingSource.
  ///
  /// In en, this message translates to:
  /// **'Selecting a download source…'**
  String get appUpdateSelectingSource;

  /// No description provided for @appUpdateAwaitingBase.
  ///
  /// In en, this message translates to:
  /// **'The original installer is required'**
  String get appUpdateAwaitingBase;

  /// No description provided for @appUpdateValidatingBase.
  ///
  /// In en, this message translates to:
  /// **'Verifying the selected installer…'**
  String get appUpdateValidatingBase;

  /// No description provided for @appUpdateDownloadingPatch.
  ///
  /// In en, this message translates to:
  /// **'Downloading the incremental patch…'**
  String get appUpdateDownloadingPatch;

  /// No description provided for @appUpdateDownloadingFull.
  ///
  /// In en, this message translates to:
  /// **'Downloading the full installer…'**
  String get appUpdateDownloadingFull;

  /// No description provided for @appUpdateVerifyingBase.
  ///
  /// In en, this message translates to:
  /// **'Verifying the original installer…'**
  String get appUpdateVerifyingBase;

  /// No description provided for @appUpdateVerifyingPatch.
  ///
  /// In en, this message translates to:
  /// **'Verifying the incremental patch…'**
  String get appUpdateVerifyingPatch;

  /// No description provided for @appUpdateExpandingBase.
  ///
  /// In en, this message translates to:
  /// **'Preparing the original installer…'**
  String get appUpdateExpandingBase;

  /// No description provided for @appUpdateVerifyingExpandedBase.
  ///
  /// In en, this message translates to:
  /// **'Verifying update files…'**
  String get appUpdateVerifyingExpandedBase;

  /// No description provided for @appUpdateApplyingPatch.
  ///
  /// In en, this message translates to:
  /// **'Applying the patch…'**
  String get appUpdateApplyingPatch;

  /// No description provided for @appUpdateRestoringPackage.
  ///
  /// In en, this message translates to:
  /// **'Creating the new installer…'**
  String get appUpdateRestoringPackage;

  /// No description provided for @appUpdateVerifyingPackage.
  ///
  /// In en, this message translates to:
  /// **'Verifying the new installer…'**
  String get appUpdateVerifyingPackage;

  /// No description provided for @appUpdateSavingPackage.
  ///
  /// In en, this message translates to:
  /// **'Saving the installer…'**
  String get appUpdateSavingPackage;

  /// No description provided for @appUpdatePreparingFiles.
  ///
  /// In en, this message translates to:
  /// **'Preparing update files…'**
  String get appUpdatePreparingFiles;

  /// No description provided for @appUpdateReady.
  ///
  /// In en, this message translates to:
  /// **'Update downloaded and verified. Confirm to install.'**
  String get appUpdateReady;

  /// No description provided for @appUpdatePreparingInstall.
  ///
  /// In en, this message translates to:
  /// **'Preparing to install…'**
  String get appUpdatePreparingInstall;

  /// No description provided for @appUpdateInstalling.
  ///
  /// In en, this message translates to:
  /// **'Starting installation. The app will restart shortly…'**
  String get appUpdateInstalling;

  /// No description provided for @appUpdateRestartRequired.
  ///
  /// In en, this message translates to:
  /// **'Download complete. Restart to install the new version.'**
  String get appUpdateRestartRequired;

  /// No description provided for @appUpdateInstallSubmitted.
  ///
  /// In en, this message translates to:
  /// **'Installation requested. Confirm in the system dialog.'**
  String get appUpdateInstallSubmitted;

  /// No description provided for @appUpdatePermissionRequired.
  ///
  /// In en, this message translates to:
  /// **'App installation permission required'**
  String get appUpdatePermissionRequired;

  /// No description provided for @appUpdateUpdateIncomplete.
  ///
  /// In en, this message translates to:
  /// **'Update incomplete'**
  String get appUpdateUpdateIncomplete;

  /// No description provided for @appUpdateInstallIncomplete.
  ///
  /// In en, this message translates to:
  /// **'Installation incomplete'**
  String get appUpdateInstallIncomplete;

  /// No description provided for @appUpdatePreparingDownload.
  ///
  /// In en, this message translates to:
  /// **'Preparing to download the update…'**
  String get appUpdatePreparingDownload;

  /// No description provided for @appUpdateTitle.
  ///
  /// In en, this message translates to:
  /// **'Update Operit'**
  String get appUpdateTitle;

  /// No description provided for @appUpdateNoOriginalInstaller.
  ///
  /// In en, this message translates to:
  /// **'I no longer have the installer'**
  String get appUpdateNoOriginalInstaller;

  /// No description provided for @appUpdateChooseInstaller.
  ///
  /// In en, this message translates to:
  /// **'Select'**
  String get appUpdateChooseInstaller;

  /// No description provided for @appUpdateBackground.
  ///
  /// In en, this message translates to:
  /// **'Update in background'**
  String get appUpdateBackground;

  /// No description provided for @appUpdateCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel update'**
  String get appUpdateCancel;

  /// No description provided for @appUpdateOpenRelease.
  ///
  /// In en, this message translates to:
  /// **'Open release page'**
  String get appUpdateOpenRelease;

  /// No description provided for @appUpdateContinueInstall.
  ///
  /// In en, this message translates to:
  /// **'Continue installation'**
  String get appUpdateContinueInstall;

  /// No description provided for @appUpdateRetry.
  ///
  /// In en, this message translates to:
  /// **'Retry update'**
  String get appUpdateRetry;

  /// No description provided for @appUpdateReinstall.
  ///
  /// In en, this message translates to:
  /// **'Reinstall'**
  String get appUpdateReinstall;

  /// No description provided for @appUpdateRestartInstall.
  ///
  /// In en, this message translates to:
  /// **'Restart and install'**
  String get appUpdateRestartInstall;

  /// No description provided for @appUpdateReopenInstaller.
  ///
  /// In en, this message translates to:
  /// **'Reopen installer'**
  String get appUpdateReopenInstaller;

  /// No description provided for @appUpdateRestartUpdate.
  ///
  /// In en, this message translates to:
  /// **'Restart and update'**
  String get appUpdateRestartUpdate;

  /// No description provided for @appUpdateSelectOriginalInstaller.
  ///
  /// In en, this message translates to:
  /// **'Select the original installer used to install this app. Once verified, the incremental patch will be downloaded and applied to create the new installer.'**
  String get appUpdateSelectOriginalInstaller;

  /// No description provided for @appUpdateProcessingHelp.
  ///
  /// In en, this message translates to:
  /// **'Update files are being processed on this device. Creating the installer may take a while. You can continue in the background and select the Operit logo later to check progress.'**
  String get appUpdateProcessingHelp;

  /// No description provided for @appUpdateDownloadHelp.
  ///
  /// In en, this message translates to:
  /// **'After downloading, the installer is verified and kept until you confirm the update. You can close this window and select the Operit logo later to check progress.'**
  String get appUpdateDownloadHelp;

  /// No description provided for @appUpdatePermissionHelp.
  ///
  /// In en, this message translates to:
  /// **'Grant the system permission, then select Continue installation. The installer has been kept.'**
  String get appUpdatePermissionHelp;

  /// No description provided for @appUpdateReadyHelp.
  ///
  /// In en, this message translates to:
  /// **'Select “Restart and update” to begin installation. Your system may ask for permission to install.'**
  String get appUpdateReadyHelp;

  /// No description provided for @appUpdateRestartHelp.
  ///
  /// In en, this message translates to:
  /// **'Select “Restart and install” to close the app, install the new version, and reopen it.'**
  String get appUpdateRestartHelp;

  /// No description provided for @appUpdateSystemInstallHelp.
  ///
  /// In en, this message translates to:
  /// **'Confirm in the system installer and reopen Operit when installation finishes. You can reopen the installer if you closed it.'**
  String get appUpdateSystemInstallHelp;

  /// No description provided for @appUpdateInstallerKept.
  ///
  /// In en, this message translates to:
  /// **'The installer has been kept. You can retry without downloading it again.'**
  String get appUpdateInstallerKept;

  /// No description provided for @appUpdateReleaseNotes.
  ///
  /// In en, this message translates to:
  /// **'What’s new'**
  String get appUpdateReleaseNotes;

  /// No description provided for @appUpdateAvailableTitle.
  ///
  /// In en, this message translates to:
  /// **'Update available'**
  String get appUpdateAvailableTitle;

  /// No description provided for @appUpdateNow.
  ///
  /// In en, this message translates to:
  /// **'Update now'**
  String get appUpdateNow;

  /// No description provided for @appUpdateAvailable.
  ///
  /// In en, this message translates to:
  /// **'A new version of Operit is available'**
  String get appUpdateAvailable;

  /// No description provided for @appUpdatePromptReady.
  ///
  /// In en, this message translates to:
  /// **'The update is ready. Confirm to restart and install it.'**
  String get appUpdatePromptReady;

  /// No description provided for @appUpdatePromptAvailable.
  ///
  /// In en, this message translates to:
  /// **'Update to get the latest features and improvements.'**
  String get appUpdatePromptAvailable;

  /// No description provided for @appUpdateCurrentVersion.
  ///
  /// In en, this message translates to:
  /// **'Current version'**
  String get appUpdateCurrentVersion;

  /// No description provided for @appUpdateLatestVersion.
  ///
  /// In en, this message translates to:
  /// **'Latest version'**
  String get appUpdateLatestVersion;

  /// No description provided for @appUpdatePromptLater.
  ///
  /// In en, this message translates to:
  /// **'You can continue the update later by selecting the Operit logo in the sidebar.'**
  String get appUpdatePromptLater;

  /// No description provided for @appUpdateOpenReleaseFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not open the release page'**
  String get appUpdateOpenReleaseFailed;

  /// No description provided for @appUpdateViewProgress.
  ///
  /// In en, this message translates to:
  /// **'View update progress'**
  String get appUpdateViewProgress;

  /// No description provided for @appUpdateFailedRetry.
  ///
  /// In en, this message translates to:
  /// **'Update failed. Select to retry.'**
  String get appUpdateFailedRetry;

  /// No description provided for @appUpdateUnavailableHelp.
  ///
  /// In en, this message translates to:
  /// **'The installer required for this update could not be retrieved, and no full installer is available. Download the installer for {version}, or find the original installer for {baseVersion} and retry.'**
  String appUpdateUnavailableHelp(String version, String baseVersion);

  /// No description provided for @appUpdateAvailableVersion.
  ///
  /// In en, this message translates to:
  /// **'Version {version} is available'**
  String appUpdateAvailableVersion(String version);

  /// No description provided for @appUpdateInstalledVersion.
  ///
  /// In en, this message translates to:
  /// **'Current version: {version}'**
  String appUpdateInstalledVersion(String version);

  /// No description provided for @appUpdateOriginalVersionPrompt.
  ///
  /// In en, this message translates to:
  /// **'Do you still have the original installer for version {version}?'**
  String appUpdateOriginalVersionPrompt(String version);

  /// No description provided for @appUpdatePatchSize.
  ///
  /// In en, this message translates to:
  /// **'Incremental download: about {size}'**
  String appUpdatePatchSize(String size);

  /// No description provided for @appUpdateStepProgress.
  ///
  /// In en, this message translates to:
  /// **'Current step: {percent}%'**
  String appUpdateStepProgress(int percent);

  /// No description provided for @appUpdateElapsed.
  ///
  /// In en, this message translates to:
  /// **'Elapsed: {duration}'**
  String appUpdateElapsed(String duration);

  /// No description provided for @appUpdateDownloadSizePending.
  ///
  /// In en, this message translates to:
  /// **'{size} · Checking file size'**
  String appUpdateDownloadSizePending(String size);

  /// No description provided for @appUpdateBadgeAvailable.
  ///
  /// In en, this message translates to:
  /// **'Version {version} is available. Select to update.'**
  String appUpdateBadgeAvailable(String version);

  /// No description provided for @apiErrorInvalidParametersTitle.
  ///
  /// In en, this message translates to:
  /// **'Invalid request parameters'**
  String get apiErrorInvalidParametersTitle;

  /// No description provided for @apiErrorReasoningContentTitle.
  ///
  /// In en, this message translates to:
  /// **'Use a provider that supports reasoning replay'**
  String get apiErrorReasoningContentTitle;

  /// No description provided for @apiErrorReasoningContentMessage.
  ///
  /// In en, this message translates to:
  /// **'The request involves reasoning_content. In model settings, select a provider type that supports sending reasoning content back, such as DeepSeek or OpenAI Compatible, and make sure it matches your API endpoint.'**
  String get apiErrorReasoningContentMessage;

  /// No description provided for @apiErrorInvalidParametersMessage.
  ///
  /// In en, this message translates to:
  /// **'The API could not process the request parameters. Check the model name, capabilities, and parameters, and review the original error.'**
  String get apiErrorInvalidParametersMessage;

  /// No description provided for @apiErrorInvalidKeyTitle.
  ///
  /// In en, this message translates to:
  /// **'Invalid API key'**
  String get apiErrorInvalidKeyTitle;

  /// No description provided for @apiErrorInvalidKeyMessage.
  ///
  /// In en, this message translates to:
  /// **'The server rejected the API key. Check that it is correct, complete, and active.'**
  String get apiErrorInvalidKeyMessage;

  /// No description provided for @apiErrorBillingTitle.
  ///
  /// In en, this message translates to:
  /// **'Insufficient API balance'**
  String get apiErrorBillingTitle;

  /// No description provided for @apiErrorBillingMessage.
  ///
  /// In en, this message translates to:
  /// **'Your API balance is too low or your quota is exhausted. Check your balance and billing with the API provider.'**
  String get apiErrorBillingMessage;

  /// No description provided for @apiErrorBlockedTitle.
  ///
  /// In en, this message translates to:
  /// **'Request blocked by server'**
  String get apiErrorBlockedTitle;

  /// No description provided for @apiErrorBlockedMessage.
  ///
  /// In en, this message translates to:
  /// **'The API server refused the request. Contact the provider to check for security or access restrictions.'**
  String get apiErrorBlockedMessage;

  /// No description provided for @apiErrorNotFoundTitle.
  ///
  /// In en, this message translates to:
  /// **'Endpoint or model not found'**
  String get apiErrorNotFoundTitle;

  /// No description provided for @apiErrorNotFoundMessage.
  ///
  /// In en, this message translates to:
  /// **'The API server could not find the resource. Check the server address, endpoint path, and model name.'**
  String get apiErrorNotFoundMessage;

  /// No description provided for @apiErrorUnsupportedMethodTitle.
  ///
  /// In en, this message translates to:
  /// **'Request method not supported'**
  String get apiErrorUnsupportedMethodTitle;

  /// No description provided for @apiErrorUnsupportedMethodMessage.
  ///
  /// In en, this message translates to:
  /// **'The server does not support this request method or operation. Check the provider type and API protocol.'**
  String get apiErrorUnsupportedMethodMessage;

  /// No description provided for @apiErrorUnsupportedContentTitle.
  ///
  /// In en, this message translates to:
  /// **'Unsupported request content'**
  String get apiErrorUnsupportedContentTitle;

  /// No description provided for @apiErrorUnsupportedContentMessage.
  ///
  /// In en, this message translates to:
  /// **'The server does not accept this content type. Check the provider protocol, media format, and custom headers.'**
  String get apiErrorUnsupportedContentMessage;

  /// No description provided for @apiErrorProxyAuthTitle.
  ///
  /// In en, this message translates to:
  /// **'Proxy authentication failed'**
  String get apiErrorProxyAuthTitle;

  /// No description provided for @apiErrorProxyAuthMessage.
  ///
  /// In en, this message translates to:
  /// **'The network proxy requires authentication. Check the proxy credentials and authentication settings.'**
  String get apiErrorProxyAuthMessage;

  /// No description provided for @apiErrorTimeoutTitle.
  ///
  /// In en, this message translates to:
  /// **'API request timed out'**
  String get apiErrorTimeoutTitle;

  /// No description provided for @apiErrorTimeoutMessage.
  ///
  /// In en, this message translates to:
  /// **'The request or a server connection timed out. Check your network connection and the provider status, then retry.'**
  String get apiErrorTimeoutMessage;

  /// No description provided for @apiErrorConflictTitle.
  ///
  /// In en, this message translates to:
  /// **'Request conflicts with resource state'**
  String get apiErrorConflictTitle;

  /// No description provided for @apiErrorConflictMessage.
  ///
  /// In en, this message translates to:
  /// **'The API server reported a conflict. Review the original error for details.'**
  String get apiErrorConflictMessage;

  /// No description provided for @apiErrorLengthRequiredTitle.
  ///
  /// In en, this message translates to:
  /// **'Request content length required'**
  String get apiErrorLengthRequiredTitle;

  /// No description provided for @apiErrorLengthRequiredMessage.
  ///
  /// In en, this message translates to:
  /// **'The API requires a declared content length. Contact the provider to confirm its requirements.'**
  String get apiErrorLengthRequiredMessage;

  /// No description provided for @apiErrorTooLargeTitle.
  ///
  /// In en, this message translates to:
  /// **'Request too large'**
  String get apiErrorTooLargeTitle;

  /// No description provided for @apiErrorTooLargeMessage.
  ///
  /// In en, this message translates to:
  /// **'The message, attachments, address, or headers exceed the server limit. See the original error to find what to reduce.'**
  String get apiErrorTooLargeMessage;

  /// No description provided for @apiErrorLockedTitle.
  ///
  /// In en, this message translates to:
  /// **'Resource locked'**
  String get apiErrorLockedTitle;

  /// No description provided for @apiErrorLockedMessage.
  ///
  /// In en, this message translates to:
  /// **'The API server temporarily locked the resource. Retry later or contact the provider.'**
  String get apiErrorLockedMessage;

  /// No description provided for @apiErrorTooEarlyTitle.
  ///
  /// In en, this message translates to:
  /// **'Server is not ready for this request'**
  String get apiErrorTooEarlyTitle;

  /// No description provided for @apiErrorTooEarlyMessage.
  ///
  /// In en, this message translates to:
  /// **'The server considered the request premature. Retry later.'**
  String get apiErrorTooEarlyMessage;

  /// No description provided for @apiErrorUpgradeRequiredTitle.
  ///
  /// In en, this message translates to:
  /// **'API protocol upgrade required'**
  String get apiErrorUpgradeRequiredTitle;

  /// No description provided for @apiErrorUpgradeRequiredMessage.
  ///
  /// In en, this message translates to:
  /// **'The server requires a different protocol version. Check the server address and provider protocol settings.'**
  String get apiErrorUpgradeRequiredMessage;

  /// No description provided for @apiErrorRateLimitedTitle.
  ///
  /// In en, this message translates to:
  /// **'Request rate limited or blocked'**
  String get apiErrorRateLimitedTitle;

  /// No description provided for @apiErrorRateLimitedMessage.
  ///
  /// In en, this message translates to:
  /// **'The API throttled or blocked the request. Retry later or contact the provider.'**
  String get apiErrorRateLimitedMessage;

  /// No description provided for @apiErrorLegalRestrictionTitle.
  ///
  /// In en, this message translates to:
  /// **'Service unavailable due to regional or legal restrictions'**
  String get apiErrorLegalRestrictionTitle;

  /// No description provided for @apiErrorLegalRestrictionMessage.
  ///
  /// In en, this message translates to:
  /// **'The server refused access because of regional or legal restrictions. Ask the provider where the service is available.'**
  String get apiErrorLegalRestrictionMessage;

  /// No description provided for @apiErrorInterruptedTitle.
  ///
  /// In en, this message translates to:
  /// **'Request interrupted or blocked'**
  String get apiErrorInterruptedTitle;

  /// No description provided for @apiErrorInterruptedMessage.
  ///
  /// In en, this message translates to:
  /// **'The request was interrupted. Share the original error with the provider to check for a security block, gateway issue, or closed connection.'**
  String get apiErrorInterruptedMessage;

  /// No description provided for @apiErrorInternalTitle.
  ///
  /// In en, this message translates to:
  /// **'Internal API server error'**
  String get apiErrorInternalTitle;

  /// No description provided for @apiErrorInternalMessage.
  ///
  /// In en, this message translates to:
  /// **'The API server encountered an internal error. Retry later or share the original error with the provider.'**
  String get apiErrorInternalMessage;

  /// No description provided for @apiErrorGatewayTitle.
  ///
  /// In en, this message translates to:
  /// **'Invalid API gateway response'**
  String get apiErrorGatewayTitle;

  /// No description provided for @apiErrorGatewayMessage.
  ///
  /// In en, this message translates to:
  /// **'The gateway did not receive a valid upstream response. Retry later or contact the provider.'**
  String get apiErrorGatewayMessage;

  /// No description provided for @apiErrorUnavailableTitle.
  ///
  /// In en, this message translates to:
  /// **'API service temporarily unavailable'**
  String get apiErrorUnavailableTitle;

  /// No description provided for @apiErrorUnavailableMessage.
  ///
  /// In en, this message translates to:
  /// **'The service may be under maintenance, overloaded, or out of capacity. Retry later or contact the provider.'**
  String get apiErrorUnavailableMessage;

  /// No description provided for @apiErrorOriginTitle.
  ///
  /// In en, this message translates to:
  /// **'API origin server unreachable'**
  String get apiErrorOriginTitle;

  /// No description provided for @apiErrorOriginMessage.
  ///
  /// In en, this message translates to:
  /// **'The gateway could not connect to the origin server. Contact the API provider to check for outages or routing problems.'**
  String get apiErrorOriginMessage;

  /// No description provided for @apiErrorTlsTitle.
  ///
  /// In en, this message translates to:
  /// **'API server TLS or certificate error'**
  String get apiErrorTlsTitle;

  /// No description provided for @apiErrorTlsMessage.
  ///
  /// In en, this message translates to:
  /// **'TLS negotiation or certificate validation failed between the gateway and the origin server. Contact the API provider.'**
  String get apiErrorTlsMessage;

  /// No description provided for @apiErrorStorageTitle.
  ///
  /// In en, this message translates to:
  /// **'API server storage exhausted'**
  String get apiErrorStorageTitle;

  /// No description provided for @apiErrorStorageMessage.
  ///
  /// In en, this message translates to:
  /// **'The API server has insufficient storage to complete the request. Contact the provider.'**
  String get apiErrorStorageMessage;

  /// No description provided for @apiErrorClientTitle.
  ///
  /// In en, this message translates to:
  /// **'API server refused the request'**
  String get apiErrorClientTitle;

  /// No description provided for @apiErrorClientMessage.
  ///
  /// In en, this message translates to:
  /// **'The API server returned a request error. Review the original error and check your API configuration.'**
  String get apiErrorClientMessage;

  /// No description provided for @apiErrorServerTitle.
  ///
  /// In en, this message translates to:
  /// **'API server error'**
  String get apiErrorServerTitle;

  /// No description provided for @apiErrorServerMessage.
  ///
  /// In en, this message translates to:
  /// **'The server could not complete the request. Send the status code and original error to the provider.'**
  String get apiErrorServerMessage;

  /// No description provided for @apiErrorHttpTitle.
  ///
  /// In en, this message translates to:
  /// **'HTTP {status}: {title}'**
  String apiErrorHttpTitle(int status, String title);

  /// No description provided for @apiErrorOriginal.
  ///
  /// In en, this message translates to:
  /// **'Original error:\n{error}'**
  String apiErrorOriginal(String error);

  /// No description provided for @apiErrorModelExistsTitle.
  ///
  /// In en, this message translates to:
  /// **'Model already exists'**
  String get apiErrorModelExistsTitle;

  /// No description provided for @apiErrorModelListTitle.
  ///
  /// In en, this message translates to:
  /// **'Could not fetch models'**
  String get apiErrorModelListTitle;

  /// No description provided for @apiErrorModelListMessage.
  ///
  /// In en, this message translates to:
  /// **'No model list was received. Check the provider address, access key, and network connection.'**
  String get apiErrorModelListMessage;

  /// No description provided for @apiErrorNetworkTitle.
  ///
  /// In en, this message translates to:
  /// **'Network connection failed'**
  String get apiErrorNetworkTitle;

  /// No description provided for @apiErrorNetworkMessage.
  ///
  /// In en, this message translates to:
  /// **'Could not reach the model provider. Check the network connection and server address.'**
  String get apiErrorNetworkMessage;

  /// No description provided for @apiErrorModelConfigTitle.
  ///
  /// In en, this message translates to:
  /// **'Model configuration failed'**
  String get apiErrorModelConfigTitle;

  /// No description provided for @apiErrorModelConfigMessage.
  ///
  /// In en, this message translates to:
  /// **'The model configuration operation failed. Review the original error for details.'**
  String get apiErrorModelConfigMessage;

  /// No description provided for @apiErrorModelExistsMessage.
  ///
  /// In en, this message translates to:
  /// **'Model “{modelId}” has already been added to provider “{providerName}”.'**
  String apiErrorModelExistsMessage(String modelId, String providerName);

  /// No description provided for @chatCopyMessage.
  ///
  /// In en, this message translates to:
  /// **'Copy message'**
  String get chatCopyMessage;

  /// No description provided for @chatEditResend.
  ///
  /// In en, this message translates to:
  /// **'Edit and resend'**
  String get chatEditResend;

  /// No description provided for @chatRollbackHere.
  ///
  /// In en, this message translates to:
  /// **'Roll back to here'**
  String get chatRollbackHere;

  /// No description provided for @chatRegenerate.
  ///
  /// In en, this message translates to:
  /// **'Regenerate'**
  String get chatRegenerate;

  /// No description provided for @chatEditMemory.
  ///
  /// In en, this message translates to:
  /// **'Edit memory'**
  String get chatEditMemory;

  /// No description provided for @chatPlaySpeech.
  ///
  /// In en, this message translates to:
  /// **'Generate or play speech'**
  String get chatPlaySpeech;

  /// No description provided for @chatDeleteVariant.
  ///
  /// In en, this message translates to:
  /// **'Delete current variant'**
  String get chatDeleteVariant;

  /// No description provided for @chatReply.
  ///
  /// In en, this message translates to:
  /// **'Reply'**
  String get chatReply;

  /// No description provided for @chatInsertSummary.
  ///
  /// In en, this message translates to:
  /// **'Insert summary'**
  String get chatInsertSummary;

  /// No description provided for @chatCreateBranch.
  ///
  /// In en, this message translates to:
  /// **'Create branch'**
  String get chatCreateBranch;

  /// No description provided for @chatMultiSelect.
  ///
  /// In en, this message translates to:
  /// **'Select messages'**
  String get chatMultiSelect;

  /// No description provided for @chatMultiSelectExit.
  ///
  /// In en, this message translates to:
  /// **'Exit selection'**
  String get chatMultiSelectExit;

  /// No description provided for @chatSelectionClear.
  ///
  /// In en, this message translates to:
  /// **'Clear selection'**
  String get chatSelectionClear;

  /// No description provided for @chatSelectionAll.
  ///
  /// In en, this message translates to:
  /// **'Select all'**
  String get chatSelectionAll;

  /// No description provided for @chatSelectionCopy.
  ///
  /// In en, this message translates to:
  /// **'Copy selected messages'**
  String get chatSelectionCopy;

  /// No description provided for @chatGenerateShareImage.
  ///
  /// In en, this message translates to:
  /// **'Create share image'**
  String get chatGenerateShareImage;

  /// No description provided for @chatSelectionDelete.
  ///
  /// In en, this message translates to:
  /// **'Delete selected messages'**
  String get chatSelectionDelete;

  /// No description provided for @chatEditMessage.
  ///
  /// In en, this message translates to:
  /// **'Edit message'**
  String get chatEditMessage;

  /// No description provided for @chatEditorVisual.
  ///
  /// In en, this message translates to:
  /// **'Visual'**
  String get chatEditorVisual;

  /// No description provided for @chatPlainText.
  ///
  /// In en, this message translates to:
  /// **'Plain text'**
  String get chatPlainText;

  /// No description provided for @chatMarkdownSource.
  ///
  /// In en, this message translates to:
  /// **'Markdown source'**
  String get chatMarkdownSource;

  /// No description provided for @chatCopyPlainText.
  ///
  /// In en, this message translates to:
  /// **'Copy plain text'**
  String get chatCopyPlainText;

  /// No description provided for @chatCopyMarkdown.
  ///
  /// In en, this message translates to:
  /// **'Copy Markdown source'**
  String get chatCopyMarkdown;

  /// No description provided for @chatMessageCopied.
  ///
  /// In en, this message translates to:
  /// **'Message copied to clipboard'**
  String get chatMessageCopied;

  /// No description provided for @chatSaveResend.
  ///
  /// In en, this message translates to:
  /// **'Save and resend'**
  String get chatSaveResend;

  /// No description provided for @chatUpdateMemory.
  ///
  /// In en, this message translates to:
  /// **'Update memory'**
  String get chatUpdateMemory;

  /// No description provided for @chatEditorPlainTextContent.
  ///
  /// In en, this message translates to:
  /// **'Plain text content'**
  String get chatEditorPlainTextContent;

  /// No description provided for @chatEditorParts.
  ///
  /// In en, this message translates to:
  /// **'Content blocks'**
  String get chatEditorParts;

  /// No description provided for @chatEditorAddText.
  ///
  /// In en, this message translates to:
  /// **'Add text'**
  String get chatEditorAddText;

  /// No description provided for @chatEditorAddTag.
  ///
  /// In en, this message translates to:
  /// **'Add tag'**
  String get chatEditorAddTag;

  /// No description provided for @chatEditorText.
  ///
  /// In en, this message translates to:
  /// **'Text'**
  String get chatEditorText;

  /// No description provided for @chatEditorTextHint.
  ///
  /// In en, this message translates to:
  /// **'Enter text'**
  String get chatEditorTextHint;

  /// No description provided for @chatEditorEditTag.
  ///
  /// In en, this message translates to:
  /// **'Edit tag'**
  String get chatEditorEditTag;

  /// No description provided for @chatEditorTagName.
  ///
  /// In en, this message translates to:
  /// **'Tag name'**
  String get chatEditorTagName;

  /// No description provided for @chatEditorTagHint.
  ///
  /// In en, this message translates to:
  /// **'For example, custom_tag'**
  String get chatEditorTagHint;

  /// No description provided for @chatEditorAttributes.
  ///
  /// In en, this message translates to:
  /// **'Attributes (optional)'**
  String get chatEditorAttributes;

  /// No description provided for @chatEditorAttributesHint.
  ///
  /// In en, this message translates to:
  /// **'For example, type=\"note\"'**
  String get chatEditorAttributesHint;

  /// No description provided for @chatEditorContent.
  ///
  /// In en, this message translates to:
  /// **'Content'**
  String get chatEditorContent;

  /// No description provided for @chatRollbackConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Roll back the workspace?'**
  String get chatRollbackConfirmTitle;

  /// No description provided for @chatResendConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Edit and resend the message?'**
  String get chatResendConfirmTitle;

  /// No description provided for @chatRollbackConfirmMessage.
  ///
  /// In en, this message translates to:
  /// **'This restores the workspace to its state before this message and deletes this and all later messages.'**
  String get chatRollbackConfirmMessage;

  /// No description provided for @chatResendConfirmMessage.
  ///
  /// In en, this message translates to:
  /// **'This restores the workspace to its state before this message and sends the edited message again.'**
  String get chatResendConfirmMessage;

  /// No description provided for @chatRollbackConfirm.
  ///
  /// In en, this message translates to:
  /// **'Roll back'**
  String get chatRollbackConfirm;

  /// No description provided for @chatNoWorkspaceChanges.
  ///
  /// In en, this message translates to:
  /// **'No workspace file changes detected.'**
  String get chatNoWorkspaceChanges;

  /// No description provided for @chatShareImageGenerating.
  ///
  /// In en, this message translates to:
  /// **'Creating share image…'**
  String get chatShareImageGenerating;

  /// No description provided for @chatSelectionCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{count} message selected} other{{count} messages selected}}'**
  String chatSelectionCount(int count);

  /// No description provided for @chatSelectionDeleteConfirm.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{Delete the selected message?} other{Delete the {count} selected messages?}}'**
  String chatSelectionDeleteConfirm(int count);

  /// No description provided for @chatCopyFailed.
  ///
  /// In en, this message translates to:
  /// **'Copy failed: {error}'**
  String chatCopyFailed(String error);

  /// No description provided for @chatPlainTextConversionFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not convert to plain text: {error}'**
  String chatPlainTextConversionFailed(String error);

  /// No description provided for @chatPlaySpeechFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not generate or play speech: {error}'**
  String chatPlaySpeechFailed(String error);

  /// No description provided for @chatShareImageFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not create the share image: {error}'**
  String chatShareImageFailed(String error);

  /// No description provided for @chatWorkspaceChangeLines.
  ///
  /// In en, this message translates to:
  /// **'{change} · {count, plural, one{{count} line} other{{count} lines}}'**
  String chatWorkspaceChangeLines(String change, int count);

  /// No description provided for @speechTitle.
  ///
  /// In en, this message translates to:
  /// **'Read aloud'**
  String get speechTitle;

  /// No description provided for @speechIdle.
  ///
  /// In en, this message translates to:
  /// **'Idle'**
  String get speechIdle;

  /// No description provided for @speechPreparing.
  ///
  /// In en, this message translates to:
  /// **'Generating speech'**
  String get speechPreparing;

  /// No description provided for @speechPlaying.
  ///
  /// In en, this message translates to:
  /// **'Playing'**
  String get speechPlaying;

  /// No description provided for @speechStopped.
  ///
  /// In en, this message translates to:
  /// **'Stopped'**
  String get speechStopped;

  /// No description provided for @speechError.
  ///
  /// In en, this message translates to:
  /// **'Error'**
  String get speechError;

  /// No description provided for @speechQueue.
  ///
  /// In en, this message translates to:
  /// **'Queue: {count}'**
  String speechQueue(int count);

  /// No description provided for @speechTestText.
  ///
  /// In en, this message translates to:
  /// **'Hello! This is the Operit voice preview.'**
  String get speechTestText;

  /// No description provided for @speechTtsMissingId.
  ///
  /// In en, this message translates to:
  /// **'Cannot preview without a TTS configuration ID'**
  String get speechTtsMissingId;

  /// No description provided for @speechTtsPreviewPlaying.
  ///
  /// In en, this message translates to:
  /// **'TTS preview started'**
  String get speechTtsPreviewPlaying;

  /// No description provided for @speechTtsProviders.
  ///
  /// In en, this message translates to:
  /// **'TTS providers'**
  String get speechTtsProviders;

  /// No description provided for @speechAddTtsProvider.
  ///
  /// In en, this message translates to:
  /// **'Add TTS provider'**
  String get speechAddTtsProvider;

  /// No description provided for @speechSttCurrentCannotDelete.
  ///
  /// In en, this message translates to:
  /// **'The active STT configuration cannot be deleted'**
  String get speechSttCurrentCannotDelete;

  /// No description provided for @speechDeleteSttProvider.
  ///
  /// In en, this message translates to:
  /// **'Delete STT provider'**
  String get speechDeleteSttProvider;

  /// No description provided for @speechSttProviders.
  ///
  /// In en, this message translates to:
  /// **'STT providers'**
  String get speechSttProviders;

  /// No description provided for @speechAddSttProvider.
  ///
  /// In en, this message translates to:
  /// **'Add STT provider'**
  String get speechAddSttProvider;

  /// No description provided for @speechEditProvider.
  ///
  /// In en, this message translates to:
  /// **'Edit provider'**
  String get speechEditProvider;

  /// No description provided for @speechPreview.
  ///
  /// In en, this message translates to:
  /// **'Preview'**
  String get speechPreview;

  /// No description provided for @speechGlobalActive.
  ///
  /// In en, this message translates to:
  /// **'Global default'**
  String get speechGlobalActive;

  /// No description provided for @speechSetGlobal.
  ///
  /// In en, this message translates to:
  /// **'Set as global default'**
  String get speechSetGlobal;

  /// No description provided for @speechNoSttProviders.
  ///
  /// In en, this message translates to:
  /// **'No speech recognition providers configured'**
  String get speechNoSttProviders;

  /// No description provided for @speechCurrentCannotDelete.
  ///
  /// In en, this message translates to:
  /// **'The active configuration cannot be deleted'**
  String get speechCurrentCannotDelete;

  /// No description provided for @speechEditTtsProvider.
  ///
  /// In en, this message translates to:
  /// **'Edit TTS provider'**
  String get speechEditTtsProvider;

  /// No description provided for @speechProviderType.
  ///
  /// In en, this message translates to:
  /// **'Provider type'**
  String get speechProviderType;

  /// No description provided for @speechProviderName.
  ///
  /// In en, this message translates to:
  /// **'Provider name'**
  String get speechProviderName;

  /// No description provided for @speechHttpMethod.
  ///
  /// In en, this message translates to:
  /// **'HTTP method'**
  String get speechHttpMethod;

  /// No description provided for @speechBodyTemplate.
  ///
  /// In en, this message translates to:
  /// **'Request body template'**
  String get speechBodyTemplate;

  /// No description provided for @speechPipelineJson.
  ///
  /// In en, this message translates to:
  /// **'Response pipeline JSON'**
  String get speechPipelineJson;

  /// No description provided for @speechAddTtsVoice.
  ///
  /// In en, this message translates to:
  /// **'Add TTS voice'**
  String get speechAddTtsVoice;

  /// No description provided for @speechCustomVoice.
  ///
  /// In en, this message translates to:
  /// **'Custom voice'**
  String get speechCustomVoice;

  /// No description provided for @speechEnterVoice.
  ///
  /// In en, this message translates to:
  /// **'Enter the model and voice manually'**
  String get speechEnterVoice;

  /// No description provided for @speechModelVoiceRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter a model or a voice'**
  String get speechModelVoiceRequired;

  /// No description provided for @speechCustomTtsVoice.
  ///
  /// In en, this message translates to:
  /// **'Custom TTS voice'**
  String get speechCustomTtsVoice;

  /// No description provided for @speechModelRequired.
  ///
  /// In en, this message translates to:
  /// **'Model is required'**
  String get speechModelRequired;

  /// No description provided for @speechVoice.
  ///
  /// In en, this message translates to:
  /// **'Voice'**
  String get speechVoice;

  /// No description provided for @speechVoiceRequired.
  ///
  /// In en, this message translates to:
  /// **'Voice is required'**
  String get speechVoiceRequired;

  /// No description provided for @speechLanguageTag.
  ///
  /// In en, this message translates to:
  /// **'Language tag (optional, e.g. en-US)'**
  String get speechLanguageTag;

  /// No description provided for @speechModelLocale.
  ///
  /// In en, this message translates to:
  /// **'Model / locale'**
  String get speechModelLocale;

  /// No description provided for @speechSystemVoiceName.
  ///
  /// In en, this message translates to:
  /// **'System voice name (optional)'**
  String get speechSystemVoiceName;

  /// No description provided for @speechEditTtsVoice.
  ///
  /// In en, this message translates to:
  /// **'Edit TTS voice'**
  String get speechEditTtsVoice;

  /// No description provided for @speechTestVoice.
  ///
  /// In en, this message translates to:
  /// **'Test voice'**
  String get speechTestVoice;

  /// No description provided for @speechAudioFormat.
  ///
  /// In en, this message translates to:
  /// **'Audio format'**
  String get speechAudioFormat;

  /// No description provided for @speechSpeed.
  ///
  /// In en, this message translates to:
  /// **'Speed'**
  String get speechSpeed;

  /// No description provided for @speechNewTtsProvider.
  ///
  /// In en, this message translates to:
  /// **'New TTS provider'**
  String get speechNewTtsProvider;

  /// No description provided for @speechModelVersion.
  ///
  /// In en, this message translates to:
  /// **'Model ID@version'**
  String get speechModelVersion;

  /// No description provided for @speechNewSttProvider.
  ///
  /// In en, this message translates to:
  /// **'New STT provider'**
  String get speechNewSttProvider;

  /// No description provided for @speechEditSttProvider.
  ///
  /// In en, this message translates to:
  /// **'Edit STT provider'**
  String get speechEditSttProvider;

  /// No description provided for @speechMultipart.
  ///
  /// In en, this message translates to:
  /// **'Multipart request settings'**
  String get speechMultipart;

  /// No description provided for @speechAudioField.
  ///
  /// In en, this message translates to:
  /// **'Audio field name'**
  String get speechAudioField;

  /// No description provided for @speechModelField.
  ///
  /// In en, this message translates to:
  /// **'Model field name'**
  String get speechModelField;

  /// No description provided for @speechModelFieldHelp.
  ///
  /// In en, this message translates to:
  /// **'Leave blank if the endpoint does not accept a model parameter'**
  String get speechModelFieldHelp;

  /// No description provided for @speechLanguageField.
  ///
  /// In en, this message translates to:
  /// **'Language field name'**
  String get speechLanguageField;

  /// No description provided for @speechTranscriptJsonPath.
  ///
  /// In en, this message translates to:
  /// **'Transcript JSONPath'**
  String get speechTranscriptJsonPath;

  /// No description provided for @speechModelNotNeeded.
  ///
  /// In en, this message translates to:
  /// **'This endpoint does not require a model'**
  String get speechModelNotNeeded;

  /// No description provided for @speechReloadModels.
  ///
  /// In en, this message translates to:
  /// **'Reload models'**
  String get speechReloadModels;

  /// No description provided for @speechModelIdHelp.
  ///
  /// In en, this message translates to:
  /// **'Enter a supported model ID or select a preset'**
  String get speechModelIdHelp;

  /// No description provided for @speechPresetModel.
  ///
  /// In en, this message translates to:
  /// **'Select preset model'**
  String get speechPresetModel;

  /// No description provided for @speechNoLocalSttModels.
  ///
  /// In en, this message translates to:
  /// **'No local STT models installed'**
  String get speechNoLocalSttModels;

  /// No description provided for @speechChooseModel.
  ///
  /// In en, this message translates to:
  /// **'Select a model'**
  String get speechChooseModel;

  /// No description provided for @speechChooseSttModel.
  ///
  /// In en, this message translates to:
  /// **'Select an STT model'**
  String get speechChooseSttModel;

  /// No description provided for @speechPreviewTitle.
  ///
  /// In en, this message translates to:
  /// **'Preview · {voice}'**
  String speechPreviewTitle(String voice);

  /// No description provided for @speechPreviewFailed.
  ///
  /// In en, this message translates to:
  /// **'TTS preview failed: {error}'**
  String speechPreviewFailed(String error);

  /// No description provided for @speechDeleteTtsConfigFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not delete TTS configuration: {error}'**
  String speechDeleteTtsConfigFailed(String error);

  /// No description provided for @speechLoadConfigFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not load speech settings: {error}'**
  String speechLoadConfigFailed(String error);

  /// No description provided for @speechDeleteSttConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete “{name}” and its recognition settings?'**
  String speechDeleteSttConfirm(String name);

  /// No description provided for @speechSttFailed.
  ///
  /// In en, this message translates to:
  /// **'STT operation failed: {error}'**
  String speechSttFailed(String error);

  /// No description provided for @speechModelsFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not load models: {error}'**
  String speechModelsFailed(String error);

  /// No description provided for @speechFieldRequired.
  ///
  /// In en, this message translates to:
  /// **'{label} is required'**
  String speechFieldRequired(String label);

  /// No description provided for @speechFieldPositive.
  ///
  /// In en, this message translates to:
  /// **'{label} must be positive'**
  String speechFieldPositive(String label);

  /// No description provided for @onboardingStorageRequired.
  ///
  /// In en, this message translates to:
  /// **'Both the runtime and workspace directories are required'**
  String get onboardingStorageRequired;

  /// No description provided for @onboardingFetchModelsFirst.
  ///
  /// In en, this message translates to:
  /// **'Fetch available models first'**
  String get onboardingFetchModelsFirst;

  /// No description provided for @onboardingChooseDefaultModel.
  ///
  /// In en, this message translates to:
  /// **'Select a default model'**
  String get onboardingChooseDefaultModel;

  /// No description provided for @onboardingPickSnapshot.
  ///
  /// In en, this message translates to:
  /// **'Choose snapshot'**
  String get onboardingPickSnapshot;

  /// No description provided for @onboardingPickSnapshotHelp.
  ///
  /// In en, this message translates to:
  /// **'Select an Operit1 snapshot in the file picker.'**
  String get onboardingPickSnapshotHelp;

  /// No description provided for @onboardingUploadSnapshot.
  ///
  /// In en, this message translates to:
  /// **'Upload snapshot'**
  String get onboardingUploadSnapshot;

  /// No description provided for @onboardingInspectSnapshot.
  ///
  /// In en, this message translates to:
  /// **'Inspect snapshot'**
  String get onboardingInspectSnapshot;

  /// No description provided for @onboardingInspectSnapshotHelp.
  ///
  /// In en, this message translates to:
  /// **'Inspecting the contents of the Operit1 snapshot…'**
  String get onboardingInspectSnapshotHelp;

  /// No description provided for @onboardingSnapshotRequired.
  ///
  /// In en, this message translates to:
  /// **'Select an Operit1 snapshot file'**
  String get onboardingSnapshotRequired;

  /// No description provided for @onboardingPrepareImport.
  ///
  /// In en, this message translates to:
  /// **'Prepare import'**
  String get onboardingPrepareImport;

  /// No description provided for @onboardingPrepareMigration.
  ///
  /// In en, this message translates to:
  /// **'Preparing to migrate the contents of the Operit1 snapshot…'**
  String get onboardingPrepareMigration;

  /// No description provided for @onboardingWait.
  ///
  /// In en, this message translates to:
  /// **'Please wait'**
  String get onboardingWait;

  /// No description provided for @onboardingAgree.
  ///
  /// In en, this message translates to:
  /// **'Agree'**
  String get onboardingAgree;

  /// No description provided for @onboardingSaving.
  ///
  /// In en, this message translates to:
  /// **'Saving…'**
  String get onboardingSaving;

  /// No description provided for @onboardingConfirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm'**
  String get onboardingConfirm;

  /// No description provided for @onboardingPreparing.
  ///
  /// In en, this message translates to:
  /// **'Preparing…'**
  String get onboardingPreparing;

  /// No description provided for @onboardingContinue.
  ///
  /// In en, this message translates to:
  /// **'Continue'**
  String get onboardingContinue;

  /// No description provided for @onboardingFetching.
  ///
  /// In en, this message translates to:
  /// **'Fetching…'**
  String get onboardingFetching;

  /// No description provided for @onboardingReading.
  ///
  /// In en, this message translates to:
  /// **'Reading…'**
  String get onboardingReading;

  /// No description provided for @onboardingImporting.
  ///
  /// In en, this message translates to:
  /// **'Importing…'**
  String get onboardingImporting;

  /// No description provided for @onboardingWorking.
  ///
  /// In en, this message translates to:
  /// **'Working…'**
  String get onboardingWorking;

  /// No description provided for @onboardingDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get onboardingDone;

  /// No description provided for @onboardingAgreement.
  ///
  /// In en, this message translates to:
  /// **'User agreement'**
  String get onboardingAgreement;

  /// No description provided for @onboardingStorage.
  ///
  /// In en, this message translates to:
  /// **'Storage locations'**
  String get onboardingStorage;

  /// No description provided for @onboardingMode.
  ///
  /// In en, this message translates to:
  /// **'Setup method'**
  String get onboardingMode;

  /// No description provided for @onboardingModelSetup.
  ///
  /// In en, this message translates to:
  /// **'Model setup'**
  String get onboardingModelSetup;

  /// No description provided for @onboardingImportSetup.
  ///
  /// In en, this message translates to:
  /// **'Import settings'**
  String get onboardingImportSetup;

  /// No description provided for @onboardingDeviceSpace.
  ///
  /// In en, this message translates to:
  /// **'Device space'**
  String get onboardingDeviceSpace;

  /// No description provided for @onboardingPermissions.
  ///
  /// In en, this message translates to:
  /// **'System permissions'**
  String get onboardingPermissions;

  /// No description provided for @onboardingWelcome.
  ///
  /// In en, this message translates to:
  /// **'Welcome'**
  String get onboardingWelcome;

  /// No description provided for @onboardingPreparingRuntime.
  ///
  /// In en, this message translates to:
  /// **'Preparing the local runtime'**
  String get onboardingPreparingRuntime;

  /// No description provided for @onboardingSkip.
  ///
  /// In en, this message translates to:
  /// **'Skip'**
  String get onboardingSkip;

  /// No description provided for @onboardingPreviousPage.
  ///
  /// In en, this message translates to:
  /// **'Previous page'**
  String get onboardingPreviousPage;

  /// No description provided for @onboardingStart.
  ///
  /// In en, this message translates to:
  /// **'Start'**
  String get onboardingStart;

  /// No description provided for @onboardingTagline.
  ///
  /// In en, this message translates to:
  /// **'Make everyday tasks easier'**
  String get onboardingTagline;

  /// No description provided for @onboardingChooseMode.
  ///
  /// In en, this message translates to:
  /// **'Choose how to get started'**
  String get onboardingChooseMode;

  /// No description provided for @onboardingChooseModeHelp.
  ///
  /// In en, this message translates to:
  /// **'Set up a new environment, import Operit1 data, join an existing device space, or restore chats from a backup.'**
  String get onboardingChooseModeHelp;

  /// No description provided for @onboardingQuickStart.
  ///
  /// In en, this message translates to:
  /// **'Quick start'**
  String get onboardingQuickStart;

  /// No description provided for @onboardingQuickStartHelp.
  ///
  /// In en, this message translates to:
  /// **'Configure a model provider to complete basic setup'**
  String get onboardingQuickStartHelp;

  /// No description provided for @onboardingImportLegacyHelp.
  ///
  /// In en, this message translates to:
  /// **'Import settings and data from the previous version'**
  String get onboardingImportLegacyHelp;

  /// No description provided for @onboardingJoinExisting.
  ///
  /// In en, this message translates to:
  /// **'Join an existing device space'**
  String get onboardingJoinExisting;

  /// No description provided for @onboardingJoinExistingHelp.
  ///
  /// In en, this message translates to:
  /// **'Find a nearby device space and sync models and data after joining'**
  String get onboardingJoinExistingHelp;

  /// No description provided for @onboardingJoinSpace.
  ///
  /// In en, this message translates to:
  /// **'Join a device space'**
  String get onboardingJoinSpace;

  /// No description provided for @onboardingJoinSpaceHelp.
  ///
  /// In en, this message translates to:
  /// **'Select Add device, choose a nearby device, and pair with it. Settings and data sync after an administrator approves the request.'**
  String get onboardingJoinSpaceHelp;

  /// No description provided for @onboardingConfirmStorage.
  ///
  /// In en, this message translates to:
  /// **'Confirm local storage locations'**
  String get onboardingConfirmStorage;

  /// No description provided for @onboardingConfirmStorageHelp.
  ///
  /// In en, this message translates to:
  /// **'Runtime data and workspace data are stored separately. Enter paths or choose each directory.'**
  String get onboardingConfirmStorageHelp;

  /// No description provided for @onboardingRuntimeDirectory.
  ///
  /// In en, this message translates to:
  /// **'Runtime directory'**
  String get onboardingRuntimeDirectory;

  /// No description provided for @onboardingWorkspaceDirectory.
  ///
  /// In en, this message translates to:
  /// **'Workspace directory'**
  String get onboardingWorkspaceDirectory;

  /// No description provided for @onboardingReadingPaths.
  ///
  /// In en, this message translates to:
  /// **'Reading storage paths'**
  String get onboardingReadingPaths;

  /// No description provided for @onboardingStorageMountHelp.
  ///
  /// In en, this message translates to:
  /// **'These directories store runtime state and user workspaces separately. The local host mounts them after you confirm.'**
  String get onboardingStorageMountHelp;

  /// No description provided for @onboardingPrepareSnapshotHelp.
  ///
  /// In en, this message translates to:
  /// **'Preparing the Operit1 snapshot…'**
  String get onboardingPrepareSnapshotHelp;

  /// No description provided for @onboardingImportSnapshotHelp.
  ///
  /// In en, this message translates to:
  /// **'Use a snapshot exported by the latest version of Operit1 to migrate settings, chats, character cards, and resources to Operit2.'**
  String get onboardingImportSnapshotHelp;

  /// No description provided for @onboardingReadingSnapshot.
  ///
  /// In en, this message translates to:
  /// **'Reading snapshot'**
  String get onboardingReadingSnapshot;

  /// No description provided for @onboardingPickSnapshotFile.
  ///
  /// In en, this message translates to:
  /// **'Choose snapshot file'**
  String get onboardingPickSnapshotFile;

  /// No description provided for @onboardingMigrationContents.
  ///
  /// In en, this message translates to:
  /// **'Data available to migrate'**
  String get onboardingMigrationContents;

  /// No description provided for @onboardingChats.
  ///
  /// In en, this message translates to:
  /// **'Chats'**
  String get onboardingChats;

  /// No description provided for @onboardingMessages.
  ///
  /// In en, this message translates to:
  /// **'Messages'**
  String get onboardingMessages;

  /// No description provided for @onboardingPreferenceFiles.
  ///
  /// In en, this message translates to:
  /// **'Preference files'**
  String get onboardingPreferenceFiles;

  /// No description provided for @onboardingResourceFiles.
  ///
  /// In en, this message translates to:
  /// **'Resource files'**
  String get onboardingResourceFiles;

  /// No description provided for @onboardingExternalResources.
  ///
  /// In en, this message translates to:
  /// **'External resources'**
  String get onboardingExternalResources;

  /// No description provided for @onboardingImportInProgress.
  ///
  /// In en, this message translates to:
  /// **'Importing the snapshot…'**
  String get onboardingImportInProgress;

  /// No description provided for @onboardingImportContinueHelp.
  ///
  /// In en, this message translates to:
  /// **'Select Continue to migrate the entire snapshot.'**
  String get onboardingImportContinueHelp;

  /// No description provided for @onboardingEndpoint.
  ///
  /// In en, this message translates to:
  /// **'Server address'**
  String get onboardingEndpoint;

  /// No description provided for @onboardingCompleteModel.
  ///
  /// In en, this message translates to:
  /// **'Complete model setup'**
  String get onboardingCompleteModel;

  /// No description provided for @onboardingCompleteModelHelp.
  ///
  /// In en, this message translates to:
  /// **'Select a model provider, enter your API key, and fetch and select a default model.'**
  String get onboardingCompleteModelHelp;

  /// No description provided for @onboardingModelProvider.
  ///
  /// In en, this message translates to:
  /// **'Model provider'**
  String get onboardingModelProvider;

  /// No description provided for @onboardingModelProviderRequired.
  ///
  /// In en, this message translates to:
  /// **'Select a model provider'**
  String get onboardingModelProviderRequired;

  /// No description provided for @onboardingContinueModel.
  ///
  /// In en, this message translates to:
  /// **'Continue setup'**
  String get onboardingContinueModel;

  /// No description provided for @onboardingFetchingModels.
  ///
  /// In en, this message translates to:
  /// **'Fetching models'**
  String get onboardingFetchingModels;

  /// No description provided for @onboardingFetchModels.
  ///
  /// In en, this message translates to:
  /// **'Fetch available models'**
  String get onboardingFetchModels;

  /// No description provided for @onboardingDefaultModel.
  ///
  /// In en, this message translates to:
  /// **'Default model'**
  String get onboardingDefaultModel;

  /// No description provided for @onboardingGrantPermissions.
  ///
  /// In en, this message translates to:
  /// **'Grant system permissions as needed'**
  String get onboardingGrantPermissions;

  /// No description provided for @onboardingGrantPermissionsHelp.
  ///
  /// In en, this message translates to:
  /// **'Grant permissions for the features you want to use. You can continue without granting any permissions. Features that require them will be unavailable.'**
  String get onboardingGrantPermissionsHelp;

  /// No description provided for @onboardingNoPermissions.
  ///
  /// In en, this message translates to:
  /// **'No permissions to review on this device'**
  String get onboardingNoPermissions;

  /// No description provided for @onboardingNoPermissionsHelp.
  ///
  /// In en, this message translates to:
  /// **'This environment has no system permissions to configure during setup.'**
  String get onboardingNoPermissionsHelp;

  /// No description provided for @onboardingRefreshPermissions.
  ///
  /// In en, this message translates to:
  /// **'Refresh permissions'**
  String get onboardingRefreshPermissions;

  /// No description provided for @onboardingOptional.
  ///
  /// In en, this message translates to:
  /// **'Optional'**
  String get onboardingOptional;

  /// No description provided for @onboardingRequired.
  ///
  /// In en, this message translates to:
  /// **'Required'**
  String get onboardingRequired;

  /// No description provided for @onboardingPermissionGranted.
  ///
  /// In en, this message translates to:
  /// **'Granted'**
  String get onboardingPermissionGranted;

  /// No description provided for @onboardingGrantPermission.
  ///
  /// In en, this message translates to:
  /// **'Grant permission'**
  String get onboardingGrantPermission;

  /// No description provided for @onboardingNoAction.
  ///
  /// In en, this message translates to:
  /// **'No action needed'**
  String get onboardingNoAction;

  /// No description provided for @onboardingUploadProgress.
  ///
  /// In en, this message translates to:
  /// **'Reading and uploading the snapshot ({percent}%).'**
  String onboardingUploadProgress(int percent);

  /// No description provided for @onboardingChooseDirectory.
  ///
  /// In en, this message translates to:
  /// **'Choose {label}'**
  String onboardingChooseDirectory(String label);

  /// No description provided for @onboardingDefaultChatModel.
  ///
  /// In en, this message translates to:
  /// **'Default chat model: {model}'**
  String onboardingDefaultChatModel(String model);

  /// No description provided for @onboardingImportFailed.
  ///
  /// In en, this message translates to:
  /// **'Snapshot import failed\n{error}'**
  String onboardingImportFailed(String error);

  /// No description provided for @onboardingAgreementTitle.
  ///
  /// In en, this message translates to:
  /// **'User Agreement and Privacy Policy'**
  String get onboardingAgreementTitle;

  /// No description provided for @onboardingAgreementHelp.
  ///
  /// In en, this message translates to:
  /// **'Read and accept this agreement to continue setup.'**
  String get onboardingAgreementHelp;

  /// No description provided for @onboardingAgreementVersion.
  ///
  /// In en, this message translates to:
  /// **'Version: {version}'**
  String onboardingAgreementVersion(String version);

  /// No description provided for @onboardingAgreementWait.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{You can accept in {count} second.} other{You can accept in {count} seconds.}}'**
  String onboardingAgreementWait(int count);

  /// No description provided for @extensionsView.
  ///
  /// In en, this message translates to:
  /// **'View'**
  String get extensionsView;

  /// No description provided for @extensionsCloseSearch.
  ///
  /// In en, this message translates to:
  /// **'Close search'**
  String get extensionsCloseSearch;

  /// No description provided for @extensionsSearchShortcut.
  ///
  /// In en, this message translates to:
  /// **'Search (Ctrl+F)'**
  String get extensionsSearchShortcut;

  /// No description provided for @extensionsAdd.
  ///
  /// In en, this message translates to:
  /// **'Add extension'**
  String get extensionsAdd;

  /// No description provided for @extensionsMore.
  ///
  /// In en, this message translates to:
  /// **'More'**
  String get extensionsMore;

  /// No description provided for @extensionsCheckUpdates.
  ///
  /// In en, this message translates to:
  /// **'Check for updates'**
  String get extensionsCheckUpdates;

  /// No description provided for @extensionsEnvironment.
  ///
  /// In en, this message translates to:
  /// **'Environment variables'**
  String get extensionsEnvironment;

  /// No description provided for @extensionsReload.
  ///
  /// In en, this message translates to:
  /// **'Reload catalog'**
  String get extensionsReload;

  /// No description provided for @extensionsSearchInstalled.
  ///
  /// In en, this message translates to:
  /// **'Search installed extensions'**
  String get extensionsSearchInstalled;

  /// No description provided for @extensionsSearchMarket.
  ///
  /// In en, this message translates to:
  /// **'Search the market and official extensions'**
  String get extensionsSearchMarket;

  /// No description provided for @extensionsTab.
  ///
  /// In en, this message translates to:
  /// **'Extensions'**
  String get extensionsTab;

  /// No description provided for @extensionsDrop.
  ///
  /// In en, this message translates to:
  /// **'Drop to import extensions'**
  String get extensionsDrop;

  /// No description provided for @extensionsDropTypes.
  ///
  /// In en, this message translates to:
  /// **'Supports .toolpkg, .js, .ts, and .hjson'**
  String get extensionsDropTypes;

  /// No description provided for @extensionsImportPlugin.
  ///
  /// In en, this message translates to:
  /// **'Import plugin'**
  String get extensionsImportPlugin;

  /// No description provided for @extensionsSelectToolPkg.
  ///
  /// In en, this message translates to:
  /// **'Choose a .toolpkg file'**
  String get extensionsSelectToolPkg;

  /// No description provided for @extensionsImportScript.
  ///
  /// In en, this message translates to:
  /// **'Import script package'**
  String get extensionsImportScript;

  /// No description provided for @extensionsScriptTypes.
  ///
  /// In en, this message translates to:
  /// **'.js, .ts, .hjson, or .toolpkg'**
  String get extensionsScriptTypes;

  /// No description provided for @extensionsAddSkill.
  ///
  /// In en, this message translates to:
  /// **'Add skill'**
  String get extensionsAddSkill;

  /// No description provided for @extensionsSkillSources.
  ///
  /// In en, this message translates to:
  /// **'GitHub repository, ZIP, or manual entry'**
  String get extensionsSkillSources;

  /// No description provided for @extensionsAddMcp.
  ///
  /// In en, this message translates to:
  /// **'Add MCP server'**
  String get extensionsAddMcp;

  /// No description provided for @extensionsMcpSources.
  ///
  /// In en, this message translates to:
  /// **'GitHub, ZIP, configuration, or form'**
  String get extensionsMcpSources;

  /// No description provided for @extensionsAddA2a.
  ///
  /// In en, this message translates to:
  /// **'Add A2A'**
  String get extensionsAddA2a;

  /// No description provided for @extensionsA2aSources.
  ///
  /// In en, this message translates to:
  /// **'Server address, Agent Card, or configuration file'**
  String get extensionsA2aSources;

  /// No description provided for @extensionsAiCreate.
  ///
  /// In en, this message translates to:
  /// **'Create a plugin with AI'**
  String get extensionsAiCreate;

  /// No description provided for @extensionsAiCreateHelp.
  ///
  /// In en, this message translates to:
  /// **'Describe what you need, then generate and install it in chat'**
  String get extensionsAiCreateHelp;

  /// No description provided for @extensionsAddHelp.
  ///
  /// In en, this message translates to:
  /// **'New extensions are installed on this device by default. A ToolPkg may declare a fixed install scope. Updating a local ToolPkg with the same ID keeps its current location and configuration. You can also drag and drop files on desktop.'**
  String get extensionsAddHelp;

  /// No description provided for @extensionsSyncChanged.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{Device space synced. One extension changed.} other{Device space synced. {count} extensions changed.}}'**
  String extensionsSyncChanged(int count);

  /// No description provided for @extensionsDropRejected.
  ///
  /// In en, this message translates to:
  /// **'Cannot import these files directly: {files}. Use Add extension to import skills or MCP servers.'**
  String extensionsDropRejected(String files);

  /// No description provided for @marketRangeWeek.
  ///
  /// In en, this message translates to:
  /// **'Past week'**
  String get marketRangeWeek;

  /// No description provided for @marketRangeMonth.
  ///
  /// In en, this message translates to:
  /// **'Past month'**
  String get marketRangeMonth;

  /// No description provided for @marketRangeAll.
  ///
  /// In en, this message translates to:
  /// **'All time'**
  String get marketRangeAll;

  /// No description provided for @marketSortUpdated.
  ///
  /// In en, this message translates to:
  /// **'Recently updated'**
  String get marketSortUpdated;

  /// No description provided for @marketSortDownloads.
  ///
  /// In en, this message translates to:
  /// **'Most downloaded'**
  String get marketSortDownloads;

  /// No description provided for @marketSortLikes.
  ///
  /// In en, this message translates to:
  /// **'Most popular'**
  String get marketSortLikes;

  /// No description provided for @marketFilterSummary.
  ///
  /// In en, this message translates to:
  /// **'Filter: {sort}, {range}'**
  String marketFilterSummary(String sort, String range);

  /// No description provided for @marketFilterTitle.
  ///
  /// In en, this message translates to:
  /// **'Filter market'**
  String get marketFilterTitle;

  /// No description provided for @marketFilterSort.
  ///
  /// In en, this message translates to:
  /// **'Sort by'**
  String get marketFilterSort;

  /// No description provided for @marketFilterPeriod.
  ///
  /// In en, this message translates to:
  /// **'Time range'**
  String get marketFilterPeriod;

  /// No description provided for @marketFilterUpdatedHelp.
  ///
  /// In en, this message translates to:
  /// **'Filter by the date content was last updated.'**
  String get marketFilterUpdatedHelp;

  /// No description provided for @marketFilterCategory.
  ///
  /// In en, this message translates to:
  /// **'Category'**
  String get marketFilterCategory;

  /// No description provided for @marketFilterAllCategories.
  ///
  /// In en, this message translates to:
  /// **'All categories'**
  String get marketFilterAllCategories;

  /// No description provided for @marketFilterCategoryRetry.
  ///
  /// In en, this message translates to:
  /// **'Could not load categories. Retry.'**
  String get marketFilterCategoryRetry;

  /// No description provided for @marketFilterReset.
  ///
  /// In en, this message translates to:
  /// **'Reset'**
  String get marketFilterReset;

  /// No description provided for @marketFilterApply.
  ///
  /// In en, this message translates to:
  /// **'Apply filters'**
  String get marketFilterApply;

  /// No description provided for @marketFilterRankingHelp.
  ///
  /// In en, this message translates to:
  /// **'Rankings default to items first published in the last 30 days, ordered by total {metric}. Updates do not reset the publication date.'**
  String marketFilterRankingHelp(String metric);

  /// No description provided for @marketFilterLikes.
  ///
  /// In en, this message translates to:
  /// **'likes'**
  String get marketFilterLikes;

  /// No description provided for @marketFilterDownloads.
  ///
  /// In en, this message translates to:
  /// **'downloads'**
  String get marketFilterDownloads;

  /// No description provided for @attachmentContext.
  ///
  /// In en, this message translates to:
  /// **'Additional context'**
  String get attachmentContext;

  /// No description provided for @attachmentExtraInfo.
  ///
  /// In en, this message translates to:
  /// **'Additional information'**
  String get attachmentExtraInfo;

  /// No description provided for @attachmentPastedText.
  ///
  /// In en, this message translates to:
  /// **'Pasted text'**
  String get attachmentPastedText;

  /// No description provided for @attachmentResetZoom.
  ///
  /// In en, this message translates to:
  /// **'Reset zoom'**
  String get attachmentResetZoom;

  /// No description provided for @attachmentCloseShortcut.
  ///
  /// In en, this message translates to:
  /// **'Close (Esc)'**
  String get attachmentCloseShortcut;

  /// No description provided for @attachmentCopyContent.
  ///
  /// In en, this message translates to:
  /// **'Copy content'**
  String get attachmentCopyContent;

  /// No description provided for @attachmentCopied.
  ///
  /// In en, this message translates to:
  /// **'Copied to clipboard'**
  String get attachmentCopied;

  /// No description provided for @attachmentCopyLocation.
  ///
  /// In en, this message translates to:
  /// **'Copy file location'**
  String get attachmentCopyLocation;

  /// No description provided for @attachmentClipboardImage.
  ///
  /// In en, this message translates to:
  /// **'Clipboard image'**
  String get attachmentClipboardImage;

  /// No description provided for @chatSharePreview.
  ///
  /// In en, this message translates to:
  /// **'Share image preview'**
  String get chatSharePreview;

  /// No description provided for @chatShareSaveFile.
  ///
  /// In en, this message translates to:
  /// **'Save to file'**
  String get chatShareSaveFile;

  /// No description provided for @chatShareImage.
  ///
  /// In en, this message translates to:
  /// **'Share image'**
  String get chatShareImage;

  /// No description provided for @chatShareSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not save'**
  String get chatShareSaveFailed;

  /// No description provided for @chatSharePng.
  ///
  /// In en, this message translates to:
  /// **'PNG image'**
  String get chatSharePng;

  /// No description provided for @chatShareSaved.
  ///
  /// In en, this message translates to:
  /// **'Image saved'**
  String get chatShareSaved;

  /// No description provided for @chatShareFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not share'**
  String get chatShareFailed;

  /// No description provided for @attachmentExpandCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{Show attachment} other{Show {count} attachments}}'**
  String attachmentExpandCount(int count);

  /// No description provided for @attachmentCollapseCount.
  ///
  /// In en, this message translates to:
  /// **'Collapse ({count})'**
  String attachmentCollapseCount(int count);

  /// No description provided for @attachmentContextCount.
  ///
  /// In en, this message translates to:
  /// **'Additional context · {count}'**
  String attachmentContextCount(int count);

  /// No description provided for @attachmentCopyLocationPath.
  ///
  /// In en, this message translates to:
  /// **'Copy file location\n{path}'**
  String attachmentCopyLocationPath(String path);

  /// No description provided for @attachmentLocationPath.
  ///
  /// In en, this message translates to:
  /// **'Saved at: {path}'**
  String attachmentLocationPath(String path);

  /// No description provided for @marketCategorySearch.
  ///
  /// In en, this message translates to:
  /// **'Search & research'**
  String get marketCategorySearch;

  /// No description provided for @marketCategorySearchDescription.
  ///
  /// In en, this message translates to:
  /// **'Search engines, research tools, and knowledge retrieval'**
  String get marketCategorySearchDescription;

  /// No description provided for @marketCategoryDevelopment.
  ///
  /// In en, this message translates to:
  /// **'Development tools'**
  String get marketCategoryDevelopment;

  /// No description provided for @marketCategoryDevelopmentDescription.
  ///
  /// In en, this message translates to:
  /// **'Code editing, debugging, and development tools'**
  String get marketCategoryDevelopmentDescription;

  /// No description provided for @marketCategoryAutomation.
  ///
  /// In en, this message translates to:
  /// **'Automation'**
  String get marketCategoryAutomation;

  /// No description provided for @marketCategoryAutomationDescription.
  ///
  /// In en, this message translates to:
  /// **'Task automation and workflows'**
  String get marketCategoryAutomationDescription;

  /// No description provided for @marketCategoryDocuments.
  ///
  /// In en, this message translates to:
  /// **'Documents & knowledge'**
  String get marketCategoryDocuments;

  /// No description provided for @marketCategoryDocumentsDescription.
  ///
  /// In en, this message translates to:
  /// **'Document processing and knowledge management'**
  String get marketCategoryDocumentsDescription;

  /// No description provided for @marketCategoryMedia.
  ///
  /// In en, this message translates to:
  /// **'Media & content'**
  String get marketCategoryMedia;

  /// No description provided for @marketCategoryMediaDescription.
  ///
  /// In en, this message translates to:
  /// **'Image, video, and audio processing and content creation'**
  String get marketCategoryMediaDescription;

  /// No description provided for @marketCategoryCommunication.
  ///
  /// In en, this message translates to:
  /// **'Chat & communication'**
  String get marketCategoryCommunication;

  /// No description provided for @marketCategoryCommunicationDescription.
  ///
  /// In en, this message translates to:
  /// **'Chat tools, messaging, and communication integrations'**
  String get marketCategoryCommunicationDescription;

  /// No description provided for @marketCategoryIntegration.
  ///
  /// In en, this message translates to:
  /// **'Services & APIs'**
  String get marketCategoryIntegration;

  /// No description provided for @marketCategoryIntegrationDescription.
  ///
  /// In en, this message translates to:
  /// **'Third-party service integrations and API tools'**
  String get marketCategoryIntegrationDescription;

  /// No description provided for @marketCategorySystem.
  ///
  /// In en, this message translates to:
  /// **'System & data'**
  String get marketCategorySystem;

  /// No description provided for @marketCategorySystemDescription.
  ///
  /// In en, this message translates to:
  /// **'File systems, data management, and system tools'**
  String get marketCategorySystemDescription;

  /// No description provided for @marketCategoryProductivity.
  ///
  /// In en, this message translates to:
  /// **'Business & productivity'**
  String get marketCategoryProductivity;

  /// No description provided for @marketCategoryProductivityDescription.
  ///
  /// In en, this message translates to:
  /// **'Business workflows and productivity tools'**
  String get marketCategoryProductivityDescription;

  /// No description provided for @marketCategoryEntertainment.
  ///
  /// In en, this message translates to:
  /// **'Lifestyle & entertainment'**
  String get marketCategoryEntertainment;

  /// No description provided for @marketCategoryEntertainmentDescription.
  ///
  /// In en, this message translates to:
  /// **'Everyday utilities and entertainment apps'**
  String get marketCategoryEntertainmentDescription;

  /// No description provided for @marketCategoryOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get marketCategoryOther;

  /// No description provided for @marketCategoryOtherDescription.
  ///
  /// In en, this message translates to:
  /// **'Uncategorized extensions and other types'**
  String get marketCategoryOtherDescription;

  /// No description provided for @settingsCategorySpeechTitle.
  ///
  /// In en, this message translates to:
  /// **'Speech & recognition'**
  String get settingsCategorySpeechTitle;

  /// No description provided for @settingsCategorySpeechSubtitle.
  ///
  /// In en, this message translates to:
  /// **'TTS and STT providers'**
  String get settingsCategorySpeechSubtitle;

  /// No description provided for @settingsCategorySpeechDescription.
  ///
  /// In en, this message translates to:
  /// **'Manage speech synthesis and recognition providers, and choose the active configuration for each.'**
  String get settingsCategorySpeechDescription;

  /// No description provided for @settingsCategoryAboutTitle.
  ///
  /// In en, this message translates to:
  /// **'About Operit2'**
  String get settingsCategoryAboutTitle;

  /// No description provided for @settingsCategoryAboutSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Version, project, and license'**
  String get settingsCategoryAboutSubtitle;

  /// No description provided for @settingsCategoryAboutDescription.
  ///
  /// In en, this message translates to:
  /// **'View the Operit2 version, project links, and open source licenses.'**
  String get settingsCategoryAboutDescription;

  /// No description provided for @aboutUpdateSection.
  ///
  /// In en, this message translates to:
  /// **'Updates'**
  String get aboutUpdateSection;

  /// No description provided for @aboutAppDescription.
  ///
  /// In en, this message translates to:
  /// **'An AI workbench for terminal and desktop, with conversations, workspaces, tools, plugins, MCP, remote connections, and web access.'**
  String get aboutAppDescription;

  /// No description provided for @aboutReadingVersion.
  ///
  /// In en, this message translates to:
  /// **'Reading version information…'**
  String get aboutReadingVersion;

  /// No description provided for @aboutUpToDate.
  ///
  /// In en, this message translates to:
  /// **'You’re on the latest version'**
  String get aboutUpToDate;

  /// No description provided for @aboutCheckingUpdates.
  ///
  /// In en, this message translates to:
  /// **'Checking for updates…'**
  String get aboutCheckingUpdates;

  /// No description provided for @aboutRetryCheck.
  ///
  /// In en, this message translates to:
  /// **'Update check failed. Select to retry.'**
  String get aboutRetryCheck;

  /// No description provided for @aboutCheckHint.
  ///
  /// In en, this message translates to:
  /// **'Select to check for updates'**
  String get aboutCheckHint;

  /// No description provided for @aboutProject.
  ///
  /// In en, this message translates to:
  /// **'Project'**
  String get aboutProject;

  /// No description provided for @aboutSource.
  ///
  /// In en, this message translates to:
  /// **'Project source code'**
  String get aboutSource;

  /// No description provided for @aboutDocs.
  ///
  /// In en, this message translates to:
  /// **'Documentation'**
  String get aboutDocs;

  /// No description provided for @aboutDocsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'README and command-line guide'**
  String get aboutDocsSubtitle;

  /// No description provided for @aboutLicenses.
  ///
  /// In en, this message translates to:
  /// **'Open source licenses'**
  String get aboutLicenses;

  /// No description provided for @aboutLicenseSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Operit2 is licensed under AGPL-3.0'**
  String get aboutLicenseSubtitle;

  /// No description provided for @aboutContact.
  ///
  /// In en, this message translates to:
  /// **'Contact'**
  String get aboutContact;

  /// No description provided for @aboutDeveloper.
  ///
  /// In en, this message translates to:
  /// **'Developer AAswordman'**
  String get aboutDeveloper;

  /// No description provided for @aboutCopyright.
  ///
  /// In en, this message translates to:
  /// **'© 2025–2026 Operit. All rights reserved.'**
  String get aboutCopyright;

  /// No description provided for @aboutReadVersionFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not read version information: {error}'**
  String aboutReadVersionFailed(String error);

  /// No description provided for @aboutVersion.
  ///
  /// In en, this message translates to:
  /// **'Version {version}'**
  String aboutVersion(String version);

  /// No description provided for @aboutUpdatingTo.
  ///
  /// In en, this message translates to:
  /// **'Updating to {version}. Select to view progress.'**
  String aboutUpdatingTo(String version);

  /// No description provided for @aboutLinkCopied.
  ///
  /// In en, this message translates to:
  /// **'No app can open this link. Copied to clipboard: {url}'**
  String aboutLinkCopied(String url);

  /// No description provided for @settingsToolsAdvanced.
  ///
  /// In en, this message translates to:
  /// **'Advanced settings'**
  String get settingsToolsAdvanced;

  /// No description provided for @settingsToolsReadOnly.
  ///
  /// In en, this message translates to:
  /// **'Read only'**
  String get settingsToolsReadOnly;

  /// No description provided for @settingsToolsReadWrite.
  ///
  /// In en, this message translates to:
  /// **'Read and write'**
  String get settingsToolsReadWrite;

  /// No description provided for @settingsToolsFullAccess.
  ///
  /// In en, this message translates to:
  /// **'Full access'**
  String get settingsToolsFullAccess;

  /// No description provided for @settingsToolsSandboxed.
  ///
  /// In en, this message translates to:
  /// **'Sandboxed (macOS App Sandbox enabled)'**
  String get settingsToolsSandboxed;

  /// No description provided for @settingsToolsUnsandboxed.
  ///
  /// In en, this message translates to:
  /// **'Not sandboxed (macOS App Sandbox disabled)'**
  String get settingsToolsUnsandboxed;

  /// No description provided for @settingsToolsIsolation.
  ///
  /// In en, this message translates to:
  /// **'Application isolation'**
  String get settingsToolsIsolation;

  /// No description provided for @settingsToolsRestrictions.
  ///
  /// In en, this message translates to:
  /// **'AI capability restrictions'**
  String get settingsToolsRestrictions;

  /// No description provided for @settingsToolsConfirmCalls.
  ///
  /// In en, this message translates to:
  /// **'Tool call confirmation'**
  String get settingsToolsConfirmCalls;

  /// No description provided for @settingsToolsConfirmDirectCalls.
  ///
  /// In en, this message translates to:
  /// **'Approval applies to tools the AI calls directly.'**
  String get settingsToolsConfirmDirectCalls;

  /// No description provided for @settingsToolsRuntimeEnvironment.
  ///
  /// In en, this message translates to:
  /// **'Current runtime environment'**
  String get settingsToolsRuntimeEnvironment;

  /// No description provided for @settingsToolsPlatform.
  ///
  /// In en, this message translates to:
  /// **'Platform'**
  String get settingsToolsPlatform;

  /// No description provided for @settingsToolsOuterIsolation.
  ///
  /// In en, this message translates to:
  /// **'Host isolation'**
  String get settingsToolsOuterIsolation;

  /// No description provided for @settingsToolsFileCapability.
  ///
  /// In en, this message translates to:
  /// **'File capability'**
  String get settingsToolsFileCapability;

  /// No description provided for @settingsToolsTerminalCapability.
  ///
  /// In en, this message translates to:
  /// **'Terminal capability'**
  String get settingsToolsTerminalCapability;

  /// No description provided for @settingsToolsRegistered.
  ///
  /// In en, this message translates to:
  /// **'Registered'**
  String get settingsToolsRegistered;

  /// No description provided for @settingsToolsUnregistered.
  ///
  /// In en, this message translates to:
  /// **'Unregistered'**
  String get settingsToolsUnregistered;

  /// No description provided for @settingsToolsAuthorizations.
  ///
  /// In en, this message translates to:
  /// **'Authorizations'**
  String get settingsToolsAuthorizations;

  /// No description provided for @settingsToolsStructuredCapabilities.
  ///
  /// In en, this message translates to:
  /// **'Structured capabilities'**
  String get settingsToolsStructuredCapabilities;

  /// No description provided for @settingsToolsNotGranted.
  ///
  /// In en, this message translates to:
  /// **'Not granted'**
  String get settingsToolsNotGranted;

  /// No description provided for @settingsToolsSystemRequired.
  ///
  /// In en, this message translates to:
  /// **'Requires system action'**
  String get settingsToolsSystemRequired;

  /// No description provided for @settingsToolsOpenSettings.
  ///
  /// In en, this message translates to:
  /// **'Open settings'**
  String get settingsToolsOpenSettings;

  /// No description provided for @settingsToolsUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Unavailable'**
  String get settingsToolsUnavailable;

  /// No description provided for @settingsToolsHandle.
  ///
  /// In en, this message translates to:
  /// **'Review'**
  String get settingsToolsHandle;

  /// No description provided for @settingsToolsReadOnlyDescription.
  ///
  /// In en, this message translates to:
  /// **'The AI can read the current workspace and cannot run tools that write to it.'**
  String get settingsToolsReadOnlyDescription;

  /// No description provided for @settingsToolsReadWriteDescription.
  ///
  /// In en, this message translates to:
  /// **'The AI can read and write to the current workspace. The in-app sandbox stays enabled.'**
  String get settingsToolsReadWriteDescription;

  /// No description provided for @settingsToolsFullAccessDescription.
  ///
  /// In en, this message translates to:
  /// **'The AI can read and write to the current workspace. This mode disables the in-app sandbox.'**
  String get settingsToolsFullAccessDescription;

  /// No description provided for @settingsToolsAuthorizationFailed.
  ///
  /// In en, this message translates to:
  /// **'Authorization failed: {error}'**
  String settingsToolsAuthorizationFailed(String error);

  /// No description provided for @settingsToolsItemCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{count} item} other{{count} items}}'**
  String settingsToolsItemCount(int count);

  /// No description provided for @settingsUsageTabAnalytics.
  ///
  /// In en, this message translates to:
  /// **'Token Usage & Speed'**
  String get settingsUsageTabAnalytics;

  /// No description provided for @settingsUsageTabQuotas.
  ///
  /// In en, this message translates to:
  /// **'Balances & Quotas'**
  String get settingsUsageTabQuotas;

  /// No description provided for @settingsUsageDeviceScopeAll.
  ///
  /// In en, this message translates to:
  /// **'All Space devices'**
  String get settingsUsageDeviceScopeAll;

  /// No description provided for @settingsUsageDeviceScopeLocal.
  ///
  /// In en, this message translates to:
  /// **'This device'**
  String get settingsUsageDeviceScopeLocal;

  /// No description provided for @settingsUsageExportJson.
  ///
  /// In en, this message translates to:
  /// **'Copy JSON report'**
  String get settingsUsageExportJson;

  /// No description provided for @settingsUsageExportCsv.
  ///
  /// In en, this message translates to:
  /// **'Copy CSV table'**
  String get settingsUsageExportCsv;

  /// No description provided for @settingsUsageExportCopied.
  ///
  /// In en, this message translates to:
  /// **'Report copied to clipboard'**
  String get settingsUsageExportCopied;

  /// No description provided for @settingsUsageResetScopeTitle.
  ///
  /// In en, this message translates to:
  /// **'Reset usage statistics'**
  String get settingsUsageResetScopeTitle;

  /// No description provided for @settingsUsageResetLocalOnly.
  ///
  /// In en, this message translates to:
  /// **'This device only'**
  String get settingsUsageResetLocalOnly;

  /// No description provided for @settingsUsageResetAllSpace.
  ///
  /// In en, this message translates to:
  /// **'All devices in Space'**
  String get settingsUsageResetAllSpace;

  /// No description provided for @settingsUsageResetScopeWarning.
  ///
  /// In en, this message translates to:
  /// **'Local reset keeps synced records on other devices; Space-wide reset clears history across all connected devices.'**
  String get settingsUsageResetScopeWarning;

  /// No description provided for @settingsUsageKpiThroughput.
  ///
  /// In en, this message translates to:
  /// **'Avg generation speed'**
  String get settingsUsageKpiThroughput;

  /// No description provided for @settingsUsageKpiTtftMedian.
  ///
  /// In en, this message translates to:
  /// **'Median TTFT: {ttft}'**
  String settingsUsageKpiTtftMedian(String ttft);

  /// No description provided for @settingsUsageKpiSpeedSamples.
  ///
  /// In en, this message translates to:
  /// **'{count} streamed'**
  String settingsUsageKpiSpeedSamples(int count);

  /// No description provided for @settingsUsageKpiReasoning.
  ///
  /// In en, this message translates to:
  /// **'Reasoning'**
  String get settingsUsageKpiReasoning;

  /// No description provided for @settingsUsageKpiCacheWrite.
  ///
  /// In en, this message translates to:
  /// **'Cache write'**
  String get settingsUsageKpiCacheWrite;

  /// No description provided for @settingsUsageKpiSuccessRate.
  ///
  /// In en, this message translates to:
  /// **'{ok} ok · {err} fail/cancel'**
  String settingsUsageKpiSuccessRate(int ok, int err);

  /// No description provided for @settingsUsageTimelineSpeedTitle.
  ///
  /// In en, this message translates to:
  /// **'Generation speed (tok/s)'**
  String get settingsUsageTimelineSpeedTitle;

  /// No description provided for @settingsUsageTimelineSpeedDetail.
  ///
  /// In en, this message translates to:
  /// **'Streamed output {speed} tok/s · TTFT {ttft}'**
  String settingsUsageTimelineSpeedDetail(String speed, String ttft);

  /// No description provided for @settingsUsageTimelineNoSpeedData.
  ///
  /// In en, this message translates to:
  /// **'No streamed requests ≥100ms in range'**
  String get settingsUsageTimelineNoSpeedData;

  /// No description provided for @settingsUsageAttrModels.
  ///
  /// In en, this message translates to:
  /// **'Models'**
  String get settingsUsageAttrModels;

  /// No description provided for @settingsUsageAttrSources.
  ///
  /// In en, this message translates to:
  /// **'Function & Source'**
  String get settingsUsageAttrSources;

  /// No description provided for @settingsUsageAttrDevices.
  ///
  /// In en, this message translates to:
  /// **'Space Devices'**
  String get settingsUsageAttrDevices;

  /// No description provided for @settingsUsageAttrLogs.
  ///
  /// In en, this message translates to:
  /// **'Request Logs'**
  String get settingsUsageAttrLogs;

  /// No description provided for @settingsUsageColSpeed.
  ///
  /// In en, this message translates to:
  /// **'Speed'**
  String get settingsUsageColSpeed;

  /// No description provided for @settingsUsageColTtft.
  ///
  /// In en, this message translates to:
  /// **'TTFT'**
  String get settingsUsageColTtft;

  /// No description provided for @settingsUsageColSpend.
  ///
  /// In en, this message translates to:
  /// **'Est. Cost'**
  String get settingsUsageColSpend;

  /// No description provided for @settingsUsageColShare.
  ///
  /// In en, this message translates to:
  /// **'Share'**
  String get settingsUsageColShare;

  /// No description provided for @settingsUsageDeviceUnknown.
  ///
  /// In en, this message translates to:
  /// **'Local / Legacy Node'**
  String get settingsUsageDeviceUnknown;

  /// No description provided for @settingsUsageLogFilterRecent.
  ///
  /// In en, this message translates to:
  /// **'Recent'**
  String get settingsUsageLogFilterRecent;

  /// No description provided for @settingsUsageLogFilterTopTokens.
  ///
  /// In en, this message translates to:
  /// **'Peak Tokens'**
  String get settingsUsageLogFilterTopTokens;

  /// No description provided for @settingsUsageLogFilterIssues.
  ///
  /// In en, this message translates to:
  /// **'Failed / Cancelled'**
  String get settingsUsageLogFilterIssues;

  /// No description provided for @settingsUsageLogFilterReasoning.
  ///
  /// In en, this message translates to:
  /// **'Reasoning'**
  String get settingsUsageLogFilterReasoning;

  /// No description provided for @settingsUsageStatusCompleted.
  ///
  /// In en, this message translates to:
  /// **'Completed'**
  String get settingsUsageStatusCompleted;

  /// No description provided for @settingsUsageStatusCancelled.
  ///
  /// In en, this message translates to:
  /// **'Cancelled'**
  String get settingsUsageStatusCancelled;

  /// No description provided for @settingsUsageStatusFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get settingsUsageStatusFailed;

  /// No description provided for @settingsUsageRequestDetailTitle.
  ///
  /// In en, this message translates to:
  /// **'Request Telemetry'**
  String get settingsUsageRequestDetailTitle;

  /// No description provided for @settingsUsageLatencyBreakdown.
  ///
  /// In en, this message translates to:
  /// **'Wait {wait} · Stream {stream}'**
  String settingsUsageLatencyBreakdown(String wait, String stream);

  /// No description provided for @settingsQuotaProbeAll.
  ///
  /// In en, this message translates to:
  /// **'Refresh all'**
  String get settingsQuotaProbeAll;

  /// No description provided for @settingsQuotaProbeSingle.
  ///
  /// In en, this message translates to:
  /// **'Refresh'**
  String get settingsQuotaProbeSingle;

  /// No description provided for @settingsQuotaProbeViaNode.
  ///
  /// In en, this message translates to:
  /// **'Query via Space node'**
  String get settingsQuotaProbeViaNode;

  /// No description provided for @settingsQuotaFilterAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get settingsQuotaFilterAll;

  /// No description provided for @settingsQuotaFilterBalance.
  ///
  /// In en, this message translates to:
  /// **'Balances'**
  String get settingsQuotaFilterBalance;

  /// No description provided for @settingsQuotaFilterSubscription.
  ///
  /// In en, this message translates to:
  /// **'Plans'**
  String get settingsQuotaFilterSubscription;

  /// No description provided for @settingsQuotaFilterLocal.
  ///
  /// In en, this message translates to:
  /// **'Local'**
  String get settingsQuotaFilterLocal;

  /// No description provided for @settingsQuotaFilterBudget.
  ///
  /// In en, this message translates to:
  /// **'Budgets & Console'**
  String get settingsQuotaFilterBudget;

  /// No description provided for @settingsQuotaRemainingLabel.
  ///
  /// In en, this message translates to:
  /// **'Remaining'**
  String get settingsQuotaRemainingLabel;

  /// No description provided for @settingsQuotaCashLabel.
  ///
  /// In en, this message translates to:
  /// **'Cash'**
  String get settingsQuotaCashLabel;

  /// No description provided for @settingsQuotaGrantedLabel.
  ///
  /// In en, this message translates to:
  /// **'Granted'**
  String get settingsQuotaGrantedLabel;

  /// No description provided for @settingsQuotaUsedLabel.
  ///
  /// In en, this message translates to:
  /// **'Used'**
  String get settingsQuotaUsedLabel;

  /// No description provided for @settingsQuotaDailyUsedLabel.
  ///
  /// In en, this message translates to:
  /// **'Today'**
  String get settingsQuotaDailyUsedLabel;

  /// No description provided for @settingsQuotaMonthlyUsedLabel.
  ///
  /// In en, this message translates to:
  /// **'Month'**
  String get settingsQuotaMonthlyUsedLabel;

  /// No description provided for @settingsQuotaKeyPoolCount.
  ///
  /// In en, this message translates to:
  /// **'{count} pool keys'**
  String settingsQuotaKeyPoolCount(int count);

  /// No description provided for @settingsQuotaStatusOk.
  ///
  /// In en, this message translates to:
  /// **'Active'**
  String get settingsQuotaStatusOk;

  /// No description provided for @settingsQuotaStatusUnauthorized.
  ///
  /// In en, this message translates to:
  /// **'Unauthorized'**
  String get settingsQuotaStatusUnauthorized;

  /// No description provided for @settingsQuotaStatusError.
  ///
  /// In en, this message translates to:
  /// **'Error'**
  String get settingsQuotaStatusError;

  /// No description provided for @settingsQuotaStatusStale.
  ///
  /// In en, this message translates to:
  /// **'Not queried'**
  String get settingsQuotaStatusStale;

  /// No description provided for @settingsQuotaStatusUnsupported.
  ///
  /// In en, this message translates to:
  /// **'No Public API'**
  String get settingsQuotaStatusUnsupported;

  /// No description provided for @settingsQuotaProbedBy.
  ///
  /// In en, this message translates to:
  /// **'Queried by {device} · {time}'**
  String settingsQuotaProbedBy(String device, String time);

  /// No description provided for @settingsQuotaOpenConsole.
  ///
  /// In en, this message translates to:
  /// **'Console'**
  String get settingsQuotaOpenConsole;

  /// No description provided for @settingsQuotaProtocolConfig.
  ///
  /// In en, this message translates to:
  /// **'Protocol'**
  String get settingsQuotaProtocolConfig;

  /// No description provided for @settingsQuotaSetBudget.
  ///
  /// In en, this message translates to:
  /// **'Budget Cap'**
  String get settingsQuotaSetBudget;

  /// No description provided for @settingsQuotaEditBudget.
  ///
  /// In en, this message translates to:
  /// **'Edit Budget'**
  String get settingsQuotaEditBudget;

  /// No description provided for @settingsQuotaBudgetProgress.
  ///
  /// In en, this message translates to:
  /// **'{spent} / {limit} this cycle'**
  String settingsQuotaBudgetProgress(String spent, String limit);

  /// No description provided for @settingsQuotaBudgetExceeded.
  ///
  /// In en, this message translates to:
  /// **'Budget alert threshold reached'**
  String get settingsQuotaBudgetExceeded;

  /// No description provided for @settingsQuotaDialogBudgetLimit.
  ///
  /// In en, this message translates to:
  /// **'Monthly spending cap'**
  String get settingsQuotaDialogBudgetLimit;

  /// No description provided for @settingsQuotaDialogResetDay.
  ///
  /// In en, this message translates to:
  /// **'Cycle reset day (1-28)'**
  String get settingsQuotaDialogResetDay;

  /// No description provided for @settingsQuotaDialogAlertPct.
  ///
  /// In en, this message translates to:
  /// **'Alert threshold (%)'**
  String get settingsQuotaDialogAlertPct;

  /// No description provided for @settingsQuotaDialogProtocolLabel.
  ///
  /// In en, this message translates to:
  /// **'Balance query protocol'**
  String get settingsQuotaDialogProtocolLabel;

  /// No description provided for @settingsQuotaDialogCustomPath.
  ///
  /// In en, this message translates to:
  /// **'Custom URL / relative path'**
  String get settingsQuotaDialogCustomPath;

  /// No description provided for @settingsQuotaDialogAmountJsonPath.
  ///
  /// In en, this message translates to:
  /// **'Amount JSON dot path'**
  String get settingsQuotaDialogAmountJsonPath;

  /// No description provided for @settingsQuotaDialogCurrencyJsonPath.
  ///
  /// In en, this message translates to:
  /// **'Currency JSON dot path (optional)'**
  String get settingsQuotaDialogCurrencyJsonPath;

  /// No description provided for @settingsQuotaDialogClear.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get settingsQuotaDialogClear;

  /// No description provided for @settingsQuotaEmptyProviders.
  ///
  /// In en, this message translates to:
  /// **'No model providers configured yet'**
  String get settingsQuotaEmptyProviders;

  /// No description provided for @settingsQuotaLocalZeroCost.
  ///
  /// In en, this message translates to:
  /// **'On-device inference · Zero API fee'**
  String get settingsQuotaLocalZeroCost;

  /// No description provided for @settingsUsageColStatus.
  ///
  /// In en, this message translates to:
  /// **'Status'**
  String get settingsUsageColStatus;

  /// No description provided for @settingsUsageColTime.
  ///
  /// In en, this message translates to:
  /// **'Time'**
  String get settingsUsageColTime;

  /// No description provided for @settingsUsageModeStream.
  ///
  /// In en, this message translates to:
  /// **'Stream'**
  String get settingsUsageModeStream;

  /// No description provided for @settingsUsageModeNonStream.
  ///
  /// In en, this message translates to:
  /// **'Standard'**
  String get settingsUsageModeNonStream;

  /// No description provided for @settingsUsageTopModelsTitle.
  ///
  /// In en, this message translates to:
  /// **'Model Distribution & Token Mix'**
  String get settingsUsageTopModelsTitle;

  /// No description provided for @settingsUsageTokenMixLabel.
  ///
  /// In en, this message translates to:
  /// **'Token composition'**
  String get settingsUsageTokenMixLabel;

  /// No description provided for @settingsUsageDirectInputLabel.
  ///
  /// In en, this message translates to:
  /// **'Direct input'**
  String get settingsUsageDirectInputLabel;

  /// No description provided for @settingsUsageNoModelsInRange.
  ///
  /// In en, this message translates to:
  /// **'No model traffic in this window'**
  String get settingsUsageNoModelsInRange;

  /// No description provided for @settingsUsageKpiNoPriceHint.
  ///
  /// In en, this message translates to:
  /// **'No model pricing matched'**
  String get settingsUsageKpiNoPriceHint;

  /// No description provided for @settingsUsageKpiStreamHint.
  ///
  /// In en, this message translates to:
  /// **'Needs streamed reply ≥100ms'**
  String get settingsUsageKpiStreamHint;

  /// No description provided for @settingsUsageHeatmapWeeks.
  ///
  /// In en, this message translates to:
  /// **'{weeks}-week activity'**
  String settingsUsageHeatmapWeeks(int weeks);

  /// No description provided for @settingsQuotaSummaryTotalRemaining.
  ///
  /// In en, this message translates to:
  /// **'Tracked Balance'**
  String get settingsQuotaSummaryTotalRemaining;

  /// No description provided for @settingsQuotaSummaryProviders.
  ///
  /// In en, this message translates to:
  /// **'{ready} active · {total} configured'**
  String settingsQuotaSummaryProviders(int ready, int total);

  /// No description provided for @settingsQuotaSummaryAlertCount.
  ///
  /// In en, this message translates to:
  /// **'{count} alerts'**
  String settingsQuotaSummaryAlertCount(int count);

  /// No description provided for @settingsUsageHeatmapSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Daily usage over the last {weeks} weeks'**
  String settingsUsageHeatmapSubtitle(int weeks);

  /// No description provided for @settingsUsageHeatmapMetricTokens.
  ///
  /// In en, this message translates to:
  /// **'Token'**
  String get settingsUsageHeatmapMetricTokens;

  /// No description provided for @settingsUsageHeatmapMetricRequests.
  ///
  /// In en, this message translates to:
  /// **'Requests'**
  String get settingsUsageHeatmapMetricRequests;

  /// No description provided for @settingsUsageHeatmapMetricCost.
  ///
  /// In en, this message translates to:
  /// **'Cost'**
  String get settingsUsageHeatmapMetricCost;

  /// No description provided for @settingsUsageWeekdayMon.
  ///
  /// In en, this message translates to:
  /// **'Mon'**
  String get settingsUsageWeekdayMon;

  /// No description provided for @settingsUsageWeekdayWed.
  ///
  /// In en, this message translates to:
  /// **'Wed'**
  String get settingsUsageWeekdayWed;

  /// No description provided for @settingsUsageWeekdayFri.
  ///
  /// In en, this message translates to:
  /// **'Fri'**
  String get settingsUsageWeekdayFri;

  /// No description provided for @settingsUsageWeekdaySun.
  ///
  /// In en, this message translates to:
  /// **'Sun'**
  String get settingsUsageWeekdaySun;
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
      <String>['en', 'fr', 'zh'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'fr':
      return AppLocalizationsFr();
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
