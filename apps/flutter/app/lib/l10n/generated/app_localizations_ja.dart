// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get askOperitHint => 'Operitに相談';

  @override
  String get aiChat => 'AIチャット';

  @override
  String get fullscreenInput => '入力欄を全画面表示';

  @override
  String get expandInput => '入力欄を広げる';

  @override
  String get collapseInput => '入力欄を戻す';

  @override
  String get settings => '設定';

  @override
  String get packageManager => 'パッケージ管理';

  @override
  String get market => 'マーケット';

  @override
  String get addAttachment => '添付を追加';

  @override
  String get attachmentPhoto => '写真';

  @override
  String get attachmentCamera => 'カメラ';

  @override
  String get attachmentMemory => 'メモリー';

  @override
  String get attachmentFile => 'ファイル';

  @override
  String get attachmentScreenContent => '画面の内容';

  @override
  String get attachmentNotifications => '現在の通知';

  @override
  String get attachmentLocation => '現在地';

  @override
  String get attachmentPackage => 'パッケージ';

  @override
  String get attachmentPackageSelectTitle => 'パッケージを選択';

  @override
  String get attachmentPackageEmpty => '利用できるパッケージはありません';

  @override
  String get attachmentPackageSearchPlaceholder => 'パッケージ名や説明を検索';

  @override
  String get attachmentPackageSearchEmpty => '一致するパッケージはありません';

  @override
  String get attachmentPackageKindPackage => 'パッケージ';

  @override
  String get attachmentPackageKindSkill => 'スキル';

  @override
  String get attachmentPackageKindMcp => 'MCP';

  @override
  String get attachmentCameraUnavailable => 'Flutter版ではカメラ撮影をまだ利用できません';

  @override
  String get attachmentMemoryUnavailable => 'Flutter版ではメモリーフォルダーをまだ選択できません';

  @override
  String get clearSearch => '検索をクリア';

  @override
  String chatPendingQueueTitle(int count) {
    return '送信待ちのメッセージ（$count件）';
  }

  @override
  String get chatQueueAddMessage => '送信待ちに追加';

  @override
  String get chatQueueAdded => '送信待ちに追加しました';

  @override
  String get chatPleaseCreateNewChat => '新しいチャットを作成してください';

  @override
  String get cancel => 'キャンセル';

  @override
  String get send => '送信';

  @override
  String get model => 'モデル';

  @override
  String get processingInput => '入力を処理しています…';

  @override
  String get processingMessage => 'メッセージを処理しています…';

  @override
  String get connectingAiService => 'AIサービスに接続しています…';

  @override
  String get receivingAiResponse => 'AIの回答を受信しています…';

  @override
  String get receivingToolResultAiResponse => 'ツール実行後のAI回答を受信しています…';

  @override
  String get roleResponsePlannerPlanning => 'グループの発言順を考えています…';

  @override
  String roleResponsePlannerMemberReplying(String memberName) {
    return '「$memberName」の回答を作成しています…';
  }

  @override
  String get roleResponsePlannerFailed => 'グループの発言計画に失敗しました';

  @override
  String get summarizingMemories => 'メモリーを要約しています…';

  @override
  String get executingPlan => '計画を実行しています…';

  @override
  String executingTool(String toolName) {
    return 'ツールを実行しています: $toolName';
  }

  @override
  String processingToolResult(String toolName) {
    return 'ツールの結果を処理しています: $toolName';
  }

  @override
  String get statusWarningAiErrorSummary => 'AIでエラーが発生しました';

  @override
  String get statusWarningAiErrorDetailTitle => 'AIエラーの原因';

  @override
  String get toolRunning => 'ツールを実行しています…';

  @override
  String toolRunningWithName(String toolName) {
    return '$toolName: 実行中…';
  }

  @override
  String toolStatusWithName(String toolName, String message) {
    return '$toolName: $message';
  }

  @override
  String get close => '閉じる';

  @override
  String get create => '作成';

  @override
  String get save => '保存';

  @override
  String get delete => '削除';

  @override
  String get search => '検索';

  @override
  String get loading => '読み込み中';

  @override
  String get toolApprovalTitle => 'ツールの許可確認';

  @override
  String get toolApprovalToolLabel => 'ツール';

  @override
  String get toolApprovalActionLabel => '実行内容';

  @override
  String get toolApprovalDeny => '許可しない';

  @override
  String get toolApprovalAllowOnce => '今回だけ許可';

  @override
  String get toolApprovalAlwaysAllow => 'このセッションでは常に許可';

  @override
  String get createGroupTitle => '新しいグループ';

  @override
  String get groupNameLabel => 'グループ名';

  @override
  String get renameConversationTitle => 'タイトルを変更';

  @override
  String get newTitleLabel => '新しいタイトル';

  @override
  String get deleteConversationTitle => '会話を削除しますか？';

  @override
  String deleteConversationMessage(String title) {
    return '「$title」を削除しますか？';
  }

  @override
  String get chatHistory => 'チャット履歴';

  @override
  String get editTitle => 'タイトルを変更';

  @override
  String get moveUp => '上へ移動';

  @override
  String get moveDown => '下へ移動';

  @override
  String get pin => '固定';

  @override
  String get unpin => '固定を解除';

  @override
  String get lock => 'ロック';

  @override
  String get unlock => 'ロックを解除';

  @override
  String get chatLockedCannotDelete => 'このチャットはロックされているため削除できません';

  @override
  String get messageLocatorTitle => 'メッセージを探す';

  @override
  String messageLocatorCurrent(int current, int total) {
    return '$current / $total';
  }

  @override
  String get messageLocatorSearchHint => 'メッセージの内容を検索';

  @override
  String get messageLocatorInstruction => '一覧をスクロールするか検索してメッセージへ移動します';

  @override
  String messageLocatorResultCount(int count) {
    return '$count件';
  }

  @override
  String get messageLocatorNoMatches => '一致するメッセージはありません';

  @override
  String get messageSenderUser => 'あなた';

  @override
  String get messageSenderSummary => '要約';

  @override
  String get messageSenderSystem => 'システム';

  @override
  String get messageSenderThinking => '思考中';

  @override
  String get thinkingProcess => '考えた内容';

  @override
  String thinkingToolsGroupTitleWithCount(int count) {
    return '思考とツール実行（$count件）';
  }

  @override
  String toolsGroupTitleWithCount(int count) {
    return 'ツール実行（$count件）';
  }

  @override
  String get messageSenderOther => 'その他';

  @override
  String get hiddenUserMessage => '非表示のユーザーメッセージ';

  @override
  String get workspaceSetupTitle => '作業フォルダーを準備';

  @override
  String get workspaceSetupSubtitle => 'AIに作業を任せるための専用フォルダーを用意します';

  @override
  String get workspaceCreateDefaultTitle => '新しく作成';

  @override
  String get workspaceCreateDefaultDescription => 'アプリ内に新しい作業フォルダーを作成します';

  @override
  String get workspaceBindExistingTitle => '既存フォルダーを選択';

  @override
  String get workspaceBindExistingDescription => 'この端末にあるフォルダーを使用します';

  @override
  String get workspaceProjectTypeDialogTitle => 'プロジェクトの種類を選択';

  @override
  String get workspaceProjectTypeDialogDescription => '作成する作業フォルダーの種類を選んでください';

  @override
  String get workspaceBindDialogTitle => '既存の作業フォルダーを選択';

  @override
  String get workspacePathLabel => '選択中の作業フォルダー';

  @override
  String get workspaceEnvLabel => '作業環境';

  @override
  String get optionalHint => 'オプション';

  @override
  String get workspacePathRequired => '作業フォルダーを選択してください';

  @override
  String get bind => 'バインド';

  @override
  String get workspaceProjectBlankTitle => '空の作業フォルダー';

  @override
  String get workspaceProjectBlankDescription => 'ひな形なしの空フォルダーを作成します';

  @override
  String get workspaceProjectOfficeTitle => '文書作業';

  @override
  String get workspaceProjectOfficeDescription => '文書編集、ファイル処理などの一般的な事務作業向け';

  @override
  String get workspaceProjectWebTitle => 'Webプロジェクト';

  @override
  String get workspaceProjectWebDescription =>
      'HTML、CSS、JavaScriptと自動ローカルサーバーを使うWeb開発向け';

  @override
  String get workspaceProjectAndroidTitle => 'Androidプロジェクト';

  @override
  String get workspaceProjectAndroidDescription =>
      'よく使うGradle操作を備えたAndroid開発向け';

  @override
  String get workspaceProjectFlutterTitle => 'Flutterプロジェクト';

  @override
  String get workspaceProjectFlutterDescription =>
      '安定したひな形と基本操作を備えたFlutter開発向け';

  @override
  String get workspaceProjectNodeTitle => 'Node.jsプロジェクト';

  @override
  String get workspaceProjectNodeDescription => 'npmの基本操作を備えたNode.jsバックエンド開発向け';

  @override
  String get workspaceProjectTypeScriptTitle => 'TypeScriptプロジェクト';

  @override
  String get workspaceProjectTypeScriptDescription =>
      'TypeScriptとpnpmを使う型安全な開発向け';

  @override
  String get workspaceProjectPythonTitle => 'Pythonプロジェクト';

  @override
  String get workspaceProjectPythonDescription => 'pipとHTTPサーバーを使うPython開発向け';

  @override
  String get workspaceProjectJavaTitle => 'Javaプロジェクト';

  @override
  String get workspaceProjectJavaDescription => 'GradleとMavenを使うJava開発向け';

  @override
  String get workspaceProjectGoTitle => 'Goプロジェクト';

  @override
  String get workspaceProjectGoDescription => 'go modとビルド操作を備えたGo開発向け';

  @override
  String get version => 'バージョン';

  @override
  String get author => '著者';

  @override
  String get entry => 'エントリー';

  @override
  String get source => 'ソース';

  @override
  String get category => 'カテゴリ';

  @override
  String get defaultStatus => 'デフォルトステータス';

  @override
  String get builtIn => '内蔵';

  @override
  String get external => '外部';

  @override
  String get enabledByDefault => 'デフォルトで有効になっています';

  @override
  String get disabledByDefault => 'デフォルトでは無効になっています';

  @override
  String get toolPkgResources => 'ツールパッケージのリソース';

  @override
  String resourcesCount(int count) {
    return 'リソース $count';
  }

  @override
  String uiModulesCount(int count) {
    return 'UIモジュール $count';
  }

  @override
  String navigationEntriesCount(int count) {
    return 'ナビゲーションエントリ $count';
  }

  @override
  String desktopWidgetsCount(int count) {
    return 'デスクトップウィジェット $count';
  }

  @override
  String workflowTemplatesCount(int count) {
    return 'ワークフロー テンプレート $count';
  }

  @override
  String workspaceTemplatesCount(int count) {
    return 'ワークスペース テンプレート $count';
  }

  @override
  String get pluginConfiguration => 'プラグインの設定';

  @override
  String get subpackages => 'サブパッケージ';

  @override
  String get toolPkgNoSubpackages => 'この ToolPkg はサブパッケージを宣言していません';

  @override
  String subpackageToolCount(String packageName, int count) {
    return '$packageName · $count ツール';
  }

  @override
  String get workflowTemplates => 'ワークフローテンプレート';

  @override
  String get workspaceTemplates => 'ワークスペーステンプレート';

  @override
  String get disable => '無効にする';

  @override
  String get enable => '有効にする';

  @override
  String get environmentVariables => '環境変数';

  @override
  String get required => '必須';

  @override
  String get states => '州';

  @override
  String stateToolSummary(String condition, int toolCount, int excludeCount) {
    return '$condition · $toolCount ツール · $excludeCount を除く';
  }

  @override
  String get inherit => '継承する';

  @override
  String get tools => 'ツール';

  @override
  String get packageNoTools => 'このパッケージはツールを宣言していません';

  @override
  String get permissionsTitle => '権限';

  @override
  String get clear => 'クリア';

  @override
  String get noPermissionRecords => 'まだ許可記録がありません';

  @override
  String get allow => '許可する';

  @override
  String get deny => '拒否する';

  @override
  String get camera => 'カメラ';

  @override
  String get microphone => 'マイク';

  @override
  String get protectedMedia => '保護されたメディア';

  @override
  String get midiDevice => 'MIDIデバイス';

  @override
  String get browserPermissionRequestTitle => 'ウェブサイトの許可リクエスト';

  @override
  String chatSpeechInputFailed(Object error) {
    return '音声入力に失敗しました: $error';
  }

  @override
  String get chatSpeechInputConfigurationRequired =>
      '音声入力を使う前に、設定の「音声と認識」で音声認識を選択してください。';

  @override
  String get chatSpeechNoTextRecognized => '音声を認識できませんでした。';

  @override
  String get history => '歴史';

  @override
  String get bookmarks => 'ブックマーク';

  @override
  String get downloads => 'ダウンロード';

  @override
  String get scripts => 'スクリプト';

  @override
  String get zoom => 'ズーム';

  @override
  String get zoomIn => 'ズームイン';

  @override
  String get zoomOut => 'ズームアウト';

  @override
  String get desktopMode => 'デスクトップモード';

  @override
  String get clearLocalStorage => 'ローカルストレージをクリアする';

  @override
  String get searchHistory => '検索履歴';

  @override
  String get noDownloadTasks => 'ダウンロードタスクはまだありません';

  @override
  String get openFile => 'ファイルを開く';

  @override
  String get openLocation => 'オープンロケーション';

  @override
  String get retry => 'もう一度試す';

  @override
  String get removeRecord => '履歴から削除';

  @override
  String get pending => '保留中';

  @override
  String get completed => '完了しました';

  @override
  String get failed => '失敗しました';

  @override
  String get back => '戻る';

  @override
  String get forward => '進む';

  @override
  String get stop => '停止';

  @override
  String get refresh => 'リフレッシュ';

  @override
  String get home => 'ホーム';

  @override
  String get newTab => '新しいタブ';

  @override
  String get openExternalApplication => '外部アプリケーションを開く';

  @override
  String get open => '開く';

  @override
  String get ok => 'OK';

  @override
  String get webPage => 'ウェブページ';

  @override
  String get tabs => 'タブ';

  @override
  String get noBookmarks => 'まだブックマークはありません';

  @override
  String get removeBookmark => 'ブックマークを解除';

  @override
  String get addBookmark => 'ブックマークに追加';

  @override
  String get menu => 'メニュー';

  @override
  String get siteData => 'サイトデータ';

  @override
  String get clearAllWebViewCookies => 'すべての WebView Cookie をクリアする';

  @override
  String get clearCookies => 'クリアクッキー';

  @override
  String get noData => 'データなし';

  @override
  String get local => 'ローカル';

  @override
  String get pageLoadFailed => 'ページの読み込みに失敗しました';

  @override
  String get pause => '一時停止';

  @override
  String get resume => '再開';

  @override
  String get paused => '一時停止中';

  @override
  String get cancelled => 'キャンセルしました';

  @override
  String get downloading => 'ダウンロード中';

  @override
  String savedTo(String path) {
    return '保存先: $path';
  }

  @override
  String get sslCertificateError => 'SSL証明書エラー';

  @override
  String get edit => '編集';

  @override
  String get files => 'ファイル';

  @override
  String get terminal => 'ターミナル';

  @override
  String get browser => 'ブラウザ';

  @override
  String get filePreview => 'ファイルのプレビュー';

  @override
  String get workspaceBoundTitle => '使用中の作業フォルダー';

  @override
  String get selectFile => 'ファイルを選択';

  @override
  String get selectFileDescription => 'ワークスペースからファイルを選択して、表示、編集、または AI に送信します';

  @override
  String get openTerminal => 'ターミナルを開く';

  @override
  String get openTerminalDescription => '現在のワークスペースのコマンド ラインを入力します';

  @override
  String get openBrowser => 'ブラウザを開く';

  @override
  String get openBrowserDescription =>
      'フルブラウザセッション、プロジェクトプレビュー、Webオートメーションを開きます。';

  @override
  String get noWorkspaceBound => 'この会話にはバインドされたワークスペースがありません。';

  @override
  String get terminalSessionPlaceholder => '現在の作業フォルダーのターミナルがここに表示されます。';

  @override
  String get emptyFolder => 'このフォルダは空です';

  @override
  String get imagePreview => '画像プレビュー';

  @override
  String get audioPreview => 'オーディオプレビュー';

  @override
  String get videoPreview => 'ビデオプレビュー';

  @override
  String get pdfPreview => 'PDF プレビュー';

  @override
  String get wordPreview => '単語のプレビュー';

  @override
  String get spreadsheetPreview => 'スプレッドシートのプレビュー';

  @override
  String get presentationPreview => 'プレゼンテーションのプレビュー';

  @override
  String get webPagePreview => 'Webページのプレビュー';

  @override
  String get markdownPreview => 'マークダウンプレビュー';

  @override
  String get textPreview => 'テキストプレビュー';

  @override
  String get file => 'ファイル';

  @override
  String get unsupportedReadOnlyPreview =>
      'このファイルは、組み込みの読み取り専用プレビュー タイプではありません。';

  @override
  String get cannotPreview => 'プレビューできない';

  @override
  String get openProjectInFullBrowser => 'フルブラウザでプロジェクトを開く';

  @override
  String get openInBrowser => 'ブラウザで開く';

  @override
  String get emptySpreadsheet => 'スプレッドシートが空です';

  @override
  String get settingsCategoryModelTitle => 'モデルとAI';

  @override
  String get settingsCategoryModelSubtitle => 'モデル、APIキー、コンテキスト';

  @override
  String get settingsCategoryModelDescription =>
      'AIへの接続、チャット用モデル、思考、コンテキスト、画像や音声の利用を設定します。';

  @override
  String get settingsCategoryLocalModelsTitle => '端末内モデル';

  @override
  String get settingsCategoryLocalModelsSubtitle => 'ダウンロード、実行エンジン、音声認識・読み上げ';

  @override
  String get settingsCategoryLocalModelsDescription =>
      '必要に応じて端末へ入れるモデルと実行エンジンを管理します。';

  @override
  String get settingsCategoryCharactersTitle => 'キャラクターとメモリー';

  @override
  String get settingsCategoryCharactersSubtitle => 'カード、グループ、割り当て';

  @override
  String get settingsCategoryCharactersDescription =>
      'キャラクター、グループ、使用中の役割、モデル・メモリー・ツールの割り当てを管理します。';

  @override
  String get settingsCategoryToolsTitle => 'ツールと権限';

  @override
  String get settingsCategoryToolsSubtitle => 'AIの操作範囲、端末の許可、拡張機能';

  @override
  String get settingsCategoryToolsDescription =>
      'AIに許可する操作範囲を選び、端末側の権限状態を確認します。';

  @override
  String get settingsCategoryWorkspaceTitle => '作業フォルダーとブラウザー';

  @override
  String get settingsCategoryWorkspaceSubtitle => 'ファイル、ターミナル、ブラウザー';

  @override
  String get settingsCategoryWorkspaceDescription =>
      '標準の作業フォルダー、ターミナル、ブラウザー、自動操作を管理します。';

  @override
  String get settingsCategoryGlobalBehaviorTitle => '基本動作';

  @override
  String get settingsCategoryGlobalBehaviorSubtitle => '入力処理と操作方法';

  @override
  String get settingsCategoryGlobalBehaviorDescription =>
      'キャラクターによらない入力と操作の動作を設定します。';

  @override
  String get settingsCategoryAppearanceTitle => '表示と言語';

  @override
  String get settingsCategoryAppearanceSubtitle => 'テーマと言語';

  @override
  String get settingsCategoryAppearanceDescription => '画面のテーマと表示言語を変更します。';

  @override
  String get settingsCategoryDataTitle => 'データとバックアップ';

  @override
  String get settingsCategoryDataSubtitle => 'バックアップ、復元、統計';

  @override
  String get settingsCategoryDataDescription =>
      'チャット、キャラクター、モデル設定をバックアップ・復元し、利用状況を確認します。';

  @override
  String get settingsCategoryAccessLinksTitle => '端末と接続';

  @override
  String get settingsCategoryAccessLinksSubtitle => '接続、同期、アクセス';

  @override
  String get settingsCategoryAccessLinksDescription =>
      '別の端末を接続し、データ同期やブラウザーからのアクセスを設定します。';

  @override
  String get settingsCategoryGroupAssistant => 'AIと作成';

  @override
  String get settingsCategoryGroupWorkspace => '作業フォルダーと自動化';

  @override
  String get settingsCategoryGroupExperience => '表示と操作';

  @override
  String get settingsCategoryGroupSystem => 'データとシステム';

  @override
  String get settingsGlobalBehaviorChatInputSection => 'チャット入力';

  @override
  String get settingsGlobalBehaviorLongPastedTextAsAttachment =>
      '長く貼り付けられたテキストをファイルに変換する';

  @override
  String get settingsGlobalBehaviorLongPastedTextThreshold => '変換しきい値';

  @override
  String settingsGlobalBehaviorLongPastedTextThresholdValue(int count) {
    return '$count 文字';
  }

  @override
  String get settingsComingSoon =>
      'この領域では、引き続き既存のランタイム機能が接続されます。モデル、キャラクター、ツールが先に完成しています。';

  @override
  String get settingsAdvanced => '詳細設定';

  @override
  String get settingsActive => '使用中';

  @override
  String get settingsActivate => '使用する';

  @override
  String get settingsModelCurrentSection => '現在のチャットモデル';

  @override
  String get settingsModelCurrentChatModel => 'チャットで使用';

  @override
  String get settingsModelCurrentActive => '使用中';

  @override
  String get settingsModelSetCurrentActive => 'このモデルを使用';

  @override
  String get settingsChatThinkingMode => '思考モード';

  @override
  String get settingsChatThinkingModeDescription => '対応モデルに、より安定した推論を行わせます。';

  @override
  String get settingsChatStreamOutput => '回答を順次表示';

  @override
  String get settingsChatStreamOutputDescription => '生成中の回答を少しずつ表示します。';

  @override
  String get agentModelSelectorThinkingSettings => '思考設定';

  @override
  String get agentModelSelectorThinkingSettingsDescription =>
      '現在のチャットモデルの考え方を調整します。';

  @override
  String get agentModelSelectorThinkingQuality => '思考の深さ';

  @override
  String get agentModelSelectorThinkingQualityDescription =>
      '思考モードでのみ有効です。数値が高いほど深く考え、1は自動調整です。';

  @override
  String agentModelSelectorMaxModeDescription(
    String enabledLength,
    String disabledLength,
  ) {
    return '最大コンテキストモードをオンにすると${enabledLength}k、オフでは${disabledLength}kのコンテキストを使います。';
  }

  @override
  String get agentModelSelectorModelConfiguration => 'モデル設定';

  @override
  String get agentModelSelectorModelConfigurationDescription =>
      '設定済みモデルを選びます。追加や変更は下の「モデル設定を管理」から行えます。';

  @override
  String get agentModelSelectorModel => 'モデル';

  @override
  String get agentModelSelectorNoModels => '利用できるモデルがありません';

  @override
  String get agentModelSelectorManageConfiguration => 'モデル設定を管理';

  @override
  String agentModelSelectorModelCount(int count) {
    return '$count個のモデル';
  }

  @override
  String get agentModelSelectorOn => 'オン';

  @override
  String get agentModelSelectorOff => 'オフ';

  @override
  String get settingsModelProfilesSection => 'モデル設定';

  @override
  String get settingsModelFunctionMappingsSection => '機能別モデル設定';

  @override
  String get settingsModelFunctionMappingsDescription =>
      'チャット、要約、記憶、画像認識などの機能で使用するモデルプロファイルと具体的なモデルを選択します。';

  @override
  String get settingsModelFunctionMappingsReset => 'すべてリセット';

  @override
  String get settingsModelFunctionMappingsChange => '変更';

  @override
  String settingsModelFunctionMappingsSelect(String name) {
    return '$nameモデルを選択してください';
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
    return 'バインドされたモデルが存在しません: $providerId · $modelId';
  }

  @override
  String settingsModelDeleteBlocked(String functions) {
    return 'このモデルは次の機能で使用されます。最初にモデルの割り当てを変更します: $functions';
  }

  @override
  String settingsModelDeleteProviderBlocked(String functions) {
    return 'このプロバイダーのモデルは、次の機能で使用されます。最初にモデルの割り当てを変更します: $functions';
  }

  @override
  String settingsModelDeleteProviderConfirm(String name, int count) {
    return 'プロバイダー「$name」を削除しますか？ この操作で、その配下のモデル $count 件も削除されます。';
  }

  @override
  String get settingsModelDeleteProviderConfirmAction => 'プロバイダーの削除';

  @override
  String get settingsTtsDeleteProvider => 'TTSプロバイダーの削除';

  @override
  String settingsTtsDeleteProviderConfirm(String name, int count) {
    return 'TTSプロバイダー「$name」と、その音声設定 $count 件を削除しますか？';
  }

  @override
  String settingsTtsDeleteProviderFailed(String error) {
    return 'TTS プロバイダーの削除に失敗しました: $error';
  }

  @override
  String get settingsTtsCurrentConfigCannotDelete =>
      '現在使用されている TTS 構成は削除できません。';

  @override
  String get settingsTtsConfigUsedByCharacter =>
      'この TTS 設定はキャラクター カードによって使用されるため、削除できません。';

  @override
  String get settingsModelChatAutoGlmWarning =>
      'AutoGLM をメインのチャット モデルとして使用することはできません。チャットと UI コントロールは個別のモデル割り当てを使用します。別の大きなモデルを選択してください。';

  @override
  String get settingsModelFunctionChat => 'チャット';

  @override
  String get settingsModelFunctionChatDescription => '主要な会話の応答に使用されるモデル。';

  @override
  String get settingsModelFunctionSummary => '概要';

  @override
  String get settingsModelFunctionSummaryDescription =>
      '長いコンテキストの自動要約に使用されるモデル。';

  @override
  String get settingsModelFunctionTitleGeneration => '会話のタイトル';

  @override
  String get settingsModelFunctionTitleGenerationDescription =>
      '最初のメッセージと添付ファイルを会話のタイトルに要約するために使用されるモデル。';

  @override
  String get settingsModelFunctionMemory => '記憶';

  @override
  String get settingsModelFunctionMemoryDescription => '記憶の抽出、整理、更新に使用されるモデル。';

  @override
  String get settingsModelFunctionUiController => 'UIコントロール';

  @override
  String get settingsModelFunctionUiControllerDescription =>
      'インターフェース制御と軽量アクションプランニングに使用されるモデル。';

  @override
  String get settingsModelFunctionTranslation => '翻訳';

  @override
  String get settingsModelFunctionTranslationDescription =>
      'テキストとローカライズされたコンテンツを翻訳するために使用されるモデル。';

  @override
  String get settingsModelFunctionGrep => 'テキスト検索';

  @override
  String get settingsModelFunctionGrepDescription =>
      '検索結果をフィルタリングし、テキストの一致を判断するために使用されるモデル。';

  @override
  String get settingsModelFunctionRoleResponsePlanner => 'グループ返信プランナー';

  @override
  String get settingsModelFunctionRoleResponsePlannerDescription =>
      'グループ会話における話し手の役割と順序を計画するために使用されるモデル。';

  @override
  String get settingsModelFunctionImageRecognition => '画像認識';

  @override
  String get settingsModelFunctionImageRecognitionDescription =>
      '画像を理解し、画像コンテンツを抽出するために使用されるモデル。';

  @override
  String get settingsModelFunctionAudioRecognition => '音声認識';

  @override
  String get settingsModelFunctionAudioRecognitionDescription =>
      '音声を理解し、音声コンテンツを抽出するために使用されるモデル。';

  @override
  String get settingsModelFunctionVideoRecognition => 'ビデオ認識';

  @override
  String get settingsModelFunctionVideoRecognitionDescription =>
      'ビデオを理解し、ビデオ コンテンツを抽出するために使用されるモデル。';

  @override
  String get settingsModelFunctionImageUnsupported =>
      '選択したモデル プロファイルでは、直接画像入力が無効になっています。';

  @override
  String get settingsModelFunctionAudioUnsupported =>
      '選択したモデル プロファイルでは、直接オーディオ入力が無効になっています。';

  @override
  String get settingsModelFunctionVideoUnsupported =>
      '選択したモデル プロファイルでは、直接ビデオ入力が無効になっています。';

  @override
  String get settingsModelCreateProfile => '新しいモデル設定';

  @override
  String get settingsModelEditProfile => 'モデル設定を編集';

  @override
  String get settingsModelProfileName => '設定名';

  @override
  String get settingsModelApiEndpoint => 'API接続先';

  @override
  String get settingsModelModelNames => 'モデル名';

  @override
  String get settingsModelApiKey => 'APIキー';

  @override
  String get settingsModelProviderTypeOpenaiCodex => 'ChatGPT Codex（サブスク利用）';

  @override
  String get settingsModelCodexTitle => 'ChatGPT / Codex ログイン';

  @override
  String get settingsModelCodexDescription =>
      'ChatGPTプランに含まれるCodex利用枠を使います。APIキーや中継サーバーは不要です。';

  @override
  String get settingsModelCodexSignIn => 'ChatGPTでログイン';

  @override
  String get settingsModelCodexConnected => 'ログイン済み';

  @override
  String get settingsModelCodexSignedOut => '未ログイン';

  @override
  String get settingsModelCodexWaiting => '承認待ち';

  @override
  String get settingsModelCodexExpired => 'コードの期限切れ';

  @override
  String get settingsModelCodexChecking => '確認中';

  @override
  String get settingsModelCodexCodeHelp =>
      'ChatGPTの画面でこの1回限りのコードを入力してください。有効期限は15分です。';

  @override
  String get settingsModelCodexOpenBrowser => 'ChatGPTの画面を開く';

  @override
  String get settingsModelCodexOpenFailed =>
      'ChatGPTの画面を開けませんでした。もう一度ボタンを押してください。';

  @override
  String get settingsModelCodexLogout => 'ログアウト';

  @override
  String get settingsModelCodexRequired => 'この接続先を保存する前にChatGPTへログインしてください。';

  @override
  String get settingsModelCodexPlan => 'プラン';

  @override
  String get settingsModelApiKeyPool => 'APIキー一覧';

  @override
  String get settingsModelApiKeyPoolDescription =>
      '複数のAPIキーを登録し、自動で切り替えて利用できます。';

  @override
  String settingsModelApiKeyPoolCount(int count) {
    return '$count キー';
  }

  @override
  String get settingsModelApiKeyPoolEmpty =>
      'まだ鍵がありません。キーを追加すると、このプロファイルはキー プールを使用します。';

  @override
  String get settingsModelAddApiKey => 'APIキーを追加';

  @override
  String get settingsModelEditApiKey => 'APIキーを編集';

  @override
  String get settingsModelApiKeyName => 'キーの名前';

  @override
  String get settingsModelApiKeyEnabled => 'このキーを使用する';

  @override
  String get settingsModelProviderId => 'プロバイダーID';

  @override
  String get settingsModelProvidersSection => '接続先';

  @override
  String get settingsModelProviderType => '接続先の種類';

  @override
  String settingsModelProviderTypeOption(String name, String original) {
    return '$name（$original）';
  }

  @override
  String get settingsModelProviderTypeOpenai => 'OpenAI';

  @override
  String get settingsModelProviderTypeOpenaiResponses => 'OpenAI の応答';

  @override
  String get settingsModelProviderTypeOpenaiResponsesGeneric =>
      'OpenAI レスポンスとの互換性';

  @override
  String get settingsModelProviderTypeOpenaiGeneric => 'OpenAI対応';

  @override
  String get settingsModelProviderTypeAnthropic => '人間的';

  @override
  String get settingsModelProviderTypeAnthropicGeneric => '人間互換性';

  @override
  String get settingsModelProviderTypeGoogle => 'Google ジェミニ';

  @override
  String get settingsModelProviderTypeGeminiGeneric => 'ジェミニ対応';

  @override
  String get settingsModelProviderTypeBaidu => '百度';

  @override
  String get settingsModelProviderTypeAliyun => 'アリユン';

  @override
  String get settingsModelProviderTypeXunfei => 'シュンフェイ';

  @override
  String get settingsModelProviderTypeZhipu => '志浦AI';

  @override
  String get settingsModelProviderTypeBaichuan => '白川';

  @override
  String get settingsModelProviderTypeMoonshot => 'ムーンショット';

  @override
  String get settingsModelProviderTypeMimo => 'ミモ';

  @override
  String get settingsModelProviderTypeDeepseek => 'ディープシーク';

  @override
  String get settingsModelProviderTypeMistral => 'ミストラル';

  @override
  String get settingsModelProviderTypeSiliconflow => 'シリコンフロー';

  @override
  String get settingsModelProviderTypeIflow => 'iFlow';

  @override
  String get settingsModelProviderTypeOpenrouter => 'オープンルーター';

  @override
  String get settingsModelProviderTypeFourRouter => '4ルーター';

  @override
  String get settingsModelProviderTypeNousPortal => 'ヌースポータル';

  @override
  String get settingsModelProviderTypeInfiniai => 'インフィニアイ';

  @override
  String get settingsModelProviderTypeAlipayBailing => 'アリペイのバイリング';

  @override
  String get settingsModelProviderTypeDoubao => '豆宝';

  @override
  String get settingsModelProviderTypeNvidia => 'エヌビディア';

  @override
  String get settingsModelProviderTypeLmstudio => 'LMスタジオ';

  @override
  String get settingsModelProviderTypeOllama => 'オラマ';

  @override
  String get settingsModelProviderTypeOpenaiLocal => 'OpenAIローカル';

  @override
  String get settingsModelProviderTypeLocalModel => 'ローカルモデル';

  @override
  String localModelsLoadFailed(Object error) {
    return 'ローカル モデル ステータスのロードに失敗しました: $error';
  }

  @override
  String localModelsOperationFailed(Object error) {
    return 'ローカル モデルの操作が失敗しました: $error';
  }

  @override
  String get localModelsCatalog => 'モデルカタログ';

  @override
  String get localModelsCategorySpeechToText => '音声テキスト変換モデル';

  @override
  String get localModelsCategoryTextToSpeech => 'テキスト読み上げモデル';

  @override
  String get localModelsCategoryChat => 'LLMモデル';

  @override
  String get localModelsCategoryEmbedding => '埋め込みモデル';

  @override
  String get localModelsInstalledEngines => '搭載エンジン';

  @override
  String get localModelsNoInstalledEngines =>
      'このプラットフォームにはローカル推論エンジンはインストールされていません。';

  @override
  String get localModelsDeleteModelTitle => 'ローカルモデルの削除';

  @override
  String localModelsDeleteModelMessage(Object modelName) {
    return '$modelName のモデル ファイルを削除しますか?';
  }

  @override
  String get localModelsDeleteEngineTitle => 'ローカルエンジンを削除する';

  @override
  String localModelsDeleteEngineMessage(Object engineName, Object version) {
    return '$engineName $version を削除しますか？';
  }

  @override
  String get localModelsCancelling => '一時停止';

  @override
  String localModelsDownloadPaused(Object downloaded, Object total) {
    return '一時停止中: $downloaded / $total';
  }

  @override
  String get localModelsDownloadInstalling => 'ダウンロード完了、インストール中';

  @override
  String localModelsDownloading(Object downloaded, Object total) {
    return 'ダウンロード中: $downloaded / $total';
  }

  @override
  String localModelsLicense(Object license) {
    return 'ライセンス: $license';
  }

  @override
  String get localModelsPlatformCompatible => 'プラットフォーム互換性';

  @override
  String get localModelsPlatformIncompatible => 'プラットフォームに互換性がない';

  @override
  String get localModelsModelInstalled => '搭載モデル';

  @override
  String get localModelsModelNotInstalled => 'モデルがインストールされていません';

  @override
  String get localModelsEngineInstalled => 'エンジン搭載';

  @override
  String get localModelsEngineNotInstalled => 'エンジンが搭載されていない';

  @override
  String get localModelsVerifyModelAndEngine => 'モデルとエンジンを確認する';

  @override
  String get localModelsDeleteModel => 'モデルの削除';

  @override
  String get localModelsPauseDownload => 'ダウンロードを一時停止する';

  @override
  String get localModelsDeleteDownload => 'ダウンロードの削除';

  @override
  String get localModelsResumeDownload => '再開';

  @override
  String get localModelsInstalling => 'インストール中';

  @override
  String get localModelsInstall => 'インストール';

  @override
  String get localModelsDeleteEngine => 'エンジンの削除';

  @override
  String get localModelDescriptionSherpaOnnxStreamingStt =>
      '中国語と英語のバイリンガル音声認識のストリーミング。';

  @override
  String get localModelDescriptionSherpaOnnxVitsAishell3 =>
      'ローカル中国語のマルチスピーカー音声合成。';

  @override
  String get localModelDescriptionSherpaOnnxVitsZhLl =>
      '地元の中国語の 5 人の話者による音声合成。';

  @override
  String get localModelDescriptionSherpaOnnxMatchaBaker =>
      '地元の中国語のシングルスピーカー Matcha 音声合成。';

  @override
  String get localModelDescriptionSherpaOnnxKittenNano =>
      '地元の英語 8 スピーカー KittenTTS 音声合成。';

  @override
  String get localModelDescriptionSherpaOnnxWebParaformer =>
      'ブラウザにパッケージ化された中国語と英語の Paraformer 音声認識。';

  @override
  String get localModelDescriptionSherpaOnnxWebVitsPiper =>
      'ブラウザパッケージ化された英語マルチスピーカーVITS音声合成。';

  @override
  String get settingsModelProviderTypeMnn => 'MNN';

  @override
  String get settingsModelProviderTypeLlamaCpp => 'ラマ.cpp';

  @override
  String get settingsModelProviderTypePpinfra => 'PPインフラ';

  @override
  String get settingsModelProviderTypeNovita => 'ノビタAI';

  @override
  String get settingsModelProviderTypeOther => 'その他';

  @override
  String get settingsModelEditModelSettings => 'モデル設定';

  @override
  String get settingsModelCreateProvider => '接続先を追加';

  @override
  String get settingsModelEditProvider => '接続先を編集';

  @override
  String get settingsModelAddModel => 'モデルを追加';

  @override
  String get settingsModelAddModelShort => '追加';

  @override
  String get settingsModelCustomModel => 'カスタムモデル';

  @override
  String get settingsModelModelId => 'モデルID';

  @override
  String get settingsModelDuplicateModelId => 'このモデルはすでにこのプロバイダーに追加されています。';

  @override
  String get settingsModelMaxTokens => '最大出力トークン数';

  @override
  String get settingsModelMaxTokensDescription => '1回の回答で生成できるトークン数を制限します。';

  @override
  String get settingsModelTemperature => 'Temperature（ランダム性）';

  @override
  String get settingsModelTemperatureDescription => '低いほど安定し、高いほど多様な回答になります。';

  @override
  String get settingsModelTopP => 'トップ';

  @override
  String get settingsModelTopPDescription => '累積上位確率範囲からのみサンプリングします。';

  @override
  String get settingsModelTopK => 'トップk';

  @override
  String get settingsModelTopKDescription =>
      'K 個の最も可能性の高い候補トークンからのサンプル。 0 は無効にします。';

  @override
  String get settingsModelPresencePenalty => 'プレゼンスペナルティ';

  @override
  String get settingsModelPresencePenaltyDescription =>
      '新しいトピックを奨励し、既存のコンテンツの再利用を減らします。';

  @override
  String get settingsModelFrequencyPenalty => '周波数ペナルティ';

  @override
  String get settingsModelFrequencyPenaltyDescription =>
      'トークンが繰り返されると、頻度に応じてペナルティが課されます。';

  @override
  String get settingsModelRepetitionPenalty => '反復ペナルティ';

  @override
  String get settingsModelRepetitionPenaltyDescription =>
      '繰り返し出力をさらに削減します。 1.0 はペナルティがないことを意味します。';

  @override
  String get settingsModelRequestLimit => '1分あたりのリクエスト数';

  @override
  String get settingsModelMaxConcurrent => '最大同時リクエスト';

  @override
  String get settingsModelContextLength => 'コンテキスト長';

  @override
  String get settingsModelMaxContextLength => '最大コンテキスト長';

  @override
  String get settingsModelMaxContextLengthInvalid =>
      '0 より大きい最大コンテキスト長を入力してください';

  @override
  String get settingsModelMaxContextMode => 'マックスコンテキストモード';

  @override
  String get settingsModelSummaryThreshold => 'サマリートークンのしきい値';

  @override
  String get settingsModelSummaryByMessageCount => 'メッセージ数ごとに要約する';

  @override
  String get settingsModelSummaryMessageCount => '概要メッセージのしきい値';

  @override
  String get settingsModelCustomHeaders => 'カスタムヘッダー';

  @override
  String get settingsModelCustomParameters => 'カスタムパラメータJSON';

  @override
  String get settingsModelToolCall => 'ツール呼び出し';

  @override
  String get settingsModelToolCallDescription => 'モデルが構造化されたツール呼び出しを使えるようにします。';

  @override
  String get settingsModelDirectImage => '直接画像入力';

  @override
  String get settingsModelDirectImageDescription => '画像入力対応機種に直接画像を送信します。';

  @override
  String get settingsModelDirectAudio => '直接オーディオ入力';

  @override
  String get settingsModelDirectAudioDescription =>
      '音声入力をサポートするモデルに音声を直接送信します。';

  @override
  String get settingsModelDirectVideo => '直接ビデオ入力';

  @override
  String get settingsModelDirectVideoDescription =>
      'ビデオ入力をサポートするモデルにビデオを直接送信します。';

  @override
  String get settingsModelGoogleSearch => 'Google検索';

  @override
  String get settingsModelGoogleSearchDescription => 'プロバイダー側の検索機能を有効にします。';

  @override
  String get settingsModelContext => 'コンテキストウィンドウ';

  @override
  String get settingsModelSummary => '自動サマリー';

  @override
  String get settingsModelMediaHistory => 'メディアの歴史';

  @override
  String get settingsModelCapabilities => '能力';

  @override
  String get settingsModelBuiltinTools => '内蔵ツール';

  @override
  String get settingsModelBuiltinToolExclusive => '有効にすると外部ツールの呼び出しをオフにします';

  @override
  String get settingsModelConnectionTestSection => '接続テスト';

  @override
  String get settingsModelRunConnectionTest => '現在のモデルをテストする';

  @override
  String get settingsModelTestModel => 'テストモデル';

  @override
  String get settingsModelTestingConnection => '現在のモデルの接続をテストしています…';

  @override
  String get settingsModelTestedModel => 'テスト済みモデル';

  @override
  String get settingsModelConnectionTestPassed => 'すべてのチェックに合格しました';

  @override
  String get settingsModelConnectionTestFailed => 'いくつかのチェックが失敗しました';

  @override
  String get settingsModelCapabilitiesApplied => 'モデルの機能スイッチがテスト結果から更新されました。';

  @override
  String get settingsModelCapabilitiesNeedChat =>
      'チャット テストに合格しなかったため、モデル機能スイッチは更新されませんでした。';

  @override
  String settingsModelConnectionTestError(String error) {
    return '接続テストが失敗しました: $error';
  }

  @override
  String get settingsModelTestItemChat => 'チャット';

  @override
  String get settingsModelTestItemToolCall => 'ツール呼び出し';

  @override
  String get settingsModelTestItemImage => '画像';

  @override
  String get settingsModelTestItemAudio => 'オーディオ';

  @override
  String get settingsModelTestItemVideo => 'ビデオ';

  @override
  String get settingsModelTestItemUnknown => '不明なアイテム';

  @override
  String get settingsCharactersCreateCard => '新しいキャラクターカード';

  @override
  String get settingsCharactersEditCard => 'キャラクターカードを編集する';

  @override
  String get settingsCharactersCardName => 'キャラクター名';

  @override
  String get settingsCharactersCreateGroup => '新しいグループ';

  @override
  String get settingsCharactersEditGroup => 'グループの編集';

  @override
  String get settingsCharactersGroupName => 'グループ名';

  @override
  String get settingsCharactersDescription => '説明';

  @override
  String get settingsCharactersCharacterSetting => 'キャラクター設定';

  @override
  String get settingsCharactersOpeningStatement => '冒頭陳述';

  @override
  String get settingsCharactersOtherContentChat => '追加のチャットコンテンツ';

  @override
  String get settingsCharactersOtherContentVoice => '追加音声コンテンツ';

  @override
  String get settingsCharactersAdvancedPrompt => '高度なカスタム プロンプト';

  @override
  String get settingsCharactersMarks => '注意事項';

  @override
  String get settingsCharactersTags => 'タグ';

  @override
  String get settingsCharactersNoTags =>
      '使用可能なタグがありません。タグ管理で作成し、このキャラクター カードにバインドします。';

  @override
  String get settingsCharactersImport => 'インポート';

  @override
  String get settingsCharactersExport => 'エクスポート';

  @override
  String get settingsCharactersImportJson => 'JSONをインポートする';

  @override
  String get settingsCharactersCopyJson => 'JSONをコピー';

  @override
  String get settingsCharactersImportTavernJson => '酒場のJSONをインポート';

  @override
  String get settingsCharactersCopyTavernJson => '酒場のJSONをコピー';

  @override
  String get settingsCharactersJsonInput => 'JSONコンテンツ';

  @override
  String get settingsCharactersTavernJsonInput => '酒場のJSONコンテンツ';

  @override
  String settingsCharactersJsonCopied(String name) {
    return '「$name」の JSON をコピーしました。';
  }

  @override
  String settingsCharactersTavernJsonCopied(String name) {
    return '「$name」の居酒屋 JSON をコピーしました。';
  }

  @override
  String get settingsCharactersImportCardJson => 'キャラクターカードJSONをインポート';

  @override
  String get settingsCharactersImportCardJsonDone => 'キャラクターカードがインポートされました。';

  @override
  String get settingsCharactersImportTavernJsonDone =>
      '酒場のキャラクターカードがインポートされました。';

  @override
  String get settingsCharactersImportGroupJson => 'グループ JSON をインポートする';

  @override
  String get settingsCharactersImportGroupJsonDone => 'グループがインポートされました。';

  @override
  String settingsCharactersImportJsonError(String error) {
    return 'JSON インポートに失敗しました: $error';
  }

  @override
  String settingsCharactersImportTavernJsonError(String error) {
    return '酒場の JSON インポートに失敗しました: $error';
  }

  @override
  String settingsCharactersTavernJsonCopyError(String error) {
    return '酒場の JSON コピーに失敗しました: $error';
  }

  @override
  String get settingsCharactersTagsSection => 'タグ';

  @override
  String get settingsCharactersManageTags => 'タグを管理する';

  @override
  String get settingsCharactersCreateTag => '新しいタグ';

  @override
  String get settingsCharactersEditTag => 'タグを編集する';

  @override
  String get settingsCharactersDeleteTag => 'タグの削除';

  @override
  String settingsCharactersDeleteTagMessage(String name) {
    return '「$name」を削除しますか?';
  }

  @override
  String get settingsCharactersTagName => 'タグ名';

  @override
  String get settingsCharactersTagDescription => 'タグの説明';

  @override
  String get settingsCharactersTagPromptContent => 'プロンプトコンテンツ';

  @override
  String get settingsCharactersChatModelBindingMode => 'チャットモデルバインディングモード';

  @override
  String get settingsCharactersChatModelConfigId => 'チャットモデル構成ID';

  @override
  String get settingsCharactersChatModelIndex => 'チャットモデルインデックス';

  @override
  String get settingsCharactersToolAccess => 'ツール許可モード';

  @override
  String get settingsCharactersChatModelFollowGlobal => 'グローバルモデルに従う';

  @override
  String get settingsCharactersChatModelFixedConfig => '固定モデル構成を使用する';

  @override
  String get settingsCharactersChatModelConfig => 'モデル構成';

  @override
  String get settingsCharactersToolAccessFollowGlobal => 'グローバルツール権限に従ってください';

  @override
  String get settingsCharactersToolAccessCustom => 'カスタムキャラクターツールの権限';

  @override
  String get settingsCharactersToolAccessEmpty => 'ツールが選択されていない場合に有効になります';

  @override
  String settingsCharactersToolAccessSummaryCounts(
    int builtinCount,
    int packageCount,
    int skillCount,
    int mcpCount,
  ) {
    return '内蔵 $builtinCount · パッケージ $packageCount · スキル $skillCount · MCP $mcpCount';
  }

  @override
  String get settingsCharactersToolAccessConfigure => 'ツールの許可リストを構成する';

  @override
  String get settingsCharactersToolAccessTitle => 'カスタム許可ツール';

  @override
  String get settingsCharactersToolAccessTabBuiltin => '内蔵';

  @override
  String get settingsCharactersToolAccessTabPackage => 'パッケージ';

  @override
  String get settingsCharactersToolAccessTabSkill => 'スキル';

  @override
  String get settingsCharactersToolAccessTabMcp => 'MCP';

  @override
  String get settingsCharactersToolAccessSearchPlaceholder =>
      '名前、説明、または ID を検索します';

  @override
  String get settingsCharactersToolAccessEmptySearch => '一致するツールが見つかりませんでした';

  @override
  String get settingsCharactersToolAccessRequiresUsePackage =>
      'パッケージ、スキル、または MCP を選択するには、組み込みの use_package ツールを許可する必要もあります。';

  @override
  String get settingsCharactersToolAccessEmptyBuiltin =>
      '構成に使用できる組み込みツールはありません';

  @override
  String get settingsCharactersToolAccessEmptyPackages =>
      '現在、世界中で利用可能なパッケージはありません';

  @override
  String get settingsCharactersToolAccessEmptySkills =>
      '現在利用できる AI 可視スキルはありません';

  @override
  String get settingsCharactersToolAccessEmptyMcp =>
      '現在使用できる有効な MCP サーバーはありません';

  @override
  String get settingsCharactersBuiltinTools => '許可される組み込みツール';

  @override
  String get settingsCharactersAllowedPackages => '許可されたパッケージ';

  @override
  String get settingsCharactersAllowedSkills => '許可されるスキル';

  @override
  String get settingsCharactersAllowedMcpServers => '許可された MCP サーバー';

  @override
  String get settingsCharactersGroupMembersTitle => 'グループキャラクター';

  @override
  String get settingsCharactersOpenMemoryGraph => 'メモリグラフを表示する';

  @override
  String settingsCharactersMemoryGraphTitle(String profileName) {
    return '$profileName のメモリグラフ';
  }

  @override
  String get settingsCharactersMemoryGraphEmpty => 'メモリノードはまだありません';

  @override
  String settingsCharactersMemoryGraphStats(int nodes, int edges) {
    return '$nodes ノード · $edges リンク';
  }

  @override
  String get settingsCharactersMemoryGraphLink => 'メモリーリンク';

  @override
  String get settingsCharactersEditUserMarkdown => 'ユーザープロファイルを編集する';

  @override
  String settingsCharactersUserMarkdownTitle(String profileName) {
    return '$profileNameのユーザープロフィール';
  }

  @override
  String get settingsCharactersUserMarkdownSaved => 'ユーザープロファイルが保存されました';

  @override
  String get settingsCharactersUserMarkdownContent => 'ユーザープロフィールの内容';

  @override
  String get settingsCharactersMemoryAutoUpdate => 'メモリストアの自動更新';

  @override
  String get settingsCharactersMemoryAutoUpdateDescription =>
      'AI が会話情報をメモリ ストアに整理できるようにします。';

  @override
  String get settingsCharactersPreferenceDescription => 'ユーザープロファイルをモデルに提供する';

  @override
  String get settingsCharactersPreferenceDescriptionSubtitle =>
      '現在のユーザー プロファイルをチャット プロンプトに含めます。';

  @override
  String get settingsCharactersCardsSection => 'キャラクターカード';

  @override
  String get settingsCharactersGroupsSection => 'グループ';

  @override
  String settingsCharactersGroupMembers(int count) {
    return '$count メンバー';
  }

  @override
  String get settingsToolsPermissionMode => 'AI機能モード';

  @override
  String get settingsToolsAsk => '尋ねる';

  @override
  String get settingsToolsExtensions => '拡張機能の管理';

  @override
  String get settingsToolsPlugins => 'プラグイン';

  @override
  String get settingsToolsPluginsDescription =>
      'ToolPkg プラグイン コンテナーと UI 拡張機能を管理します。';

  @override
  String get settingsToolsPackages => 'ツールパッケージ';

  @override
  String get settingsToolsPackagesDescription =>
      '組み込みまたは外部ツール パッケージを有効化、無効化、検査します。';

  @override
  String get settingsToolsSkills => 'スキル';

  @override
  String get settingsToolsSkillsDescription => 'スキル パッケージの表示とインポートを管理します。';

  @override
  String get settingsToolsMcp => 'MCPサーバー';

  @override
  String settingsToolsMcpDescription(int seconds) {
    return 'MCP 構成を管理します。起動待ち時間は$seconds秒です。';
  }

  @override
  String get settingsToolsOverrides => '工具記録';

  @override
  String get settingsToolsToolGroups => '登録ツール';

  @override
  String get settingsToolsToolGroupsDescription =>
      'AI で使用するために現在のランタイムによって登録されているツール。';

  @override
  String get settingsToolsAlwaysAllow => 'このセッションでは許可されています';

  @override
  String get settingsToolsAlwaysAllowDescription => 'これらのツールは現在のセッションで承認されました。';

  @override
  String get settingsToolsAlwaysForbid => '必ず禁止する';

  @override
  String get settingsToolsAlwaysForbidDescription => 'AI はこれらのツールを呼び出しません。';

  @override
  String get settingsToolsAddTool => 'ツールの追加';

  @override
  String get settingsToolsAddAllowTool => '許可されたツールを追加';

  @override
  String get settingsToolsAddForbidTool => '禁止ツールを追加する';

  @override
  String get settingsToolsSearchTools => '検索ツール';

  @override
  String get settingsToolsNoToolsInGroup => 'このグループにはツールがありません。';

  @override
  String get settingsToolsMcpStartupTimeout => 'MCP 起動タイムアウト';

  @override
  String get settingsToolsMcpStartupTimeoutSeconds => '数秒待ってください';

  @override
  String get settingsToolsToolPkgPreHookTimeout => 'ToolPkg のフック前タイムアウト';

  @override
  String settingsToolsToolPkgPreHookDescription(int seconds) {
    return '1 つの ToolPkg プリフック チェーンの合計は $seconds 秒です。';
  }

  @override
  String get settingsToolsToolPkgPreHookTimeoutSeconds => '合計秒数';

  @override
  String get settingsWorkspaceCurrentDesign => '現在のワークスペース構造';

  @override
  String get settingsWorkspaceCurrentDesignDescription =>
      'ワークスペースはチャットに関連付けられています。ターミナル セッションとブラウザ セッションは、ワークスペース内にフラットに表示されるグローバル セッションです。';

  @override
  String get settingsWorkspaceOpenChat => 'チャットワークスペースに戻る';

  @override
  String get settingsWorkspaceOpenChatDescription =>
      'チャットの右側でファイル、ターミナル、ブラウザ、Web オートメーションを開きます。';

  @override
  String get settingsWorkspaceContains => 'ワークスペースに含まれるもの';

  @override
  String get settingsWorkspacePerChat => 'チャットごとに制限される';

  @override
  String get settingsWorkspaceGlobalSessions => 'グローバルターミナルセッション';

  @override
  String get settingsWorkspaceBrowserSessions => 'ブラウザーおよび WebVisit セッション';

  @override
  String get settingsWorkspaceBoundOverview => 'ワークスペースバインディングの概要';

  @override
  String get settingsWorkspaceBoundOverviewDescription =>
      'チャット履歴によって記録されたワークスペース パスがバインド ソースとして使用されます。';

  @override
  String get settingsWorkspaceBoundChats => 'バウンドチャット';

  @override
  String get settingsWorkspaceInternalRoot => '内部ワークスペースルート';

  @override
  String get settingsWorkspaceExternalRoot => '従来の外部ワークスペースのルート';

  @override
  String get settingsWorkspaceUnboundTitle => 'バインドされていないワークスペース';

  @override
  String get settingsWorkspaceUnboundSubtitle =>
      'これらのワークスペース フォルダーはチャットでは使用されません。';

  @override
  String get settingsWorkspaceNoUnbound => 'バインドされていないワークスペースはありません。';

  @override
  String settingsWorkspaceSelectedCount(int selected, int total) {
    return '選択済み $selected / $total';
  }

  @override
  String get settingsWorkspaceSelectAllCurrentList => 'すべて選択';

  @override
  String get settingsWorkspaceClearAll => 'クリア';

  @override
  String get settingsWorkspaceInternalStorage => '内部ストレージ';

  @override
  String get settingsWorkspaceExternalStorage => '外部ストレージ';

  @override
  String get settingsWorkspaceNotUsedByAnyChat => 'どのチャットでも使用されていません';

  @override
  String settingsWorkspaceDeleteSelected(int count) {
    return '選択したワークスペースを削除します ($count)';
  }

  @override
  String get settingsWorkspaceConfirmDeleteTitle => '削除の確認';

  @override
  String settingsWorkspaceDeleteConfirmation(int count) {
    return '選択した $count ワークスペース フォルダーを削除しますか?';
  }

  @override
  String settingsWorkspaceDeleted(int count) {
    return '$count 非バインド ワークスペースを削除しました。';
  }

  @override
  String settingsWorkspaceDeleteFailed(String error) {
    return '削除に失敗しました: $error';
  }

  @override
  String settingsWorkspaceLoadFailed(String error) {
    return 'ワークスペースのロードに失敗しました: $error';
  }

  @override
  String get settingsWorkspaceRefresh => 'リフレッシュ';

  @override
  String get runtimeIdentity => '現在のアイデンティティ';

  @override
  String get runtimeIdentityManage => 'ID の切り替えまたは管理';

  @override
  String get runtimeIdentitySheetTitle => 'アイデンティティ';

  @override
  String get runtimeIdentityCreate => '新しいアイデンティティ';

  @override
  String get runtimeIdentityCreateTitle => '新しいアイデンティティ';

  @override
  String get runtimeIdentityRename => 'ID の名前を変更する';

  @override
  String get runtimeIdentityRenameTitle => 'ID の名前を変更する';

  @override
  String get runtimeIdentityName => '名前 (オプション)';

  @override
  String runtimeIdentitySwitchTitle(String identityName) {
    return '$identityName に切り替えますか?';
  }

  @override
  String get runtimeIdentitySwitchDescription =>
      '各 ID には、個別のチャット、設定、デバイス スペース、ペアリングされたデバイス、およびワークスペースがあります。切り替えると現在のランタイムが終了します。選択した ID は、次回アプリを起動するときに使用されます。';

  @override
  String get runtimeIdentitySwitchConfirm => 'ID を切り替える';

  @override
  String get runtimeIdentityCurrent => '現在';

  @override
  String get settingsUserProfileTitle => 'ユーザープロフィール';

  @override
  String get settingsUserProfileSubtitle => 'アバター、名前、アイデンティティ、および GitHub';

  @override
  String get settingsUserProfileDescription => 'このプロファイルを管理し、分離された ID を切り替えます。';

  @override
  String get settingsUserProfileUnnamed => '無名';

  @override
  String get settingsUserProfileNotLoggedIn => 'ログインしていません';

  @override
  String get settingsUserProfileGitHubLoading => 'GitHub アカウントを読み込んでいます...';

  @override
  String settingsUserProfileGitHubAccount(String account) {
    return 'GitHub: @$account';
  }

  @override
  String settingsUserProfileGitHubStatusError(String error) {
    return 'GitHub ステータス エラー: $error';
  }

  @override
  String get settingsUserProfileOverview => 'プロフィール';

  @override
  String get settingsUserProfileAvatar => 'アバター';

  @override
  String get settingsUserProfileName => '名前';

  @override
  String get settingsUserProfileChooseAvatar => 'アバターを選択';

  @override
  String get settingsUserProfileClearAvatar => 'クリアアバター';

  @override
  String get settingsUserProfileEditName => '名前の編集';

  @override
  String get settingsUserProfileIdentities => 'アイデンティティ';

  @override
  String get settingsUserProfileGitHub => 'GitHub アカウント';

  @override
  String get settingsUserProfileGitHubDescription => 'ログインしていません';

  @override
  String get settingsUserProfileLogin => 'ログイン';

  @override
  String get settingsUserProfileLogout => 'ログアウト';

  @override
  String get settingsAppearanceAvatarCustom => 'カスタムアバター';

  @override
  String get settingsRuntimeConnection => '現在のデバイススペース';

  @override
  String get settingsRuntimeConnectionDescription => 'このデバイスとそのデバイス空間の接続ステータス。';

  @override
  String get settingsRuntimeCurrentSpace => '現在のデバイススペース';

  @override
  String get settingsRuntimeRenameSpace => 'デバイススペースの名前を変更する';

  @override
  String get settingsRuntimeLeaveSpace => 'デバイスのスペースを残す';

  @override
  String get settingsRuntimeLeaveSpaceTitle => '現在のデバイススペースをそのままにしますか?';

  @override
  String get settingsRuntimeLeaveSpaceDescription =>
      'このデバイスは、新しい単一デバイスのスペースを作成します。ビジネスデータとペアリングされたデバイスは保持されます。';

  @override
  String get settingsRuntimeLeaveSpaceConfirm => '休暇を取る';

  @override
  String get settingsRuntimeSpaceName => 'デバイス空間名';

  @override
  String settingsRuntimeSpaceId(String spaceId) {
    return 'デバイス空間ID: $spaceId';
  }

  @override
  String settingsRuntimeSpaceDeviceCount(int count) {
    return '$count デバイス';
  }

  @override
  String get settingsRuntimeViewSpaceTopology => 'デバイストポロジの表示';

  @override
  String settingsRuntimeSpaceTopologyTitle(String spaceName) {
    return '$spaceName デバイス トポロジ';
  }

  @override
  String settingsRuntimeSpaceTopologySummary(
    int deviceCount,
    int connectionCount,
  ) {
    return '$deviceCount デバイス · $connectionCount 直接接続';
  }

  @override
  String get settingsRuntimeDisconnectConnection => '切断する';

  @override
  String get settingsRuntimeDisconnectConnectionTitle => 'ダイレクト接続を切断する';

  @override
  String settingsRuntimeDisconnectConnectionMessage(String deviceName) {
    return '$deviceName への直接接続を切断しますか?ペアリング記録は残ります。';
  }

  @override
  String get settingsRuntimeDisconnectConnectionFailed => '切断に失敗しました';

  @override
  String get settingsRuntimeCurrentDevice => '現在のデバイス';

  @override
  String get settingsRuntimeRemoteTitle => '接続されたデバイス';

  @override
  String get settingsRuntimeRemoteDescription =>
      '直接ペアリングされたデバイス。ペアリングにより接続が確立されます。デバイス空間に参加すると、データとルーティングの共有が可能になります。';

  @override
  String get settingsRuntimePairRemote => '別のデバイスを接続する';

  @override
  String get settingsRuntimeNoPairedRemote => 'まだ接続されているデバイスがありません。';

  @override
  String get settingsRuntimeConnectionInitiatedByOtherDevice =>
      '他のデバイスによって開始された接続';

  @override
  String get settingsRuntimePairToken => '接続トークン';

  @override
  String get settingsRuntimePairCode => 'ペアリングコード';

  @override
  String get settingsRuntimeStartPairing => '接続を開始する';

  @override
  String get settingsRuntimeFinishPairing => '接続を終了する';

  @override
  String get settingsRuntimeBaseUrl => 'デバイスアドレス';

  @override
  String settingsRuntimeConnectionFailed(String error) {
    return 'デバイスの接続に失敗しました: $error';
  }

  @override
  String get settingsRuntimePairingRejected => 'デバイスの接続が拒否されました';

  @override
  String get settingsRuntimePairedChecking => 'チェック中';

  @override
  String get settingsRuntimePairedOnline => 'オンライン';

  @override
  String get settingsRuntimePairedOffline => 'オフライン';

  @override
  String get settingsRuntimePairedInvalid => 'ペアリングが無効です';

  @override
  String get settingsRuntimePairedError => '状態を取得できません';

  @override
  String get settingsRuntimePairingRevokedTitle => '相手のデバイスがペアリングを解除しました';

  @override
  String get settingsRuntimePairingRevokedMessage =>
      '相手のデバイスがこのデバイスとのペアリングを解除しました。このデバイスに保存されているペアリング情報は無効です。';

  @override
  String get settingsRuntimePairingRevokedConfirm => '確認しました';

  @override
  String get settingsRuntimePairedRemovedFromSpace => 'デバイススペースから削除されました';

  @override
  String get settingsRuntimeRemovedFromSpaceTitle => 'デバイススペースから削除されました';

  @override
  String get settingsRuntimeRemovedFromSpaceMessage =>
      '相手のデバイスによって、このデバイスがデバイススペースから削除されました。現在のデバイススペースを退出し、このデバイス専用のスペースを作成します。';

  @override
  String get settingsRuntimeRemovedFromSpaceConfirm => '退出して専用スペースを作成';

  @override
  String get settingsRuntimeJoinSpace => 'デバイススペースに参加する';

  @override
  String settingsRuntimeJoinSpaceTitle(String deviceName) {
    return '$deviceName のデバイススペースに参加しますか?';
  }

  @override
  String get settingsRuntimeJoinSpaceDescription =>
      '現在のデバイス空間は他のデバイス空間とマージされ、その名前が使用されます。';

  @override
  String get settingsRuntimePairingComplete => 'デバイスがペアリングされました';

  @override
  String get settingsRuntimeDeviceInCurrentSpace => '現在のデバイス空間内';

  @override
  String get settingsRuntimeDiscoverSpaces => 'デバイススペースを発見する';

  @override
  String get settingsRuntimeDiscoverSpacesDescription =>
      '近くのデバイスはデバイススペースごとにグループ化されます。デバイス スペースを拡張して、そのデバイスの 1 つに直接接続します。';

  @override
  String settingsRuntimeDiscoveredSpaceSummary(
    int memberCount,
    int nearbyCount,
  ) {
    return '合計 $memberCount デバイス · 近くのデバイス $nearbyCount 台';
  }

  @override
  String get settingsRuntimeScan => 'スキャン';

  @override
  String get settingsRuntimeScanning => 'スキャン中…';

  @override
  String get settingsRuntimeEnterManually => '手動で入力';

  @override
  String get settingsRuntimeConnect => '接続する';

  @override
  String get settingsRuntimeEnableDiscovery => '近くのデバイスがこのデバイススペースを検出できるようにする';

  @override
  String get settingsRuntimeEnableDiscoveryDescription =>
      '同じネットワーク上のデバイスは、このデバイス スペースを見つけて、直接接続するためにこのデバイスを選択できます。';

  @override
  String settingsRuntimeEnableDiscoveryFailed(String error) {
    return 'デバイススペース検出を有効にできませんでした: $error';
  }

  @override
  String settingsRuntimeDisableDiscoveryFailed(String error) {
    return 'デバイススペースの検出を無効にできませんでした: $error';
  }

  @override
  String get settingsRuntimeUsingLocal => '使用: このデバイス';

  @override
  String settingsRuntimeUsingRemote(String device) {
    return '使用: $device';
  }

  @override
  String get settingsRuntimeRemoteInUseDescription =>
      'チャットとツールは、この接続されたデバイス上で実行されます。';

  @override
  String get settingsWebAccessService => 'アクセスを許可する';

  @override
  String get settingsWebAccessServiceDescription =>
      '有効にすると、ブラウザはアドレスとトークンを使用してこのデバイスにアクセスできます。';

  @override
  String get settingsWebAccessEnable => '外部アクセスを許可する';

  @override
  String get settingsWebAccessPortMode => 'ポートモード';

  @override
  String get settingsWebAccessPortAutomatic => '自動';

  @override
  String get settingsWebAccessPortFixed => '修正済み';

  @override
  String get settingsWebAccessPortAutomaticDescription =>
      'アプリはポートを自動的に選択します。手動セットアップは必要ありません。';

  @override
  String get settingsWebAccessPortFixedDescription => 'リッスン アドレスのポートのみが使用されます。';

  @override
  String get settingsWebAccessBindAddress => 'リッスンアドレス';

  @override
  String get settingsWebAccessToken => 'アクセストークン';

  @override
  String get settingsWebAccessRotateToken => 'トークンの変更';

  @override
  String get settingsWebAccessCopyToken => 'トークンをコピーする';

  @override
  String get settingsWebAccessAccessUrl => 'アクセスアドレス';

  @override
  String get settingsWebAccessLocalUrl => 'このデバイス';

  @override
  String get settingsWebAccessPairingUrl => 'ペアリングアドレス';

  @override
  String get settingsWebAccessPairingUrlLocalOnly => 'この端末のみ';

  @override
  String get settingsWebAccessPairingUrlUnavailable => 'LANアドレスが見つかりません';

  @override
  String get settingsWebAccessCopyUrl => 'URLをコピー';

  @override
  String get settingsWebAccessOpenUrl => 'オープンアドレス';

  @override
  String get settingsWebAccessRunning => 'オン';

  @override
  String get settingsWebAccessStopped => 'オフ';

  @override
  String get settingsWebAccessSaved => 'アクセス設定が保存されました。';

  @override
  String get settingsWebAccessTokenCopied => 'アクセストークンがコピーされました。';

  @override
  String get settingsWebAccessUrlCopied => 'アクセスURLがコピーされました。';

  @override
  String get settingsWebAccessPairedClients => '認可されたデバイス';

  @override
  String get settingsWebAccessNoPairedClients => 'まだデバイスが認証されていません。';

  @override
  String get settingsWebAccessPairedDeleted => '認証されたデバイスが削除されました。';

  @override
  String get settingsWebAccessPairingRequest => 'ペアリングリクエスト';

  @override
  String settingsWebAccessPairingRequestMessage(String code, String client) {
    return 'ペアリングコード: $code\nデバイス: $client';
  }

  @override
  String get settingsWebAccessInvalidBindAddress =>
      'バインド アドレスはホスト:ポートである必要があります。';

  @override
  String settingsWebAccessStartFailed(String error) {
    return 'アクセスを有効にできませんでした: $error';
  }

  @override
  String settingsWebAccessStopFailed(String error) {
    return 'アクセスをオフにできませんでした: $error';
  }

  @override
  String get settingsAppearanceThemeSection => 'テーマ';

  @override
  String get settingsAppearanceThemeMode => '現在のモード';

  @override
  String get settingsAppearanceThemeTarget => 'テーマの保存先';

  @override
  String get settingsAppearanceThemeTargetGlobal => 'グローバル';

  @override
  String settingsAppearanceThemeTargetCharacter(Object name) {
    return '現在のキャラクター: $name';
  }

  @override
  String settingsAppearanceThemeTargetGroup(Object name) {
    return '現在のグループ: $name';
  }

  @override
  String get settingsAppearanceThemeSystem => 'システム';

  @override
  String get settingsAppearanceThemeLight => 'ライト';

  @override
  String get settingsAppearanceThemeDark => '暗い';

  @override
  String get settingsAppearanceInputSection => '入力';

  @override
  String get settingsAppearanceInputStyle => '入力スタイル';

  @override
  String get settingsAppearanceInputStyleClassic => 'クラシック';

  @override
  String get settingsAppearanceInputStyleAgent => 'エージェント';

  @override
  String get settingsAppearanceInputFloating => 'フローティング入力';

  @override
  String get settingsAppearanceColorSection => 'テーマカラー';

  @override
  String get settingsAppearanceColorDescription =>
      'シンプルな色のプリセットを選択します。システム バーと現在のアプリ クロムは自動的にテーマに従います。';

  @override
  String get settingsAppearanceColorDefault => 'デフォルト';

  @override
  String get settingsAppearanceColorSky => '空';

  @override
  String get settingsAppearanceColorMatcha => '抹茶';

  @override
  String get settingsAppearanceColorEmber => '残り火';

  @override
  String get settingsAppearanceColorRose => 'ローズ';

  @override
  String get settingsAppearanceColorCustom => 'カスタムカラー';

  @override
  String get settingsAppearanceCustomColorsTitle => 'カスタムテーマカラー';

  @override
  String get settingsAppearancePrimaryColor => '原色';

  @override
  String get settingsAppearanceSecondaryColor => '二次色';

  @override
  String get settingsAppearanceHexColorHint => '#RRGGBB';

  @override
  String get settingsAppearanceHexColorInvalid => '#RRGGBB 形式で色を入力してください';

  @override
  String get settingsAppearanceBackgroundSection => '背景';

  @override
  String get settingsAppearanceBackgroundDescription =>
      'アプリの背景としてローカルの画像またはビデオを選択します。アプリのサーフェスとシステム バーは自動的にテーマに従います。';

  @override
  String get settingsAppearanceBackgroundImage => 'バックグラウンドメディア';

  @override
  String get settingsAppearanceBackgroundNone => '何も選択されていません';

  @override
  String get settingsAppearanceBackgroundChooseImage => '画像を選択してください';

  @override
  String get settingsAppearanceBackgroundChooseVideo => 'ビデオを選択してください';

  @override
  String get settingsAppearanceBackgroundDisable => '背景を無効にする';

  @override
  String get settingsAppearanceBackgroundEnabled => 'バックグラウンドを有効にする';

  @override
  String get settingsAppearanceBackgroundOpacity => '背景の不透明度';

  @override
  String get settingsAppearanceBackgroundBlur => '背景をぼかす';

  @override
  String get settingsAppearanceBackgroundBlurRadius => 'ぼかしの強さ';

  @override
  String get settingsAppearanceBackgroundVideoMuted => 'ビデオの背景をミュートする';

  @override
  String get settingsAppearanceBackgroundVideoLoop => 'ループビデオの背景';

  @override
  String get settingsAppearanceTextSection => 'テキスト';

  @override
  String get settingsAppearanceFontFamily => 'フォント';

  @override
  String get settingsAppearanceFontDefault => 'デフォルト';

  @override
  String get settingsAppearanceCustomFont => 'カスタムフォント';

  @override
  String get settingsAppearanceFontCustom => 'カスタム';

  @override
  String get settingsAppearanceChooseCustomFont => 'カスタムフォントを選択してください';

  @override
  String get settingsAppearanceClearCustomFont => 'クリアカスタムフォント';

  @override
  String get settingsAppearanceFontSerif => 'セリフ';

  @override
  String get settingsAppearanceFontMonospace => 'モノラル';

  @override
  String get settingsAppearanceFontScale => 'フォントサイズ';

  @override
  String get settingsAppearanceAvatarSection => 'アバター';

  @override
  String get settingsAppearanceUserAvatar => 'ユーザーのアバター';

  @override
  String get settingsAppearanceAiAvatar => 'AIアバター';

  @override
  String get settingsAppearanceAvatarDefault => 'デフォルトのアバター';

  @override
  String get settingsAppearanceAvatarShape => 'アバターの形状';

  @override
  String get settingsAppearanceAvatarShapeCircle => 'サークル';

  @override
  String get settingsAppearanceAvatarShapeSquare => '正方形';

  @override
  String get settingsAppearanceChooseUserAvatar => 'ユーザーのアバターを選択';

  @override
  String get settingsAppearanceChooseAiAvatar => 'AIアバターを選択';

  @override
  String get settingsAppearanceClearUserAvatar => 'ユーザーアバターをクリアする';

  @override
  String get settingsAppearanceClearAiAvatar => 'クリアAIアバター';

  @override
  String get settingsAppearanceChatDisplaySection => 'チャット表示';

  @override
  String get settingsAppearanceMessageStyle => 'メッセージスタイル';

  @override
  String get settingsAppearanceMessageStyleClean => 'コマンド';

  @override
  String get settingsAppearanceMessageStyleCard => 'バブル';

  @override
  String get settingsAppearanceMessageColors => 'メッセージカラー';

  @override
  String get settingsAppearanceMessageColorsTheme => 'テーマに従う';

  @override
  String get settingsAppearanceMessageColorsSky => 'クリーンブルー';

  @override
  String get settingsAppearanceMessageColorsMatcha => '抹茶';

  @override
  String get settingsAppearanceMessageColorsInk => '暗い';

  @override
  String get settingsAppearanceMessageColorsCustom => 'カスタムメッセージの色';

  @override
  String get settingsAppearanceCustomMessageColorsTitle => 'カスタムメッセージの色';

  @override
  String get settingsAppearanceCursorUserBubbleColor => 'コマンドユーザーバブル';

  @override
  String get settingsAppearanceUserBubbleColor => 'ユーザーバブル';

  @override
  String get settingsAppearanceAiBubbleColor => 'AIバブル';

  @override
  String get settingsAppearanceUserTextColor => 'ユーザーテキスト';

  @override
  String get settingsAppearanceAiTextColor => 'AIテキスト';

  @override
  String get settingsAppearanceMessageSurface => 'グローバルテクスチャ';

  @override
  String get settingsAppearanceMessageSurfaceNormal => 'ノーマル';

  @override
  String get settingsAppearanceMessageSurfaceTransparent => '透明';

  @override
  String get settingsAppearanceUserBubbleFont => 'ユーザーバブルフォント';

  @override
  String get settingsAppearanceAiBubbleFont => 'AIバブルフォント';

  @override
  String get settingsAppearanceAdjustUserBubbleFont => 'ユーザーバブルフォントを調整する';

  @override
  String get settingsAppearanceAdjustAiBubbleFont => 'AIバブルフォントを調整する';

  @override
  String get settingsAppearanceEnableBubbleFont => 'バブル固有のフォントを有効にする';

  @override
  String get settingsAppearanceUserBubbleImage => 'ユーザーのバブル画像';

  @override
  String get settingsAppearanceAiBubbleImage => 'AIバブルイメージ';

  @override
  String get settingsAppearanceChooseUserBubbleImage => 'ユーザーバブルを選択';

  @override
  String get settingsAppearanceChooseAiBubbleImage => 'AIバブルを選択';

  @override
  String get settingsAppearanceClearUserBubbleImage => 'ユーザーバブルをクリアする';

  @override
  String get settingsAppearanceClearAiBubbleImage => 'クリアAIバブル';

  @override
  String get settingsAppearanceBubbleImageRenderMode => 'バブル画像モード';

  @override
  String get settingsAppearanceBubbleImageTiledNineSlice => 'タイル状 9 スライス';

  @override
  String get settingsAppearanceBubbleImageNinePatch => 'ストレッチ9パッチ';

  @override
  String get settingsAppearanceBubbleImageAdjustUser => 'ユーザーのバブル画像を調整する';

  @override
  String get settingsAppearanceBubbleImageAdjustAi => 'AIバブル画像を調整する';

  @override
  String get settingsAppearanceBubbleImagePreview => 'プレビュー';

  @override
  String get settingsAppearanceBubbleImagePreviewText =>
      '9 スライス ガイドによるバブル プレビュー';

  @override
  String get settingsAppearanceBubbleImageCrop => '作物';

  @override
  String get settingsAppearanceBubbleImageRepeat => 'リピート領域';

  @override
  String get settingsAppearanceBubbleImageScale => '画像スケール';

  @override
  String get settingsAppearanceBubbleImageCropLeft => '左をトリミング';

  @override
  String get settingsAppearanceBubbleImageCropTop => 'クロップトップ';

  @override
  String get settingsAppearanceBubbleImageCropRight => '右にトリミング';

  @override
  String get settingsAppearanceBubbleImageCropBottom => '作物の底';

  @override
  String get settingsAppearanceBubbleImageRepeatStart => 'リピートXスタート';

  @override
  String get settingsAppearanceBubbleImageRepeatEnd => 'リピート X 終了';

  @override
  String get settingsAppearanceBubbleImageRepeatYStart => 'リピートYスタート';

  @override
  String get settingsAppearanceBubbleImageRepeatYEnd => 'リピートYエンド';

  @override
  String get settingsAppearanceMessageDensity => 'メッセージの間隔';

  @override
  String get settingsAppearanceMessageDensityComfortable => '快適';

  @override
  String get settingsAppearanceMessageDensityCompact => 'コンパクト';

  @override
  String get settingsAppearanceWideLayout => 'より広いチャット レイアウトを使用する';

  @override
  String get settingsAppearanceRoundedMessages => '丸いメッセージカード';

  @override
  String get settingsAppearanceShowAvatars => 'メッセージアバターを表示';

  @override
  String get settingsAppearanceMessageDisplaySection => 'メッセージ表示';

  @override
  String get settingsAppearanceShowThinkingProcess => '思考プロセスを示す';

  @override
  String get settingsAppearanceShowRoleName => 'ロール名を表示';

  @override
  String get settingsAppearanceShowUserName => 'ユーザー名を表示';

  @override
  String get settingsAppearanceShowModelName => 'モデル名を表示';

  @override
  String get settingsAppearanceShowModelProvider => 'モデルプロバイダーを表示';

  @override
  String get settingsAppearanceShowMessageTokenStats => 'トークンの統計を表示する';

  @override
  String get settingsAppearanceShowMessageTimingStats => 'タイミング統計を表示する';

  @override
  String get settingsAppearanceShowMessageTimestamp => 'メッセージ時間を表示する';

  @override
  String get settingsAppearanceShowInputProcessingStatus => '入力処理状況の表示';

  @override
  String get settingsAppearanceResetTheme => 'テーマ設定をリセットする';

  @override
  String get settingsAppearanceLanguageSection => '言語';

  @override
  String get settingsAppearanceLanguage => '現在の言語';

  @override
  String get settingsAppearanceLanguageDescription =>
      '言語は、アプリの起動時に読み込まれるローカリゼーション設定に従います。';

  @override
  String get settingsDataRuntimeSection => 'データの概要';

  @override
  String get settingsDataCoreVersion => '現在のバージョン';

  @override
  String get settingsDataStorageSection => '保管場所';

  @override
  String get settingsDataStorageDescription =>
      'ランタイム データとワークスペース データを、個別に選択したローカル フォルダーに移動します。';

  @override
  String get settingsDataRuntimeRoot => 'ランタイムルート';

  @override
  String get settingsDataWorkspaceRoot => 'ワークスペースルート';

  @override
  String get settingsDataChooseStorageRoots => '場所を編集する';

  @override
  String get settingsDataEditStorageRootsTitle => '保管場所を編集する';

  @override
  String get settingsDataStorageRootsRequired => 'ランタイム ルートとワークスペース ルートが必要です。';

  @override
  String get settingsDataStorageConfirmTitle => '保存場所を変更する';

  @override
  String get settingsDataStorageConfirmMessage =>
      'ランタイム データとワークスペース データは選択したディレクトリにコピーされ、アプリは再起動後にそれらを使用します。';

  @override
  String get settingsDataStorageConfirmAction => '場所を変更する';

  @override
  String get settingsDataStorageChanged => '保管場所が変更されました。アプリを再起動して使用してください。';

  @override
  String settingsDataStorageChangeError(String error) {
    return '保存場所の変更に失敗しました: $error';
  }

  @override
  String get settingsDataTokenSection => '使用状況の統計';

  @override
  String get settingsDataInputTokens => '入力';

  @override
  String get settingsDataOutputTokens => '出力';

  @override
  String get settingsDataOpenDetailedStats => '詳細な統計を表示する';

  @override
  String get settingsDataOpenDetailedStatsDescription =>
      '毎日の傾向、入出力トークンの変化、プロバイダー、モデル、会話ごとの使用量の内訳を開きます。';

  @override
  String get settingsDataRefreshTokenStats => '統計を更新する';

  @override
  String get settingsDataResetTokenStats => '統計をリセットする';

  @override
  String get settingsDataDetailedStatsTitle => '詳細な統計';

  @override
  String get settingsDataDetailedStatsDescription =>
      '統計は、専用のモデル要求レコードから計算されます。';

  @override
  String get settingsDataDetailedStatsEmpty => '詳細な使用記録はまだありません';

  @override
  String settingsDataDetailedStatsDateRange(String start, String end) {
    return '$start ～ $end';
  }

  @override
  String get settingsDataDetailedStatsSourceLabel => 'モデルリクエストレコード';

  @override
  String get settingsDataDetailedStatsSourceChat => 'チャットの応答';

  @override
  String get settingsDataDetailedStatsSourceToolResult => 'ツール結果の応答';

  @override
  String get settingsDataDetailedStatsSourceSummary => 'サマリーの生成';

  @override
  String get settingsDataDetailedStatsSourceTitleGeneration => 'タイトル生成';

  @override
  String get settingsDataDetailedStatsSourceMemory => 'メモリ分析';

  @override
  String get settingsDataDetailedStatsTotalRequests => 'リクエストの合計';

  @override
  String get settingsDataDetailedStatsCachedInput => 'キャッシュされた入力';

  @override
  String get settingsDataDetailedStatsActiveDays => '活動的な日';

  @override
  String get settingsDataDetailedStatsChats => '会話';

  @override
  String get settingsDataDetailedStatsProviders => 'プロバイダー';

  @override
  String get settingsDataDetailedStatsModels => 'モデル';

  @override
  String get settingsDataDetailedStatsDailyUsageTitle => '日々の使用量の傾向';

  @override
  String get settingsDataDetailedStatsDailyUsageSubtitle => '日別のリクエスト数';

  @override
  String get settingsDataDetailedStatsRequestsSeries => 'リクエスト';

  @override
  String get settingsDataDetailedStatsInputOutputTitle => '入出力消費傾向';

  @override
  String get settingsDataDetailedStatsInputOutputSubtitle => '入力と出力の毎日のトークン変更';

  @override
  String get settingsDataDetailedStatsProviderPieTitle => 'プロバイダーの配布';

  @override
  String get settingsDataDetailedStatsModelPieTitle => 'モデルの配布';

  @override
  String get settingsDataDetailedStatsChatPieTitle => '会話配信';

  @override
  String get settingsDataDetailedStatsTotalTokens => '総トークン数';

  @override
  String get settingsDataDetailedStatsTopRequestsTitle => '上位のリクエスト';

  @override
  String get settingsDataDetailedStatsTopRequestsSubtitle =>
      '単一リクエストのトークン消費量が最も多い';

  @override
  String get settingsDataDetailedStatsTopChatsTitle => 'よくある会話';

  @override
  String get settingsDataDetailedStatsTopChatsSubtitle => '会話による合計トークン消費量が最も多い';

  @override
  String get settingsDataDetailedStatsOther => 'その他';

  @override
  String settingsDataDetailedStatsInputOutputSummary(
    String input,
    String output,
    String chatTitle,
    String time,
  ) {
    return '入力 $input · 出力 $output · $chatTitle · $time';
  }

  @override
  String settingsDataDetailedStatsRequestModelSummary(
    int requests,
    int models,
  ) {
    return '$requests リクエスト · $models モデル';
  }

  @override
  String get settingsDataDetailedStatsUnlabeledProvider => 'ラベルのないプロバイダー';

  @override
  String get settingsDataDetailedStatsUnlabeledModel => 'ラベルなしモデル';

  @override
  String get settingsDataDetailedStatsUntitledChat => '無題の会話';

  @override
  String get settingsDataBackupSection => 'バックアップと復元';

  @override
  String get settingsDataChatHistoriesBackup => 'チャットデータ';

  @override
  String get settingsDataChatHistoriesBackupDescription =>
      'すべてのチャットとメッセージをバックアップします。チャット ID によって更新を復元するか、チャットを作成します。';

  @override
  String get settingsDataCharacterCardsBackup => 'キャラクターカードデータ';

  @override
  String get settingsDataCharacterCardsBackupDescription =>
      'すべてのキャラクター カードと参照されているタグをバックアップします。元の ID に基づいて更新または項目を復元します。';

  @override
  String get settingsDataCharacterGroupsBackup => 'グループデータ';

  @override
  String get settingsDataCharacterGroupsBackupDescription =>
      'すべてのグループをバックアップします。復元では、メンバーの参照と順序が保持されます。';

  @override
  String get settingsDataModelConfigsBackup => 'モデル設定';

  @override
  String get settingsDataModelConfigsBackupDescription =>
      'モデル パラメーターや API キー プールを含むすべてのモデル設定をバックアップします。';

  @override
  String settingsDataBackupCount(int count) {
    return '$count アイテム';
  }

  @override
  String get settingsDataCopyBackupJson => 'バックアップのコピー';

  @override
  String get settingsDataImportBackupJson => 'データを復元する';

  @override
  String get settingsDataBackupJsonInput => 'コンテンツを復元する';

  @override
  String settingsDataBackupCopied(String name) {
    return '「$name」のバックアップをコピーしました。';
  }

  @override
  String settingsDataBackupImportResult(
    int newCount,
    int updatedCount,
    int skippedCount,
  ) {
    return '復元が完了しました: 新規 $newCount 件、更新 $updatedCount 件、スキップ $skippedCount 件。';
  }

  @override
  String settingsDataBackupImportError(String error) {
    return '復元に失敗しました: $error';
  }

  @override
  String settingsDataBackupCopyError(String error) {
    return 'コピーに失敗しました: $error';
  }

  @override
  String get settingsDataSnapshotBackupTitle => '完全なスナップショット';

  @override
  String get settingsDataExportRawSnapshot => 'スナップショットのエクスポート';

  @override
  String get settingsDataImportRawSnapshot => 'スナップショットを復元する';

  @override
  String get settingsDataExportRawSnapshotDescription =>
      'チャット、キャラクター、モデル設定、ローカルファイルを 1 つのバックアップ ファイルにまとめます。復元すると、現在のデータがバックアップに置き換えられます。';

  @override
  String settingsDataSnapshotBytes(int bytes) {
    return 'スナップショットのサイズ: $bytes バイト';
  }

  @override
  String get settingsDataSnapshotImported => 'スナップショットが復元されました。';

  @override
  String settingsDataSnapshotExportError(String error) {
    return 'スナップショットのエクスポートに失敗しました: $error';
  }

  @override
  String settingsDataSnapshotImportError(String error) {
    return 'スナップショットの復元に失敗しました: $error';
  }

  @override
  String get settingsDataSnapshotRestoreConfirmTitle => '完全なスナップショットを復元する';

  @override
  String settingsDataSnapshotRestoreConfirmMessage(
    int formatVersion,
    int fileCount,
    String createdAt,
    int bytes,
  ) {
    return '復元すると、現在のランタイムデータが置き換えられます。\n形式バージョン: $formatVersion\nファイル数: $fileCount\n作成日時: $createdAt\nスナップショットサイズ: $bytes バイト';
  }

  @override
  String get settingsDataSnapshotRestoreConfirmAction => '復元';

  @override
  String get settingsDataImportOperit1Snapshot => 'Operit 1から引き継ぐ';

  @override
  String get settingsDataOperit1SnapshotImported => 'Operit 1の引き継ぎが完了しました。';

  @override
  String settingsDataOperit1SnapshotImportError(String error) {
    return 'Operit 1の引き継ぎに失敗しました: $error';
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
    return 'このOperit 1の引き継ぎファイルを現在のRuntimeへ読み込みます。\nファイル: $fileName\n形式: $formatVersion\nチャットモデル: $chatModelId\nチャット: $chatCount件、メッセージ: $messageCount件\nデータファイル: $fileCount件\nファイルサイズ: $byteCountバイト';
  }

  @override
  String get settingsDataOperit1SnapshotImportAction => '引き継ぐ';

  @override
  String get settingsDataAdvancedBackupOptions => '詳細オプション';

  @override
  String get settingsDataAdvancedBackupOptionsDescription =>
      '単一アイテムの JSON エクスポートと復元';

  @override
  String get onboardingIntroTagline => '毎日の作業を、ここからもっと簡単に';

  @override
  String get onboardingStart => 'はじめる';

  @override
  String get onboardingPleaseWait => 'しばらくお待ちください';

  @override
  String get onboardingAgree => '同意する';

  @override
  String get onboardingAgreementProgress => '利用規約';

  @override
  String get onboardingSkip => 'スキップ';

  @override
  String get onboardingPrevious => '前のページ';

  @override
  String get onboardingAgreementTitle => '利用規約・プライバシー';

  @override
  String get onboardingAgreementDescription => '内容を確認し、同意すると端末の設定を続けられます。';

  @override
  String onboardingAgreementVersion(String version) {
    return 'バージョン：$version';
  }

  @override
  String onboardingAgreementWait(int seconds) {
    return 'あと$seconds秒で同意できます。';
  }

  @override
  String get onboardingAgreementPlainTitle => 'わかりやすい要約（法的な正文ではありません）';

  @override
  String get onboardingAgreementPlainIntro =>
      'Operitは端末上で動作するオープンソースのクライアントです。Operit自身はAIモデルの推論サービスやチャット履歴の保管、共用APIキーの提供を行いません。クラウドモデル、音声、検索、画像生成、MCPなどのネットワーク機能を設定すると、データは各サービス提供者へ直接送信され、その利用規約とプライバシーポリシーが適用されます。ローカルモデルは端末内で処理されます。';

  @override
  String get onboardingAgreementPlainCapabilities =>
      'このアプリは、ファイル、ターミナル、自動操作、システム権限、Root、ADB、拡張機能などを利用する場合があります。実行内容を確認し、大切なデータをバックアップしたうえで、権限は慎重に許可してください。操作、設定、外部サービス、拡張機能によって端末・データ・アカウントなどに損害が生じた場合は、実際に操作した人が適用法令に従って対応する責任を負います。';

  @override
  String get onboardingAgreementPlainThirdParty =>
      'マーケットのプラグイン、スクリプト、Skill、ツールパッケージなど第三者のコンテンツに関する著作権と責任は、各作者または権利者に帰属します。表示やインストールは、Operitによる保証・推奨・権利取得を意味しません。';

  @override
  String get onboardingAgreementLegalTitle => '正式な法的規約';

  @override
  String get onboardingAgreementLegalSection1 =>
      '1. 適用範囲と規約のバージョン\n本規約は、Operitが公式に公開するクライアントと任意のオンライン機能に適用されます。本アプリを使用すると、現在の規約を読み、同意したものとみなされます。アプリは同意したバージョンを記録し、重要な変更があった場合は改めて同意を求めます。オープンソースのライセンスは、リポジトリ直下のLICENSEに記載されたLGPLv3に従います。本規約は、適用法令やオープンソースライセンスによって認められた権利を除外または制限するものではありません。';

  @override
  String get onboardingAgreementLegalSection2 =>
      '2. 製品の位置づけと外部サービス\nOperitは、大規模言語モデルの推論、共用APIキー、チャット要求の中継、チャット履歴のクラウド保管を提供しません。外部サービスはご自身で選択・設定・有効化し、サービス提供者、モデル、接続先、拡張機能の安全性・適法性・適合性を判断して、認証情報を適切に管理してください。';

  @override
  String get onboardingAgreementLegalSection3 =>
      '3. データの処理とプライバシー\nチャット履歴、キャラクターカード、メモリー、モデル設定、APIキーは、通常、端末上のアプリデータに保存されます。エクスポート、バックアップ、ファイルのアップロード、外部ネットワーク機能、外部HTTPサービスへの送信を行うと、その操作に応じて関連データが複製・送信・開示されます。マーケット、お知らせ、更新確認、GitHubログイン、公開などの機能は、Operit、GitHub、または関連する外部サービスへアクセスします。';

  @override
  String get onboardingAgreementLegalSection4 =>
      '4. 外部公開と運用者の責任\n外部HTTPサービス、ボット、自動返信などの機能は、ご自身の判断で有効にします。他人や一般向けに公開する場合、実際の公開者・運用者として、アクセス制御、利用者の同意、コンテンツの安全性、データ保護、未成年者の保護、必要な表示など、適用される義務を負います。';

  @override
  String get onboardingAgreementLegalSection5 =>
      '5. 適法な利用とコンテンツへの責任\n適用法令、外部サービスの規約、プラットフォームのルールを守ってください。本アプリ、拡張機能、設定を、違法行為、他者の権利侵害、無許可でのシステムやデータへのアクセス、違法・有害コンテンツの配布に利用してはいけません。AIの出力には誤り、抜け、偏りが含まれる可能性があり、医療・法律・金融などの専門的な助言ではありません。';

  @override
  String get onboardingAgreementLegalSection6 =>
      '6. 現状のままでの提供\n適用法令で認められる範囲において、本ソフトウェアは「現状のまま」「利用可能な状態」で提供されます。貢献者は、本ソフトウェアや外部サービスの継続的な利用可能性、正確性、安全性、商品性、特定目的への適合性、第三者の権利を侵害しないことについて、明示・黙示を問わず保証しません。';

  @override
  String get onboardingAgreementLegalSection7 =>
      '7. 規約の更新と問い合わせ\n機能、法令、安全上の要件により本規約を更新し、アプリ内で現在のバージョンを提供する場合があります。利用者の権利に重要な影響がある更新は、規約のバージョンを上げ、改めて同意を求めることで有効になります。質問や意見は、プロジェクトのリポジトリ、アプリ内のフィードバック窓口、公開されている連絡先から送ることができます。';

  @override
  String get onboardingAgreementPrecedenceNote =>
      'わかりやすい要約は理解を助けるためのものです。中国語の正式な規約と内容が異なる場合は、中国語の正式な規約が優先されます。';

  @override
  String get onboardingStorageRequired => '実行データと作業フォルダーの両方を指定してください。';

  @override
  String get onboardingModelLoadFirst => '先に利用できるモデルを取得してください。';

  @override
  String get onboardingModelSelectDefault => '利用するモデルを選んでください。';

  @override
  String get onboardingImportSelectSnapshotError =>
      'Operit 1の引き継ぎファイルを選んでください。';

  @override
  String get onboardingRemoteCredentialsRequired => '接続先とアクセストークンの両方を入力してください。';

  @override
  String get onboardingRemoteCodeRequired => '1回限りの接続コードを入力してください。';

  @override
  String get onboardingActionSaving => '保存中';

  @override
  String get onboardingActionConfirm => '確認';

  @override
  String get onboardingActionPreparing => '準備中';

  @override
  String get onboardingActionContinue => '次へ';

  @override
  String get onboardingActionLoadingModels => '取得中';

  @override
  String get onboardingActionReading => '読み込み中';

  @override
  String get onboardingActionImporting => '引き継ぎ中';

  @override
  String get onboardingActionConnecting => '接続中';

  @override
  String get onboardingActionPairing => '確認中';

  @override
  String get onboardingActionStartPairing => '接続を始める';

  @override
  String get onboardingActionFinishConnection => '接続を完了';

  @override
  String get onboardingActionProcessing => '処理中';

  @override
  String get onboardingActionComplete => '完了';

  @override
  String get onboardingProgressStorage => '保存場所';

  @override
  String get onboardingProgressStartMode => 'はじめ方';

  @override
  String get onboardingProgressModel => 'モデル設定';

  @override
  String get onboardingProgressImport => 'データ引き継ぎ';

  @override
  String get onboardingProgressRemote => '別の端末へ接続';

  @override
  String get onboardingProgressDeviceSpace => 'デバイススペース';

  @override
  String get onboardingProgressPermissions => '端末の権限';

  @override
  String get onboardingProgressWelcome => 'ようこそ';

  @override
  String get onboardingPreparingLocalRuntime => '端末内の実行環境を準備しています';

  @override
  String get onboardingModeTitle => 'はじめ方を選ぶ';

  @override
  String get onboardingModeDescription =>
      '初めて使う場合はかんたん設定を選びます。Operit 1のデータを引き継ぐことも、別のRuntimeへ接続することもできます。';

  @override
  String get onboardingModeQuickTitle => 'かんたん設定';

  @override
  String get onboardingModeQuickSubtitle => 'AIサービスとモデルを設定して使い始めます';

  @override
  String get onboardingModeImportTitle => 'Operit 1から引き継ぐ';

  @override
  String get onboardingModeImportSubtitle => 'Operit 1の設定とデータを移します';

  @override
  String get onboardingModeRemoteTitle => '別のRuntimeへ接続';

  @override
  String get onboardingModeRemoteSubtitle => '接続先、アクセストークン、1回限りの接続コードを入力します';

  @override
  String get onboardingModeDeviceSpaceTitle => '既存のデバイススペースに参加';

  @override
  String get onboardingModeDeviceSpaceSubtitle =>
      '近くのデバイススペースを探し、参加後にモデルとデータを同期します';

  @override
  String get onboardingDeviceSpaceDescription =>
      '近くのデバイススペースから端末を選びます。接続後は、モデル・設定・データを同期してからセットアップを完了します。';

  @override
  String get onboardingRemoteDescription =>
      'このアプリを別のRuntimeの操作端末として使います。接続が完了すると、自動で接続先へ切り替わります。';

  @override
  String get onboardingRemoteAddress => '接続先';

  @override
  String get onboardingRemoteToken => 'アクセストークン';

  @override
  String get onboardingRemotePairingStarted => '接続の準備ができました';

  @override
  String onboardingRemotePairingSummary(
    String platform,
    String model,
    String deviceId,
  ) {
    return '接続先：$platform / $model\n端末：$deviceId\n接続先のアプリに表示された1回限りのコードを入力してください。';
  }

  @override
  String get onboardingRemotePairingCode => '1回限りの接続コード';

  @override
  String get onboardingRemoteBeforePairing => '接続先とアクセストークンを入力して、接続を始めます。';

  @override
  String get onboardingRemoteAfterPairing =>
      'コードは今回の確認だけに使われます。完了後、このRuntimeが接続先として保存されます。';

  @override
  String get onboardingStorageTitle => 'この端末の保存場所を確認';

  @override
  String get onboardingStorageDescription =>
      '実行データと作業フォルダーは別々に保存されます。パスを直接入力するか、それぞれのフォルダーを選んでください。';

  @override
  String get onboardingStorageRuntimeFolder => '実行データのフォルダー';

  @override
  String get onboardingStorageWorkspaceFolder => '作業フォルダー';

  @override
  String get onboardingStorageReading => '保存場所を確認しています';

  @override
  String get onboardingStorageDetail =>
      '実行状態と作業データを別々に保存します。確認後、この端末のHostが各フォルダーを使用します。';

  @override
  String onboardingStorageChooseFolder(String label) {
    return '$labelを選ぶ';
  }

  @override
  String get onboardingImportTitle => 'Operit 1から引き継ぐ';

  @override
  String get onboardingImportDescription =>
      'Operit 1の引き継ぎファイルを選び、設定、チャット、キャラクターカード、各種データをOperit 2へ移します。';

  @override
  String get onboardingImportReadingSnapshot => '引き継ぎファイルを確認中';

  @override
  String get onboardingImportSelectSnapshot => '引き継ぎファイルを選ぶ';

  @override
  String get onboardingImportDetected => '引き継げる内容';

  @override
  String get onboardingImportModelSettings => 'モデル設定';

  @override
  String get onboardingImportChats => 'チャット';

  @override
  String get onboardingImportMessages => 'メッセージ';

  @override
  String get onboardingImportPreferences => '設定ファイル';

  @override
  String get onboardingImportResources => 'データファイル';

  @override
  String get onboardingImportExternalResources => '外部データ';

  @override
  String onboardingImportDefaultModel(String modelId) {
    return '既定のチャットモデル：$modelId';
  }

  @override
  String get onboardingImportInProgress => 'データを引き継いでいます。しばらくお待ちください。';

  @override
  String get onboardingImportReady => '「次へ」を押すと、すべてのデータを引き継ぎます。';

  @override
  String get onboardingModelTitle => 'モデル設定を完了する';

  @override
  String get onboardingModelDescription =>
      'AIサービスを選び、APIキーを入力して、利用するモデルを設定します。';

  @override
  String get onboardingModelProvider => 'AIサービス';

  @override
  String get onboardingModelProviderRequired => 'AIサービスを選んでください。';

  @override
  String get onboardingModelContinueSetup => '設定を続ける';

  @override
  String get onboardingModelEndpoint => 'サービスの接続先';

  @override
  String get onboardingModelLoadingAvailable => 'モデルを取得中';

  @override
  String get onboardingModelLoadAvailable => '利用できるモデルを取得';

  @override
  String get onboardingModelDefault => '利用するモデル';

  @override
  String get onboardingPermissionsTitle => '必要な権限だけを許可する';

  @override
  String get onboardingPermissionsDescription =>
      '使う機能に合わせて権限を選べます。何も許可せず次へ進むこともできますが、許可していない機能は利用できません。';

  @override
  String get onboardingPermissionsEmptyTitle => 'この端末で確認が必要な権限はありません';

  @override
  String get onboardingPermissionsEmptySubtitle =>
      '初期設定で対応する権限はありません。そのまま次へ進めます。';

  @override
  String get onboardingPermissionsRefresh => '権限の状態を再確認';

  @override
  String get onboardingPermissionOptional => '任意';

  @override
  String get onboardingFieldRequired => '入力が必要です';

  @override
  String get onboardingPermissionGranted => '許可済み';

  @override
  String get onboardingPermissionGrant => '許可する';

  @override
  String get onboardingPermissionNoAction => '操作不要';

  @override
  String get networkErrorBadRequestTitle => '設定内容を確認してください';

  @override
  String get networkErrorBadRequestMessage =>
      'モデル一覧の取得が拒否されました。AIサービスと接続先の組み合わせを確認してください。';

  @override
  String get networkErrorUnauthorizedTitle => 'APIキーを確認できませんでした';

  @override
  String get networkErrorUnauthorizedMessage =>
      'AIサービスがAPIキーを受け付けませんでした。キー全体を入力し直して、もう一度モデルを取得してください。';

  @override
  String get networkErrorForbiddenTitle => '利用する権限がありません';

  @override
  String get networkErrorForbiddenMessage =>
      'このAPIキーではサービスを利用できません。アカウントの権限とモデルサービスの利用状態を確認してください。';

  @override
  String get networkErrorNotFoundTitle => '接続先が見つかりません';

  @override
  String get networkErrorNotFoundMessage =>
      'この接続先にはモデル一覧がありません。アドレスとパスを確認してください。';

  @override
  String get networkErrorRateLimitedTitle => 'しばらく待ってください';

  @override
  String get networkErrorRateLimitedMessage =>
      '短時間の利用回数が上限に達しました。少し待ってから、もう一度モデルを取得してください。';

  @override
  String get networkErrorServerTitle => 'AIサービスで問題が発生しています';

  @override
  String get networkErrorServerMessage => '現在、モデル一覧を取得できません。時間をおいて試してください。';

  @override
  String get networkErrorModelListTitle => 'モデル一覧を取得できませんでした';

  @override
  String get networkErrorModelListMessage =>
      'AIサービスからモデル一覧が返りませんでした。接続先、APIキー、ネットワークを確認してください。';

  @override
  String get networkErrorConnectionTitle => 'ネットワークへ接続できませんでした';

  @override
  String get networkErrorConnectionMessage =>
      'AIサービスへ接続できません。ネットワークとサービスの接続先を確認してください。';

  @override
  String get networkErrorDuplicateModelTitle => '追加済みのモデルです';

  @override
  String networkErrorDuplicateModelMessage(
    String modelId,
    String providerName,
  ) {
    return 'モデル「$modelId」はAIサービス「$providerName」へ追加済みです。';
  }

  @override
  String get networkErrorDefaultTitle => 'モデル設定を完了できませんでした';

  @override
  String get networkErrorDefaultMessage =>
      'モデルの取得中に問題が発生しました。AIサービス、接続先、APIキーを確認してください。';

  @override
  String get onboardingRequirementWindowsAdminTitle => '管理者権限';

  @override
  String get onboardingRequirementWindowsAdminDescription =>
      'Hostが管理者として動作しているか表示します。管理者として起動する操作はWindows側で行います。';

  @override
  String get onboardingRequirementAndroidFileManagementTitle => 'ファイルへのアクセス';

  @override
  String get onboardingRequirementAndroidFileManagementDescription =>
      '選んだAndroid共有フォルダーをHostが読み書きできるようにします。';

  @override
  String get onboardingRequirementAndroidNotificationsTitle => '通知';

  @override
  String get onboardingRequirementAndroidNotificationsDescription =>
      '実行中のサービス、タスクの進み具合、ツールの結果を通知できるようにします。';

  @override
  String get onboardingRequirementAndroidAppListTitle => 'インストール済みアプリ';

  @override
  String get onboardingRequirementAndroidAppListDescription =>
      'Androidアプリの一覧表示、起動、停止をできるようにします。';

  @override
  String get onboardingRequirementAndroidUsageStatsTitle => 'アプリの使用状況';

  @override
  String get onboardingRequirementAndroidUsageStatsDescription =>
      '各アプリを画面上で使用した時間を読み取れるようにします。';

  @override
  String get onboardingRequirementAndroidWriteSettingsTitle => 'システム設定の変更';

  @override
  String get onboardingRequirementAndroidWriteSettingsDescription =>
      '対応しているAndroidのシステム設定をHostから変更できるようにします。';

  @override
  String get onboardingRequirementAndroidLocationTitle => '近くの機器の検出';

  @override
  String get onboardingRequirementAndroidLocationDescription =>
      '近くの機器を見つける一部の機能で必要な位置情報を許可します。';

  @override
  String get onboardingRequirementAndroidBluetoothTitle => 'Bluetooth';

  @override
  String get onboardingRequirementAndroidBluetoothDescription =>
      'Bluetooth機器を見つけて接続できるようにします。';

  @override
  String get onboardingRequirementAndroidOverlayTitle => '他のアプリの上に表示';

  @override
  String get onboardingRequirementAndroidOverlayDescription =>
      '他のアプリを使用中でもOperitの入口を表示できるようにします。';

  @override
  String get onboardingRequirementAndroidBatteryOptimizationTitle => '長い処理を継続';

  @override
  String get onboardingRequirementAndroidBatteryOptimizationDescription =>
      '同期、連携、長い処理が止まりにくいように、Operitを電池最適化の対象外にします。';

  @override
  String get onboardingRequirementAndroidShizukuTitle => 'Shizuku';

  @override
  String get onboardingRequirementAndroidShizukuDescription =>
      '任意です。先にShizukuまたはSuiを起動し、対応しているAndroidシステム機能の利用を許可します。';

  @override
  String get onboardingRequirementAndroidRootTitle => 'Root';

  @override
  String get onboardingRequirementAndroidRootDescription =>
      '任意です。Rootが必要なAndroidシステム機能の利用を許可します。';

  @override
  String get onboardingRequirementOhosLocationTitle => '位置情報';

  @override
  String get onboardingRequirementOhosLocationDescription =>
      '端末の位置情報と、位置情報を必要とする近くの機器の検出を許可します。';

  @override
  String get onboardingRequirementOhosBluetoothTitle => 'Bluetooth';

  @override
  String get onboardingRequirementOhosBluetoothDescription =>
      'BluetoothとBLE機器の検出、接続、読み書きを許可します。';

  @override
  String get runtimeBootstrapPreparingAssets => '端末内の実行ファイルを準備しています';

  @override
  String get runtimeBootstrapInitializingCore => '端末内の基本サービスを起動しています';

  @override
  String get runtimeBootstrapFailed => '端末内のRuntimeを起動できませんでした';

  @override
  String get runtimeBootstrapReady => '端末内のRuntimeを利用できます';

  @override
  String get runtimeBootstrapUnconfigured => '先にメイン画面で実行データと作業フォルダーを選んでください';

  @override
  String get mainExitConfirm => 'もう一度戻ると終了します';

  @override
  String get messageMenuCopy => 'メッセージをコピー';

  @override
  String get messageMenuEditAndResend => '編集して再送信';

  @override
  String get messageMenuRollback => 'ここまで戻す';

  @override
  String get messageMenuRegenerate => 'もう一度生成';

  @override
  String get messageMenuModifyMemory => '記憶を編集';

  @override
  String get messageMenuPlayVoice => '音声を生成・再生';

  @override
  String get messageMenuDeleteVariant => '現在の候補を削除';

  @override
  String get messageMenuReply => '返信';

  @override
  String get messageMenuInsertSummary => '要約を挿入';

  @override
  String get messageMenuCreateBranch => '分岐を作成';

  @override
  String get messageMenuInfo => '詳細情報';

  @override
  String get messageMenuMultiSelect => '複数選択';

  @override
  String get messageMenuDeleteConfirmTitle => 'メッセージを削除しますか？';

  @override
  String get messageMenuDeleteConfirmMessage => 'このメッセージを削除します。よろしいですか？';

  @override
  String get messageMenuInfoTitle => 'メッセージの情報';

  @override
  String messageMenuSender(String value) {
    return '送信者：$value';
  }

  @override
  String messageMenuTimestamp(String value) {
    return '日時情報：$value';
  }

  @override
  String messageMenuRole(String value) {
    return '役割：$value';
  }

  @override
  String messageMenuModel(String value) {
    return 'モデル：$value';
  }

  @override
  String messageMenuProvider(String value) {
    return 'AIサービス：$value';
  }

  @override
  String messageMenuInputTokens(String value) {
    return '入力トークン：$value';
  }

  @override
  String messageMenuCachedInputTokens(String value) {
    return 'キャッシュ済み入力トークン：$value';
  }

  @override
  String messageMenuOutputTokens(String value) {
    return '出力トークン：$value';
  }

  @override
  String messageMenuWaitDuration(String value) {
    return '待ち時間：$valueミリ秒';
  }

  @override
  String messageMenuOutputDuration(String value) {
    return '出力時間：$valueミリ秒';
  }

  @override
  String get messageMenuConfirm => '閉じる';

  @override
  String get messageCopyTitle => 'メッセージをコピー';

  @override
  String get messageCopyPlainText => '通常のテキスト';

  @override
  String get messageCopyMarkdownSource => 'Markdownの元データ';

  @override
  String messageCopyPlainTextConversionFailed(String error) {
    return '通常のテキストへ変換できませんでした：$error';
  }

  @override
  String get messageCopyCopyPlainText => '通常のテキストをコピー';

  @override
  String get messageCopyCopyMarkdownSource => 'Markdownの元データをコピー';

  @override
  String messageCopyFailed(String error) {
    return 'コピーできませんでした：$error';
  }

  @override
  String get messageCopyCompleted => 'メッセージをクリップボードへコピーしました';

  @override
  String get messageEditorTitle => 'メッセージを編集';

  @override
  String get messageEditorMemoryTitle => '記憶を編集';

  @override
  String get messageEditorVisualMode => '見やすい表示';

  @override
  String get messageEditorPlainTextMode => '元データ';

  @override
  String get messageEditorSaveAndResend => '保存して再送信';

  @override
  String get messageEditorUpdateMemory => '記憶を更新';

  @override
  String get messageEditorPlainTextContent => 'メッセージの元データ';

  @override
  String get messageEditorContentParts => '内容のまとまり';

  @override
  String get messageEditorAddText => '文章を追加';

  @override
  String get messageEditorAddTag => 'タグを追加';

  @override
  String get messageEditorTextLabel => '文章';

  @override
  String get messageEditorTextHint => '文章を入力してください';

  @override
  String get messageEditorTagTitle => 'タグを編集';

  @override
  String get messageEditorTagName => 'タグ名';

  @override
  String get messageEditorTagNameHint => '例：memory';

  @override
  String get messageEditorAttributes => '属性（任意）';

  @override
  String get messageEditorAttributesHint => '例：type=\"note\"';

  @override
  String get messageEditorContent => '内容';

  @override
  String get aboutDescription =>
      'スマートフォンとパソコンで使えるAI作業環境です。チャット、作業フォルダー、ツール、プラグイン、MCP、遠隔接続、Webアクセスを利用できます。';

  @override
  String aboutVersion(String version) {
    return 'バージョン $version';
  }

  @override
  String get aboutProjectSection => 'プロジェクト';

  @override
  String get aboutSourceTitle => 'ソースコード';

  @override
  String get aboutDocumentationTitle => '使い方';

  @override
  String get aboutDocumentationSubtitle => 'READMEとコマンドラインの説明';

  @override
  String get aboutOpenSourceLicenses => 'オープンソースライセンス';

  @override
  String get aboutOpenSourceLicensesSubtitle => 'Operit2はAGPL-3.0で公開されています';

  @override
  String get aboutContactSection => '連絡先';

  @override
  String get aboutMaintainer => '開発者 AAswordman';

  @override
  String get aboutCopyright => '© 2025 - 2026 Operit. All rights reserved.';

  @override
  String get chatTtsNoMatchingCharacter => 'このメッセージに対応するキャラクターがありません。';

  @override
  String chatTtsMatchingCharacterCount(String name) {
    return '「$name」に一致するキャラクターカードは1件必要です。';
  }

  @override
  String get chatTtsEmptyMessage => '音声にするメッセージ内容がありません。';

  @override
  String chatTtsPlaybackFailed(String error) {
    return '音声を生成または再生できませんでした：$error';
  }

  @override
  String chatCopyFailed(String error) {
    return 'コピーできませんでした：$error';
  }

  @override
  String get chatConfirmDeleteSelectedTitle => '選択したメッセージを削除しますか？';

  @override
  String chatConfirmDeleteSelectedMessage(int count) {
    return '選択したメッセージを$count件削除しますか？';
  }

  @override
  String get chatGeneratingShareImage => '共有画像を作成しています...';

  @override
  String chatGenerateShareImageFailed(String error) {
    return '共有画像を作成できませんでした：$error';
  }
}
