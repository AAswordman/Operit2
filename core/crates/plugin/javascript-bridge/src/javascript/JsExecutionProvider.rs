use std::sync::Arc;

use operit_plugin_sdk::javascript::{
    JsExecutionEngine, JsExecutionFuture, JsExecutionHost, JsExecutionProvider, JsPackageExecutor,
    JsPackageRuntime, ToolPkgExecutionContext,
};
use operit_plugin_sdk::toolpkg::ToolPkgManager::ToolPkgExecutionEngineFactory;

use crate::javascript::JsEngine::{JsEngine, PreparedJsEngine};
use crate::javascript::JsToolManager::JsToolManager;
use crate::javascript::PreparedEnginePool::PreparedEnginePool;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_util::AppLogger::AppLogger;

/// Provides Host-backed execution for caller-owned SDK contracts.
pub struct HostJsExecutionProvider {
    prepared_engines: Arc<PreparedEnginePool<PreparedJsEngine>>,
}

impl HostJsExecutionProvider {
    /// Starts preparing two unbound engines through the application's Host scheduler.
    pub fn new() -> Self {
        let scheduler = defaultHostRuntimeTaskSchedulerHost();
        let prepared_engines = PreparedEnginePool::new(
            Arc::new(|| {
                let engine = PreparedJsEngine::new();
                if let Err(error) = &engine {
                    AppLogger::e(
                        "JsEnginePool",
                        &format!("event=engine_prepare_failed error={error}"),
                    );
                }
                engine
            }),
            Arc::new(move |task| {
                scheduler
                    .scheduleHostRuntimeTask("ToolPkgEnginePrepare", task)
                    .map_err(|error| error.to_string())
            }),
        );
        Self { prepared_engines }
    }
}

impl Default for HostJsExecutionProvider {
    /// Creates the default unbound Host JavaScript execution provider.
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct HostJsExecutionEngineFactory {
    execution_host: Arc<dyn JsExecutionHost>,
    prepared_engines: Arc<PreparedEnginePool<PreparedJsEngine>>,
}

impl ToolPkgExecutionEngineFactory for HostJsExecutionEngineFactory {
    /// Creates one Host JavaScript engine without a ToolPkg package environment.
    #[allow(non_snake_case)]
    fn createExecutionEngine(&self) -> Result<Arc<dyn JsExecutionEngine>, String> {
        JsEngine::new(self.execution_host.clone())
            .map(|engine| Arc::new(engine) as Arc<dyn JsExecutionEngine>)
            .map_err(|error| error.to_string())
    }

    /// Creates one Host JavaScript engine bound to the supplied execution host.
    #[allow(non_snake_case)]
    fn createToolPkgExecutionEngine(
        &self,
        context: ToolPkgExecutionContext,
    ) -> Result<Arc<dyn JsExecutionEngine>, String> {
        JsEngine::new_toolpkg_execution_engine(self.execution_host.clone(), context)
            .map(|engine| Arc::new(engine) as Arc<dyn JsExecutionEngine>)
            .map_err(|error| error.to_string())
    }
    /// Acquires a pristine prepared engine for a package executor's asynchronous callers.
    #[allow(non_snake_case)]
    fn acquireToolPkgExecutionEngine(
        &self,
        context: ToolPkgExecutionContext,
    ) -> JsExecutionFuture<Result<Arc<dyn JsExecutionEngine>, String>> {
        acquirePreparedEngine(
            self.prepared_engines.clone(),
            self.execution_host.clone(),
            context,
        )
    }
}

impl JsExecutionProvider for HostJsExecutionProvider {
    /// Creates one Host JavaScript engine bound to a caller-owned execution host.
    fn create_execution_engine(
        &self,
        execution_host: Arc<dyn JsExecutionHost>,
    ) -> Result<Arc<dyn JsExecutionEngine>, String> {
        JsEngine::new(execution_host)
            .map(|engine| Arc::new(engine) as Arc<dyn JsExecutionEngine>)
            .map_err(|error| error.to_string())
    }

    /// Creates one Host JavaScript engine bound to a ToolPkg package environment.
    fn create_toolpkg_execution_engine(
        &self,
        execution_host: Arc<dyn JsExecutionHost>,
        context: ToolPkgExecutionContext,
    ) -> Result<Arc<dyn JsExecutionEngine>, String> {
        JsEngine::new_toolpkg_execution_engine(execution_host, context)
            .map(|engine| Arc::new(engine) as Arc<dyn JsExecutionEngine>)
            .map_err(|error| error.to_string())
    }

    /// Awaits a prepared engine and installs its exact package identity once.
    fn acquire_toolpkg_execution_engine(
        &self,
        execution_host: Arc<dyn JsExecutionHost>,
        context: ToolPkgExecutionContext,
    ) -> JsExecutionFuture<Result<Arc<dyn JsExecutionEngine>, String>> {
        acquirePreparedEngine(self.prepared_engines.clone(), execution_host, context)
    }

    /// Creates one package executor bound to caller-owned runtime contracts.
    fn create_package_executor(
        &self,
        package_runtime: Arc<dyn JsPackageRuntime>,
        execution_host: Arc<dyn JsExecutionHost>,
    ) -> Arc<dyn JsPackageExecutor> {
        Arc::new(JsToolManager::new(
            package_runtime,
            Arc::new(HostJsExecutionEngineFactory {
                execution_host,
                prepared_engines: self.prepared_engines.clone(),
            }),
        ))
    }
}

/// Keeps waiting asynchronous and releases an unclaimed engine when acquisition is cancelled.
#[allow(non_snake_case)]
fn acquirePreparedEngine(
    pool: Arc<PreparedEnginePool<PreparedJsEngine>>,
    host: Arc<dyn JsExecutionHost>,
    context: ToolPkgExecutionContext,
) -> JsExecutionFuture<Result<Arc<dyn JsExecutionEngine>, String>> {
    Box::pin(async move {
        context.validate()?;
        let engine = pool
            .request()
            .await
            .map_err(|_| "Prepared engine pool closed during acquisition".to_string())??;
        let engine = engine.bind(host, context).await?;
        Ok(Arc::new(engine) as Arc<dyn JsExecutionEngine>)
    })
}
