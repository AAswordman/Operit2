#![allow(non_snake_case)]

use super::ToolArguments::{decodeArguments, decodeConfigArguments, EmptyArguments, RequestArguments};
use crate::tools::ToolInvocation::prepareInvocation;
use crate::tools::ToolResultDataClasses::{
    ToolResultData, TtsGetCurrentConfigIdResultData, TtsListConfigsResultData,
    TtsSetCurrentConfigIdResultData,
};
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};

/// Selects the registered speech configuration operation.
#[derive(Clone, Copy)]
pub(crate) enum TtsToolOperation {
    Synthesize,
    Speak,
    ListConfigs,
    GetCurrentConfigId,
    SetCurrentConfigId,
}

/// Executes speech configuration operations through the standard tool lifecycle.
pub(crate) struct TtsToolExecutor {
    pub operation: TtsToolOperation,
}

impl SharedToolExecutor for TtsToolExecutor {
    /// Validates speech requests before entering the standard tool lifecycle.
    fn prepare<'a>(
        &'a self,
        request: &'a ToolCall,
        context: &'a ToolInvocationContext,
        checkAccess: bool,
    ) -> Result<PreparedToolInvocation<'a>, String> {
        match self.operation {
            TtsToolOperation::Synthesize => {
                let args: RequestArguments<operit_plugin_sdk::js_sdk::tts::TtsSynthesizeRequest> = decodeArguments(request)?;
                if args.request.text.trim().is_empty() { return Err("TTS text is empty".into()); }
                Ok(prepareInvocation(request, checkAccess, ToolEffect::READ, async move {
                    Ok(ToolResultData::TtsSynthesizeResultData(context.host().runtimeSupport().synthesizeSpeech(args.request)?))
                }))
            }
            TtsToolOperation::Speak => {
                let args: RequestArguments<operit_plugin_sdk::js_sdk::tts::TtsSpeakRequest> = decodeArguments(request)?;
                if args.request.text.trim().is_empty() {
                    return Err("TTS text is empty".into());
                }
                Ok(prepareInvocation(
                    request,
                    checkAccess,
                    ToolEffect::WRITE,
                    async move {
                        let result = context.host().runtimeSupport().speakSpeech(args.request)?;
                        Ok(ToolResultData::TtsSpeakResultData(result))
                    },
                ))
            }
            TtsToolOperation::ListConfigs => {
                let _: EmptyArguments = decodeArguments(request)?;
                Ok(prepareInvocation(
                    request,
                    checkAccess,
                    ToolEffect::READ,
                    async move {
                        let configs = context.host().runtimeSupport().listTtsConfigs()?;
                        Ok(ToolResultData::TtsListConfigsResultData(
                            TtsListConfigsResultData { configs },
                        ))
                    },
                ))
            }
            TtsToolOperation::GetCurrentConfigId => {
                let _: EmptyArguments = decodeArguments(request)?;
                Ok(prepareInvocation(
                    request,
                    checkAccess,
                    ToolEffect::READ,
                    async move {
                        let configId = context.host().runtimeSupport().getCurrentTtsConfigId()?;
                        Ok(ToolResultData::TtsGetCurrentConfigIdResultData(
                            TtsGetCurrentConfigIdResultData { configId },
                        ))
                    },
                ))
            }
            TtsToolOperation::SetCurrentConfigId => {
                let args = decodeConfigArguments(request)?;
                Ok(prepareInvocation(
                    request,
                    checkAccess,
                    ToolEffect::WRITE,
                    async move {
                        let configId = context
                            .host()
                            .runtimeSupport()
                            .setCurrentTtsConfigId(args.id)?;
                        Ok(ToolResultData::TtsSetCurrentConfigIdResultData(
                            TtsSetCurrentConfigIdResultData { configId },
                        ))
                    },
                ))
            }
        }
    }
}

/// Executes recognition on this host; no device transport or chat orchestration.
pub(crate) struct SttToolExecutor;
impl SharedToolExecutor for SttToolExecutor {
    fn prepare<'a>(&'a self, request: &'a ToolCall, context: &'a ToolInvocationContext, checkAccess: bool)
        -> Result<PreparedToolInvocation<'a>, String> {
        let args: RequestArguments<operit_plugin_sdk::js_sdk::stt::SttTranscribeRequest> = decodeArguments(request)?;
        if args.request.audio.is_empty() { return Err("STT audio is empty".into()); }
        Ok(prepareInvocation(request, checkAccess, ToolEffect::READ, async move {
            Ok(ToolResultData::SttTranscribeResultData(context.host().runtimeSupport().transcribeSpeech(args.request)?))
        }))
    }
}
