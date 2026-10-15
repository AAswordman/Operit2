# 设备传输与原生配置调用

`Tools.device(nodeId)` 选择明确的设备空间节点。`transport.call` 将一个厂商请求送到原生端点，`transport.open` 用现有 Stream / ReverseStream 打开双向通道。传输支持 null、布尔值、有限数值、字符串、数组、对象，以及任意嵌套的 `Uint8Array` / `ArrayBuffer`；字节在 Link 中保持为 Bytes。

```typescript
const device = Tools.device(nodeId);
const config = await device.transport.call('edge.scene', 'config.read', {v:1});
const channel = await device.transport.open({
    target: 'vendor.link', method: 'input', property: 'output', args: {session: 'one'}
});
```

双向通道提供 `send(value)`、`finish()`、`close()` 和异步迭代。`finish` 结束输入，输出继续读取；`close` 释放两端并唤醒等待。每个 Watch 事件的 `kind` 和 `value` 原样保留。无屏幕、麦克风或 GPIO 的设备同样可以实现这个入口。业务格式由厂商协议解释。

既有小型硬件配置入口继续可用：

```typescript
const status = await device.Edge.execute({target:'edge.status', method:'read'}, {});
const pin = await device.Io.execute({port:'gpio', operation:'read'}, {pin:2});
```

`Edge.execute` 接受原生 `edge.*` 目标和方法；`Io.execute` 当前支持 GPIO 数字输出状态的 read/write。配置请求上限 1024 字节，配置响应上限 4096 字节。通用传输使用底层 Link 的容量限制。

所有入口共用现有节点路由、权限和连接。通道打开及 Call 保留插件调用拦截，数据帧由已打开的端点处理。错误会拒绝 Promise；硬件动作没有额外应用层自动重试。
