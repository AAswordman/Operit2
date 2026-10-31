# 大肥鱼桌宠

DeepSeek 社区二创「鲸鱼娘·大肥鱼」悬浮窗插件，非官方产品。

在工具箱打开「大肥鱼桌宠」，查看平台范围；需要授权的平台先点击申请权限，授权返回后刷新状态，再点击「保存并打开 / 应用设置」。

- 透明悬浮形象，呼吸、摇摆、跳跃、喂食与侧面踏步动画。
- 单击跳跃并说话，双击喂小鱼干，拖动移动，长按显示设置入口提示。
- 独立设置页：大小、动画、行走展示、整窗穿透、恢复默认、显示、隐藏和关闭。
- 设置通过 PluginConfig 持久化。修改大小、动画、行走展示和穿透后自动保存并更新已打开的桌宠，保留窗口、UI 会话和当前位置。大小滑块合并 80 毫秒内的连续编辑；拖动位置属于当前会话，下次打开使用保存的默认位置。
- 穿透后可从工具箱直接关闭穿透。插件不隐式申请权限，不启动后台服务，也不要求 API Key。
- macOS、Windows、Android 和合成器支持的 Linux/X11 使用系统悬浮窗；其他平台使用应用内悬浮层，并在设置页明确显示范围。应用内悬浮不会跨越其他应用。

动画为本地 WebP，宿主图片解码器负责播放，状态切换时才更新 DSL；没有逐帧 JavaScript 桥调用。关闭窗口会释放 UI 会话及 effect 定时器。拖动由宿主直接处理，以按下时的指针屏幕坐标和窗口位置为固定锚点，用实时屏幕坐标差定位，不经过逐帧 JS/Rust 桥。

## 素材与许可

三视图 PNG 来自 `1190fasheqi/dafeiyu-pet`，固定版本 `5b0e01856116bd2bae82df1f43c32faa5f056196`。上游 MIT 原文保存在 `resources/source/LICENSE`，原素材也保存在该目录。动画由 `tools/animate.py` 通过 Pillow 制作，安装 Pillow 后运行即可重建；不访问网络。

上游仓库地址记录在 `THIRD_PARTY_NOTICES.md`。

## 构建

从仓库根目录执行：

```sh
.ci-tools/typescript/node_modules/.bin/tsc -p plugins/packages/external/deepseek_dafeiyu/tsconfig.json
```

按仓库插件同步流程打包为 `deepseek_dafeiyu.toolpkg`。
