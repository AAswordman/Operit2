#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

use crate::core::application::OperitApplicationContext::OperitApplicationContext;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHostDescriptor {
    pub id: String,
    pub displayName: String,
    pub pathStyleDescriptionEn: String,
    pub pathStyleDescriptionCn: String,
    pub examplePaths: Vec<String>,
    pub usesEnvironmentParameter: bool,
    pub environmentParameterDescriptionEn: String,
    pub environmentParameterDescriptionCn: String,
    pub capabilities: Vec<String>,
    pub fileSystemHost: bool,
    pub webVisitHost: bool,
    pub systemOperationHost: bool,
    pub audioPlaybackHost: bool,
    pub ttsSynthesisHost: bool,
    pub managedRuntimeHost: bool,
    pub runtimeStorageHost: bool,
    pub runtimeSqliteHost: bool,
    pub browserAutomationHost: bool,
    pub composeDslWebViewHost: bool,
    pub terminalHost: bool,
    pub hostRuntimeEventHost: bool,
}

pub struct RuntimeHostInfoService {
    descriptor: RuntimeHostDescriptor,
}

impl RuntimeHostInfoService {
    pub fn getInstance(context: &OperitApplicationContext) -> Self {
        let host = &context.hostEnvironment;
        Self {
            descriptor: RuntimeHostDescriptor {
                id: host.id.clone(),
                displayName: host.displayName.clone(),
                pathStyleDescriptionEn: host.pathStyleDescriptionEn.clone(),
                pathStyleDescriptionCn: host.pathStyleDescriptionCn.clone(),
                examplePaths: host.examplePaths.clone(),
                usesEnvironmentParameter: host.usesEnvironmentParameter,
                environmentParameterDescriptionEn: host.environmentParameterDescriptionEn.clone(),
                environmentParameterDescriptionCn: host.environmentParameterDescriptionCn.clone(),
                capabilities: host.capabilities.clone(),
                fileSystemHost: context.fileSystemHost.is_some(),
                webVisitHost: context.webVisitHost.is_some(),
                systemOperationHost: context.systemOperationHost.is_some(),
                audioPlaybackHost: context.audioPlaybackHost.is_some(),
                ttsSynthesisHost: context.ttsSynthesisHost.is_some(),
                managedRuntimeHost: context.managedRuntimeHost.is_some(),
                runtimeStorageHost: context.runtimeStorageHost.is_some(),
                runtimeSqliteHost: context.runtimeSqliteHost.is_some(),
                browserAutomationHost: context.browserAutomationHost.is_some(),
                composeDslWebViewHost: context.composeDslWebViewHost.is_some(),
                terminalHost: context.terminalHost.is_some(),
                hostRuntimeEventHost: context.hostRuntimeEventHost.is_some(),
            },
        }
    }

    pub fn runtimeHostDescriptor(&self) -> RuntimeHostDescriptor {
        self.descriptor.clone()
    }
}
