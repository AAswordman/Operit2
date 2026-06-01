use std::sync::{Arc, OnceLock};

use operit_host_api::{
    BrowserAutomationHost, FileSystemHost, HostEnvironmentDescriptor, HttpHost, ManagedRuntimeHost,
    RuntimeSqliteHost, RuntimeStorageHost, SystemOperationHost, TerminalHost, WebVisitHost,
};

static DEFAULT_HTTP_HOST: OnceLock<Arc<dyn HttpHost>> = OnceLock::new();

pub type CoreCommandExecutor = Arc<dyn Fn(Vec<String>) -> Result<String, String> + Send + Sync>;

#[allow(non_snake_case)]
pub fn setDefaultHttpHost(host: Arc<dyn HttpHost>) {
    let _ = DEFAULT_HTTP_HOST.set(host);
}

#[allow(non_snake_case)]
pub fn defaultHttpHost() -> Arc<dyn HttpHost> {
    DEFAULT_HTTP_HOST
        .get()
        .expect("HTTP host must be configured before using HTTP-backed runtime services")
        .clone()
}

#[derive(Clone, Default)]
pub struct OperitApplicationContext {
    pub fileSystemHost: Option<Arc<dyn FileSystemHost>>,
    pub webVisitHost: Option<Arc<dyn WebVisitHost>>,
    pub browserAutomationHost: Option<Arc<dyn BrowserAutomationHost>>,
    pub httpHost: Option<Arc<dyn HttpHost>>,
    pub systemOperationHost: Option<Arc<dyn SystemOperationHost>>,
    pub managedRuntimeHost: Option<Arc<dyn ManagedRuntimeHost>>,
    pub terminalHost: Option<Arc<dyn TerminalHost>>,
    pub runtimeStorageHost: Option<Arc<dyn RuntimeStorageHost>>,
    pub runtimeSqliteHost: Option<Arc<dyn RuntimeSqliteHost>>,
    pub hostEnvironment: HostEnvironmentDescriptor,
    pub coreCommandExecutor: Option<CoreCommandExecutor>,
}

impl OperitApplicationContext {
    pub fn new() -> Self {
        Self {
            fileSystemHost: None,
            webVisitHost: None,
            browserAutomationHost: None,
            httpHost: None,
            systemOperationHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostEnvironment: HostEnvironmentDescriptor::android(),
            coreCommandExecutor: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn withFileSystemHost(host: Arc<dyn FileSystemHost>) -> Self {
        let hostEnvironment = host.environmentDescriptor();
        Self {
            fileSystemHost: Some(host),
            webVisitHost: None,
            browserAutomationHost: None,
            httpHost: None,
            systemOperationHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn withFileSystemAndWebVisitHosts(
        fileSystemHost: Arc<dyn FileSystemHost>,
        webVisitHost: Arc<dyn WebVisitHost>,
    ) -> Self {
        let hostEnvironment = fileSystemHost.environmentDescriptor();
        Self {
            fileSystemHost: Some(fileSystemHost),
            webVisitHost: Some(webVisitHost),
            browserAutomationHost: None,
            httpHost: None,
            systemOperationHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn withFileSystemWebVisitAndSystemOperationHosts(
        fileSystemHost: Arc<dyn FileSystemHost>,
        webVisitHost: Arc<dyn WebVisitHost>,
        systemOperationHost: Arc<dyn SystemOperationHost>,
    ) -> Self {
        let hostEnvironment = fileSystemHost.environmentDescriptor();
        Self {
            fileSystemHost: Some(fileSystemHost),
            webVisitHost: Some(webVisitHost),
            browserAutomationHost: None,
            httpHost: None,
            systemOperationHost: Some(systemOperationHost),
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        fileSystemHost: Arc<dyn FileSystemHost>,
        webVisitHost: Arc<dyn WebVisitHost>,
        httpHost: Arc<dyn HttpHost>,
        systemOperationHost: Arc<dyn SystemOperationHost>,
        managedRuntimeHost: Arc<dyn ManagedRuntimeHost>,
        runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
        runtimeSqliteHost: Arc<dyn RuntimeSqliteHost>,
    ) -> Self {
        let hostEnvironment = fileSystemHost.environmentDescriptor();
        Self {
            fileSystemHost: Some(fileSystemHost),
            webVisitHost: Some(webVisitHost),
            browserAutomationHost: None,
            httpHost: Some(httpHost),
            systemOperationHost: Some(systemOperationHost),
            managedRuntimeHost: Some(managedRuntimeHost),
            terminalHost: None,
            runtimeStorageHost: Some(runtimeStorageHost),
            runtimeSqliteHost: Some(runtimeSqliteHost),
            hostEnvironment,
            coreCommandExecutor: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn withCoreCommandExecutor(mut self, executor: CoreCommandExecutor) -> Self {
        self.coreCommandExecutor = Some(executor);
        self
    }

    #[allow(non_snake_case)]
    pub fn withTerminalHost(mut self, terminalHost: Arc<dyn TerminalHost>) -> Self {
        self.terminalHost = Some(terminalHost);
        self
    }

    #[allow(non_snake_case)]
    pub fn withBrowserAutomationHost(
        mut self,
        browserAutomationHost: Arc<dyn BrowserAutomationHost>,
    ) -> Self {
        self.browserAutomationHost = Some(browserAutomationHost);
        self
    }
}
