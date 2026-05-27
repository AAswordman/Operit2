use std::sync::{Arc, OnceLock};

use operit_host_api::{
    FileSystemHost, HostEnvironmentDescriptor, HttpHost, ManagedRuntimeHost, RuntimeSqliteHost,
    RuntimeStorageHost, SystemOperationHost, WebVisitHost,
};

static DEFAULT_HTTP_HOST: OnceLock<Arc<dyn HttpHost>> = OnceLock::new();

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
    pub httpHost: Option<Arc<dyn HttpHost>>,
    pub systemOperationHost: Option<Arc<dyn SystemOperationHost>>,
    pub managedRuntimeHost: Option<Arc<dyn ManagedRuntimeHost>>,
    pub runtimeStorageHost: Option<Arc<dyn RuntimeStorageHost>>,
    pub runtimeSqliteHost: Option<Arc<dyn RuntimeSqliteHost>>,
    pub hostEnvironment: HostEnvironmentDescriptor,
}

impl OperitApplicationContext {
    pub fn new() -> Self {
        Self {
            fileSystemHost: None,
            webVisitHost: None,
            httpHost: None,
            systemOperationHost: None,
            managedRuntimeHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostEnvironment: HostEnvironmentDescriptor::android(),
        }
    }

    #[allow(non_snake_case)]
    pub fn withFileSystemHost(host: Arc<dyn FileSystemHost>) -> Self {
        let hostEnvironment = host.environmentDescriptor();
        Self {
            fileSystemHost: Some(host),
            webVisitHost: None,
            httpHost: None,
            systemOperationHost: None,
            managedRuntimeHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostEnvironment,
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
            httpHost: None,
            systemOperationHost: None,
            managedRuntimeHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostEnvironment,
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
            httpHost: None,
            systemOperationHost: Some(systemOperationHost),
            managedRuntimeHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostEnvironment,
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
            httpHost: Some(httpHost),
            systemOperationHost: Some(systemOperationHost),
            managedRuntimeHost: Some(managedRuntimeHost),
            runtimeStorageHost: Some(runtimeStorageHost),
            runtimeSqliteHost: Some(runtimeSqliteHost),
            hostEnvironment,
        }
    }
}
