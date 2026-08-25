use std::sync::Arc;

use operit_plugin_sdk::javascript::{
    JsExecutionEngine, JsExecutionHost, JsExecutionProvider, JsPackageExecutor, JsPackageRuntime,
    ToolPkgExecutionContext,
};
use operit_plugin_sdk::toolpkg::ToolPkgManager::ToolPkgExecutionEngineFactory;

use crate::javascript::JsEngine::JsEngine;
use crate::javascript::JsToolManager::JsToolManager;

/// Provides QuickJS-backed execution for caller-owned SDK contracts.
pub struct QuickJsExecutionProvider;

impl QuickJsExecutionProvider {
    /// Creates an unbound QuickJS execution provider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuickJsExecutionProvider {
    /// Creates the default unbound QuickJS execution provider.
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct QuickJsExecutionEngineFactory {
    execution_host: Arc<dyn JsExecutionHost>,
}

impl ToolPkgExecutionEngineFactory for QuickJsExecutionEngineFactory {
    /// Creates one QuickJS engine without a ToolPkg package environment.
    #[allow(non_snake_case)]
    fn createExecutionEngine(&self) -> Arc<dyn JsExecutionEngine> {
        Arc::new(JsEngine::new(self.execution_host.clone()))
    }

    /// Creates one QuickJS engine bound to the supplied execution host.
    #[allow(non_snake_case)]
    fn createToolPkgExecutionEngine(
        &self,
        context: ToolPkgExecutionContext,
    ) -> Arc<dyn JsExecutionEngine> {
        Arc::new(JsEngine::new_toolpkg_execution_engine(
            self.execution_host.clone(),
            context,
        ))
    }
}

impl JsExecutionProvider for QuickJsExecutionProvider {
    /// Creates one QuickJS engine bound to a caller-owned execution host.
    fn create_execution_engine(
        &self,
        execution_host: Arc<dyn JsExecutionHost>,
    ) -> Arc<dyn JsExecutionEngine> {
        Arc::new(JsEngine::new(execution_host))
    }

    /// Creates one QuickJS engine bound to a ToolPkg package environment.
    fn create_toolpkg_execution_engine(
        &self,
        execution_host: Arc<dyn JsExecutionHost>,
        context: ToolPkgExecutionContext,
    ) -> Arc<dyn JsExecutionEngine> {
        Arc::new(JsEngine::new_toolpkg_execution_engine(
            execution_host,
            context,
        ))
    }

    /// Creates one package executor bound to caller-owned runtime contracts.
    fn create_package_executor(
        &self,
        package_runtime: Arc<dyn JsPackageRuntime>,
        execution_host: Arc<dyn JsExecutionHost>,
    ) -> Arc<dyn JsPackageExecutor> {
        Arc::new(JsToolManager::new(
            package_runtime,
            Arc::new(QuickJsExecutionEngineFactory { execution_host }),
        ))
    }
}
