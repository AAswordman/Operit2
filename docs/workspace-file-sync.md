# 工作区文件同步

同一 Space 内，`/app/workspaces/<目录>/...` 下的普通文件自动双向复制到各设备自己的工作区根目录。同步完成后副本可离线使用。只有配对、尚未加入同一 Space 时不共享文件。

外部挂载目录（`/mnt/...`、`/data/...`）不会被扫描；要随手机带走的文件，需要实际复制到工作区本体。软链接、链接目录、失效链接、Windows junction/reparse point 以及非普通文件跳过，不跟随目标。接收端也不通过已有链接读写文件。

## 实现

- 复用 `runtime_file` 域、SHA-256 Blob、原有向量时钟、成员校验与传输，不增加第二套同步协议。
- `WorkspaceFileSyncStore` 保存本机 `runtime/sync/workspace_inventory.json`，记录已观察到的文件哈希。此清单不会同步给其他设备。
- 工作区 UI 保存、AI 文件写入/应用修改/复制/移动/删除、模板创建触发扫描；这些入口在写入后立即重哈希被改动的文件，同长度覆写也会即时发布。Core 运行时的定时扫描覆盖终端、外部编辑器和其它绕过这些入口的写入：稳定空闲时以指数退避拉长轮询间隔（2 秒起，上限 60 秒），发现变更立即回到 2 秒；关闭 Core 后不后台运行，重新启动时补扫。
- 定时扫描按文件尺寸短路：尺寸与上一轮清单一致的文件直接复用已记录的哈希，不读取、不重新哈希，稳态成本只剩目录枚举；清单记录上一轮全量哈希时间，最长每 5 分钟强制一轮全量哈希，用于发现"同长度覆写"的外部修改（发现延迟上限即该间隔）。
- 文件内容改变生成 upsert，消失生成 delete。重命名表示旧路径删除、新路径写入。删除优先于新增，以支持文件与目录之间切换。
- 收到文件后更新清单，不把远端内容当成新本地修改再次广播。每个接收批次只扫一次工作区；应用整批操作后才推进同步时钟，避免跳过同批偏好等其它数据。
- 接收前先持久化待应用内容，异常退出后重放待应用写入。扫描失败不发布缺失文件删除；工作区根目录不可用时拒绝批量删除。
- 首次加入 Space 后重新登记设备已有的本体文件，避免旧操作日志不再导出而使本地文件无法分享。
- 停止 Space 同步服务时停止定时扫描。

## 行为边界

- 复制文件字节，不是 Git 或文本协同编辑；同时修改同一文件，采用现有同步操作的确定性顺序（时间、设备 ID、序号）决定最终版本，不做逐行合并。
- 目录随文件创建；删除后清理空的子目录，保留工作区根目录。独立空目录、文件权限和执行位不作为同步对象。
- 当前扫描不遵循 `.gitignore`，不排除大型依赖目录。不要把“不想共享”的数据放进本体；外部目录保持挂载即可。
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

绑定工作区的聊天在中英文工作区指南中明确提供同步边界：同一 Space 内的工作区本体
普通文件可自动双向复制，外部挂载、软链接和临时附件不会因此同步。指南也说明同步是
最终一致的，使用前需确认当前执行节点的文件可用；不能仅凭节点已配对或路径一致就
断言文件已到达。自定义系统提示模板未包含工作区占位符时，也会补入这些指南。
