# Markdown 图片显示

Markdown 的 `![说明](图片地址)` 支持位图与 SVG。SVG 使用矢量渲染，点击图片后可在全屏预览中缩放、拖动；图片说明也作为无障碍标签。

- 相对 `.svg` 路径按消息保存的 `file_base_path` 解析，通过已有 Core VFS 文件服务读取。绝对 VFS 路径和节点限定的 VFS 路径使用同一读取链路。
- HTTP/HTTPS 图片沿用 Runtime Host 的图片交付协议。返回字节时，SVG 扩展名或 XML 根元素决定矢量渲染；没有扩展名的 SVG 响应也能识别。Host 选择 URL 直接显示时，继续由原有图片显示组件交给浏览器解码。
- `data:image/svg+xml` 同时支持 Base64 和 URI 编码。
- SVG 不经过 Flutter 的位图解码器；PNG、JPEG、GIF、HEIF 等仍走各自已有链路。侧栏宽度变化复用已加载的 SVG，不重新读取文件或下载。

示例：

```markdown
![流程图](./images/flow.svg)
![架构图](/operitapp/outputs/architecture.svg)
![网络图](https://example.com/diagram.svg?version=2)
```

SVG 的具体图形特性由项目现有的 `flutter_svg` 渲染库支持范围决定。无效图片沿用图片加载失败提示。

相关回归检查：`apps/flutter/app/test/markdown_image_rendering_test.dart`，覆盖 SVG 的相对路径、绝对 VFS、节点地址、网络地址、无扩展名响应、两种 data URI，以及预览缩放、布局变化与原有位图行为。
