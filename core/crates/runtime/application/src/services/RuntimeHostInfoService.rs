#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    HostCapability, HostIsolation, HostOnboardingRequirement, HostPlatform, HostPrivilege,
    WorkspaceRootDescriptor,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHostDescriptor {
    pub id: String,
    pub displayName: String,
    pub platform: HostPlatform,
    pub privilege: HostPrivilege,
    pub isolation: HostIsolation,
    pub pathStyleDescriptionEn: String,
    pub pathStyleDescriptionCn: String,
    pub examplePaths: Vec<String>,
    pub usesEnvironmentParameter: bool,
    pub environmentParameterDescriptionEn: String,
    pub environmentParameterDescriptionCn: String,
    pub capabilities: Vec<String>,
    pub structuredCapabilities: Vec<HostCapability>,
    pub onboardingRequirements: Vec<HostOnboardingRequirement>,
    pub workspaceRoots: Vec<WorkspaceRootDescriptor>,
    pub fileSystemHost: bool,
    pub webVisitHost: bool,
    pub systemOperationHost: bool,
    pub audioPlaybackHost: bool,
    pub ttsSynthesisHost: bool,
    pub ttsPlaybackHost: bool,
    pub systemTtsPlaybackHost: bool,
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
    /// Captures a host descriptor snapshot from the current host manager.
    pub fn getInstance(context: &HostManager) -> Self {
        let host = &context.hostEnvironment;
        Self {
            descriptor: RuntimeHostDescriptor {
                id: host.id.clone(),
                displayName: host.displayName.clone(),
                platform: host.platform.clone(),
                privilege: host.privilege.clone(),
                isolation: host.isolation.clone(),
                pathStyleDescriptionEn: host.pathStyleDescriptionEn.clone(),
                pathStyleDescriptionCn: host.pathStyleDescriptionCn.clone(),
                examplePaths: host.examplePaths.clone(),
                usesEnvironmentParameter: host.usesEnvironmentParameter,
                environmentParameterDescriptionEn: host.environmentParameterDescriptionEn.clone(),
                environmentParameterDescriptionCn: host.environmentParameterDescriptionCn.clone(),
                capabilities: host.capabilities.clone(),
                structuredCapabilities: host.structuredCapabilities.clone(),
                onboardingRequirements: host.onboardingRequirements.clone(),
                workspaceRoots: host.workspaceRoots.clone(),
                fileSystemHost: context.fileSystemHost.is_some(),
                webVisitHost: context.webVisitHost.is_some(),
                systemOperationHost: context.systemOperationHost.is_some(),
                audioPlaybackHost: context.audioPlaybackHost.is_some(),
                ttsSynthesisHost: context.ttsSynthesisHost.is_some(),
                ttsPlaybackHost: context.ttsPlaybackHost.is_some(),
                systemTtsPlaybackHost: context
                    .ttsPlaybackHost
                    .as_ref()
                    .is_some_and(|host| host.supportsSystemSpeech()),
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

    /// Returns the captured runtime host descriptor.
    pub fn runtimeHostDescriptor(&self) -> RuntimeHostDescriptor {
        self.descriptor.clone()
    }
}
