# 插件 WebSocket API

ToolPkg API `2.0.0` 提供 `Tools.Net.webSocket(request)`，用于通过统一网络宿主打开文本 WebSocket。请求和响应由 Rust SDK 声明，TypeScript 定义由 SDK 生成器输出。

## 操作

```ts
const opened = await Tools.Net.webSocket({
  op: "open",
  url: "wss://example.com/events",
  connect_timeout_seconds: 20,
});
if (opened.op !== "open") throw new Error("Unexpected socket operation");
const handle = opened.handle;

const read = await Tools.Net.webSocket({ op: "read", handle });
if (read.op !== "read") throw new Error("Unexpected socket operation");
for (const event of read.events) {
  switch (event.type) {
    case "opened":
      await Tools.Net.webSocket({ op: "send", handle, text: "hello" });
      break;
    case "message":
      console.log(event.text);
      break;
    case "error":
      throw new Error(event.message);
    case "closed":
      console.log("Connection closed");
      break;
  }
}
await Tools.Net.webSocket({ op: "close", handle });
```

`open` 返回引擎拥有的句柄；只有收到 `opened` 后才可发送。`read` 按顺序取走当前队列中的事件，可使用宿主定时器持续读取。业务认证、消息编码、心跳和服务端指定的会话恢复由调用方处理。

| 请求 | 成功响应 |
| --- | --- |
| `{ op: "open", url, connect_timeout_seconds }` | `{ op: "open", handle }` |
| `{ op: "read", handle }` | `{ op: "read", events }` |
| `{ op: "send", handle, text }` | `{ op: "send" }` |
| `{ op: "close", handle }` | `{ op: "close" }` |

事件为 `opened`、带 `text` 的 `message`、`closed`，或带 `message` 的 `error`。连接错误通过事件交付；无效句柄、参数和发送错误使当前调用失败。终止事件仍可读取，之后应显式 `close` 释放句柄。

## 所有权和容量

- 必须有当前启用插件的真实引擎所有者。
- 句柄只在创建它的引擎内有效。UI 和工具应通过 main runtime IPC 访问常驻连接。
- 一个引擎最多拥有 16 个连接。引擎释放时关闭所有未释放的连接。
- 每个接收队列最多保存 256 个普通事件、4 MiB 文本；超出限制会终止连接并保留明确错误事件。
- 单条发送文本最多 4 MiB。连接超时范围为 1..120 秒。
- 接口接受 `ws` 和 `wss`，TLS 校验保持开启，使用标准 WebSocket 握手，不暴露平台专用握手参数。

## 宿主实现

`WebSocketHost` 在既有二进制发送方法之外提供 `sendWebSocketText`。共享原生 HTTP 宿主通过同一异步网络执行器发送 WebSocket 文本帧，Android、iOS/macOS、Windows、Linux 和 OpenHarmony 的分发器转交这一能力；浏览器宿主使用浏览器 WebSocket 的文本发送与接收。

插件侧调用沿 `Tools` → 普通工具管线 → 引擎会话 → `WebSocketHost` 执行，复用工具准入、错误交付和插件身份，不建立额外网络执行通道。
