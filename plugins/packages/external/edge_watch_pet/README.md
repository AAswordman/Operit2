# 手表桌宠

管理已加入 Operit 设备空间的 Android Edge 桌宠。插件提供同款圆屏预览、三种配色、点按触感和首页亮屏开关。点「保存到手表」后，手表保存设置并更新桌宠。

## 使用

1. 通过 Operit 的「设备空间」完成手表配对和加入，并保持两台设备直连。[手表端接入](../../../../edge/boards/android_phone/README.md)
2. 导入 `edge_watch_pet.toolpkg` 并启用「手表桌宠」，从侧栏打开。首次选择时输入手表名称或节点 ID。名称含空格或存在重名时，可长按手表桌宠进入「设备空间 → 手动配对」查看节点 ID。
3. 调整配色和开关后保存。插件记住上次选中的设备，点「切换」可选择另一块表。

插件通过现有 Edge 桥接先解析设备身份，再用 `Tools.device(nodeId).Edge.execute` 访问 `edge.watchPet`。手表仅接受同空间当前成员的直连设置请求。`getConfig` 返回当前值，`setConfig` 接收完整的版本 1 配置，包含 `accent`、`haptics` 和 `keepAwake`。WebView 所用资源随包分发。

## 单包打包

在仓库根目录运行以下命令。

```powershell
tsc -p plugins/packages/external/edge_watch_pet/tsconfig.json
.venv/Scripts/python.exe -X utf8 -c "from pathlib import Path; from plugins.tools.sync_plugin_packages import _pack_toolpkg_folder; root=Path.cwd(); _pack_toolpkg_folder(root, root/'plugins/packages/external/edge_watch_pet', root/'plugins/.out/edge_watch_pet.toolpkg')"
```
