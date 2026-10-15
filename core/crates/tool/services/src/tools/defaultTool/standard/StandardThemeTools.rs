#![allow(non_snake_case)]

use super::ToolArguments::{decodeArguments, decodeConfigArguments, EmptyArguments};
use crate::tools::ToolInvocation::prepareInvocation;
use crate::tools::ToolResultDataClasses::{
    ThemeApplyConfigResultData, ThemeListConfigsResultData, ToolResultData,
};
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};

/// Selects the registered appearance configuration operation.
#[derive(Clone, Copy)]
pub(crate) enum ThemeToolOperation {
    ListConfigs,
    ApplyConfig,
}

/// Executes appearance configuration operations through the standard tool lifecycle.
pub(crate) struct ThemeToolExecutor {
    pub operation: ThemeToolOperation,
}

impl SharedToolExecutor for ThemeToolExecutor {
    fn prepare<'a>(
        &'a self,
        request: &'a ToolCall,
        context: &'a ToolInvocationContext,
        checkAccess: bool,
    ) -> Result<PreparedToolInvocation<'a>, String> {
        match self.operation {
            ThemeToolOperation::ListConfigs => {
                let _: EmptyArguments = decodeArguments(request)?;
                Ok(prepareInvocation(
                    request,
                    checkAccess,
                    ToolEffect::READ,
                    async move {
                        let configs = context.host().runtimeSupport().listThemeConfigs()?;
                        Ok(ToolResultData::ThemeListConfigsResultData(
                            ThemeListConfigsResultData { configs },
                        ))
                    },
                ))
            }
            ThemeToolOperation::ApplyConfig => {
                let args = decodeConfigArguments(request)?;
                Ok(prepareInvocation(
                    request,
                    checkAccess,
                    ToolEffect::WRITE,
                    async move {
                        let config = context.host().runtimeSupport().applyThemeConfig(args.id)?;
                        Ok(ToolResultData::ThemeApplyConfigResultData(
                            ThemeApplyConfigResultData::from(config),
                        ))
                    },
                ))
            }
        }
    }
}
