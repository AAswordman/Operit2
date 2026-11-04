# 工作区文件同步

同一 Space 内，应用管理的工作区、无工作区时的对话产出目录，以及兼容旧版的附件目录，按照各自的 `.operit/sync.json` 自动双向复制普通文件。同步完成后副本可离线使用。只有配对、尚未加入同一 Space 时不共享文件。

| VFS 入口 | 存储位置 | 相对路径基准 |
| --- | --- | --- |
| `/operitapp/workspaces/<目录>` | `workspaces/<目录>` | 绑定工作区时使用主目录 |
| `/operitapp/outputs/<父对话创建时间>` | `runtime/data/chat_outputs/<父对话创建时间>` | 没有工作区时使用固定产出目录 |
| `/operitapp/data/data/chat_attachments/<chatId>` | `runtime/data/chat_attachments/<chatId>` | 兼容旧附件地址 |

产出目录绑定保存在 `runtime/data/chat_output_bindings/<chatId>.json`，随 Space 同步。目录名采用首次父对话创建时间 `YYYY-MM-DD_HH-mm-ss-SSS`，绑定保存后不按当天重算。子孙对话和消息分支继承已保存的绑定；提升子对话、删除父对话、切换工作区不会改变已有产出目录。创建时间相同允许共用同一目录。产出区域不会被当作用户绑定的工作区。

外部挂载目录（`/virtual/...`、`/data/...`）不会被扫描；要随手机带走的文件，需要实际复制到工作区本体。软链接、链接目录、失效链接、Windows junction/reparse point 以及非普通文件跳过，不跟随目标。接收端也不通过已有链接读写文件。

## 可编辑同步策略

工作区或产出目录首次使用时会补上 `.operit/sync.json`，已有配置保留。默认内容：

```json
{
  "version": 1,
  "ignoreFiles": [".gitignore"],
  "exclude": [
    ".git/", "node_modules/", "target/", "build/", "dist/",
    ".dart_tool/", ".gradle/", "__pycache__/"
  ],
  "maxFileBytes": 16777216,
  "maxTotalBytes": 268435456
}
```

`ignoreFiles` 指定目录内的相对忽略文件；包含 `.gitignore` 时也读取各级子目录的 `.gitignore`。规则采用 Gitignore 语义，支持注释、目录模式和 `!` 反选。`exclude` 是额外的忽略规则。大小单位为字节：默认单文件 16 MiB、每个受管理目录自动同步总量 256 MiB；这两个值控制自动复制，不限制本地原件大小，也不是设备总磁盘配额。`.operit/sync.json` 本身始终参与同步。

超限原件保留在本机，分享小型 `<原文件名>.operit-ref.json`，包含 `nodeId`、准确 VFS `path`、`fileName`、`fileSize` 与可选修改时间。接收节点在申请 Blob 之前检查本地策略，避免先下载大文件再排除。需要原件时读取定位文件并访问来源节点；来源离线时不能把定位文件当作原件。

新增忽略规则、文件变大或超过总量预算只会停止自动复制，**不会作为删除广播**，也不清理其他设备已有副本。取消忽略或放宽限额后，保留的远端最新状态可重新应用。无效 JSON、超大的规则文件或非法规则会报告错误并停止该次扫描，不发布一批错误删除。

## 实现

- 复用 `runtime_file` 域、SHA-256 Blob、原有向量时钟、成员校验与传输，不增加第二套同步协议。
- `WorkspaceFileSyncStore` 保存本机 `runtime/sync/workspace_inventory.json`，记录已观察到的文件修改时间和大小。此清单不会同步给其他设备。
- 工作区 UI 保存、AI 文件写入/应用修改/复制/移动/删除、模板创建触发扫描；这些入口在写入后立即比较文件修改时间和大小，同长度覆写也会即时发布。Core 运行时的定时扫描覆盖终端、外部编辑器和其它绕过这些入口的写入：稳定空闲时以指数退避拉长轮询间隔（2 秒起，上限 60 秒），发现变更立即回到 2 秒；关闭 Core 后不后台运行，重新启动时补扫。
- 文件只按修改时间和大小判同；没有创建时间或定期内容哈希检查。未变化的文件不读内容；同长度外部修改在下一次扫描发现。文件有变化时只读取一次，内容哈希仅用于 Blob 寻址及传输完整性校验。
- upsert 携带源文件修改时间（Unix 微秒），接收端写入字节后恢复该时间，再提交本地清单；原生文件系统和 Web OPFS 通过统一 Host 接口提供该能力。
- 文件内容改变生成 upsert，消失生成 delete。重命名表示旧路径删除、新路径写入。删除优先于新增，以支持文件与目录之间切换。
- 普通文件 upsert/delete 是完整替换状态；历史 skip 可以直接重试，不等待偏好或 SQL 的初始化快照。同一文件的一组待处理状态只落盘最终胜者，成功后才完成对应原文；缺 Blob 或落盘失败仍保留重试和真实错误。
- 收到文件后更新清单，不把远端内容当成新本地修改再次广播。每个接收批次只扫一次工作区；应用整批操作后才推进同步时钟，避免跳过同批偏好等其它数据。
- 接收前先持久化待应用内容，异常退出后重放待应用写入。扫描失败不发布缺失文件删除；工作区根目录不可用时拒绝批量删除。
- 首次加入 Space 后重新登记设备已有的本体文件，避免旧操作日志不再导出而使本地文件无法分享。
- 停止 Space 同步服务时停止定时扫描。

## 行为边界

- 复制文件字节，不是 Git 或文本协同编辑；同时修改同一文件，采用现有同步操作的确定性顺序（时间、设备 ID、序号）决定最终版本，不做逐行合并。
- 目录随文件创建；删除后清理空的子目录，保留工作区根目录。独立空目录、文件权限和执行位不作为同步对象。
- 扫描和接收均遵循同步策略；排除文件继续保留在原设备。已有副本不会因为策略收紧而自动删除。外部目录保持挂载时不参与扫描。
- 双端需要运行支持本功能的版本；没有新增手机后台常驻或绕过系统省电限制的能力。离开电脑前应让两端在线完成传输。

## 验证

```sh
cargo test --manifest-path core/Cargo.toml -p operit-store --test workspace_file_sync
cargo test --manifest-path hosts/common/operit-host-native-storage/Cargo.toml
cargo build --manifest-path apps/cli/Cargo.toml
python3 apps/cli/tests/workspace_sync_session.py --transport tcp
python3 apps/cli/tests/workspace_sync_session.py --transport http
python3 apps/cli/tests/workspace_sync_session.py --transport ws
python3 apps/cli/tests/network_control_session.py --binary apps/cli/target/debug/operit2 --transport tcp
```

CLI 测试使用两套独立身份目录、真实进程与连接，检查首次加入时两端已有文件、配对隔离、二进制/空文件/中文路径、双向修改、重命名/删除、循环链接、双端退出后的修改及重启补同步。

## AI 上下文

中英文提示词明确区分工作区和固定产出区域，并说明 `.operit/sync.json`、大文件定位和外部挂载边界。文件工具以当前目录解析相对路径；终端使用现有 `TerminalHost.resolveWorkingDirectory` 获取各终端的准确目录，提示词不按平台拼宿主或虚拟机路径。

同步最终一致，使用前应确认当前执行节点上的精确 VFS 路径。附件的来源节点不会因本机已有同步副本而被改写；存在副本时可以直接读取，缺失时再访问来源。自定义系统提示模板未包含工作区占位符时，也会补入这些指南。提示词从实际 `FileSyncPolicy` 默认值生成 JSON 示例，解释 `version`、`ignoreFiles`、`exclude`、`maxFileBytes` 和 `maxTotalBytes`、Gitignore 目录模式与反选，并要求 AI 在安装依赖、构建、下载数据集或批量生成文件前读取并补充规则、保留用户已有配置。大量小文件应主动按整个目录排除；被排除的目录在遍历前剪枝，不复制内容，也不逐文件生成定位索引，不能只依赖字节限额，也不应为同步构建产物而放大限额；不得整体排除存放附件和定位文件的 `.operit/`。

## 工作区回退快照

聊天的文件变更快照保存在对应工作区的 `.operit/backups/<聊天作用域>/` 中：
`state.json` 记录最近编辑轮次的文件变更，`objects/` 保存按内容哈希寻址的文件字节。
每个聊天只保留最近一次涉及文件修改的轮次，删除聊天时只清理该工作区内对应的备份目录，
不删除 `.operit/config.json` 或其他聊天的备份。

快照读写通过工作区的 VFS 和文件系统 Host 完成，不另设 runtime 备份目录。
`.operit` 被快照捕获规则排除，避免备份把自身再次备份；工作区本体中的备份文件
按上述普通文件同步规则处理，外部挂载内的备份仍遵循外部挂载不参与同步的边界。
