pub const TTS_CATALOG_VOICE_ROWS: &str = r#"
SYSTEM_TTS|||系统默认音色|使用系统默认语言与默认声音
OPENAI_COMPATIBLE||alloy|Alloy|中性均衡音色
OPENAI_COMPATIBLE||ash|Ash|低沉自然音色
OPENAI_COMPATIBLE||ballad|Ballad|叙事感音色
OPENAI_COMPATIBLE||coral|Coral|明亮自然音色
OPENAI_COMPATIBLE||echo|Echo|清晰男声音色
OPENAI_COMPATIBLE||fable|Fable|故事叙述音色
OPENAI_COMPATIBLE||nova|Nova|清晰女声音色
OPENAI_COMPATIBLE||onyx|Onyx|深沉男声音色
OPENAI_COMPATIBLE||sage|Sage|稳重自然音色
OPENAI_COMPATIBLE||shimmer|Shimmer|轻快女声音色
OPENAI_COMPATIBLE||verse|Verse|表达型音色
MINIMAX_TTS|speech-2.8-hd|male-qn-qingse|青涩男声|MiniMax 清澈男声音色
MINIMAX_TTS|speech-2.8-hd|male-qn-jingying|精英男声|MiniMax 稳重男声音色
MINIMAX_TTS|speech-2.8-hd|female-shaonv|少女音|MiniMax 中文女声音色
MINIMAX_TTS|speech-2.8-hd|female-yujie|御姐音|MiniMax 成熟女声音色
MINIMAX_TTS|speech-2.8-hd|presenter_male|男主持|MiniMax 主持男声音色
MINIMAX_TTS|speech-2.8-hd|presenter_female|女主持|MiniMax 主持女声音色
MIMO_TTS|mimo-v2.5-tts|mimo_default|MiMo Default|MiMo 默认音色
MIMO_TTS|mimo-v2.5-tts|冰糖|冰糖|MiMo 中文女声
MIMO_TTS|mimo-v2.5-tts|茉莉|茉莉|MiMo 中文女声
MIMO_TTS|mimo-v2.5-tts|苏打|苏打|MiMo 中文男声
MIMO_TTS|mimo-v2.5-tts|白桦|白桦|MiMo 中文男声
MIMO_TTS|mimo-v2.5-tts|Mia|Mia|MiMo 英文女声
MIMO_TTS|mimo-v2.5-tts|Chloe|Chloe|MiMo 英文女声
MIMO_TTS|mimo-v2.5-tts|Milo|Milo|MiMo 英文男声
MIMO_TTS|mimo-v2.5-tts|Dean|Dean|MiMo 英文男声
SILICONFLOW_TTS|FunAudioLLM/CosyVoice2-0.5B|FunAudioLLM/CosyVoice2-0.5B:alex|Alex|SiliconFlow CosyVoice2 男声
SILICONFLOW_TTS|FunAudioLLM/CosyVoice2-0.5B|FunAudioLLM/CosyVoice2-0.5B:anna|Anna|SiliconFlow CosyVoice2 女声
SILICONFLOW_TTS|FunAudioLLM/CosyVoice2-0.5B|FunAudioLLM/CosyVoice2-0.5B:bella|Bella|SiliconFlow CosyVoice2 女声
SILICONFLOW_TTS|FunAudioLLM/CosyVoice2-0.5B|FunAudioLLM/CosyVoice2-0.5B:benjamin|Benjamin|SiliconFlow CosyVoice2 男声
SILICONFLOW_TTS|FunAudioLLM/CosyVoice2-0.5B|FunAudioLLM/CosyVoice2-0.5B:charles|Charles|SiliconFlow CosyVoice2 男声
SILICONFLOW_TTS|FunAudioLLM/CosyVoice2-0.5B|FunAudioLLM/CosyVoice2-0.5B:claire|Claire|SiliconFlow CosyVoice2 女声
SILICONFLOW_TTS|FunAudioLLM/CosyVoice2-0.5B|FunAudioLLM/CosyVoice2-0.5B:david|David|SiliconFlow CosyVoice2 男声
SILICONFLOW_TTS|FunAudioLLM/CosyVoice2-0.5B|FunAudioLLM/CosyVoice2-0.5B:diana|Diana|SiliconFlow CosyVoice2 女声
ELEVENLABS_TTS|eleven_multilingual_v2|21m00Tcm4TlvDq8ikWAM|Rachel|ElevenLabs 英文女声
ELEVENLABS_TTS|eleven_multilingual_v2|EXAVITQu4vr4xnSDxMaL|Bella|ElevenLabs 英文女声
ELEVENLABS_TTS|eleven_multilingual_v2|ErXwobaYiN019PkySvjV|Antoni|ElevenLabs 英文男声
ELEVENLABS_TTS|eleven_multilingual_v2|MF3mGyEYCl7XYWbV9V6O|Elli|ElevenLabs 英文女声
ELEVENLABS_TTS|eleven_multilingual_v2|TxGEqnHWrfWFTfGW9XjX|Josh|ElevenLabs 英文男声
ELEVENLABS_TTS|eleven_multilingual_v2|VR6AewLTigWG4xSOukaG|Arnold|ElevenLabs 英文男声
ELEVENLABS_TTS|eleven_multilingual_v2|pNInz6obpgDQGcFmaJgB|Adam|ElevenLabs 英文男声
ELEVENLABS_TTS|eleven_multilingual_v2|yoZ06aMxZJJ28mfd3POQ|Sam|ElevenLabs 英文男声
DOUBAO_TTS||BV700_V2_streaming|豆包默认音色|火山引擎默认中文音色
DEEPGRAM_TTS|aura-2-thalia-en||Thalia|Deepgram Aura 2 英文女声
DEEPGRAM_TTS|aura-2-andromeda-en||Andromeda|Deepgram Aura 2 英文女声
DEEPGRAM_TTS|aura-2-apollo-en||Apollo|Deepgram Aura 2 英文男声
DEEPGRAM_TTS|aura-2-arcas-en||Arcas|Deepgram Aura 2 英文男声
DEEPGRAM_TTS|aura-2-asteria-en||Asteria|Deepgram Aura 2 英文女声
DEEPGRAM_TTS|aura-2-orpheus-en||Orpheus|Deepgram Aura 2 英文男声
GROQ_TTS|playai-tts|Arista-PlayAI|Arista|Groq PlayAI 女声
GROQ_TTS|playai-tts|Basil-PlayAI|Basil|Groq PlayAI 男声
GROQ_TTS|playai-tts|Briggs-PlayAI|Briggs|Groq PlayAI 男声
GROQ_TTS|playai-tts|Calum-PlayAI|Calum|Groq PlayAI 男声
GROQ_TTS|playai-tts|Celeste-PlayAI|Celeste|Groq PlayAI 女声
GROQ_TTS|playai-tts|Cheyenne-PlayAI|Cheyenne|Groq PlayAI 女声
GROQ_TTS|playai-tts|Chip-PlayAI|Chip|Groq PlayAI 男声
GROQ_TTS|playai-tts|Cillian-PlayAI|Cillian|Groq PlayAI 男声
GROQ_TTS|playai-tts|Deedee-PlayAI|Deedee|Groq PlayAI 女声
GROQ_TTS|playai-tts|Fritz-PlayAI|Fritz|Groq PlayAI 男声
GROQ_TTS|playai-tts|Gail-PlayAI|Gail|Groq PlayAI 女声
GROQ_TTS|playai-tts|Indigo-PlayAI|Indigo|Groq PlayAI 中性音色
GROQ_TTS|playai-tts|Mamaw-PlayAI|Mamaw|Groq PlayAI 女声
GROQ_TTS|playai-tts|Mason-PlayAI|Mason|Groq PlayAI 男声
GROQ_TTS|playai-tts|Mikail-PlayAI|Mikail|Groq PlayAI 男声
GROQ_TTS|playai-tts|Mitch-PlayAI|Mitch|Groq PlayAI 男声
GROQ_TTS|playai-tts|Quinn-PlayAI|Quinn|Groq PlayAI 中性音色
GROQ_TTS|playai-tts|Thunder-PlayAI|Thunder|Groq PlayAI 男声
AZURE_TTS|zh-CN|zh-CN-XiaoxiaoNeural|晓晓|Azure 中文女声
AZURE_TTS|zh-CN|zh-CN-XiaoyiNeural|晓伊|Azure 中文女声
AZURE_TTS|zh-CN|zh-CN-YunjianNeural|云健|Azure 中文男声
AZURE_TTS|zh-CN|zh-CN-YunxiNeural|云希|Azure 中文男声
AZURE_TTS|zh-CN|zh-CN-YunxiaNeural|云夏|Azure 中文男声
AZURE_TTS|zh-CN|zh-CN-YunyangNeural|云扬|Azure 中文男声
AZURE_TTS|en-US|en-US-JennyNeural|Jenny|Azure 英文女声
AZURE_TTS|en-US|en-US-GuyNeural|Guy|Azure 英文男声
AZURE_TTS|en-US|en-US-AriaNeural|Aria|Azure 英文女声
AZURE_TTS|en-US|en-US-DavisNeural|Davis|Azure 英文男声
GOOGLE_CLOUD_TTS|zh-CN|cmn-CN-Wavenet-A|中文普通话女声 A|Google Cloud 中文普通话女声
GOOGLE_CLOUD_TTS|zh-CN|cmn-CN-Wavenet-B|中文普通话男声 B|Google Cloud 中文普通话男声
GOOGLE_CLOUD_TTS|zh-CN|cmn-CN-Wavenet-C|中文普通话男声 C|Google Cloud 中文普通话男声
GOOGLE_CLOUD_TTS|zh-CN|cmn-CN-Wavenet-D|中文普通话女声 D|Google Cloud 中文普通话女声
GOOGLE_CLOUD_TTS|en-US|en-US-Neural2-C|English US C|Google Cloud 美式英语女声
GOOGLE_CLOUD_TTS|en-US|en-US-Neural2-D|English US D|Google Cloud 美式英语男声
GOOGLE_CLOUD_TTS|en-US|en-US-Neural2-F|English US F|Google Cloud 美式英语女声
GOOGLE_CLOUD_TTS|en-US|en-US-Neural2-J|English US J|Google Cloud 美式英语男声
GEMINI_TTS||Zephyr|Zephyr|Gemini 明亮音色
GEMINI_TTS||Puck|Puck|Gemini 活泼音色
GEMINI_TTS||Charon|Charon|Gemini 信息型音色
GEMINI_TTS||Kore|Kore|Gemini 坚定音色
GEMINI_TTS||Fenrir|Fenrir|Gemini 兴奋音色
GEMINI_TTS||Leda|Leda|Gemini 年轻音色
GEMINI_TTS||Orus|Orus|Gemini 稳重音色
GEMINI_TTS||Aoede|Aoede|Gemini 轻快音色
GEMINI_TTS||Callirrhoe|Callirrhoe|Gemini 从容音色
GEMINI_TTS||Autonoe|Autonoe|Gemini 明亮自然音色
GEMINI_TTS||Enceladus|Enceladus|Gemini 气息感音色
GEMINI_TTS||Iapetus|Iapetus|Gemini 清晰音色
GEMINI_TTS||Umbriel|Umbriel|Gemini 随和音色
GEMINI_TTS||Algieba|Algieba|Gemini 平滑音色
GEMINI_TTS||Despina|Despina|Gemini 柔和音色
GEMINI_TTS||Erinome|Erinome|Gemini 清澈音色
GEMINI_TTS||Algenib|Algenib|Gemini 沙哑音色
GEMINI_TTS||Rasalgethi|Rasalgethi|Gemini 信息播报音色
GEMINI_TTS||Laomedeia|Laomedeia|Gemini 活泼自然音色
GEMINI_TTS||Achernar|Achernar|Gemini 柔和明亮音色
GEMINI_TTS||Alnilam|Alnilam|Gemini 坚定平稳音色
GEMINI_TTS||Schedar|Schedar|Gemini 均衡音色
GEMINI_TTS||Gacrux|Gacrux|Gemini 成熟音色
GEMINI_TTS||Pulcherrima|Pulcherrima|Gemini 亲切音色
GEMINI_TTS||Achird|Achird|Gemini 友好音色
GEMINI_TTS||Zubenelgenubi|Zubenelgenubi|Gemini 随性音色
GEMINI_TTS||Vindemiatrix|Vindemiatrix|Gemini 温和音色
GEMINI_TTS||Sadachbia|Sadachbia|Gemini 活泼轻快音色
GEMINI_TTS||Sadaltager|Sadaltager|Gemini 专业音色
GEMINI_TTS||Sulafat|Sulafat|Gemini 温暖音色
KOKORO_TTS||af_heart|AF Heart|Kokoro 美式英语女声
KOKORO_TTS||af_alloy|AF Alloy|Kokoro 美式英语女声
KOKORO_TTS||af_aoede|AF Aoede|Kokoro 美式英语女声
KOKORO_TTS||af_bella|AF Bella|Kokoro 美式英语女声
KOKORO_TTS||af_jessica|AF Jessica|Kokoro 美式英语女声
KOKORO_TTS||af_kore|AF Kore|Kokoro 美式英语女声
KOKORO_TTS||af_nicole|AF Nicole|Kokoro 美式英语女声
KOKORO_TTS||af_nova|AF Nova|Kokoro 美式英语女声
KOKORO_TTS||af_river|AF River|Kokoro 美式英语女声
KOKORO_TTS||af_sarah|AF Sarah|Kokoro 美式英语女声
KOKORO_TTS||af_sky|AF Sky|Kokoro 美式英语女声
KOKORO_TTS||am_adam|AM Adam|Kokoro 美式英语男声
KOKORO_TTS||am_echo|AM Echo|Kokoro 美式英语男声
KOKORO_TTS||am_eric|AM Eric|Kokoro 美式英语男声
KOKORO_TTS||am_fenrir|AM Fenrir|Kokoro 美式英语男声
KOKORO_TTS||am_liam|AM Liam|Kokoro 美式英语男声
KOKORO_TTS||am_michael|AM Michael|Kokoro 美式英语男声
KOKORO_TTS||am_onyx|AM Onyx|Kokoro 美式英语男声
KOKORO_TTS||am_puck|AM Puck|Kokoro 美式英语男声
KOKORO_TTS||bf_alice|BF Alice|Kokoro 英式英语女声
KOKORO_TTS||bf_emma|BF Emma|Kokoro 英式英语女声
KOKORO_TTS||bf_isabella|BF Isabella|Kokoro 英式英语女声
KOKORO_TTS||bf_lily|BF Lily|Kokoro 英式英语女声
KOKORO_TTS||bm_daniel|BM Daniel|Kokoro 英式英语男声
KOKORO_TTS||bm_fable|BM Fable|Kokoro 英式英语男声
KOKORO_TTS||bm_george|BM George|Kokoro 英式英语男声
KOKORO_TTS||bm_lewis|BM Lewis|Kokoro 英式英语男声
"#;
