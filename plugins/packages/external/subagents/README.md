# 子代理插件 / Subagents

插件 ID：`com.operit.subagents`，ToolPkg API：`2.0.0`。

安装并启用 `dist/subagents.toolpkg` 后，Classic 和 Agent 聊天输入框上方会在存在运行中子代理时显示面板；Classic 模式将面板放在输入气泡外的上方。没有运行中任务时整块区域收起，不占输入区高度。面板默认只显示状态栏，点击标题或箭头可展开、收起任务列表；任务执行、等待期间仍可收起，状态更新不会自动展开。也可通过项目原有的 external 插件打包流程分发。本插件作为可选插件提供，默认不启用。

## 使用

已有任务时可点击面板的「创建」，填写名称和自包含的任务说明，启动一个独立会话；首个任务通过 `spawn_agent` 工具创建。子代理会写入原生 `parentChatId`，在工作区、角色和分组侧边栏中收在父会话下，默认不展开；同时通过独立的 `sourceChatId` 继承源会话配置，不复制消息历史；工具创建会携带实际调用参与者 ID。角色组中的具体参与者应通过主聊天工具创建，以保留真实参与者身份。

点击列表中的任务通过宿主统一 `AiChat` 控件查看子会话，包括以往轮次在内的持久消息和实时执行内容；正文、思考内容和工具展示均复用主聊天的渲染。控件以 `chatId` 绑定独立会话槽，不切换主聊天；`showInput: false` 将发送、等待和打断操作交给子代理管理器。「完整会话」可在宿主聊天页面打开该会话。

「等待 30 秒」可以反复使用。超时仅结束这次等待，agent 继续执行。「发送」用于已结束的轮次；「打断并发送」请求取消当前轮次，收到真实结束回执后，在同一子会话发送新指令。原生取消请求确认不代表轮次已经结束。

「关闭子代理」会取消并关闭任务及其后代，保留原有会话。用户仍可打开保留的聊天继续提问，并在该聊天创建新的子代理；旧任务的关闭状态不受影响。关闭过程中的并发创建仍被拒绝。关闭后的任务不能继续发送，有其他任务运行时可在面板「历史」中查看。最后一个任务结束后面板收起，已打开的详情仍可查看结果。

主聊天激活 `subagents` 工具包后可以使用：

| 工具 | 参数 | 行为 |
| --- | --- | --- |
| `spawn_agent` | `name`, `task` | 创建并立即返回 agent ID，后台执行 |
| `wait_agent` | `agent_ids`, `timeout_ms=30000` | 等待任一所选当前轮次结束，允许 1..30000 毫秒；超时可再次等待 |
| `send_input` | `agent_id`, `message`, `interrupt=false` | 发送新轮次；运行中需显式打断 |
| `close_agent` | `agent_id` | 等待实际停止，关闭任务及后代，保留历史 |
| `list_agents` | 无 | 读取当前聊天子任务与最终结果 |

所有模型工具从宿主取得真实聊天 ID，不接受调用参数指定其他父会话。最多同时运行 8 个子代理；新建和后续轮次共享这个限制。

`wait_agent` 观察的是调用时的轮次。其他入口打断并启动下一轮时，旧 wait 返回旧轮次的取消回执；下一次 wait 才观察新轮次。多选等待采用任一结束语义，继续等待剩余任务时应传入仍在运行的 ID。

## 架构和生命周期

- `src/manager.ts` 管理创建、轮次、容量、等待订阅、取消和关闭。
- `src/service.ts` 在包主运行时持有唯一 manager，通过 `ToolPkg.ipc` 接收工具和 UI 操作。
- `src/storage.ts` 使用 host Storage 对详细记录和父会话状态列表进行原子提交，UI 通过 exact-key watch 订阅。
- `src/ui/panel.ui.ts` 使用 `AiChat({ chatId, showInput: false })` 展示会话，只持有表单、弹窗和状态订阅；首次读取和没有运行中子代理时保持零高度，任务创建与完成通过 Compose 状态订阅直接更新。关闭详情或切换聊天不会取消任务。
- 原生 `part` 事件是完整语义快照：同一消息的新修订替换整份 parts，删除的内容会消失。

插件主运行时重启会丢失正在执行轮次的观察句柄。这类记录明确标记为 `interrupted`，不伪造完成回执、不自动重新提交。重新发送前会核对原生会话是否仍在执行；关闭时会请求取消并等待实际空闲。SDK 当前没有重新附着到已有执行流或直接向生成中模型注入指令的接口。

插件不会在结果完成后自动恢复主模型生成；主聊天通过 `wait_agent` 工具获取结果并继续自己的轮次。

## 检查和打包

在仓库根目录运行，使用工作区已有的 TypeScript 和 fflate 开发依赖：

```powershell
node plugins/packages/external/subagents/scripts/check.mjs
node --test plugins/packages/external/subagents/tests/manager.test.mjs plugins/packages/external/subagents/tests/boundaries.test.mjs
node plugins/packages/external/subagents/scripts/pack.mjs
```

打包只发出插件 JavaScript 并创建 `dist/subagents.toolpkg`，不构建 Rust 或 Flutter。归档包含 manifest、README 和所有运行时 JS，相对模块路径保持不变。

打包检查使用归档内实际发出的模块执行注册，并要求运行时相对导入显式携带 `.js` 扩展名。工具子包的默认启用声明位于 `src/tools.ts` 的 METADATA 中；容器本身仍默认关闭。

## 与 Goal Mode 一起运行 CLI 验证

仓库根目录使用已有 CLI 和项目 venv 运行：

```powershell
.venv/Scripts/python.exe tools/tests/subagents_goal_cli_smoke.py --binary apps/cli/target/debug/operit2.exe --scenario wait
.venv/Scripts/python.exe tools/tests/subagents_goal_cli_smoke.py --binary apps/cli/target/debug/operit2.exe --scenario lifecycle
```

脚本创建独立身份、配置、运行目录和工作区，启动真正的 CLI `link session`，通过实际插件导入和模型工具调用执行测试。模型请求只发送到测试进程的 loopback HTTP/SSE 服务；插件、工具、持久化、子聊天和原生完成回执都由真实 Core 执行。脚本不调用 Rust 或 Flutter 构建；验证宿主改动时需要先更新被测 CLI。模型使用 loopback 协议夹具，不访问真实外部模型服务。

`wait` 场景检查：

- 插件导入后的 `sourceType` 为 `EXTERNAL`，并发现全部五个工具。
- Goal 与 Subagents 的 `above_input` 贡献同时存在，Goal 排在子任务面板上方。
- Goal 设置、暂停、恢复、模型请求前的目标注入，以及父子会话目标隔离。
- 主模型实际创建子代理，默认等待真实的 30 秒超时，再次等待同一轮次得到原生 `completed` 回执；只产生一次子任务模型请求。
- 父子消息落库，Goal 独立模型复核后变为 `completed`，最后清除目标。

`lifecycle` 场景还检查同一子会话的后续指令、运行中打断、新轮次等待、列表、关闭已完成任务和取消并关闭运行中任务。失败会直接保留错误，不跳过步骤。脚本会打印诊断目录，内含 CLI 命令与响应、模型请求、宿主日志，以及已执行成功的场景观测。

2026-10-23 验证结果：严格 TypeScript、26 项插件行为测试、实际归档加载检查通过；包含插件、Compose 和调用上下文回归在内的 139 项 Node 测试通过；真实 Rust/JavaScript 宿主的 11 项 Compose 测试通过（另外 4 项性能测试按原声明忽略）。更新后的 CLI 完整 `lifecycle` 场景通过，包含默认 30 秒超时与继续等待、同会话后续对话、运行中打断、关闭已完成与运行中任务、持久化历史，以及 Goal 的独立完成复核，共 16 个主模型步骤、5 次子模型请求。

本次修复同时涉及宿主运行时：异步宿主请求恢复时保持所属调用的 ID 与 runtime 一致，调用结束时释放自己的状态监听；Compose 首次渲染后的 effect 状态变化会持续推送到所属页面。应用需加载更新后的宿主并重新导入插件包。Flutter 实机布局、真实外部模型和其他平台尚未验收。

2026-10-23 统一聊天控件验证：子代理详情已移除自定义消息渲染，复用宿主 `AiChat`。26 项插件行为测试及其所属的 139 项 Node 回归通过，10 项 Flutter 控件与布局测试通过，涵盖 DSL 参数传递、局部会话绑定与切换、同会话更新不重复绑定和绑定错误展示。

2026-10-23 输入区域验证：Classic 的 `above_input` 槽位已移到输入气泡外上方；没有处于 `starting`、`running`、`closing` 状态的任务时，面板及历史入口保持零高度，已打开的详情可继续查看。139 项 Node 回归及 10 项 Flutter 测试通过，覆盖全部终态、展开历史后收起、详情保留、聊天切换，以及普通与悬浮 Classic 输入布局和原有弹窗定位。

2026-10-23 对齐验证：标题、状态图标、历史与创建按钮，以及任务行内的名称、状态和关闭按钮显式垂直居中；面板使用统一的左右留白，宿主槽位跟随输入气泡的悬浮边距，空内容仍为零高度。14 项 Flutter 测试通过，直接将插件源码输出交给 Flutter DSL 渲染器，检查 320/700 宽度、1/1.5 倍字体、长名称及 Classic/Agent 普通与悬浮布局的实际坐标；139 项 Node 回归通过。

在 `apps/flutter/app` 运行对齐回归，测试使用工作区现有的 Node 与 TypeScript 依赖导出插件节点：

```powershell
fvm flutter test --no-pub test/subagents_composer_alignment_test.dart test/chat_input_popup_geometry_test.dart
```

2026-10-23 保留聊天修复：已关闭任务的 `closed` 状态不再永久阻止其保留聊天创建新任务；关闭中及控制操作期间仍拒绝并发创建，旧任务仍禁止继续发送。新增同运行时和重启后的回归，28 项插件行为测试及包含它们的 141 项 Node 回归通过。真实 CLI 的 Goal 联动 `lifecycle` 场景通过，并补充在已关闭子会话中直接发送用户消息后创建、等待、关闭新代理的 4 个模型步骤，总计 16 个原主模型步骤、4 个保留聊天步骤和 6 次子模型请求；模型使用 loopback 服务。

2026-10-23 输入辅助面板统一：Classic 与 Agent 的待发送队列共用原生组件，均位于输入气泡上方；队列、子代理和目标卡片使用相同的输入框外边距，插件不再叠加横向留白。标题统一使用跟随全局字号的 `labelMedium`，正文使用 `bodySmall`，标题内边距为横向 12px、纵向 4px，标题按钮为 32px、图标 20px，任务行操作为 28px、图标 16px。DSL Card 的 `themeSurface` 由 SDK 源定义生成，直接接入宿主 `OperitGlassSurface`；外层使用 `card` 质感，条目使用 `control` 质感，透明与磨砂设置跟随全局主题。保留默认单行、按需展开和空状态零高度。36 项 Node 测试、46 项 Flutter 测试及修改文件的 Dart 静态检查通过，覆盖真实队列与插件的实际边缘、间距、字号、按钮尺寸、明暗主题、透明开关、320/700 宽度、字体放大，以及两种输入模式的普通与悬浮布局。
