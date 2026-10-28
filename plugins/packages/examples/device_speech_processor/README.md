# 宿主语音处理插件

这个 ToolPkg 安装在手机或桌面等能执行 JS 的宿主上，为选定设备提供 STT 识别和 TTS 合成。它与厂商配置编辑器分开安装；配置包无需持续运行。硬件具备本地语音能力时可以直接使用本地实现。

传输复用 `Tools.device(nodeId).transport` 的 Watch / Push 双向通道。`operit.device.speech` 是示例固件和这个插件共同定义的应用协议，Core 的传输层不解释语音数据，也不注册专用音频服务。设备聊天发送、同步和屏幕点击由设备原生代码处理。

- `device_speech_processor:start({node_id})`：在当前宿主连接指定设备。
- `device_speech_processor:stop()`：关闭双向通道。
- `device_speech_processor:status()`：查询运行状态与最后错误。

插件使用当前宿主的 STT/TTS 配置。`Tools.Stt.transcribe` 通过已有识别服务处理 WAV；`Tools.Tts.synthesize` 通过已有合成服务返回音频字节，不启动宿主播放。宿主服务支持它们原有的本地模型、系统合成和远程供应商；可以用 `configId` 指定配置，省略时使用当前配置。

示例采集协议为 16 kHz、单声道、16 位有符号 PCM。插件在 Core 最多收集 300 秒（9,600,000 字节）PCM，再封装 WAV 调用现有识别服务；完整录音不驻留 ESP32。设备每块最多上传 4096 字节，四个队列槽满后等待接收，不因短暂拥塞主动断开。超过上限明确失败，不能把前缀当作完整识别结果。双向通道超时为 10 分钟；实际识别服务仍受供应商文件大小、时长和超时限制。

协议 v2 把识别文字拆为最多 2048 UTF-8 字节的 `stt.text` 包，最后用 `stt.result` 完成；设备只在完成后发布草稿，完整文字上限为 16384 字节。v1 设备仍收到原有的单包结果，较长结果会明确要求升级固件。设备请求朗读时单次文字上限为 4096 UTF-8 字节；Core 将其按 1024 字节切分调用已有 TTS 服务，顺序发送各服务返回的音频分段，每个二进制包最多 4096 字节，播放队列满后等待消费。分段可能影响句间停顿，尚未验证外部供应商的实际长语音质量。包均包含请求 ID，取消后的旧结果被设备拒绝。供应商失败、输入超限和处理端断开会明确返回错误。

ESP 模拟器使用浏览器作为麦克风和播放驱动：聊天页麦克风采集，识别文字回填草稿；编辑器的“朗读当前显示的回复”只合成当前显示窗口中的回复片段，不会读取 Core 历史全文。真实硬件的麦克风采集和音频播放仍需要固件的本地驱动，本示例不提供 ESP 的 I²S 驱动。

构建和测试：

```sh
tsc -p plugins/packages/examples/device_speech_processor/tsconfig.json
node --test plugins/packages/examples/device_speech_processor/tests/processor.test.cjs
node --test --experimental-strip-types edge/boards/esp32/editor/tests/speech-processor.test.mjs
```

把 `manifest.json` 和 `dist/` 打包为 ZIP，扩展名改为 `.toolpkg`，通过现有插件导入入口安装。
