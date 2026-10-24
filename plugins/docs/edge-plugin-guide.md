# 硬件 ToolPkg 接入

厂商提供原生固件和安装在手机、桌面等宿主上的 ToolPkg。固件负责硬件运行、屏幕交互和自身业务；ToolPkg 负责设备配置与资源编辑。配置保存到设备后，退出、禁用或卸载配置包不影响设备继续运行。

Core 对插件提供通用节点传输。厂商协议使用结构化数据和字节数据，自行决定指令、资源、音频格式及处理方式。TTS/STT 可以在设备本地实现，也可以使用通道交给具备处理能力的宿主插件。聊天保持现有原生请求和同步链路。

设备在空间中的职责见 [Edge 架构](../../docs/edge-architecture.md)。
配置器、持续处理插件和固件是不同的运行主体，接口可用性以目标设备声明为准。

## 通用传输入口

```typescript
const device = Tools.device(nodeId, {timeoutMs: 120000});
const capability = await device.transport.call('vendor.device', 'capabilities', {});
const channel = await device.transport.open({
    target: 'vendor.device',
    method: 'input',        // ReverseStream：Push
    property: 'output',     // Stream：Watch
    args: {session: 'vendor-session'}
});
try {
    const receiving = (async () => {
        for await (const event of channel) {
            // event.kind 保留 Snapshot / Changed / Delta / Completed。
            // event.value 可以包含 Uint8Array。
            handleVendorValue(event.kind, event.value);
        }
    })();
    await channel.send({command: 'resource', bytes: new Uint8Array([0, 128, 255])});
    await channel.finish(); // 输入 EOF，仍可读取输出
    await receiving;
} finally {
    await channel.close();
}
```

`call` 复用 Call；通道的输出和输入复用 Watch、Push 以及已有 Stream / ReverseStream 语义。端点由固件提供，同一 `args` 传给两端，厂商可以用自己的 session 字段关联方向。接收和发送可同时等待，每个方向只允许一个待完成操作，缓冲有界。`timeoutMs` 限制请求、打开和写入操作；等待输出不会因端点空闲而超时。通道持续到显式关闭、取消、端点结束或运行时释放。

`device.cancel()`、AbortSignal、JS 引擎释放和 Core 释放都会清理相应通道。句柄属于创建它的引擎和目标节点。所有操作使用当前 Core 树的现有 Router，沿用配对、空间成员和 NetworkControl 检查。

## 小型配置入口与数据边界

```typescript
const device = Tools.device(nodeId);
const status = await device.Edge.execute({target:'edge.status', method:'read'}, {});
const pin = await device.Io.execute({port:'gpio', operation:'read'}, {pin:2});
```

`Edge.execute` 接受原生 `edge.*` 目标和方法；`Io.execute` 当前支持 GPIO 数字输出的
read/write。小型配置请求上限 1024 字节，响应上限 4096 字节；通用 `transport`
使用 Link 容量限制。传输支持结构化值及任意嵌套的 `Uint8Array` / `ArrayBuffer`，
二进制在 Link 中保留为 Bytes。

Call 和通道打开沿用插件拦截、共享路由、当前成员资格和 NetworkControl 检查。
硬件动作提交后，取消等待不能撤销动作；错误拒绝 Promise，没有额外应用层自动重试。

## 设备本地交互

屏幕点击、场景命中和本地动画由原生设备应用处理，不等待配置 ToolPkg 的点击回调。
需要送到其他端点的数据才使用现有 Stream / ReverseStream；厂商协议决定事件内容。
配置和资源保存到设备后，关闭、禁用或卸载配置器不会停止设备行为。

## 硬件配置示例

`com.operit.edge_pixel_pet` 是设备配置器。它查询 `edge.scene` 能力，校验并上传素材，然后保存设备显示配置。公开工具是 `inspect`、`apply_configuration`、`clear_configuration`。点击和动画由设备本地处理。

原生配置操作也可通过 `Tools.device(nodeId).Edge.execute({target:'edge.scene', method:'config.read'}, {v:1})` 调用。协议和资源格式见 [edge-scene.md](edge-scene.md)。

## 宿主语音处理

TTS 合成和 STT 识别继续使用宿主已有实现。处理插件可以调用：

```typescript
const recognized = await Tools.Stt.transcribe({
    audio: Array.from(wavBytes), fileName: 'capture.wav', contentType: 'audio/wav'
});
const speech = await Tools.Tts.synthesize({text: recognized.text});
// speech.segments 按合成顺序包含 audio 字节与 contentType。
// 插件自行定义消息格式，再通过 channel.send 发送到设备播放端。
```

省略 `configId` 时使用当前宿主配置；显式传入时只使用该配置。
这两个接口执行宿主计算，传输仍只使用通用通道。
完整处理插件见 [device_speech_processor](../packages/examples/device_speech_processor/README.md)。
配置包与处理插件分别负责编辑设备配置和执行语音任务；设备本地实现不需要处理插件。
