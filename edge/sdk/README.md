# operit-edge-sdk

稳定的厂商入口是本目录的 `operit-edge-sdk` crate。接口手工维护，不要求厂商使用 Core 的生成客户端。

```toml
[dependencies]
operit-edge-sdk = { path = "../../sdk", features = ["space"] }
```

| 功能 | 用途 | 默认启用 |
| --- | --- | --- |
| `agent` 模块 | 注册本地工具、有界 agent 执行循环、可替换模型后端 | 是，无第三方依赖 |
| `space` feature | 配对、加密连接、空间审批、权限、设备服务路由 | 否 |
| `http-agent` feature | 使用 Host HTTP 流的 OpenAI 兼容模型工具调用 | 否 |
| `scene` feature | 有屏产品的资源和场景渲染 | 否 |

`SpaceDevice::assemble` 将厂商服务接到共享配对与空间服务上。`EdgeNode::headless` 可以只注册自定义服务；显示、GPIO 和表情均不是必需能力。产品提供 Host 适配器，SDK 不选择 MCU、操作系统、网络库或 UI。

`agent::Agent` 只执行产品显式注册的工具，限制输入、输出、上下文和执行步数。`Backend` 可以对接本地推理、HTTP 模型或经过认证的 Core 服务。可选的 `http_agent::HttpBackend` 提供 HTTP 模型适配，包含工具调用结果回传、响应字节限制、超时及取消时的连接清理。物理动作的参数、范围与权限由产品工具实现检查。

`device`、`contract`、`proxy`、`host`、`link`、`runtime`、`storage` 为共享实现的命名入口。产品首先使用 SDK 装配接口；底层模块用于需要精细配置的场景。内部子 crate 继续复用 Core 的协议、存储与权限实现，避免维护两套配对协议。

```sh
cargo test --manifest-path edge/sdk/Cargo.toml --workspace --all-features
python3 edge/tools/check-sdk-dependencies.py
```

这不是 `no_std` SDK。启用空间接入需要 Rust `std`、持久化、熵源和异步 Host；ESP32 产品使用 ESP-IDF。默认零依赖不意味着启用网络与认证也零成本。见 [厂商指南](../docs/manufacturer-guide.md)。
