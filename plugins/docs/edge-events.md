# 通用 Edge → Core action 事件（v1）

Core 的 ToolPkg 插件通过静态类 `Tools.Edge.execute` 调用固件原生动作。
反向事件统一注入 Core 的 `edge_event`，在原 ToolPkg main 运行时执行固定导出
`on_edge_event(event)`。屏幕触摸只是一个生产者；Core 不解析触摸／租约专属 schema。
传感器、GPIO、串口、按键、任务完成等都使用相同 envelope 和 action 分发。

## 两个方向

```text
Core ToolPkg → Tools.Edge.execute(nodeId, {pluginId, action}, args) → Edge 原生动作
Edge 事件源 → 现有 Space/Binding 路由 → chatEdgeEvent → on_edge_event → 插件 action handler
```

通用事件是输入数据，不是任意 RPC／JavaScript 调用通道。每个固件服务仍需自行注册
允许的原生动作及事件来源；此接口不自动开放串口端口、配对或扩大授权。

## 通用信封

Edge 通过已有 Space 客户端调用 `$core.internal / chatEdgeEvent`：

```json
{
  "chatId":"<现有 Binding>",
  "nodeId":"<本 Edge 身份>",
  "packageName":"com.example.environment",
  "payload":{
    "v":1,
    "source":"sensor.environment",
    "stream":"<本次订阅或启动流 ID>",
    "events":[{"seq":1,"action":"temperature.changed","data":{"celsius":23.5}}],
    "next":1,
    "lostBefore":0
  }
}
```

- `source` 是固件服务标识，1–108 ASCII 字符；`action` 是业务 action，1–64 字符。
- `stream` 是生产者流标识，1–128 字符；新订阅／重启必须使用新标识。
  上述标识仅接受 `[A-Za-z0-9_.-]`，不接受地址、token、路径或函数名。
- 每条 `data` 必须为 JSON 对象，具体 schema 由 action handler 校验；未知字段在信封层拒绝。
- 最多 4 条，seq 为大于零的严格递增安全整数；`next` 必须等于最后一条 seq，
  `lostBefore` 小于本批第一条 seq，最大安全序号 `9007199254740991`。
- **完整参数 JSON**（包括路由标识）最多 1024 UTF-8 字节；worker 必要时缩小批次。
  所有生产者必须限制队列和单条数据，不能用通用 data 实现无界缓冲或任意二进制流。

## Core 监听与 action 层

SDK 提供 `EdgeEventHookEvent`、`EdgeEventPayload`、`EdgeEventBatch`、`EdgeActionEvent`
和 `EdgeEventAck` 类型；从 `plugins/types/edge` 引入 type。
回调的 `event` / `eventName` 为 `edge_event`，`eventPayload` 为 `{chatId,nodeId,batch}`。

```ts
// 示意：store、persist 和串行队列由插件实现，存储应属于同一个 main actor。
export async function on_edge_event(event: EdgeEventHookEvent): Promise<EdgeEventAck> {
  const {nodeId, batch} = event.eventPayload;
  if (batch.source !== 'sensor.environment') return {accepted:false};
  const key = `${nodeId}:${batch.source}:${batch.stream}`;
  const cursor = store.cursors[key] ?? 0;
  const nextState = JSON.parse(JSON.stringify(store));
  const handlers: Record<string, (data: any) => void> = {
    'temperature.changed': data => {
      if (typeof data.celsius !== 'number' || !Number.isFinite(data.celsius)) throw Error('Invalid sample');
      nextState.temperature = data.celsius;
    },
    'serial.message': data => {
      if (typeof data.text !== 'string') throw Error('Invalid message');
      nextState.lastMessage = data.text;
    }
  };
  // 真正实现应先在副本上校验/应用全部 actions，再原子保存状态与 cursor；失败不 ACK。
  for (const item of batch.events.filter(item => item.seq > cursor)) {
    if (!Object.prototype.hasOwnProperty.call(handlers, item.action)) return {accepted:false};
    handlers[item.action](item.data);
  }
  if (batch.next > cursor) {
    nextState.lostEvents ||= batch.lostBefore > cursor;
    nextState.cursors[key] = batch.next;
    await persist(nextState);
    store = nextState;
  }
  return {accepted:true,next:batch.next};
}
```

Core 不把 action 当成函数名动态执行，只调用固定 `on_edge_event`。插件可使用 switch
或显式 handler 表；检查自身允许的来源／设备／stream，避免对象原型键被误当 action。
命令和事件共用 main actor 的串行队列，防止存档和 ACK 游标互相覆盖。

## 固件的通用生产者接口

`operit_node_edge::events::EdgeEventSource` 提供：
`set_event_route_provider`、`pending_event_delivery`、`acknowledge_event_delivery`、
`disable_event_delivery`、`wait_for_event`。共享 worker `startEdgeEvents` 仅依赖该 trait，
与屏幕及触摸无关。端侧可选库 `operit-edge-scene` 的 `ScenePlugin` 通过 adapter 使用原场景队列；Core 与公共 Edge 节点库不依赖此库。

普通生产者可复用 `EdgeEventQueue`：设置已授权路由 provider，通过认证后的原生订阅
动作调用 `subscribe(packageName)`，设备发生变化时调用 `publish(action, data)`。
队列最多 16 条，单条 data 最多 512 JSON 字节，返回 stream；重订阅使旧 ACK 失效。
`unsubscribe(stream)` 显式结束订阅。具体订阅动作由固件作者声明，不是新通用 RPC。
例如 GPIO producer 可发布 `gpio.changed / {pin,level}`，串口 producer 可发布
`serial.message / {text}`；没有事件就休眠，不要求 Core 定时查询输入。

## 授权、ACK 与失败边界

- 复用现有配对、Space 身份、chat Binding 和 `caller:chat.write`；没有隐式创建 Binding。
- Router 验证 `nodeId` 与可信 origin 一致，拒绝 Target 绕过、伪造设备来源；保持现有路由分类。
- 接收包必须已注册、已启用、依赖可用，main 导出存在；嵌套 Tools 调用继续执行权限链。
- 回调六秒时限，只返回 `{accepted:true,next}` 才确认对应批次。Core 返回有界
  `{success,next,message}`，错误提示最多 80 字符，不传 JS 源码／堆栈。
- 插件自行保证持久性与去重。重复 ACK 不能重复业务操作；不能把收到消息当成已保存。
- 只有待确认事件重发，不能重放硬件动作。失败按 1/2/4/8/16 秒退避，最多五次后休眠；
  新事件或 Peer 连接变化可再次唤醒。保持一个在途调用并完成 ACK 清理。
- 同 Space 重连可更换入口，原 chat Binding 和接收方保持固定；不同 Space 不继承旧订阅。
  旧 stream／接收方的迟到 ACK 不能修改新订阅。设备重启的 RAM 队列不持久化。
- `lostBefore` 表示真实丢失，不能编造业务事件。此入口不承诺分布式 exactly-once。

## 迁移

统一使用 `Tools.Edge` / `Tools.Io` 静态类；旧小写 `edge/io` 不再生成。
本 PR 中尚未合并的 `chatEdgeSceneEvent` 和 `on_edge_scene_event` 已替换为
`chatEdgeEvent` 和 `on_edge_event`；触摸字段归入 action data。Core、示例及固件需匹配更新。
