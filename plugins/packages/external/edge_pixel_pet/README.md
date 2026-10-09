# 像素桌宠 Core ToolPkg 示例

随 Core 分发在「内置更多包」，**不会默认导入或开启**。使用现有 ToolPkg 运行时与 `Tools.Edge.execute`，不在 Edge 执行 JS。
协议见 `plugins/docs/edge-scene.md`。原件保存在本包的 `resources/demo.esp`；
生成器、源代码和编译后的 CommonJS 同包分发。

## 单包构建（从仓库根目录）

```powershell
python plugins/packages/external/edge_pixel_pet/generate_assets.py
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
设 `EDGE_SCENE_NODE_ID` 为已授权的 Edge ID（`test` 使用该变量），或为其他工具显式传 `node_id`。

- `edge_pixel_pet:start_pet`：查询能力、申请 60 秒租约、分块校验安装素材并显示宠物与道具。
- `edge_pixel_pet:renew_pet`：只续显示租约，不查询输入；已退出时停止。
- `edge_pixel_pet:pet_status`：只读 Core 已保存的互动次数、游标和退出状态，不访问 Edge。
- `edge_pixel_pet:close_pet`：释放区域，保留 Core 数据和设备缓存。
- `edge_pixel_pet:test`：只查原生能力，不占屏幕。

点击宠物／道具后 Edge 主动调用 Core 的固定通用 `chatEdgeEvent` 入口，`on_edge_event` 在原 main 运行时持久化后 ACK。
空闲时不发送事件请求、不写配置。`events.poll` 仅保留为固件的手动诊断，示例不再使用。
如需持续显示，可每 20 秒执行一次 renew_pet（只保活）；触摸与退出无需等待保活。
系统右上角退出和 TTL 到期均停止场景，不自动重开。
首版缓存不是重启持久化缓存，Core 原件不丢；重新 start 会按 SHA 判断／重装。
示例是单 Core 控制器，不提供分布式养成事务。

事件使用通用 `EdgeEventBatch`，本插件仅消费 `source:"display.scene"`，按 action 分发。
其他插件可监听传感器、GPIO、串口或其他固件来源；通用协议见 `plugins/docs/edge-events.md`。

工具使用既有同包 `ToolPkg.ipc` 进入 main actor，与事件回调共用一个串行队列和配置缓存。关闭后保留 lease/cursor 去重 tombstone；重发 ACK 不重复计数或写盘。

插件卡片的连通性／工具调用测试复用既有 `test_connection` 和 `test_tool_call` 导出，仅调用原生 capabilities，不申请区域；需要设置 `EDGE_SCENE_NODE_ID`。
