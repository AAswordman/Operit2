# 工作区终端重排与会话生命周期

## 尺寸重排

工作区使用仓库内的 `apps/flutter/thirdparty/xterm`。旧行最后一列写入
双列字符时，`getTrimmedLength(oldWidth)` 可以比旧宽度多一列。窗口恰好
增加一列后，重排器重用的旧行已经填满新行；继续拼接下一条软换行时，
必须先提交满行，再计算复制范围。否则复制数量为零，最后一个字符的
索引会成为 `-1`，在布局阶段抛出 `RangeError`。

## 会话资源归属

`WorkspacePtyProcess` 是现有 PTY 的视图订阅，不拥有后端会话。

- 输出结束后，订阅读取退出码、结束本地输出并完成 `exitCode`。
- `kill()` 只取消本地订阅和待发送的尺寸更新；之后的结束事件不再访问后端。
- 关闭工作区终端标签或会话时，工作区移除视图，再通过
  `WorkspaceTerminalSessions.closePtySession` 释放后端会话。

输出结束不能自动调用 `closeTerminalPty`：终端标签仍然存在，用户随后
关闭标签会再次释放同一会话，严格检查会话 ID 的宿主会因此报错。
保留会话直到拥有者关闭，也使退出码可以被其他订阅者正常读取。

## 回归检查

在 `apps/flutter/app` 运行：

```sh
fvm flutter test --no-pub test/terminal_reflow_test.dart test/terminal_session_lifecycle_test.dart test/terminal_session_failure_test.dart test/terminal_escape_sequences_test.dart
```

重排用例覆盖行末双列字符、软换行和增加一列的组合；生命周期用例
使用拒绝重复释放的宿主，验证自然退出后关闭标签只释放一次，视图
卸载不会释放宿主会话。
