# AI 自主回复

Operit2 官方可选 ToolPkg，包 ID 为 `com.operit.ai_chat_presentation`。随应用提供安装包，不会自动安装。

先在「插件市场 → 官方扩展」中安装「AI 自主回复」，在包管理中启用后，再从聊天输入菜单打开当前会话的「AI 自主回复」开关。开关默认关闭，设置保存在会话中。当前会话开启 Waifu 时优先使用 Waifu。若 Waifu 版本低于 1.1.4 或状态读取失败，插件会让出展示权。

回复生成后，插件使用已配置的聊天模型，结合最近一条用户消息决定展示原回复中的哪些内容。模型可以选择一个或多个气泡，也可以保持沉默。原始回复保留在模型历史中，聊天页显示选中的气泡。模型保持沉默时，本轮没有文字气泡。工具执行记录和思考过程仍按聊天设置展示。

启用后每轮最多增加一次计入用量的模型调用，气泡会在判断完成后显示。判断失败、输出无效或超过 8 秒时，聊天页显示原回复。超过 12,000 字符的回复直接展示原文。

## 开发检查

在仓库根目录运行以下命令。

```powershell
tsc -p plugins/packages/external/ai_chat_presentation/tsconfig.json
node --test plugins/packages/external/ai_chat_presentation/tests/presentation.test.mjs
```
