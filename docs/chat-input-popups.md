# 输入框提及建议弹层生命周期

## 故障证据

`operit-diagnostics-1795164758095.log.txt` 在 `09:04:55.004` 记录：

```text
Null check operator used on a null value
Overlay.of
_overlayBoxOf
overlayTargetRectOf
_ClassicChatInputSectionState._targetRect
_ClassicChatInputSectionState._popupPlacement
_ClassicChatInputSectionState._showMentionSuggestionPopup.<anonymous closure>
```

Classic 与 Agent 原先各自把提及建议插入独立的 `OverlayEntry`，在下一帧同步内容，仅在 `dispose` 时删除弹层。弹层重建时又通过输入框 State 的 `context` 查 Overlay、通过 `GlobalKey` 读取输入框的 RenderObject。

`mounted` 不能证明 Element 当前处于 active 状态。在输入框失活、迁移到另一个页面、等待重新挂接的阶段，State 仍然 mounted，旧 Overlay 的独立 Entry 也仍能重建。此时旧弹层读取了失活输入框的祖先或渲染节点。

真实输入框回归用例保留同一个 GlobalKey/EditableText State，将它从一个 `RetainedPage` 迁移到另一个页面的 Overlay。修改前两种样式均抛出 `Cannot get renderObject of inactive element`，调用链都经过 `_targetRect` 与提及弹层 builder。这复现了相同的生命周期归属缺口；Android 原日志的 release 空值堆栈仍需实机复测。

## 修复边界

两种输入样式统一使用 `ChatMentionSuggestionPopup`：

- 输入卡片始终位于同一个子节点位置；建议出现、关闭或更新不会替换 EditableText。
- 仅在建议可见时挂接 `OverlayPortal`。弹层随所属输入框失活、迁移、恢复和销毁，不再由独立 Entry 和下一帧回调维护。
- 弹层 RenderBox 在布局回调中读取 portal 所属 Overlay 中的锚点原点，坐标变换在框架允许访问相关渲染节点的阶段执行，随后约束和定位候选面板。
- 仅在已有帧中由有效的 portal State 刷新定位，不额外请求空闲帧；失活时取消刷新。Overlay 的 RenderObject 延迟到布局阶段解析，避免初次挂接时尚未创建，也避免迁移时把旧 Overlay 中的渲染节点提前标脏。
- 宽度上限 700、页面左右各 12 像素边距、输入框上方 8 像素间隔、96–420 的最大高度限制保持一致。MediaQuery 的键盘变化也会触发布局更新。
- 点击面板内部可继续选择，点击外部关闭；新建议内容可以重新打开。Classic 与 Agent 沿用各自的外部点击行为。

定位使用普通 `OverlayPortal`，并把坐标计算放在 RenderBox 的布局回调中。固定 SDK 的弹层布局构建器在上述 RetainedPage 布局期跨页面迁移中触发 `_RenderLayoutBuilder was mutated`，因此构建与坐标计算分开处理；现有迁移用例同时覆盖这一交互。坐标变换可能经过路由动画的 `RenderFractionalTranslation`，也必须在布局回调允许访问其尺寸时执行。

本次只迁移提及建议弹层；输入设置、附件与模型选择器仍有各自的交互链路。修复没有改变提及搜索、候选项加载、选中回调和模型请求策略。

## 本地验收

在 `apps/flutter/app` 使用项目固定的 FVM SDK 执行：

```powershell
fvm flutter test --no-pub test/chat_input_popup_geometry_test.dart test/chat_input_menu_presentation_test.dart test/retained_page_overlay_test.dart test/agent_input_keyboard_test.dart test/chat_input_ime_keyboard_test.dart
fvm flutter analyze --no-pub lib/ui/features/chat/components/style/input/common/ChatMentionSuggestionPopup.dart lib/ui/features/chat/components/style/input/classic/ClassicChatInputSection.dart lib/ui/features/chat/components/style/input/agent/AgentChatInputSection.dart test/chat_input_popup_geometry_test.dart
```

回归覆盖两个真实输入框跨 Overlay 迁移、页面关闭、保留页面隐藏/调整大小/恢复、键盘弹出与收起、建议消失与重现、内部选择及外部关闭，并检查输入 State、草稿、焦点和 IME composing 区间。既有输入菜单与键盘行为一并验证。

2026-11-20 本地验收：弹层专项 22 项通过，上述五个文件共 238 项通过；四个修改 Dart 文件的静态分析无问题。原工作区的角色卡插件有并行修改导致资源准备阶段类型错误，因此测试在 HEAD 的独立验证检出中执行，复制相同的三个输入组件文件及回归测试，并通过 SHA-256 确认四文件与原工作区完全一致。静态分析直接在原工作区执行。未构建安装包，也未完成 Android 实机验收。

本次不新增或修改工作流。设备验收仍需在 Android 上打开 `@`/`/` 建议，反复切换聊天和页面、切换输入样式、展开输入框及开关键盘，确认候选正常选择、输入不丢失、没有残留弹层或异常。
