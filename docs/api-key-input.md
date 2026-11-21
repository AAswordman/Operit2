# API Key 输入

模型供应商、密钥池及复用 `ApiKeyTextField` 的语音供应商配置使用普通文本输入。
输入框获得焦点时显示密钥原文，失去焦点时自动显示星号；不再提供手动显隐按钮。
密钥池批量添加仍支持按行粘贴多个 Key，失焦遮挡时保留换行。

遮挡仅通过 `TextEditingController.buildTextSpan` 改变渲染内容，原始控制器中的密钥、
选区和输入法组合区保持真实值。输入连接始终使用 `obscureText: false` 和普通文本类型，
避免密码输入标记触发系统安全键盘，支持输入法及系统菜单的正常粘贴。

保留 `enableSuggestions: true`：Flutter Android 输入层会把 `enableSuggestions: false`
映射成 `TYPE_TEXT_VARIATION_VISIBLE_PASSWORD`，仅关闭 `obscureText` 仍会带有密码类型标记。
关闭自动纠错、智能引号和智能破折号，避免改写 Key；通过
`enableIMEPersonalizedLearning: false` 请求输入法不将密钥用于个性化学习。

实现：`apps/flutter/app/lib/ui/features/settings/components/ApiKeyTextField.dart`。
回归检查：`apps/flutter/app/test/api_key_text_field_test.dart`、
`apps/flutter/app/test/api_key_pool_editor_test.dart`。
