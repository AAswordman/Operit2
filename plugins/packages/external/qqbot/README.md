# QQ Bot

将 `assistance/examples/qqbot` 迁入 Operit2，使用 ToolPkg API `2.0.0`。包 ID 保持为 `com.operit.qqbot_bundle`，作为可选官方插件放在 `plugins/packages/external/qqbot`。

Gateway 直接使用 `Tools.Net.webSocket`；HTTP 和附件下载分别使用 `Tools.Net.http`、`Tools.Files`。配置页使用 Compose DSL。Python Gateway、终端会话、本地控制服务器、端口和 shell 命令均已移除。

插件声明 `installation_scope: "device"`，只允许安装在“仅本设备”，不能移入共享空间。AppID、AppSecret、监听开关、自动回复配置和会话绑定保存在该设备的插件配置中，不随空间扩展同步。由选定的一台设备运行同一个 Bot 的监听和自动回复；Waifu 依赖可独立选择安装范围。

## 使用

1. 在 QQ Bot 管理端配置应用及 C2C、群消息事件权限，准备 AppID 和 AppSecret。在应用中配置聊天模型即可使用文本自动回复；Waifu 模式和角色卡按需安装。
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
| `qqbot_upload_image` | 兼容图片上传，返回 `file_info`，不发送消息 |
| `qqbot_upload_media` | 上传图片、视频、语音或文件，返回 `file_info`，不发送消息 |
| `qqbot_send_media` | 上传并发送图片、视频、语音或文件，支持正文、被动回复和原生引用 |
| `qqbot_waifu_preview` | 在指定会话中调用真实 Waifu 依赖，返回分段及时间，不向 QQ 发送 |

发送回复时传入原始 `msg_id`。服务按场景、目标和原消息维护发送序号，也接受显式 `msg_seq`。富消息既可使用 `payload` JSON 文本，也可显式提供 `markdown`、`keyboard`、`card`、`media`、`message_reference` JSON 文本；显式参数覆盖 `payload` 中的同名字段。省略 `msg_type` 时，由媒体、Markdown 或图文卡片字段推断为 7、2 或 8；普通文本为 0。互相冲突的内容字段会在联网前报错。

## 生命周期与边界

- 设置页和工具通过包内 IPC 调用同一个 main runtime；Gateway、队列和自动回复只有一个所有者。
- 自动回复可选使用 `com.operit.waifu >= 1.0.0` 分句并匹配表情，未安装或未启用时只发送原生 Chat 的最终文本。Waifu 图片通过 `/v2/users/{openid}/files` 或 `/v2/groups/{openid}/files` 上传后以 `msg_type: 7` 发送。所有消息段共用原始 `msg_id` 和单调递增的序号。失败后不会换用另一个模式重跑同一轮聊天。
- `waifu_profile` 指定依赖插件中的命名配置（初始为 `default`），打字节奏、标点、表情和自拍规则统一在 Waifu 模式页管理。取消、被拦截、被消费和超时均明确报错，已经交付的消息段保留。
- 图片、视频、语音、文件及嵌套引用中的附件通过宿主文件接口下载，作为结构化 Chat 附件提交；语音优先使用 QQ 提供的 WAV 地址，并将 ASR 参考文本交给模型。嵌套消息最多 6 层、50 个元素；合计最多 10 个附件。自动回复会跳过超过 20 MiB 的附件，继续处理正文和其余附件。文件保存在当前插件配置目录的 `incoming` 子目录，保留聊天附件引用。
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

协议字段核对来源：腾讯 `tencent-connect/botgo` 的 token 与消息定义，以及 QQ Bot 官方文档的 C2C/群文件上传与富媒体消息接口。本地媒体二进制直传限制为 5 MiB，支持 HTTP(S) 地址上传；更大本地文件的分片上传尚未实现。媒体格式和场景权限由 QQ 校验，模型能否理解音视频取决于所选模型。

## 渠道公开 API

`manifest.public_api` 发布 `src/channel-api.ts`。调用方在 manifest 中声明本 Bot 为依赖后，只需将该文件复制到自己的 `src/dependencies/qqbot/api.ts`，即可本地引用。消息、配置、上下文、请求和返回类型均定义在此单文件内；不需要复制收件箱或 Bot 实现。

```ts
import {request, peek} from "./dependencies/qqbot/api";
const status = await request({action: "status"});
const {messages} = await peek(20, "实际会话 ID");
// 消费者持久化处理成功后，再逐条确认。
for (const message of messages) {
  // 先由调用方处理并持久化 message。
  // await request({action: "ack", key: message.key});
}
```

`request` 按 `action` 推导返回类型；未安装、停用、版本不符或调用失败时会拒绝 Promise。`reply` 和 `send` 的上游响应保持 `unknown`，不对平台未承诺的字段作类型保证。

## 多模态与消息属性（1.2.0）

事件摘要和渠道收件箱现在提供 `author`（昵称、头像、机器人标记）、`messageType`、`attachments`（类型、MIME、大小、宽高、WAV 地址和语音转写）、`card`、递归 `elements`，以及本条消息的 `referenceIndex` 和用户引用对象的 `referencedMessageIndex`。这些属性不依赖 `include_raw`。原始事件仍可按需读取。

`msg_id` 用于被动回复及去重；`message_reference.message_id` 使用 QQ 的引用索引：收到消息时取 `message_scene.ext` 的 `msg_idx`（规范化为 `referenceIndex`），机器人自己发送的消息取响应 `ext_info.ref_idx`。不能用 Operit 内部消息 ID 替代。自动回复和渠道 `reply` 默认只提交 `msg_id`。调用 `reply` 时可用 `quote: true` 引用原消息，或显式传入 `message_reference`。渠道 `send` 使用已发现会话的 `conversationKey` 主动发送，受 QQ 的主动消息设置和频控限制。

统一发送出口会移除已知的内部引用封套（如 `[[reply_to:...]]`、`[CQ:reply,id=...]` 和 XML 引用属性）。流式回复跨分句缓存未闭合的引用标记，避免把 ID 的片段发到群里。模型输入使用结构化 JSON，区分消息 ID、发送者身份和正文，正文包含可读引用、卡片摘要和附件；鉴权令牌及原始事件不会进入模型输入。此过滤针对已知协议标记，不会任意删改普通正文中的数字、URL 或用户自行书写的标识。

渠道客户端保持原有文本调用兼容，并支持：

```ts
// 普通被动回复。
await request({ action: "reply", key: message.key, text: "收到。" });
// 引用原消息，需有 QQ 引用索引。
await request({ action: "reply", key: message.key, quote: true, text: "收到。" });
// 发送视频；audio、image、file 同样适用。
await request({ action: "reply", key: message.key,
  media: { kind: "video", source: "https://example.com/demo.mp4" }, text: "演示视频" });
// Markdown 和内嵌按钮使用 QQ 原生对象。
await request({ action: "reply", key: message.key,
  markdown: { content: "**处理完成**" }, keyboard: { id: "已获授权的按钮模板 ID" } });
```

确认收件箱消息后，延迟回复仅保留必要的路由信息及引用索引，不保留已确认消息的正文、附件、作者资料或原始事件。

## 群聊上下文与反馈订阅

- 默认同群共享一个持久聊天。会触发自动回复的新消息会打断该群正在生成的回复，并替换尚未开始的自动回复。不同群和私聊独立，最多同时处理 3 个会话。
- 设置页“按成员隔离群上下文”可为成员使用独立 AI 会话。只有同群同一成员的新消息会打断该成员的回复，上下文工具也按成员过滤。关闭隔离恢复原来的共享聊天。
- “记录群友普通消息”默认开启，近期历史最多保留 80 条，每条正文最多 6000 字；只记录平台实际下发的消息。群消息回复条件默认“仅被 @ 时回复”，可单独切换“回复所有收到的群消息”。监听普通消息是否可用取决于 QQ 对该机器人的事件权限，此开关无法取得额外权限。
- 被 @ 后 AI 可调用 `qqbot_chat_context` 按需读取所在会话的前文，默认 20 条、最多 50 条、总正文最多 12000 字。普通群消息不逐条提交给模型。工具不消费消息，也不能切到任意其他群。
- 待回复任务会持久保存，同一会话只保留最新一条。此前已发出的内容保留，入站消息可通过近期历史和订阅查看。发送失败或应用中断后，可在“诊断信息”核对已发送的内容，再选择重试或跳过。
- `channel.v1` 提供 `conversations`、`open_chat`、`history`、`subscribe` 和带 `consumerId` 的 `peek/ack`。反馈插件从真实 Bot 会话订阅消息，每个消费者独立确认。维护不改写 Bot 的全局过滤配置。复制 `src/channel-api.ts` 即可单文件引用全部类型和客户端。

这些规则已通过本地协议与宿主模拟测试；群事件权限、真实账号网络和模型行为需按实际环境验证。
