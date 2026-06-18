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
  String get packageManager => '包管理';

  @override
  String get market => '市场';

  @override
  String get addAttachment => '添加附件';

  @override
  String get attachmentPhoto => '照片';

  @override
  String get attachmentCamera => '拍照';

  @override
  String get attachmentMemory => '记忆';

  @override
  String get attachmentFile => '文件';

  @override
  String get attachmentScreenContent => '屏幕内容';

  @override
  String get attachmentNotifications => '当前通知';

  @override
  String get attachmentLocation => '当前位置';

  @override
  String get attachmentPackage => '包';

  @override
  String get attachmentPackageSelectTitle => '选择包';

  @override
  String get attachmentPackageEmpty => '暂无可用包';

  @override
  String get attachmentPackageSearchPlaceholder => '搜索包名、描述';

  @override
  String get attachmentPackageSearchEmpty => '没有匹配的包';

  @override
  String get attachmentPackageKindPackage => '包';

  @override
  String get attachmentPackageKindSkill => '技能';

  @override
  String get attachmentPackageKindMcp => 'MCP';

  @override
  String get attachmentCameraUnavailable => '当前 Flutter 端没有相机采集能力';

  @override
  String get attachmentMemoryUnavailable => '当前 Flutter 端没有记忆文件夹选择器';

  @override
  String get clearSearch => '清空搜索';

  @override
  String chatPendingQueueTitle(int count) {
    return '待发送消息（$count）';
  }

  @override
  String get chatQueueAddMessage => '加入队列';

  @override
  String get chatQueueAdded => '已加入发送队列';

  @override
  String get chatPleaseCreateNewChat => '请新建对话';

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
  String get statusWarningAiErrorSummary => 'AI犯了一个错误';

  @override
  String get statusWarningAiErrorDetailTitle => 'AI错误原因';

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
  String get groupNameLabel => '群组名称';

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
  String get thinkingProcess => '思考过程';

  @override
  String thinkingToolsGroupTitleWithCount(int count) {
    return '思考与工具调用（$count）';
  }

  @override
  String toolsGroupTitleWithCount(int count) {
    return '工具调用（$count）';
  }

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
  String get workspacePathLabel => '已选工作区';

  @override
  String get workspaceEnvLabel => '工作区环境';

  @override
  String get optionalHint => '可留空';

  @override
  String get workspacePathRequired => '请选择工作区文件夹';

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
  String get workspaceBoundTitle => '已绑定工作区';

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

  @override
  String get settingsCategoryModelTitle => '模型与 AI';

  @override
  String get settingsCategoryModelSubtitle => '模型、Key、上下文';

  @override
  String get settingsCategoryModelDescription =>
      '配置模型连接，选择聊天模型，并管理思考、上下文和多模态能力。';

  @override
  String get settingsCategoryCharactersTitle => '角色与记忆';

  @override
  String get settingsCategoryCharactersSubtitle => '角色卡、群组、绑定';

  @override
  String get settingsCategoryCharactersDescription =>
      '管理角色卡、群组、当前激活角色和角色级模型/记忆/工具绑定。';

  @override
  String get settingsCategoryToolsTitle => '工具与扩展';

  @override
  String get settingsCategoryToolsSubtitle => '权限、包、技能、MCP';

  @override
  String get settingsCategoryToolsDescription =>
      '控制 Operit 可以调用哪些工具，并管理插件、工具包、技能和 MCP 服务。';

  @override
  String get settingsCategoryWorkspaceTitle => '工作区与浏览器';

  @override
  String get settingsCategoryWorkspaceSubtitle => '文件、终端、浏览器';

  @override
  String get settingsCategoryWorkspaceDescription =>
      '管理默认工作区、终端会话、浏览器模式、脚本和网页自动化。';

  @override
  String get settingsCategoryAppearanceTitle => '外观与交互';

  @override
  String get settingsCategoryAppearanceSubtitle => '主题、语言';

  @override
  String get settingsCategoryAppearanceDescription => '调整客户端主题和当前本地化显示。';

  @override
  String get settingsCategoryDataTitle => '数据与备份';

  @override
  String get settingsCategoryDataSubtitle => '备份、恢复、统计';

  @override
  String get settingsCategoryDataDescription => '备份聊天、角色和模型配置，恢复备份内容，并查看数据统计。';

  @override
  String get settingsCategoryAccessLinksTitle => '设备与访问';

  @override
  String get settingsCategoryAccessLinksSubtitle => '连接、同步、访问';

  @override
  String get settingsCategoryAccessLinksDescription =>
      '连接另一台设备，同步数据，或允许浏览器访问这台设备。';

  @override
  String get settingsComingSoon => '这个区域会继续接入现有 runtime 能力。当前优先完成模型、角色和工具设置。';

  @override
  String get settingsAdvanced => '高级设置';

  @override
  String get settingsActive => '当前';

  @override
  String get settingsActivate => '设为当前';

  @override
  String get settingsModelCurrentSection => '当前聊天模型';

  @override
  String get settingsModelCurrentChatModel => '聊天使用';

  @override
  String get settingsModelCurrentActive => '当前激活';

  @override
  String get settingsModelSetCurrentActive => '设为当前';

  @override
  String get settingsChatThinkingMode => '思考模式';

  @override
  String get settingsChatThinkingModeDescription => '让支持的模型输出更稳的推理过程。';

  @override
  String get settingsChatStreamOutput => '流式输出';

  @override
  String get settingsChatStreamOutputDescription => '回复生成时逐步显示内容。';

  @override
  String get settingsModelProfilesSection => '模型档案';

  @override
  String get settingsModelFunctionMappingsSection => '功能模型分配';

  @override
  String get settingsModelFunctionMappingsDescription =>
      '为聊天、总结、记忆、识图等功能指定使用的模型配置和具体模型。';

  @override
  String get settingsModelFunctionMappingsReset => '全部重置';

  @override
  String get settingsModelFunctionMappingsChange => '更改';

  @override
  String settingsModelFunctionMappingsSelect(String name) {
    return '选择$name模型';
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
    return '绑定的模型不存在：$providerId · $modelId';
  }

  @override
  String settingsModelDeleteBlocked(String functions) {
    return '该模型正在被这些功能使用，请先更改功能模型分配：$functions';
  }

  @override
  String settingsModelDeleteProviderBlocked(String functions) {
    return '该供应商下的模型正在被这些功能使用，请先更改功能模型分配：$functions';
  }

  @override
  String settingsModelDeleteProviderConfirm(String name, int count) {
    return '确定删除供应商“$name”吗？这会同时删除其中的 $count 个模型。';
  }

  @override
  String get settingsModelDeleteProviderConfirmAction => '删除供应商';

  @override
  String get settingsModelChatAutoGlmWarning =>
      '禁止使用 AutoGLM 作为对话主模型。对话模型和 UI 控制模型是分离的，请选择其他大模型。';

  @override
  String get settingsModelFunctionChat => '聊天';

  @override
  String get settingsModelFunctionChatDescription => '主对话回复使用的模型。';

  @override
  String get settingsModelFunctionSummary => '总结';

  @override
  String get settingsModelFunctionSummaryDescription => '长上下文自动总结使用的模型。';

  @override
  String get settingsModelFunctionMemory => '记忆';

  @override
  String get settingsModelFunctionMemoryDescription => '记忆提取、整理和更新使用的模型。';

  @override
  String get settingsModelFunctionUiController => 'UI 控制';

  @override
  String get settingsModelFunctionUiControllerDescription =>
      '界面控制和轻量操作规划使用的模型。';

  @override
  String get settingsModelFunctionTranslation => '翻译';

  @override
  String get settingsModelFunctionTranslationDescription => '翻译文本和本地化内容使用的模型。';

  @override
  String get settingsModelFunctionGrep => '文本检索';

  @override
  String get settingsModelFunctionGrepDescription => '搜索结果筛选和文本匹配判断使用的模型。';

  @override
  String get settingsModelFunctionRoleResponsePlanner => '群聊发言规划';

  @override
  String get settingsModelFunctionRoleResponsePlannerDescription =>
      '群组对话中规划发言角色和顺序使用的模型。';

  @override
  String get settingsModelFunctionImageRecognition => '图片识别';

  @override
  String get settingsModelFunctionImageRecognitionDescription =>
      '图片理解和图片内容提取使用的模型。';

  @override
  String get settingsModelFunctionAudioRecognition => '音频识别';

  @override
  String get settingsModelFunctionAudioRecognitionDescription =>
      '音频理解和音频内容提取使用的模型。';

  @override
  String get settingsModelFunctionVideoRecognition => '视频识别';

  @override
  String get settingsModelFunctionVideoRecognitionDescription =>
      '视频理解和视频内容提取使用的模型。';

  @override
  String get settingsModelFunctionImageUnsupported => '当前模型配置未开启图片直传。';

  @override
  String get settingsModelFunctionAudioUnsupported => '当前模型配置未开启音频直传。';

  @override
  String get settingsModelFunctionVideoUnsupported => '当前模型配置未开启视频直传。';

  @override
  String get settingsModelCreateProfile => '新建模型档案';

  @override
  String get settingsModelEditProfile => '编辑模型档案';

  @override
  String get settingsModelProfileName => '档案名称';

  @override
  String get settingsModelApiEndpoint => 'API 地址';

  @override
  String get settingsModelModelNames => '模型名称';

  @override
  String get settingsModelApiKey => 'API Key';

  @override
  String get settingsModelApiKeyPool => 'API Key 池';

  @override
  String get settingsModelApiKeyPoolDescription =>
      '给同一个模型配置准备多个 Key，用于多 Key 轮换。';

  @override
  String settingsModelApiKeyPoolCount(int count) {
    return '$count 个 Key';
  }

  @override
  String get settingsModelApiKeyPoolEmpty => '还没有 Key。添加后，运行时会按当前配置使用 Key 池。';

  @override
  String get settingsModelAddApiKey => '添加 Key';

  @override
  String get settingsModelEditApiKey => '编辑 Key';

  @override
  String get settingsModelApiKeyName => 'Key 名称';

  @override
  String get settingsModelApiKeyEnabled => '启用这个 Key';

  @override
  String get settingsModelProviderId => '供应商 ID';

  @override
  String get settingsModelProvidersSection => '供应商';

  @override
  String get settingsModelProviderType => '供应商类型';

  @override
  String settingsModelProviderTypeOption(String name, String original) {
    return '$name（$original）';
  }

  @override
  String get settingsModelProviderTypeOpenai => 'OpenAI';

  @override
  String get settingsModelProviderTypeOpenaiResponses => 'OpenAI Responses';

  @override
  String get settingsModelProviderTypeOpenaiResponsesGeneric =>
      'OpenAI Responses 兼容';

  @override
  String get settingsModelProviderTypeOpenaiGeneric => 'OpenAI 兼容';

  @override
  String get settingsModelProviderTypeAnthropic => 'Anthropic';

  @override
  String get settingsModelProviderTypeAnthropicGeneric => 'Anthropic 兼容';

  @override
  String get settingsModelProviderTypeGoogle => 'Google Gemini';

  @override
  String get settingsModelProviderTypeGeminiGeneric => 'Gemini 兼容';

  @override
  String get settingsModelProviderTypeBaidu => '百度';

  @override
  String get settingsModelProviderTypeAliyun => '阿里云';

  @override
  String get settingsModelProviderTypeXunfei => '讯飞';

  @override
  String get settingsModelProviderTypeZhipu => '智谱';

  @override
  String get settingsModelProviderTypeBaichuan => '百川';

  @override
  String get settingsModelProviderTypeMoonshot => '月之暗面';

  @override
  String get settingsModelProviderTypeMimo => '小米 MiMo';

  @override
  String get settingsModelProviderTypeDeepseek => 'DeepSeek';

  @override
  String get settingsModelProviderTypeMistral => 'Mistral';

  @override
  String get settingsModelProviderTypeSiliconflow => '硅基流动';

  @override
  String get settingsModelProviderTypeIflow => '心流';

  @override
  String get settingsModelProviderTypeOpenrouter => 'OpenRouter';

  @override
  String get settingsModelProviderTypeFourRouter => '4Router';

  @override
  String get settingsModelProviderTypeNousPortal => 'Nous Portal';

  @override
  String get settingsModelProviderTypeInfiniai => '无问芯穹';

  @override
  String get settingsModelProviderTypeAlipayBailing => '支付宝百灵';

  @override
  String get settingsModelProviderTypeDoubao => '豆包';

  @override
  String get settingsModelProviderTypeNvidia => 'NVIDIA';

  @override
  String get settingsModelProviderTypeLmstudio => 'LM Studio';

  @override
  String get settingsModelProviderTypeOllama => 'Ollama';

  @override
  String get settingsModelProviderTypeOpenaiLocal => 'OpenAI 本地';

  @override
  String get settingsModelProviderTypeMnn => 'MNN';

  @override
  String get settingsModelProviderTypeLlamaCpp => 'llama.cpp';

  @override
  String get settingsModelProviderTypePpinfra => 'PPInfra';

  @override
  String get settingsModelProviderTypeNovita => 'Novita AI';

  @override
  String get settingsModelProviderTypeOther => '其他';

  @override
  String get settingsModelEditModelSettings => '模型设置';

  @override
  String get settingsModelCreateProvider => '创建供应商';

  @override
  String get settingsModelEditProvider => '编辑供应商';

  @override
  String get settingsModelAddModel => '添加模型';

  @override
  String get settingsModelAddModelShort => '添加';

  @override
  String get settingsModelCustomModel => '自定义模型';

  @override
  String get settingsModelModelId => '模型 ID';

  @override
  String get settingsModelMaxTokens => 'Max tokens';

  @override
  String get settingsModelMaxTokensDescription => '限制一次回复最多生成多少 Token。';

  @override
  String get settingsModelTemperature => 'Temperature';

  @override
  String get settingsModelTemperatureDescription => '控制随机性，越低越稳定，越高越发散。';

  @override
  String get settingsModelTopP => 'Top-p';

  @override
  String get settingsModelTopPDescription => '只从累计概率 Top-p 范围内采样。';

  @override
  String get settingsModelTopK => 'Top-k';

  @override
  String get settingsModelTopKDescription => '只从概率最高的 K 个候选 Token 中采样，0 表示关闭。';

  @override
  String get settingsModelPresencePenalty => 'Presence penalty';

  @override
  String get settingsModelPresencePenaltyDescription => '鼓励引入新话题，减少重复已有内容。';

  @override
  String get settingsModelFrequencyPenalty => 'Frequency penalty';

  @override
  String get settingsModelFrequencyPenaltyDescription => '按出现频率惩罚重复 Token。';

  @override
  String get settingsModelRepetitionPenalty => 'Repetition penalty';

  @override
  String get settingsModelRepetitionPenaltyDescription => '进一步降低重复，1.0 表示不惩罚。';

  @override
  String get settingsModelRequestLimit => '每分钟请求数';

  @override
  String get settingsModelMaxConcurrent => '最大并发请求';

  @override
  String get settingsModelContextLength => '上下文长度';

  @override
  String get settingsModelMaxContextLength => '最大上下文长度';

  @override
  String get settingsModelMaxContextLengthInvalid => '请输入大于 0 的最大上下文长度';

  @override
  String get settingsModelMaxContextMode => '最大上下文模式';

  @override
  String get settingsModelSummaryThreshold => '总结 Token 阈值';

  @override
  String get settingsModelSummaryByMessageCount => '按消息数总结';

  @override
  String get settingsModelSummaryMessageCount => '总结消息数阈值';

  @override
  String get settingsModelCustomHeaders => '自定义 Headers';

  @override
  String get settingsModelCustomParameters => '自定义参数 JSON';

  @override
  String get settingsModelToolCall => '工具调用';

  @override
  String get settingsModelToolCallDescription => '允许模型使用结构化工具调用能力。';

  @override
  String get settingsModelDirectImage => '图片直传';

  @override
  String get settingsModelDirectImageDescription => '支持图片能力的模型可直接接收图片。';

  @override
  String get settingsModelDirectAudio => '音频直传';

  @override
  String get settingsModelDirectAudioDescription => '支持音频能力的模型可直接接收音频。';

  @override
  String get settingsModelDirectVideo => '视频直传';

  @override
  String get settingsModelDirectVideoDescription => '支持视频能力的模型可直接接收视频。';

  @override
  String get settingsModelGoogleSearch => 'Google 搜索';

  @override
  String get settingsModelGoogleSearchDescription => '启用供应商侧搜索能力。';

  @override
  String get settingsModelContext => '上下文窗口';

  @override
  String get settingsModelSummary => '自动总结';

  @override
  String get settingsModelMediaHistory => '媒体历史';

  @override
  String get settingsModelCapabilities => '能力';

  @override
  String get settingsModelBuiltinTools => '内置工具';

  @override
  String get settingsModelBuiltinToolExclusive => '开启后会关闭外部工具调用';

  @override
  String get settingsModelConnectionTestSection => '连接测试';

  @override
  String get settingsModelRunConnectionTest => '测试当前模型';

  @override
  String get settingsModelTestModel => '测试模型';

  @override
  String get settingsModelTestingConnection => '正在测试当前模型连接…';

  @override
  String get settingsModelTestedModel => '测试模型';

  @override
  String get settingsModelConnectionTestPassed => '全部通过';

  @override
  String get settingsModelConnectionTestFailed => '存在失败项';

  @override
  String get settingsModelCapabilitiesApplied => '已按测试结果更新模型能力开关。';

  @override
  String get settingsModelCapabilitiesNeedChat => '聊天测试未通过，模型能力开关未更新。';

  @override
  String settingsModelConnectionTestError(String error) {
    return '连接测试失败：$error';
  }

  @override
  String get settingsModelTestItemChat => '聊天';

  @override
  String get settingsModelTestItemToolCall => '工具调用';

  @override
  String get settingsModelTestItemImage => '图片';

  @override
  String get settingsModelTestItemAudio => '音频';

  @override
  String get settingsModelTestItemVideo => '视频';

  @override
  String get settingsModelTestItemUnknown => '未知项';

  @override
  String get settingsCharactersCreateCard => '新建角色卡';

  @override
  String get settingsCharactersEditCard => '编辑角色卡';

  @override
  String get settingsCharactersCardName => '角色名称';

  @override
  String get settingsCharactersCreateGroup => '新建群组';

  @override
  String get settingsCharactersEditGroup => '编辑群组';

  @override
  String get settingsCharactersGroupName => '群组名称';

  @override
  String get settingsCharactersDescription => '描述';

  @override
  String get settingsCharactersCharacterSetting => '角色设定';

  @override
  String get settingsCharactersOpeningStatement => '开场白';

  @override
  String get settingsCharactersOtherContentChat => '聊天附加内容';

  @override
  String get settingsCharactersOtherContentVoice => '语音附加内容';

  @override
  String get settingsCharactersAdvancedPrompt => '高级自定义 Prompt';

  @override
  String get settingsCharactersMarks => '备注';

  @override
  String get settingsCharactersTags => '标签';

  @override
  String get settingsCharactersNoTags => '当前没有可选标签。可在提示词/标签管理中创建后绑定到角色卡。';

  @override
  String get settingsCharactersImport => '导入';

  @override
  String get settingsCharactersExport => '导出';

  @override
  String get settingsCharactersImportJson => '导入 JSON';

  @override
  String get settingsCharactersCopyJson => '复制 JSON';

  @override
  String get settingsCharactersImportTavernJson => '导入 Tavern JSON';

  @override
  String get settingsCharactersCopyTavernJson => '复制 Tavern JSON';

  @override
  String get settingsCharactersJsonInput => 'JSON 内容';

  @override
  String get settingsCharactersTavernJsonInput => 'Tavern JSON 内容';

  @override
  String settingsCharactersJsonCopied(String name) {
    return '已复制“$name”的 JSON。';
  }

  @override
  String settingsCharactersTavernJsonCopied(String name) {
    return '已复制“$name”的 Tavern JSON。';
  }

  @override
  String get settingsCharactersImportCardJson => '导入角色卡 JSON';

  @override
  String get settingsCharactersImportCardJsonDone => '已导入角色卡。';

  @override
  String get settingsCharactersImportTavernJsonDone => '已导入 Tavern 角色卡。';

  @override
  String get settingsCharactersImportGroupJson => '导入群组 JSON';

  @override
  String get settingsCharactersImportGroupJsonDone => '已导入群组。';

  @override
  String settingsCharactersImportJsonError(String error) {
    return '导入 JSON 失败：$error';
  }

  @override
  String settingsCharactersImportTavernJsonError(String error) {
    return '导入 Tavern JSON 失败：$error';
  }

  @override
  String settingsCharactersTavernJsonCopyError(String error) {
    return '复制 Tavern JSON 失败：$error';
  }

  @override
  String get settingsCharactersTagsSection => '标签';

  @override
  String get settingsCharactersCreateTag => '新建标签';

  @override
  String get settingsCharactersEditTag => '编辑标签';

  @override
  String get settingsCharactersDeleteTag => '删除标签';

  @override
  String settingsCharactersDeleteTagMessage(String name) {
    return '确定删除“$name”吗？';
  }

  @override
  String get settingsCharactersTagName => '标签名称';

  @override
  String get settingsCharactersTagDescription => '标签描述';

  @override
  String get settingsCharactersTagPromptContent => 'Prompt 内容';

  @override
  String get settingsCharactersChatModelBindingMode => '聊天模型绑定模式';

  @override
  String get settingsCharactersChatModelConfigId => '聊天模型配置 ID';

  @override
  String get settingsCharactersChatModelIndex => '聊天模型序号';

  @override
  String get settingsCharactersToolAccess => '工具权限模式';

  @override
  String get settingsCharactersChatModelFollowGlobal => '跟随全局模型';

  @override
  String get settingsCharactersChatModelFixedConfig => '固定模型配置';

  @override
  String get settingsCharactersChatModelConfig => '模型配置';

  @override
  String get settingsCharactersToolAccessFollowGlobal => '跟随全局工具权限';

  @override
  String get settingsCharactersToolAccessCustom => '自定义角色工具权限';

  @override
  String get settingsCharactersToolAccessEmpty => '已启用，但未选择任何工具';

  @override
  String settingsCharactersToolAccessSummaryCounts(
    int builtinCount,
    int packageCount,
    int skillCount,
    int mcpCount,
  ) {
    return '内置 $builtinCount · 工具包 $packageCount · 技能 $skillCount · MCP $mcpCount';
  }

  @override
  String get settingsCharactersToolAccessConfigure => '配置工具白名单';

  @override
  String get settingsCharactersToolAccessRequiresUsePackage =>
      '选择工具包、技能或 MCP 时，需要同时允许内置工具 use_package。';

  @override
  String get settingsCharactersToolAccessEmptyBuiltin => '当前没有可选内置工具。';

  @override
  String get settingsCharactersToolAccessEmptyPackages => '当前没有可选工具包。';

  @override
  String get settingsCharactersToolAccessEmptySkills => '当前没有可选技能。';

  @override
  String get settingsCharactersToolAccessEmptyMcp => '当前没有可选 MCP 服务。';

  @override
  String get settingsCharactersBuiltinTools => '允许的内置工具';

  @override
  String get settingsCharactersAllowedPackages => '允许的工具包';

  @override
  String get settingsCharactersAllowedSkills => '允许的技能';

  @override
  String get settingsCharactersAllowedMcpServers => '允许的 MCP 服务';

  @override
  String get settingsCharactersGroupMembersTitle => '组内角色';

  @override
  String get settingsCharactersOpenMemoryGraph => '查看记忆图谱';

  @override
  String settingsCharactersMemoryGraphTitle(String profileName) {
    return '$profileName 的记忆图谱';
  }

  @override
  String get settingsCharactersMemoryGraphEmpty => '当前记忆库还没有节点';

  @override
  String settingsCharactersMemoryGraphStats(int nodes, int edges) {
    return '$nodes 个节点 · $edges 条关系';
  }

  @override
  String get settingsCharactersMemoryGraphLink => '记忆关系';

  @override
  String get settingsCharactersEditUserMarkdown => '编辑 USER.md';

  @override
  String settingsCharactersUserMarkdownTitle(String profileName) {
    return '$profileName 的 USER.md';
  }

  @override
  String get settingsCharactersUserMarkdownSaved => 'USER.md 已保存';

  @override
  String get settingsCharactersMemoryAutoUpdate => '自动更新记忆偏好';

  @override
  String get settingsCharactersMemoryAutoUpdateDescription =>
      '允许 AI 从对话中更新用户偏好与长期记忆。';

  @override
  String get settingsCharactersPreferenceDescription => '向模型提供用户偏好';

  @override
  String get settingsCharactersPreferenceDescriptionSubtitle =>
      '聊天时把当前偏好档案写入提示词。';

  @override
  String get settingsCharactersCardsSection => '角色卡';

  @override
  String get settingsCharactersGroupsSection => '群组';

  @override
  String settingsCharactersGroupMembers(int count) {
    return '$count 个成员';
  }

  @override
  String get settingsToolsPermissionMode => '工具权限模式';

  @override
  String get settingsToolsAsk => '询问';

  @override
  String get settingsToolsExtensions => '扩展管理';

  @override
  String get settingsToolsPlugins => '插件';

  @override
  String get settingsToolsPluginsDescription => '管理 ToolPkg 插件容器和 UI 扩展。';

  @override
  String get settingsToolsPackages => '工具包';

  @override
  String get settingsToolsPackagesDescription => '启用、停用和查看内置/外部工具包。';

  @override
  String get settingsToolsSkills => '技能';

  @override
  String get settingsToolsSkillsDescription => '管理技能包可见性和导入。';

  @override
  String get settingsToolsMcp => 'MCP 服务';

  @override
  String settingsToolsMcpDescription(int seconds) {
    return '管理 MCP 配置，当前启动等待 $seconds 秒。';
  }

  @override
  String get settingsToolsOverrides => '单工具权限记录';

  @override
  String get settingsToolsToolGroups => '逐工具权限';

  @override
  String get settingsToolsToolGroupsDescription =>
      '常用情况下保持“询问”即可；明确可信的工具放到允许组，危险或不想让 AI 用的工具放到禁止组。';

  @override
  String get settingsToolsAlwaysAllow => '始终允许';

  @override
  String get settingsToolsAlwaysAllowDescription => '这些工具执行时不再询问。';

  @override
  String get settingsToolsAlwaysForbid => '始终禁止';

  @override
  String get settingsToolsAlwaysForbidDescription => '这些工具不会被 AI 调用。';

  @override
  String get settingsToolsAddTool => '添加工具';

  @override
  String get settingsToolsAddAllowTool => '添加允许工具';

  @override
  String get settingsToolsAddForbidTool => '添加禁止工具';

  @override
  String get settingsToolsSearchTools => '搜索工具';

  @override
  String get settingsToolsNoToolsInGroup => '当前没有工具。';

  @override
  String get settingsToolsMcpStartupTimeout => 'MCP 启动超时';

  @override
  String get settingsToolsMcpStartupTimeoutSeconds => '等待秒数';

  @override
  String get settingsWorkspaceCurrentDesign => '当前工作区结构';

  @override
  String get settingsWorkspaceCurrentDesignDescription =>
      '工作区跟随聊天绑定；终端会话和浏览器会话作为全局会话在工作区里扁平展示。';

  @override
  String get settingsWorkspaceOpenChat => '回到聊天工作区';

  @override
  String get settingsWorkspaceOpenChatDescription => '在聊天右侧打开文件、终端、浏览器和网页自动化。';

  @override
  String get settingsWorkspaceContains => '工作区包含';

  @override
  String get settingsWorkspacePerChat => '按聊天绑定';

  @override
  String get settingsWorkspaceGlobalSessions => '全局终端会话';

  @override
  String get settingsWorkspaceBrowserSessions => '浏览器与 WebVisit 会话';

  @override
  String get settingsWorkspaceBoundOverview => '工作区绑定概览';

  @override
  String get settingsWorkspaceBoundOverviewDescription => '聊天记录里的工作区路径就是绑定来源。';

  @override
  String get settingsWorkspaceBoundChats => '已绑定聊天';

  @override
  String get settingsWorkspaceInternalRoot => '内部工作区根目录';

  @override
  String get settingsWorkspaceExternalRoot => '旧外部工作区根目录';

  @override
  String get settingsWorkspaceUnboundTitle => '无绑定工作区';

  @override
  String get settingsWorkspaceUnboundSubtitle => '这些工作区文件夹没有被任何聊天使用。';

  @override
  String get settingsWorkspaceNoUnbound => '没有无绑定工作区。';

  @override
  String settingsWorkspaceSelectedCount(int selected, int total) {
    return '已选择 $selected / $total';
  }

  @override
  String get settingsWorkspaceSelectAllCurrentList => '全选';

  @override
  String get settingsWorkspaceClearAll => '清空';

  @override
  String get settingsWorkspaceInternalStorage => '内部存储';

  @override
  String get settingsWorkspaceExternalStorage => '外部存储';

  @override
  String get settingsWorkspaceNotUsedByAnyChat => '未被任何聊天使用';

  @override
  String settingsWorkspaceDeleteSelected(int count) {
    return '删除选中的工作区（$count）';
  }

  @override
  String get settingsWorkspaceConfirmDeleteTitle => '确认删除';

  @override
  String settingsWorkspaceDeleteConfirmation(int count) {
    return '确定删除选中的 $count 个工作区文件夹吗？';
  }

  @override
  String settingsWorkspaceDeleted(int count) {
    return '已删除 $count 个无绑定工作区。';
  }

  @override
  String settingsWorkspaceDeleteFailed(String error) {
    return '删除失败：$error';
  }

  @override
  String settingsWorkspaceLoadFailed(String error) {
    return '加载工作区失败：$error';
  }

  @override
  String get settingsWorkspaceRefresh => '刷新';

  @override
  String get settingsRuntimeConnection => '当前设备';

  @override
  String get settingsRuntimeConnectionDescription =>
      '选择让聊天和工具运行在这台设备，还是运行在另一台已连接设备。';

  @override
  String get settingsRuntimeUseLocal => '使用这台设备';

  @override
  String get settingsRuntimeLocalTitle => '这台设备';

  @override
  String get settingsRuntimeLocalDescription => '聊天和工具会在这台设备上运行。';

  @override
  String get settingsRuntimeRemoteTitle => '已连接的设备';

  @override
  String get settingsRuntimeRemoteDescription => '这些设备已经完成连接，可以切换使用或移除。';

  @override
  String get settingsRuntimeTesting => '正在检查连接…';

  @override
  String get settingsRuntimePairRemote => '连接另一台设备';

  @override
  String get settingsRuntimeNoPairedRemote => '还没有已连接的设备。';

  @override
  String get settingsRuntimePairToken => '连接 Token';

  @override
  String get settingsRuntimePairCode => '配对码';

  @override
  String get settingsRuntimeStartPairing => '开始连接';

  @override
  String get settingsRuntimeFinishPairing => '完成连接';

  @override
  String get settingsRuntimeBaseUrl => '设备地址';

  @override
  String get settingsRuntimeTestCurrent => '检查连接';

  @override
  String get settingsRuntimeSwitchedLocal => '已切换到这台设备。';

  @override
  String get settingsRuntimeSwitchedRemote => '已切换到已连接设备。';

  @override
  String settingsRuntimeTestResult(String version) {
    return '设备可用：$version';
  }

  @override
  String settingsRuntimeTestFailed(String error) {
    return '连接检查失败：$error';
  }

  @override
  String get settingsRuntimeRemoteDisconnected => '已连接设备断开';

  @override
  String settingsRuntimeRemoteDisconnectedMessage(String error) {
    return '已切换到这台设备。\n\n$error';
  }

  @override
  String get settingsRuntimePairingRejected => '有设备的连接请求被拒绝';

  @override
  String get settingsRuntimePairedChecking => '检测中';

  @override
  String get settingsRuntimePairedOnline => '在线';

  @override
  String get settingsRuntimePairedOffline => '离线';

  @override
  String get settingsRuntimeSync => '同步数据';

  @override
  String get settingsRuntimeSyncing => '同步中…';

  @override
  String settingsRuntimeSyncCompleted(int localApplied, int remoteApplied) {
    return '同步完成：本机 $localApplied 项，远端 $remoteApplied 项。';
  }

  @override
  String settingsRuntimeSyncFailed(String error) {
    return '同步失败：$error';
  }

  @override
  String get settingsRuntimeDiscoverDevices => '发现设备';

  @override
  String get settingsRuntimeDiscoverDevicesDescription => '扫描附近设备或手动输入地址来建立连接。';

  @override
  String get settingsRuntimeScan => '扫描';

  @override
  String get settingsRuntimeScanning => '扫描中…';

  @override
  String get settingsRuntimeEnterManually => '手动输入';

  @override
  String get settingsRuntimeConnect => '连接';

  @override
  String get settingsRuntimeEnableDiscovery => '允许其他设备发现这台设备';

  @override
  String get settingsRuntimeEnableDiscoveryDescription =>
      '开启后，同网络下的设备能自动搜索到这台设备。';

  @override
  String settingsRuntimeEnableDiscoveryFailed(String error) {
    return '无法开启设备发现：$error';
  }

  @override
  String settingsRuntimeDisableDiscoveryFailed(String error) {
    return '无法关闭设备发现：$error';
  }

  @override
  String get settingsRuntimeUsingLocal => '正在使用：这台设备';

  @override
  String settingsRuntimeUsingRemote(String device) {
    return '正在使用：$device';
  }

  @override
  String get settingsRuntimeRemoteInUseDescription => '聊天和工具会在这台已连接设备上运行。';

  @override
  String get settingsWebAccessService => '允许访问';

  @override
  String get settingsWebAccessServiceDescription => '开启后，浏览器可以通过地址和口令访问这台设备。';

  @override
  String get settingsWebAccessEnable => '允许外部访问';

  @override
  String get settingsWebAccessPortMode => '端口模式';

  @override
  String get settingsWebAccessPortAutomatic => '自动';

  @override
  String get settingsWebAccessPortFixed => '固定';

  @override
  String get settingsWebAccessPortAutomaticDescription => '端口由系统自动选择，不需要手动配置。';

  @override
  String get settingsWebAccessPortFixedDescription => '只使用监听地址中填写的端口。';

  @override
  String get settingsWebAccessBindAddress => '监听地址';

  @override
  String get settingsWebAccessToken => '访问口令';

  @override
  String get settingsWebAccessRotateToken => '更换口令';

  @override
  String get settingsWebAccessCopyToken => '复制口令';

  @override
  String get settingsWebAccessAccessUrl => '访问地址';

  @override
  String get settingsWebAccessLocalUrl => '本机';

  @override
  String get settingsWebAccessPairingUrl => '配对地址';

  @override
  String get settingsWebAccessPairingUrlLocalOnly => '仅本机可用';

  @override
  String get settingsWebAccessPairingUrlUnavailable => '未找到局域网地址';

  @override
  String get settingsWebAccessCopyUrl => '复制地址';

  @override
  String get settingsWebAccessOpenUrl => '打开地址';

  @override
  String get settingsWebAccessRunning => '已开启';

  @override
  String get settingsWebAccessStopped => '已关闭';

  @override
  String get settingsWebAccessSaved => '访问设置已保存。';

  @override
  String get settingsWebAccessTokenCopied => '访问口令已复制。';

  @override
  String get settingsWebAccessUrlCopied => '访问地址已复制。';

  @override
  String get settingsWebAccessPairedClients => '已授权设备';

  @override
  String get settingsWebAccessNoPairedClients => '还没有设备被授权。';

  @override
  String get settingsWebAccessPairedDeleted => '已删除授权设备。';

  @override
  String get settingsWebAccessPairingRequest => '配对请求';

  @override
  String settingsWebAccessPairingRequestMessage(String code, String client) {
    return '配对码：$code\n设备：$client';
  }

  @override
  String get settingsWebAccessInvalidBindAddress => '绑定地址格式应为 host:port。';

  @override
  String settingsWebAccessStartFailed(String error) {
    return '开启访问失败：$error';
  }

  @override
  String settingsWebAccessStopFailed(String error) {
    return '关闭访问失败：$error';
  }

  @override
  String get settingsAppearanceThemeSection => '主题';

  @override
  String get settingsAppearanceThemeMode => '当前模式';

  @override
  String get settingsAppearanceThemeTarget => '主题保存目标';

  @override
  String get settingsAppearanceThemeTargetGlobal => '全局';

  @override
  String settingsAppearanceThemeTargetCharacter(Object name) {
    return '当前角色：$name';
  }

  @override
  String settingsAppearanceThemeTargetGroup(Object name) {
    return '当前群组：$name';
  }

  @override
  String get settingsAppearanceThemeSystem => '跟随系统';

  @override
  String get settingsAppearanceThemeLight => '浅色';

  @override
  String get settingsAppearanceThemeDark => '深色';

  @override
  String get settingsAppearanceColorSection => '主题色';

  @override
  String get settingsAppearanceColorDescription =>
      '选择一个简单的颜色预设。系统栏和当前应用外壳会自动跟随主题，不再单独配置。';

  @override
  String get settingsAppearanceColorDefault => '默认';

  @override
  String get settingsAppearanceColorSky => '天蓝';

  @override
  String get settingsAppearanceColorMatcha => '抹茶';

  @override
  String get settingsAppearanceColorEmber => '暖橙';

  @override
  String get settingsAppearanceColorRose => '玫瑰';

  @override
  String get settingsAppearanceColorCustom => '自定义颜色';

  @override
  String get settingsAppearanceCustomColorsTitle => '自定义主题色';

  @override
  String get settingsAppearancePrimaryColor => '主色';

  @override
  String get settingsAppearanceSecondaryColor => '辅色';

  @override
  String get settingsAppearanceHexColorHint => '#RRGGBB';

  @override
  String get settingsAppearanceHexColorInvalid => '请输入 #RRGGBB 格式的颜色';

  @override
  String get settingsAppearanceBackgroundSection => '背景';

  @override
  String get settingsAppearanceBackgroundDescription =>
      '选择本地图片或视频作为应用背景，界面底色和系统栏会自动跟随主题。';

  @override
  String get settingsAppearanceBackgroundImage => '背景媒体';

  @override
  String get settingsAppearanceBackgroundNone => '未选择';

  @override
  String get settingsAppearanceBackgroundChooseImage => '选择图片';

  @override
  String get settingsAppearanceBackgroundChooseVideo => '选择视频';

  @override
  String get settingsAppearanceBackgroundDisable => '关闭背景';

  @override
  String get settingsAppearanceBackgroundEnabled => '启用背景';

  @override
  String get settingsAppearanceBackgroundOpacity => '背景透明度';

  @override
  String get settingsAppearanceBackgroundBlur => '背景模糊';

  @override
  String get settingsAppearanceBackgroundBlurRadius => '模糊强度';

  @override
  String get settingsAppearanceBackgroundVideoMuted => '视频背景静音';

  @override
  String get settingsAppearanceBackgroundVideoLoop => '视频背景循环';

  @override
  String get settingsAppearanceTextSection => '文字';

  @override
  String get settingsAppearanceFontFamily => '字体';

  @override
  String get settingsAppearanceFontDefault => '默认';

  @override
  String get settingsAppearanceCustomFont => '自定义字体';

  @override
  String get settingsAppearanceFontCustom => '自定义';

  @override
  String get settingsAppearanceChooseCustomFont => '选择自定义字体';

  @override
  String get settingsAppearanceClearCustomFont => '清除自定义字体';

  @override
  String get settingsAppearanceFontSerif => '衬线';

  @override
  String get settingsAppearanceFontMonospace => '等宽';

  @override
  String get settingsAppearanceFontScale => '字体大小';

  @override
  String get settingsAppearanceAvatarSection => '头像';

  @override
  String get settingsAppearanceUserAvatar => '用户头像';

  @override
  String get settingsAppearanceAiAvatar => 'AI 头像';

  @override
  String get settingsAppearanceAvatarDefault => '默认头像';

  @override
  String get settingsAppearanceAvatarShape => '头像形状';

  @override
  String get settingsAppearanceAvatarShapeCircle => '圆形';

  @override
  String get settingsAppearanceAvatarShapeSquare => '方形';

  @override
  String get settingsAppearanceChooseUserAvatar => '选择用户头像';

  @override
  String get settingsAppearanceChooseAiAvatar => '选择 AI 头像';

  @override
  String get settingsAppearanceClearUserAvatar => '清除用户头像';

  @override
  String get settingsAppearanceClearAiAvatar => '清除 AI 头像';

  @override
  String get settingsAppearanceChatDisplaySection => '聊天显示';

  @override
  String get settingsAppearanceMessageStyle => '消息样式';

  @override
  String get settingsAppearanceMessageStyleClean => '命令式';

  @override
  String get settingsAppearanceMessageStyleCard => '气泡式';

  @override
  String get settingsAppearanceMessageColors => '消息配色';

  @override
  String get settingsAppearanceMessageColorsTheme => '跟随主题';

  @override
  String get settingsAppearanceMessageColorsSky => '清爽蓝';

  @override
  String get settingsAppearanceMessageColorsMatcha => '抹茶';

  @override
  String get settingsAppearanceMessageColorsInk => '深色';

  @override
  String get settingsAppearanceMessageColorsCustom => '自定义消息颜色';

  @override
  String get settingsAppearanceCustomMessageColorsTitle => '自定义消息颜色';

  @override
  String get settingsAppearanceCursorUserBubbleColor => '命令式用户气泡';

  @override
  String get settingsAppearanceUserBubbleColor => '用户气泡';

  @override
  String get settingsAppearanceAiBubbleColor => 'AI 气泡';

  @override
  String get settingsAppearanceUserTextColor => '用户文字';

  @override
  String get settingsAppearanceAiTextColor => 'AI 文字';

  @override
  String get settingsAppearanceMessageSurface => '全局质感';

  @override
  String get settingsAppearanceMessageSurfaceNormal => '普通';

  @override
  String get settingsAppearanceMessageSurfaceTransparent => '透明';

  @override
  String get settingsAppearanceUserBubbleFont => '用户气泡字体';

  @override
  String get settingsAppearanceAiBubbleFont => 'AI 气泡字体';

  @override
  String get settingsAppearanceAdjustUserBubbleFont => '调整用户气泡字体';

  @override
  String get settingsAppearanceAdjustAiBubbleFont => '调整 AI 气泡字体';

  @override
  String get settingsAppearanceEnableBubbleFont => '启用气泡专属字体';

  @override
  String get settingsAppearanceUserBubbleImage => '用户气泡图片';

  @override
  String get settingsAppearanceAiBubbleImage => 'AI 气泡图片';

  @override
  String get settingsAppearanceChooseUserBubbleImage => '选择用户气泡';

  @override
  String get settingsAppearanceChooseAiBubbleImage => '选择 AI 气泡';

  @override
  String get settingsAppearanceClearUserBubbleImage => '清除用户气泡';

  @override
  String get settingsAppearanceClearAiBubbleImage => '清除 AI 气泡';

  @override
  String get settingsAppearanceBubbleImageRenderMode => '气泡图片模式';

  @override
  String get settingsAppearanceBubbleImageTiledNineSlice => '平铺九宫格';

  @override
  String get settingsAppearanceBubbleImageNinePatch => '拉伸九宫格';

  @override
  String get settingsAppearanceBubbleImageAdjustUser => '调整用户气泡图片';

  @override
  String get settingsAppearanceBubbleImageAdjustAi => '调整 AI 气泡图片';

  @override
  String get settingsAppearanceBubbleImagePreview => '预览';

  @override
  String get settingsAppearanceBubbleImagePreviewText => '这是一条气泡预览，切线展示九宫格区域';

  @override
  String get settingsAppearanceBubbleImageCrop => '裁切';

  @override
  String get settingsAppearanceBubbleImageRepeat => '重复区域';

  @override
  String get settingsAppearanceBubbleImageScale => '图片缩放';

  @override
  String get settingsAppearanceBubbleImageCropLeft => '左侧裁切';

  @override
  String get settingsAppearanceBubbleImageCropTop => '顶部裁切';

  @override
  String get settingsAppearanceBubbleImageCropRight => '右侧裁切';

  @override
  String get settingsAppearanceBubbleImageCropBottom => '底部裁切';

  @override
  String get settingsAppearanceBubbleImageRepeatStart => '横向重复起点';

  @override
  String get settingsAppearanceBubbleImageRepeatEnd => '横向重复终点';

  @override
  String get settingsAppearanceBubbleImageRepeatYStart => '纵向重复起点';

  @override
  String get settingsAppearanceBubbleImageRepeatYEnd => '纵向重复终点';

  @override
  String get settingsAppearanceMessageDensity => '消息间距';

  @override
  String get settingsAppearanceMessageDensityComfortable => '舒适';

  @override
  String get settingsAppearanceMessageDensityCompact => '紧凑';

  @override
  String get settingsAppearanceWideLayout => '使用更宽的聊天布局';

  @override
  String get settingsAppearanceRoundedMessages => '消息卡片圆角';

  @override
  String get settingsAppearanceShowAvatars => '显示消息头像';

  @override
  String get settingsAppearanceShowThinkingProcess => '显示思考过程';

  @override
  String get settingsAppearanceShowRoleName => '显示角色名';

  @override
  String get settingsAppearanceShowUserName => '显示用户名';

  @override
  String get settingsAppearanceShowModelName => '显示模型名';

  @override
  String get settingsAppearanceShowModelProvider => '显示模型供应商';

  @override
  String get settingsAppearanceShowMessageTokenStats => '显示 Token 统计';

  @override
  String get settingsAppearanceShowMessageTimingStats => '显示耗时统计';

  @override
  String get settingsAppearanceShowMessageTimestamp => '显示消息时间';

  @override
  String get settingsAppearanceShowInputProcessingStatus => '显示输入处理状态';

  @override
  String get settingsAppearanceResetTheme => '重置主题设置';

  @override
  String get settingsAppearanceLanguageSection => '语言';

  @override
  String get settingsAppearanceLanguage => '当前语言';

  @override
  String get settingsAppearanceLanguageDescription => '语言跟随应用启动时的本地化配置。';

  @override
  String get settingsDataRuntimeSection => '数据概览';

  @override
  String get settingsDataCoreVersion => '当前版本';

  @override
  String get settingsDataTokenSection => '使用统计';

  @override
  String get settingsDataInputTokens => '输入';

  @override
  String get settingsDataOutputTokens => '输出';

  @override
  String get settingsDataRefreshTokenStats => '刷新统计';

  @override
  String get settingsDataResetTokenStats => '重置统计';

  @override
  String get settingsDataBackupSection => '备份与恢复';

  @override
  String get settingsDataChatHistoriesBackup => '聊天数据';

  @override
  String get settingsDataChatHistoriesBackupDescription =>
      '备份全部聊天和消息；恢复时按聊天 ID 更新或新增。';

  @override
  String get settingsDataCharacterCardsBackup => '角色卡数据';

  @override
  String get settingsDataCharacterCardsBackupDescription =>
      '备份全部角色卡和已引用标签；恢复时按原 ID 更新或新增。';

  @override
  String get settingsDataCharacterGroupsBackup => '群组数据';

  @override
  String get settingsDataCharacterGroupsBackupDescription =>
      '备份全部群组；恢复时保留组内角色引用和顺序。';

  @override
  String get settingsDataModelConfigsBackup => '模型配置';

  @override
  String get settingsDataModelConfigsBackupDescription =>
      '备份全部模型配置，包含模型参数和 API Key 池。';

  @override
  String settingsDataBackupCount(int count) {
    return '当前 $count 项';
  }

  @override
  String get settingsDataCopyBackupJson => '复制备份';

  @override
  String get settingsDataImportBackupJson => '恢复数据';

  @override
  String get settingsDataBackupJsonInput => '恢复内容';

  @override
  String settingsDataBackupCopied(String name) {
    return '已复制“$name”备份。';
  }

  @override
  String settingsDataBackupImportResult(
    int newCount,
    int updatedCount,
    int skippedCount,
  ) {
    return '恢复完成：新增 $newCount，更新 $updatedCount，跳过 $skippedCount。';
  }

  @override
  String settingsDataBackupImportError(String error) {
    return '恢复失败：$error';
  }

  @override
  String settingsDataBackupCopyError(String error) {
    return '复制失败：$error';
  }

  @override
  String get settingsDataSnapshotBackupTitle => '完整快照';

  @override
  String get settingsDataExportRawSnapshot => '导出快照';

  @override
  String get settingsDataImportRawSnapshot => '恢复快照';

  @override
  String get settingsDataExportRawSnapshotDescription =>
      '把聊天、角色、模型设置和本地文件打包成一个备份文件。恢复时会用备份里的数据覆盖当前数据。';

  @override
  String settingsDataSnapshotBytes(int bytes) {
    return '快照大小：$bytes 字节';
  }

  @override
  String get settingsDataSnapshotImported => '快照恢复完成。';

  @override
  String settingsDataSnapshotExportError(String error) {
    return '快照导出失败：$error';
  }

  @override
  String settingsDataSnapshotImportError(String error) {
    return '快照恢复失败：$error';
  }

  @override
  String get settingsDataSnapshotRestoreConfirmTitle => '恢复完整快照';

  @override
  String settingsDataSnapshotRestoreConfirmMessage(
    int formatVersion,
    int fileCount,
    String createdAt,
    int bytes,
  ) {
    return '恢复会替换当前 runtime 数据。\n格式版本：$formatVersion\n文件数量：$fileCount\n创建时间：$createdAt\n快照大小：$bytes 字节';
  }

  @override
  String get settingsDataSnapshotRestoreConfirmAction => '确认恢复';

  @override
  String get settingsDataAdvancedBackupOptions => '高级选项';

  @override
  String get settingsDataAdvancedBackupOptionsDescription => '单项 JSON 导出与恢复';
}
