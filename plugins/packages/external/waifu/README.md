# Waifu 模式插件

独立的 ToolPkg API 2.0.0 插件，包 ID 为 `com.operit.waifu`。普通聊天可以直接使用 Waifu 模式，支持输入合并、流式分句、打字节奏、情绪表情图片、自定义提示词和自拍提示规则。QQBot 等插件也可以通过 `requires` 和公开客户端使用分句能力。

## 安装与配置

导入并启用 `waifu.toolpkg` 及其声明依赖的角色卡插件。在工具箱的“Waifu 模式”页面为当前角色开启模式，也可以在聊天输入菜单中切换。设置绑定到角色卡、角色组或模型对话选择，普通聊天会按会话实际绑定的角色读取设置。

设置页按 Operit1 的方式提供总开关、合并发送与等待时间、打字速度、句末标点、自定义提示词、表情管理和自拍外貌关键词。表情管理可预览内置图片、导入本地图片和移除图片。修改自动保存，从下一次回复起生效。

输入合并会等待短消息停止输入后再提交；连续提交的附件与引用保留原有信息并按顺序发送。通过插件 API 提交的消息不经过人工输入合并。分句交付使用宿主消息展示接口，每条已交付气泡和完成状态都会写入会话记录；重新进入聊天后保留相同的气泡。模型上下文仍使用完整的原始语义消息。

- 首个文本段立即交付，后续文本段按每字符延迟等待，单段最多 3 秒。等待和网络发送不会阻塞模型流的收集。
- 可移除句末标点，保留省略号、代码和公式。
- 内置旧版的 9 类、53 张表情图片。启用后，提示词要求模型按情绪输出 `<emotion>happy</emotion>` 等标签，接口返回独立图片段。
- `emojis` 保存完整的分类到图片来源映射。`resource:` 指定包内资源，HTTP(S) 地址和 Host 文件路径表示自定义图片。可新增分类或修改已有分类。
- 自拍开关添加外貌和绘图提示规则。需要填写外貌关键词，并安装、配置可用的绘图工具。模型产生的 Markdown 图片也会作为独立图片段交付。
- 每次回复固定配置快照。修改配置从下一次回复起生效。

普通聊天模式 CLI 工具：

```text
waifu:waifu_mode_get {}
waifu:waifu_mode_set {"selection":"card:<角色 ID>","mode_json":"完整 Mode JSON"}
```

`Mode` 包含 `enabled`、`mergeSend`、`mergeDelayMs` 和 `settings`。`waifu_mode_get` 返回当前角色选择和完整配置。

外部插件配置通过命名 profile 独立管理，默认名称为 `default`；消费插件明确选择 profile。CLI 工具：

```text
waifu:waifu_settings_get {"profile":"default"}
waifu:waifu_settings_set {"profile":"default","settings_json":"完整 Settings JSON"}
```

## 作为插件依赖调用

在消费插件的 manifest 中声明：

```json
{
  "requires": [{ "id": "com.operit.waifu", "min_version": "1.0.0" }]
}
```

把公开文件 `src/api.ts` 原样复制到消费插件的 `src/dependencies/waifu/api.ts`：

```ts
import { stream } from "./dependencies/waifu/api.js";

/** Delivers native AI output through the declared waifu dependency. */
async function reply(chatId: string, text: string): Promise<void> {
  for await (const event of stream({
    kind: "submit", chatId, runtime: "main",
    input: { text, attachments: [], replyToMessageTimestamp: null },
    turn: { kind: "execute" }, notifyReply: false,
  }, "default")) {
    if (event.type === "segment") {
      // Send text or upload the actual image using the consumer's transport.
      console.log(event.index, event.segment);
    }
  }
}
```

事件协议：

```ts
{ type: "segment", index: 0, segment: { kind: "text", text: "你好。" } }
{ type: "segment", index: 1, segment: { kind: "image", source: "/app/data/cache/...", emotion: "happy" } }
{ type: "completed", result: /* originating native Chat receipt */ }
```

`source` 是 Host 可读的文件路径或图片地址。消费插件负责将图片交付到目标平台。不要把路径当作聊天文本发送。`completed.result.aiResponse` 是原始最终回执，已经交付的句子无需再次发送。

## 调用边界

公开客户端保留调用插件的原生 Chat 执行所有权。模型调用和取消使用该调用方的 `Tools.Chat`；Waifu main runtime 持有设置、分句会话和表情解析，通过 `session.open` / `session.feed` / `session.close` 公共方法接收 JSON。单次 IPC 只处理一个快照，不会挂起整个模型请求，公开调用的 60 秒期限不会限制整个回复时长。

提示钩子作用于已启用普通 Waifu 模式的会话，以及外部消费者明确开启的分句会话。外部消费者持有会话时，由该消费者交付分句，普通聊天展示接口不会重复接管。思考、工具、状态语义部分不会交付为句子。已发送内容遭到改写、未知情绪、图片读取失败、队列超限或模型未成功完成都会明确报错。提前退出迭代会取消实际模型请求、关闭观察器并释放插件会话。最多允许 64 个并发会话，每个会话最多 64 条助手消息；单快照限制 2 MiB，待交付队列限制 256 段。

## 检查

仓库根目录：

```powershell
node plugins/packages/buildin/workflow/node_modules/typescript/bin/tsc --noEmit -p plugins/packages/external/waifu/tsconfig.json
node --test plugins/packages/external/waifu/tests/*.test.mjs plugins/packages/external/qqbot/tests/*.test.mjs
```

正式 CLI 的真实模型与依赖验证：

```powershell
.venv/Scripts/python.exe tools/tests/waifu_cli_smoke.py --chat <chat-id>
```

脚本使用已安装 CLI 的当前身份和持久化设置，调用 QQBot 的真实公开客户端；不创建临时身份或改写模型配置，也不向 QQ 发消息。结果包含文本、图片和交付时间。
