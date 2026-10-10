# Edge 插件作者指南

市场插件使用 JS/TS，在 Core 的既有 ToolPkg 运行时执行。Edge 可以是不同单片机或
操作系统；端侧提供原生能力和交互事件，Core 保存业务状态与素材原件。
桌宠、传感器或按键设备共用节点路由与通用事件，不要求所有 Edge 有屏幕。

## 开发前确认能力

插件作者需确认用户安装的 Core SDK 和端侧服务版本匹配。现有入口为
`Tools.Edge.execute(nodeId, {pluginId, action}, args)`、GPIO 专用 `Tools.Io.execute`
与 ToolPkg main 导出 `on_edge_event`。`Tools`、`Edge`、`Io` 均注意大小写；
Edge/Io 是静态类，不创建实例。可达不等于授权：配对、设备空间与相应 Binding、
工具权限都使用项目现有流程。

从设备空间拓扑或 `toolCall("list_core_nodes", {})` 取得稳定节点 ID，由用户明确选择。
不要用 IP、串口号或默认 Core 代替 `nodeId`。检查端侧实际注册的服务和允许的 action；
无显示能力的节点可能仅提供传感器或按键服务，不能假定有 `display.scene`。
原生动作清单可通过既有 `operit2 cli link edge-plugin <device> list` 查看。

若设备提供场景服务，可使用如下 JS/TS 查询：

```ts
async function scene(nodeId: string, action: string, fields = {}) {
  const {data} = await Tools.Edge.execute(
    nodeId, {pluginId: 'display.scene', action}, {v: 1, ...fields},
  );
  if (data?.v !== 1 || data.ok !== true) {
    throw new Error(data?.error?.message ?? 'Invalid scene response');
  }
  return data.result;
}
const capabilities = await scene(selectedNodeId, 'capabilities');
```

`Tools.Edge` 外层完成传输不代表业务成功；检查 `data.ok`。离线、权限或动作声明失败
则拒绝 Promise。接口的参数最多 1024 UTF-8 JSON 字节，结果最多 4096 JSON 字节。
通用串口事件不表示 `Tools.Io` 支持串口读写；新增端侧能力仍需端侧作者声明服务。

## JS/TS 插件与端侧服务的分工

| 内容 | 所属位置 | 更新方式 |
| --- | --- | --- |
| 养成逻辑、存档、事件 action handler | Core ToolPkg 的 JS/TS | 编译 TS、打包 `.toolpkg`、在 Core 安装/启用 |
| 素材原件 | ToolPkg 资源与 Core 存储 | 发布资源后通过已安装服务上传 |
| 素材缓存、本地动画、输入采集 | 选定 Edge 的原生服务 | 在服务能力范围内由插件调用 action |
| 屏幕驱动、系统 UI、原生新能力 | 端侧应用与 Host | 端侧作者构建/更新固件或应用 |

`core/crates/node/edge` 与 `edge-contract` 仅提供通用节点/通信能力。
可选渲染器和显示协议位于 `hosts/common/operit-edge-scene`；设备应用显式依赖并注册。
ESP32 使用 320×240、顶部 24 像素保留区；这些值是端侧配置，不是 Core 的平台条件。
其他设备可以配置不同尺寸、无触摸，或实现另一种呈现服务。
现成兼容服务已安装时，插件作者更新业务与素材不需要写 Rust 或重编固件。

## 素材与场景适配

查询 `capabilities` 中的屏幕、保留区、格式、动画和容量，申请 `lease.open`，
再按返回的 `rect` 排布图层。不要写死 ESP32 坐标作为所有设备的布局。
当前示例插件的美术与图层坐标针对 320×240，是可运行示例，不是自适应模板。

当前可选渲染器使用 ESP1/ESI1 索引像素格式；名称是兼容的 magic bytes，
不限制运行芯片。素材应满足设备的调色板、宽高、帧数、包大小与缓存容量。
上传原始字节按序分块、base64 编码，通过 SHA-256 校验后提交。
不要把任意 PNG 文件直接当成此素材包；构建时生成或转换，运行时读取原件。
详细二进制结构与 action 表见 [场景协议](edge-scene.md)。

首版共享渲染器只有 RAM 缓存：设备重启后重新查询并安装素材。保存原件和业务存档
在 Core，不能只保存在 Edge 缓存。租约默认 30 秒，示例申请 60 秒；持续展示需要
显式续约。退出或到期后不自动抢回屏幕。设备系统 UI 与退出机制由端侧保留。

## 通用事件与存档

main 导出 `on_edge_event(event)`，其 `event.eventPayload` 是
`{chatId, nodeId, batch}`；每条事件有 `seq`、`action` 与 JSON 对象 `data`。
按来源 `source`、节点、`stream` 及明确的 action 表处理。Core 不把 action 当函数名
动态执行，不解析桌宠、触摸或传感器业务字段。

接收方应将命令与事件放在同一个 main actor 串行队列中；先校验并应用业务变化，
持久保存状态和去重游标，成功后才返回 `{accepted:true,next:batch.next}`。
失败、未知 action 或来源不匹配不 ACK。`lostBefore` 表示队列丢失，需要插件自己的
恢复策略；有限重试不提供 exactly-once。可运行的保存/去重实现见
[示例 pet.ts](../packages/external/edge_pixel_pet/src/pet.ts)，完整信封与授权见
[通用事件协议](edge-events.md)。

## ESP32 模拟器联调

1. 从仓库根目录运行 `npm run build --prefix tools/esp32-editor` 生成 C/Wasm 预览，
   再运行 `npm start --prefix tools/esp32-editor`，打开本地 8766 端口的页面。
2. 网页自动启动 Rust 模拟设备。用面板 TCP/Token 在 Core 配对，在设备屏幕读取
   配对码，并批准 Core 发起的设备空间申请。配对本身不代表 Binding 就绪。
3. 在 Core 的「内置更多包」导入并启用 `com.operit.edge_pixel_pet`。
   配置 `EDGE_SCENE_NODE_ID`，或在工具参数中显式传 `node_id`。
4. 调用 `edge_pixel_pet:start_pet`。设备显示像素宠物，点击宠物/道具后查询
   `pet_status`，验证 Core 已保存计数；`renew_pet` 只续约，`close_pet` 释放显示。
5. 从系统出口关闭，停止/启动模拟器并重新下发，验证缓存恢复和存档不丢失。
   “刷新预览页”只重载网页/Wasm，不能当成固件重启验证。

网页的 `scene-view` 是开发镜像，不是插件 API；市场插件不依赖编辑器 HTTP 服务。
真实自动联调用 `npm run test:space --prefix tools/esp32-editor`，启动真实 Core 和
模拟器并验证权限、素材、事件与重启。它只证明该模拟器链路，不能证明其他板卡已
实现服务，也不能代替实机内存、性能、触摸及烧录验证。

## 发布与跨设备边界

参考 [示例打包说明](../packages/external/edge_pixel_pet/README.md)，
将 TS 编译成 JS，manifest 的 main 指向 `dist/main.js`，资源一并打入 `.toolpkg`。
在用户安装包后启用，通过已有权限流程访问所选节点；不另建数据同步后端。
专属/一般列表标记使用 `"edge": {"exclusive": true}`，旧 `esp32` 标记需迁移。

多个呈现设备连接同一个可达且已授权的 Core 时，可以由该 Core 插件管理共同业务
状态，并为各设备选择呈现。设备空间提供通信和既有存储同步机制；这两个端口不会
自动把插件所有缓存同步到任意 Core，也不会自动解决多个 Core 的存档冲突。
跨 Core 的可用性取决于现有存储与同步范围、包启用状态和 Binding；首版示例是
单 Core 控制器，未实现分布式养成事务。“到任何设备互动”仍要求目标端侧有兼容能力
且权威业务状态可通过现有系统访问。
