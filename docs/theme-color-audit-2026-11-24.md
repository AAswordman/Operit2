# 主题颜色改动逐文件核对

核对范围为本轮修复的三个提交：`3335f9bce`、`f3c0c6423`、`3cfb177bc`。界面颜色角色以 `3cfb177bc^` 为基准；此次不更换更早历史版本的品牌种子或背景色。

## 已还原的界面改动

`3cfb177bc` 中以下 12 个生产文件均恢复到该提交之前的内容。核对时，这些文件在该提交之后没有其他已提交的修改。自定义文字颜色阈值另按下文说明调整。

| 文件（相对于仓库根目录） | 上次改变的行为 | 此次还原 |
| --- | --- | --- |
| `apps/flutter/app/lib/ui/common/ChatBackupRestorePanel.dart` | 备份项选中背景改为主色容器，并指定主色前景 | 副色容器背景及原来的前景处理 |
| `apps/flutter/app/lib/ui/common/contributions/ChatSidebarTabHost.dart` | “工作区”等页签选中背景和文字改为主色角色 | `secondaryContainer` / `onSecondaryContainer` |
| `apps/flutter/app/lib/ui/features/chat/components/ChatMessageLocatorPanel.dart` | 消息筛选标签图标和文字改为主色容器前景 | `onSecondaryContainer` |
| `apps/flutter/app/lib/ui/features/chat/components/style/ThemedChatMessage.dart` | 默认 AI 气泡由中性表面改为副色容器 | `surfaceContainerHighest` / `onSurface` |
| `apps/flutter/app/lib/ui/features/chat/components/style/input/agent/AgentChatInputSection.dart` | 排队按钮从第三色改为副色 | `tertiary` / `onTertiary` |
| `apps/flutter/app/lib/ui/features/chat/components/style/input/classic/ClassicChatInputSection.dart` | 经典输入框排队按钮从第三色改为副色 | `tertiary` / `onTertiary` |
| `apps/flutter/app/lib/ui/features/packages/components/StoreVisuals.dart` | 商店筛选标签的选中背景、边框和文字改为主色角色 | 副色容器及对应前景 |
| `apps/flutter/app/lib/ui/features/settings/appearance/AppearanceSettingsPanel.dart` | AI 气泡颜色编辑器默认值改为副色角色 | 原来的中性背景和文字默认值 |
| `apps/flutter/app/lib/ui/features/settings/appearance/ThemeAppearancePreview.dart` | 缩略图中的默认 AI 气泡改为副色容器 | `surfaceContainerHighest` |
| `apps/flutter/packages/operit_ui/lib/src/fixtures/theme_client_scene.dart` | Studio 默认 AI 气泡改为副色，AI 头像由第三色容器改为副色容器 | 原来的中性 AI 气泡及第三色头像 |
| `apps/flutter/packages/operit_ui/lib/src/render/drawer_chrome.dart` | 最近会话、分组会话及共用侧栏组件的选中态改为主色容器 | 副色容器及对应前景；保留原来的透明度 |
| `apps/flutter/packages/operit_ui/lib/src/render/theme_data.dart` | 为全局分段按钮、筛选标签、底部导航、导航栏和导航抽屉新增主色选中覆盖 | 删除全部新增覆盖，恢复 Material 原来的角色分配 |

`default_theme_rendering_test.dart` 恢复原有侧栏颜色预期；`theme_color_roles_test.dart` 改为验证原有角色分配、独立颜色覆盖以及发送/排队/取消操作。删除 `theme-v1-contract.md` 中上次新增的全局角色重分配说明。新增会话行渲染回归测试，检查实际组件的选中背景、文字和透明度，并支持导出真实 Flutter 渲染对照图。

## 保留的自定义颜色修复

- `3335f9bce`：黑、白、灰种子使用灰阶生成；设置页“自定义”色块显示实际保存的主辅色。这些修改没有改变默认品牌种子。
- `f3c0c6423`：用户明确选择的主辅色直接进入对应颜色角色，并设置可读前景；副色固定色调独立生成。这些修改限定于启用的自定义颜色，默认品牌配色继续使用原来的 `tonalSpot`。
- `OperitTheme.dart` 的语言、滚动导航等其他提交不在此次还原范围内。

此次没有新增“默认主题一套规则、自定义主题另一套规则”的组件取色分支。界面组件继续使用原来的颜色角色，颜色生成修复继续处理用户保存的具体色值。

## 后续截图指出的两处修复

- 金色、绿色上的黑字：`f3c0c6423` 按黑白之间更高的 WCAG 对比度选择前景，黑字从约 0.179 的相对亮度就开始胜出。按用户要求，直接将既有 `_themeAccentForeground` 的切换阈值改为相对亮度 0.6：有明显颜色的背景优先白色文字、图标，仅特别浅的背景使用黑色。该规则统一生成自定义主辅色的对应前景，不增加逐组件的对比度判断。
- 设置页分类选中项：`SettingsCategoryList.dart` 在本轮修复之前就单独使用 `primaryContainer`。按用户新指出的设置页截图，将分类选中背景、边框和图标底色改为对应副色角色，文字沿用配套的 `onSecondaryContainer`。
- 新建对话按钮本身为不透明主色容器，背景和文字颜色角色保持原样；自定义金色、绿色的前景会随统一阈值调整变为白色。

回归检查覆盖默认会话样式、绿色/琥珀色、黑/白、白/黑及品红/青色自定义组合，以及深浅色和透明背景。
