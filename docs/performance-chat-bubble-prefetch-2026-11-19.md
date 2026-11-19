# 2026-11-19：滚动时提前准备聊天气泡

## 原因与修改

聊天行挂载后先通过 `projectMessagesForDisplay` 获取显示副本，再异步解析 Markdown。原先使用 Flutter 默认的 250px 滚动缓存，两段异步工作容易在气泡已经进入屏幕后才完成。显示转换等待期间还只返回一个 64px 空白占位，没有通知 `ChatMessageExtent` 正文仍在准备；已经测量过的高消息重新挂载时可能因此暂时缩短。

`ChatArea` 现在在视口两侧各准备一个屏幕高度的消息，缓存距离上限为 1200px。这个范围内的消息沿用原有批量显示转换和 Markdown 解析，远处历史仍保持按需挂载。已有的视口原点、历史分页与底部导航逻辑保持原有行为。

`TransformedChatMessage` 现在也发送正文准备状态通知，让行高记录覆盖显示转换和 Markdown 解析两个阶段。等待期间保留已测量的高度，完成后接受真实的新高度；卸载时释放对应的等待状态。

## 回归证据

新增两个实际气泡样式的滚动用例，分别使用顶部和末尾布局原点。显示转换和 Markdown 解析各人为延迟 80ms。测试选择离屏超过 300px 的气泡，确认其正文已提前解析；滚入屏幕后立即检查第一帧，正文节点已存在，且没有重复解析该消息。另一个测试验证 320px 消息重新挂载等待转换时仍保持 320px，完成后可以正常收缩到 180px，卸载也不会残留等待状态。

用 `git show HEAD` 的原始生产源码建立临时独立模块执行相同用例，三个新增用例均失败；修复后均通过。临时模块和测试副本在检查后删除，没有替换工作区的生产文件。

执行：

```sh
cd apps/flutter/app
./.fvm/flutter_sdk/bin/flutter test --no-pub \
  test/chat_area_streaming_cursor_test.dart \
  test/transformed_chat_message_test.dart \
  test/chat_message_extent_test.dart \
  test/chat_bubble_geometry_test.dart
```

结果为 142 项通过、2 项失败。以下失败也在上述原始源码模块上独立复现：

- `initially aligns a switched live chat without snapping later growth`：仍有一个帧回调。
- `retains live output until final message parts are ready`：未找到预期的 `ToolCallRow`。

长聊天回归中，80 条消息首次进入或切换时只解析附近的 23–24 条；500 条消息点击回到底部时解析 18–30 条，没有解析中途的第 250 条消息。扩大附近缓存会增加有限的提前准备工作，这是为滚动时留出异步准备时间。

四个修改的 Dart 文件静态分析没有错误或警告，仅有 `ChatArea.dart` 原有的 `collection` 未直接声明依赖 info。上述结果来自 widget 测试，尚未使用用户的真实长聊天做真机端到端验证。
