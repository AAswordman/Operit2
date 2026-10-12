# Theme v1 契约与渲染映射

状态：v1 契约实现与客户端/Studio 共享渲染映射说明。

## 1. 唯一契约源

- [manifest-v1.schema.json](../core/crates/theme/contract/schema/manifest-v1.schema.json)
- [theme-v1.schema.json](../core/crates/theme/contract/schema/theme-v1.schema.json)
- Rust：`core/crates/theme/contract`，crate 为 `operit-theme-contract`。
- Flutter：`apps/flutter/packages/operit_theme`；独立预览入口为 `apps/flutter/theme_preview`。
- 展开后的共同样例：`operit_theme/assets/fixtures/theme-v1.json`。Rust 测试直接引用此文件，独立预览通过 package asset 读取，不另存一套主题默认值。

Schema 为闭合的 JSON Schema 2020-12 文档，Rust 只解释这些受信任文档实际使用的关键字子集，不接受作者提供的 schema、外部 `$ref` 或脚本。运行时使用与 schema 相同的属性、枚举、范围和默认值定义，跨字段关系、路径和版本另外检查。

`theme.json` 自带整数 `themeApiVersion: 1`；`manifest.json` 的 `formatVersion` 与 `themeApiVersion` 分别固定为整数 `1`。`1.0` 和 `1.5` 均不被版本/字节数字段接收。

### 草稿和发行包不是同一输入

`normalize_draft` 接受省略了 **schema 内定义默认值** 的编辑态字段；但不能省略 `themeApiVersion`，也不能用省略的 `image.asset` 创建图片。显式非法值不会被替换。`parse_theme` 要求所有展开后的视觉字段，不能把不完整发行包当作有效主题。

当前固定配色为 schema 中亮色与暗色两套各 46 个 Material 颜色角色。v1 发行包不接受 seed 或任意颜色角色名；Studio 可用 seed 辅助生成，但导出时必须展开为两套明确的 46 色角色。公开颜色为大写 `#RRGGBBAA`，Dart 显式转换为 Flutter 的 `AARRGGBB`。

v1 规范化摘要的输入为：字典键显式排序、紧凑 UTF-8 JSON、视觉 number 中的整数值以整数写出（包括负零）。不依赖 `serde_json` 的 `preserve_order` feature。该规范只用于本契约；后续市场精选 attestation 的 RFC 8785 JCS 是另外的签名编码规则。

### manifest 和许可证的具体形状

- `author` 当前为 `{ "name": "作者署名" }`；没有可执行内容、主页 URL 或认证含义。
- `platforms` 必须恰好覆盖 `android` / `ios` / `windows` / `macos` / `linux` / `ohos` / `web`，不提供平台专属样式。
- `licenses/theme.txt` 必须存在并登记在 `files`；每个打包的图片或字体都必须有一个 `assetLicenses` 项，声明 `asset`、SPDX ID 和包内许可证原文路径。
- 首版 SPDX ID 白名单为 `MIT`、`Apache-2.0`、`AGPL-3.0-only`、`AGPL-3.0-or-later`、`CC0-1.0`、`CC-BY-4.0`、`CC-BY-SA-4.0`、`OFL-1.1`，不解析任意 SPDX expression。授权真实性仍需要人工审核，填写 ID 不等于证明拥有素材权利。
- 包内资源与引用集合必须相等；不接收未使用的素材、未引用的许可证或额外代码文件。
- 图片素材支持 PNG/JPEG 与受限静态 SVG。SVG 最多 1 MiB、512 个元素、32 层深、固有宽高各不超过 4096 且不超过 16,777,216 像素。只接受标准 SVG 命名空间中的基本图形/路径、分组、渐变和本地引用；拒绝脚本、事件属性、CSS/style、DTD/实体、外链、滤镜、动画、`foreignObject`、嵌入文档和未登记的元素/属性。SVG 由共用 Flutter 解码器按固有尺寸栅格化，气泡与输入框仍由同一个九宫格绘制器渲染。

## 2. Snapshot 映射（101 字段）

内部 `ThemePreferenceSnapshot` 共包含 **101 字段**（原 78 字段 + 输入框底色/圆角/九宫格贴图 13 字段 + 输入框右侧 5 个动作按钮贴图与尺寸 10 字段）。类型与 JSON 传输语义位于共用包 `operit_theme`；应用 `UserPreferencesManager.dart` 重新导出同一个类型，默认快照也直接引用共用定义。

| 公开字段或边界 | Snapshot 字段 / 渲染输入 | 处理 |
| --- | --- | --- |
| `palette.light` / `dark` 的 46 角色 | 渲染根的 `lightColorScheme` / `darkColorScheme` | 各角色直接传入 `ColorScheme`；不按新 SDK 重新生成颜色。 |
| `palette` 的内部投影 | `useCustomColors`、`customPrimaryColor`、`customSecondaryColor` | 固定启用自定义配色，后两项取浅色 palette 的 primary/secondary；完整 palette 不压缩进这三个字段。 |
| `background.asset` | `useBackgroundImage`、`backgroundImageUri` | `null` 明确表示不使用背景；非空为包内引用。 |
| 静态图片范围 | `backgroundMediaType`、`videoBackgroundMuted`、`videoBackgroundLoop` | `mediaType` 固定 `image`；视频参数不进入公开契约。 |
| `background.opacity` / `blur.enabled` / `blur.radius` | `backgroundImageOpacity`、`useBackgroundBlur`、`backgroundBlurRadius` | 直接映射。`fit=cover`、`alignment=center` 为 v1 固定行为。 |
| `surfaces.transparent` | `transparentSurfaceEnabled` | 共用玻璃/输入/气泡表面使用同一启用状态；不提供任意材质表达式。 |
| `input.style` / `floating` | `inputStyle`、`chatInputFloating` | `classic`/`agent`；浮动布局由客户端布局负责。 |
| `input.color` / `roundedCorners` | `inputBackgroundColor`、`inputRoundedCornersEnabled` | `#RRGGBBAA` 转 ARGB（`null` 时使用登记部件的 palette 角色）；布尔值控制输入框圆角。 |
| `input.image.asset` | `inputUseImage`、`inputImageUri` | `image=null` 明确禁用输入框贴图；非空为包内图片/SVG 引用。 |
| `input.image.renderMode` / `crop.*` / `slice.*` / `scale` | `inputImageRenderMode`、`inputImageCropLeft/Top/Right/Bottom`、`inputImageRepeatStart/End/YStart/YEnd`、`inputImageScale` | `tiled_nine_slice` / `nine_patch`；裁切 0～0.45，切片满足 `end >= start + 0.01`，缩放 0.25～3。 |
| `input.settingsAsset` / `attachmentAsset` / `micAsset` / `sendAsset` / `stopAsset` | `inputSettingsAsset`、`inputAttachmentAsset`、`inputMicAsset`、`inputSendAsset`、`inputStopAsset` | 分别替换输入框右侧的「设置」「添加附件」及最右侧主按钮的「语音输入」「发送」「停止」三个状态贴图（PNG/JPEG/静态 SVG），`null` 时保持默认图标。 |
| `input.settingsAssetSize` / `attachmentAssetSize` / `micAssetSize` / `sendAssetSize` / `stopAssetSize` | `inputSettingsAssetSize`（默认 18，12～44）、`inputAttachmentAssetSize`（默认 18，12～44）、`inputMicAssetSize`（默认 26，12～48）、`inputSendAssetSize`（默认 26，12～48）、`inputStopAssetSize`（默认 26，12～48） | 分别控制上述 5 个自定义按钮贴图的显示尺寸（单位 px）。 |
| `chat.style` / `showAvatar` / `wideLayout` | `chatStyle`、`bubbleShowAvatar`、`bubbleWideLayoutEnabled` | `cursor`/`bubble`；头像内容不是主题数据。 |
| `chat.cursorUserColor` | `cursorUserBubbleColor` | `null` 表示采用登记部件的 palette 角色。 |
| `chat.user` / `ai` 的 `color` / `textColor` | `bubbleUserBubbleColor`、`bubbleAiBubbleColor`、`bubbleUserTextColor`、`bubbleAiTextColor` | RGBA 转 ARGB；`null` 是契约中明确的角色配色语义。 |
| `chat.user` / `ai` 的 `image.asset` | `bubbleUserUseImage`、`bubbleAiUseImage`、`bubbleUserImageUri`、`bubbleAiImageUri` | `image=null` 明确禁用图片；资源加载错误不是此语义。 |
| `chat.user` / `ai` 的 `image.renderMode` | `bubbleUserImageRenderMode`、`bubbleAiImageRenderMode` | `tiled_nine_slice` / `nine_patch`。 |
| `chat.user` / `ai` 的 `image.crop.*` | `bubbleUserImageCropLeft/Top/Right/Bottom`、`bubbleAiImageCropLeft/Top/Right/Bottom` | 直接映射（0～0.45）。 |
| `chat.user` / `ai` 的 `image.slice.*` | `bubbleUserImageRepeatStart/End/YStart/YEnd`、`bubbleAiImageRepeatStart/End/YStart/YEnd` | 直接映射，检查每条 `end >= start + 0.01`。 |
| `chat.user` / `ai` 的 `image.scale` | `bubbleUserImageScale`、`bubbleAiImageScale` | 直接映射（0.25～3）。 |
| `chat.user` / `ai` 的 `roundedCorners` | `bubbleUserRoundedCornersEnabled`、`bubbleAiRoundedCornersEnabled` | 直接映射。 |
| `chat.user` / `ai` 的 `padding.left` / `right` | `bubbleUserContentPaddingLeft/Right`、`bubbleAiContentPaddingLeft/Right` | 0～32。 |
| `chat.avatarShape` / `avatarCornerRadius` | `avatarShape`、`avatarCornerRadius` | `circle`/`square`；半径 0～32。 |
| `typography.global` | `useCustomFont`、`fontType`、`systemFontName`、`customFontPath` | builtin ID（`noto-sans-sc` / `cascadia-mono`）或 `assets/fonts/*.ttf` 引用；真实家族名由准备好的字体字节提供。 |
| `typography.user` / `ai` | `bubbleUserUseCustomFont`、`bubbleUserFontType`、`bubbleUserSystemFontName`、`bubbleUserCustomFontPath`；对应 Ai 四项 | `null` 明确沿用全局字体；不能作为字体加载异常的处理。 |
| 节点模式，不在包内 | `themeMode` | 从调用方节点设置保留。 |
| 用户字体缩放，不在包内 | `fontScale` | 从调用方节点设置保留。 |
| 用户头像内容，不在包内 | `customUserAvatarUri` | 从调用方节点设置保留。 |
| 用户信息显示，不在包内 | `showThinkingProcess`、`showModelProvider`、`showModelName`、`showRoleName`、`showUserName`、`showMessageTokenStats`、`showMessageTimingStats`、`showMessageTimestamp`、`showInputProcessingStatus` | 从调用方节点设置保留；主题无权隐藏这些信息。 |

`ThemeVisualSpecV1.fromCheckedContract` 是已检查 JSON 的 Dart 投影，**不是第二套包校验器**。公开主题的完整 `ColorScheme` 与准备好的字体家族通过 `ThemeRenderRoot` 直接进入 `buildOperitThemeData`；旧应用的双种子色派生和平台字体判断不作为公开主题的渲染路径。

### 包锁定字段 vs 节点可微调字段（`kUserConfigurableThemeFields`）

当节点应用某个主题包时（`applyTheme`），Store 以该主题包展开后的默认外观值初始化本节点的 `privateAppearanceJson`。随后在主题包生效期间：

- **节点可微调字段（`kUserConfigurableThemeFields`，共 49 项）**：包含头像显隐/形状/圆角（`bubbleShowAvatar`、`avatarShape`、`avatarCornerRadius`）、聊天与输入布局样式（`chatStyle`、`bubbleWideLayoutEnabled`、`inputStyle`、`chatInputFloating`）、透明与背景参数（`transparentSurfaceEnabled`、`backgroundImageOpacity`、`useBackgroundBlur`、`backgroundBlurRadius`）、以及用户/AI 气泡与输入框的颜色、圆角、内边距和九宫格裁切/切片/缩放参数。用户可在客户端外观设置中按节点微调这些字段，`ThemeVisualSpecV1.toSnapshot(..., preserveUserSettings: true)` 会保留节点覆盖值。
- **主题包只读锁定项**：包内受检媒体引用路径（`background.asset`、`chat.user.image.asset`、`chat.ai.image.asset`、`input.image.asset`、`input.*Asset`、`typography.*`）与亮暗两套各 46 个 Material 颜色角色（`palette.light` / `palette.dark`）始终由已校验的主题包权威提供，不允许将私有 runtime 路径混入公开主题包资源作用域。

## 3. 共用渲染层与分层校验

已提取到 `apps/flutter/packages/operit_theme` 的共享组件：

- `ThemePreferenceDefaults`、`ThemePreferenceSnapshot`、`ThemeVisualSpecV1`（含 `kUserConfigurableThemeFields`）
- `ThemeAssetResolver` / `MemoryThemeAssetResolver`、`ThemeAssetScope`、`ResolvedThemeImage`、`ThemeImageDecoder`（PNG/JPEG + 静态 SVG 栅格化）
- `ThemeFontRegistry`、`BuiltinThemeFonts`（固定内置字体字节目录与 SHA-256 核对）、`ThemeFontGlyphs`
- `buildOperitThemeData`、`ThemeRenderRoot`、`ThemeBackgroundLayer`、`ThemeGlassSurface`
- `BubbleImageStyle`、`NineSliceBubbleSurface`、`BubbleSurface`
- `ChatInputSurface`、`ThemeComposerChrome`、`ThemeTopBarChrome`、`ThemeDrawerChrome`、`ThemeHistoryChrome`、`ThemeDesktopSidebarSurface`
- `ThemeClientScene`（`client-shell-v6`）与 `ThemeSurfaceProbe`（`surfaces-v1`）

### L0–L4 校验入口（`operit-theme-contract`）

- `validate_theme_package` / `validate_theme_package_with_expectation`：在同一 fail-closed 入口内完成原始 ZIP 目录检查（L0/L1）、JSON Schema 与语义检查（L2）、文件集合/字节数/SHA-256/SPDX 许可证闭合检查（L3）、以及 PNG/JPEG/受限静态 SVG 解码、九宫格像素区间和静态 TrueType 表/`loca`/`cmap`/轮廓遍历（L4）。
- `ValidatedThemePackage` 保留原始 sealed archive 并返回 `ThemeValidationReceipt`；`trust` 明确为 `Untrusted`。
- `validate_theme_preview_transfer` / `export_theme_package_from_transfer`：使用闭合 JSON 目录 + 连续二进制缓冲区，供原生与 Web Worker 共用。
