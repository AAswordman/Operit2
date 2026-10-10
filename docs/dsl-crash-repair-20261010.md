# DSL 崩溃与角色卡安装包修复（2026-10-10）

## 最新日志的直接原因

附件 `0e24028f-563e-41ee-aee5-5f89a73dfb21/已粘贴的文本.txt` 中，20:30:00 的首个业务错误是重复注册 `com.operit.daily_life`；随后 `operit-toolpkg-notifications` 和 `operit-flutter-call` 在 `JsEngine.rs` 初始化处 panic，角色卡页面得到 `Runtime task result channel closed`。

设备实际同时安装了 `daily_life.js` 和 `daily_life.toolpkg`。旧 JS 的 METADATA 名称是 `daily_life`，新 ToolPkg 的容器是 `com.operit.daily_life`，其子包名称也为 `daily_life`。真正发生冲突的是子包名称，旧诊断却只报告新容器名称。相同旧 JS 还残留在迁移前的 `extensions/packages` 中。

包扫描严格拒绝重名，登记目录因此处于错误状态。`AIToolHandler::for_toolpkg_execution_context` 调用 `packageRegistryReadiness().require_ready()`，把这个错误返回给 JS 引擎初始化。旧 `JsEngineWorker::new` 用 `expect` 接收 Host 初始化结果，将正常可传递的业务错误变成线程 panic。该错误链与之前 Acorn 的 QuickJS 栈限额问题不同。

## 代码修复

- JS 引擎构造、SDK 引擎工厂、执行上下文获取和租约获取返回 Result。调用者传递初始化失败；失败时不登记引擎、不占用租约。并发创建时，落败的引擎在注册锁外销毁。
- 独立工具执行池在获取槽位时创建引擎，创建失败由本次工具调用返回。保留四个并发槽位与已有引擎复用。
- 消息处理任务在自己的任务范围处理获取失败并关闭本次流；取消和创建交错时释放创建出的引擎。
- 外部 ToolPkg 冲突诊断报告真正冲突的成员及已登记的所有者；冲突验证完成前不移除已有包。
- 已导入内置资源消失时，旧同步逻辑只删导入记录，却保留可执行源文件。现在根据记录指纹核对字节，归档旧代码，撤销设备安装登记，保留配置、设置与业务数据。修改过的源文件明确报错，不能自动当成原资源退役。
- 中间渲染任务及其调度 Promise 的拒绝有明确的错误接收者：当前 action 的最终结果携带失败，分离任务通过现有 JS 调用错误通道报告失败，避免悬空的 Promise rejection。
- 角色卡编辑器渲染移动到 `src/ui-editor.ts`；Host 文件保留路由、IPC 和生命周期注册。删除未使用的 `installService` 别名和私有 `service-runtime.snapshot` 导出，更新错误的存储说明。旧 CLI 代理已不在当前插件源码中。
- 侧栏测试夹具在首次渲染、输入刷新及 action 中保持同一执行上下文标识，并停止向会话回传过期 state/memo。

CoreLink 未改；现有插件 JS 写法保持不变。

## 本机安装目录处理

针对已经丢失原导入记录、无法由新同步流程识别的当前旧文件，实际执行了可撤销的归档：

`C:/Users/12809/AppData/Roaming/Operit2/runtime/identities/identity-1788930179675-68462c82/extensions/device/retired-packages/20261010-dsl-crash-repair`

归档包含设备目录的旧 `daily_life.js`、迁移前目录的同字节旧文件、旧独立插件的安装登记，以及修改前的角色卡代码归档。两份旧源文件均在移动前核对 SHA-256：

`9a3e92ea76b9a7c14395a615d56d7bcfe20da4159721cc8123372563260b7032`

活动目录保留 `daily_life.toolpkg`。角色卡安装包已用本次通过验证的插件归档替换，并核对与仓库内置归档一致。代码包复制使用临时文件和原子替换；没有操作插件配置、数据库或业务数据目录。

部署指纹及备份位置记录在 `target/runtime-deployment-repair.json`。归档构建会改变 ZIP 指纹，因此实际部署以该记录及字节核对为准。

## 验证

| 验证范围 | 结果 | 日志 |
| --- | --- | --- |
| 角色卡 Node 全套 | 409/409 | `target/character-cleanup-tests.log` |
| DSL 状态、导航、WebView、真实插件屏幕 | 26/26 | `target/compose-crash-tests.log` |
| 原生 JS bridge 全套，串行 | 74/74 | `target/js-runtime-crash-tests.log` |
| SDK 执行上下文和租约 | 10/10 | `target/engine-lease-tests.log` |
| 安装与同步 Store | 20/20 | `target/extension-retirement-tests.log` |
| 源指纹与真实子包冲突诊断 | 2/2 | `target/bundled-retirement-tests.log` |
| Flutter 侧栏、菜单、附件、目录刷新、错误生命周期与 DSL 布局 | 94/94 | `target/flutter-crash-tests.log` |
| Rust runtime 及测试检查 | 通过 | `target/runtime-factory-check.log` |
| 工作区 diff 空白检查 | 通过 | `target/dsl-crash-diff-check.log` |

新增真实 Host 用例重现登记目录认证失败：构造返回 Initialization 错误，修正 Host 目录状态后，同一个 Host 仍可创建引擎并执行脚本。另一个用例证明初始化失败不留下租约；并发不同所有者创建的用例证明被拒绝的 worker 被销毁。

## 验证边界

没有启动整套 Flutter 应用做人工端到端操作，也没有执行 Flutter 应用打包。已验证源码、测试和安装代码包；Rust 的新错误传播逻辑需要下一次应用构建并重启后进入运行进程。测试结果不能用于宣称所有历史插件或整套 DSL 已无其他问题。
