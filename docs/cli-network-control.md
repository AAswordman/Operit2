# CLI 网络控制与空间审批

CLI 与 Flutter 共用 `CoreApplication`、`RuntimeRemoteLinkService` 和同一套权限检查。
配对只授予设备通信授权，加入 Space 必须另行申请并由分配到的审批人明确批准。

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
token show
pair-start <对方-node-id> <地址> tcp
pair-finish <pairing-id> <对方显示的六位验证码>
space join <对方-node-id>
space requests outgoing
```

- 局域网免 token 配对要求接收端开启发现，且来源通过实际连接地址校验。
- 使用 `--no-discovery`、回环地址或其他不满足局域网条件的连接时，需要显式 token：
  `pair-start <node-id> <地址> tcp --token <接收端-token>`。
- HTTP 地址形如 `http://host:port/link`，WS 地址形如 `ws://host:port/link`。
- 单纯配对不授予空间权限。申请获批并加入同一 Space 后，双方通过原已认证连接交换空间专用回连凭证，**不需要再反向配对一次**。双方需运行新版本并启用 TCP/HTTP/WS 监听；回连地址使用实际接入来源 IP 与已绑定监听端口，不依赖发现开关。
- 空间回连凭证不改变配对方向或设备角色；每次使用重新检查当前空间、成员资格、原配对和撤销状态。离开空间或撤销后旧凭证不可用。
- `space status <node-id>` 沿用 Core 的空间状态语义：尚未加入同一空间时返回 `RemovedFromSpace`，不表示配对被删除。加入后再检查在线状态。
- `space join` 输出正常申请状态、审批人、请求编号和 `assignment-version`，等待审批不是命令失败。
- `space leave` 只重置本机状态，不会通知对方。之后再次 `space join` 同一目标时，接收端会把旧的申请/批准记录退役（源 Space 已不同），按一条全新申请重新走审批；同一源 Space 的重复提交仍会被拒绝。旧记录被退役后不可再被审批。

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

- `disconnect <device>`：策略级隔离。立即断开该设备，并禁止其直连与路由中转；成员资格、身份与配对保留，`admit <device>` 解除限制后才允许重连。
- `admit <device>`：解除上一行的隔离；不对该设备发起连接，链路由其自身恢复。
- `remove <device>`：彻底遗忘，不是“断开连接”。清除策略记录、成员资格、加入申请历史与本侧配对凭证，设备回到陌生人状态。
- `unpair <node-id>`（`link unpair`）：只删除配对授权，保留空间成员资格；它删的是本机与该设备之间的那条直连边，不影响其它成员。

链路与成员资格是两条轴：`pair`/`unpair` 只增减本机与某台设备之间的直连边，`admit`/`disconnect`/`remove` 作用在成员资格与策略上。
因此给只有中继路径的成员补一条直连不需要重新入网：配对完成即生效；该设备已是本空间成员时不再提交加入申请，重复提交会被核心按“已是成员”拒绝。
TUI 设备窗口（`/network devices`）与 Flutter 设备行都提供这个入口，配对时带目标 node id 绑定握手，仍需要对方显示、由本端输入六位确认码。

## 角色、策略与设备控制

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

`control identity clear` 把设备身份重置为默认 User（自带 `chat.read` 等普通成员能力）。
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

原生 Peer runtime 在监听启动时尝试恢复已保存的出站配对和仍有效的同 Space 返回通道；
不要求对方先被标成在线，不需要重新配对，也不依赖发现设备或手动同步。
后台每 3 秒执行一次有鉴权的可用性检查，单次尝试上限 5 秒，最多 4 个并发对端。
成功鉴权才产生在线证据；连接失败会撤掉在线证据，之后继续重试。
连接变化触发现有持久化同步，不把同步层变成连接管理器。
停止监听会取消并等待重连任务退出；已被断开或移除的节点不会被后台任务自动接回。
测试重用原身份、配对数据和监听端口，覆盖返回通道恢复及双向远端聊天订阅，不清理应用数据。
