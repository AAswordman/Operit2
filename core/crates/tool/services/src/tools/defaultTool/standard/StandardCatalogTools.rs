#![allow(non_snake_case)]

use super::ToolArguments::{decodeArguments, EmptyArguments};
use crate::tools::ToolInvocation::prepareInvocation;
use crate::tools::ToolResultDataClasses::{CatalogReadToolSourcesResultData, ToolResultData};
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};

/// Reads the actual tool catalog through the registered tool pipeline.
pub(crate) struct CatalogToolExecutor;

impl SharedToolExecutor for CatalogToolExecutor {
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
                let sources = context
                    .host()
                    .runtimeSupport()
                    .readToolSourceCatalog()
                    .await?;
                Ok(ToolResultData::CatalogReadToolSourcesResultData(
                    CatalogReadToolSourcesResultData {
                        builtinTools: sources.builtinTools,
                        packages: sources.packages,
                        skills: sources.skills,
                        mcpServers: sources.mcpServers,
                    },
                ))
            },
        ))
    }
}
