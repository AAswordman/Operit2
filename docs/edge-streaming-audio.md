# Edge 流式音频接口

插件在 Core 上消费音频，Edge 负责采集。控制请求使用现有 Link Call，上传使用
现有 Link Push，TCP/串口继续复用 PeerLink 的连接、加密、身份校验、分帧和序列确认。

## 插件 API

| 方法 | 返回 | 行为 |
| --- | --- | --- |
| `Tools.Edge.listAudioInputs(nodeId)` | `{nodeId, inputs}` | 查询指定 Edge 的输入 ID 和支持格式 |
| `Tools.Edge.openAudioInput(nodeId, options)` | `{nodeId, streamId, format, maxDurationMs}` | Core 登记接收会话，Edge 开始采集并打开上传 Push |
| `Tools.Edge.readAudioInput(streamId)` | 音频块或终态 | 单次最长等待 5 秒；同一流只能有一个读取者 |
| `Tools.Edge.closeAudioInput(streamId)` | `{value: true}` | 先撤销 Core 接收会话，再请求 Edge 停止；可重复调用 |

`options` 包含必填的 `inputId`、可选 `format` 和 `maxDurationMs`。
默认格式是 `{encoding: "pcm_s16le", sampleRateHz: 16000, channels: 1}`，
默认时长 60000 毫秒，允许 1 到 300000 毫秒。Host 必须支持请求的准确格式，
不支持时返回错误。协议支持 8000/16000/24000/48000 Hz，1 或 2 声道。

读取结果包含：

- `sequence`：从零开始的块序号；每个非空音频块递增。
- `sampleOffset`：该块之前的采样帧数；双声道的一对样本算一帧。
- `dataBase64`、`byteLength`：原始 PCM；没有 WAV 头。Base64 只用于 JS/JSON 边界。
- `pending: true`：五秒内没有新数据，继续读取；不表示 EOF。
- `done: true, error: null`：正常结束，包括达到设定录音时长。
- `done: true, error: "..."`：采集故障、过载、提前断线或异常关闭。

终态读取前会先排空已接收的音频块。不要把错误终态当作识别成功。
关闭后的 `streamId` 不能再读取或重新上传，关闭本身可重复调用。

```ts
const stream = await Tools.Edge.openAudioInput(nodeId, {
  inputId: "mic0",
  maxDurationMs: 20000,
});
try {
  await recognizer.start(stream.format);
  while (true) {
    const block = await Tools.Edge.readAudioInput(stream.streamId);
    if (block.pending) continue;
    if (block.done) {
      if (block.error) throw new Error(block.error);
      return await recognizer.finish();
    }
    await recognizer.write(decodePcm(block.dataBase64, block.byteLength));
  }
} finally {
  await Tools.Edge.closeAudioInput(stream.streamId);
}
```

带有类型、字节解码、顺序检查和 STT 异常释放的完整适配器见
[`plugins/packages/examples/edge_audio_stream`](../plugins/packages/examples/edge_audio_stream/README.md)。
`recognizer` 由插件作者实现，可以接本地识别器或云端会话。此接口不会替换现有
整段音频 STT 服务。录音循环运行在普通插件调用中，时长必须小于调用超时；
不要放进有六秒超时的 `on_edge_event` 钩子。

## 路由与接收生命周期

Core 运行时拥有独立 `EdgeAudioRegistry`，每次打开分配随机会话 ID，登记指定
Edge 和格式。`CoreEdgeToolRuntime` 把接收端固定为执行插件的 Core，插件不能
指定别的接收端。Edge 控制适配器校验请求的 Space 和请求 Core 来源。

Edge 从共享 `NodeServices` 取得捕获了准入代次的 `CoreLinkSpacePushClient`，
经已有 Space 入口将 Push 定向到接收 Core。接收 Core 可以与相邻入口不同。
音频入口是 `edge.audio.ingress.upload`；它通过 `CoreNodeRouter` 在常规 Space
成员/撤销校验后执行，只接受 SpaceRoute，并核对已登记会话的认证 Edge 来源
和准确格式。普通 Target 或本地 Proxy Push 不能绕过这条入口。

一条 Push 的路由在打开时固定。代次变化、撤销或断线导致失败，不会在重连后
重新发送已经上传的 PCM。每项使用现有 Push 序列和 ACK，音频数据是
`CoreValue::Bytes`；结束项是 `{end: true}` 或 `{end: true, error: "..."}`。
没有结束项的 Push Close/Drop 是错误 EOF。取消的 Push 会在原连接上请求 Close。

每个 Core 最多登记 16 条流，每条队列最多 8 块，每块最多 4096 字节并包含
完整的采样帧。Edge 同时最多采集一条流。满队列会将背压传到 Push，消费者
持续停滞十秒或发送等待 ACK 十二秒会结束并报告错误。关闭先取消接收队列，让被背压阻塞的上传
得以退出，再停止 Edge；Core 注册表释放也会取消其流。麦克风在网络清理前释放。
未关闭的会话在录音时长后留出 30 秒排空时间，随后不可继续读取；过期记录在
后续打开操作中回收。插件必须在 `finally` 中关闭，避免占用有限的会话配额。

## Host 采集适配

`HostManager.withAudioCaptureHost(host)` 注入可选 `AudioCaptureHost`，同时要有
现有 `HostRuntimeTaskSchedulerHost`。未安装能力时返回 `AUDIO_UNAVAILABLE`。
Host 实现 `listInputs()`、`openInput(inputId, format)`；采集会话实现
`read() -> Option<Vec<u8>>`，`None` 表示正常结束。单块必须有界，硬件缓冲区
溢出必须返回错误。采集会话 Drop 必须关闭麦克风并释放资源。

当前实现包含通用能力接口和模拟麦克风测试，尚未实现具体 ESP I2S/ADC 麦克风驱动。
音频接口不依赖显示系统。

## 串口带宽

16 kHz、16 位单声道 PCM 每秒 32000 字节。115200 baud 的 8N1 串口理论上只有
11520 字节/秒，还要扣除加密、分帧和 ACK 开销，因此默认串口配置会明确拒绝
开始这种录音。

现有串口适配的波特率可用 `HostManager.withPeerSerialBaudRate(921600)` 配置；
两端必须配置一致，Host/硬件必须支持该速率。未配置时继续使用 115200。
Edge 对已知串口容量保留至少 25% 余量；这是开始前的容量检查，不能保证真实
硬件、其他共享流或后续中转链路的实时吞吐。每个采集 Host 仍要报告实际过载。
TCP 和串口的连接及分帧协议相同于改动之前。

## 验证范围

- 实际 TCP + 原配对/Space 准入 + Core/Edge Router：模拟采集连续 PCM 的字节一致性、
  错误结束、提前关闭、重复会话拒绝和关闭后其他调用可用。
- 现有 TCP/串口分帧适配器：碎片化 TCP 输入、模拟 UART 上的二进制 Push 和有序 ACK。
- Core 接收队列：来源/格式校验、容量背压、读取消、正常与异常 EOF、运行时释放。
- SDK：Rust 绑定、生成声明和 JavaScript 参数映射。
- 调试网页 + 真实 Core JS 包：`sim-pcm` 按实时节奏采集，校验 1 秒 / 32000 字节、
  录音结束前消费、过载终态/识别器 abort、提前关闭及再次录音。输入设置错误显示后可重试。
  该测试没有配置 STT 模型，验证的是传输和流式识别适配器生命周期。

串口测试使用模拟 UART 字节流，尚未做物理串口吞吐或真实麦克风测试。

独立音频回归命令：`npm run test:audio --prefix tools/esp32-editor`。它复用真实 Core、
C/Wasm 审批及 JS 包的测试夹具，验证上传、逐块消费、过载和关闭后重开；
完整空间审批/重启回归仍运行 `npm run test:space --prefix tools/esp32-editor`。
