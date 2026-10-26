# TTS 供应商目录

TTS 设置界面从 Rust 的 `TtsCatalog` 读取供应商、请求模板、鉴权头、音频格式、响应管道和音色列表操作。新增目录项会出现在 Flutter 的“新建 TTS 供应商”选项中。合成请求和音色查询通过现有 Host HTTP 接口执行。

## 供应商配置

| 供应商 | 默认模型 | 默认音频 | 音色来源 | 配置说明 |
| --- | --- | --- | --- | --- |
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
- Speechify 的 `audio_data` 和 Inworld 的 `audioContent` 使用现有 `parse_json → pick → base64_decode` 管道。
- 通义千问使用 `parse_json → pick $.output.audio.url → http_get` 下载返回的音频；下载步骤不携带百炼的鉴权头。
- Cartesia、Inworld、Hume 和 ElevenLabs 的模板包含服务端速度参数。通义千问、Speechify 和 Rime 的预设不声明此参数。

## 维护与验证

供应商定义位于 `core/crates/foundation/model/src/TtsCatalog.rs`；预置音色位于 `core/crates/foundation/model/collects/TtsCatalog.rs`。新增协议参数应通过目录的数据字段表达。

`HttpVoiceProviderTests.rs` 覆盖生产模板的文本转义、模型与音色字段、速度类型、鉴权头、二进制与 Base64 音频、错误响应，以及通义千问下载 URL 的提取。`TtsVoiceListFetcherTests.rs` 覆盖在线音色的 JSON 路径、配置模型的保留、查询路由与鉴权。

以下官方资料用于核对接口：

- [通义千问 HTTP API](https://help.aliyun.com/zh/model-studio/qwen-tts-api)
- [Cartesia 音频接口](https://docs.cartesia.ai/api-reference/tts/bytes) 与 [音色列表](https://docs.cartesia.ai/api-reference/voices/list)
- [Speechify 合成接口](https://docs.sws.speechify.com/build/api-reference/v1/audio/speech) 与 [音色列表](https://docs.sws.speechify.com/build/api-reference/v1/voices/get)
- [Inworld 合成接口](https://docs.inworld.ai/api-reference/ttsAPI/texttospeech/synthesize-speech) 与 [音色列表](https://docs.inworld.ai/api-reference/voiceAPI/voiceservice/list-voices)
- [Hume 文件合成接口](https://dev.hume.ai/voice/reference/text-to-speech-tts/synthesize-file) 与 [音色列表](https://dev.hume.ai/voice/reference/voices/list)
- [Rime HTTP 接口](https://docs.rime.ai/api-reference/coda/http) 与 [音色说明](https://docs.rime.ai/api-reference/data/voice-details)
- [ElevenLabs 合成接口](https://elevenlabs.io/docs/api-reference/text-to-speech/convert) 与 [音色列表](https://elevenlabs.io/docs/api-reference/voices/search)
