# 大肥鱼桌宠

DeepSeek 社区二创「鲸鱼娘·大肥鱼」悬浮窗插件，非官方产品。

在工具箱打开「大肥鱼桌宠」，查看平台范围；需要授权的平台先点击申请权限，授权返回后刷新状态，再点击「保存并打开 / 应用设置」。

- 五组实际动作素材：待机眨眼转头、元气挥手、端碗吃饭、原地踏步、对屏碎碎念。每组约 10 秒、源素材 241 帧；角色的眼睛、口型、表情和肢体随动作变化。
- 单击挥手，双击喂饭，拖动移动，长按播放碎碎念。气泡为预设台词；没有连接模型、语音或聊天会话。
- 设置页可分别预览、重播五组动作。播放动画开关和行走展示开关也立即影响预览。
- 独立设置页：大小、动画、行走展示、整窗穿透、恢复默认、显示、隐藏和关闭。
- 设置通过 PluginConfig 持久化。修改大小、动画、行走展示和穿透后自动保存并更新已打开的桌宠，保留窗口、UI 会话和当前位置。大小滑块合并 80 毫秒内的连续编辑；拖动位置属于当前会话，下次打开使用保存的默认位置。
- 穿透后可从工具箱直接关闭穿透。插件不隐式申请权限，不启动后台服务，也不要求 API Key。
- macOS、Windows、Android 和合成器支持的 Linux/X11 使用系统悬浮窗；其他平台使用应用内悬浮层，并在设置页明确显示范围。应用内悬浮不会跨越其他应用。

透明动画为本地 WebP，宿主图片解码器负责播放，状态切换时才更新 DSL；没有逐帧 JavaScript 桥调用。待机、踏步循环播放，交互动作按素材实际时长播放一次，然后回到待机。Image 的 `animationKey` 在交互触发时更新，为动画资源创建新的解码流，重复点击也能从头播放；无关的重绘保留播放进度。关闭窗口会释放 UI 会话及 effect 定时器。拖动由宿主直接处理，以按下时的指针屏幕坐标和窗口位置为固定锚点，用实时屏幕坐标差定位，不经过逐帧 JS/Rust 桥。

## 素材与许可

动作素材来自 **https://github.com/PC2005-cloud/dsh-pet**，固定版本 `38ae8465cd69ba15e8c8a29e7208997a0baa3451`。上游允许素材开源使用、**禁止商用**；形象遵循 **CC BY-NC-SA 4.0**。这些素材不属于上游代码的 MIT 许可范围。作者、原始文件、许可原文及修改说明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。对外分发请保留原项目地址、署名和许可；商业发行需要另行取得素材和形象授权。

原始透明 WebM 和 SHA-256 固定在 `resources/source/`，转换脚本只做统一裁切、缩放、近透明底清理和编码，保留 24 fps 的实际动作。重建无需联网：

```sh
python3 -m pip install Pillow imageio-ffmpeg
python3 plugins/packages/external/deepseek_dafeiyu/tools/convert_animations.py
```

也可安装带 `libvpx-vp9` 的 ffmpeg，通过 `--ffmpeg /path/to/ffmpeg` 指定；无需安装 imageio-ffmpeg。脚本会验证源文件摘要，输出 `resources/animation-info.json` 中的尺寸、帧数和时长。更换不同时长的源视频时，需要同步 `src/main.ts` 的 `ACTION_DURATION_MS`，相关测试会检查实际编码时长。

## 构建

从仓库根目录执行：

```sh
.ci-tools/typescript/node_modules/.bin/tsc -p plugins/packages/external/deepseek_dafeiyu/tsconfig.json
node --test tools/tests/deepseek_dafeiyu.test.mjs
```

按仓库插件同步流程打包为 `deepseek_dafeiyu.toolpkg`。
