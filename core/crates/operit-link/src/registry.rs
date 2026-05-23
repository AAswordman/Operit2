use std::collections::BTreeMap;
use std::sync::Arc;

use operit_runtime::core::application::OperitApplication::OperitApplication;
use serde_json::Value;

use crate::protocol::{CoreCallRequest, CoreEvent, CoreLinkError, CoreWatchRequest};

type CoreMethodHandler =
    Arc<dyn Fn(&mut OperitApplication, CoreCallRequest) -> Result<Value, CoreLinkError> + Send + Sync>;
type CoreWatchHandler =
    Arc<dyn Fn(&mut OperitApplication, CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> + Send + Sync>;

#[derive(Clone, Default)]
pub struct CoreMethodRegistry {
    handlers: BTreeMap<String, CoreMethodHandler>,
}

impl CoreMethodRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        targetPath: impl AsRef<str>,
        methodName: impl AsRef<str>,
        handler: impl Fn(&mut OperitApplication, CoreCallRequest) -> Result<Value, CoreLinkError>
            + Send
            + Sync
            + 'static,
    ) {
        self.handlers.insert(
            Self::key(targetPath.as_ref(), methodName.as_ref()),
            Arc::new(handler),
        );
    }

    pub fn dispatch(
        &self,
        application: &mut OperitApplication,
        request: CoreCallRequest,
    ) -> Result<Value, CoreLinkError> {
        let key = request.registryKey();
        let Some(handler) = self.handlers.get(&key) else {
            return Err(CoreLinkError::methodNotFound(&key));
        };
        handler(application, request)
    }

    pub fn methodKeys(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    fn key(targetPath: &str, methodName: &str) -> String {
        format!("{targetPath}::{methodName}")
    }
}

#[derive(Clone, Default)]
pub struct CoreWatchRegistry {
    handlers: BTreeMap<String, CoreWatchHandler>,
}

impl CoreWatchRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        targetPath: impl AsRef<str>,
        propertyName: impl AsRef<str>,
        handler: impl Fn(&mut OperitApplication, CoreWatchRequest) -> Result<CoreEvent, CoreLinkError>
            + Send
            + Sync
            + 'static,
    ) {
        self.handlers.insert(
            Self::key(targetPath.as_ref(), propertyName.as_ref()),
            Arc::new(handler),
        );
    }

    pub fn snapshot(
        &self,
        application: &mut OperitApplication,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let key = request.registryKey();
        let Some(handler) = self.handlers.get(&key) else {
            return Err(CoreLinkError::watchNotFound(&key));
        };
        handler(application, request)
    }

    pub fn watchKeys(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    fn key(targetPath: &str, propertyName: &str) -> String {
        format!("{targetPath}::{propertyName}")
    }
}
