# AI 主动消息

可选的设备本地插件，默认关闭。在工具箱的“AI 主动消息”中为对话设置固定间隔、可选随机追加间隔、每次触发概率、提前准备次数和提示词。

- 每次间隔为固定分钟数加上独立抽取的 `0～随机追加分钟数`。
- 每次检查独立抽取触发概率；0% 不调用 AI，100% 每次都准备。
- 支持最多 8 个对话，每个对话一条规则，每条提前准备 1～8 次，所有对话最多 32 条待处理计划。
- 默认新规则关闭，间隔 60 分钟、随机追加 0 分钟、概率 50%、提前准备 4 次。

## 插件与宿主的分工

插件在前台计算时间、抽取概率，通过 `Tools.Chat.scheduleMessage` 提交生成指令。宿主复用正式聊天的输入处理、角色配置、提示词、模型请求与用量统计流程准备消息，随后持久化预约。插件不自行拼接历史、不直接调用功能模型，没有平台分支或发送定时器。

准备使用目标对话实际绑定的角色、模型、记忆输入处理和摘要后的正式历史，保留角色头像、发言者身份、模型、token 与生成耗时。准备消耗正常模型调用额度，未命中概率不调用模型。生成过程不写入用户发言、不切换当前对话，不执行工具。

当前规则未提供多人群聊的发言者选择；需要明确参与者的对话会停止准备并显示错误。角色配置解析失败同样停止准备，避免发送其他身份的内容。

应用前台启动、恢复、打开设置页和分钟时钟会补充计划。宿主定期及投递前检查对话和角色配置；变化后撤销旧计划并返回 `invalidated`，插件在前台按原预约时间重新准备，已过期的计划跳过。准备过程中对话变化会拒绝该结果。升级时先撤销旧版纯文本预约。后台无法调用模型时，需要回到前台补充。

## 统一正式接口

所有平台使用同一套 SDK 接口、参数和结果。TypeScript 声明由 Rust SDK 定义生成，经统一工具注册、参数校验和插件身份认证。

```ts
await Tools.Chat.scheduleMessage({
  id: "unique-message-id",
  chatId,
  title: "Operit",
  text: "",
  generation: { instruction: "根据最近对话自然地延续话题，只输出主动消息正文。" },
  timestamp: dueAt, // Unix 毫秒，未来 1 秒至 30 天
});

const { messages } = await Tools.Chat.listScheduledMessages();
await Tools.Chat.cancelScheduledMessage("unique-message-id");
```

`scheduleMessage` 和 `listScheduledMessages` 返回的消息包含 `id, chatId, timestamp, status, error, text, notificationAttempted, notificationRegistered, notificationError`，`text` 是宿主实际准备的正文。传入 `generation` 时要求 `text` 为空；已有纯文本调用仍使用非空 `text`。`listScheduledMessages` 只列出调用插件自己的预约及结果。终态包括 `delivered`、`cancelled`、`skipped`、`invalidated` 和 `failed`，`error` 为字符串或 `null`。`cancelScheduledMessage` 返回 `{ id, cancelled: true }`，重复撤销安全。

`scheduled` 表示正文准备和预约持久化成功；`delivered` 只表示聊天消息已入库，提交入口同时向已打开的主界面、悬浮窗及独立窗口消息流发布同一数据库记录。去重恢复也重新发布，不重复写入。界面收到更新会记录 `ChatFlowTrace chat.messages_applied`；这表示消息列表状态已应用，不等于已经进入可见区域，当前没有 UI 可见性回执。

通知结果独立记录：`notificationAttempted` 表示已发起通知请求，`notificationRegistered` 表示 Host 接受请求，不证明系统展示；`notificationError` 保存通知失败，消息仍可为 `delivered`。旧 Android 记录没有写入这两个布尔字段，其 `false` 不能作为未尝试通知的证据。未收到 Host 确认的请求不自动重发。插件记录页分别显示写入结果与通知结果。

ID 自动按已认证的插件身份隔离。相同 ID 和相同参数可安全重试，即使调用响应丢失或预约已到期；相同 ID 与不同内容冲突时拒绝。需要新计划时使用新 ID。宿主保留最多 60 条待处理消息和最近 256 条结果。

宿主还提供跨平台的 `Tools.Chat.deliverPrepared({ chatId, text, timestamp, notify })`，供需要立即提交已生成 AI 文本的其他插件使用。本插件的发送完全使用预约接口。即时提交以对话 ID 和毫秒时间戳去重，返回通知错误与消息写入结果。

## 宿主内部的平台适配

Android、HarmonyOS、Windows、macOS、Linux、Web 使用宿主调度器，在可执行时将内容写入指定对话，并通过已有的平台通知实现展示通知。对话忙时宿主自动延后；恢复后同一对话的多条过期消息仅投递最近一条，超过一天的旧消息跳过。

iOS 在预约时将已生成内容提交系统本地通知，到点由系统展示。宿主在应用恢复前台后检查上下文并同步有效消息，不再次通知。应用挂起期间不能更新或撤回已经展示的系统通知，恢复后检测到上下文变化的正文不会再写入对话。iOS 的通知权限交互和实际展示仍受系统设置约束。

所有平台的预约均保存在宿主设备本地存储中，宿主重启后自动恢复。应用退出、设备休眠、浏览器后台节流等会影响非 iOS 的实际投递时间；iOS 的系统通知不会唤醒插件调用 AI。接口不承诺绕过操作系统的执行限制。

恢复任务由应用初始化统一启动，先预约插件目录加载，再等待目录就绪，随后检查对话与角色配置。准备中断、对话忙、上下文或参与者改变会保留各自原因；它们不等于数据库投递失败。对话忙时不准备新正文，已经接受的预约在忙状态结束后继续核验。

关闭或删除规则会通过统一接口撤销待发消息。停用或卸载插件前，请先在设置页关闭规则：预约由宿主持有，插件停止运行本身不会撤销已经提交的计划。更改规则会先撤销旧计划，再重新抽取和准备。

## 持久化与错误处理

插件的规则和准备记录通过 `Tools.Storage.dataStore` 保存。宿主的已接受预约单独保存在设备本地 `runtime/scheduled_chat_messages.json` 中，不要求插件运行时保持存活。插件不跨设备同步发送状态。

`DataStore.data().get("state")` 返回深度只读快照。插件校验后复制为自己持有的可修改运行状态，再通过 `DataStore.edit` 显式保存。0.1.3 修复了旧版直接修改只读快照导致的重启、重新打开设置、旧数据升级和计划恢复报错；保留原有规则和预约 ID，无需清除插件数据。

插件写入成功后才提交预约；宿主先原子保存预约身份，再调用系统。iOS 预约结果未知时保留身份，宿主在前台使用相同 ID 重试。撤销意图先持久化，失败可恢复。聊天文本提交使用真实数据库的完成消息，重试不会重复写入；通知失败在结果中单独记录。

## 构建与测试

该插件需要包含这些正式 SDK 和宿主实现的应用版本，不能单独安装到旧应用中。由仓库既有的 `plugins/tools/sync_plugin_packages.py` 编译和打包，无额外前端依赖。

```sh
node plugins/packages/buildin/workflow/node_modules/typescript/bin/tsc --project plugins/packages/external/proactive_messages/tsconfig.json --noEmit
node --test plugins/packages/external/proactive_messages/tests/*.test.mjs
node --test core/crates/plugin/javascript-bridge/tests/storage-preferences.test.mjs
node --test core/crates/plugin/javascript-bridge/tests/chat-prepared-delivery.test.mjs
cargo test --manifest-path core/Cargo.toml -p operit-plugin-sdk prepared_delivery_tests
cargo test --manifest-path core/Cargo.toml -p operit-runtime --test prepared_chat_delivery --features operit-host-native-scheduler/javascript
cargo check --manifest-path core/Cargo.toml -p operit-runtime --target aarch64-apple-ios
cargo check --manifest-path core/Cargo.toml -p operit-runtime --target wasm32-unknown-unknown
```

插件测试直接读取当前 TypeScript 源码，在内存中转译执行，无需预先生成 `dist`。存储使用生产 SDK 的 DataStore facade，保留深度只读快照和显式编辑语义；仅模拟宿主持久化和聊天预约。设置页测试运行实际 Compose 渲染、动作分发和 `withContext`，覆盖重启后的页面加载、编辑保存、删除及预约撤销。其他用例验证概率与间隔、存储失败、预约响应丢失、前后台切换、撤销、升级、重启和计划上限。

SDK 测试验证实际生成绑定和错误传播；原生集成测试验证真实数据库、去重、预约持久化、隔离、恢复投递和撤销。iOS 实际通知展示、权限交互与点击跳转需真机验证。
