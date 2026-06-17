use std::future::Future;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::chat::enhance::MultiServiceManager::MultiServiceManager;
use crate::api::chat::llmprovider::AIService::{collect_stream_chunks, SendMessageRequest};
use crate::api::chat::llmprovider::MediaLinkBuilder::MediaLinkBuilder;
use crate::core::chat::hooks::PromptTurn::{toPromptTurns, PromptTurn, PromptTurnKind};
use crate::data::model::ModelConfigData::ResolvedModelConfig;
use crate::data::model::ToolPrompt::{ToolParameterSchema, ToolPrompt};
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::util::ChatMarkupRegex::ChatMarkupRegex;
use crate::util::ImagePoolManager::ImagePoolManager;
use crate::util::MediaPoolManager::MediaPoolManager;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelConnectionTestType {
    CHAT,
    TOOL_CALL,
    IMAGE,
    AUDIO,
    VIDEO,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelConnectionTestItem {
    pub r#type: ModelConnectionTestType,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ModelConnectionTestReport {
    pub providerId: String,
    pub modelId: String,
    pub providerName: String,
    pub providerType: String,
    pub success: bool,
    pub items: Vec<ModelConnectionTestItem>,
}

pub struct ModelConfigConnectionTester;

impl ModelConfigConnectionTester {
    pub async fn run(
        rootDir: PathBuf,
        providerId: &str,
        modelId: &str,
    ) -> Result<ModelConnectionTestReport, String> {
        let modelConfigManager = ModelConfigManager::new(rootDir.clone());
        modelConfigManager
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        let config = modelConfigManager
            .getResolvedModelConfig(providerId, modelId)
            .map_err(|error| error.to_string())?;
        let mut items = Vec::new();

        let mut serviceManager = MultiServiceManager::new(rootDir.clone());
        let bundleResult = serviceManager
            .createTransientServiceBundleForModel(providerId.to_string(), modelId.to_string());
        let (configForTest, parameters, serviceHandle) = match bundleResult {
            Ok(bundle) => bundle,
            Err(error) => {
                items.push(ModelConnectionTestItem {
                    r#type: ModelConnectionTestType::CHAT,
                    success: false,
                    error: Some(error.to_string()),
                });
                return Ok(Self::report(config, items));
            }
        };

        {
            let mut service = serviceHandle;
            Self::runCase(&mut items, ModelConnectionTestType::CHAT, || async {
                let stream = service
                    .send_message(SendMessageRequest {
                        chat_history: vec![PromptTurn::new(PromptTurnKind::USER, "Hi")],
                        model_parameters: parameters.clone(),
                        enable_thinking: false,
                        stream: false,
                        available_tools: Vec::new(),
                        preserve_think_in_history: false,
                        enable_retry: false,
                        on_non_fatal_error: None,
                        on_tool_invocation: None,
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                let _ = collect_stream_chunks(stream);
                Ok(())
            })
            .await;

            if configForTest.capabilities.toolCall {
                Self::runCase(&mut items, ModelConnectionTestType::TOOL_CALL, || async {
                    let toolTagName = ChatMarkupRegex::generate_random_tool_tag_name();
                    let toolResultTagName = ChatMarkupRegex::generate_random_tool_result_tag_name();
                    let history = vec![
                        ("system".to_string(), "You are a helpful assistant.".to_string()),
                        (
                            "assistant".to_string(),
                            format!(
                                "<{toolTagName} name=\"echo\"><param name=\"text\">ping</param></{toolTagName}>"
                            ),
                        ),
                        (
                            "user".to_string(),
                            format!(
                                "<{toolResultTagName} name=\"echo\" status=\"success\"><content>pong</content></{toolResultTagName}>"
                            ),
                        ),
                    ];
                    let stream = service
                        .send_message(SendMessageRequest {
                            chat_history: toPromptTurns(&history),
                            model_parameters: parameters.clone(),
                            enable_thinking: false,
                            stream: false,
                            available_tools: vec![Self::echoToolPrompt()],
                            preserve_think_in_history: false,
                            enable_retry: false,
                            on_non_fatal_error: None,
                            on_tool_invocation: None,
                        })
                        .await
                        .map_err(|error| error.to_string())?;
                    let _ = collect_stream_chunks(stream);
                    Ok(())
                })
                .await;
            }

            if configForTest.capabilities.directImage {
                Self::runCase(&mut items, ModelConnectionTestType::IMAGE, || async {
                    let imagePath = Self::copyAssetToCache(
                        &rootDir,
                        "1.jpg",
                        include_bytes!("../../../../assets/test/1.jpg"),
                    )
                    .map_err(|error| error.to_string())?;
                    let imageId = ImagePoolManager::add_image(&imagePath.to_string_lossy(), None);
                    if imageId == "error" {
                        let _ = std::fs::remove_file(&imagePath);
                        return Err("Failed to create test image".to_string());
                    }
                    let prompt = format!(
                        "{}\nPlease analyze this image briefly.",
                        MediaLinkBuilder::image(&imageId)
                    );
                    let result = service
                        .send_message(SendMessageRequest {
                            chat_history: vec![PromptTurn::new(PromptTurnKind::USER, prompt)],
                            model_parameters: parameters.clone(),
                            enable_thinking: false,
                            stream: false,
                            available_tools: Vec::new(),
                            preserve_think_in_history: false,
                            enable_retry: false,
                            on_non_fatal_error: None,
                            on_tool_invocation: None,
                        })
                        .await
                        .map(|stream| {
                            let _ = collect_stream_chunks(stream);
                        })
                        .map_err(|error| error.to_string());
                    ImagePoolManager::remove_image(&imageId);
                    let _ = std::fs::remove_file(&imagePath);
                    result
                })
                .await;
            }

            if configForTest.capabilities.directAudio {
                Self::runCase(&mut items, ModelConnectionTestType::AUDIO, || async {
                    let audioPath = Self::copyAssetToCache(
                        &rootDir,
                        "1.mp3",
                        include_bytes!("../../../../assets/test/1.mp3"),
                    )
                    .map_err(|error| error.to_string())?;
                    let audioId =
                        MediaPoolManager::add_media(&audioPath.to_string_lossy(), "audio/mpeg");
                    if audioId == "error" {
                        let _ = std::fs::remove_file(&audioPath);
                        return Err("Failed to create test audio".to_string());
                    }
                    let prompt = format!(
                        "{}\nPlease analyze this audio briefly.",
                        MediaLinkBuilder::audio(&audioId)
                    );
                    let result = service
                        .send_message(SendMessageRequest {
                            chat_history: vec![PromptTurn::new(PromptTurnKind::USER, prompt)],
                            model_parameters: parameters.clone(),
                            enable_thinking: false,
                            stream: false,
                            available_tools: Vec::new(),
                            preserve_think_in_history: false,
                            enable_retry: false,
                            on_non_fatal_error: None,
                            on_tool_invocation: None,
                        })
                        .await
                        .map(|stream| {
                            let _ = collect_stream_chunks(stream);
                        })
                        .map_err(|error| error.to_string());
                    MediaPoolManager::remove_media(&audioId);
                    let _ = std::fs::remove_file(&audioPath);
                    result
                })
                .await;
            }

            if configForTest.capabilities.directVideo {
                Self::runCase(&mut items, ModelConnectionTestType::VIDEO, || async {
                    let videoPath = Self::copyAssetToCache(
                        &rootDir,
                        "1.mp4",
                        include_bytes!("../../../../assets/test/1.mp4"),
                    )
                    .map_err(|error| error.to_string())?;
                    let videoId =
                        MediaPoolManager::add_media(&videoPath.to_string_lossy(), "video/mp4");
                    if videoId == "error" {
                        let _ = std::fs::remove_file(&videoPath);
                        return Err("Failed to create test video".to_string());
                    }
                    let prompt = format!(
                        "{}\nPlease analyze this video briefly.",
                        MediaLinkBuilder::video(&videoId)
                    );
                    let result = service
                        .send_message(SendMessageRequest {
                            chat_history: vec![PromptTurn::new(PromptTurnKind::USER, prompt)],
                            model_parameters: parameters.clone(),
                            enable_thinking: false,
                            stream: false,
                            available_tools: Vec::new(),
                            preserve_think_in_history: false,
                            enable_retry: false,
                            on_non_fatal_error: None,
                            on_tool_invocation: None,
                        })
                        .await
                        .map(|stream| {
                            let _ = collect_stream_chunks(stream);
                        })
                        .map_err(|error| error.to_string());
                    MediaPoolManager::remove_media(&videoId);
                    let _ = std::fs::remove_file(&videoPath);
                    result
                })
                .await;
            }

            service.cancel_streaming();
            service.release();
        }

        Ok(Self::report(configForTest, items))
    }

    async fn runCase<F, Fut>(
        items: &mut Vec<ModelConnectionTestItem>,
        r#type: ModelConnectionTestType,
        block: F,
    ) where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<(), String>>,
    {
        match block().await {
            Ok(()) => items.push(ModelConnectionTestItem {
                r#type,
                success: true,
                error: None,
            }),
            Err(error) => items.push(ModelConnectionTestItem {
                r#type,
                success: false,
                error: Some(error),
            }),
        }
    }

    fn report(
        config: ResolvedModelConfig,
        items: Vec<ModelConnectionTestItem>,
    ) -> ModelConnectionTestReport {
        ModelConnectionTestReport {
            providerId: config.providerId,
            modelId: config.modelId,
            providerName: config.providerName,
            providerType: config.apiProviderTypeId,
            success: items.iter().all(|item| item.success),
            items,
        }
    }

    fn echoToolPrompt() -> ToolPrompt {
        let mut tool = ToolPrompt::new("echo".to_string(), "Echoes the provided text.".to_string());
        tool.parametersStructured = Some(vec![ToolParameterSchema {
            name: "text".to_string(),
            r#type: "string".to_string(),
            description: "Text to echo.".to_string(),
            required: true,
            default: None,
        }]);
        tool
    }

    fn copyAssetToCache(
        rootDir: &Path,
        fileName: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, std::io::Error> {
        let dir = RuntimeStorePaths::new(rootDir.to_path_buf()).model_connection_test_cache_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}_{}", uuid::Uuid::new_v4(), fileName));
        std::fs::write(&path, bytes)?;
        Ok(path)
    }
}
