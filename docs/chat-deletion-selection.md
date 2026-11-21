# 删除对话后的选中状态

删除当前对话成功后，Core 清理该对话的执行状态、消息流和配置缓存，然后打开剩余对话中 `updatedAt` 最大的一条。更新时间相同时按置顶、手动顺序和 ID 确定顺序。置顶和手动排序不会覆盖不同对话之间的活动时间先后。

这项切换由 `ChatHistoryDelegate.clearDeletedChatState` 统一处理，本地删除和同步收到的删除使用相同链路。删除非当前对话不改变当前选择；删除最后一条对话后保持空选择。`FOLLOW_GLOBAL` 模式持久化新选择，`LOCAL_ONLY` 模式只切换自己的界面。

切换通过已有的 `openChatHistory` 加载消息与发布选中状态，不依赖执行配置解析，不创建新对话。存储和打开对话的错误继续向调用方传播。

回归场景位于 `ChatServiceCore.rs` 的 `chat_deletion_and_route_handoff_preserve_execution_ownership`：当前对话删除后打开最近的对话、重复删除通知、置顶旧对话不抢占最近对话、非当前对话删除、局部选择与全局选择隔离，以及删除最后一条对话。
