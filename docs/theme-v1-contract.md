# ToolPkg 的 theme 模板能力

ToolPkg 的 manifest 可以通过 `themes` 声明多个主题模板，并与 `main`、`subpackages`、`resources`、`workspace_templates` 等能力共存。发行格式只有 `.toolpkg`；安装、作用域、同步、更新和卸载全部由现有包管理系统负责。

视觉定义见 [theme-v1.schema.json](../core/crates/plugin/theme/schema/theme-v1.schema.json)，模板声明见 [theme-template.schema.json](../core/crates/plugin/theme/schema/theme-template.schema.json)。模板声明 schema 只约束 `themes` 的一项，不定义另一份包 manifest。

```json
{
  "schema_version": 1,
  "api_version": "2.0.0",
  "toolpkg_id": "org.example.tools",
  "version": "1",
  "main": "main.js",
  "license": "licenses/agreement.txt",
  "themes": [{
    "id": "night",
    "path": "templates/night/visual.json",
    "preview": "previews/night.png",
    "files": [
      {"path": "theme.json", "mediaType": "application/json"},
      {"path": "preview.png", "mediaType": "image/png"}
    ]
  }]
}
```

`id` 在同一个包中唯一；节点选择使用 `toolpkg_id:template_id` 身份。`path` 和 `preview` 是 manifest 所在目录的相对路径。资源清单中的 `theme.json`、`preview.png` 对应这两个显式引用；`assets/images/*` 和 `assets/fonts/*` 相对于视觉定义所在目录解析。回执中的资源路径是渲染器使用的模板内路径。

`files` 只登记该模板引用的资源路径和媒体类型，无需填写字节数或 SHA-256。旧包残留的 `bytes`、`sha256` 字段会被忽略；文件大小限制根据实际资源检查，内部缓存摘要由系统自动计算。模板校验只读取这些资源；包内其他脚本、模板、协议文本和资源不受主题文件白名单限制。

模板及视觉定义不包含平台列表、协议字段、主题 API 版本或最低／最高应用版本。包的 `api_version` 和 `version` 沿用 ToolPkg 本身的含义，主题模块不增加版本格式要求。可选的包级 `license` 引用包内协议原文，使用普通安全相对路径，不要求 SPDX 枚举或每个图片、字体另附协议。

`normalize_draft` 按视觉 schema 展开编辑态默认值；`parse_theme` 校验完整视觉状态。显式非法值、未知视觉字段、远程资源路径及非法九宫格切片会报错。

Rust `operit-plugin-theme` 为每个模板检查资源存在性、实际大小、视觉关系、PNG/JPEG 解码、受限静态 SVG、九宫格像素范围及静态 TrueType 字体结构。`ValidatedThemeTemplate` 和 `ThemeValidationReceipt` 只表示对应模板的渲染校验结果，`trust` 为 `Untrusted`。

`ToolPkgArchiveParser::readThemeTemplates` 通过普通 ToolPkg manifest 解析器枚举模板，再调用 `validate_declared_theme_templates` 校验其资源；没有 `themes` 的普通包返回空集合。主题模块不独立寻找 manifest，也不执行脚本、安装包或创建独立归档存储。`validate_toolpkg_theme_templates` 和 `validate_toolpkg_theme_template` 用于检查 Studio 明确生成的根目录 manifest 输出。

Studio 通过 `export_toolpkg_from_theme_transfer` 创建包含模板的普通 ToolPkg。编辑既有包时保留其他 manifest 能力、其他模板及原有资源；导出只更新所选模板及用户修改的包元数据。

## 从模板创建外观配置

客户端沿用原有外观设置和命名配置管理。页面只有配置选择、新建、重命名、删除，以及主题、背景、气泡、交互四个编辑分区。

新建时可以选择默认外观、当前外观、继承已有配置，或者使用已安装 ToolPkg 声明的模板。模板来源每次打开新建对话框时通过 `packageManager().getToolPkgContainerRuntimes()` 动态读取；安装、卸载、更新和 device / space 归属由普通插件管理系统负责。

选择模板后，`previewToolPkgTheme(packageName, templateId)` 读取当前注册包，验证模板和媒体，返回规范化视觉 JSON 与资源。`ThemeTemplateService` 将模板图片和字体复制到既有 `runtime/data/user_assets/theme` 资源池，把包内相对引用替换为外观资源引用，再通过原有 `ThemeConfigManager.create` 和 `apply` 创建并选中独立配置。

创建不会修改继承来源或此前选中的命名配置。新配置的颜色、背景、字体、输入样式和气泡都可以继续编辑。完整明暗配色作为可选的 `themePaletteJson` 外观字段保存；用户重新选择配色时清除完整配色，使用原有种子配色逻辑。

配置和复制后的资源沿用原有 Space 同步。后续渲染读取配置与外观资源，不再依赖来源插件是否安装，也不会随来源模板更新而改写配置。因此没有独立主题库、共享模板选择、只读锁定、主题专用卸载约束或模板恢复页面。

普通 ToolPkg 的 `themes` 声明、模板校验、共享 Flutter 渲染，以及 Studio 编辑和导出继续保留。

外观资源或配色加载失败时，客户端展示明确错误及“重试”“重置外观”操作。重试读取原有值；重置只由用户点击触发，保留头像与已保存配置，并显式选中自定义外观。错误不会触发自动替换。应用旧版配置时清除其没有声明的新增外观字段，避免沿用此前模板的输入框样式。
