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

下载以文件流写入临时安装目录；支持查看进度和取消。安装前校验 SHA-256，
解压时限制路径、条目数量和展开大小，保留 guest 内的链接关系并检查 Bash
是否为 ELF64 / AArch64。符号链接在普通文件之后创建，guest 的绝对链接转换
为实例内的相对链接。失败或取消会清理本次临时目录；注册配置失败时回收本次
新实例。安装器不向其他发行版执行 Alpine 软件包安装脚本。

删除配置仅移除启动入口，保留根文件系统及其中的用户文件，正在运行的会话
仍可使用原环境。设备断电或进程被强制终止时，下载进度不会跨进程恢复。

## 平台注册的能力

| 平台 | 实现 / 解释器 | 可配置项 |
| --- | --- | --- |
| Android | proot / bash | 自定义 rootfs，或按需安装 ARM64 发行版 |
| Android | android-system / shell | 系统终端；权限使用现有 Android 权限配置 |
| Windows | native / powershell | PowerShell 5、7 等解释器的绝对路径和 JSON 参数数组 |
| Windows | native / bash | Bash 的绝对路径和参数；填写 WSL 发行版时通过 wsl.exe 启动 |
| Linux | native / bash | Bash 或兼容启动器的绝对路径和参数 |
| macOS | native / 已注册解释器 | 绝对路径和参数；保留原沙箱及解释器协议 |
| OpenHarmony | qemu-vroot / shell | 自定义 rootfs，或按需安装 ARM64 发行版 |
| iOS、Web 等受限 host | host 注册的实现 | 选择默认和命名配置；没有注册的字段不会在界面中显示 |

桌面端参数是 JSON 字符串数组，作为 argv 传入，不按 shell 文本拆分。应用会
附加所选解释器的交互参数和命令完成协议；程序必须与所选解释器语法匹配。
只有启动参数、没有程序路径的自定义配置会被拒绝。所有选项为空时，明确使用
该 host 的内置启动方式。

Windows 添加 WSL：选择 `native / bash`，程序填写 `wsl.exe` 的绝对路径，
WSL 发行版填写 `wsl --list --quiet` 列出的名称，例如 `Ubuntu-24.04`。host 使用
该发行版的 `wslpath` 转换工作区路径，通过 `--cd` 设置 guest 目录，再启动带
完成协议的 Bash。PowerShell 5 和 7 可分别填写 `powershell.exe` 和 `pwsh.exe`
的绝对路径，保存为不同配置。

## 插件接口

超级管理员插件统一提供 `list_terminals` 和 `terminal`，不再通过平台预设声明
固定的 powershell/bash/zsh/shell 工具。`close`、`terminal_wait`、`get_screen`、
`input` 保留。公开 SDK 的实现、解释器枚举继续描述真实环境，配置 ID 另行传递。

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
```

`run` 同时省略 `profileId` 和 `terminalType` 时使用默认配置。保留旧调用的
`terminalType` 选择，但不能和 `profileId` 同时传入。具名创建支持
`Tools.System.terminal.create(name, undefined, profileId)`。已有 sessionId 的
执行始终进入原会话；默认项改变后也不会按同名会话重定向到另一个配置。

超级管理员会话按对话和配置隔离；前台等待结束后，命令继续在原会话运行，
后续同名命令按现有代次机制取得可用会话。命令输出中的 `terminal` 和
`terminalType` 仍为实际执行环境。

## 存储及实现边界

`TerminalHost.terminalProfileSupport` 声明表单字段，`configuredTerminal` 生成
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
