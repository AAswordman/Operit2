# 应用更新与原生应用管理

## 分层与调用链

App、CLI、TUI 共用 `operit-util::GithubReleaseUtil` 的版本渠道、发布资产命名、发布检查、Host HTTP 下载、取消、安装包格式校验和失败文件清理。

App 的 `OperitApplication` 提供以下代理接口：

- `checkApplicationUpdate`：核心判断当前平台、架构和 macOS 发行类型，返回版本、说明和发布页等展示信息。
- `applicationUpdateProgressFlow`：推送已接收字节和总字节。
- `downloadApplicationUpdate(version)`：使用核心已选定的发布资产下载，返回不透明凭证。
- `cancelApplicationUpdateDownload`：取消当前 Host HTTP 下载。
- `installApplicationUpdate(receipt)`：核心验证凭证，取出原生文件路径，调用已有 `AIToolHandler.executeTool(install_app)`，返回安装状态和说明，不把原始工具结果中的安装包路径作为前端参数。

Flutter 使用本机 `ProxyCoreRuntimeBridge`，不受当前对话所选设备影响。它只保存展示状态和凭证、订阅进度、响应按钮。它不判断平台、ABI、发行渠道或安装格式，不解析 Info.plist，不发安装包 HTTP 请求，不创建、校验、删除或安装本地文件。代理代码通过项目既有的代码生成流程生成，没有新增平台 MethodChannel。

CLI/TUI 保留命令行和终端交互，其检查/下载继续调用同一共享实现。CLI 自身二进制安装由现有 CLI 原生安装实现负责，App 包安装由 `SystemOperationHost::installApp` 负责。

## 文件和任务生命周期

每次下载创建独立目录，不删除或覆盖其他更新任务的目录。App 使用核心配置的 runtime 根目录，避免在 Android 等平台猜测系统临时目录。Host HTTP 保证不发布不完整目标文件；共享更新层再次检查长度和格式。失败、取消时删除本次目录。已完成下载保留在核心内，以供安装失败或授权后重试。前端会记住本次用户取消意图：即使取消请求传输失败，也不会在下载完成后自动进入安装；错误会在弹窗中显示。

下载凭证只在当前 runtime 有效。安装重试不会重新下载；凭证失效或文件已被外部清理时，界面提供重新下载。等待重启的安装任务复用既有结果，不重复启动安装助手。Windows/Linux 便携更新由原生助手等待父进程退出，备份、替换并重新启动；替换失败尝试回滚，路径和错误写入安装日志。

目前不提供“安装请求即升级成功”的反馈。`installer_opened` 表示还需系统确认；新进程使用新版本后，后续更新检查以编译进核心的版本为准。此实现没有新增持久化的升级完成回执，也没有在安装助手中做新版本启动健康检查。

## 安装状态

`AppOperationData.installState` 是可选的原生结果字段，旧数据仍可反序列化。

| 状态 | 含义 | App 反馈 |
| --- | --- | --- |
| `permission_required` | 原生安装权限未就绪 | 提示授权，保留安装包，提供继续安装 |
| `installer_opened` | 已交给系统安装器 | 提示完成系统确认，可再次打开 |
| `restart_required` | 当前进程退出后可执行已准备的替换 | 提供重启并安装 |
| `completed` | 原生同步安装操作已完成 | 仍按核心版本核实自身升级结果 |

安装/卸载工具的 `success` 表示原生请求被成功执行或接受；具体是否仍需确认由结构化状态和 details 表达。系统工具包不会将“打开安装器/应用设置”直接改写成“安装/卸载成功”。跨平台卸载接口不能保证 `keep_data=true`，因此明确拒绝该选项。

## 应用管理标识和能力

所有实现继续使用现有的安装、卸载、列表、启动、停止和使用时长工具。

| 平台 | 应用标识 | 已实现路径与限制 |
| --- | --- | --- |
| Android | package name | APK 校验、安装来源授权、系统安装/卸载、可见应用列表、Activity 启动；停止要求已授权 Shizuku/Root；使用时长要求 Usage Access |
| macOS | bundle ID，缺少 ID 时使用 bundle 路径 | 扫描用户和系统 Applications；pkg/mpkg/dmg 交给系统，app 通过 Finder 复制；启动、退出、移入废纸篓；保护系统应用和正在运行的宿主 |
| Windows | `win32:<registry path>` / `appx:<package name>` | 注册表及 Appx 列表、原生安装/卸载、注册启动入口；停止按完整可执行路径或包目录匹配；不明确的启动入口返回错误 |
| Linux | Desktop Entry ID | XDG 用户/系统及 Flatpak/Snap 目录；启动经 gio；停止按当前用户的精确可执行路径或 Flatpak ID；卸载经 Flatpak、Snap、Debian 或 RPM 归属查询，不把删除快捷方式算作卸载 |
| OpenHarmony | bundle name | HAP 安装器、应用设置中的用户卸载、系统授权可见的列表和启动；不伪造任意应用停止能力；使用时长读取受权限保护的实际聚合毫秒值 |
| iOS / Web | 无任意系统应用管理标识 | 保留明确的能力限制错误；不返回假成功或空的“成功”使用记录 |

桌面端未接入可靠的前台历史记录器，使用时长接口明确返回当前缺少数据源，不再将进程存活时长当作前台使用时长。`include_system_apps` 在各桌面目录/注册表实现中使用该平台可见的系统组件标记；它不是统一的安全分类。

## 验证边界

使用 fixture 验证共享发布规则、格式/来源校验、取消、目录隔离、原生归档路径安全、替换回滚、Desktop Entry 解析和 Flutter 反馈。截图使用真实 Flutter 组件和模拟版本/下载进度，不执行真实安装。

开发机能够检查 macOS 和共享 Rust 核心；Windows/Linux 完整交叉检查还依赖对应 C 编译工具链，Android/OpenHarmony 端到端安装需要各自 SDK 和设备。不能将组件测试描述为各平台安装实测。
