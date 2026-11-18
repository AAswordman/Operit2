# 微信 Bot

Operit 官方可选插件，包 ID `com.operit.wechat_bot`，ToolPkg API `2.0.0`。

通过宿主原生 HTTP 直接连接微信 Bot 后端。扫码登录、长轮询、文本及加密媒体收发均在插件 main runtime 内运行，不需要 OpenClaw、Python、终端进程、本地端口或中转服务器。

## 使用

1. 安装并启用本插件。使用「外部连接器」统一管理时，也安装并启用连接器；无需安装维护中控、角色卡或 Waifu。
2. 在「外部连接器」获取微信登录二维码，扫码确认后会自动开始监听。手机要求验证码时，在同一面板提交。也可使用原「微信 Bot 设置」扫码并启动。
3. 用微信发第一条消息，控制台会自动发现会话。默认只接收消息；需要 AI 自动回复时，配置聊天模型并启用自动回复。默认 `native` 模式发送原生模型文本；可选角色卡或安装 Waifu 并选择其配置以启用分句和情绪图。
4. 每位微信用户对应独立的原生聊天；应用重启会恢复显式启用的监听。「停止监听」会取消正在处理的回复；原设置页「退出登录」会清除本机账号凭据和待处理消息，保留软件聊天。

二维码使用包内生成器生成 SVG，由原生 Compose `Svg` 显示，不依赖外部二维码服务或 WebView。登录过期、验证码受限、路由变化及接口错误均显示实际状态。

## 消息与媒体

- 支持微信 Bot 私聊文本、图片、视频、文件和语音消息；语音中的服务端转写文本会提交给模型。媒体按实际 MIME 类型作为原生 Chat 附件提交，不假造识别结果，也不宣称转换不支持的音频编码。
- 收到的 SILK 语音会在本地转为 24 kHz 单声道 WAV，再交给 Chat；转换限制为原件 2 MiB、音频 2 分钟。无法转换时仍保留原始附件，控制台下载始终提供原文件。实际识别能力取决于当前模型是否支持音频。
- 出站图片、视频、文件和语音经过真实文件读取、二进制 MD5、AES-128-ECB / PKCS#7 加密和 CDN 上传，再提交 `sendmessage`。每个文件最多 25 MiB；语音需明确提供原文件编码、采样率及毫秒时长，不进行隐式转码。
- 媒体来源接受宿主文件路径和 HTTPS URL。文件消息可指定显示名称，说明文字独立发送。上传失败不会继续提交媒体消息，也不会自动换用其他传输方式。
- 入站媒体支持协议规定的原始 16 字节和十六进制文本两种 base64 密钥表示，以及图片的显式十六进制 `aeskey`。解密时校验完整 PKCS#7 填充；文件保存在插件配置目录的 `incoming` 中，以保留聊天附件引用。
- 发送使用具体用户的 `context_token`。手动工具可明确传入令牌，也可使用该用户最近入站消息所保存的上下文；没有上下文时明确报错。
- 单条文本最多 4000 个 Unicode 字符。AI 产生的较长文本按字符边界分段发送，不拆开代理对，不把最终执行回执重复发送为消息。
- 引用会提取可用的引用文字和媒体。仅有服务端 ID 时只在相同用户历史中查找，找不到则省略，不把 ID 拼进正文。
- `wechat_set_typing` 使用实际 `typing_ticket` 设置或取消正在输入状态。

## 交付与生命周期

- 设置页和工具通过包内 IPC 访问唯一 main runtime。每个设备只运行一个监听。自动回复最多并行处理三个用户。同一用户发来新消息时会打断当前回复，多条待回复消息只处理最新一条。
- `installation_scope: "device"`。登录凭据、队列、游标、回复配置和会话绑定只保存在本设备，不同步到共享空间。会话绑定包含 Bot ID，切换账号不混用聊天记录。
- 持久化保留有效入站消息、去重标识和回复上下文。遗漏游标不会清空旧值；单条畸形或不支持的消息不会卡住同批其他消息。启用自动回复时先合并同一用户的待回复消息。不同用户的待回复任务超过容量时保留服务器游标。
- 收到成功的 `getupdates` 响应才显示已连接。网络与可恢复的业务错误使用退避重连；账号过期提示重新扫码。出站失败不自动重发。
- 停止和退出会先阻止新的回复、取消实际原生 Chat 请求，然后等待已经发出的 HTTP 请求结束。当前网络接口没有单次请求取消句柄，因此停止可能等待本次长轮询到期，不会另开第二个监听绕过它。
- 每条自动回复在认领后、开始处理前保存交付记录，每段收到发送成功后保存已交付数量。失败、部分交付或应用中断后，保留最多 200 条失败记录供核对，新消息可以继续触发回复。设置页可逐条选择「重试整个请求」或「跳过」。重试可能重复已交付的内容，需先核对。被新消息打断的回复停止发送后续内容，已发送的内容仍可在会话历史查看。
- 设置页状态刷新不覆盖未保存的输入；关闭页面会释放状态刷新定时器。保存回复设置不会启动已停止的监听，也不会打断仍启用的当前回复。
- 应用需保持运行才能接收消息。当前实现不注册普通微信群功能，也不读取个人微信的好友列表或全部聊天记录。

## 工具

| 工具 | 用途 |
| --- | --- |
| `usage_advice` | 查看接入和回复说明 |
| `wechat_status` | 登录、连接、队列、会话及交付状态，不返回 Bot token |
| `wechat_login` / `wechat_login_verify` / `wechat_login_cancel` | 获取二维码、提交验证码、取消本次扫码 |
| `wechat_logout` | 退出登录，删除本设备凭据及待处理消息 |
| `wechat_test_connection` | 验证当前账号凭据 |
| `wechat_service_start` / `wechat_service_stop` | 启停监听；启动可显式 `restart` |
| `wechat_receive_events` / `wechat_clear_events` | 预览、消费或清空队列 |
| `wechat_send_message` / `wechat_send_media` | 发送文本或加密媒体 |
| `wechat_download_media` | 下载指定已接收消息中的全部媒体 |
| `wechat_set_typing` | 设置或取消正在输入提示 |
| `wechat_auto_reply_configure` | 保存角色、Waifu 配置、超时及回复要求 |
| `wechat_auto_reply_start` / `wechat_auto_reply_stop` | 开关自动回复，不启动已停止的监听 |
| `wechat_auto_reply_run_once` | 监听就绪且持续自动回复关闭时处理一条消息 |
| `wechat_delivery_resolve` | 显式重试或跳过失败交付 |
| `wechat_reset_conversation` | 解除当前 Bot 对某位用户的会话绑定，旧聊天保留 |

## 宿主能力

媒体使用本仓库新增的通用能力，而不是插件私有的平台实现：

- `CryptoJS.binary.randomHex`：安全随机字节。
- `CryptoJS.binary.md5`：对 base64 表示的原始字节计算 MD5。
- `CryptoJS.binary.encryptAesEcb` / `decryptAesEcb`：二进制 AES-128-ECB / PKCS#7；输入、输出均为 base64，密钥为 32 位十六进制。
- `OkHttp` 请求体类型 `base64`：由标准 HTTP 工具解码成真实字节，保留显式 `Content-Type`，不会把 base64 文本上传为文件。

本插件需要包含这些更改的 Operit 宿主；仅把插件导入旧宿主不会添加原生能力。所有平台共用同一插件和 Rust 能力，没有 Android / Windows / Web 专用分支。

## 开发检查

在仓库根目录执行：

```powershell
node plugins/packages/buildin/workflow/node_modules/typescript/bin/tsc --noEmit -p plugins/packages/external/wechat/tsconfig.json
node --test plugins/packages/external/wechat/tests/*.test.mjs plugins/packages/external/qqbot/tests/*.test.mjs plugins/packages/external/waifu/tests/*.test.mjs
node plugins/packages/external/wechat/tests/live-smoke.mjs
```

`live-smoke.mjs` 只访问真实的未登录二维码接口，不输出二维码内容，不登录账号，不发送消息。其他 Node 测试使用确定性网络、文件、计时器与 Chat 边界，并调用真实的插件代码、Waifu 公开客户端和服务实现。

在 `core` 目录执行原生检查：

```powershell
cargo check -p operit-js-bridge --tests --offline --locked
cargo test -p operit-js-bridge JsBinaryCrypto --lib --offline --locked
cargo test -p operit-js-bridge wechat --lib --offline --locked
cargo test -p operit-tools binary_body_tests --lib --offline --locked
```

插件由 `plugins/tools/sync_plugin_packages.py` 的标准目录发现、TypeScript 编译和 ToolPkg 打包链路处理，与 QQ Bot 同样作为可选插件交付。真实账号扫码、设备后台运行和微信端媒体展示需要实际设备验证，协议及模拟测试不替代这些验证。

## 协议与依赖来源

- 腾讯微信渠道源码与协议：<https://github.com/Tencent/openclaw-weixin>、<https://github.com/Tencent/openclaw-weixin/blob/main/docs/protocol_zh_CN.md>。
- 包内二维码生成器：`qrcode-generator` 2.0.4，Copyright (c) Kazuhiko Arase，MIT。保留原始文件及声明中的授权说明，完整授权见 `THIRD_PARTY_LICENSES.md`。

## 渠道公开 API

`manifest.public_api` 发布 `src/channel-api.ts`。调用方在 manifest 中声明本 Bot 为依赖后，只需将该文件复制到自己的 `src/dependencies/wechat/api.ts`，即可本地引用。消息、配置、上下文、请求和返回类型均定义在此单文件内；不需要复制收件箱或 Bot 实现。

```ts
import {request, peek} from "./dependencies/wechat/api";
const status = await request({action: "status"});
const {messages} = await peek(20, "实际会话 ID");
// 消费者持久化处理成功后，再逐条确认。
for (const message of messages) {
  // 先由调用方处理并持久化 message。
  // await request({action: "ack", key: message.key});
}
```

`request` 按 `action` 推导返回类型；未安装、停用、版本不符或调用失败时会拒绝 Promise。`reply` 的上游响应保持 `unknown`，不对平台未承诺的字段作类型保证。

## 会话与维护订阅

`channel.v1` 提供 Bot 自己的私聊会话列表、软件聊天绑定、有界近期历史，以及独立消费者订阅。维护中控自动读取这些会话，直接使用其宿主聊天的工作区，无需再填写微信会话 ID 或父聊天。关闭自动回复时，普通事件队列采用最近 200 条滚动记录；已订阅的维护消息独立持久保存。开启自动回复后仍保留队列容量检查，满载时保留服务器游标，避免无声丢失待回复消息。当前微信通道只支持所接接口实际提供的私聊，不宣称支持普通微信群。
