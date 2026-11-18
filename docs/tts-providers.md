# TTS 供应商目录

TTS 设置界面从 Rust 的 `TtsCatalog` 读取供应商、请求模板、鉴权头、音频格式、响应管道和音色列表操作。新增目录项会出现在 Flutter 的“新建 TTS 供应商”选项中。合成请求和音色查询通过现有 Host HTTP 接口执行。

## 供应商配置

| 供应商 | 默认模型 | 默认音频 | 音色来源 | 配置说明 |
| --- | --- | --- | --- | --- |
| DeepSeek | 无需模型参数 | WAV | Stella、Tide | 使用 DeepSeek API Key；直接请求 `/v1/tools/tts`，自动识别语言，不支持服务端语速参数。 |
| 智谱 GLM | `glm-tts` | WAV | 彤彤、锤锤、小陈、Jam、Kazi、Douji、Luodo | 使用智谱开放平台模型 API Key；显式指定 WAV 和非流式响应。 |
| 阶跃星辰 | `stepaudio-3-tts` | MP3 | 磁性男声、邻家妹妹、温柔女声、元气少女 | 使用阶跃星辰 API Key；可在模型字段填写接口支持的 `stepaudio-2.5-tts`、`step-tts-2` 或 `step-tts-mini`。 |
| 通义千问 | `qwen3-tts-flash` | WAV | Cherry、Serena、Ethan、Chelsie | 使用百炼 API Key。默认连接北京地域；选择其他地域时，API Key 与 endpoint 必须对应。 |
| Cartesia | `sonic-3.6` | MP3 | Skylar 预置及在线音色列表 | 使用 Cartesia API Key，目录固定 `Cartesia-Version: 2026-08-14`。 |
| Speechify | `simba-3.0` | MP3 | 在线音色列表 | 使用 Speechify API Key，首次创建时填写控制台中的音色 ID。`simba-3.0` 支持英语、德语、西班牙语、法语、意大利语和葡萄牙语。 |
| Inworld | `inworld-tts-2` | MP3 | Ashley、Dennis、Alex 预置及在线音色列表 | API Key 字段填写控制台提供的 Base64 凭证，直接用于 `Authorization: Basic`。 |
| Hume Octave | `2` | MP3 | 在线共享音色列表 | 使用 Hume API Key。模型字段为 Octave 版本号；首次创建时填写 Voice Library 中的共享音色 ID。请求明确使用 `HUME_AI` 音色来源。 |
| Rime | `coda` | MP3 | Coda 的 Astra、Mist v3 的 Cove | 使用 Rime API Key。`Accept: audio/mpeg` 指定 MP3，避免返回无文件头的 PCM。预置音色为英语。 |

ElevenLabs 使用 `eleven_multilingual_v2`，提供预置音色和在线音色查询；速度设置写入 `voice_settings.speed`，在线查询每页 100 条。

在“设置 → TTS 供应商”中创建配置，选择供应商并填写凭证及音色 ID；保存后可用“添加音色”读取供应商目录。在线查询当前读取一页：Cartesia 100 条、Speechify 200 条、Inworld 2000 条、Hume 100 条、ElevenLabs 100 条。需要列表之外的音色时，可直接填写控制台中的 ID。

## 音频处理

- Cartesia、Hume、Rime、ElevenLabs 直接返回音频字节。
- DeepSeek、智谱 GLM、阶跃星辰也直接返回音频字节，复用同一条 Host HTTP 合成链路。
- Speechify 的 `audio_data` 和 Inworld 的 `audioContent` 使用现有 `parse_json → pick → base64_decode` 管道。
- 通义千问使用 `parse_json → pick $.output.audio.url → http_get` 下载返回的音频；下载步骤不携带百炼的鉴权头。
- Cartesia、Inworld、Hume 和 ElevenLabs 的模板包含服务端速度参数。通义千问、Speechify 和 Rime 的预设不声明此参数。
- 智谱 GLM 和阶跃星辰声明服务端速度参数；DeepSeek 不声明。DeepSeek HTTP 接口单次最多 10,000 个计费字符，中文字符按 2 个计算；智谱最多 1,024 字符，阶跃星辰最多 1,000 字符。聊天分段朗读继续使用现有分段服务。

## STT 供应商预设

STT 设置从 `core/crates/foundation/model/src/SttCatalog.rs` 读取预设。创建时自动填写 endpoint、上传字段、鉴权头和识别结果路径，用户保存配置后自行选择全局当前供应商。新增预设不会创建账户配置或更改当前选择。

| 供应商 | 预设模型 | 上传参数 | 官方限制与说明 |
| --- | --- | --- | --- |
| DeepSeek | 无需模型参数 | `file` | `/v1/tools/asr`，只接收音频文件，自动识别语言；支持 PCM16 WAV、Ogg Opus，最多 8 MiB、120 秒。 |
| 硅基流动 SiliconFlow | `FunAudioLLM/SenseVoiceSmall`、`TeleAI/TeleSpeechASR` | `file`、`model` | `/v1/audio/transcriptions`，不声明语言参数；最多 50 MB、1 小时。 |
| 智谱 GLM | `glm-asr-2512` | `file`、`model` | `/api/paas/v4/audio/transcriptions`，不声明语言参数；支持 WAV、MP3，最多 25 MB、30 秒。 |
| OpenAI 兼容 | `whisper-1`、`gpt-4o-transcribe`、`gpt-4o-mini-transcribe` | `file`、`model`、可选 `language` | 保留现有预设。 |
| Groq | `whisper-large-v3-turbo`、`whisper-large-v3` | `file`、`model`、可选 `language` | 保留现有预设。 |

以上新增 STT 接口均读取 JSON 的 `$.text`。模型字段名留空表示该接口不接收模型参数；保存与请求发送遵循这一声明。语言字段名留空时，调用方应不指定语言，服务自动检测；显式指定语言会报告不支持该参数。聊天录音当前使用 PCM16 WAV，并且不指定语言，符合这些预设的输入格式。

本次按 2026-11-18 官方接口资料补充预设。阶跃星辰的文件转写接口要求额外的 `response_format` 表单字段，StepAudio 3 ASR 使用 SSE；当前 STT 的通用 multipart 配置不表达这些协议，因此本次只加入其 TTS 预设。

## 维护与验证

供应商定义位于 `core/crates/foundation/model/src/TtsCatalog.rs`；预置音色位于 `core/crates/foundation/model/collects/TtsCatalog.rs`。新增协议参数应通过目录的数据字段表达。

`HttpVoiceProviderTests.rs` 覆盖生产模板的文本转义、模型与音色字段、速度类型、鉴权头、二进制与 Base64 音频、错误响应，以及通义千问下载 URL 的提取。`TtsVoiceListFetcherTests.rs` 覆盖在线音色的 JSON 路径、配置模型的保留、查询路由与鉴权。

以下官方资料用于核对接口：

- [DeepSeek TTS](https://api-docs.deepseek.com/guides/tts) 与 [STT](https://api-docs.deepseek.com/guides/asr)
- [智谱 GLM-TTS](https://docs.bigmodel.cn/api-reference/模型-api/文本转语音) 与 [GLM-ASR](https://docs.bigmodel.cn/api-reference/模型-api/语音转文本)
- [阶跃星辰 TTS](https://platform.stepfun.com/docs/zh/api-reference/audio/create-audio)、[StepAudio 3 TTS](https://platform.stepfun.com/docs/zh/guides/models/stepaudio-3-tts) 与 [文件转写](https://platform.stepfun.com/docs/zh/api-reference/audio/transcriptions)
- [硅基流动 STT](https://docs.siliconflow.cn/docs/api/audio-transcriptions-post)
- [通义千问 HTTP API](https://help.aliyun.com/zh/model-studio/qwen-tts-api)
- [Cartesia 音频接口](https://docs.cartesia.ai/api-reference/tts/bytes) 与 [音色列表](https://docs.cartesia.ai/api-reference/voices/list)
- [Speechify 合成接口](https://docs.sws.speechify.com/build/api-reference/v1/audio/speech) 与 [音色列表](https://docs.sws.speechify.com/build/api-reference/v1/voices/get)
- [Inworld 合成接口](https://docs.inworld.ai/api-reference/ttsAPI/texttospeech/synthesize-speech) 与 [音色列表](https://docs.inworld.ai/api-reference/voiceAPI/voiceservice/list-voices)
- [Hume 文件合成接口](https://dev.hume.ai/voice/reference/text-to-speech-tts/synthesize-file) 与 [音色列表](https://dev.hume.ai/voice/reference/voices/list)
- [Rime HTTP 接口](https://docs.rime.ai/api-reference/coda/http) 与 [音色说明](https://docs.rime.ai/api-reference/data/voice-details)
- [ElevenLabs 合成接口](https://elevenlabs.io/docs/api-reference/text-to-speech/convert) 与 [音色列表](https://elevenlabs.io/docs/api-reference/voices/search)
