# QQ Bot

将 `assistance/examples/qqbot` 迁入 Operit2，使用 ToolPkg API `2.0.0`。包 ID 保持为 `com.operit.qqbot_bundle`，作为可选官方插件放在 `plugins/packages/external/qqbot`。

Gateway 直接使用 `Tools.Net.webSocket`；HTTP 和附件下载分别使用 `Tools.Net.http`、`Tools.Files`。配置页使用 Compose DSL。Python Gateway、终端会话、本地控制服务器、端口和 shell 命令均已移除。

插件声明 `installation_scope: "device"`，只允许安装在“仅本设备”，不能移入共享空间。AppID、AppSecret、监听开关、自动回复配置和会话绑定保存在该设备的插件配置中，不随空间扩展同步。由选定的一台设备运行同一个 Bot 的监听和自动回复；Waifu 依赖可独立选择安装范围。

## 使用

1. 先导入并启用 Waifu 模式及角色卡依赖插件，在应用中配置好聊天模型。再在 QQ Bot 管理端配置应用及 C2C、群消息事件权限，准备 AppID 和 AppSecret。
2. 在“接入 QQ”中填写 AppID、AppSecret，点击“保存并验证”。验证使用本次输入的凭据，通过后才保存；已保存的密钥不会回显，留空保留，修改 AppID 时需要重新填写密钥。测试机器人可打开沙箱环境。
3. 在“设置回复”中直接选择角色卡或角色组，也可以跟随当前角色；选择回复私聊、群消息或两者。超时、回复要求、聊天记录分组和 Waifu 回复效果配置放在“高级设置”中，等待时间以秒填写。
4. 点击“启动机器人”，会保存当前接入信息和回复设置、验证连接并启动自动回复。只有收到 QQ 的 `READY` 后才显示连接成功。启动后下次打开应用会恢复运行；点击“停止机器人”会同时停止连接和回复，并取消下次自动启动。
5. 运行状态和具体错误显示在页面顶部。刷新状态、停止机器人和保存另一部分设置，都不会覆盖未保存的输入。单独“保存回复设置”不会启动已停止的机器人。

每个私聊和群各自绑定一个持久化的原生聊天，更换回复角色仅对新会话生效。应用必须保持运行才能继续接收消息。停止机器人会关闭连接并取消正在执行的 AI 请求。

“诊断信息”提供待处理事件数量、可读的消息预览和清空队列操作。消息预览不消费事件；自动回复已处理的消息在聊天记录中查看。需要手动收发时，也可以使用事件工具读取 `replyHint`，再调用发送工具；`openid` / `group_openid` 来自 QQ 事件，不是 QQ 号。

## 工具

| 工具 | 用途 |
| --- | --- |
| `usage_advice` | 读取设置与消息回复说明 |
| `qqbot_configure` | 保存 AppID、AppSecret、沙箱和监听设置 |
| `qqbot_status` / `qqbot_dashboard_status` | 读取连接、队列、自动回复状态 |
| `qqbot_test_connection` | 验证鉴权、机器人资料和 Gateway |
| `qqbot_service_start` / `qqbot_service_stop` | 启停连接；启动支持显式 `restart` |
| `qqbot_receive_events` | 按场景和事件名称读取或消费事件 |
| `qqbot_clear_events` | 清空队列 |
| `qqbot_send_c2c_message` / `qqbot_send_group_message` | 发送文本或通过 JSON `payload` 提交 QQ 消息字段 |
| `qqbot_auto_reply_configure` | 设置自动回复场景、模型超时、会话分组、角色选择、Waifu 配置名称及指令 |
| `qqbot_auto_reply_status` | 查看自动回复和持久化聊天绑定 |
| `qqbot_auto_reply_start` / `qqbot_auto_reply_stop` | 启停自动回复，启动前需开启监听 |
| `qqbot_auto_reply_run_once` | 消费一条符合场景设置的消息 |
| `qqbot_upload_image` | 上传真实图片，返回 `file_info`，不发送消息 |
| `qqbot_waifu_preview` | 在指定会话中调用真实 Waifu 依赖，返回分段及时间，不向 QQ 发送 |

发送回复时传入原始 `msg_id`。服务按场景、目标和原消息维护发送序号，也接受显式 `msg_seq`。富消息参数通过 `payload` JSON 文本提供，显式字段决定 `content`、`msg_type`、`msg_id`、`event_id` 和 `msg_seq`。

## 生命周期与边界

- 设置页和工具通过包内 IPC 调用同一个 main runtime；Gateway、队列和自动回复只有一个所有者。
- 自动回复声明 `com.operit.waifu >= 1.0.0` 依赖，并复制该插件的公开客户端。客户端收集原生 `Chat.sendMessageStreaming` 的语义快照，由 Waifu main runtime 分句并匹配表情。QQBot 逐段发送文本，图片通过 `/v2/users/{openid}/files` 或 `/v2/groups/{openid}/files` 上传后以 `msg_type: 7` 发送。所有消息段共用原始 `msg_id` 和单调递增的序号。最终回执不会重复发送。
- `waifu_profile` 指定依赖插件中的命名配置（初始为 `default`），打字节奏、标点、表情和自拍规则统一在 Waifu 模式页管理。取消、被拦截、被消费和超时均明确报错，已经交付的消息段保留。
- 消息图片等附件按 QQ 的 `attachments` 字段通过宿主文件接口下载，作为结构化 Chat 附件提交。最多 10 个附件，每个限制 20 MiB；文件保存在当前插件配置目录的 `incoming` 子目录，保留聊天附件引用。
- 待消费队列容量为 200，单次读取最多 100 条。队列满时停止连接并显示错误，由使用者读取或清空事件后显式启动。
- 消息事件按实际远端消息 ID 去重；自动回复已认领的最近 512 个消息标识持久化。自动回复与手动消费操作共享事件队列。
- QQ 的 `RECONNECT`、`INVALID_SESSION` 按协议处理。其他网络错误、心跳超时和协议错误保留在状态页；用户可显式启动或重启。
- 密钥只保存在插件配置中，状态和仪表盘不会返回 AppSecret。
- 插件引擎释放时，宿主会话会关闭其 WebSocket。所有平台共用同一插件实现，宿主负责具体网络连接。

## 开发检查

在仓库根目录执行：

```powershell
node plugins/packages/buildin/workflow/node_modules/typescript/bin/tsc --noEmit -p plugins/packages/external/qqbot/tsconfig.json
node --test plugins/packages/external/qqbot/tests/*.test.mjs
```

插件由仓库的 `plugins/tools/sync_plugin_packages.py` 统一发现、编译和打包。网络接口参见 `plugins/docs/WEBSOCKET.md`。

正式 CLI 的真实模型、依赖调用和表情资源验证：

```powershell
.venv/Scripts/python.exe tools/tests/waifu_cli_smoke.py --chat <chat-id>
```

测试使用当前身份、模型及 Waifu 配置，不会向 QQ 发消息。QQBot 真实收发验证需要用户在 QQ 中发出新消息，查看状态及日志确认逐条交付。

开发用隔离回归脚本 `tools/tests/qqbot_cli_smoke.py` 需要同时提供 `--package` 和 `--waifu-package`。它只用于本地协议回归，不负责部署用户的正常配置。

协议字段核对来源：腾讯 `tencent-connect/botgo` 的 token 与消息定义，以及 QQ Bot 官方文档的 C2C/群文件上传与富媒体消息接口。QQ 直接上传本地图片限制为 5 MiB。
