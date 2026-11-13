# Build 39 / 40 iOS 启动失败排查（2026-11-13）

## 反馈与结论边界

TestFlight 的 build 39 iOS 反馈 `AF6FsuAd4eM62aQDgGTLMLo` 在 2026-11-12 11:40（北京时间）报告“为啥又打不开了”。附件停在“正在准备本地运行时”，报错为：

```text
FATAL_CORE_PANIC: Runtime VFS log files must be writable:
Cannot create log directory '/operitapp/data/logs':
Operation not permitted (os error 1)
```

截图证实启动初始化失败，不能仅据此认定进程闪退。没有取得该设备的 bootstrap 配置和实际系统路径，不能确认下述恢复缺陷就是这条反馈的唯一根因。

## 日志路径没有绕过 VFS

核对 build 39 源码 `2d384ea363cde307beea9f7c10b969db9cdbc26b`、build 40 源码 `0a4c8822d18be17e92595c14d2d7b0eebb02c713` 以及当前源码，日志入口均为：

1. `OperitApplication::newWithContext` 从 `HostManager::vfs()` 构造 VFS，再传给 `AppLogger::configure_log_files`。
2. `AppLogger::ensure_log_file` 调用 VFS 的 `fileExists`、`makeDirectory` 和 `writeFile`。
3. `VisualFileSystem::makeDirectory` 经 `PathMapper::resolve`，将 `/operitapp/data/logs` 映射为实际 runtime 根目录下的 `logs`，再调用原生文件系统。
4. 原生目录创建失败后，AppLogger 的错误格式使用传入的虚拟路径。截图里的 `/operitapp/data/logs` 因此不能证明系统 API 收到了该字面路径。
5. `configure_log_files` 的失败被启动代码的 `expect` 升级为 panic，最终显示为 `OPERIT_RUNTIME_ERROR`，阻断启动。

实际映射示例：

```text
/operitapp/data/logs/operit.log
  -> <当前容器>/Library/Application Support/Operit2/runtime/identities/<身份>/logs/operit.log
```

新增真实文件读写测试覆盖两个身份，以及 `operit.log`、`toolpkg.log` 的创建、追加和读取。

## 已复现并修复的 iOS 容器恢复缺陷

两个发布版本的 `AppleRuntimeChannel.restoreSandboxPaths` 都直接比较保存路径与当前路径的容器父目录字符串。保存路径为 `/private/var/mobile/Containers/Data/Application/<旧 UUID>/...`、当前 home 为 `/var/mobile/Containers/Data/Application/<新 UUID>`（或反向）时，比较失败，旧 UUID 不会更新。

这不是软件 VFS 地址与系统路径直接混用，而是系统路径别名导致旧容器地址恢复被跳过。若实际启动使用了不再可写的旧容器地址，后续正确的 VFS 映射仍会落到错误的系统根目录，符合截图里的权限错误。

修复仅在比较容器父目录时统一 `/private/var/` 与 `/var/`，输出仍使用当前 home，保留原有子目录、身份、配置和已有数据。不同模拟器父目录、App Group 和外部目录不参与替换。

## 验证

- 修复前：直接执行生产 Swift 恢复方法，`/private/var` 到 `/var` 场景失败，保存的旧容器 UUID 原样保留。
- 修复后：`python3 -m unittest tools.tests.test_ios_sandbox_restore` 通过；实际编译并执行生产方法，覆盖 13 个场景，包括双向别名、容器迁移、空响应、目录隔离、数据复制及不覆盖已有数据。
- `cargo test --manifest-path core/Cargo.toml -p operit-vfs --lib`：41 项通过，包括新增日志路径与身份隔离测试。
- `node --test tools/tests/ios_runtime_channel_contract.test.mjs`：通过。

尚未在原反馈设备复现，也未打包或发布此修复。不将这些本地测试表述为真机问题已经解决。
