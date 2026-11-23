# 手表桌宠

管理已加入 Operit 设备空间的 Android Edge 桌宠。手表和驱动预览统一使用「任务状态猫」的六套原版像素动画，支持三种氛围配色、点按触感和首页亮屏开关。

## 使用

1. 通过 Operit 的「设备空间」完成手表配对和加入，并保持两台设备直连。[手表端接入](../../../../edge/boards/android_phone/README.md)
2. 导入 `edge_watch_pet.toolpkg` 并启用「手表桌宠」，从侧栏打开。页面自动识别设备空间里当前在线、可直连的手表桌宠节点：只有一块时自动连接，有多块时列出名称、型号和设备标识，点击选择即可，无需输入名称或节点 ID。
3. 调整配色和开关后保存。插件记住上次选中的设备，点「切换」可选择另一块表。
4. 启用插件后，真实任务状态自动同步到所选手表，关闭设置页后仍继续同步；无需另外启用悬浮窗版任务状态猫。升级旧版时，首次打开设置页会迁移已保存的手表选择。

发现流程读取 `list_core_nodes` 的真实空间成员与可达状态，按精确的 `android-edge` 平台标识筛选，排除当前设备、离线设备及多跳节点，再查询 `edge.watchPet.getConfig` 验证实际桌宠能力。最多同时检查四台设备；不根据名称推断，也不要求 `canExecute`，因为手表本来就是不承载 Core 的 Edge 节点。在线且验证通过的上次选择保持选中；没有已选设备且发现多块时等待用户选择。连接时重新检查成员和在线状态，使用完整身份调用。没有可用设备时显示接入提示和刷新按钮；能力检查失败的设备显示具体原因。

## 动作与互动

| 状态 | 行为 |
| --- | --- |
| 工作 | 输出、工具执行等实际工作时敲键盘 |
| 思考 | 模型连接、处理时思考 |
| 待批准 | 有未处理审批时抬爪提醒 |
| 完成 | 所有活跃任务完成后庆祝 3 秒，再待机 |
| 空闲 | 没有任务时放松待命；取消任务直接待机 |
| 异常 | 任务错误时显示困惑动作 |

待批准优先于异常和运行任务；多任务中仍有任务运行时不会提前庆祝。插件订阅结构化 runtime 事件与完整审批快照，不解析聊天文本，只向手表发送动作名称。审批在 Operit 原有界面处理。

轻点手表上的猫会短暂抬爪回应，再恢复当前任务动作；长按进入设备空间。驱动提供六种本地动作预览，预览按钮不修改手表的真实任务状态。配色仅改变背景氛围光，保留原始蓝猫颜色。自动同步依赖 Operit 进程和节点连接；发送失败会明确报错，下次任务事件、返回 Operit 前台或重新选择手表时重新同步，不循环轮询或自动重试。

插件通过现有 Edge 桥接先解析设备身份，再用 `Tools.device(nodeId).Edge.execute` 访问 `edge.watchPet` 和 `edge.robotFace.setExpression`。手表仅接受同空间当前成员的直连设置请求。`getConfig` 返回当前值，`setConfig` 接收完整的版本 1 配置，包含 `accent`、`haptics` 和 `keepAwake`；原有配置版本不变。动作同步由主 runtime 的单个串行状态机持有，设备选择保存到 PluginConfig；历史环境配置只作一次迁移来源。WebView 所用 HTML 与六个透明 WebP 随包分发，通过本地资源拦截加载。

## 素材同步与检查

素材唯一源为 `task_status_pet/resources/*.webp`。更新原动画后运行：

```powershell
.venv/Scripts/python.exe edge/boards/android_phone/sync_pet_assets.py
```

脚本将六个原 WebP 原样复制到驱动，并为 API 21 手表生成 90×88 像素帧图集和原始帧时长。每帧都会验证其与 720×704 原图的像素网格完全一致；不会重绘角色或丢帧。手表不依赖新版 Android 动画 WebP API，内存中只保留当前动作的图集。

```sh
node .ci-tools/typescript/node_modules/typescript/bin/tsc -p plugins/packages/external/edge_watch_pet/tsconfig.json --noEmit
node --test plugins/packages/external/edge_watch_pet/tests/*.test.mjs
.venv/Scripts/python.exe edge/boards/android_phone/sync_pet_assets.py --check
```

检查覆盖设备筛选与能力查询、单设备自动连接、多设备选择、空列表与刷新、断开后的过期选择、页面导出重载、初始化任务状态、工作/思考切换、多任务、取消、审批/异常优先级、完成定时器和退出后的迟到事件。真实手表上的圆屏、触感、后台连接与耗电仍需设备验收。

## 单包打包

在仓库根目录运行以下命令。

```powershell
tsc -p plugins/packages/external/edge_watch_pet/tsconfig.json
.venv/Scripts/python.exe -X utf8 -c "from pathlib import Path; from plugins.tools.sync_plugin_packages import _pack_toolpkg_folder; root=Path.cwd(); _pack_toolpkg_folder(root, root/'plugins/packages/external/edge_watch_pet', root/'plugins/.out/edge_watch_pet.toolpkg')"
```
