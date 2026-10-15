#![allow(non_snake_case)]

use std::future::Future;

use crate::tools::ToolResultDataClasses::{stringResultData, ToolResultData};
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    PreparedToolInvocation, ToolAccessSpec, ToolBoundary, ToolCall, ToolEffect,
};

/// Produces the common result envelope for an already validated standard tool invocation.
pub(crate) fn prepareInvocation<'a>(
    request: &'a ToolCall,
    checkAccess: bool,
    effect: ToolEffect,
    invocation: impl Future<Output = Result<ToolResultData, String>> + Send + 'a,
) -> PreparedToolInvocation<'a> {
    let access = checkAccess.then_some(ToolAccessSpec {
        effect,
        boundary: ToolBoundary::None,
    });
    PreparedToolInvocation {
        access,
        invoke: Box::pin(async move {
            let (success, result, error) = match invocation.await {
                Ok(result) => (true, result, None),
                Err(error) => (false, stringResultData(""), Some(error)),
            };
            vec![ToolResult {
                toolName: request.name.clone(),
                success,
                result,
                error,
            }]
        }),
    }
}
