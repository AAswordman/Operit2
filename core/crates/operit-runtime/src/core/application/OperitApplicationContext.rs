use std::sync::Arc;

use operit_host_api::{FileSystemHost, HostEnvironmentDescriptor};

#[derive(Clone, Default)]
pub struct OperitApplicationContext {
    pub fileSystemHost: Option<Arc<dyn FileSystemHost>>,
    pub hostEnvironment: HostEnvironmentDescriptor,
}

impl OperitApplicationContext {
    pub fn new() -> Self {
        Self {
            fileSystemHost: None,
            hostEnvironment: HostEnvironmentDescriptor::android(),
        }
    }

    #[allow(non_snake_case)]
    pub fn withFileSystemHost(host: Arc<dyn FileSystemHost>) -> Self {
        let hostEnvironment = host.environmentDescriptor();
        Self {
            fileSystemHost: Some(host),
            hostEnvironment,
        }
    }
}
