# 模型首字等待与重试

## 对应的用户故障

`operit-diagnostics-1795164758095.log.txt` 的第 12507 行记录了
16:29:12.813 向本机 `127.0.0.1:8787` 发起 OpenAI-compatible 请求。
直到第 16708 行的 16:41:54.077，该请求才因用户取消结束，耗时
761.264 秒；期间没有该请求的响应头和首字节记录。

原有读取空闲超时为 1,000 秒。仅在收到响应头之后给 SSE 读取加计时，
无法覆盖这次故障。现在共享 Provider 计时包围整个 HTTP 尝试，从调用
`sendHttpRequest` 之前开始，覆盖上传、等待响应头和等待首个有效模型输出。
因此这次日志中的等待会在单次尝试 120 秒时被中止并进入现有重试流程。
日志没有本机代理内部记录，不能据此确定代理或上游阻塞的具体原因，也不能
保证重试一定恢复。

## 行为契约

- OpenAI（含 Responses 和复用它的厂商适配器）、Claude、Gemini 的
  内置 HTTP 生成入口使用同一个 `FirstOutputDeadline`。
- 每次 Provider 尝试的首字等待上限为 **120 秒**。收到响应头、网络字节、
  SSE 心跳、角色声明、空内容、usage 或隐藏签名，都不会解除这一限制。
- Provider 的现有解析器确认非空文本、思考内容、工具调用名称或参数片段、
  图片数据等模型输出后，解除该次首字计时。之后继续沿用现有读取空闲超时
  和主动取消机制，长推理不会因总时长超过 120 秒被此计时终止。
- 非流式接口需要完整正文才能解码，计时覆盖到完整 JSON 读取和解码完成，
  中途收到零碎正文不能解除限制。
- 到期返回明确的 `AiServiceError::FirstOutputTimeout`。丢弃原 HTTP future
  会沿用 `ResponseBody::Drop` 调用拥有该请求的 Host 的
  `closeHttpByteStream`，随后才发布重试提示并等待退避。
- 重试提示为 `AI 首字等待超过 120 秒，正在进行第 N 次重试...`。
  重试次数和开关沿用 `SendMessageRequest.enable_retry` 与
  `LlmRetryPolicy`：最多 5 次重试，退避依次为 1、2、4、8、16 秒。
  禁用重试或耗尽次数时，现有响应终态通道报告失败；用户取消保持独立语义。
- 120 秒是每次尝试的上限。若 6 次尝试全部没有输出，仅超时与退避累计
  约 751 秒，另有请求构建和调度开销。此次没有改变整轮对话的累计时间策略。

计时使用现有 `HostRuntimeTaskSchedulerHost::waitForHostRuntimeDelay`，
网络打开、读取和关闭使用既有 HTTP Host。共享 Provider 没有新增平台分支，
也没有更换模型、修改响应内容或另建重试循环。工具执行与长期 Goal 状态订阅
继续由其自身生命周期管理。

## 验证

`core/crates/provider/services/tests/first_output_timeout.rs` 通过可手动推进的
Host 延迟驱动真实 Provider，串行覆盖三个协议的 30 个场景，不实际等待
120 秒：

- 流式及非流式请求都分别覆盖无响应头和有响应头但无有效输出的停滞。
  心跳、空 JSON、角色、usage、空思考和签名元数据不能解除首字计时。
- 超时时确认旧请求先关闭，提示精确匹配，退避完成后新请求能正常返回。
- 文本、思考、工具输出解除计时后，仍可主动取消并释放请求。
- 禁用重试、6 次尝试耗尽、响应头到达前取消，都按原有终态约定结束。

`FirstOutputDeadline` 单元测试还覆盖首次输出与超时在同一轮轮询就绪的
边界，以及 Host 计时失败的独立错误。既有 HTTP、SSE、取消、usage、
API Key 轮换和 OpenCode 请求头回归继续运行。

```powershell
$env:RUSTFLAGS='-Awarnings'
cargo test --locked --manifest-path core/Cargo.toml -p operit-providers --lib chat::llmprovider:: -- --test-threads=1
cargo test --locked --manifest-path core/Cargo.toml -p operit-providers --test first_output_timeout --test api_key_pool_cached_requests --test opencode_request_headers --test opencode_model_catalog
```

本次不新增或修改工作流。受控 Host 验证不能替代 Android 上原始代理的端到端复测。

2026-11-20 本地检查：159 项相关 Provider 单元测试通过，4 项集成测试
通过（其中首字超时用例包含上述 30 个场景），1 项访问真实网络的集成测试
按原标记忽略。完整 Provider 库回归为 243 项通过、1 项忽略、1 项失败：
`EnhancedAIService::workspace_path_tests::workspace_prompt_paths_remain_virtual`
对 `/app` 与 `/operitapp` 工作区别名的去重断言失败，独立运行也复现。
该函数及其 VFS 路径处理源码与工作区 HEAD 一致，本次未修改这一链路。
