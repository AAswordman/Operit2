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
  String get roleResponsePlannerPlanning => '正在规划群组发言顺序...';

  @override
  String roleResponsePlannerMemberReplying(String memberName) {
    return '正在生成「$memberName」的回复...';
  }

  @override
  String get roleResponsePlannerFailed => '群组规划失败';

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

  @override
  String get close => '关闭';

  @override
  String get create => '创建';

  @override
  String get save => '保存';

  @override
  String get delete => '删除';

  @override
  String get search => '搜索';

  @override
  String get loading => '加载中';

  @override
  String get toolApprovalTitle => '工具权限申请';

  @override
  String get toolApprovalToolLabel => '工具';

  @override
  String get toolApprovalActionLabel => '操作';

  @override
  String get toolApprovalDeny => '拒绝';

  @override
  String get toolApprovalAllowOnce => '允许本次';

  @override
  String get toolApprovalAlwaysAllow => '始终允许';

  @override
  String get createGroupTitle => '新建分组';

  @override
  String get groupNameLabel => '分组名称';

  @override
  String get renameConversationTitle => '编辑标题';

  @override
  String get newTitleLabel => '新标题';

  @override
  String get deleteConversationTitle => '确认删除对话';

  @override
  String deleteConversationMessage(String title) {
    return '删除 “$title”？';
  }

  @override
  String get chatHistory => '聊天记录';

  @override
  String get editTitle => '编辑标题';

  @override
  String get moveUp => '上移';

  @override
  String get moveDown => '下移';

  @override
  String get pin => '置顶';

  @override
  String get unpin => '取消置顶';

  @override
  String get lock => '锁定';

  @override
  String get unlock => '解锁';

  @override
  String get messageLocatorTitle => '消息定位';

  @override
  String messageLocatorCurrent(int current, int total) {
    return '当前 $current / $total';
  }

  @override
  String get messageLocatorSearchHint => '搜索消息内容';

  @override
  String get messageLocatorInstruction => '滚动列表或搜索后跳转到指定消息';

  @override
  String messageLocatorResultCount(int count) {
    return '$count 条结果';
  }

  @override
  String get messageLocatorNoMatches => '没有匹配的消息';

  @override
  String get messageSenderUser => '用户';

  @override
  String get messageSenderSummary => '摘要';

  @override
  String get messageSenderSystem => '系统';

  @override
  String get messageSenderThinking => '思考';

  @override
  String get messageSenderOther => '其他';

  @override
  String get hiddenUserMessage => '隐藏的用户消息';

  @override
  String get workspaceSetupTitle => '设置工作区';

  @override
  String get workspaceSetupSubtitle => '为您的AI项目提供一个专属的文件环境';

  @override
  String get workspaceCreateDefaultTitle => '创建默认';

  @override
  String get workspaceCreateDefaultDescription => '在应用内创建新工作区';

  @override
  String get workspaceBindExistingTitle => '选择已有';

  @override
  String get workspaceBindExistingDescription => '从设备选择文件夹';

  @override
  String get workspaceProjectTypeDialogTitle => '选择语言类型';

  @override
  String get workspaceProjectTypeDialogDescription => '请选择要创建的默认工作区类型';

  @override
  String get workspaceBindDialogTitle => '选择已有工作区';

  @override
  String get workspacePathLabel => '工作区路径';

  @override
  String get workspaceEnvLabel => '工作区环境';

  @override
  String get optionalHint => '可留空';

  @override
  String get workspacePathRequired => '请输入工作区路径';

  @override
  String get bind => '绑定';

  @override
  String get workspaceProjectBlankTitle => '空白工作区';

  @override
  String get workspaceProjectBlankDescription => '仅创建一个空的工作区目录，不包含任何模板文件';

  @override
  String get workspaceProjectOfficeTitle => '办公文档';

  @override
  String get workspaceProjectOfficeDescription => '用于文档编辑、文件处理和通用办公任务';

  @override
  String get workspaceProjectWebTitle => 'Web 项目';

  @override
  String get workspaceProjectWebDescription =>
      '适用于网页开发，支持 HTML/CSS/JavaScript，自动启动本地服务器';

  @override
  String get workspaceProjectAndroidTitle => 'Android 项目';

  @override
  String get workspaceProjectAndroidDescription =>
      '适用于 Android 工程开发，包含 Gradle 常用任务快捷按钮';

  @override
  String get workspaceProjectFlutterTitle => 'Flutter 项目';

  @override
  String get workspaceProjectFlutterDescription =>
      '适用于 Flutter 跨平台开发，内置当前稳定版应用模板和常用命令';

  @override
  String get workspaceProjectNodeTitle => 'Node.js 项目';

  @override
  String get workspaceProjectNodeDescription =>
      '适用于 Node.js 后端开发，提供 npm 命令快捷按钮';

  @override
  String get workspaceProjectTypeScriptTitle => 'TypeScript 项目';

  @override
  String get workspaceProjectTypeScriptDescription =>
      'TypeScript + pnpm，支持类型安全开发和 tsc watch 实时编译';

  @override
  String get workspaceProjectPythonTitle => 'Python 项目';

  @override
  String get workspaceProjectPythonDescription =>
      '适用于 Python 开发，支持 pip 和 HTTP 服务器';

  @override
  String get workspaceProjectJavaTitle => 'Java 项目';

  @override
  String get workspaceProjectJavaDescription =>
      '适用于 Java 开发，支持 Gradle 和 Maven 构建';

  @override
  String get workspaceProjectGoTitle => 'Go 项目';

  @override
  String get workspaceProjectGoDescription => '适用于 Go 开发，提供 go mod 和 build 命令';

  @override
  String get version => '版本';

  @override
  String get author => '作者';

  @override
  String get entry => '入口';

  @override
  String get source => '来源';

  @override
  String get category => '分类';

  @override
  String get defaultStatus => '默认状态';

  @override
  String get builtIn => '内置';

  @override
  String get external => '外部';

  @override
  String get enabledByDefault => '默认启用';

  @override
  String get disabledByDefault => '默认停用';

  @override
  String get toolPkgResources => 'ToolPkg 资源';

  @override
  String resourcesCount(int count) {
    return '资源 $count';
  }

  @override
  String uiModulesCount(int count) {
    return 'UI 模块 $count';
  }

  @override
  String navigationEntriesCount(int count) {
    return '导航入口 $count';
  }

  @override
  String desktopWidgetsCount(int count) {
    return '桌面组件 $count';
  }

  @override
  String workflowTemplatesCount(int count) {
    return 'Workflow 模板 $count';
  }

  @override
  String workspaceTemplatesCount(int count) {
    return 'Workspace 模板 $count';
  }

  @override
  String get pluginConfiguration => '插件配置';

  @override
  String get subpackages => '子包';

  @override
  String get toolPkgNoSubpackages => '此 ToolPkg 未声明子包';

  @override
  String subpackageToolCount(String packageName, int count) {
    return '$packageName · $count 工具';
  }

  @override
  String get workflowTemplates => 'Workflow 模板';

  @override
  String get workspaceTemplates => 'Workspace 模板';

  @override
  String get disable => '停用';

  @override
  String get enable => '启用';

  @override
  String get environmentVariables => '环境变量';

  @override
  String get required => '必填';

  @override
  String get states => '状态';

  @override
  String stateToolSummary(String condition, int toolCount, int excludeCount) {
    return '$condition · $toolCount 工具 · 排除 $excludeCount';
  }

  @override
  String get inherit => '继承';

  @override
  String get tools => '工具';

  @override
  String get packageNoTools => '此包未声明工具';

  @override
  String get permissionsTitle => '权限';

  @override
  String get clear => '清空';

  @override
  String get noPermissionRecords => '还没有权限记录';

  @override
  String get allow => '允许';

  @override
  String get deny => '拒绝';

  @override
  String get camera => '摄像头';

  @override
  String get microphone => '麦克风';

  @override
  String get protectedMedia => '受保护媒体';

  @override
  String get midiDevice => 'MIDI 设备';

  @override
  String get browserPermissionRequestTitle => '网页权限请求';

  @override
  String get history => '历史记录';

  @override
  String get bookmarks => '收藏夹';

  @override
  String get downloads => '下载';

  @override
  String get scripts => '脚本';

  @override
  String get zoom => '缩放';

  @override
  String get zoomIn => '放大';

  @override
  String get zoomOut => '缩小';

  @override
  String get desktopMode => '桌面模式';

  @override
  String get clearLocalStorage => '清理本地存储';

  @override
  String get searchHistory => '搜索历史';

  @override
  String get noDownloadTasks => '还没有下载任务';

  @override
  String get openFile => '打开文件';

  @override
  String get openLocation => '打开位置';

  @override
  String get retry => '重试';

  @override
  String get removeRecord => '移除记录';

  @override
  String get pending => '等待';

  @override
  String get completed => '完成';

  @override
  String get failed => '失败';

  @override
  String get back => '后退';

  @override
  String get forward => '前进';

  @override
  String get stop => '停止';

  @override
  String get refresh => '刷新';

  @override
  String get home => '主页';

  @override
  String get newTab => '新建标签页';

  @override
  String get openExternalApplication => '打开外部应用';

  @override
  String get open => '打开';

  @override
  String get ok => '确定';

  @override
  String get webPage => '网页';

  @override
  String get tabs => '标签页';

  @override
  String get noBookmarks => '还没有收藏';

  @override
  String get removeBookmark => '取消收藏';

  @override
  String get addBookmark => '收藏';

  @override
  String get menu => '菜单';

  @override
  String get siteData => '站点数据';

  @override
  String get clearAllWebViewCookies => '清除所有 WebView Cookie';

  @override
  String get clearCookies => '清除 Cookie';

  @override
  String get noData => '没有数据';

  @override
  String get local => '本地';

  @override
  String get pageLoadFailed => '页面加载失败';

  @override
  String get pause => '暂停';

  @override
  String get resume => '恢复';

  @override
  String get paused => '暂停';

  @override
  String get cancelled => '取消';

  @override
  String get downloading => '正在下载';

  @override
  String savedTo(String path) {
    return '已保存到 $path';
  }

  @override
  String get sslCertificateError => 'SSL 证书错误';

  @override
  String get edit => '编辑';

  @override
  String get files => '文件';

  @override
  String get terminal => '终端';

  @override
  String get browser => '浏览器';

  @override
  String get filePreview => '文件预览';

  @override
  String get selectFile => '选择文件';

  @override
  String get selectFileDescription => '从工作区里选择要查看、编辑或交给 AI 的文件';

  @override
  String get openTerminal => '打开终端';

  @override
  String get openTerminalDescription => '进入当前工作区的命令行';

  @override
  String get openBrowser => '打开浏览器';

  @override
  String get openBrowserDescription => '打开完整浏览器会话、项目预览和网页自动化';

  @override
  String get noWorkspaceBound => '当前对话还没有绑定工作区。';

  @override
  String get terminalSessionPlaceholder => '这里会显示当前工作区的终端会话。';

  @override
  String get emptyFolder => '这个文件夹是空的';

  @override
  String get imagePreview => '图片预览';

  @override
  String get audioPreview => '音频预览';

  @override
  String get videoPreview => '视频预览';

  @override
  String get pdfPreview => 'PDF 预览';

  @override
  String get wordPreview => 'Word 预览';

  @override
  String get spreadsheetPreview => '表格预览';

  @override
  String get presentationPreview => '演示文稿预览';

  @override
  String get webPagePreview => '网页预览';

  @override
  String get markdownPreview => 'Markdown 预览';

  @override
  String get textPreview => '文本预览';

  @override
  String get file => '文件';

  @override
  String get unsupportedReadOnlyPreview => '此文件不属于内置只读预览类型。';

  @override
  String get cannotPreview => '无法预览';

  @override
  String get openProjectInFullBrowser => '用完整浏览器打开项目';

  @override
  String get openInBrowser => '在浏览器打开';

  @override
  String get emptySpreadsheet => '表格为空';
}
