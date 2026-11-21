# 工具审批呈现接口

`ToolPkg.registerToolPermissionHook` 让启用的插件观察当前 Core 的待审批工具调用。
注册后和应用回到前台时会重放当前列表；创建、批准、拒绝、超时和取消都会触发 `requests_changed`。
事件是完整快照，按 `requestedAtMillis`、`requestId` 排序，包含各聊天的请求。
审批数据和决策属于独立的 `ToolPkg.toolPermissions`，不依赖 UI。
事件直接提供完整请求列表；其他运行时调用可用 `ToolPkg.toolPermissions.list()` 查询。

```ts
/** Declares the oldest approval as the content of one managed window. */
export async function onPermissionsChanged(event: ToolPkg.ToolPermissionHookEvent): Promise<void> {
  const requests = event.eventPayload.requests;
  await ToolPkg.ui.floatingWindows.set("approval", requests.length === 0 ? null : {
    route: "toolpkg:com.example.approvals:ui:approval",
    fill: true,
    state: { request: requests[0], pendingCount: requests.length },
  });
}

/** Registers the trusted approval presenter during registration capture. */
export function registerToolPkg(): void {
  ToolPkg.registerToolPermissionHook({ id: "approvals", function: onPermissionsChanged });
}

// From the approval UI's user action:
await ToolPkg.toolPermissions.respond(request.chatId, request.requestId, "allow");
```

每项请求含 `requestId`、`chatId`、`description`、`requestedAtMillis` 和
`tool: {name, parameters: [{name, value}]}`。插件收到的参数排除内部调用归属字段
`__operit_package_caller_name`、`__operit_package_chat_id`、`__operit_package_caller_card_id`。

| 决策 | 作用范围 |
| --- | --- |
| `allow` | 当前调用 |
| `deny` | 拒绝当前调用 |
| `allow_session` | 当前工具在这次 runtime 运行中的后续调用，跨聊天，重启清除 |

插件接口与聊天内卡片共用审批 broker。`chatId` 必须与请求所属聊天一致；
已处理、超时、取消、重复提交和未知决策都返回错误。第一项成功提交的决策生效，
其他界面随快照移除该请求。审批不会改变 AI 模式，也不能增加 Host 的系统权限。

调用方由原生执行引擎认证。只有当前已启用且注册了工具审批 hook 的包可使用
`list` / `respond`；JavaScript 参数不能指定审批插件身份。启用此类插件表示信任其
展示和提交工具审批，应只安装可信来源。它不是 AI 工具接口。
注册阶段不能查询或提交审批。

Core 在专用队列中串行派发审批通知，每次派发前读取最新的请求列表，并等待
当前回调完成后再派发下一项。队列独立于工具执行的前置策略 hook，审批 UI
不需要等待工具执行完成。窗口创建期间请求被处理时，后续通知会更新或关闭窗口。
插件直接使用事件快照，不需要自行建队列、重新查询列表或记录已展示的请求。
`floatingWindows.set` 的创建、更新、恢复显示与幂等关闭由窗口宿主管理。
等待工具的 Future 被取消时，broker 会清除请求并通知所有界面。

官方可选插件见 [权限悬浮窗](../packages/external/permission_overlay/README.md)。
悬浮窗的真实显示范围和系统授权流程见 [通用悬浮窗](FLOATING_WINDOWS.md)。
