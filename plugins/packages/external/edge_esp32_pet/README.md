# ESP32 任务猫

独立安装在本机 Operit Core 上的 ToolPkg。ESP32 是只负责显示的 Edge 节点，不运行插件脚本。插件参考手表桌宠的任务同步方式，使用设备的通用 `edge.scene` 原生接口。

## 使用

1. 在本机「设备空间」配对 ESP32，并加入同一空间。双方固件必须支持相同 Link 协议；出现 `PEER_PROTOCOL_UPGRADE_REQUIRED` 时先更新通用 Edge 固件。
2. 导入并启用 `edge_esp32_pet.toolpkg`，从插件侧栏打开「ESP32 任务猫」。
3. 只有一台在线 ESP32 时自动连接；多台时选择显示设备。按当前状态上传约 4–8 KB 素材，成功后开始显示。
4. 插件订阅真实任务和审批状态，关闭设置页后继续同步。点击「停止展示」恢复设备系统界面。

任务优先级为待批准、异常、运行中、完成、空闲；完成播放三秒后回到空闲。取消任务直接回到空闲，其他任务仍运行时不会提前庆祝。仅传输动画状态和资源，不传聊天内容或审批正文。审批仍在本机原有界面处理。

## 试验版范围

素材来自 `../task_status_pet/resources` 的六套原版动画，由 `generate_assets.py` 确定性生成。
每 5 帧取 1 帧，以 333 毫秒间隔本地播放（约 3 帧/秒）；六种状态分别保留
12、10、8、15、7、11 帧。共享裁剪框去除透明边缘，按原图的 8×8 像素块还原到
77×67，再在 320×240 屏幕以整数 3 倍显示为 231×201。颜色仅转换为屏幕 RGB565，
不进行调色板量化。ESR1 通过共享相同扫描行和运行段压缩减少资源体积。

每种状态独立一包（4166–8191 字节），只上传下一种状态；设备确认展示新状态后
释放本插件的旧素材。传输失败保留原场景，插件不会删除其他插件的资源。
ESP32 的单包 8192／总缓存 16384 字节预算由板卡配置声明，插件不修改配对或权限。

插件要求设备支持 `capabilities.configuration.transient`，通过 `config.show` 仅切换内存场景，不写 NVS，也不逐帧发送请求。离线和上传失败会返回具体错误；下次任务变化、前台恢复或重新连接时可重试。设备缓存若已被其他场景占满，应先由原配置器释放资源；此插件不删除其他插件的素材。

设备重启后需要在插件设置中重新连接，上传素材并恢复展示；自动同步需要本机 Operit 和插件继续运行。重启恢复的是设备之前持久保存的配置。禁用或退出插件不等同于清除当前场景，点击「停止展示」清除本插件场景。配对记录与 Space 准入分别管理。

## 构建和验证

```powershell
.venv/Scripts/python.exe plugins/packages/external/edge_esp32_pet/generate_assets.py
node edge/boards/esp32/editor/node_modules/typescript/bin/tsc -p plugins/packages/external/edge_esp32_pet/tsconfig.json
node --test plugins/packages/external/edge_esp32_pet/tests/*.test.mjs
.venv/Scripts/python.exe -X utf8 -c "from pathlib import Path; from plugins.tools.sync_plugin_packages import _pack_toolpkg_folder; root=Path.cwd(); _pack_toolpkg_folder(root, root/'plugins/packages/external/edge_esp32_pet', root/'plugins/.out/edge_esp32_pet.toolpkg')"
```
