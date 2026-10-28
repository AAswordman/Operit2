# Edge 厂商接入

`edge/` 收纳厂商 SDK、接入文档和完整产品。产品形态由厂商决定；设备空间、配对、加密连接和权限使用 Operit 的共享协议。

```text
edge/
  sdk/                       手写 Rust SDK，独立 Cargo workspace
    src/                     厂商入口、本地 agent、可选 HTTP 模型适配
    contract/                设备服务和聊天展示协议
    node/                    设备服务分发和共享 Peer runtime 适配
    proxy/                   手写设备服务客户端
    scene/                   可选的有界场景渲染
  docs/                      接入、架构和验收说明
  boards/
    esp32/                   ESP32 固件、硬件适配、UI、编辑器与真实 Rust 模拟器
    headless_sensor/         无屏产品：本地 agent、传感器服务、配对与空间加入
    pb_sbc01_h3/             Linux 机器人产品，运行完整 Core
```

从 [厂商接入指南](docs/manufacturer-guide.md) 开始。需要屏幕、独立聊天和设备调试的产品参考 [ESP32](boards/esp32/README.md)；无屏网关或传感器参考 [headless_sensor](boards/headless_sensor/README.md)。PB_SBC01_H3 选择运行完整 Core，这是该产品的选择，不是 SDK 的要求。

SDK 默认没有第三方依赖。`space`、`http-agent`、`scene` 分别提供设备空间接入、模型工具调用和可选显示能力。SDK 不要求 SQL、JavaScript、工具包执行器或本地模型引擎。

运行完整模拟器验收：

```sh
EMSDK=/path/to/emsdk npm run test:device-budget --prefix edge/boards/esp32/editor
```

该入口重建 C/Wasm、Rust 模拟器和本机 CLI，再检查固定堆/栈预算、独立聊天、空间加入、双向消息和重启。Emscripten 与桌面构建环境见 [编辑器说明](boards/esp32/editor/README.md)。验收范围与测量方法见 [验收记录](docs/acceptance.md)。
