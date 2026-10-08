# ESP32 界面容器与布局说明

> 历史设计文档：LVGL 后端已删除。本文的移动 App 式控件布局仅作存档，
> 不再是当前实现约定。当前自绘 UI 位于 `apps/esp32/ui_port/operit_mini_ui.c`；
> 像素科技风重设计尚未实现。


设计依据：[esp32-ui-redesign-v1.svg](./esp32-ui-redesign-v1.svg)。目标屏幕为 **320×240 横屏**，SVG 中每块屏幕放大了 2 倍；设计板标题、说明文字和外框不属于设备界面。

本文是后续重写前端的实现约定，不表示当前固件已经完成这些行为。

## 1. 总体结构：一个屏幕，页面与浮层分开

SVG 的四格不是四个独立页面：第一格是聊天页；第二格是聊天页打开侧栏；第三格是聊天页打开键盘；第四格才是配对页。

```text
AppScreen                         整块屏幕；不滚动；裁剪到显示区域
├─ PageHost                       页面承载区；填满屏幕；仅一个活动页面
│  ├─ ChatPage                    聊天页
│  ├─ PairingPage                 配对页
│  ├─ PluginsPage                 插件页，具体内部设计后续确定
│  └─ SettingsPage                设置页，具体内部设计后续确定
└─ OverlayHost                    浮层承载区；填满屏幕；本身不拦截空白处触摸
   ├─ KeyboardWindow             浮动键盘
   ├─ DrawerLayer                侧栏与背景遮罩
   └─ DialogLayer                确认框、操作失败提示等
```

绘制顺序从下到上：活动页面 → 键盘 → 侧栏 → 对话框。关闭的浮层隐藏且不接收触摸。

**关键关系：侧栏和键盘都是页面的兄弟浮层，不是聊天内容、消息列表或输入框的子元素。** 这样消息滚动不会带走键盘，侧栏动画也不会改变聊天页坐标。

## 2. 聊天页：顶部栏 + 消息区 + 输入栏

```text
ChatPage                          纵向布局，背景使用 theme.background
├─ ChatHeader                    固定顶部栏，横向布局
│  ├─ OpenDrawerButton           菜单按钮
│  │  └─ MenuIcon
│  ├─ ActiveCharacterLabel       当前角色名称；占剩余宽度；过长省略
│  └─ ConnectionIndicator        连接状态
│     ├─ StatusDot
│     └─ StatusLabel
├─ MessageViewport               占剩余高度；唯一的聊天滚动区域
│  └─ MessageList                纵向排列；高度随可见内容增长
│     ├─ UserMessageRow          靠右排列
│     │  └─ UserBubble
│     │     └─ MessageText       按泡泡可用宽度换行
│     └─ AssistantMessageRow     靠左排列
│        ├─ SpeakerLabel        角色名
│        └─ MessageText          换行显示回复
└─ ComposerHost                  输入栏承载区；关闭键盘时位于底部
   └─ ComposerRow                横向布局
      ├─ MessageInput           填充剩余宽度；保存草稿
      ├─ SendButton             固定触摸尺寸
      │  └─ SendIcon
      └─ VoiceButton            固定触摸尺寸；现阶段禁用占位
         └─ MicrophoneIcon
```

- `ChatPage`、`ChatHeader`、`ComposerHost` 不滚动。
- 输入框、发送、语音是同一行的三个兄弟元素。发送和语音不放进输入框内部。
- 多段回复属于消息区，不能为了显示长回复而让整个屏幕滚动。
- 输入框变宽、标题变长时，优先压缩弹性区域，保留按钮触摸范围。
- 消息更新沿用现有节点。用户已经向上翻阅时不强制滚到底部；原本位于底部时才跟随新内容。
- 语音占位使用弱化图标并标记为不可用，不模拟录音状态。

## 3. 侧边栏：固定头部 + 半屏滚动列表 + 固定底部

```text
DrawerLayer                      全屏浮层；不滚动
├─ DrawerScrim                   覆盖聊天页的遮罩；点击关闭侧栏
└─ DrawerPanel                   左侧面板；纵向布局；自身不滚动
   ├─ DrawerHeader               固定头部
   │  ├─ HeaderRow               横向布局
   │  │  ├─ TitleLabel           “对话”
   │  │  └─ UnpairButton         “取消配对”；由配对状态控制
   │  └─ NewChatButton           占满面板可用宽度
   │     ├─ NewChatLabel         “新建对话”
   │     └─ PlusIcon             靠右
   ├─ CharacterViewport          高度约为屏幕的一半；纵向滚动
   │  └─ CharacterList           纵向排列角色卡
   │     ├─ CharacterCard        一个角色包裹该角色下的对话
   │     │  ├─ CharacterHeader   点击展开／折叠本角色
   │     │  │  ├─ Avatar
   │     │  │  ├─ CharacterName  填充剩余宽度
   │     │  │  └─ ExpandIcon
   │     │  └─ ConversationList  随内容增长；本身不独立滚动
   │     │     ├─ ConversationItem
   │     │     │  ├─ ChatIcon
   │     │     │  └─ ChatTitle   单行省略；当前对话显示选中背景
   │     │     └─ ConversationItem …
   │     └─ CharacterCard …
   ├─ FooterDivider              固定分隔线
   └─ DrawerFooter               固定底部，横向布局
      ├─ PluginsButton
      │  ├─ PluginIcon
      │  └─ PluginsLabel         “插件”
      └─ SettingsButton
         ├─ SettingsIcon
         └─ SettingsLabel        “设置”
```

**只有 `CharacterViewport` 滚动。** 顶部新建、取消配对和底部插件、设置始终留在屏幕内。不要给整个 `DrawerPanel` 开启滚动，也不要给每张角色卡再套一个滚动区域。

“角色卡包裹对话”指真实的父子关系：`CharacterCard → ConversationList → ConversationItem`。不能把角色标题和对话列表拆成面板中彼此无关的兄弟模块。

SVG 的第二张角色卡露出一部分，是列表可以继续向下滚动的提示；这属于滚动容器的正常裁剪，不是控件越出屏幕。

侧栏采用覆盖方式：`ChatPage` 保持原位，`DrawerPanel` 从左边滑入。遮罩在面板下方，覆盖剩余可见聊天区域；遮罩右上角关闭图标也属于遮罩，不属于聊天顶部栏。

交互约定：

- 打开侧栏前收起键盘，保留草稿。
- 点击对话后切换真实会话，再关闭侧栏；角色标题只控制展开／折叠。
- 新建按钮请求 Core 创建会话，成功后加入正确角色分组并切换。不能把“清空输入框”当作“新建对话”。
- “取消配对”打开确认框；确认后调用配对管理能力，成功后才清除界面配对状态并进入配对页。失败保持现状并显示错误。
- 动画建议约 120 ms，放进 motion 配置。动画被反向操作打断时，从当前偏移继续，不重建页面。

## 4. 键盘：独立浮窗，输入栏保持在它上方

```text
OverlayHost
└─ KeyboardWindow                 独立浮窗；裁剪子元素；自身不滚动
   ├─ KeyboardHeader              横向布局
   │  ├─ DragHint                 “拖动”
   │  ├─ DragHandle               拖动把手及其触摸区域
   │  └─ CloseKeyboardButton
   └─ KeyArea                     键盘内容
      ├─ CandidateRow            输入法候选区，需要时才占高度
      ├─ LetterRows              字母／数字按键矩阵
      └─ FunctionRow
         ├─ NumberModeKey
         ├─ InputMethodKey       中／EN（设计占位）
         ├─ SpaceKey             填充剩余宽度
         └─ DoneKey
```

`ComposerHost` 仍属于 `ChatPage`，不移动到键盘内部。键盘显示时，由页面布局计算输入栏的位置与消息区的剩余高度。

- 只允许通过 `KeyboardHeader` 的拖动区域移动窗口，按键区仅处理输入。
- 拖动过程中持续限制窗口边界，不能先让它移出屏幕、松手后再拉回。
- 键盘默认靠底部显示，输入栏位于键盘上方，发送与语音按钮随输入栏一起移动。
- 在这块小屏幕上优先支持上下拖动；横向移动也应受可用宽度限制。
- 收起键盘后，输入栏恢复底部位置，消息区恢复高度；草稿和消息滚动位置保留。
- `DoneKey` 表示完成输入并收起键盘，发送仍由输入栏的发送按钮执行。
- SVG 中的中文切换键是输入法布局示意，不代表当前固件具备中文输入引擎。接入前需明确可用能力。

### 边界与避让的计算

用容器尺寸和配置参数求位置，不能为“键盘打开”另写一套散落的绝对坐标。

```text
safeRect      = 屏幕矩形减去安全边距
keyboardWidth = min(键盘期望宽度, safeRect.width)
keyboardHeight = 按可用高度选定的键盘布局高度

minKeyboardY = headerBottom + composerHeight + gap
maxKeyboardY = safeRect.bottom - keyboardHeight

keyboardY = clamp(手指目标位置, minKeyboardY, maxKeyboardY)
composerBottom = keyboardY - gap
composerTop = composerBottom - composerHeight
messageViewport.bottom = composerTop - gap
```

当 `minKeyboardY > maxKeyboardY` 时，不能直接套 clamp：先折叠可选提示、让消息区缩到最小，再采用紧凑键盘布局；仍不够则使用专用输入视图。需要保持输入框、关闭和发送操作可达，不能通过隐藏溢出来掩盖错误尺寸。

## 5. 配对页：真实状态驱动

```text
PairingPage                       纵向布局；不滚动
├─ PairingHeader
│  ├─ TitleLabel                 “连接 Operit”
│  └─ WifiIndicator
├─ PairingContent                 填充剩余空间
│  ├─ InstructionTitle           “在 Operit 中添加此设备”
│  ├─ InstructionText            操作路径
│  ├─ PairingCodeCard
│  │  ├─ CodeCaption             “配对码”
│  │  └─ CodeValue               真正收到的配对码
│  └─ PairingStatusRow
│     ├─ StatusDot
│     └─ StatusLabel
└─ ConnectionHelpButton           固定底部
```

自动进入配对页表示展示可发现设备和配对引导。现有协议由 Core 发起配对，设备收到请求后显示配对码；打开页面本身不伪造配对码，也不假定设备能够主动搜索 Core。

配对凭据和当前连接状态必须分开：

| 实际状态 | 页面行为 |
| --- | --- |
| 尚未配对 | 启动时进入配对引导；收到请求后显示配对码 |
| 已配对、暂时离线 | 保留聊天入口，显示离线／重连；不能当成未配对 |
| 已配对、已连接 | 展示聊天；侧栏右上角提供取消配对 |
| 取消配对请求失败 | 保留配对与会话状态，显示操作错误 |
| 取消配对成功 | 清理相应界面状态并进入配对引导 |

## 6. 尺寸分配参考

以下是 SVG 对应的 **逻辑像素**参考值，用来建立集中布局配置。实现时以实际父容器尺寸推导子区域，不读取 SVG 设计板上的放大坐标。

| 区域 | SVG 参考 | 布局规则 |
| --- | --- | --- |
| 屏幕 | 320×240 | 来自 display 的实际逻辑尺寸 |
| 聊天顶部栏 | 高 40 | 固定基础高度，适配字体测量 |
| 输入行 | 高 34；横向边距 8 | 输入框弹性伸缩；操作按钮固定触摸尺寸 |
| 侧栏 | 宽 240，即屏宽 75% | 按屏宽比例计算，受最小操作宽度约束 |
| 角色列表视口 | y=69，高 119 | 约半屏高；头尾预留后分配剩余高度 |
| 侧栏底部按钮 | y=203，高 29 | 固定底部；不随角色列表滚动 |
| 键盘窗口 | x=8，y=114，宽 304，高 124 | 默认靠底；按安全区域限制位置与尺寸 |
| 键盘展开时输入行 | y=70，高 34 | 由键盘顶部减间距和输入行高度求得 |

触摸设备上的实际字体、键盘行高和触摸尺寸需要在同一布局配置内调整。文字放不下优先换行、缩短说明或省略标题，不通过任意缩小字号解决所有问题。

## 7. 统一主题与状态来源

所有容器和控件使用同一份主题对象，按语义取色：

```text
Theme
├─ background                    聊天／配对页面背景
├─ surface                       侧栏、输入框、配对码卡片
├─ surfaceInset                  角色分组内部背景
├─ selection                     当前对话、用户消息
├─ accent / onAccent             新建、发送等主要操作
├─ text / textSecondary          正文、次要说明
├─ border / scrim                边界和浮层遮罩
└─ disabled                      语音占位等不可用状态
```

SVG 里的颜色是这版主题的示例值。控件实现只引用主题字段，不通过“某个十六进制颜色等于另一个颜色”来猜测语义。

界面数据来自单一状态模型：

```text
UiState
├─ page
├─ drawerOpen
├─ keyboardVisible / keyboardPosition
├─ pairingState / connectionState / pairingCode
├─ activeCharacterId / activeChatId
├─ characters / conversations / messages
├─ draft / sending / sendError
└─ theme
```

角色名、对话标题、当前会话、配对码和连接状态都要来自真实数据。SVG 中的“日常助手”“学习搭档”“今天的工作计划”和数字码均为设计样例，不能写成产品固定内容。

角色与对话能力继续由 Core/Space 负责。设备仅保存显示所需的有限视图和草稿；接口尚未提供的能力需要补齐契约，不能在 ESP32 内复制一个聊天数据库来模拟。

## 8. 映射到 LVGL 的实现方式

| 结构 | LVGL 实现建议 |
| --- | --- |
| Screen / PageHost / OverlayHost | 普通 `lv_obj` 容器，关闭不需要的滚动与触摸拦截 |
| ChatPage / DrawerPanel / CharacterList | Flex column；明确固定区域与 grow 区域 |
| Header / ComposerRow / DrawerFooter | Flex row；文本／输入 grow，按钮保留宽度 |
| MessageViewport / CharacterViewport | 仅开启纵向滚动，按需显示滚动条 |
| CharacterCard / ConversationList | 非滚动容器；高度由子内容决定 |
| MessageInput | `lv_textarea`；背景、文字、边框及焦点样式全部取 Theme |
| Buttons | `lv_button`；图标与文案属于按钮；空白也可点击 |
| KeyboardWindow | 独立 overlay 容器；标题拖动区域 + `lv_keyboard` 或按键矩阵 |
| DrawerScrim | 可点击半透明容器；放在面板下方 |
| ConfirmDialog | 顶层模态容器，拦截后方点击 |

键盘创建后应明确清除其默认底部对齐约束，再由 `KeyboardWindow` 内部布局定位。不要混用“控件仍按默认底部对齐”和“再次指定绝对 y 坐标”，否则可能产生当前遇到的越界。

实现可按以下职责划分，共享固件和模拟器的同一套视图：

- `theme`：主题语义和样式应用。
- `layout`：屏幕、安全区域、浮层边界、键盘避让。
- `chat_view`：聊天顶部、消息和输入栏。
- `drawer_view`：角色与对话分组、固定头尾。
- `keyboard_view`：窗口、拖动和输入事件。
- `pairing_view`：配对引导和状态呈现。
- `ui_controller`：状态变更、页面切换与动作分发；不承担聊天持久化。

正常刷新修改现有节点的属性；只有页面切换或列表结构变化才增删相关节点。侧栏滑动、流式文字和连接状态变化都不应反复重建整个页面。

## 9. 重写时的核对点

1. 从菜单打开侧栏，插件与设置一直在屏幕底部。
2. 滚动角色列表时，新建按钮、顶部状态和底部按钮不移动。
3. 一个角色展开后，其对话确实显示在该角色容器内部。
4. 主题切换覆盖聊天页、侧栏、键盘、配对页及控件焦点状态。
5. 打开、拖动和关闭键盘不会让窗口或发送按钮越出屏幕。
6. 点击键帽不会触发窗口拖动；点击把手不会输入字符。
7. 已配对设备短暂掉线后不自动退回首次配对流程。
8. 创建会话与取消配对均以服务返回的结果更新界面。
9. 新视图替换旧视图后，删除失效页面构造函数、旧事件路由和重复状态来源。
