# 终端配置管理

在「设置 → 工作区与浏览器 → 终端配置」中添加、编辑、删除配置，并选择当前
Core 设备的默认项。配置可以属于同一种终端：例如两套 Ubuntu、不同版本的
PowerShell，或多个 WSL 发行版。每套配置都有独立 ID 和会话上下文。

工作区的「＋」直接创建默认终端，右侧下拉打开配置选择器；单次选择不修改
默认项。新建的手动会话标题包含配置名。AI 未指定配置时使用相同的默认项。
已经创建的会话保留原宿主、根文件系统和启动参数；修改、切换或删除配置
都不迁移原会话。删除默认配置前需要先选择另一项。

## 发行版按需安装

应用继续只打包 Alpine。Android ARM64 和 OpenHarmony 的设置页提供 Ubuntu
24.04 LTS、Debian 13、Fedora 42 的 ARM64 下载目录。这里只内置发布链接、
大小和 SHA-256，不打包发行版归档。下载目录来自以下固定版本的上游元数据：

- [Ubuntu 24.04 元数据](https://github.com/termux/proot-distro/blob/v4.23.0/distro-plugins/ubuntu.sh)，镜像为 v4.18.0 的 ARM64 归档。
- [Debian 13 元数据](https://github.com/termux/proot-distro/blob/v4.31.0/distro-plugins/debian.sh)，镜像为 v4.29.0 的 ARM64 归档。
- [Fedora 42 元数据](https://github.com/termux/proot-distro/blob/v4.31.0/distro-plugins/fedora.sh)，镜像为 v4.29.0 的 ARM64 归档。上游明确标注 Android 15 及以上不兼容，下载确认框展示该说明。

用户为每次安装填写名称。即使重复下载同一发行版，也会安装到新的实例目录，
不会覆盖已有系统。完成后加入配置列表，由用户明确选择默认项。

终端和应用更新共用 GitHub 镜像列表和测速逻辑。每次传输开始前并发从上游和镜像
读取归档前 64 KiB，检查字节范围、归档总大小和 XZ 文件头，再选择实测速率最高的下载源。
发行版原行显示镜像测速、已下载大小、百分比与校验、解压进度。失败或取消后可在原行
使用相同名称重试，从已保存的字节偏移续传。安装前校验 SHA-256，
解压时限制路径、条目数量和展开大小，保留 guest 内的链接关系并检查 Bash
是否为 ELF64 / AArch64。符号链接在普通文件之后创建，guest 的绝对链接转换
为实例内的相对链接。失败或取消会清理本次解压目录，并保留可续传的下载文件；校验失败
会丢弃归档。注册配置失败时回收本次
新实例。安装器不向其他发行版执行 Alpine 软件包安装脚本。

删除配置仅移除启动入口，保留根文件系统及其中的用户文件，正在运行的会话
仍可使用原环境。重启后可以手动发起安装，复用已保存的下载字节。

## 平台注册的能力

| 平台 | 实现 / 解释器 | 可配置项 |
| --- | --- | --- |
| Android | proot / bash | 自定义 rootfs、目录挂载，或按需安装 ARM64 发行版 |
| Android | chroot / bash | 可信的 rootfs 目录、目录挂载，启动时需要 Root 授权 |
| Android | termux / bash | Termux 主目录或可由 Termux 访问的共享工作区 |
| Android | android-system / shell | 系统终端；权限使用现有 Android 权限配置 |
| Windows | native / powershell | PowerShell 5、7 等解释器的绝对路径和 JSON 参数数组 |
| Windows | native / bash | 本机 Bash 的绝对路径和参数，或从宿主发现的 WSL 发行版中选择 |
| Linux | native / bash | Bash 或兼容启动器的绝对路径和参数 |
| macOS | native / 已注册解释器 | 绝对路径和参数；保留原沙箱及解释器协议 |
| OpenHarmony | qemu-vroot / shell | 自定义 rootfs，或按需安装 ARM64 发行版 |
| iOS、Web 等受限 host | host 注册的实现 | 选择默认和命名配置；没有注册的字段不会在界面中显示 |

桌面端参数是 JSON 字符串数组，作为 argv 传入，不按 shell 文本拆分。应用会
附加所选解释器的交互参数和命令完成协议；程序必须与所选解释器语法匹配。
只有启动参数、没有程序路径的自定义配置会被拒绝。所有选项为空时，明确使用
该 host 的内置启动方式。

Windows 添加 WSL：在「添加终端配置」中选择 `WSL`，从运行 Core 的 Windows
账户已注册的发行版中选择一项。保存后可将其设为默认，也可在工作区下拉菜单中单次
选择。配置使用系统的 `wsl.exe`，已保存的显式启动器路径继续生效。host 使用该
发行版的 `wslpath` 转换工作区路径，通过 `--cd` 设置 guest 目录，再启动 Bash。
PowerShell 5 和 7 可分别填写 `powershell.exe` 和 `pwsh.exe` 的绝对路径保存。

Android 的 PRoot 启动保持其默认 seccomp 行为。自定义 rootfs 与托管运行时启动时
移除 `PROOT_NO_SECCOMP`，允许 PRoot 在设备内核支持时启用 seccomp 加速。

Android 可在添加或编辑配置时切换到 `Chroot（需要 Root）`。可以填写自行准备的
Linux rootfs，也可编辑已下载发行版的配置并沿用其路径。Chroot 使用会话独立的
挂载命名空间，Shell 以 Root 身份运行，请使用可信的 rootfs。

### Termux

安装并打开支持 `RUN_COMMAND` 的 Termux，在 Termux 内运行
`pkg install --no-install-recommends python`。在 `~/.termux/termux.properties`
中设置 `allow-external-apps = true` 并运行 `termux-reload-settings`，然后在
Android 的 Operit 应用权限中允许「在 Termux 中运行命令」。

部分 Android 系统会冻结后台的 Termux，导致工作区无法拉起命令服务。在 ColorOS 上，
可在 Termux 的应用信息中打开「耗电管理 → 完全允许后台行为」，重新打开一次 Termux，
再回 Operit 启动终端。

在终端配置中选择 `Termux` 并保存。起始目录可选 Termux 主目录，或 Termux 有权
读取的共享工作区。应用私有工作区需要选 Termux 主目录。将该配置设为默认后，终端工具
和工作区都会使用它，也可以在工作区下拉菜单中单次选择。会话以 Termux 的应用身份运行，
真实 PTY 通过带单次令牌的本机连接流式收发终端数据，支持交互、调整尺寸和退出状态。

`mounts` 是宿主能力声明中的通用选项，值为 JSON 数组。每项包含 `source`（宿主
目录）和 `target`（guest 绝对目录）。Android 的 PRoot 和 chroot 当前支持
可写目录挂载。两者都校验目标冲突，chroot 还拒绝符号链接挂载点。

`/operitapp` 属于应用 VFS 命名空间，不是终端里的挂载目录。工作区和默认对话
输出目录先通过所属 VFS 解析为用户当前配置位置下的宿主目录，再由所选终端
Host 按自身路径规则转换。这个入口与 `/virtual/android/root/...` 等平台 VFS
地址共用宿主目录解析，不要求各个终端认识 `/operitapp` 或建立同名挂载。

Android 的内置 PRoot 与自定义 PRoot 配置共用参数数组启动器，使用解析后的
终端目录作为工作目录。自定义目录中的空格、中文和引号保留在单个参数中，
不经 shell 拆分。旧的 `common.sh` 生成与探测启动链路已移除。PRoot 对真实
宿主目录的绑定属于该 Host 的文件系统实现，与应用 VFS 的地址别名分别处理。
配置声明 `source` → `target` 时，宿主目录及其子目录按最具体的 source 映射到
guest target；显式 `/virtual/linux`、`/mnt/linux` 地址继续表示 guest 内部路径。
PRoot 在启动前创建配置声明的 guest 挂载点。

桌面原生终端直接使用 VFS 对应的宿主路径，Windows WSL 通过所选发行版的
`wslpath` 转换；iOS ARM64 在创建会话前登记工作目录绑定；Web 通过 9P 将
浏览器文件系统暴露到 `/mnt/operit-host`；OpenHarmony 使用 `/mnt/host-root`。
这些转换均由 Host 实现，工作区界面和工具调用继续传递 VFS 地址。

## 插件接口

超级管理员插件统一提供 `list_terminals` 和 `terminal`，不再通过平台预设声明
固定的 powershell/bash/zsh/shell 工具。`close`、`get_screen`、`input` 保留。
AI 使用统一的 `package_wait` 领取包执行的后续输出和结果，原 `terminal_wait`
已删除。公开 SDK 的实现、解释器枚举继续描述真实环境，配置 ID 另行传递。

```typescript
const info = await Tools.System.terminal.info();
// info.types: host 注册的实现；info.profiles: 用户保存的配置及可用状态。
// info.defaultProfileId: 当前默认配置。
const result = await Tools.System.terminal.run({
  sessionName: "build",
  profileId: info.defaultProfileId,
  command: "pwd",
  waitMs: 20000,
});

// 包工具使用持续输出流，前台等待预算由外层 package_proxy 管理。
for await (const event of Tools.System.terminal.runStream({
  sessionName: "build", profileId: info.defaultProfileId, command: "pwd",
})) {
  if (event.type !== "completed") sendIntermediateResult(event);
}
```

`run` 同时省略 `profileId` 和 `terminalType` 时使用默认配置。保留旧调用的
`terminalType` 选择，但不能和 `profileId` 同时传入。具名创建支持
`Tools.System.terminal.create(name, undefined, profileId)`。已有 sessionId 的
执行始终进入原会话；默认项改变后也不会按同名会话重定向到另一个配置。

超级管理员会话按对话和配置隔离；前台等待结束后，命令继续在原会话运行，
后续同名命令按现有代次机制取得可用会话。命令输出中的 `terminal` 和
`terminalType` 仍为实际执行环境。
统一执行 ID、20秒等待和增量领取契约见 [包执行说明](package-executions.md)。

## 存储及实现边界

`TerminalHost.terminalProfileSupport` 声明表单字段和宿主发现的选项，`configuredTerminal` 生成
具有不可变参数的宿主。`terminalDistributions` 及安装接口属于 host，Flutter
不判断操作系统。安装下载和本地路径消费复用 native host 层；共享 runtime
不直接读写原生路径。应用先安装 `ProfileTerminalHost`，再安装统一的会话通知
包装层，使工作区和插件使用相同的配置、会话所有权与通知来源。

路径按 `RuntimeStorageLayout` 的命名定义和注册表风格声明：

| 定义 | 存储键 | 所有权 |
| --- | --- | --- |
| `TERMINAL_PROFILES` / `TERMINAL_PROFILES_PATH` | `runtime/config/preferences/terminal_profiles.json` | CoreNode |
| `TERMINAL_ROOTFS` / `TERMINAL_ROOTFS_DIR_PATH` | `runtime/terminal/rootfs` | CoreNode |

这些配置包含设备路径，不能作为 Space 数据同步。完整注册表包含 `revision`，
验证后原子写入；旧版本保存请求会返回冲突，防止覆盖其他编辑或安装新增的
配置。损坏的数据、未知配置、未知选项和失效默认项均返回实际错误。

### 内置名称本地化

宿主的 `TerminalTypeInfo.nameKey` 声明稳定的本地化标识，`description` 保留可读的
回退文本。Core 创建默认配置时将标识保存到 `TerminalProfile.nameKey`，并通过
`RuntimeTerminalTypeInfo.nameKey` 返回给客户端。客户端按 key 选择语言资源，不
匹配英文描述，也不通过运行客户端的操作系统推断远端终端。修改回退描述不会
改变本地化结果；新宿主的未知 key 或缺少 key 时，客户端显示回退名称。

名称归属由 Core 管理：未修改名称的保存保留 key，重命名或更换实现后清除 key，
新建的用户配置使用字面名称。设置列表、编辑框、工作区配置选择器和新建手动
会话标题使用同一套显示规则。编辑框未修改显示名称时仍提交原始名称，避免把
当前语言的译文写成用户自定义名称。

注册表 `schemaVersion: 1` 记录此名称归属规则。缺少版本的旧数据只迁移一次：
按实现和解释器查找对应宿主，仅为名称仍与宿主默认描述相同的 `builtin_<数字>`
配置补上 key；自定义名称和无法确认来源的名称原样保留。迁移原子写入并增加
`revision`，不修改配置 ID、默认项和启动参数。之后不再根据描述判断名称归属。

## CLI 管理

`operit2 cli terminal` 提供类型发现、实例增删改、默认选择、完整注册表保存、发行版
下载和会话管理。CLI 使用同一 Core 注册表，完整保存仍校验 `revision`。
Windows 的 PowerShell 及 WSL 示例、其他界面配置的命令见 [CLI 设置](cli-settings.md)。

## 检查命令

```sh
cargo test --manifest-path core/Cargo.toml -p operit-host-api --test terminal_profiles
cargo test --manifest-path hosts/common/operit-host-native-common/Cargo.toml --features terminal-rootfs TerminalRootfs::tests
cargo check --manifest-path core/Cargo.toml -p operit-runtime
cargo check --manifest-path hosts/windows/Cargo.toml --tests
node --test tools/tests/terminal_working_directory_contracts.test.mjs tools/tests/terminal_streaming.test.mjs tools/tests/android_terminal_privileges.test.mjs tools/tests/super_admin_terminal_routing.test.mjs
```

Flutter 检查在 `apps/flutter/app` 下执行，代理代码由现有 proxy 生成流程产生：

```sh
fvm flutter gen-l10n
fvm flutter test --no-pub test/terminal_profiles_settings_test.dart test/workspace_home_material_test.dart test/terminal_launch_error_test.dart
```
