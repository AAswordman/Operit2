# 回车发送设置

入口：**设置 → 全局行为 → 聊天输入 → 回车键发送消息**。

- 开启：未展开的输入框中，Enter / 数字键盘 Enter 发送消息。
- 关闭：Enter / 数字键盘 Enter 换行，使用发送按钮发送消息。
- Ctrl+Enter 和展开输入框中的 Enter 始终换行。
- 输入法仍在编辑候选文本时，Enter 交给输入法处理；长按 Enter 不会连续发送。
- Classic、Agent 两种输入样式共用开关，修改后立即更新当前编辑器，保留草稿、焦点和选区。

未设置时，沿用原有桌面/移动布局默认值：宽度达到 `navigationTabletBreakpoint` 时开启，低于该宽度时关闭。原有判定实际来自布局宽度，并非操作系统。手动选择后，窗口宽度变化不再影响开关。

设置保存于 `user_preferences.preferences.json` 的布尔键 `enter_sends_message`，沿用 Core 偏好存储及同步链路。键不存在表示尚未手动选择，不会因默认值写入而覆盖其他设备上的选择。应用启动时加载，并订阅后续偏好变化；收到同步更新后，设置页和输入框即时采用新值。非法持久化值直接报告错误。

回归覆盖：`chat_input_ime_keyboard_test.dart` 中的两种真实输入框、开关反复切换、窗口宽度变化、展开状态、主键盘与数字键盘 Enter，以及既有 IME/Ctrl+Enter/长按行为；`preferences_sync_cache_test.dart` 覆盖读取、保存、同步变更、删除与非法值。
