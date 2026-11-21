# 语音通话

启用 `com.operit.voice_call` 后，从工具箱的「语音通话」打开悬浮窗。Android 如需系统悬浮窗授权，按工具箱提示授权后重新打开。在悬浮窗点「开始通话」后，系统会按需请求麦克风权限。语音识别和朗读使用 Operit 当前选择的 STT 和 TTS 配置。

每个通话窗创建一个独立聊天，记录转录文本和 AI 回复。说完后约 0.9 秒静音会结束这一轮录音，也可以点「说完了」。插件等朗读结束后继续收音。单段录音上限为 20 秒，整段没有检测到声音时暂停通话，可以点「继续通话」恢复。「挂断」停止后续回合、取消正在等待的 AI 回复和本次朗读，并关闭窗口。

系统支持全局悬浮时，通话窗使用系统悬浮窗。其他支持应用内悬浮的平台在 Operit 内显示。实际能否在应用切到后台时继续录音由系统决定。

通话通过公开客户端的 `characterCards.defaultAvatar()` 获取 Operit 默认头像。启用通话插件前，需要安装并启用角色卡插件 0.2.8 或更新版本。收音时水波从头像向外扩散，音量越大，波纹越亮、扩散越远。识别、等待回复和朗读时使用柔和的波纹提示。此版本需要宿主提供 `recordingLevel` 接口。

窗口由 `ToolPkg.ui.floatingWindows` 承载。录音接口见[悬浮窗文档](../../../docs/FLOATING_WINDOWS.md)。插件调用 `Tools.Stt.transcribe` 和 `Tools.Chat.sendMessage`，用 `Tools.Tts.speak` 的 `waitForCompletion` 等待全部音频分段播完。`playbackId` 仅在当前插件内有效，挂断时用 `Tools.Tts.cancel` 取消同一条朗读。
