use std::sync::Arc;

use operit_host_api::{
    FileSystemHost, HostEnvironmentDescriptor, ManagedRuntimeHost, SystemOperationHost,
    WebVisitHost,
};

#[derive(Clone, Default)]
pub struct OperitApplicationContext {
    pub fileSystemHost: Option<Arc<dyn FileSystemHost>>,
    pub webVisitHost: Option<Arc<dyn WebVisitHost>>,
    pub systemOperationHost: Option<Arc<dyn SystemOperationHost>>,
    pub managedRuntimeHost: Option<Arc<dyn ManagedRuntimeHost>>,
    pub hostEnvironment: HostEnvironmentDescriptor,
}

impl OperitApplicationContext {
    pub fn new() -> Self {
        Self {
            fileSystemHost: None,
            webVisitHost: None,
            systemOperationHost: None,
            managedRuntimeHost: None,
            hostEnvironment: HostEnvironmentDescriptor::android(),
        }
    }

    #[allow(non_snake_case)]
    pub fn withFileSystemHost(host: Arc<dyn FileSystemHost>) -> Self {
        let hostEnvironment = host.environmentDescriptor();
        Self {
            fileSystemHost: Some(host),
            webVisitHost: None,
            systemOperationHost: None,
            managedRuntimeHost: None,
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
            systemOperationHost: None,
            managedRuntimeHost: None,
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
            systemOperationHost: Some(systemOperationHost),
            managedRuntimeHost: None,
            hostEnvironment,
        }
    }

    #[allow(non_snake_case)]
    pub fn withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        fileSystemHost: Arc<dyn FileSystemHost>,
        webVisitHost: Arc<dyn WebVisitHost>,
        systemOperationHost: Arc<dyn SystemOperationHost>,
        managedRuntimeHost: Arc<dyn ManagedRuntimeHost>,
    ) -> Self {
        let hostEnvironment = fileSystemHost.environmentDescriptor();
        Self {
            fileSystemHost: Some(fileSystemHost),
            webVisitHost: Some(webVisitHost),
            systemOperationHost: Some(systemOperationHost),
            managedRuntimeHost: Some(managedRuntimeHost),
            hostEnvironment,
        }
    }
}
