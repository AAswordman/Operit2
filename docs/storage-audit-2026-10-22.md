# Windows / Android 实际存储核对

日期：2026-10-22。设备文件中的时间按设备日志显示，时区为 UTC+08:00。

本次通过 Windows 本地文件和 Android `adb run-as app.operit` 读取活动 identity，未直接修改设备业务数据。SQLite 使用只读连接；Android 数据库先复制到本机临时目录检查。

## 活动存储

| 端 | Runtime root | Identity |
| --- | --- | --- |
| Windows | `C:/Users/12809/AppData/Roaming/Operit2/runtime` | `identity-1788930179675-68462c82` |
| Android | `/data/user/0/app.operit/files/runtime` | `identity-1792650824844-0f2a91a2` |

以下路径均相对于对应 identity。

## 角色配置与记忆 owner 不一致：已复现并修复代码

读取 `plugin_data/space/com.operit.character_cards/cards.preferences.json` 和 `memory.sqlite`：

| 检查项 | Windows | Android |
| --- | --- | --- |
| 角色卡数量 | 4 | 5 |
| memory.sqlite 中 owners 数量 | 4 | 4 |
| 角色缺少对应 owner | 无 | `character:a4d15b0e-3939-44b3-b430-c1799b9cafdb` |
| characters.sqlite / memory.sqlite 的 PRAGMA quick_check | 均为 ok | 均为 ok |

原始偏好同步操作 `core-1789368866633-b2c5d3c1b2da9994:7191` 明确声明：

```text
operation: alias
source: record_a4d15b0e-3939-44b3-b430-c1799b9cafdb
target: record_9986378c-3e3e-4f83-b769-603e64c7cd32
```

两端偏好文件的应用身份均包含该操作。记忆库已按 alias 收敛，Android 角色配置却保留了 source，因此严格状态检查抛出 `Character state is missing an owner memory space`。

`PreferenceMerge::applyDeclaredOperations` 原实现仅重写历史操作的 root key；`PreferencesSyncedEntry::replay` 只清空重写后的 key，现存文件中的 source key 不再被重放计划覆盖，因而残留。后续命名处理又会将残留记录作为另一张角色卡处理。

修改在同一偏好物化事务内、引用及命名处理前，按已记录 alias 删除当前 catalog 的 source key。使用统一偏好入口，不添加角色卡专用修补、默认 owner、额外同步通道，也不修改原始操作身份和载荷。

回归用例先让接收端落盘 source 和 target，再投递 alias，并重复投递。修改前稳定失败，显示 source 仍存在；修改后 source 消失、target 和 selection 收敛、无关配置保留、归档原始操作不变。

检查结果：`plugin_preferences` 15 项通过；`plugin_storage` 40 项通过、1 项手动基准测试忽略；`preferences_upgrade` 12 项通过。合计 67 项通过。

## 目标模式包：记录、设置与归档文件不一致

两端 `extensions/space/packages/goal_mode.toolpkg` 均存在，大小为 24,943 字节，SHA-256 相同：

```text
854df52be894004b3144adf67c19651a75779271caf69b1169c1fd2f527d3c23
```

两端 `extensions/space/records/package-e7761d4affd739ffd6a12e9b53e572d959e80d6c38dfb73ad848576a9e823280.json` 均记录 `sourceName=""`、`builtin=true`。独立 settings 文件也为 `builtin=true`。

settings 原始同步历史依次包含：

1. Windows 发布 `builtin=false`，installationId 为 `77b458498c92470cb60d4cc7db868916`。
2. Windows 发布启用设置，仍为 `builtin=false`。
3. Android 新实例发布 `builtin=true`，installationId 为 `2d9459b05df04f4c9989587615436352`；其逻辑时间较晚，成为当前值。

Windows 的本地 `extensions/device/manager.preferences.json` 已将 `goal_mode.toolpkg`、`plan_mode.toolpkg` 放入 removed_builtin_package_assets，且保留 goal_mode 的 bundled_external_imports 记录。Android 的同一文件只有 schema 和 installationId，没有这份退休列表。

Space 扫描只接纳 record.sourceName 指向的文件，所以当前 goal_mode 外部归档被跳过。通过 Windows 运行进程的 SDK getter 核实：goal_mode 和 plan_mode 均未注册，getToolPkgLoadIssues 为空。这也说明现有加载问题列表未覆盖这种“文件存在、描述符未引用”的状态。

工作区原有 installBundledPackageSource 修复针对显式重新导入时更新所有权；现存设备记录尚未体现这次转换。该修复不能证明初始化覆盖问题、Android 内置容器消失及既有 Hook 的调用问题均已解决。本次没有修改这些设备记录，也没有自动重新安装插件。

## Android 退出记录

系统 exit-info 记录 PID 17400 于 2026-10-22 15:58:29.751 因 ANR 后的用户关闭而退出；ANR subject 是 HostEventScheduleReceiver 的定时广播。已提取系统 DropBox 堆栈并用现有未剥离 Rust 动态库核对部分 native 地址。

退出分类与插件业务错误分别记录。现有证据尚不足以把这次广播 ANR 归因于 alias 漏洞或 ToolPkg 容器缺失；本次不宣称 ANR 已修复。

## 16:17 新运行与 16:28 实际存储复核

Windows PID 43180 于 16:17:28 启动，16:28 仍在运行并响应。Android PID 19774 仍在运行；系统 exit-info 最新退出仍为 15:58:29 的 PID 17400，没有本次运行新增的退出记录。

两端 cards.preferences.json 现在都只有 4 张卡，memory.sqlite 的 4 个 character owner 与它们逐项对应，quick_check 均为 ok。旧 alias 源 a4d15b0e-3939-44b3-b430-c1799b9cafdb 已从 Android 物化文件消失。

Android 16:17:26 的 service-initialize Hook 仍报 owner 缺失，后续 UI 与定时 Hook 重复同样错误。源码显示 createServiceRuntime 保留同一个初始化 Promise（包括 rejection），而 DatabaseCharacterRepository.open 在返回存储句柄前重建并校验所有配置和记忆数据。启动阶段读到同步未完成的数据会让整个插件永久保留该错误，磁盘随后收敛也无法改变它。

已删除 open 中这段全量预读；初始化只负责打开存储、首次安装和校验当前 metadata。领域读写继续按操作读取当前数据；run 的完整 stateFromRecords 校验保留。memoryBindingMode 枚举检查移到 assertCard，保证按需读取仍拒绝坏记录。真实存储打开、迁移与发布失败仍原样返回。

新增通过真实 API 和文件存储的回归：先保留一个缺失 owner 的 alias 源卡，读取列表、拒绝不完整状态下的修改；删除该源后，同一个插件实例立即读到当前卡并可正常提交。改前失败，改后通过。相关完整 Node 测试 589 项通过，host/browser 严格类型、架构和路由静态检查通过。

角色插件 main.js 与 toolpkg 已重新生成，并将同一个 toolpkg 放入 Core 的既有生产 asset 路径；没有编译或部署 Flutter 应用。两个归档 SHA-256 均为 969bec754ceb7348c6a297513f6f051995e05ac1037740dcb1f0887c68a5403f。

目标模式状态尚未改变：Windows SDK getToolPkgLoadIssues 返回空列表，已注册容器中仍没有 goal_mode / plan_mode。受 SDK 暴露范围限制，不能用 invokeToolPkgPublicApi 直接调用角色接口；该只读请求返回 PLUGIN_SDK_ROUTE_NOT_EXPOSED，不能据此推断插件业务接口成功。

## Watch 流池生命周期

Android 16:17:39–47 的 openCoreStream Watch 多次报 WATCH_NOT_FOUND。检查发现 LocalCoreProxy.coreNodeLocalRuntime 每次构造一个新的 SpaceRuntime，后者独立持有流池。生成的服务分发和 Core 树启动均调用该方法：同一个应用的 Call 与 Watch 可以使用不同服务视图，导致 Call 捕获的 streamId 不存在于 Watch 所读的池。

在 workspace_mode_sync_test 的真实 GoalService 调用中，改为通过同一个 LocalCoreProxy 创建的第二个路由服务视图打开流。修改前稳定复现相同 WATCH_NOT_FOUND，并在等待首事件 10 秒后失败。修复把 SpaceRuntime 移到 LocalCoreProxy 的字段，在应用构造时创建一次，所有服务视图共享该 Arc；没有新增重试、第二条流路由或数据修补。 修改后 `cargo test --manifest-path core/Cargo.toml -p operit-proxy-local --test workspace_mode_sync_test` 通过；同一检查同时覆盖真实插件流首值/变更、ReverseStream、模式状态及插件重载。

## 17:24 / 17:26 新运行：发送被目标模式远端调用阻断

新附件中的 Windows 于 17:24:58 完成运行服务启动；Android PID 20383 于 17:26:24 完成启动。两份附件未出现此前角色 owner 缺失或 WATCH_NOT_FOUND，不能据此证明所有同步场景已经验收。

Android 于 17:26:28、17:47:00、17:47:05 报 goal_mode_bundle 执行上下文没有已注册容器；17:47:17 的 submit_requested Hook 返回 ToolPkg container not found，并通过 Dart zone 记录未捕获错误。进程仍在，系统最新退出是旧 PID 19774 在 17:18:40 被 FORCE STOP，没有本次 PID 的退出记录。

再次只读核对：两端 current_chat_id 均为 009ec71a-3450-49bd-81d1-49c3822c558d。其同步 Binding 操作为 core-1789368866635-838e5f2bc3f04c99a8d056c0dfefd31d:67，generation=1，nodeId 对应 Windows。Goal 的 onChatInput 在 submit_requested 时调用按 chatId 路由的 GoalService.inputEnabled，因此安卓本地存在 Hook 仍可能收到 Windows 缺少容器的错误；不能把这条日志直接解释成安卓本地容器被删除。

Windows 活进程 SDK 再次确认容器列表无 goal_mode / plan_mode，而 getToolPkgLoadIssues 仍为空。两端 goal_mode 描述符和独立 settings 仍是 builtin=true、sourceName 为空；Windows 本地 removed_builtin_package_assets 仍包含 goal_mode.toolpkg 与 plan_mode.toolpkg，bundled_external_imports 又记录 goal_mode 的外部安装。builtin 扫描被本地移除名单排除，外部扫描被共享 sourceName 白名单排除：包文件存在但没有任何扫描路径接纳它。17:47 的发送失败与此安装来源矛盾吻合，不是普通 Watch 等待。

本轮仅核对日志、进程、当前聊天绑定及目标模式安装状态，没有修改实际设备存储或执行自动重装。

## 验收边界

实际设备业务文件未直接修改。设备正在运行的二进制尚未包含本次新修改；当前插件/存储回归通过不等同于新包在 Windows / Android 的运行验收。


## 共享安装的单一状态修复

已确认这不是两端各自安装的合理差异。目标模式和计划模式声明 installation_scope=space；共享描述符和包文件相同，本机 removed_builtin_package_assets 却仍能单独排除 Windows 的内置包，导致文件收敛而运行时缺少容器。

旧模型同时通过 records 描述符和 settings 偏好文件同步同一份安装状态。settings 读取会替换描述符内的整份 settings，连 builtin、installationId、members 都能与 sourceName 分离。历史操作还显示：Windows 的显式外部安装后，Android 首次初始化用较新时间发布 builtin=true 和初始设置，将原有设置覆盖。

修复统一为一份带 schemaVersion 的 ExtensionRecord：sourceName、安装身份、成员和配置作为一个 runtime_file 实体一起同步。删除独立 settings 的写入、同步声明、后台导入和单独刷新分支。版本零记录只迁移一次旧设置；包仅迁移可修改的启用状态和顺序，来源及安装身份保留描述符的权威值。

共享内置包首次登记使用现有文件同步机制中的初始状态，冲突顺序低于明确的安装、修改和卸载；已知安装 tombstone 阻止重新初始化。扫描共享内置包只看共享安装记录或尚未初始化的状态，本机退休名单仅用于设备本地包或无法识别的损坏资源。明确卸载共享内置包不再写本机退休名单。

同时删除 bundled_external_imports 驱动的隐式自动更新、收养、归档和安装记录移除。捆绑资源作为明确安装的来源，不再由每台机器启动时管理已共享的安装。来源未变化的引擎仍由原有 runtimeRevision 机制保留。

当前未构建或部署 Windows / Android 应用，也未直接修改真实设备业务存储。设备当前进程的行为仍属于旧二进制。

验证结果：operit-store 全量单元、集成与文档测试通过；安装激活分类 3 项和偏好合并迁移 6 项通过；shared_concurrency_test 的 15 项通过、1 项原有延迟探针忽略；workspace_mode_sync_test 的真实模式调用、Watch/ReverseStream 与重载检查通过。新增回归覆盖唯一安装实体、晚启动初始化、同步卸载防重建、旧设置单次迁移和本机退休名单不能遮蔽共享模式。

完整集成检查发现旧资源测试等待 Public API 读取 UI 文件，实际 API 已成功返回，当前 mainScript 读取直接使用资产而不经过该 UI 文件。删除该失效断言及专用计数，改为真实 API/IPC 调用前后比较引擎 Arc，验证安装设置更新和重扫保持同一执行域；保留原有独立资源阻塞和会话关闭测试。
