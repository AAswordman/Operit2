# Core 插件调用 Edge 硬件端口（首版）

插件 JavaScript 仍在 Core 的原 ToolPkg 运行时执行；只有硬件动作在选中的 Edge
节点执行。`tools` 是既有 `Tools` 对象的小写别名，`Tools.edge` / `Tools.io`
与 `tools.edge` / `tools.io` 完全相同。这不是给 ESP32 加另一套 JS 插件运行时。

## 两个入口

```ts
const response = await tools.edge.execute(
  edgeNodeId,
  { pluginId: "device.status", action: "read" },
  {}
);
console.log(response.nodeId, response.data);

const current = await tools.io.execute(
  edgeNodeId,
  { port: "gpio", operation: "read" },
  { pin: 2 }
);
// 只有已知允许控制的输出引脚才应写入；具体板子的限制由 Edge Host 校验。
await tools.io.execute(
  edgeNodeId,
  { port: "gpio", operation: "write" },
  { pin: 2, level: true }
);
```

- `nodeId`：必填稳定节点 ID，不是 IP、串口设备名、默认设备或当前 Core。
  使用现有设备空间拓扑／`toolCall("list_core_nodes", {})` 获取节点 ID；可达不等于授权。
- `edge` 的 `interfaceInfo`：`{pluginId, action}`，只能调用固件明确注册、声明的 action。
  可用动作可通过既有 `operit2 cli link edge-plugin <device> list` 查看。
- `io` 的 `interfaceInfo`：`{port, operation}`。首版只支持 `gpio/read` 与 `gpio/write`，
  复用 `edge.deviceIo.getDigitalOutput/setDigitalOutput`。GPIO 读取的是既有数字输出状态，
  **不是任意数字输入、ADC 或电压采样**。
- `args`：可省略，默认 `{}`；必须是 JSON 对象，最多 1024 UTF-8 字节。
  GPIO `read` 只接受 `pin`，`write` 接受 `pin` 与布尔 `level`；引脚为 0–255 整数，
  有效／保留引脚由板级 Host 再校验。没有 GPIO 的 Host／模拟器应明确报不可用。
- 成功返回 `{nodeId, data}`（可含工具运行时的 `__type` 标签），原生动作结果在 `data`；
  结果最多 4096 JSON 字节。失败拒绝 Promise，可 `try/catch`，不伪造成功或自动改用本机端口。

## 路由与权限

两个方法都经现有 `toolCall` / `AIToolHandler` 注册工具权限链执行，然后交给这个
Core 树注入的节点 Router。`io/read` 声明 READ，`io/write` 声明 WRITE；原生插件动作
当前没有可信读写效果元数据，`edge/execute` **保守声明 WRITE**，不按 action 名字猜权限。
它们是 INTERNAL SDK 工具，不自动变成模型可随意选择的公开硬件工具；专属 ToolPkg
可以封装为自己的、具有适当参数和权限说明的业务工具。

- 标准 Target 节点路由，不改 Chat Binding，不创建额外 Link/HTTP/串口客户端。
- 复用配对方向、空间身份、NetworkControl 和 Edge 原生 action 白名单校验。
  不在本接口执行配对、授予权限、添加成员或扩大可达范围。
- 接口信息不接受任意 `target`、`methodName`、路由种类、身份、地址或 token。
- 离线、节点未授权、插件缺失、action 未声明、端口不支持都应明确失败。
- 执行器只提交一次，不额外增加业务级自动重试／重放；Router 既有的路由不可用处理保持不变。
  本入口不承诺硬件动作 exactly-once；调用方停止等待不意味着设备操作回滚。
  已发出的 Link 请求仍由宿主任务完成 ACK 清理，避免取消业务调用破坏共享会话。
- Router 和执行器来自同一个 Core 树；不使用进程全局网络客户端。宿主缺少异步任务调度
  能力时明确报端口运行时未初始化，不影响其他 Core 功能启动。

## 暂不实现的后续能力

串口 `read/write`、端口枚举、串口会话、波特率设置、二进制分块、订阅输入流、背压、
超时／设备应答关联、跨 Core 委托及完整专属硬件插件实例生命周期暂未开放。
`{port:"serial",operation:"read"}` 当前**明确拒绝**，不返回空数据假装接通。
后续可保留 `nodeId + interfaceInfo + args` 入口并增补新的、经过板级注册与权限校验的
端口能力；开发普通插件不需要另学一套远端 JS 插件 API。

## 原生场景显示

`display.scene` v1 通过同一 `tools.edge.execute` 入口提供租约、素材分块校验缓存、
图层叠加、本地动画和触摸事件。完整协议见同目录 `edge-scene.md`。
该插件的业务协议错误使用 `data.ok:false` 信封；调用方必须检查，不能只看外层传输成功。
授权、离线、未声明 action 等入口错误仍拒绝 Promise。
