// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get askOperitHint => '向 Operit 提问';

  @override
  String get aiChat => 'AI聊天';

  @override
  String get fullscreenInput => '全屏输入';

  @override
  String get settings => '设置';

  @override
  String get addAttachment => '添加附件';

  @override
  String get cancel => '取消';

  @override
  String get send => '发送';

  @override
  String get model => '模型';

  @override
  String get processingInput => '正在处理输入...';

  @override
  String get processingMessage => '正在处理消息...';

  @override
  String get connectingAiService => '正在连接 AI 服务...';

  @override
  String get receivingAiResponse => '正在接收 AI 响应...';

  @override
  String get receivingToolResultAiResponse => '正在接收工具执行后的 AI 响应...';

  @override
  String get summarizingMemories => '正在总结记忆...';

  @override
  String get executingPlan => '正在执行计划...';

  @override
  String executingTool(String toolName) {
    return '正在执行工具: $toolName';
  }

  @override
  String processingToolResult(String toolName) {
    return '正在处理工具结果: $toolName';
  }

  @override
  String get toolRunning => '工具执行中...';

  @override
  String toolRunningWithName(String toolName) {
    return '$toolName: 工具执行中...';
  }

  @override
  String toolStatusWithName(String toolName, String message) {
    return '$toolName: $message';
  }
}
