# Operit Figma 界面桥接工具（当前为静态图层）

**目标是让设计同学在 Figma 中使用和编辑完整应用，包括页面导航、弹窗、工具结果以及插件内部 UI。当前实现尚未达到这个目标。**

本目录现有插件只能导入可编辑的界面图层，批量 ZIP 也是多个静态场景的集合。它没有应用运行时，没有恢复原来的点击、输入、状态更新或插件事件；不能作为完整可交互软件交付。主题参数的回传也不能代表任意软件界面修改已经应用。

交互应用的验收标准、现有 Web 实现和需要解决的架构问题见 [INTERACTIVE_APP.md](INTERACTIVE_APP.md)。以下说明只描述已有静态工具。

这里有两个入口，使用同一份 `.operit-design.json` 文件：

- **主题组件稿**：Theme Studio →「导出 Figma」。生成手机／桌面、浅色／深色的可编辑组件，绑定仓库的真实主题字段；它是组件起点，布局为示意，精确页面请走运行时采集。
- **运行时页面**：采集运行中的 Flutter 渲染树，以及插件 WebView 的 DOM。可以覆盖深层工具结果、Compose DSL 插件、工作区嵌入面板和弹窗，无需给每个页面另写一套 Figma 模板。

## 安装 Figma 插件

需要 Node.js 22.18+（推荐仓库当前使用的 Node.js 24）和 Figma Design 桌面端。

```sh
cd tools/figma
npm ci
npm run build
```

在 Figma 的 **Plugins → Development → Import plugin from manifest** 中选择本目录的 `manifest.json`，运行 **Operit · UI Design Bridge**。插件完全使用文件交接，不需要 Figma token、HTTP 服务器或额外上传服务。`dist/` 为可重建产物，不提交到版本库。

### 现有静态图层插件包

开发侧运行：

```sh
cd tools/figma
npm run package
```

会生成 `dist/operit-figma-designer.zip`，包含已编译的静态图层插件、范围说明，以及用于验证插件的主题组件示例。它不包含可运行的整个应用。设计同学解压后直接在 Figma 导入包内的 `manifest.json`，无需安装 Node.js 或运行构建。不要只移动 manifest，需保留同目录的 `dist/`。本地也可直接导入 `dist/designer/manifest.json`。

`manifest.json` 中的开发 ID 用于本地导入。若需要发布／团队分发，先在 Figma 的 Development → New plugin 创建自己的插件，把 Figma 分配的 ID 填入 manifest；该工具不会替用户发布插件。

## 设计同学的主题工作流

1. 打开 Theme Studio，选择默认主题或导入已有 `.toolpkg`。
2. 点击 **导出 Figma**，得到 `operit-theme.operit-design.json`。
3. 在 Figma 插件中点击 **导入设计文件**。文字、容器、气泡和输入框均为原生图层；主题组件生成 Component；颜色和显式标量参数生成 Variables。
4. 在画板中编辑颜色、文字、尺寸、图片或布局。`Light`／`Dark` 使用两个单模式集合，不依赖多模式付费功能。
5. 修改完整主题字段可在插件的 **主题完整参数** 中编辑 JSON，然后点击 **更新主题参数**。此处保留主题支持的全部字段，包括字体路径、气泡九宫格切片和样式枚举。
6. 选中导入的画板或其内部图层，点击 **导出修改与交接**。
7. 回 Theme Studio 点击 **导入 / Figma 回传**，选择导出的文件。Studio 恢复主题、素材及包元数据，用真实 Flutter 组件预览，再按原有流程导出 `.toolpkg`。

从现有 ToolPkg 开始时，设计文件同时携带原始包与所选模板 ID。回传后继续使用现有的包合并逻辑，保留脚本、其他主题、共享资源、完整本地化元数据和协议文本。

### 自动回传的范围

| 修改                                                   | 结果                                                                |
| ------------------------------------------------------ | ------------------------------------------------------------------- |
| 配色变量                                               | 回传 `palette.light`／`palette.dark`，保留 alpha                    |
| 气泡、输入框填充及气泡文字色                           | 映射回已有主题路径；原值为 null 时未修改会保留 null                 |
| 气泡左右 padding、输入动作图标尺寸                     | 回传已有数值参数，仍受主题 schema 范围限制                          |
| 圆角开关                                               | 映射回现有 `roundedCorners` 布尔值；当前契约没有任意圆角数值        |
| 替换已绑定 PNG/JPEG/SVG 素材                           | 将实际素材和路径一起回传；Rust 继续校验媒体                         |
| 任意主题字段                                           | 通过完整参数编辑保留；样式枚举、九宫格、字体文件以 Flutter 预览为准 |
| 文字内容、自由布局、新增／删除组件、工具／插件视觉修改 | 导出 `changes`、完整图层和 `source`，供开发实施                     |

公共主题字段在多个画板中改成不同值时，插件报冲突，要求设计同学统一该字段；不会任意选择一张画板。自由布局和插件业务逻辑仍需要修改源代码，设计文件不会把这些修改伪装成已应用到运行中的程序。

## 采集深层工具与插件 UI

以 debug 模式启动应用，并显式启用桥接。使用项目自己的 Flutter SDK／设备启动参数：

```sh
cd apps/flutter/app
.fvm/flutter_sdk/bin/flutter run -d macos \
  --dart-define=OPERIT_DESIGN_BRIDGE=true \
  --vmservice-out-file=/tmp/operit-design-vm.txt
```

Windows 上可使用 `fvm flutter run -d windows`，移动设备使用对应设备 ID。也可以从 Flutter 输出中复制 **Dart VM Service** 地址。手机的服务地址需要沿用 Flutter 建立的端口转发。

在应用中打开需要设计的页面，展开工具结果或进入插件子页面。然后从仓库根目录执行：

```sh
node tools/figma/capture.mjs \
  --vm-service-file /tmp/operit-design-vm.txt \
  --name '插件设置 · 工具结果展开' \
  --out /tmp/plugin-settings.operit-design.json
```

也支持直接传地址：

```sh
node tools/figma/capture.mjs \
  --vm-url 'http://127.0.0.1:12345/your-auth-code=/' \
  --list
```

`--list` 列出已挂载的采集根；`--scope` 选择根。主应用、独立聊天窗口、桌面小组件及救援／启动错误窗口均已接入；独立窗口可能属于另一个 VM Service，需使用对应进程地址。

把采集文件交给设计同学在同一个 Figma 插件中导入。每次采集生成一份可编辑图层，以及一张锁定的真实渲染参考图。请为不同页面、展开／折叠状态、滚动位置和插件路由分别采集、分别导入；采集不会触发业务操作或自动遍历未知插件。

### 深层 UI 的处理

- **工具结果**：遍历实际渲染树，保留已展开的文本、容器和图片；附带 `kind: tool-result`、工具名称和展开状态。
- **Compose DSL 插件**：使用生产渲染器采集实际原生控件；附带包名、路由、节点类型、稳定的逻辑 `composeId` 与当前挂载路径。
- **WebView 插件**：通过已挂载 WebView controller 执行只读 DOM 采集，提取可见容器、文字、输入框、SVG 和可读图片。支持同源嵌套 iframe 和 shadow DOM；插件可加 `data-operit-design-id` 提供稳定的设计标识。
- **自定义绘制**：装饰／CustomPaint 独立采集成图片，继续递归其内容子树，避免一个卡片边框使整张卡片的文字失去可编辑性。Canvas／视频、图标字体和部分平台视图保留为可替换图片。
- **边界**：懒加载、未展开及视口外内容需要先显示再采集；跨域 iframe、受限图片、CSS 伪元素等会在导出文件和插件中显示明确说明。平台 WebView 不一定出现在 Flutter 的参考截图中，DOM 图层仍可单独编辑。

桥接默认关闭，release 构建不会注册采集入口。采集保存用户当前实际可见的内容，请使用适合交给设计同学的测试数据。密码输入导出遮罩文字。

## 文件和代码入口

- `src/bridge.ts`：文件契约、主题 schema 校验、来源与图层变更。
- `src/plugin.ts`：Figma 原生节点、组件、变量、素材导入与修改回传。
- `src/theme_scene.ts`：主题组件起点。
- `capture.mjs`：VM Service 只读采集命令。
- `../../apps/flutter/packages/operit_ui/lib/src/design/`：Flutter 渲染树和 WebView DOM 采集。
- `../theme/studio/src/figma_bridge.ts`：Studio 双向文件交接及原包保留。

## 验证

```sh
cd tools/figma
npm run typecheck
npm test
npm run build
```

```sh
cd tools/theme/studio
npm run typecheck
npm test
```

```sh
cd apps/flutter/packages/operit_ui
../../app/.fvm/flutter_sdk/bin/flutter test test/design_capture_test.dart
```

```sh
cd apps/flutter/app
.fvm/flutter_sdk/bin/flutter test test/figma_deep_capture_test.dart \
  --dart-define=OPERIT_DESIGN_BRIDGE=true
```

最后一项使用真实工具结果组件和真实 Compose DSL 渲染器，验证深层文字导出及所属包／工具标识，并生成 `.dart_tool/figma-deep-capture.operit-design.json` 供检查。

TypeScript 的插件逻辑通过官方 `@figma/plugin-typings` 类型检查，并通过 API 模拟宿主验证导入、变量回传、冲突检测、素材和交接。它不能替代 Figma 桌面端的视觉验收；发布前仍应在实际 Figma 中导入并对照 Flutter 参考图。

两套 Flutter 测试生成采集文件后，还可以运行实际文件的往返验证：

```sh
cd tools/figma
npm run test:runtime
```

该检查将真实 Flutter 采集文件送入插件导入器，在原生文字图层上修改文字和位置，再验证导出的变更及所属插件信息。复杂填充、阴影和 Auto Layout 等设计参数保留在 `source.figmaAppearance` 中，供开发交接；主题契约之外的自由设计效果仍以 Figma 原稿实施。

交付包还可以单独执行验证：

```sh
cd tools/figma
npm run test:package
```

该命令解压实际 ZIP，检查 manifest 引用，执行编译后的插件，在 API 模拟宿主里验证示例组件可编辑、主题颜色可回传和文字差异可导出；仍不替代实际 Figma 的视觉验收。

## 实时运行客户端初版

真实运行与编辑连接的启动方法、候选 Code layer 项目及未完成项见 [LIVE.md](LIVE.md)。它与上文的静态导入插件是不同入口；整软件的 Figma 验收尚未完成。
