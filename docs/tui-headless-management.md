# TUI 无头管理面板

没有 Flutter 客户端的 Linux 服务器可以直接用 TUI 完成模型配置、配对、入网和扩展管理。
管理入口统一为一个可操作面板，斜杠命令只负责打开对应页签，不再把长输出堆到状态栏。

## 入口

| 命令 | 打开的页签 |
| --- | --- |
| `/status` | 状态：身份、nodeId、设备名、Space、监听地址、模型、审批模式、待处理请求、包加载异常、日志路径 |
| `/package` | 工具包：已加载、可加载、加载失败 |
| `/plugin` | ToolPkg 插件：已加载、可加载、加载失败 |
| `/skill` | 技能：已安装、可加载、扫描错误 |
| `/mcp` | MCP 服务器 |
| `/log` | 运行日志尾部（最多 500 行） |
| `/network identities` | Space 身份（自定义角色） |

`Tab` / `Shift+Tab` 在页签间切换。`Esc` 关闭面板；详情、确认和输入框各有独立按键。

## 面板操作

状态和日志页签使用上下键滚动；`/log` 额外支持 `x` 清空日志。

工具包、插件、技能和 MCP：

- `Enter`：加载可加载项，或查看已加载项详情
- `e`：启用/禁用（工具包、插件、MCP）；身份页签中编辑自定义角色
- `d`：删除（破坏性操作需要再按 `Y` 确认）
- `i`：输入本地文件路径并导入（工具包、插件）
- `s` / `k`：启动/停止 MCP 服务器
- `r`：重新加载当前页签

工具包和插件面板会同时显示加载失败项（`!` 标记）。删除失败项会调用
`package delete-issue`，不需要先把它加载成可用包。

可加载项按管理归属拆分：脚本包（`.js`）只出现在工具包页签，ToolPkg 容器（`.toolpkg`）
只出现在插件页签。加载后它们各自留在原页签，不会出现"加载成功但条目消失"。

Space 身份：

- `n`：新建自定义角色
- `e`：编辑名称和能力（能力用逗号分隔的短名，或直接使用协议名）
- `d`：删除自定义角色；内置 `admin`、`user`、`relay`、`edge` 只读
- `Enter`：查看角色的能力明细

## 设备名

服务器默认显示为 `linux-client(cli)-x86_64`。用下面的命令改成可识别的名字：

```text
/network name home-server
```

名称写入本机链接身份并持久化，重启后仍然保留；Space 中的 nodeId 不变。

## 后台 daemon

Windows、Linux 和 macOS 都可以用 daemon 保持 Core、Space 同步和监听常驻：

```bash
operit2 daemon run                    # 前台运行，适合 systemd
operit2 daemon start                  # 派生后台进程并等待就绪
operit2 daemon status [--json]
operit2 daemon stop [--json]
```

- `run` 支持 `--no-listen` 和 `--link-listen <http|ws|tcp|serial|bluetooth>[,<transport>...]`；显式传输会写入本机监听配置，下一次启动继续使用。
- 同一 identity 只允许一个 Core owner：daemon 和 TUI 互斥，第二个会直接报错并给出现有 pid。短命 `operit2 cli` 命令会先尝试复用正在运行的 daemon/TUI，通过本机控制通道执行（Windows 命名管道，Unix domain socket），不会再启动第二个 Core。
- 交互式命令不会被路由：`chat shell`、`shell`、`model codex-login`、`tts`、`stt`、`export/import/backup`、`market`、`update`、`web`、`identity`、`link` 等仍在当前进程启动一个短命 Core；`cli help` 也会短暂启动 Core 读取插件命令目录。短命 Core 使用按进程隔离的 plugin SDK IPC 实例，且不再接管持久化同步 worker，因此可以和 daemon 并存；但 `link listen` 会额外占用监听端口，不要和 daemon 同时执行。
- 状态文件在 `<runtime identity>/daemon/status.json`，分离运行的 stdout/stderr 在 `<runtime identity>/logs/daemon.out`。Windows 控制通道为 `\\.\pipe\operit2-<identity hash>-control`，仅允许当前运行用户访问，并拒绝远程连接。Unix 控制通道为运行时目录下的 `operit2-<hash>/control.sock`，目录和 socket 仅允许当前用户访问。
- `operit2 daemon stop` 只停止 daemon；如果 owner 是 TUI，需要从 TUI 退出。

Windows PowerShell 示例（`operit2.exe` 所在目录）：

```powershell
.\operit2.exe daemon start
.\operit2.exe daemon status --json
.\operit2.exe cli chat list
.\operit2.exe daemon stop
```

`daemon start` 启动的 Windows 进程不创建控制台窗口，可以在关闭启动它的终端后继续运行。`daemon run` 则保持前台运行，可以用 Ctrl+C 停止。Windows 同样执行 identity 单实例约束：同一 identity 的 TUI 和 daemon 互斥；不同 identity 使用各自的命名管道。此命令不会注册 Windows 服务或开机启动。

systemd 示例：

```ini
[Unit]
Description=Operit2 Core daemon
After=network-online.target

[Service]
Type=simple
ExecStart=/home/you/.local/bin/operit2 daemon run
Restart=on-failure
KillSignal=SIGTERM
TimeoutStopSec=20

[Install]
WantedBy=default.target
```

## 使用限制与日志

- `link` 子命令仍会启动自己的短命 Core，尚未通过 control socket 路由；不要和 daemon 同时执行 `link listen`。
- Market/GitHub 登录仍是 loopback 回调，无头服务器需要端口转发；Codex 已有 device-code 流程可参照。
- 日志保留策略：`operit.log`/`toolpkg.log` 单文件上限 4 MiB（超限淘汰最早一半并留标记行），`daemon.out` 在 daemon 启动时裁剪到 4 MiB 内（保留最新 1 MiB）。

daemon 的控制协议和 identity 生命周期在 `apps/cli/src/daemon.rs`，平台选择集中在 `apps/cli/src/host.rs`。Windows 实现在 `hosts/windows/src/daemon.rs`；Linux 和 macOS 从各自 host 入口引用 `hosts/unix/daemon.rs`。`hosts/common/operit-host-native-common/src/DaemonHost.rs` 只定义共享字节流接口，不包含平台选择、传输实现或进程操作。

平台传输测试直接引用上述生产实现，可单独运行：

```sh
cargo test --manifest-path hosts/tests/daemon-fixture/Cargo.toml --lib daemon::
```
