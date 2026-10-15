# 宿主语音处理插件

这个 ToolPkg 安装在手机或桌面等能执行 JS 的宿主上，为选定设备提供 STT 识别和 TTS 合成。它与厂商配置编辑器分开安装；配置包无需持续运行。硬件具备本地语音能力时可以直接使用本地实现。

传输复用 `Tools.device(nodeId).transport` 的 Watch / Push 双向通道。`operit.device.speech` 是示例固件和这个插件共同定义的应用协议，Core 的传输层不解释语音数据，也不注册专用音频服务。设备聊天发送、同步和屏幕点击由设备原生代码处理。

- `device_speech_processor:start({node_id})`：在当前宿主连接指定设备。
- `device_speech_processor:stop()`：关闭双向通道。
- `device_speech_processor:status()`：查询运行状态与最后错误。

插件使用当前宿主的 STT/TTS 配置。`Tools.Stt.transcribe` 通过已有识别服务处理 WAV；`Tools.Tts.synthesize` 通过已有合成服务返回音频字节，不启动宿主播放。宿主服务支持它们原有的本地模型、系统合成和远程供应商；可以用 `configId` 指定配置，省略时使用当前配置。

示例采集协议为 16 kHz、单声道、16 位有符号 PCM。识别器保留最多 60 秒数据后封装 WAV，这是插件的业务容量限制。合成结果按顺序分段，每次向设备发送最多 4096 字节。协议包包含请求 ID，取消后旧结果会被设备拒绝。通道拥塞、供应商失败和处理端断开都会明确返回错误。

ESP 模拟器使用浏览器作为麦克风和播放驱动：聊天页麦克风采集，识别文字回填草稿；编辑器的“朗读最新回复”触发合成并播放。真实硬件的麦克风采集和音频播放仍需要固件的本地驱动，本示例不提供 ESP 的 I²S 驱动。

构建和测试：

```sh
tsc -p plugins/packages/examples/device_speech_processor/tsconfig.json
node --test plugins/packages/examples/device_speech_processor/tests/processor.test.cjs
```

把 `manifest.json` 和 `dist/` 打包为 ZIP，扩展名改为 `.toolpkg`，通过现有插件导入入口安装。
