# 客户端全局主题库与 Core 接口

状态：客户端 Space 主题库、CoreNode 节点选择与多窗口同步接口说明。

## 产品与架构边界

- **全局应用外观**：主题不绑定角色卡、群组或 `ActivePrompt`；角色卡插件化不影响主题选择。
- **不迁移历史角色外观**：不读取旧角色主题前缀、不创建迁移 journal，也不删除历史数据。
- **Space 与 CoreNode 职责分离**：`Space` 保存封存的 `.optheme` 主题包与库元数据（`runtime/themes/library`），`CoreNode` 保存本节点当前主题选择与节点级外观微调（`runtime/config/preferences/theme_selection.preferences.json`）；同节点多窗口通过 `nodeThemeStateFlow` 观察同一份节点状态。
- **信任状态**：结构与媒体校验（L0–L4）通过的本地导入包信任状态为 `Untrusted`，不代表发布者认证、市场审核或官方精选签名。

## 四个独立动作

### 1. 仅导入（Import）

系统文档输入 → 有界读取（64 MiB）→ Core `inspectThemeArchive` → 展示文件名、字节数和 SHA-256 → 用户确认 → Core 按确认的字节数/摘要执行 `importThemeArchive`（expectation + L0–L4）→ Space 两阶段提交 → 返回 `ThemeInstallReceipt`。

- 导入不自动切换本节点当前主题。`Imported` / `AlreadyPresent` 回执的 `selection` 为空、`selectionChanged` 为 `false`。
- 同 `themeId` + `version` 但字节摘要不同的归档拒绝覆盖，要求发布新版本号。

### 2. 独立沙箱预览（Preview）

读取 Space 中的封存归档或待检字节 → Core 重新执行 L0–L4 校验（`readThemePackage` / `previewThemeArchive`）→ 返回带 `themeId`/`version`、规范化 `themeJson`、校验回执及二进制资产的 `ThemePreviewPackage` → 客户端核对摘要、解码图片并按内容摘要注册字体 → 在独立 `ThemeAssetScope` / `ThemeRenderRoot` 中渲染。

- 预览弹窗支持切换「完整界面（`ThemeClientScene`）」与「会话组件（`ChatAppearancePreview`）」以及预览内亮/暗模式切换，关闭后释放内存资源，不修改本节点选择。
- 主题库列表卡片与顶部节点控制台通过 `ThemeLibraryService.readBackgroundImageBytes` 在同一 `previewThemeArchive` 校验边界下提取 `background.asset` 字节，生成并缓存圆角矩形背景缩略图。

### 3. 显式应用到本节点（Apply）

用户确认所选 `themeId`/`version`/字节数/SHA-256 → 客户端完成资源准备（`PreparedThemeVisual`）→ Core 再次读取库中封存字节执行完整 L0–L4 校验（`applyTheme`）→ Store 凭 `ValidatedThemePackage` 原子写入本节点 `selection` 并用包默认值初始化 `privateAppearanceJson` → 返回 `Applied` 回执 → 同节点窗口同步生效。

- 库中的新版本不会自动替换本节点已选版本。
- 应用主题包后，节点用户仍可调节 `themeMode`、`fontScale`、信息显示开关以及 `kUserConfigurableThemeFields` 中的 49 项外观参数；切换到「个人外观」通过 `selectPersonalThemeAppearance()` 仅清除本节点 `selection`，不删除 Space 中的主题包。

### 4. 从 Space 主题库删除（Delete）

用户在主题库触发删除 → 若该主题正在本节点生效，提示用户确认「先切回个人外观并删除」（`OperitThemeController.deleteLibraryTheme(..., unselectIfActive: true)`）→ Core `deleteTheme(themeId, archiveSha256)` 校验身份并确认未被本节点选中 → 移除 `entry.json`、`package.optheme`、`pending.json` 并通过 `RuntimeFileSyncStore::delete` 写入 Space 同步删除墓碑（tombstone），同时清理客户端归档与背景缩略图缓存。

## 节点状态与容错行为

节点文件：`runtime/config/preferences/theme_selection.preferences.json`（归属 `CoreNode`）。

`ThemeNodeState`：

| 字段 | 含义 |
| --- | --- |
| `schemaVersion` | 节点主题状态版本，当前为 `1` |
| `selection` | 已选包的 `schemaVersion` / `themeId` / `version` / `archiveSha256`；为 `null` 表示个人外观模式 |
| `privateAppearanceJson` | 节点级 `ThemePreferenceSnapshot` JSON（个人外观设置或当前主题包下的节点可微调字段覆盖值） |

`listThemeLibrary()` 为每个条目独立报告状态：

- `Ready`：库元数据与封存归档字节/SHA-256 完整匹配（应用和预览时仍执行完整 L0–L4 重验）。
- `WaitingForSync`：元数据已同步到达、归档文件尚未到齐或正在同步。
- `Invalid`：元数据或归档损坏；单条损坏不会阻断其余正常条目的展示。

当本节点已选主题包处于 `WaitingForSync` 或 `Invalid` 时，客户端保留节点选择并展示恢复屏障（支持「重新校验选择」或「显式切换回个人外观」），不静默回退到其他主题。

## Core 与客户端公开接口

### Rust Core（`OperitApplication` / `ThemeLibraryStore`）

| 方法 | 参数 | 返回类型 | 说明 |
| --- | --- | --- | --- |
| `inspectThemeArchive` | `archiveBytes: Vec<u8>` | `ThemeArchiveInspection` | 有界 L0/L1 ZIP 预检，返回整包字节数、SHA-256 与条目摘要供确认框展示 |
| `importThemeArchive` | `archiveBytes: Vec<u8>, expectedArchiveBytes: u64, expectedArchiveSha256: String` | `ThemeInstallReceipt` | 核对用户确认的字节数/SHA-256 后执行 L0–L4 校验并写入 Space 主题库（不改节点选择） |
| `previewThemeArchive` | `archiveBytes: Vec<u8>` | `ThemePreviewPackage` | 对归档字节执行完整 L0–L4 校验并返回规范化 `themeJson`、回执与内存资产列表 |
| `readThemePackage` | `themeId: String, archiveSha256: String` | `ThemePreviewPackage` | 从 Space 库读取指定主题包并重新执行完整 L0–L4 校验后返回预览/渲染载荷 |
| `applyTheme` | `themeId: String, archiveSha256: String` | `ThemeInstallReceipt` | 重新完整校验库中主题包，原子更新本节点 `selection` 并用包默认值初始化节点外观 |
| `deleteTheme` | `themeId: String, archiveSha256: String` | `()` | 从 Space 主题库删除未在本节点生效的主题包并写入同步删除墓碑 |
| `listThemeLibrary` | 无 | `Vec<ThemeLibraryEntry>` | 列出 Space 主题库全部条目（含 `Ready` / `WaitingForSync` / `Invalid` 状态） |
| `listThemes` | 无 | `Vec<ThemeLibraryEntry>` | 仅返回完整可用条目的严格视图 |
| `nodeThemeState` | 无 | `ThemeNodeState` | 读取本节点当前主题选择与 `privateAppearanceJson` |
| `nodeThemeStateFlow` | 无 | `Flow<ThemeNodeState>` | 订阅本节点主题状态变更流（跨同节点多窗口实时同步） |
| `initializeNodeThemeAppearance` | `snapshotJson: String` | `ThemeNodeState` | 首次初始化尚未存在的节点个人外观 JSON |
| `updateNodeThemeAppearance` | `updates: BTreeMap<String, Value>` | `ThemeNodeState` | 原子合并更新本节点外观字段 |
| `selectPersonalThemeAppearance` | 无 | `ThemeNodeState` | 显式清除本节点主题包选择，恢复个人外观模式 |

### Flutter 客户端服务与控制器（`ThemeLibraryService` / `OperitThemeController`）

- `ThemeLibraryService.inspectArchive(archiveBytes)` / `importArchive(...)` / `previewArchive(archiveBytes)` / `readPackage(themeId, archiveSha256)` / `applyTheme(themeId, archiveSha256)` / `deleteTheme(themeId, archiveSha256)`
- `ThemeLibraryService.readBackgroundImageBytes(themeId, archiveSha256)` / `cachedBackgroundImageBytes(themeId, archiveSha256)`：通过 `previewThemeArchive` 校验边界提取 `theme.json` 中 `background.asset` 声明的自定义背景图片字节并缓存，供主题库卡片圆角缩略图零延迟复用。
- `OperitThemeController.applyLibraryTheme(themeId, archiveSha256)` / `selectPersonalAppearance()` / `deleteLibraryTheme({required themeId, required archiveSha256, bool unselectIfActive = false})` / `revalidateSelectedTheme()`
