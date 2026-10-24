# 脚本执行与 ToolPkg 前置 Hook 超时

普通 JS 调用、AI 工具调用、工作流、IPC、公共 API、插件通知及 after Hook 不设置脚本完成超时。Promise await、Host 异步操作和定时器等待不会消耗普通脚本的执行预算。调用持续到结果完成、调用方主动取消或运行时销毁；移除完成期限后仍保留主动取消对同步死循环的中断能力。

仅消息提交前的 `submit_requested` 与前置 Hook 使用总超时预算。默认 10 秒，可在“工具设置 → 高级设置”调整，范围 1 至 60 秒。预算覆盖整个前置链路，包括排队、同步执行和异步等待；每个 Hook 使用剩余时长，不会重新取得完整预算。

受限阶段包括聊天 `before_create`、工具执行前的拦截、Prompt Input 的 `before_process`、History 的 `before_prepare_history`、Finalize 的 `before_finalize_prompt` / `before_send_to_model`、Summary 的准备与发送前阶段，以及系统／工具提示词的前置编排与过滤。`after_process`、`after_prepare_history`、`after_compose_system_prompt`、`after_compose_tool_prompt` 和 `after_generate_summary` 不设置超时。

前置链路超时后立即终止当前 Hook，并将错误传回调用方，中止依赖该 Hook 的操作。通过 Host Toast 弹出“前置插件「包名:HookID」响应超时（阶段），本次 Hook 已终止。”，同时记录错误日志。提示展示失败也会写入返回错误，不会吞掉。

JS 引擎按当前实际运行的调用选择中断令牌。一个前置 Hook 超时，只终止其自身，不会使同一运行时里等待 Promise 的普通调用一起失败。Host 状态调度层接受调用方的令牌，不再自行叠加 60 秒期限。

Host 状态创建显式声明执行或编译角色。原生 Host 为编译角色保留独立的执行资源，避免所有脚本 worker 同步等待共用池里的编译器。取消会话后的 Promise 拒绝与原生 await continuation 仍携带取消状态，不能绕过前置超时继续占用执行器。
