Operit2 — Windows 卸载与用户数据

MSI 安装版
  在 Windows 设置 → 应用 → 已安装的应用中卸载 Operit2。
  也可以通过开始菜单的 Operit2 Uninstall，或再次打开 MSI → 卸载应用。
  卸载默认保留用户数据。完整卸载界面可勾选“同时永久删除本机用户数据”，
  查看实际数据目录并确认后，才会执行数据清理。
  只显示系统简化界面的卸载入口会保留数据；需要清理数据时，请使用完整卸载界面，
  或先运行本目录的 Operit2DataCleanup.exe，再卸载应用。

压缩包版
  请先退出所有 Operit2 窗口、后台进程和命令行。
  运行 Operit2DataCleanup.exe，查看实际目录，确认后可永久清理本机用户数据。
  清理结束后可以删除解压的程序目录。
  需要先删除程序、稍后清理时，可先将 Operit2DataCleanup.exe 单独保存到其他目录。
  该工具不依赖 Flutter、Rust DLL 或 MSI，但需要 Windows 自带的 .NET Framework 4.7.2 或更新版本。

清理范围
  %APPDATA%\Operit2（包括固定启动配置、默认数据和工作区）
  启动配置中记录的自定义 runtimeRoot、workspaceRoot 目录及其全部内容
  %LOCALAPPDATA%\app.operit\Operit2（托管运行环境）
  %LOCALAPPDATA%\Operit2（崩溃报告）
  %LOCALAPPDATA%\webview_all_windows\operit2（浏览器缓存、Cookie 等）
  Windows 凭据管理器中以 Operit2: 开头的应用凭据
  清理会覆盖本机所有身份；安装版、压缩包版和命令行可能共用这些数据。
  请先导出需要保留的聊天、插件数据和工作区文件。
  清理不会向其他设备或云端发送删除指令。
  目录经过链接、指向受保护位置、配置损坏或文件被占用时会明确报错。
  文件删除无法回滚；中途失败会说明未全部完成，请按提示处理后重新打开工具。

程序入口
  MSI 普通卸载会清除属于当前安装副本的桌面、开始菜单和 operit2:// 协议入口。
  独立清理工具只负责数据，不卸载 MSI，不删除其他副本的程序或快捷方式。
  删除程序目录不会自动删除存放在 AppData 或自定义位置的用户数据。

无人值守 MSI
  默认保留数据：msiexec /x {ProductCode} /qn
  明确要求清理：msiexec /x {ProductCode} /qn OPERIT_DELETE_USER_DATA=1
  请使用当前用户身份执行，关闭所有 Operit2 进程，并在运行前备份自定义目录。
  自动升级、同版本覆盖安装、修复均不会清理用户数据。
