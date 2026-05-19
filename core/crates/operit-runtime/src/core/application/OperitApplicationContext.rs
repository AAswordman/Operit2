use std::sync::Arc;

use operit_host_api::FileSystemHost;

#[derive(Clone, Default)]
pub struct OperitApplicationContext {
    pub fileSystemHost: Option<Arc<dyn FileSystemHost>>,
}

impl OperitApplicationContext {
    pub fn new() -> Self {
        Self {
            fileSystemHost: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn withFileSystemHost(host: Arc<dyn FileSystemHost>) -> Self {
        Self {
            fileSystemHost: Some(host),
        }
    }
}
