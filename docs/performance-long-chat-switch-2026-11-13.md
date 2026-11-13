# 2026-11-13：长聊天切换、回到底部及折叠详情的渲染开销

## 上次修复的范围

`364d1d594`（2026-11-11，`fix(chat): avoid parsing offscreen history when opening conversations`）让 `ChatArea` 首次布局以最后一条消息为原点，避免从最早一条消息开始反复估算并追赶底部。原回归测试覆盖 80 条很短的静态消息，没有覆盖仍在输出的会话和单个很大的代码块。

本轮使用本地 Flutter widget 测试复现。没有用户反馈中的原始聊天数据，也没有采集该用户真机的火焰图。

## 复现一：仍在输出的长聊天绕过了末尾原点

原条件包含 `!_hasLiveBottomStream()`。末尾 AI 消息带着 `contentStream` 时，即使开启自动跟随，也会从第一条历史消息开始布局；初始底部对齐过程随即触发大量已经离屏的 Markdown 请求。

新测试先挂载另一个会话，再切到 80 条消息、末尾仍有输出流的会话，统计所有 Markdown 解析请求，包括随后被卸载的消息：

| 场景 | 修改前 | 修改后 |
| --- | ---: | ---: |
| 切回仍在输出的长会话，历史 Markdown 请求 | 79 | 8 |
| 普通静态长会话，历史 Markdown 请求 | 9 | 9 |
| 最新消息包含 5000 行代码的长会话，历史 Markdown 请求 | 未单独测量 | 9 |

修复取消初始原点选择中的输出流排除条件。实时增长仍由原来的底部跟随器负责，后续消息增长不会重设原点。回归测试同时验证仍能向上翻到第一条消息。

## 复现二：代码块限制了高度，但仍然排版全文

`EnhancedCodeBlock.build` 原来会先对全部代码行做语法高亮，然后 `CanvasMonospaceCodeBlockBody` 用 `Column` 为每一行创建代码和行号控件。外围的 420px 高度限制和 `SingleChildScrollView` 只限制可见面积，没有减少子树创建和布局。

5000 行代码的回归测试中：

| 指标 | 修改前 | 修改后 |
| --- | ---: | ---: |
| 首次挂载的 `Text` 控件 | 10001 | 51 |
| 单轮首次布局耗时 | 2515ms | 345ms |

时间来自同一台机器上的不同 widget 测试进程，包含测试框架及首次布局开销，是单次观测，不能当成真机性能基准或端到端切换耗时。控件数量和请求数量是本轮主要的可复核证据。

代码正文改用有高度上限的 `ListView.builder`，在行进入可视区域及缓存范围时才创建和高亮。保留 `ChainedScrollController` 的边缘滚动交接。关闭换行时使用横向滚动容器及按代码列数估算的宽度；代码清洗和拆行按源码变化更新，复制操作仍使用完整源码。

新增检查覆盖首次只高亮少量行、滚到第 5000 行、切换换行、完整复制、替换源码以及中文/emoji/不完整代理字符。同时在聊天切换测试中通过语义代码块事件走完整的代码控件渲染路径。

## 验证与现存失败

主要相关回归命令：

```sh
cd apps/flutter/app
flutter test --no-pub \
  test/enhanced_code_block_utf16_test.dart \
  test/chat_area_streaming_cursor_test.dart \
  test/chained_scroll_controller_test.dart \
  test/turn_diff_summary_test.dart \
  test/tool_display_components_test.dart
```

结果为 75 项通过、2 项失败。两项失败为：

- `initially aligns a switched live chat without snapping later growth`：预期没有剩余帧回调，实际导航锚点更新仍留下一个回调。
- `retains live output until final message parts are ready`：预期出现 `ToolCallRow`，实际未出现。

扩展检查 `structured_message_part_renderer_test.dart` 时，`uses the shared tool row for completed structured tool calls` 也存在相同的工具行缺失失败。

为区分本轮回归与已有问题，曾临时将本轮修改的三个生产源码文件全部恢复到 `HEAD`，单独运行以上三项；三个失败均原样复现。随后恢复本轮补丁，未修改或屏蔽这些原有断言。

对修改涉及的 Dart 文件执行静态分析，无错误；`ChatArea.dart` 原有 `collection` 未直接声明依赖的 info 仍存在。`git diff --check` 通过。

这些结果确认修复了两条可复现的多余渲染路径。数据库读取、80 条显示窗口的总字节量、传输解码，以及单条消息含大量工具记录的解析成本，仍需要结合用户那条真实聊天的真机采样判断，不能据本轮 widget 测试宣称所有长聊天等待均已消除。


## 补充复现：点击回到底部沿途布局历史消息

导航按钮原来向估算的 `maxScrollExtent` 执行 220ms `animateTo`。长聊天的行高尚未全部确定，滚动途中会挂载大量中间消息，触发它们的 Markdown 请求。

改为通过已有的消息定位机制，以最新一条消息作为布局原点，直接建立末尾视口。若当前显示的是旧的历史窗口，先请求最新窗口，等窗口加载完成后再建立原点。通过导航代次丢弃被切换会话或用户导航取代的请求，避免延迟完成的回调再次跳转。

500 条消息、从最早消息点击底部按钮的 widget 测试结果：

| 场景 | 修改前 Markdown 请求 | 修改后 Markdown 请求 |
| --- | ---: | ---: |
| 静态聊天 | 1489 | 6 |
| 最新消息仍在输出 | 471 | 0 条历史请求 |

统计在首次窗口布局结束后清零，包含点击后所有中间挂载触发的请求。测试同时断言到达实际底部，并恢复自动跟随。新增窗口测试覆盖最新快照已更新但加载尚未结束、加载结束后异步回调才返回，以及回调返回前已经切换聊天。

## 补充复现：折叠工具组仍提前准备隐藏正文

`ThinkToolsXmlNodeGrouper` 原来在父级 `build` 中无条件调用 `mergeRender`，然后才让可见性控件返回空节点。隐藏详情虽然没有挂载，工具参数、结果及正文控件仍会提前准备。

现在可见性控件接收惰性 builder，默认折叠时不调用详情合并和渲染。展开才构建正文；收起时复用已展开的子树完成原有淡出，动画结束即卸载，收起期间的新数据也不再准备详情。带计数器的测试验证默认折叠及折叠更新的详情准备、正文构建次数均为 0。

同时检查了其它折叠入口：

- 工具参数全文格式化、结构化结果 JSON 缩进格式化，延迟到各自详情展开或用户主动复制时执行。外层工具行展开、内层参数仍折叠时也不格式化；格式化结果由详情控件缓存，输入变更时失效。
- 普通 `<details>` 仅在展开时提取正文和创建 Markdown。思考、搜索正文原本已有展开条件，新增测试防止退化；折叠思考标题的无障碍标签不再附加整段隐藏正文。
- 折叠文件 diff 仅保留标题信息。标题增删计数改为一次轻量扫描，不再为计数创建完整差异行和行内高亮；不显示标题时跳过计数。

折叠测试使用 5000 行隐藏正文、5000 对差异行和延迟求值计数器，覆盖默认折叠、展开、重新收起、折叠期间更新、重新展开，以及不展开直接复制完整结果。必要的原始消息结构解析和可见标题/预览仍然存在，本轮保证的是隐藏详情不提前构建与排版，并移除了已定位的隐藏全文格式化开销。

## 补充修复后的最终验证

```sh
cd apps/flutter/app
flutter test --no-pub \
  test/collapsed_chat_content_test.dart \
  test/think_tools_xml_node_grouper_test.dart \
  test/think_tools_group_activity_test.dart \
  test/thinking_preview_widget_test.dart \
  test/thinking_preview_buffer_test.dart \
  test/tool_display_declaration_test.dart \
  test/turn_diff_summary_test.dart \
  test/chat_scroll_navigator_test.dart \
  test/chat_area_streaming_cursor_test.dart \
  test/enhanced_code_block_utf16_test.dart \
  test/chained_scroll_controller_test.dart \
  test/tool_display_components_test.dart
```

最终结果：206 项通过、2 项失败。失败仍为前文已经在 `HEAD` 复现的两个 ChatArea 用例，新增的底部导航和折叠测试均通过。单独运行折叠内容与工具展示声明测试，18 项全部通过。最终同轮代码块检查仍仅挂载 51 个 Text，首次布局单次观测为 352ms。

13 个修改涉及的 Dart 源码/测试文件静态分析无错误，仅保留原有 `collection` 直接依赖声明 info；`git diff --check` 通过。以上是 widget 回归证据，尚未使用用户反馈中的真实超长聊天做真机端到端计时。
