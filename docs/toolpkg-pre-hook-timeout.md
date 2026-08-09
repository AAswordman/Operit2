# ToolPkg 前置 Hook 总超时

ToolPkg 的 Chat Input、Prompt 与 Summary 前置 Hook 链各自使用一份总超时预算。默认值为 10 秒，可在“工具设置 → 高级设置”中、MCP 启动超时的下方调整，允许范围为 1 至 60 秒。

一次分发开始时会建立绝对截止时间。每个 Hook 会取得当时的剩余时长；到达截止时间后，正在等待的结果不会应用，后续 Hook 也不会启动。这避免多个 Hook 各自消耗完整超时而累计卡住一次用户操作。

聊天的 `submit_requested` 链超时时，当前已确认的文本仍会继续发送，并通过 `ChatToastHost` 显示具体的“前置插件「包名:HookID」响应超时，已跳过并继续发送”提示。发送前的 Prompt Input 链超时也通过同一条 Toast 事件流提示具体插件；Prompt History、Prompt Finalize 与 Summary 链只记录链路日志，以免在模型处理期间打断界面。

消息处理接管链不属于这项前置 Hook 门禁；它具有独立的回复接管语义。
