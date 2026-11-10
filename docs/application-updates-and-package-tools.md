# 应用更新与原生应用管理

## 分层与调用链

App、CLI、TUI 共用 `operit-util::GithubReleaseUtil` 的版本渠道、发布资产命名、发布检查、Host HTTP 下载、取消、安装包格式校验和失败文件清理。

App 的 `OperitApplication` 提供以下代理接口：

- `checkApplicationUpdate`：通过本机 `SystemOperationHost::applicationUpdateTarget` 获取架构与安装发行类型，按共享发布规则选择资产，返回版本、说明和发布页等展示信息。平台目录、注册表及 macOS 发行元数据由 Host 读取。
- `applicationUpdateProgressFlow`：推送已接收字节和总字节。
- `downloadApplicationUpdate(version)`：使用核心已选定的发布资产下载，返回不透明凭证。
- `cancelApplicationUpdateDownload`：取消当前 Host HTTP 下载。
- `installApplicationUpdate(receipt)`：更新模块验证凭证，通过 VFS 检查文件并取得文件引用，再调用通用的 `SystemOperationHost::installApp(source, timing)`。安装执行时机由 Host 报告的安装身份决定，Windows 更新要求当前进程退出后执行。版本选择、凭证、重试和安装状态复用属于更新模块，VFS 不包含任何安装或更新业务入口。

Flutter 使用本机 `ProxyCoreRuntimeBridge`，不受当前对话所选设备影响。它只保存展示状态和凭证、订阅进度、响应按钮。它不判断平台、ABI、发行渠道或安装格式，不解析 Info.plist，不发安装包 HTTP 请求，不创建、校验、删除或安装本地文件。代理代码通过项目既有的代码生成流程生成，没有新增平台 MethodChannel。

CLI/TUI 保留命令行和终端交互，其检查/下载继续调用同一共享实现。VFS 只负责路径映射和文件访问；解析结果实现通用 `FileSource` 文件引用接口，系统安装器和 CLI 原生 Host 消费这个引用，在 Host 内部取得本机路径。普通 `install_app` 复用 `PathMapper::resolveFileResourcePath` 接受绝对路径兼容输入，再通过 VFS 取得文件引用。`AppInstallTiming` 只描述立即执行或当前进程退出后执行，不包含更新版本、下载凭证或升级校验。无法处理延迟安装的 provider 明确报错。

## 文件和任务生命周期

发布查询和下载统一等待 `HttpHost::openHttpResponseStream` 的响应头、分块数据和结束回调。公共层不按 WASM 分流，不调用同步 HTTP，也不包装为阻塞任务；CLI/TUI 同样使用异步调用。取消通过已有下载控制令牌的事件订阅唤醒等待者，并关闭对应 Host 请求，等待响应头时也可以取消。

下载使用 `RuntimeStorageWriteHost` 的流式写入会话，保存在已登记为 Ephemeral 的 `runtime/temp/application_updates/<任务ID>/` 虚拟存储目录。共享层核对 GET 响应声明的长度和包格式后才提交会话；失败、取消、Future 被丢弃时丢弃会话并清理本次目录，其他任务不受影响。App 凭证和 CLI/TUI 输出统一保存或返回 `/operitapp/data/temp/application_updates/...` VFS 路径；通过已有 `VisualFileSystem::runtimePath` 映射存储命名空间，业务调用方通过 VFS 路径检查文件，并将解析出的 `FileSource` 文件引用交给系统安装接口。本机路径由安装 Host 消费文件引用时解析，文档资源挂载不能直接当作本机文件路径；CLI 解包目录位于对应下载任务内。共享下载层不使用系统临时目录、绝对路径或 `std::fs` 读写更新包。已完成下载保留在核心内，以供安装失败或授权后重试。前端会记住本次用户取消意图：即使取消请求传输失败，也不会在下载完成后自动进入安装；错误会在弹窗中显示。

下载凭证只在当前 runtime 有效。安装重试不会重新下载；凭证失效或文件已被外部清理时，界面提供重新下载。等待重启的安装任务复用既有结果，不重复启动安装助手。Windows/Linux 便携更新由原生助手等待父进程退出，备份、替换并重新启动；替换失败尝试回滚，路径和错误写入安装日志。

目前不提供“安装请求即升级成功”的反馈。`installer_opened` 表示还需系统确认；新进程使用新版本后，后续更新检查以编译进核心的版本为准。Windows MSI 助手另写安装结果记录；它表示安装器结果，不表示新进程健康检查通过。当前没有新版本启动健康检查或跨平台的持久化完成回执消费接口。

## Windows MSI 自身更新

Windows Host 将当前可执行文件所在目录与当前用户的 `Software\Operit\Operit2\Installer\InstallLocation` 比对，并通过 Windows Installer 核实固定 UpgradeCode 下的用户安装产品。另一目录中的便携副本保持 ZIP 发行身份；目录匹配而产品登记缺失或冲突时明确报错。MSI 安装版只选择 MSI，便携版只选择 ZIP；目前发布矩阵仅提供 x86_64 MSI。

复用现有的版本检查、下载、格式校验、凭证、进度和重启按钮。MSI 下载增加 Compound File 头校验；Host 在发布助手就绪标记前读取 MSI 数据库，核对产品族、产品名、厂商、架构、用户安装范围、资产版本名及 MSI 升级版本。安装身份查询与通用安装请求的执行由 `SystemOperationHost` 的平台实现负责，公共更新层不再额外包装阻塞调度。

助手复用便携安装的隐藏启动、独立脚本、日志重定向及就绪确认代码，存放在下载任务目录。准备完成返回 `restart_required`，沿用现有“重启并安装”交互；应用正常退出后也会触发已准备的安装。助手持有父进程句柄，等待其真正退出，确认同一安装副本没有其他运行实例，再以 `/qn /norestart` 升级到原安装目录。文件事务、产品登记、快捷方式和升级期间的数据保留由已有 MSI 配置负责。普通 `install_app` 的 MSI 安装仍保持交互行为。

返回码为 0 时，助手核对新产品登记后重新打开应用，并保存 `installed` 结果；3010 单独记录 `system_restart_required` 并提示用户重启 Windows。失败保存 `failed`、错误及日志路径，退出后的异常会通过原生错误窗口告知用户。结果保存在下载目录的 `installer-*.log.result.json`，Windows Installer 详细日志为 `installer-*.log.msi.log`。就绪前失败不会请求应用退出。

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

使用 fixture 验证共享发布规则、异步回调顺序、等待响应头时取消、Future 丢弃、虚拟存储会话提交和清理、VFS 安装映射、绝对路径兼容、资源挂载拒绝、格式/来源校验、目录隔离、原生归档路径安全、替换回滚、Desktop Entry 解析和 Flutter 反馈。截图使用真实 Flutter 组件和模拟版本/下载进度，不执行真实安装。

Windows 更新助手测试通过隔离的进程、安装器及产品登记数据验证执行顺序与失败结果，不安装真实软件；原生助手启动测试实际运行 Windows PowerShell，验证中文路径、参数传递及就绪确认。旧版 MSI 到新版 MSI 的完整升级仍需隔离 Windows 环境验证；Android/OpenHarmony 端到端安装需要各自 SDK 和设备。不能将组件测试描述为各平台安装实测。
