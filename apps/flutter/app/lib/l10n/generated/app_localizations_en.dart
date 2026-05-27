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
}
