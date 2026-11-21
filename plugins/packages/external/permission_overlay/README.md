# 权限悬浮窗

`com.operit.permission_overlay` 是默认关闭的官方可选插件。从「更多插件」导入并启用后，
工具需要批准时会同步打开权限悬浮窗；工具箱中的「权限悬浮窗」可查询状态和申请
系统悬浮窗权限。Android 需先授予此权限，返回应用后 Core 会重放待审批请求。

UI 参考一代 `PermissionRequestOverlay.kt`：60% 黑色遮罩、居中圆角卡片、
盾牌图标、操作说明、工具名称、可滚动完整参数，以及拒绝、允许与持续授权操作。
按钮位于滚动区外。卡片适配屏幕大小，并沿用当前宿主的明暗主题。

聊天内权限卡片与弹窗使用同一个审批请求。在任意一处处理后，另一处同步更新。
多个聊天同时请求时按请求时间依次显示；后续请求会更新当前窗口，不叠加多个遮罩。
超时、取消或停用插件会移除对应弹窗；窗口不跨应用退出保存。持续授权使用二代现有语义：
仅在本次 runtime 运行中记住该工具，跨聊天生效，重启清除。

插件通过 [工具审批接口](../../../docs/TOOL_PERMISSIONS.md) 和
[通用悬浮窗接口](../../../docs/FLOATING_WINDOWS.md) 实现。
Android、Windows、macOS、X11 使用系统范围；其他宿主按接口报告的应用范围显示。
不自动申请系统授权，未授权时不会创建弹窗。

审批业务使用 `ToolPkg.toolPermissions`。展示回调只读取事件快照并调用
`ToolPkg.ui.floatingWindows.set("tool-permission", content | null)`；窗口生命周期
由宿主管理，审批通知由 Core 串行派发。插件没有窗口列表查询、句柄缓存、
手动队列、请求对象复制或跨 UI 上下文刷新协议。

开发检查：

```sh
tsc -p plugins/packages/external/permission_overlay/tsconfig.json --noEmit
node --test tools/tests/tool_permission_overlay.test.mjs
# 在 apps/flutter/app 下通过 FVM 执行宿主测试：
fvm flutter test test/floating_window_managed_test.dart test/floating_window_ownership_test.dart --no-pub
```

设备验证：授予悬浮权限后，在受限 AI 模式下触发一个需要批准的工具，切到其他应用
检查弹窗；分别从弹窗和聊天内卡片批准、拒绝；并行触发两个聊天的请求并检查顺序；
停止生成、等待超时、停用插件，确认弹窗随请求移除。
