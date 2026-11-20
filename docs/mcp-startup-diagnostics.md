# MCP 启动失败诊断

MCP 启动顺序为注册服务、`initialize`、`notifications/initialized`、分页读取
`tools/list`。远程服务使用 Host 的流式 HTTP 能力；本地服务使用 Host 的托管进程能力。
所有启动阶段共用用户设置的启动截止时间。

应用后台自动启动和手动 `mcp start` 可能同时到达。远程连接配置相同的重复注册必须
保留原注册的启动锁和当前会话，两个调用在已有锁上串行完成初始化；重新注册时取消
旧会话并换锁，会使另一调用收到 `MCP service stopped while connecting`。只有端点、
传输或鉴权配置改变时才替换连接，显示描述改变不影响连接。该竞态已通过完整应用
初始化后立即执行命令的探针复现，尚不能据此判定投诉设备也触发了同一竞态。

`MCPStarter` 通过 `StartStatus` 返回具体错误。`mcp start <id>` 必须将失败状态中的
原因写入返回的错误；写入临时 stdout 后再返回通用错误会丢失原因，因为 Core 命令失败时
直接返回 `Err`，不会返回 `CoreCommandOutput`。

修复后通过完整初始化的应用执行 `mcp start`，故意使用格式错误的 Token，实际返回
`MCP start failed: command-invalid-token` 以及
`Remote MCP HTTP request failed with status 400: bad request: Authorization header is badly formatted`。
网络失败的另一次执行也保留了 `tls handshake eof`，没有再被通用错误覆盖。

扩展详情页在本次页面状态中保留手动启动、重启或停止失败的错误，提示消失后仍可查看；
下次生命周期操作成功后清除。该提示不作为连接成功与否的额外判据，也不跨页面重建持久化。

远程错误通常包含 HTTP 状态码和最多 4096 字节的服务端响应摘要，或协议解析、网络及
截止时间错误。排查投诉时应同时记录应用版本、传输类型、端点和完整错误。详情中的
`Headers: Authorization` 只表明配置包含这个名称，不能证明凭据内容和有效性。

GitHub 官方远程端点为 `https://api.githubcopilot.com/mcp/`。没有凭据的请求可以验证
端点的鉴权错误响应，不能验证用户凭据或完整 MCP 握手。不要把无凭据探测的 401
归因到用户的 Token。

回归检查：

```sh
cargo test --manifest-path core/Cargo.toml -p operit-tools --lib remote_tests::
```

远程传输的 12 项确定性测试通过，包含相同配置并发注册、保留就绪会话，以及凭据
变更后替换连接。详情页错误展示：

```sh
cd apps/flutter/app
fvm flutter test test/extension_catalog_manager_test.dart --plain-name 'MCP startup error stays visible until a successful retry'
```

此组件测试覆盖错误保留和成功重试后的清除，不代表真实 Android 设备或 GitHub 凭据的
端到端验收。网络集成测试位于 `apps/cli/tests/mcp_remote_smoke.rs`。其中
`authenticated_github_remote_mcp` 使用环境变量 `OPERIT_MCP_GITHUB_TOKEN`，覆盖
Bearer 字段、显式 Authorization Header、工具发现和连续两次 `get_me` 调用。
默认跳过，需要有效凭据和网络：

```sh
cd apps/cli
cargo test --test mcp_remote_smoke authenticated_github_remote_mcp -- --ignored --exact
```

## 2026-11-20 实际验证

使用自动化浏览器核对 GitHub 官方远程 MCP 文档，并使用本机已有的有效 GitHub 凭据，
通过命令行运行链接当前生产代码的独立 Rust 探针。探针使用 `MCPBridge`、
`MCPBridgeClient`、`MCPLocalServer`、`MCPStarter` 和公共 Native HTTP Host，
未使用模拟 MCP 服务，也未将凭据写入源码、命令行参数或报告。

| 配置 | 启动耗时 | 工具数 | 实际调用 |
| --- | --- | --- | --- |
| Bearer 字段，空 Headers | 3291 ms | 46 | 连续两次 `get_me` 成功 |
| 显式 Authorization Header | 2758 ms | 46 | 连续两次 `get_me` 成功 |
| 保存并重新读取 Bearer、空 Headers，默认 10 秒截止时间 | 4409 ms | 46 | `get_me` 成功 |
| 故意使用格式错误的 Token | 327 ms | — | 服务端拒绝，HTTP 400 和具体原因保留 |

以上为重复注册竞态修复后的同一轮实际结果；工具响应同时检查传输成功、内容非空和
工具级 `isError` 未置位。

另一次运行遇到本机 TLS 握手提前断开，没有收到 HTTP 响应。它只能说明该次网络请求
未完成，不能推断投诉用户的 Token 无效或归因到其设备。生产代码未增加重试或降级。

完整 CLI 的重新构建被无关的本地推理模块 llama.cpp CMake 解压重命名错误阻断，
因此以上结果来自当前源码的独立生产链路探针，不代表完整 CLI 可执行文件或 Android
设备验收。截图无法确定投诉设备的应用版本、凭据有效性和底层错误；这些仍需该设备
实际运行结果确认。
