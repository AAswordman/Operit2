# 设备显示配置：edge.scene v1

SceneService 是设备原生显示服务。ToolPkg 查询能力、上传资源并保存配置；设备独立渲染和处理点击。

## 调用

```typescript
async function scene(nodeId, method, fields = {}) {
    const reply = await Tools.device(nodeId).Edge.execute(
        {target: 'edge.scene', method}, {v:1, ...fields});
    if (!reply.data.ok) throw new Error(reply.data.error.message);
    return reply.data.result;
}
```

## 配置和资源操作

| 方法 | 参数 | 结果 |
|---|---|---|
| capabilities | 无 | 屏幕、区域、输入、格式、容量、缓存和持久化能力 |
| config.read | 无 | enabled、background、layers |
| config.apply | enabled、background、layers | configured、persistent、revision |
| config.clear | 无 | 保存默认关闭配置并恢复系统 UI |
| state | 无 | 本地 active、taps、revision 诊断状态 |
| pack.begin | packId、bytes、sha256 | upload、chunkBytes |
| pack.chunk | upload、offset、data | 当前 offset；data 为标准 base64 |
| pack.commit | upload | 校验长度、SHA-256 和格式后发布缓存资源 |
| pack.abort | upload | 放弃未完成上传 |
| pack.remove | packId | 删除未被保存配置引用的资源 |

显示配置和引用的原始资源作为单个快照原子保存。保存失败不改变当前配置。设备重启恢复配置及资源；动画帧和点击计数属于本次设备运行。配置包可以在保存完成后关闭或卸载。设备上的系统退出只停止当前显示，保存配置仍保留；`config.clear` 清除保存配置。

当前实现请求最多 1024 字节、响应最多 4096 字节，单包 4096 字节，缓存 8192 字节，最多 2 包、每包 4 素材、4 图层、256 字节上传块。设备的可用持久化空间可能进一步限制保存，失败返回 SCENE_STORAGE。

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
- 每次 `config.apply` 从保存配置重新开始当前动画。
  绘制条带在整帧开始冻结时间戳，避免同一帧不同条带使用不同动画帧。
  本地动画不依赖 Core 每帧 RPC；图层变更仍可能在正在绘制的帧结束前生效。

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

## 本地输入与集成

有 target 的图层按从前到后进行命中测试，透明像素不响应。`SceneService.touch` 返回本地 target；固件调用对应动作，示例中的 pet/treat 在本地推进动画帧。系统退出拥有本地优先级。

设备应用使用 `SceneService::new(width, height, reserved_top, touch).with_storage(storage)`，把原生 `edge.scene` Call 接到 `SceneService.call`，把显示条带接到 `paint_strip`。当前 ESP32 和模拟器使用 320×240 屏幕与 24 像素系统栏。其他设备可以采用自己的显示协议。

`com.operit.edge_pixel_pet` 提供 inspect、apply_configuration、clear_configuration，用于配置这个原生服务。
