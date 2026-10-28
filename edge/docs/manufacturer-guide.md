# 创建不同形态的厂商产品

一个产品拥有自己的固件入口、设备能力、配置、存储策略和开发工具。只需在 `edge/boards/<product>` 下创建独立项目，通过 `operit-edge-sdk` 使用共享协议。

## 选择能力

离线传感器或小机器人可以只使用默认的 `agent` 模块。实现 `Tool` 接到传感器或电机，注册到 `Agent`；实现 `Backend` 来选择本地规则、本地推理或远程模型。SDK 不会为此启动 Core、插件引擎或聊天数据库。

需要 HTTP 模型时启用 `http-agent`，创建 `HttpBackend`，传入已有的 `HttpHost` 和 `HttpAgentConfig`。配置的是完整 chat completions URL，模型和密钥由产品管理。模型响应只允许每一步调用一个已注册工具，工具返回值会回传模型；`Limits` 和 HTTP 字节预算分别约束保留的上下文与网络缓冲。厂商仍需在工具中校验硬件参数。

需要加入 Operit 空间时启用 `space`。屏幕产品可以额外启用 `scene`；无屏设备无需选择显示尺寸、表情或触摸组件。

## 接到设备空间

参考 [完整无屏产品](../boards/headless_sensor/src/main.rs)。最小装配过程是：

1. 提供 `HostManager`，安装持久化存储、TCP/串口等传输、熵源和任务调度能力。硬件寄存器和操作系统 API 留在本产品的 `board/`。
2. 用 `EdgeNode::headless()` 创建服务边界，再通过 `withCallHandler` 注册自己的服务。SDK 的 GPIO、屏幕和表情服务均可按产品需要注册。
3. 调用 `space::SpaceDevice::assemble(host, device_info, node, limits)`，并持有返回值。这会复用同一个身份、配对服务和空间权限投影。
4. 调用 `device.listen(host_config)` 启动实际传输。产品决定监听地址、发现方式和是否允许自动端口。
5. 通过 `device.services.peers()` 完成发现、发起配对和六位验证码确认。二维码或地址仅传递连接坐标；确认由用户在产品 UI 或控制台完成。
6. 调用 `device.space.requestDeviceSpaceJoin(peer_node_id)` 申请加入空间。审批通过后刷新请求；配对与空间授权分别验证，不能把“TCP 已连通”当成“已获授权”。

成员身份、审批和持久化由共享 runtime 维护。自定义服务通过原有 Link 路由进入设备，路由检查空间、调用方和能力；不要另开绕过这些检查的业务 HTTP 入口。

## 两种产品

[ESP32](../boards/esp32/README.md) 包含原生 UI、独立聊天、空间聊天、语音、板卡适配与调试工具。模拟器编译产品共用的 Rust 模块，并通过真实 TCP 连到桌面 Core。

[无屏传感器](../boards/headless_sensor/README.md) 只暴露传感器和 agent 服务，控制台完成配对与加入。它复用同一 SDK，不带 ESP32 UI、表情或 scene。桌面演示传感器返回固定读数，厂商替换 `Tool::call` 即可接入真实 HAL；配对和空间加入使用生产协议。

Linux 机器人 [PB_SBC01_H3](../boards/pb_sbc01_h3/README.md) 选择运行完整 Core。只有需要本机执行完整 Core 的产品才承担对应依赖。

## 资源与构建边界

SDK 默认无外部依赖；`space` 的正常依赖中不允许出现完整 Core、SQL、JavaScript、本地模型引擎或通用桌面 HTTP 客户端。`edge/tools/check-sdk-dependencies.py` 会检查默认和可选功能的依赖图。

ESP32 模拟器固定 256 KiB Rust 堆，主线程、异步任务、阻塞任务分别使用 16/24/20 KiB 逻辑栈和保护页。超过预算会真实失败。桌面操作系统、C/Wasm UI 和网络系统库拥有各自内存，这些数值不能代表整板 RAM 或闪存；修改固件后仍需 ESP-IDF 构建与硬件验收。

厂商扩展能力时应明确网络响应、历史记录、工具输出、并发和消息帧上限。优先通过 Host 和本产品服务提供能力，保持 SDK 与板卡、UI 和模型引擎解耦。
