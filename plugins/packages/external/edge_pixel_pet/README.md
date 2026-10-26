# 像素桌宠设备配置器

随 Core 分发在「内置更多包」，**不会默认导入或开启**。使用现有 ToolPkg 运行时与 `Tools.device(nodeId).Edge.execute`，不在 Edge 执行 JS。
协议见 `plugins/docs/edge-scene.md`。原件保存在本包的 `resources/demo.esp`；
生成器、源代码和编译后的 CommonJS 同包分发。

## 单包构建（从仓库根目录）

```powershell
python -X utf8 plugins/packages/external/edge_pixel_pet/generate_assets.py
node tools/esp32-editor/node_modules/typescript/bin/tsc -p plugins/packages/external/edge_pixel_pet/tsconfig.json
python -X utf8 -c "from pathlib import Path; from plugins.tools.sync_plugin_packages import _pack_toolpkg_folder; root=Path.cwd(); _pack_toolpkg_folder(root, root/'plugins/packages/external/edge_pixel_pet', root/'core/crates/runtime/application/assets/plugins/external/edge_pixel_pet.toolpkg')"
```

也可以使用既有 `sync_plugin_packages.py --source external --no-hot-reload` 流程。
单包方式不重打包其他内置插件。

在 Core 的「内置更多包」导入并显式启用 `com.operit.edge_pixel_pet`；也可以使用 CLI：

```text
core package more
core package load com.operit.edge_pixel_pet
core plugin enable com.operit.edge_pixel_pet
```

同一 `.toolpkg` 仍可独立导入，用于测试或市场分发。
设 `EDGE_SCENE_NODE_ID` 为已授权的 Edge ID，或为其他工具显式传 `node_id`。

- `edge_pixel_pet:inspect`：查询设备的原生显示能力。
- `edge_pixel_pet:apply_configuration`：上传并校验资源，将桌宠显示配置保存到设备。
- `edge_pixel_pet:clear_configuration`：清除保存配置，恢复系统界面。

本包安装在手机或其他可运行 ToolPkg 的宿主上，只承担配置编辑器的作用。设备负责渲染、点击命中和动作，配置包退出、禁用或卸载后设备仍能继续运行。没有 keepalive、远程点击回调或宿主管理的互动状态。
