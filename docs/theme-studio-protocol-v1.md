# Theme Studio ↔ Flutter Preview 协议 v1

状态：Theme Studio 与独立 Flutter Web + Rust/WASM 预览核（`client-shell-v7`）之间的同源 MessagePort 协议与二进制导出规范。

## 1. 信任边界

- 主题作者只提供声明式 JSON、PNG/JPEG、受限静态 SVG 和静态 TrueType（`.ttf`）字节，不含任何可执行脚本。
- TypeScript 通道（`tools/theme/studio_message_port.ts`）、预览 JS Worker（`theme_contract_worker.js`）、Dart 预览壳（`tools/theme/preview`）与 `wasm-bindgen` 胶水均为受信任的应用代码。
- 父页声称的 `themeSha256` 或素材 SHA-256 仅用于一致性比对，**不授予媒体解析权限**。
- 预览核在自身同源 Rust/WASM Worker 中独立执行闭合 JSON Schema/语义校验以及与客户端导入完全一致的 L4 图片/SVG/字体/九宫格像素检查；校验通过后才允许调用 Flutter 图片解码与 `FontLoader`。
- SVG 仅支持标准命名空间下的路径、基本图形、分组、渐变和本地 clip/paint 引用；最大 1 MiB、512 个元素、32 层深、4096×4096 且不超过 16,777,216 像素。拒绝 DTD/实体、脚本、事件属性、CSS/style、外链、`foreignObject`、滤镜、动画、嵌入式文档和未知属性。Flutter 使用共享解码器按固有尺寸栅格化，气泡与输入框继续使用同一九宫格绘制器。

## 2. 固定握手流程

1. Studio 加载部署生成的 `theme-studio-config.json` / `renderer-release.json`，获取固定的 `rendererReleaseId`、`fixtureVersion` 与 `fixtureSourceSha256`，并先订阅 `window.message`。
2. Studio 将 `iframe.src` 指向同源 `/theme-preview/<rendererReleaseId>/?studioOrigin=<origin>&nonce=<nonce>&rendererReleaseId=<id>`。
3. 预览核在自身 Rust/WASM Worker 启动就绪后，向 `window.parent` 发送 `operit.theme.preview.available.v1`。
4. Studio 通过 `operit.theme.studio.connect.v1` 向子页转移**唯一一个** `MessagePort`。
5. 子页核对 exact origin、`window.parent` 消息源、相同协议头、单一端口且无既有连接后，在私有端口返回 `operit.theme.preview.ready.v1` 与 `nextRevision`。

所有协议消息均携带固定头部：

```text
protocolVersion: 1
nonce: 16～128 个 URL-safe 字符，由 Studio 用加密安全随机源生成
rendererReleaseId: 当前固定的构建版本 ID
```

初始内置样例占用 revision 1；Studio 从 `nextRevision` 开始递增，通道不可重复握手或在致命协议错误后复用。单次 Worker 契约操作超时为 15 秒；首次 Flutter Web 场景帧等待上限为 60 秒。

## 3. 草稿加载、真实帧回执与场景模式

### `operit.theme.preview.load.v1`

```text
revision: 递增安全整数
canonicalThemeJson: Rust normalizeThemeDraft 输出的完整规范化 JSON 字符串
themeSha256: 该规范化 UTF-8 字节的 SHA-256
brightness: "light" | "dark"（预览环境亮暗模式，不写入 theme.json）
viewport: { width, height }（逻辑像素整数：1280×800 / 390×844 / 768×1024 / 1440×900）
showConversation: boolean（固定场景对话开关，Studio 默认开启 true；不写入 theme.json）
assets: [{ path, mediaType, sha256, bytes: Uint8Array }]
```

素材通过 structured clone 的 `Uint8Array` 传输，不使用 base64 或 JSON 整数数组。新 revision 一经接收立即撤销旧回执，执行链路为：

```text
reserve revision → preview-owned Rust/WASM validation
→ verify normalization + immutable asset receipt
→ prepare global/user/AI fonts and glyph coverage + decode images
→ operit.theme.preview.loaded.v1 → actual Flutter post-frame callback → operit.theme.preview.rendered.v1
```

`loaded.v1` 仅表示资源准备完成；`rendered.v1` 在真实 Flutter 帧绘制完成后发出，绑定同一 `revision`、`fixtureVersion`、`fixtureSourceSha256`、`themeSha256`、`brightness`、`viewport`、`showConversation`、完整资产摘要与字体摘要（按 global、probe monospace、user、AI 顺序返回四项）。

### 组件点选（Inspector）与 Stage Color HUD

- Studio 对当前已渲染的 `revision` 发送 `operit.theme.preview.inspectMode.v1`（携带 `revision`、`enabled` 与可空 `targetId`）。
- 预览核注册 6 个可检视部件目标：`background`（画布背景）、`navigation.sidebar`（导航侧栏）、`chat.topBar`（对话顶栏）、`chat.userMessage`（用户消息气泡）、`chat.aiMessage`（AI 消息气泡）与 `chat.composer`（消息输入框），点击后通过 `operit.theme.preview.selection.v1` 回传选中目标。
- Studio 同步联动左侧组件参数面板与画布底部停靠式明暗双色 Stage HUD（`stage-color-hud`，默认启用 Light/Dark 双套色彩角色同屏编辑，停靠在画布下方不遮挡预览输入框）。参数或颜色修改后经防抖重跑 Rust/WASM 校验并渲染新 Flutter revision。
- `operit.theme.preview.sceneMode.v1` 支持在不改 `theme.json` 的前提下切换正常对话（`showConversation: true`）与空白会话（`showConversation: false`），并等待真实帧结束后刷新 `rendered.v1` 回执。

### 帧捕获（`capture.v1`）与确定性导出

- Studio 发送 `operit.theme.preview.capture.v1`（要求当前未处于 Inspector 高亮描边状态，且 `revision` / `viewport` / `showConversation` 与当前 `rendered.v1` 回执完全一致）。
- 预览核调用 Flutter `RepaintBoundary.toImage(pixelRatio: 1)` 截取真实渲染帧，通过 `operit.theme.preview.captured.v1` 返回 `pngBytes` 与 `pngSha256`。
- 当创作者在 Studio 中点击「导出 `.toolpkg`」时，若当前处于暗色、非 1280×800 视口、空白会话或存在未重新渲染的草稿改动，Studio 会自动切换至 **亮色 + 1280×800 桌面视口 + 正常对话场景** 完成真实帧渲染与 `preview.png` 捕获，再调用 Rust/WASM `exportToolPkgFromTheme` 生成确定性 Stored ZIP 归档。

## 4. Rust/WASM 二进制 ABI

`core/crates/plugin/theme/src/wasm.rs` 导出以下绑定：

- `normalizeThemeDraft(themeJson: &[u8]) -> String`：校验并展开编辑态草稿默认值，返回 `canonicalThemeJson` 与 `themeSha256`。
- `validateThemePreviewDraft(themeJson: &[u8], directoryJson: &[u8], payload: &[u8]) -> String`：接收闭合资产目录与连续二进制缓冲区，执行完整契约与 L4 媒体校验，返回规范化 JSON 与 `ThemePreviewValidationReceipt`。
- `exportToolPkgFromTheme(metadataJson: &[u8], themeJson: &[u8], previewPng: &[u8], directoryJson: &[u8], payload: &[u8]) -> Result<Vec<u8>, JsValue>`：生成固定顺序、固定 DOS 时间戳与权限的 Stored ZIP（`.toolpkg`），并在返回前立刻用导入校验器（`validate_toolpkg_theme_templates`）对生成字节执行完整自检。
- `validateAuthoredToolPkg(archiveBytes: &[u8], manifestPath: &str) -> String`：导出已有 ToolPkg 时，对合并后归档的所有模板执行校验；支持重写为 JSON 的嵌套 manifest 路径。校验失败时不提供下载。

## 5. 构建与验证命令

生成不可变预览核版本（需显式传入与应用一致的 Flutter SDK 与匹配 `core/Cargo.lock` 的 `wasm-bindgen` CLI）：

```powershell
.venv/Scripts/python.exe tools/theme/build_preview.py --renderer-release-id <release-id> --flutter-sdk <flutter-sdk-root> --wasm-bindgen <wasm-bindgen-cli-path>
```

协议与契约测试入口：

```powershell
cargo test --manifest-path core/Cargo.toml -p operit-plugin-theme
node --experimental-strip-types --test tools/theme/studio_message_port.test.ts
```

编辑既有 ToolPkg 时，Studio 将所选模板的渲染结果写回对应声明，保留其他脚本、资源和模板；原包及所选模板 ID 随本地草稿一起保存在 IndexedDB。
