# 模型错误回调审计（2026-11-21）

本次核对范围是 `core` 中所有生产 `SendMessageRequest` 构造点，以及聊天总结、自动继续、标题生成、定时回复和工具直接调用模型的错误出口。测试夹具中的空回调不代表产品调用行为。

| 入口 | 错误消费方式与处理 |
| --- | --- |
| 普通聊天首次请求 | `EnhancedAIService::provider_error_callback` 同时转发函数回调和 `SendMessageCallbacks` |
| 工具结果后的模型请求 | 与首次请求共用同一回调桥接，修复只读取旧函数回调的遗漏 |
| 聊天最终失败 | `MessageProcessingDelegate::finishChatExecution` 失败分支发送原始错误提示 |
| 自动总结、手动总结 | 总结 API 必传错误回调；`launchSummary` 发布前检查任务身份；最终失败使用 `reportSummaryFailure` |
| 手动插入总结 | 明确传入当前 Core 的提示流；最终失败使用 `summaryInsertionFailed` |
| 总结完成后的自动继续 | 重新进入 `sendMessageInternal`，安装普通聊天回调，后续工具轮次共用同一错误桥接 |
| 标题生成 | 必传错误回调，后台最终失败发送标题生成错误；收完流后验证终态 |
| 工具直接调用模型 | 请求回调发送提示，发送失败和流最终失败同时返回给调用工具；收完流后验证终态 |
| 定时回复生成 | 为 `SendMessageOptions.callbacks` 装入实际提示实现；最终失败仍通过原有准备结果返回并保存失败状态 |
| 模型连接测试、单个 API Key 测试 | 请求字面量中仍有 `None`，但 `executeTestRequest` 在唯一实际发送点统一替换为真实错误收集器；错误进入测试报告；这些请求禁用自动重试 |
| `SendMessageOptions::new` | 可选字段默认值，不是请求发送点；普通聊天和定时回复的实际发送入口都安装真实回调 |

总结及标题生成方法现在要求显式传入 `Arc<dyn Fn(String) + Send + Sync>`，内部再写入 Provider 请求，防止调用方省略错误消费者。所有仓库内调用点均同步调整；Rust 外部调用方更新这些方法时也需要提供实际错误消费者。

## 空实现清理

本次列出的历史空实现已删除或改为必须履行的接口：

- 删除 `EnhancedAIService` 的 `startAiService`、`stopAiService`、`notifyReplyCompleted`、全部调用及对应的虚假生命周期阶段。聊天执行状态和完成通知继续由 runtime 的真实实现负责。
- `SendMessageCallbacks` 只保留实际发布的 `onNonFatalError` 和 `onTokenLimitExceeded`，两者都必须实现。删除无人发布的 `onToolInvocation` 和 `onInputProcessingStateChanged` trait 方法；真实工具调用事件继续使用现有独立回调。普通聊天将 Token 超限记录到压缩控制标志，定时回复将其发布到提示流。
- `finalizeAssistantResponse` 只接收实际使用的回复正文并保存，删除所有未使用的错误、执行上下文、子任务和通知参数。Provider 重试通过实际请求回调，最终错误通过流终态进入 runtime。
- 删除只传向空通知实现的 `SendMessageOptions.avatarUri`、`SendMessageOptions.notifyReplyOverride` 和 `AIMessageManager::SendMessageRequest.notifyReplyOverride`。通知控制仍由 runtime 的 `ChatTurnOptions.notifyReply` 和路由继续上下文持有；身份头像仍由执行配置持有。
- `ToolRuntimeSupport::closeDeviceTransports` 改为必实现方法，生产 runtime 调用设备传输清理。两个不使用设备传输的测试替身明确拒绝意外调用。
- 删除完全未使用的 `AIForegroundService` 模块及其空通知函数、`MessageCoordinationDelegate::setUiBridge` 空函数，以及 OpenAI 请求组装中无实际处理的 `customize_final_request_object` 函数和调用。OpenAI Responses 的同名方法有实际协议处理，继续保留。

以上删除涉及 Rust 接口调整；仓库内调用方已同步清理，外部 Rust 消费方需要移除这些旧接口的调用，并实现现在必需的回调与传输清理方法。

## 检查边界

本次对修改文件执行 Rust 语法解析和 Git 差异空白检查，并核对所有受影响方法的仓库调用点，以上检查通过。新增回调行为测试覆盖无旧函数回调时的错误传递，以及两种消费者并存时原始错误各传递一次。

执行 `node --test tools/tests/tool_result_continuation.test.mjs`，7 项检查中 6 项通过。剩余检查仍断言展示使用 `toolResultMessage`，实际代码在本次修改前的 HEAD 中已使用 `displayMessage`；本次没有修改该展示逻辑或对应测试。

尝试执行 `cargo check --locked --manifest-path core/Cargo.toml -p operit-runtime -p operit-providers -p operit-tools --lib --tests`，检查在工作区其他改动中停止：`core/crates/plugin/sdk/src/js_sdk/toolpkg.rs` 的 `ToolPkgHookEventName` 同时定义了 `Variant21(ToolPkgChatLifecycleEventName)` 和 `Variant21(ToolPkgToolPermissionEventName)`，报 `E0428`。本次未修改该文件，完整类型检查尚未通过，新增 Rust 测试尚未执行。没有执行设备端复现；实际提示展示仍需相应运行环境验证。

2026-11-22 整合工作区提交时，权限事件已改用唯一的 `Variant24`，修复上述名称重复。
SDK 声明一致性检查通过；未重新执行完整 Rust 类型检查。
