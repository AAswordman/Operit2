use crate::LocalModelManifest::{
    LocalEngineKind, LocalEngineRequirement, LocalModelArchive, LocalModelArchiveFormat,
    LocalModelDriver, LocalModelFile, LocalModelInstallSource, LocalModelKind, LocalModelManifest,
    LocalModelSource, LocalModelSourceKind,
};

const OLD_ASSISTANCE_STT_ID: &str = "sherpa-ncnn-streaming-zipformer-bilingual-zh-en-2023-02-13";
const OLD_ASSISTANCE_STT_REVISION: &str = "05945efc40afe4b572542f01104ca5c413a9f6e1";
const OLD_ASSISTANCE_STT_SOURCE_ID: &str = "huggingface-main";
const SHERPA_ONNX_ENGINE_ID: &str = "sherpa-onnx";
const SHERPA_ONNX_ENGINE_VERSION: &str = "1.13.2";
const SHERPA_ONNX_STT_ID: &str = "sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20";
const SHERPA_ONNX_STT_REVISION: &str = "98590b7ed6443e77b714204da2757d75e1a642f4";
const SHERPA_ONNX_TTS_ID: &str = "vits-zh-aishell3-int8";
const SHERPA_ONNX_TTS_REVISION: &str = "e3e808eaab2385b812286c6707323362251bba65";
const SHERPA_ONNX_TTS_ZH_LL_ID: &str = "sherpa-onnx-vits-zh-ll";
const SHERPA_ONNX_TTS_ZH_LL_REVISION: &str = "7ddf37bcacf05ed56afee360d96835be633a5265";
const SHERPA_ONNX_MATCHA_BAKER_ID: &str = "matcha-icefall-zh-baker";
const SHERPA_ONNX_MATCHA_BAKER_REVISION: &str = "75e64a57a80fb370abb18a4e4d86bb090ed46e93";
const SHERPA_ONNX_KITTEN_NANO_ID: &str = "kitten-nano-en-v0_8-int8";
const SHERPA_ONNX_KITTEN_NANO_VERSION: &str = "tts-models";
const SHERPA_ONNX_WEB_ASR_ID: &str = "sherpa-onnx-web-paraformer-small-zh-en";
const SHERPA_ONNX_WEB_ASR_VERSION: &str = "1.13.2";
const SHERPA_ONNX_WEB_TTS_ID: &str = "sherpa-onnx-web-vits-piper-en-us-libritts-r-medium";
const SHERPA_ONNX_WEB_TTS_VERSION: &str = "1.13.2";

pub struct LocalModelCatalog;

impl LocalModelCatalog {
    /// Returns all local model manifests bundled as built-in catalog entries.
    pub fn manifests() -> Vec<LocalModelManifest> {
        vec![
            Self::sherpaOnnxStreamingStt(),
            Self::sherpaOnnxVitsTts(),
            Self::sherpaOnnxVitsZhLlTts(),
            Self::sherpaOnnxMatchaBakerTts(),
            Self::sherpaOnnxKittenNanoTts(),
            Self::sherpaOnnxWebAsr(),
            Self::sherpaOnnxWebVitsTts(),
        ]
    }

    /// Returns the cross-platform bilingual Sherpa ONNX streaming STT manifest.
    pub fn sherpaOnnxStreamingStt() -> LocalModelManifest {
        let sourceId = "huggingface-sherpa-onnx-stt";
        LocalModelManifest {
            id: SHERPA_ONNX_STT_ID.to_string(),
            version: SHERPA_ONNX_STT_REVISION.to_string(),
            displayName: "Sherpa ONNX Zipformer zh/en STT".to_string(),
            description: "Streaming bilingual Chinese and English speech recognition.".to_string(),
            kind: LocalModelKind::SpeechToText,
            engine: LocalEngineKind::SherpaOnnx,
            license: "apache-2.0".to_string(),
            homepage: format!("https://huggingface.co/csukuangfj/{SHERPA_ONNX_STT_ID}"),
            languages: vec!["zh".to_string(), "en".to_string()],
            tags: vec![
                "streaming".to_string(),
                "zipformer".to_string(),
                "sherpa-onnx".to_string(),
            ],
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxStreamingTransducer {
                encoder: "encoder-epoch-99-avg-1.int8.onnx".to_string(),
                decoder: "decoder-epoch-99-avg-1.onnx".to_string(),
                joiner: "joiner-epoch-99-avg-1.int8.onnx".to_string(),
                tokens: "tokens.txt".to_string(),
                modelType: "zipformer".to_string(),
            }),
            sources: vec![huggingFaceSource(
                sourceId,
                &format!("csukuangfj/{SHERPA_ONNX_STT_ID}"),
                SHERPA_ONNX_STT_REVISION,
            )],
            installSource: LocalModelInstallSource::Files,
            files: vec![
                modelFile(
                    sourceId,
                    "encoder-epoch-99-avg-1.int8.onnx",
                    "8fa764187a261844f859d7143ebaa563af5d10adfece4c18a8f414c88cba2a9b",
                    181_895_032,
                ),
                modelFile(
                    sourceId,
                    "decoder-epoch-99-avg-1.onnx",
                    "2e3b5ec371f8899ee6acd829fd753ba45772df57a91bdf37cde3136354e7db7d",
                    13_876_452,
                ),
                modelFile(
                    sourceId,
                    "joiner-epoch-99-avg-1.int8.onnx",
                    "1ed689c5ed19dbaa725d9d191bb4822b5f4855a39e1ffd28cbc1f340d25b2ee0",
                    3_228_404,
                ),
                modelFile(
                    sourceId,
                    "tokens.txt",
                    "a8e0e4ec53810e433789b54a5c0134a7eaa2ffca595a6334d54c00da858841d3",
                    56_317,
                ),
            ],
        }
    }

    /// Returns the multi-speaker Chinese Sherpa ONNX VITS manifest.
    pub fn sherpaOnnxVitsTts() -> LocalModelManifest {
        let sourceId = "huggingface-sherpa-onnx-tts";
        LocalModelManifest {
            id: SHERPA_ONNX_TTS_ID.to_string(),
            version: SHERPA_ONNX_TTS_REVISION.to_string(),
            displayName: "Sherpa ONNX VITS AISHELL-3 int8".to_string(),
            description: "Local Chinese multi-speaker speech synthesis.".to_string(),
            kind: LocalModelKind::TextToSpeech,
            engine: LocalEngineKind::SherpaOnnx,
            license: "apache-2.0".to_string(),
            homepage: "https://huggingface.co/csukuangfj/vits-zh-aishell3".to_string(),
            languages: vec!["zh".to_string()],
            tags: vec![
                "vits".to_string(),
                "multi-speaker".to_string(),
                "sherpa-onnx".to_string(),
            ],
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxVits {
                model: "vits-aishell3.int8.onnx".to_string(),
                lexicon: "lexicon.txt".to_string(),
                tokens: "tokens.txt".to_string(),
                ruleFsts: Vec::new(),
                ruleFars: Vec::new(),
                speakerCount: 175,
            }),
            sources: vec![huggingFaceSource(
                sourceId,
                "csukuangfj/vits-zh-aishell3",
                SHERPA_ONNX_TTS_REVISION,
            )],
            installSource: LocalModelInstallSource::Files,
            files: vec![
                modelFile(
                    sourceId,
                    "vits-aishell3.int8.onnx",
                    "5ef667dbbe0795688da93d779828cf38e6ad8ef8e82aa7dc5804873a50556057",
                    39_870_124,
                ),
                modelFile(
                    sourceId,
                    "lexicon.txt",
                    "ab2e61d357551e7b24ddd965d924aca784c20165ff58c150794e539c6b5e9e35",
                    2_042_943,
                ),
                modelFile(
                    sourceId,
                    "tokens.txt",
                    "50b45a7b7de1752fd3c7b4755661c285f1547f59186eca2281089a81307ad953",
                    1_671,
                ),
            ],
        }
    }

    /// Returns the five-speaker Chinese Sherpa ONNX VITS zh-ll manifest.
    pub fn sherpaOnnxVitsZhLlTts() -> LocalModelManifest {
        let sourceId = "huggingface-sherpa-onnx-vits-zh-ll";
        LocalModelManifest {
            id: SHERPA_ONNX_TTS_ZH_LL_ID.to_string(),
            version: SHERPA_ONNX_TTS_ZH_LL_REVISION.to_string(),
            displayName: "Sherpa ONNX VITS zh-ll".to_string(),
            description: "Local Chinese five-speaker speech synthesis.".to_string(),
            kind: LocalModelKind::TextToSpeech,
            engine: LocalEngineKind::SherpaOnnx,
            license: "community".to_string(),
            homepage: "https://huggingface.co/csukuangfj/sherpa-onnx-vits-zh-ll".to_string(),
            languages: vec!["zh".to_string()],
            tags: vec![
                "vits".to_string(),
                "multi-speaker".to_string(),
                "sherpa-onnx".to_string(),
            ],
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxVits {
                model: "model.onnx".to_string(),
                lexicon: "lexicon.txt".to_string(),
                tokens: "tokens.txt".to_string(),
                ruleFsts: vec![
                    "phone.fst".to_string(),
                    "number.fst".to_string(),
                    "date.fst".to_string(),
                    "new_heteronym.fst".to_string(),
                ],
                ruleFars: Vec::new(),
                speakerCount: 5,
            }),
            sources: vec![huggingFaceSource(
                sourceId,
                "csukuangfj/sherpa-onnx-vits-zh-ll",
                SHERPA_ONNX_TTS_ZH_LL_REVISION,
            )],
            installSource: LocalModelInstallSource::Files,
            files: vec![
                modelFile(
                    sourceId,
                    "model.onnx",
                    "6c349bdd73dc928234dd7bc86929748bba32cd5264d32d915bf7b7aa0595965b",
                    121_100_803,
                ),
                modelFile(
                    sourceId,
                    "lexicon.txt",
                    "b3a82f16b286c424953dea3686039e7ab465fa8e15d87ef8abd0ec69175beb21",
                    376_868,
                ),
                modelFile(
                    sourceId,
                    "tokens.txt",
                    "34b035b9aeb070df6188b022f29c00e0e142c7ade9f25611ced65db5e9cc8402",
                    331,
                ),
                modelFile(
                    sourceId,
                    "phone.fst",
                    "1ac2b6fa56b1442320c4de7db08353bab8963a2b57f365eebcdd3a2d3562f8d7",
                    88_630,
                ),
                modelFile(
                    sourceId,
                    "number.fst",
                    "743f402181fcfebf76cc2f0546b71fa26476e626fbe4e460fb7b4c3a7a8bd5bd",
                    64_482,
                ),
                modelFile(
                    sourceId,
                    "date.fst",
                    "eb8aa079ae3cb81d8f4404992f39d61a0cb990947512b5b8d1e54d1f6980e718",
                    59_154,
                ),
                modelFile(
                    sourceId,
                    "new_heteronym.fst",
                    "ca14b2127e27baa571664e4bb791e143e7425f56a6bc29db08d74f97e6aa4e29",
                    21_974,
                ),
            ],
        }
    }

    /// Returns the single-speaker Chinese Sherpa ONNX Matcha Baker manifest.
    pub fn sherpaOnnxMatchaBakerTts() -> LocalModelManifest {
        let modelSourceId = "huggingface-sherpa-onnx-matcha-baker";
        let vocoderSourceId = "github-sherpa-onnx-vocoder-models";
        LocalModelManifest {
            id: SHERPA_ONNX_MATCHA_BAKER_ID.to_string(),
            version: SHERPA_ONNX_MATCHA_BAKER_REVISION.to_string(),
            displayName: "Sherpa ONNX Matcha Baker zh".to_string(),
            description: "Local Chinese single-speaker Matcha speech synthesis.".to_string(),
            kind: LocalModelKind::TextToSpeech,
            engine: LocalEngineKind::SherpaOnnx,
            license: "non-commercial".to_string(),
            homepage: "https://huggingface.co/csukuangfj/matcha-icefall-zh-baker".to_string(),
            languages: vec!["zh".to_string()],
            tags: vec![
                "matcha".to_string(),
                "single-speaker".to_string(),
                "sherpa-onnx".to_string(),
            ],
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxMatcha {
                acousticModel: "model-steps-3.onnx".to_string(),
                vocoder: "vocos-22khz-univ.onnx".to_string(),
                lexicon: "lexicon.txt".to_string(),
                tokens: "tokens.txt".to_string(),
                ruleFsts: vec![
                    "phone.fst".to_string(),
                    "number.fst".to_string(),
                    "date.fst".to_string(),
                ],
                ruleFars: Vec::new(),
                speakerCount: 1,
            }),
            sources: vec![
                huggingFaceSource(
                    modelSourceId,
                    "csukuangfj/matcha-icefall-zh-baker",
                    SHERPA_ONNX_MATCHA_BAKER_REVISION,
                ),
                LocalModelSource {
                    id: vocoderSourceId.to_string(),
                    kind: LocalModelSourceKind::DirectHttp,
                    repository: "k2-fsa/sherpa-onnx".to_string(),
                    revision: "vocoder-models".to_string(),
                    baseUrl:
                        "https://github.com/k2-fsa/sherpa-onnx/releases/download/vocoder-models"
                            .to_string(),
                },
            ],
            installSource: LocalModelInstallSource::Files,
            files: vec![
                modelFile(
                    modelSourceId,
                    "model-steps-3.onnx",
                    "ef7ebdf5987e16a5836136a51d6f3560ca997ffd33d06a40daab5af92b4b86e5",
                    75_624_611,
                ),
                modelFile(
                    modelSourceId,
                    "tokens.txt",
                    "56209b2bf609d5ac1d66ede6dae7bf5254bd3f8aa24c4a6823713d5b884d87ba",
                    19_591,
                ),
                modelFile(
                    modelSourceId,
                    "lexicon.txt",
                    "38b886d46aefa50da6322a64d72fd595d5f4fae1051adb160d647541b1e0a4a2",
                    1_363_705,
                ),
                modelFile(
                    modelSourceId,
                    "phone.fst",
                    "1ac2b6fa56b1442320c4de7db08353bab8963a2b57f365eebcdd3a2d3562f8d7",
                    88_630,
                ),
                modelFile(
                    modelSourceId,
                    "number.fst",
                    "743f402181fcfebf76cc2f0546b71fa26476e626fbe4e460fb7b4c3a7a8bd5bd",
                    64_482,
                ),
                modelFile(
                    modelSourceId,
                    "date.fst",
                    "eb8aa079ae3cb81d8f4404992f39d61a0cb990947512b5b8d1e54d1f6980e718",
                    59_154,
                ),
                modelFile(
                    vocoderSourceId,
                    "vocos-22khz-univ.onnx",
                    "0574a135aa1db2de6e181050db2ec528496cacd4a4701fc5d7faf9f9804c0081",
                    53_884_024,
                ),
            ],
        }
    }

    /// Returns the eight-speaker English Sherpa ONNX Kitten nano int8 manifest.
    pub fn sherpaOnnxKittenNanoTts() -> LocalModelManifest {
        let sourceId = "github-sherpa-onnx-tts-models";
        let modelRoot = "kitten-nano-en-v0_8-int8";
        LocalModelManifest {
            id: SHERPA_ONNX_KITTEN_NANO_ID.to_string(),
            version: SHERPA_ONNX_KITTEN_NANO_VERSION.to_string(),
            displayName: "Sherpa ONNX Kitten nano en int8".to_string(),
            description: "Local English eight-speaker KittenTTS speech synthesis.".to_string(),
            kind: LocalModelKind::TextToSpeech,
            engine: LocalEngineKind::SherpaOnnx,
            license: "apache-2.0".to_string(),
            homepage: "https://github.com/KittenML/KittenTTS".to_string(),
            languages: vec!["en".to_string()],
            tags: vec![
                "kitten".to_string(),
                "multi-speaker".to_string(),
                "sherpa-onnx".to_string(),
                "archive".to_string(),
            ],
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxKitten {
                model: format!("{modelRoot}/model.int8.onnx"),
                voices: format!("{modelRoot}/voices.bin"),
                tokens: format!("{modelRoot}/tokens.txt"),
                dataDir: format!("{modelRoot}/espeak-ng-data"),
                speakerCount: 8,
            }),
            sources: vec![LocalModelSource {
                id: sourceId.to_string(),
                kind: LocalModelSourceKind::DirectHttp,
                repository: "k2-fsa/sherpa-onnx".to_string(),
                revision: "tts-models".to_string(),
                baseUrl: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models"
                    .to_string(),
            }],
            installSource: LocalModelInstallSource::Archives {
                archives: vec![LocalModelArchive {
                    archiveId: "kitten-nano-en-v0_8-int8".to_string(),
                    relativePath: "kitten-nano-en-v0_8-int8.tar.bz2".to_string(),
                    sha256: "6fa5be852612ce761094ba74ee6123b4fc4acfefa79bf64dc63acae4a83af2fd"
                        .to_string(),
                    byteSize: 31_220_690,
                    sourceId: sourceId.to_string(),
                    archiveFormat: LocalModelArchiveFormat::TarBz2,
                }],
            },
            files: vec![
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/model.int8.onnx"),
                    "0ba1e21eda9c8bcc4a70ada7e0d27fefc9ba775aaa037547248ec71f9a3d9b7d",
                    24_370_878,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/voices.bin"),
                    "d520519c4a3519d44fcfcd943ed0b1e3c5da5cee0eea501d922fac1a93cd24dc",
                    3_276_800,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/tokens.txt"),
                    "934a4188addc7665dd3410256bb622169242357fbb99d840d9351209b486dabb",
                    1_064,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/espeak-ng-data/phontab"),
                    "886f3fa402cb0ba73d483aa8ad000af47a6b7cc06293c75a97913fba68a530f6",
                    55_796,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/espeak-ng-data/phondata"),
                    "4e0288957874029a8c3c9f41a8f517ad4bf18127046decbdd4b9d1d6807ce3a3",
                    550_424,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/espeak-ng-data/phonindex"),
                    "3ca7b8fa3b42624e4b0f152707e7a39245fce569aa99ea47c055d9e622fcf0c4",
                    39_074,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/espeak-ng-data/en_dict"),
                    "71bd330ba8a2e3e8076e631508208ef49449d6147c17b7bd2b4b1e1468292e35",
                    166_944,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/espeak-ng-data/lang/gmw/en-US"),
                    "41534c2a22df5dd4f1052ff9e1a33a3ea7bff5a26b5c02bdad5ba8ddb7524704",
                    257,
                ),
            ],
        }
    }

    /// Returns the browser packaged Paraformer STT manifest.
    pub fn sherpaOnnxWebAsr() -> LocalModelManifest {
        let sourceId = "github-sherpa-onnx-web-asr";
        let modelRoot = "sherpa-onnx-wasm-simd-1.13.2-vad-asr-zh_en-paraformer_small";
        LocalModelManifest {
            id: SHERPA_ONNX_WEB_ASR_ID.to_string(),
            version: SHERPA_ONNX_WEB_ASR_VERSION.to_string(),
            displayName: "Sherpa ONNX Web Paraformer zh/en".to_string(),
            description: "Browser packaged Chinese and English Paraformer speech recognition."
                .to_string(),
            kind: LocalModelKind::SpeechToText,
            engine: LocalEngineKind::SherpaOnnx,
            license: "apache-2.0".to_string(),
            homepage: "https://github.com/k2-fsa/sherpa-onnx".to_string(),
            languages: vec!["zh".to_string(), "en".to_string()],
            tags: vec![
                "web".to_string(),
                "paraformer".to_string(),
                "sherpa-onnx".to_string(),
                "archive".to_string(),
            ],
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxWebAsrBundle {
                recognizerScript: format!("{modelRoot}/sherpa-onnx-asr.js"),
                runtimeScript: format!("{modelRoot}/sherpa-onnx-wasm-main-vad-asr.js"),
                runtimeWasm: format!("{modelRoot}/sherpa-onnx-wasm-main-vad-asr.wasm"),
                runtimeData: format!("{modelRoot}/sherpa-onnx-wasm-main-vad-asr.data"),
            }),
            sources: vec![LocalModelSource {
                id: sourceId.to_string(),
                kind: LocalModelSourceKind::DirectHttp,
                repository: "k2-fsa/sherpa-onnx".to_string(),
                revision: "v1.13.2".to_string(),
                baseUrl: "https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.2"
                    .to_string(),
            }],
            installSource: LocalModelInstallSource::Archives {
                archives: vec![LocalModelArchive {
                    archiveId: "web-paraformer-small-zh-en".to_string(),
                    relativePath:
                        "sherpa-onnx-wasm-simd-1.13.2-vad-asr-zh_en-paraformer_small.tar.bz2"
                            .to_string(),
                    sha256: "0390faa8d5855f11055aa68283b1203e47945d5234fdab537e677df3b8e6331f"
                        .to_string(),
                    byteSize: 80_500_154,
                    sourceId: sourceId.to_string(),
                    archiveFormat: LocalModelArchiveFormat::TarBz2,
                }],
            },
            files: vec![
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/sherpa-onnx-asr.js"),
                    "d51ae8e8b756ee5e53423ffada0c9702973f154f561aca7984fe0b12f4060178",
                    53_867,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/sherpa-onnx-wasm-main-vad-asr.js"),
                    "a5187acadf89293b6b3d9131e900cc4ce8d7b9b4498274bbedbe20884dc57fc5",
                    116_691,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/sherpa-onnx-wasm-main-vad-asr.wasm"),
                    "a1f3fb15701fad8c556af45d785ddd17dcfe8e25272aa1a02ba0369c9f2ce828",
                    12_898_602,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/sherpa-onnx-wasm-main-vad-asr.data"),
                    "9b89b7afaf5375704e758e85e88955abab381f81c466b24ecc7320b24d81b80f",
                    82_547_881,
                ),
            ],
        }
    }

    /// Returns the browser packaged VITS Piper TTS manifest.
    pub fn sherpaOnnxWebVitsTts() -> LocalModelManifest {
        let sourceId = "github-sherpa-onnx-web-tts";
        let modelRoot = "sherpa-onnx-wasm-simd-1.13.2-vits-piper-en_US-libritts_r-medium";
        LocalModelManifest {
            id: SHERPA_ONNX_WEB_TTS_ID.to_string(),
            version: SHERPA_ONNX_WEB_TTS_VERSION.to_string(),
            displayName: "Sherpa ONNX Web VITS Piper LibriTTS-R".to_string(),
            description: "Browser packaged English multi-speaker VITS speech synthesis."
                .to_string(),
            kind: LocalModelKind::TextToSpeech,
            engine: LocalEngineKind::SherpaOnnx,
            license: "cc-by-4.0".to_string(),
            homepage: "https://github.com/k2-fsa/sherpa-onnx".to_string(),
            languages: vec!["en".to_string()],
            tags: vec![
                "web".to_string(),
                "vits".to_string(),
                "piper".to_string(),
                "multi-speaker".to_string(),
                "sherpa-onnx".to_string(),
                "archive".to_string(),
            ],
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxWebTtsBundle {
                ttsScript: format!("{modelRoot}/sherpa-onnx-tts.js"),
                runtimeScript: format!("{modelRoot}/sherpa-onnx-wasm-main-tts.js"),
                runtimeWasm: format!("{modelRoot}/sherpa-onnx-wasm-main-tts.wasm"),
                runtimeData: format!("{modelRoot}/sherpa-onnx-wasm-main-tts.data"),
                speakerCount: 904,
            }),
            sources: vec![LocalModelSource {
                id: sourceId.to_string(),
                kind: LocalModelSourceKind::DirectHttp,
                repository: "k2-fsa/sherpa-onnx".to_string(),
                revision: "v1.13.2".to_string(),
                baseUrl: "https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.2"
                    .to_string(),
            }],
            installSource: LocalModelInstallSource::Archives {
                archives: vec![LocalModelArchive {
                    archiveId: "web-vits-piper-libritts-r-medium".to_string(),
                    relativePath:
                        "sherpa-onnx-wasm-simd-1.13.2-vits-piper-en_US-libritts_r-medium.tar.bz2"
                            .to_string(),
                    sha256: "bfcbcbea6faba2db04a1d303414647b4e256e30968b6fc426aca6b2eb1b3d9d5"
                        .to_string(),
                    byteSize: 85_160_582,
                    sourceId: sourceId.to_string(),
                    archiveFormat: LocalModelArchiveFormat::TarBz2,
                }],
            },
            files: vec![
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/sherpa-onnx-tts.js"),
                    "9970101676928fb126bec52eed4a1447d26d90d0f8a667f328d86cfd16e895bd",
                    32_010,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/sherpa-onnx-wasm-main-tts.js"),
                    "835100a0b5b7a0d4b78ffa654422c89630cd546e34c719e27bca21a952fee3bd",
                    144_036,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/sherpa-onnx-wasm-main-tts.wasm"),
                    "9554feafc2bf4452c3e1f5d5d4b29b690e6e7db1eb3835478a793e864111f640",
                    12_883_754,
                ),
                modelFile(
                    sourceId,
                    &format!("{modelRoot}/sherpa-onnx-wasm-main-tts.data"),
                    "bcf45b1441eb0aa228a3c6de1ea62a25f5a691eb99fdae68f3d6dc10f5e995f7",
                    96_525_193,
                ),
            ],
        }
    }

    /// Returns the built-in STT manifest used by the legacy assistance app.
    pub fn oldAssistanceSherpaNcnnStt() -> LocalModelManifest {
        LocalModelManifest {
            id: OLD_ASSISTANCE_STT_ID.to_string(),
            version: OLD_ASSISTANCE_STT_REVISION.to_string(),
            displayName: "Sherpa NCNN Zipformer zh/en STT".to_string(),
            description: "Streaming bilingual Chinese and English STT model used by the legacy assistance app.".to_string(),
            kind: LocalModelKind::SpeechToText,
            engine: LocalEngineKind::SherpaNcnn,
            license: "apache-2.0".to_string(),
            homepage: format!("https://huggingface.co/csukuangfj/{OLD_ASSISTANCE_STT_ID}"),
            languages: vec!["zh".to_string(), "en".to_string()],
            tags: vec![
                "legacy-assistance".to_string(),
                "streaming".to_string(),
                "zipformer".to_string(),
                "sherpa-ncnn".to_string(),
            ],
            engineRequirement: None,
            driver: Some(LocalModelDriver::SherpaNcnnStreamingTransducer {
                encoderParam: "encoder_jit_trace-pnnx.ncnn.param".to_string(),
                encoderBin: "encoder_jit_trace-pnnx.ncnn.bin".to_string(),
                decoderParam: "decoder_jit_trace-pnnx.ncnn.param".to_string(),
                decoderBin: "decoder_jit_trace-pnnx.ncnn.bin".to_string(),
                joinerParam: "joiner_jit_trace-pnnx.ncnn.param".to_string(),
                joinerBin: "joiner_jit_trace-pnnx.ncnn.bin".to_string(),
                tokens: "tokens.txt".to_string(),
            }),
            sources: vec![LocalModelSource {
                id: OLD_ASSISTANCE_STT_SOURCE_ID.to_string(),
                kind: LocalModelSourceKind::HuggingFace,
                repository: format!("csukuangfj/{OLD_ASSISTANCE_STT_ID}"),
                revision: OLD_ASSISTANCE_STT_REVISION.to_string(),
                baseUrl: format!(
                    "https://huggingface.co/csukuangfj/{OLD_ASSISTANCE_STT_ID}/resolve/{OLD_ASSISTANCE_STT_REVISION}"
                ),
            }],
            installSource: LocalModelInstallSource::Files,
            files: vec![
                file(
                    "encoder_jit_trace-pnnx.ncnn.param",
                    "97ad0954fb2cb4730f87a7eb66401b024f756752ece246e4b2063f870ebf3e18",
                    161_888,
                ),
                file(
                    "encoder_jit_trace-pnnx.ncnn.bin",
                    "4ed65f05b78c0106d3d176018ab01e26a15c200604490d3d49b08cc75a122dd0",
                    127_364_056,
                ),
                file(
                    "decoder_jit_trace-pnnx.ncnn.param",
                    "cb88f5894978fd3e85369d2f8ea55621809fceb2b5158243fb0cd025eb4f1aaf",
                    439,
                ),
                file(
                    "decoder_jit_trace-pnnx.ncnn.bin",
                    "dc4df2d8e1ddee1b90ac72a2de982eb1d320ee6c9a70e1dee4d23d9acfc8b978",
                    6_412_296,
                ),
                file(
                    "joiner_jit_trace-pnnx.ncnn.param",
                    "46c339f3869136c2f6d9d9d6983a6cbc2bfbcd0e3dab0f76ae25e9477f00a360",
                    490,
                ),
                file(
                    "joiner_jit_trace-pnnx.ncnn.bin",
                    "0e6c4370017394de5d74128756233d2e4451209e63ac2abd525da3b089e8bee1",
                    7_350_724,
                ),
                file(
                    "tokens.txt",
                    "a8e0e4ec53810e433789b54a5c0134a7eaa2ffca595a6334d54c00da858841d3",
                    56_317,
                ),
            ],
        }
    }
}

/// Builds the shared Sherpa ONNX engine requirement.
fn sherpaOnnxRequirement() -> LocalEngineRequirement {
    LocalEngineRequirement {
        engineId: SHERPA_ONNX_ENGINE_ID.to_string(),
        version: SHERPA_ONNX_ENGINE_VERSION.to_string(),
    }
}

/// Builds one pinned Hugging Face model source.
fn huggingFaceSource(id: &str, repository: &str, revision: &str) -> LocalModelSource {
    LocalModelSource {
        id: id.to_string(),
        kind: LocalModelSourceKind::HuggingFace,
        repository: repository.to_string(),
        revision: revision.to_string(),
        baseUrl: format!("https://huggingface.co/{repository}/resolve/{revision}"),
    }
}

/// Builds one model file entry bound to a source id.
fn modelFile(sourceId: &str, relativePath: &str, sha256: &str, byteSize: u64) -> LocalModelFile {
    LocalModelFile {
        relativePath: relativePath.to_string(),
        sha256: sha256.to_string(),
        byteSize,
        sourceId: sourceId.to_string(),
    }
}

/// Builds a manifest file entry bound to the built-in Hugging Face source.
fn file(relativePath: &str, sha256: &str, byteSize: u64) -> LocalModelFile {
    modelFile(OLD_ASSISTANCE_STT_SOURCE_ID, relativePath, sha256, byteSize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalEngineManifest::LocalPlatform;

    /// Verifies the legacy STT manifest files and declared byte size.
    #[test]
    fn oldAssistanceSherpaNcnnSttDeclaresRequiredFiles() {
        let manifest = LocalModelCatalog::oldAssistanceSherpaNcnnStt();
        let paths: Vec<&str> = manifest
            .files
            .iter()
            .map(|file| file.relativePath.as_str())
            .collect();

        assert_eq!(
            paths,
            vec![
                "encoder_jit_trace-pnnx.ncnn.param",
                "encoder_jit_trace-pnnx.ncnn.bin",
                "decoder_jit_trace-pnnx.ncnn.param",
                "decoder_jit_trace-pnnx.ncnn.bin",
                "joiner_jit_trace-pnnx.ncnn.param",
                "joiner_jit_trace-pnnx.ncnn.bin",
                "tokens.txt",
            ]
        );
        assert_eq!(manifest.declaredByteSize(), 141_346_210);
        assert_eq!(manifest.sources.len(), 1);
        assert_eq!(manifest.sources[0].id, OLD_ASSISTANCE_STT_SOURCE_ID);
    }

    /// Verifies the active catalog contains executable STT and TTS driver metadata.
    #[test]
    fn activeCatalogDeclaresSpeechDrivers() {
        let manifests = LocalModelCatalog::manifests();
        assert_eq!(manifests.len(), 7);
        assert!(manifests.iter().all(|manifest| manifest.driver.is_some()));
        assert!(manifests
            .iter()
            .all(|manifest| manifest.engineRequirement.is_some()));
    }

    /// Verifies browser bundles and native models cannot be selected on the wrong runtime.
    #[test]
    fn activeCatalogSeparatesNativeAndWebDrivers() {
        let nativeModels = [
            LocalModelCatalog::sherpaOnnxStreamingStt(),
            LocalModelCatalog::sherpaOnnxVitsTts(),
            LocalModelCatalog::sherpaOnnxVitsZhLlTts(),
            LocalModelCatalog::sherpaOnnxMatchaBakerTts(),
            LocalModelCatalog::sherpaOnnxKittenNanoTts(),
        ];
        for model in nativeModels {
            assert!(model.supportsPlatform(&LocalPlatform::Windows));
            assert!(model.supportsPlatform(&LocalPlatform::Android));
            assert!(model.supportsPlatform(&LocalPlatform::Ohos));
            assert!(model.supportsPlatform(&LocalPlatform::Ios));
            assert!(!model.supportsPlatform(&LocalPlatform::Web));
        }

        let webModels = [
            LocalModelCatalog::sherpaOnnxWebAsr(),
            LocalModelCatalog::sherpaOnnxWebVitsTts(),
        ];
        for model in webModels {
            assert!(model.supportsPlatform(&LocalPlatform::Web));
            assert!(!model.supportsPlatform(&LocalPlatform::Windows));
            assert!(!model.supportsPlatform(&LocalPlatform::Android));
            assert!(!model.supportsPlatform(&LocalPlatform::Ohos));
            assert!(!model.supportsPlatform(&LocalPlatform::Ios));
        }
    }

    /// Verifies the Kitten manifest declares the official archive download bytes.
    #[test]
    fn kittenNanoTtsDeclaresArchiveInstallSource() {
        let manifest = LocalModelCatalog::sherpaOnnxKittenNanoTts();

        assert_eq!(manifest.declaredByteSize(), 31_220_690);
        assert!(matches!(
            manifest.installSource,
            LocalModelInstallSource::Archives { .. }
        ));
    }
}
