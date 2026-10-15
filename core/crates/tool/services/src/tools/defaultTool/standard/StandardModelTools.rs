#![allow(non_snake_case)]

use super::ToolArguments::{decodeArguments, EmptyArguments};
use crate::tools::ToolInvocation::prepareInvocation;
use crate::tools::ToolResultDataClasses::{ModelsListSummariesResultData, ToolResultData};
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};

/// Executes model discovery through the registered tool pipeline.
pub(crate) struct ModelToolExecutor;

impl SharedToolExecutor for ModelToolExecutor {
    fn prepare<'a>(
        &'a self,
        request: &'a ToolCall,
        context: &'a ToolInvocationContext,
        checkAccess: bool,
    ) -> Result<PreparedToolInvocation<'a>, String> {
        let _: EmptyArguments = decodeArguments(request)?;
        Ok(prepareInvocation(
            request,
            checkAccess,
            ToolEffect::READ,
            async move {
                let models = context.host().runtimeSupport().listModelSummaries()?;
                Ok(ToolResultData::ModelsListSummariesResultData(
                    ModelsListSummariesResultData { models },
                ))
            },
        ))
    }
}
