# Edge 原生场景显示插件：`display.scene` v1

面向运行在 **Core 既有 ToolPkg 运行时**的桌宠、仪表盘和专属硬件插件。
Core 管理养成数据、素材原件和业务逻辑；具备显示能力的 Edge 注册原生显示服务，不运行插件 JavaScript。
场景服务、解析器和显示协议位于端侧可选库 `hosts/common/operit-edge-scene`；
公共 Core/Edge 节点库不依赖该库，也不要求节点有屏幕。当前 ESP32 固件和模拟器
显式引用该库；浏览器只适配模拟器的显示输出。设备应用通过
`ScenePlugin::new(width, height, reserved_top, touch)` 传入显示参数，
无顶部系统栏可传 `reserved_top=0`。其他设备可复用此库或实现同一服务协议。
模拟器现有开发者 HTTP 中的 `scene-view` 仅是浏览器显示镜像，可携带有界素材快照，
不是固件入口或新的插件 API；Core ToolPkg 不依赖该路径，仍只调用 `Tools.Edge.execute`。

## 入口与授权

```ts
async function scene(nodeId: string, action: string, fields = {}) {
  const {data} = await Tools.Edge.execute(nodeId,
    {pluginId: 'display.scene', action}, {v: 1, ...fields});
  if (data.v !== 1 || !data.ok) throw new Error(`${data.error?.code}: ${data.error?.message}`);
  return data.result;
}
const capabilities = await scene(edgeNodeId, 'capabilities');
```

- 继续使用现有配对、设备空间、Target 节点路由、NetworkControl、工具权限链和原生 action 白名单。
  本插件不建立额外网络连接、不配对、不授予访问权、不接受 RPC 地址/token。
- `nodeId` 必须明确指定；可达不等于授权。`edge_execute` 仍保守要求 **WRITE** 权限，包括能力查询。
- 每个请求的 `args` **最多 1024 UTF-8 JSON 字节**，原生回复最多 **4096 JSON 字节**。
  首版一块最多 256 原始字节，base64 最多 344 字符；连同两个 UUID 等字段通常不足 600 字节。
- 所有 action 都要求 `v:1`；未知版本、未知字段、错误类型拒绝。`rect`、`layers` 也拒绝未知字段。
- 原生协议成功：`{v:1, ok:true, result:{...}}`。
  原生协议失败：`{v:1, ok:false, error:{code,message}}`。
  **后者不是操作成功**：工具外层已成功传输，但调用方必须检查 `data.ok`。
  离线、未授权、插件不存在、action 未声明等仍由既有入口拒绝 Promise。
- 不提供业务级自动重试，不承诺 exactly-once。丢失回复不代表设备操作未发生。

## 首版能力和容量

插件应通过 `capabilities` 与 `lease.open` 返回的区域适配设备；不能假定所有 Edge 有屏幕或尺寸相同。以下是当前 320×240 ESP32／模拟器 profile：

| 项目 | 上限／约定 |
| --- | --- |
| 屏幕 | 320×240 逻辑像素，RGB565 小端 |
| 系统保留区 | 全宽 `y=0..23`，右上角退出按钮始终属于系统 |
| 显示租约 | 同时一个区域／一个拥有者；默认区域 `{x:0,y:24,w:320,h:216}` |
| TTL | 默认 30000 ms，可指定 1000–60000 ms；`lease.renew` 按原 TTL 续约 |
| 缓存 | 最多两个包，总原始字节 8192，单包最多 4096 |
| 未完成上传 | 同时一个，最多 4096 原始字节；与已提交缓存独立计量 |
| 素材 | 每包最多 4 个，宽高各 1–96，帧数 1–8，调色板 1–16 |
| 动画 | 本地播放；帧间隔 50–2000 ms，不保证每一帧都能在物理屏上显示 |
| 图层 | 最多 4 层，整数缩放 1–8，区域内裁剪，不旋转 |
| 事件 | 最近 16 个；主动推送最多 4 个/批，ACK 和丢失游标；显式诊断 poll 最多 8 个 |
| 缓存持久性 | **`persistent:false`：首版仅 RAM 缓存，重启后 Core 重装** |

尺寸／帧数上限不能同时保证：像素索引总量、调色板和包头仍须符合单包 4096 字节。
缓存不自动驱逐；满时由持有租约的调用方明确移除未使用的包。
Core 应保留原件并按 SHA-256 判断是否需要重装，不能把 Edge 缓存作为素材或养成数据的唯一副本。

场景素材内存另计，不塞进 C UI 的静态缓冲：最多 8 KiB 已提交字节 + 4 KiB 上传字节，
另有有界索引、调色板、租约、图层、事件以及解析／传输暂存。
现有 C UI 继续保持 **静态 RAM <10 KiB、1280 字节两行绘制条带、UI heap=0**；
不创建整屏 framebuffer，也不展开复制每张素材。上述是容量限制，不是实机可用堆测量结果。

## 坐标、锚点、叠加与动画

- `rect` 使用屏幕绝对逻辑坐标，原点左上，x 向右、y 向下；宽高为像素数。
  必须完整位于屏幕内，`y>=24` 且宽高非零。
- 每层 `x,y` 是**区域局部坐标**，有符号 16 位整数；可部分位于区域外，显示时裁剪。
- `anchor` 默认 `tl`：左上锚点。`center`：中心锚点；`bc`：底边中心锚点。
  缩放后宽 `W`、高 `H`：
  - tl：左上 `(x,y)`；
  - center：左上 `(x-floor(W/2),y-floor(H/2))`；
  - bc：左上 `(x-floor(W/2),y-H)`。
- `scale` 默认 1，最近邻整数缩放；`asset` 为 `packId:assetId`。
- `background` 是 0–65535 的 RGB565 整数，默认黑色，填满租赁区域。
  如要图片背景，在 `layers` 首层放背景素材。后续宠物、道具按数组顺序从后到前叠加。
- alpha 为 **straight alpha**，0 完全透明、255 不透明；各 RGB565 通道以
  `floor((foreground*alpha + background*(255-alpha) + 127)/255)` 混合。
- `frame` 默认 0，必须是素材内有效帧；`play` 默认 false；`loop` 默认 true。
  播放从 `frame` 开始，按素材 `frameMs` 推进，循环时取模，不循环时停在最后一帧。
- 一次不同的 `scene.set` 重设所有图层的动画时基；内容完全相同的更新不重新起播。
  绘制条带在整帧开始冻结时间戳，避免同一帧不同条带使用不同动画帧。
  本地动画不依赖 Core 每帧 RPC；图层变更仍可能在正在绘制的帧结束前生效。

## 租约与系统所有权

`lease.open` 的 `owner` 仅为 ASCII 标签（1–24 字符），**不是经认证的 ToolPkg 身份**。
租约返回随机 UUID 操作句柄，用来避免误改其他场景，不取代、不扩大现有网络授权。
已获得相应 Edge 访问权的 Core 仍必须遵守现有工具权限链。

配对码、设备空间审批和系统错误界面优先于场景。触摸不会穿透这些系统界面。
右上角退出入口独立于 Core；退出或 TTL 到期立即取消显示及未完成上传，恢复普通 UI。
缓存保留。Core 离线时也会过期；不允许插件通过坐标覆盖保留区。

`lease.open` 的可选 notify 只指定 Core ToolPkg 包 ID，申请时捕获当前已授权 Space 和初始化完成的 chat Binding；无有效会话返回 `SCENE_NOTIFY`。它不是 URL、RPC 方法名或任意工具入口。

旧租约关闭后保留一个事件墓碑，允许原句柄显式查询 `system.exit`／`lease.expired`／`lease.closed`；
下一次 `lease.open` 会替换墓碑、清空事件页，旧句柄不再有效。
不在能力查询或模拟器浏览器快照中公开租约句柄。

## Action 表

下列参数均省略了必需的 `v:1`。ID 限 1–24 字符 ASCII `[A-Za-z0-9_.-]`，不支持路径。

| action | 参数 | result / 行为 |
| --- | --- | --- |
| `capabilities` | 无 | 协议、屏幕、保留区、触摸、格式、动画、limits、缓存占用和包／素材 ID／SHA |
| `lease.open` | `owner`, `rect?`, `ttlMs?`, `notify?:{packageName}` | `lease`, `rect`, `ttlMs`, `cursor`；忙时拒绝，不抢占 |
| `lease.renew` | `lease` | `ttlMs`；过期／关闭不可复活 |
| `lease.close` | `lease` | `{}`；同一关闭墓碑可重复关闭，不产生重复关闭事件 |
| `pack.begin` | `lease`, `packId`, `bytes`, `sha256` | `upload`, `chunkBytes`；SHA 是 64 字符小写 hex |
| `pack.chunk` | `lease`, `upload`, `offset`, `data` | `offset`：当前已接收字节总数；data 为标准 base64 |
| `pack.commit` | `lease`, `upload` | `packId`；精确长度、SHA、格式全通过后才原子替换缓存包 |
| `pack.abort` | `lease`, `upload` | `{}`；丢弃未完成上传 |
| `pack.remove` | `lease`, `packId` | `{}`；正在显示的包不可删除 |
| `scene.set` | `lease`, `background?`, `layers` | `revision`；先完整校验再替换，空数组清除图层但保留区域背景 |
| `events.poll` | `lease`, `after?` | `events`, `next`, `lostBefore`, `closed` |

上传按字节偏移顺序进行。完全匹配已收到内容的重复 chunk 可幂等 ACK；冲突、越界、
缺失或错误偏移拒绝。未完成上传不对显示可见；校验失败保留旧包和上传，调用方应 abort。
正在使用的素材包不可开始替换，也不可在提交时替换。

```ts
const lease = await scene(node, 'lease.open', {owner:'market.pet', ttlMs:30000});
const upload = await scene(node, 'pack.begin', {
  lease:lease.lease, packId:'pet', bytes:originalByteLength, sha256:originalSha256
});
// Core 侧将原件分成 <=256 原始字节的块，再编码为标准 base64。
for (const {offset, data} of chunks) {
  await scene(node, 'pack.chunk', {lease:lease.lease, upload:upload.upload, offset, data});
}
await scene(node, 'pack.commit', {lease:lease.lease, upload:upload.upload});
await scene(node, 'scene.set', {lease:lease.lease, layers:[
  {asset:'pet:background', x:0, y:0, scale:4},
  {asset:'pet:idle', x:160, y:180, anchor:'bc', scale:6, play:true, target:'pet'}
]});
```

## 二进制素材格式（v1）

没有 ZIP、外部图片解码器、路径、脚本、下载 URL 或可执行代码。
包里只存索引色素材；**多字节整数均小端**，禁止尾随垃圾／截断。

`ESP1` 包：

```
4 bytes  magic ASCII "ESP1"
u8       assetCount (1..4)
repeat assetCount:
  u8       idLength (1..24)
  bytes    ASCII assetId（包内不能重名）
  u16 LE   assetLength
  bytes    ESI1 素材（精确 assetLength）
```

`ESI1` 素材：

```
4 bytes  magic ASCII "ESI1"
u8       width (1..96)
u8       height (1..96)
u8       frameCount (1..8)
u8       paletteCount (1..16)
u16 LE   frameMs (50..2000，即使静态图也须有效)
repeat paletteCount:
  u16 LE   RGB565 color
  u8       straight alpha (0..255)
bytes    frameCount * width * height 个 u8 调色板索引
```

每帧按行存放，先上后下、先左后右；每个索引必须小于 paletteCount。
所有帧共用同一调色板、尺寸和帧间隔。SHA-256 覆盖整个 ESP1 包，包括头与 ID。

## 触摸事件与 Core 养成逻辑

若 `touch:true`，通过 `target` 为可交互图层绑定 ID。按由前到后的顺序测试有 target 的图层，
命中区域及该帧 alpha 非零像素才发送 `target.tap`；未绑定 target 的图层不拦截命中。
只上报一次轻触的释放，不提供多点／拖动／长按。无触摸时仍主动通知系统退出／过期事件。

```json
{
  "events":[{"seq":1,"type":"target.tap","target":"pet","x":144,"y":122}],
  "next":1,"lostBefore":0,"closed":null
}
```

- `x,y` 为区域局部像素；系统事件的 x/y/target 为 null。
- `after` 默认 0，表示只返回序号更大的事件；客户端保存 `next`，避免重复执行养成逻辑。
- `lostBefore` 是已经不可恢复的最后序号；`after < lostBefore` 说明遗漏事件，
  不得编造点击次数。事件缓存不等于持久业务队列，Core 仍是业务数据的权威。
- `closed` 为 null 或 `system.exit`／`lease.expired`／`lease.closed`。
- 输入默认使用文末的主动事件传输，Core 不定时查询触摸；`events.poll` 仅是兼容／手动诊断。
  保活只使用 `lease.renew`，处理关闭事件后停止，不自动抢回系统界面；不改变嵌入式流的路由语义。

## 错误码

| code | 含义／建议 |
| --- | --- |
| `SCENE_VERSION` | 缺少或不支持协议版本 |
| `SCENE_NOTIFY` | 申请主动通知时没有当前已授权且初始化完成的 Space/chat |
| `SCENE_ARGS` | 未知字段、错误类型、ID／锚点／帧／base64／游标无效 |
| `SCENE_BUSY` | 区域或上传被占用，不抢占 |
| `SCENE_LEASE` | 无租约／句柄不匹配；重启或新拥有者后明确重开 |
| `SCENE_EXPIRED` | TTL 已过期；需用户明确重新申请 |
| `SCENE_CLOSED` | 用户／Core 已关闭；不能续约复活 |
| `SCENE_LIMIT` | 请求、区域、TTL、包、缓存、图层或 chunk 超限 |
| `SCENE_FORMAT` | ESP1／ESI1 格式、长度、调色板索引等校验失败 |
| `SCENE_HASH` | SHA-256 不一致，abort 后从 Core 原件重装 |
| `SCENE_OFFSET` | 上传顺序／长度不正确或尚未完整上传 |
| `SCENE_RESOURCE` | 缓存包、素材或上传句柄不存在 |
| `SCENE_IN_USE` | 正显示的素材包不可删除／替换；先更新场景移除引用 |

message 只供诊断，不作为稳定机器判断条件；依据 code 分支。

## 可测试的 Core ToolPkg

目录 `plugins/packages/external/edge_pixel_pet/`，包 ID `com.operit.edge_pixel_pet`，随 Core 分发在「内置更多包」，默认不导入、不启用。
内含代码生成的原创像素宠物、透明道具和背景，Core 保留 `resources/demo.esp` 及同内容分块，
无需刷固件即可替换美术。`generate_assets.py` 可重现原件、SHA 和分块。

1. 已刷入支持 `display.scene` 的固件，或启动当前模拟器，完成既有配对／空间授权。
2. 编译 `tsconfig.json`；使用既有 `sync_plugin_packages.py` 的打包路径生成 `.toolpkg`，
   或按该目录的 README 单独打包。正式构建随 Core 分发，可直接从「内置更多包」导入，再显式启用。
3. `test` 需要环境变量 `EDGE_SCENE_NODE_ID`；其他工具也可直接传 `node_id`。
4. `start_pet` 显示桌宠并订阅主动事件；点击宠物／道具后 Edge 调用 Core，Core `PluginConfig` 保存互动次数和游标。
   `pet_status` 只读 Core 状态；`renew_pet` 只续显示租约（例如每 20 秒一次），不查询触摸。`close_pet` 释放显示区域。
5. 右上角退出也主动上报 `system.exit`，不会自动重开。60 秒不续约会退出。空闲时没有事件 RPC 或事件引起的配置写入。

示例采用**一个 Core 控制器**；工具通过现有同包 `ToolPkg.ipc` 进入 main actor，与事件回调串行操作，避免多个运行时的配置缓存互相覆盖。互动次数和游标作为一个 Core 配置状态更新，
不把测试示例当作跨 Core 分布式事务或 exactly-once 养成引擎。市场插件应自行处理并发、
断连和业务持久性；传输失败不应盲目重放硬件操作。

自动测试覆盖：截断包、无效索引／SHA／偏移、原子提交与更新、满缓存／使用中保护、
错误／过期租约、透明叠加／锚点／动画像素金样、事件分页丢失、系统退出、真实 C UI 内存预算，
以及真实 Core ToolPkg → Tools.Edge → TCP 模拟器原生服务 → 渲染／触摸／Core 配置链路。

## 通用 Edge 事件传输中的场景适配

`capabilities.events` 返回 `{push:true,method:"chatEdgeEvent",maxBatch:4,ack:true}`。
Core 监听入口和传输协议见 [edge-events.md](edge-events.md)，不再限定屏幕／触摸。
场景服务只是 `EdgeEventSource` 的一个实现，共享原有租约事件队列、去重与授权边界。

原生 `events.poll` 的诊断回复仍保留上文格式；主动推送改为通用 envelope：

```json
{"v":1,"source":"display.scene","stream":"<租约 UUID>","events":[{"seq":1,"action":"target.tap","data":{"x":144,"y":122,"target":"pet"}}],"next":1,"lostBefore":0}
```

- 场景租约映射为 `stream`，`type` 映射为 `action`，触摸字段归入 `data`。
- `system.exit`、`lease.expired`、`lease.closed` 是独立 action，data 为 `{}`。
- 固件调用固定通用 route `chatEdgeEvent`；Core 使用 `edge_event` discriminator
  调用已启用包的固定导出 `on_edge_event`，不允许选择任意函数。
- 场景业务插件校验 `source === "display.scene"` 和当前 stream，再按 action 处理，
  持久化互动与游标后返回 `{accepted:true,next:batch.next}`。
- 只有发生事件才唤醒 worker；没有事件不轮询、不发 RPC。保活与输入完全独立。
- 旧 Space 的接收方不带入新 Space；旧 stream ACK 不改变新 stream；ACK 丢失只重送事件。
- 早期开发版本的 `chatEdgeSceneEvent` / `on_edge_scene_event` 协议已替换，
  示例、固件与 Core 必须同时更新；没有保留屏幕专用旁路。

插件开发、素材适配、模拟器联调与发布步骤见 [Edge 插件作者指南](edge-plugin-guide.md)。
