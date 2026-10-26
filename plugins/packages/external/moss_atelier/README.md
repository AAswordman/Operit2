# 苔庭 · Moss Atelier

这是一个普通的 ToolPkg 示例，提供两套可以直接从外观设置新建的工作区模板：

- `moss`：苔绿、米白和浅青灰，适合长时间使用。
- `dune`：暖砂、陶土和奶油白，视觉更温暖。

每套模板都同时声明了亮色和暗色配色，带有低对比度的纸张纹理背景、气泡式消息和悬浮输入框。预览图由共享 Flutter 工作区界面实际渲染生成，尺寸为 1280×800。

## 目录

- `manifest.json`：普通 ToolPkg 清单；模板通过 `themes` 声明。
- `templates/*/theme.json`：模板定义。
- `templates/*/preview.png`：模板预览图。
- `templates/*/preview-dark.png`：暗色预览，供查看；模板清单使用亮色预览作为入口。
- `templates/*/assets/images/ambient.png`：模板背景素材。
- `resources/logo.svg`：包图标。

## 加载

导入同目录下生成的 `build/moss_atelier.toolpkg`，然后在外观设置的新建主题流程中选择“苔庭”包里的 `moss` 或 `dune` 模板。它们使用 ToolPkg 的普通模板能力，没有单独的主题包格式。
