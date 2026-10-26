# CLI 网络控制与空间审批

CLI 与 Flutter 共用 `CoreApplication`、`RuntimeRemoteLinkService` 和同一套权限检查。
配对只授予设备通信授权，加入 Space 必须另行申请并由分配到的审批人明确批准。

## TUI 启动时的监听

TUI 与 Flutter 一样在启动时应用已保存的监听配置，不需要额外传参：首次运行使用存储迁移写入的默认
`http,ws`、`0.0.0.0:37195` 与发现开关，之后沿用上次保存的传输集合。

- `--link-listen <transport>[,<transport>...]`：用给定传输替换并保存暴露配置，随后立即启动监听；
  与 `operit2 cli link listen` 一致默认自动避让端口。显式指定时绑定失败会让启动失败。
- 不传 `--link-listen`：按已保存配置启动。Host 不支持的传输会被跳过；绑定失败（例如端口被其它
  实例占用）只写入日志并在 TUI 状态行提示，不阻止终端启动。
- `--no-listen`：本次启动只出站，不改动已保存的配置。

## 常驻终端

```sh
operit2 cli link session tcp --bind 0.0.0.0:37195
```

`session` 同时运行监听和后台同步。下列命令始终使用**同一个运行中的应用实例**，不会逐条重启，退出用 `quit` 或 Ctrl-C。
也可以选择 `http`、`ws` 或 `http,ws`；TCP 不能与 HTTP/WS 共用同一监听端口。

终端内输入：

```text
space show
control show
peers
prompts
pair-start <对方-node-id> <地址> tcp
pair-finish <pairing-id> <对方显示的六位验证码>
space join <对方-node-id>
space requests outgoing
```

- 所有传输都通过接收端显示的 6 位配对码确认；发现开关和连接来源不改变配对流程。
- HTTP 地址形如 `http://host:port/link`，WS 地址形如 `ws://host:port/link`。
- 单纯配对不授予空间权限。申请获批并加入同一 Space 后，双方通过原已认证连接交换空间专用回连凭证，**不需要再反向配对一次**。双方需运行新版本并启用 TCP/HTTP/WS 监听；回连通告携带已绑定端口和具体监听 IP；通配绑定或旧通告使用实际接入来源 IP。出站连接与监听地址可以属于不同地址族，不依赖发现开关。
- 空间回连凭证不改变配对方向或设备角色；每次使用重新检查当前空间、成员资格、原配对和撤销状态。离开空间或撤销后旧凭证不可用。
- `space status <node-id>` 沿用 Core 的空间状态语义：尚未加入同一空间时返回 `RemovedFromSpace`，不表示配对被删除。加入后再检查在线状态。
- `space join` 输出正常申请状态、审批人、请求编号和 `assignment-version`，等待审批不是命令失败。
- `space leave` 只重置本机状态，不会通知对方。之后再次 `space join` 同一目标时，接收端会把旧的申请/批准记录退役（源 Space 已不同），按一条全新申请重新走审批；同一源 Space 的重复提交仍会被拒绝。旧记录被退役后不可再被审批。

Windows、Linux、macOS 的 CLI 使用非空 `OPERIT_CLI_CONFIG_DIR` 时，插件 IPC 均按当前身份的 runtime 根路径隔离，
不需要另设 `TMPDIR`。宿主分别使用命名管道、D-Bus 服务名和 Unix socket 目录承载实例。
外部 SDK 要连接指定实例，可为 CLI 和 SDK 客户端共同设置 `OPERIT_PLUGIN_SDK_INSTANCE`（例如 `test-a`），
它优先于自动实例选择。配置目录和实例均未指定时使用标准 SDK 端点。
显式实例需先启动对应 CLI；SDK 不会用默认应用的启动入口代替该实例。
Apple 宿主仍可通过 `OPERIT_PLUGIN_SDK_SOCKET_DIR` 指定 socket 目录，该目录覆盖实例对应的自动目录。
Apple 端点文件锁一直持有至监听和连接清理完成，重复启动不会删除已有实例的 socket。

## 审批人终端

```text
space requests incoming
space approve <request-id> <assignment-version>
# 或者
space reject <request-id> <assignment-version>
```

只有被分配到且仍具备权限的设备能够审批。版本号取自当前请求，旧版本会被拒绝，不能绕过审批分配。
申请者随后查询并完成加入：

```text
space refresh <request-id>
space show
```

其他申请操作：

```text
space cancel <request-id>
space leave
space rename <name>
space sync
```

`space leave` 保留配对，创建独立的新空间，并在创建流程中初始化本机管理员权限；不额外修复旧申请或旧空间状态。
`space sync` 手动等待一次同步，常驻会话正常情况下也会响应业务数据和设备变化自动同步。

设备操作使用统一词汇：`space` 与 `control device` 两棵命令树、TUI 的 `/network` 及 Flutter 控制面板对下列动词含义一致。
命令动词保持 `disconnect`/`admit`；TUI 设备窗口与 Flutter 设备菜单的显示文案为「隔离设备 / 解除隔离设备」（Isolate device / Lift isolation），与下述语义一一对应。

- `disconnect <device>`：策略级隔离。立即断开该设备，并禁止其直连与路由中转；成员资格、身份与配对保留，`admit <device>` 解除隔离后才允许重连。
- `admit <device>`：解除上一行的隔离；不对该设备发起连接，链路由其自身恢复。
- `remove <device>`：彻底遗忘，不是“断开连接”。清除策略记录、成员资格、加入申请历史与本侧配对凭证，设备回到陌生人状态。
- `unpair <node-id>`（`link unpair`）：只删除配对授权，保留空间成员资格；它删的是本机与该设备之间的那条直连边，不影响其它成员。

在线移除会尽最大努力把移除通知送达：先送给因这次移除而失去路径的成员（例如被移除桥另一侧的设备，通知沿移除前解析出的路由借道该桥转发），最后才通知被移除设备本身；被移除设备用本机重放证据自校验后自退役（成为独立空间）并删除全部配对（保留它正在用于重新申请的那条连接），其余成员在应用移除的当刻删除与被移除设备的本侧配对——即“互相遗忘”，遗忘是即时的、不延迟到后续清扫。遗忘按移除操作记账（同一操作无论经哪条路径先送达都只生效一次），并比对**配对世代号**：每台设备持有一个世代号（离网/退役时重铸，握手与设备资料携带），移除命令记录的是它被移除时公告的世代，成员只删除世代相符（或来自旧 core、没有世代）的配对——因此离线迟学、或在移除之后才入网的设备，即便到那时才处理这条移除，也不会删掉它在此期间与该设备新配的边；成员表收缩同样只依据重放后的当前策略，被重新接纳的成员不会被历史移除踢出。送达是尽力而为：目标离线或链路断开时移除照常完成，落下的设备在重新连通后经正常同步收敛。被移除设备可重新接纳：重新配对后在客户端提交加入申请即可，**当次生效**（若它还没有退役——离线错过通知或旧 core——申请会先对账并让它自行退役为自己的独立空间，再照常提交）。接收侧永远只按自己重放出的策略行事，不采信任何一台设备的转述。

链路与成员资格是两条轴：`pair`/`unpair` 只增减本机与某台设备之间的直连边，`admit`/`disconnect`/`remove` 作用在成员资格与策略上。
因此给只有中继路径的成员补一条直连不需要重新入网：配对完成即生效；该设备已是本空间成员时不再提交加入申请，重复提交会被核心按“已是成员”拒绝。
TUI 设备窗口（`/network devices`）与 Flutter 设备行都提供这个入口，配对时带目标 node id 绑定握手，仍需要对方显示、由本端输入六位确认码。

## 角色、策略与设备控制

内置身份为 Administrator、User、Relay 和 Edge。User 具备查看成员设备、网络中继和承载 Core 执行能力；
Relay 具备查看成员设备和网络中继能力；Edge 具备查看成员设备和承载 Core 执行能力。
`runtime.execute` 描述设备能否承载 Core 执行，`switch_core` 和初始 Binding 分配据此选择目标，并检查目标确实提供 Core。
聊天读写不再单独设置权限，仍通过已认证的空间成员关系和 Binding 路由。
User 和 Relay 默认具有 `storage.provide`，Edge 默认没有；审计和其他能力也可
通过自定义身份分配。权限角色不表示节点实际实现了相应服务，设备侧职责见
[Edge 架构](edge-architecture.md)。

```text
control show
control audit
control identity list
control identity define Reviewer approve join view
control identity set <device-id> Reviewer
control identity clear <device-id>
control identity set <device-id> User
control policy list
control policy set <policy-name> <value>
control device list
control device disconnect <device-id>
control device remove <device-id>
```

审批角色需要同时具备 `approve`（`network.approval`）和 `join`（`network.members.join`）。
普通成员不能修改网络策略，CLI 不会替它提升权限。

`control identity clear` 把设备身份重置为默认 User（自带查看成员设备、网络中继和承载 Core 执行能力）。
成员始终拥有身份，不存在无身份状态：身份被清空的设备会失去包括自身 UI 依赖能力在内的全部能力，
且单机空间内没有任何带内恢复路径。

`control device disconnect` 保留成员与配对，仅禁止直连和路由中转，`control device admit` 可恢复；
`control device remove` 彻底遗忘设备：清除策略记录与加入申请历史、把它移出空间成员并删除本侧配对凭证。
被移除设备回到陌生人状态，重新接入需要重新配对并重新申请加入，不存在移除黑名单。

## 在同一进程里执行业务命令

```text
core chat new --group CLI-sync-test
core chat list
core prefs media-history 37 41
core prefs show
```

带空格的参数使用 JSON 参数数组，避免引入第二套 shell 转义规则：

```json
["space", "rename", "My Space"]
```

启动时加 `--json`，每条命令输出 JSON；执行失败返回 `{"error":"..."}`，常驻会话仍可继续接收命令。
已有单次执行方式也支持新增审批命令，例如：

```sh
operit2 cli --json link space requests incoming
operit2 cli --json link space approve <request-id> <assignment-version>
```

## 两个独立 CLI 的端到端回归

```sh
cargo build --manifest-path apps/cli/Cargo.toml --release
python3 apps/cli/tests/network_control_session.py --transport tcp
python3 apps/cli/tests/network_control_session.py --transport http
python3 apps/cli/tests/network_control_session.py --transport ws
```

脚本在 macOS/Linux 使用两个真实 PTY 终端、独立 CLI 进程和本机网络连接，不使用模拟 Router。
验证六位验证码配对、审批前保持独立、审批后加入、同进程退出重建及再次加入、拒绝/取消/旧版本拦截、角色和策略权限、双向自动偏好及聊天同步、双边重启、先启动一边再启动另一边、任意一边退出后的离线检测和自动重连、离线期间修改的自动同步、断开/移除。

通过 `OPERIT_CLI_CONFIG_DIR` 指定不同配置目录；目录里的 `storage.json` 指向各自的测试运行目录。
脚本所有数据和终端日志保存在它打印的临时目录中。在 macOS 还为测试 HOME 创建独立测试 Keychain，并检查真实用户的默认 Keychain 没有变化；不清除或修改 Flutter、模拟器、现有 CLI 的运行数据。

## 验证聊天路由，不只验证列表同步

常驻终端内执行 `chat-watch <chat-id>`，会通过与 Flutter 相同的生成代理入口打开
`chatMessagesFlow` 和 `chatStateFlow`，返回各自首个快照，10 秒未收到会明确失败。
双终端测试只配对 A → B，审批加入后同时验证 A、B 创建聊天后的双向状态订阅。

## 启动后的在线状态与自动重连

原生 Peer runtime 在监听启动时恢复已保存的出站配对；配对发起方负责建立和重连唯一的双向连接，接收方复用该连接。
不要求对方先被标成在线，不需要重新配对，也不依赖发现设备或手动同步。
后台每 3 秒执行一次有鉴权的可用性检查，单次尝试上限 5 秒，最多 4 个并发对端。
成功鉴权才产生在线证据；连接失败会撤掉在线证据，之后继续重试。
连接变化触发现有持久化同步，不把同步层变成连接管理器。
停止监听会取消并等待重连任务退出；已被断开或移除的节点不会被后台任务自动接回。
测试重用原身份、配对数据和监听端口，覆盖单连接恢复及双向远端聊天订阅，不清理应用数据。

## 设备配置与储存能力

`storage.provide` 表示保存共享业务数据副本：聊天历史与附件、插件共享数据和文件。
它不控制本机读写、聊天收发或设备 preference 同步。已加入空间、未被断开的 ESP
可以双向同步自己的小份配置，不接收聊天数据库、插件包、插件数据或文件副本。
插件包仍由宿主的安装机制管理，数据同步不会替设备安装插件。

配置按空间和目标设备隔离，每个设备最多 24 个键，完整明文文件最多 6144 字节；
本地使用加密 preference 保存，不建立 SQL 数据库或业务同步日志。
每个键独立合并，删除保留有界的删除标记，离线设备返回时不会把旧值恢复出来。
同一键的并发修改按逻辑版本、写入节点 ID 确定结果。

```text
space preferences <device-id>
space preference-set <device-id> <key> <JSON-value>
space preference-delete <device-id> <key>
space sync
```

例如配置 ESP 的独立聊天（示例中的地址、模型和密钥需要替换）：

```text
["space","preference-set","<esp-id>","chat","{\"endpoint\":\"https://example.invalid/v1/chat/completions\",\"model\":\"your-model\",\"apiKey\":\"your-key\",\"systemPrompt\":\"你是设备助手。\"}"]
["space","sync"]
```

API 地址是设备自身可访问的完整 chat-completions 地址。它需要支持流式 SSE 回复。
保存并同步后，ESP 自己请求服务，CLI 可以关闭。不要把真实密钥放进共享日志或截图。
配置编辑 ToolPkg 也可通过已有 `Tools.device(nodeId).Edge.execute` 调用
`edge.preferences` 的 `keys`、`read`（可传 `{key}` 单独读取）、`set`（`{key,value}`）、`remove`（`{key}`），无需运行在 ESP 上。

无交互的 CLI 在 Windows、Linux、macOS 上均可显式设置 `OPERIT_CLI_SECRET_STORE=file`，将密钥存于当前 CLI
身份的运行目录 `secrets/`，Unix 目录权限 0700、文件权限 0600；Windows 继承配置目录的访问权限。默认仍使用平台密钥存储。
集成测试使用此选项及临时 `OPERIT_CLI_CONFIG_DIR`，避免访问开发者的 macOS 钥匙串。
