use std::sync::{Arc, OnceLock};

use crate::{
    AudioPlaybackHost, BluetoothHost, BrowserAutomationHost, BrowserSessionHost,
    ComposeDslWebViewHost, FileSystemHost, HostEnvironmentDescriptor, HostRuntimeEventHost,
    HostSecretStore, HttpHost, LocalInferenceHost, ManagedRuntimeHost, RuntimeSqliteHost,
    RuntimeStorageHost, SystemOperationHost, TerminalHost, TtsPlaybackHost, TtsSynthesisHost,
    WebVisitHost,
};

static DEFAULT_HTTP_HOST: OnceLock<Arc<dyn HttpHost>> = OnceLock::new();

/// Command callback used by hosts that expose core operations as argv-style calls.
pub type CoreCommandExecutor = Arc<dyn Fn(Vec<String>) -> Result<String, String> + Send + Sync>;

/// Registers the HTTP host shared by services that are not passed an explicit context.
#[allow(non_snake_case)]
pub fn setDefaultHttpHost(host: Arc<dyn HttpHost>) {
    let _ = DEFAULT_HTTP_HOST.set(host);
}

/// Returns the globally registered HTTP host for runtime network services.
#[allow(non_snake_case)]
pub fn defaultHttpHost() -> Arc<dyn HttpHost> {
    DEFAULT_HTTP_HOST
        .get()
        .expect("HTTP host must be configured before using HTTP-backed runtime services")
        .clone()
}

/// Bundles host-provided capabilities that the runtime can call through stable traits.
#[derive(Clone, Default)]
pub struct HostManager {
    pub fileSystemHost: Option<Arc<dyn FileSystemHost>>,
    pub webVisitHost: Option<Arc<dyn WebVisitHost>>,
    pub browserAutomationHost: Option<Arc<dyn BrowserAutomationHost>>,
    pub browserSessionHost: Option<Arc<dyn BrowserSessionHost>>,
    pub composeDslWebViewHost: Option<Arc<dyn ComposeDslWebViewHost>>,
    pub httpHost: Option<Arc<dyn HttpHost>>,
    pub systemOperationHost: Option<Arc<dyn SystemOperationHost>>,
    pub audioPlaybackHost: Option<Arc<dyn AudioPlaybackHost>>,
    pub bluetoothHost: Option<Arc<dyn BluetoothHost>>,
    pub ttsSynthesisHost: Option<Arc<dyn TtsSynthesisHost>>,
    pub ttsPlaybackHost: Option<Arc<dyn TtsPlaybackHost>>,
    pub localInferenceHost: Option<Arc<dyn LocalInferenceHost>>,
    pub managedRuntimeHost: Option<Arc<dyn ManagedRuntimeHost>>,
    pub terminalHost: Option<Arc<dyn TerminalHost>>,
    pub runtimeStorageHost: Option<Arc<dyn RuntimeStorageHost>>,
    pub runtimeSqliteHost: Option<Arc<dyn RuntimeSqliteHost>>,
    pub hostSecretStore: Option<Arc<dyn HostSecretStore>>,
    pub hostRuntimeEventHost: Option<Arc<dyn HostRuntimeEventHost>>,
    pub hostEnvironment: HostEnvironmentDescriptor,
    pub coreCommandExecutor: Option<CoreCommandExecutor>,
}

impl HostManager {
    /// Creates a context with no host integrations and the default Android descriptor.
    pub fn new() -> Self {
        Self {
            fileSystemHost: None,
            webVisitHost: None,
            browserAutomationHost: None,
            browserSessionHost: None,
            composeDslWebViewHost: None,
            httpHost: None,
            systemOperationHost: None,
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            localInferenceHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostSecretStore: None,
            hostRuntimeEventHost: None,
            hostEnvironment: HostEnvironmentDescriptor::android(),
            coreCommandExecutor: None,
        }
    }

    /// Creates a context backed by a file-system host and its environment descriptor.
    #[allow(non_snake_case)]
    pub fn withFileSystemHost(host: Arc<dyn FileSystemHost>) -> Self {
        let hostEnvironment = host.environmentDescriptor();
        Self {
            fileSystemHost: Some(host),
            webVisitHost: None,
            browserAutomationHost: None,
            browserSessionHost: None,
            composeDslWebViewHost: None,
            httpHost: None,
            systemOperationHost: None,
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            localInferenceHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostSecretStore: None,
            hostRuntimeEventHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
        }
    }

    /// Creates a context with file-system and web-visit hosts.
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
            browserSessionHost: None,
            composeDslWebViewHost: None,
            httpHost: None,
            systemOperationHost: None,
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            localInferenceHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostSecretStore: None,
            hostRuntimeEventHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
        }
    }

    /// Creates a context with file, web, and system-operation hosts.
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
            browserSessionHost: None,
            composeDslWebViewHost: None,
            httpHost: None,
            systemOperationHost: Some(systemOperationHost),
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            localInferenceHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostSecretStore: None,
            hostRuntimeEventHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
        }
    }

    /// Creates the standard full runtime context used by managed desktop and mobile hosts.
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
            browserSessionHost: None,
            composeDslWebViewHost: None,
            httpHost: Some(httpHost),
            systemOperationHost: Some(systemOperationHost),
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            localInferenceHost: None,
            managedRuntimeHost: Some(managedRuntimeHost),
            terminalHost: None,
            runtimeStorageHost: Some(runtimeStorageHost),
            runtimeSqliteHost: Some(runtimeSqliteHost),
            hostSecretStore: None,
            hostRuntimeEventHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
        }
    }

    /// Adds a command executor for host-dispatched core commands.
    #[allow(non_snake_case)]
    pub fn withCoreCommandExecutor(mut self, executor: CoreCommandExecutor) -> Self {
        self.coreCommandExecutor = Some(executor);
        self
    }

    /// Adds host-owned secret storage for runtime encryption keys.
    #[allow(non_snake_case)]
    pub fn withHostSecretStore(mut self, hostSecretStore: Arc<dyn HostSecretStore>) -> Self {
        self.hostSecretStore = Some(hostSecretStore);
        self
    }

    /// Adds a terminal host for shell command execution.
    #[allow(non_snake_case)]
    pub fn withTerminalHost(mut self, terminalHost: Arc<dyn TerminalHost>) -> Self {
        self.terminalHost = Some(terminalHost);
        self
    }

    /// Adds an audio playback host for generated or captured media output.
    #[allow(non_snake_case)]
    pub fn withAudioPlaybackHost(mut self, audioPlaybackHost: Arc<dyn AudioPlaybackHost>) -> Self {
        self.audioPlaybackHost = Some(audioPlaybackHost);
        self
    }

    /// Adds a Bluetooth host for nearby-device operations.
    #[allow(non_snake_case)]
    pub fn withBluetoothHost(mut self, bluetoothHost: Arc<dyn BluetoothHost>) -> Self {
        self.bluetoothHost = Some(bluetoothHost);
        self
    }

    /// Adds a host-backed text-to-speech synthesis service.
    #[allow(non_snake_case)]
    pub fn withTtsSynthesisHost(mut self, ttsSynthesisHost: Arc<dyn TtsSynthesisHost>) -> Self {
        self.ttsSynthesisHost = Some(ttsSynthesisHost);
        self
    }

    /// Adds a host-backed text-to-speech playback service.
    #[allow(non_snake_case)]
    pub fn withTtsPlaybackHost(mut self, ttsPlaybackHost: Arc<dyn TtsPlaybackHost>) -> Self {
        self.ttsPlaybackHost = Some(ttsPlaybackHost);
        self
    }

    /// Adds a platform host for local model inference.
    #[allow(non_snake_case)]
    pub fn withLocalInferenceHost(
        mut self,
        localInferenceHost: Arc<dyn LocalInferenceHost>,
    ) -> Self {
        self.localInferenceHost = Some(localInferenceHost);
        self
    }

    /// Adds a browser automation host for web interaction tools.
    #[allow(non_snake_case)]
    pub fn withBrowserAutomationHost(
        mut self,
        browserAutomationHost: Arc<dyn BrowserAutomationHost>,
    ) -> Self {
        self.browserAutomationHost = Some(browserAutomationHost);
        self
    }

    /// Adds a browser session host for interactive browser sessions.
    #[allow(non_snake_case)]
    pub fn withBrowserSessionHost(
        mut self,
        browserSessionHost: Arc<dyn BrowserSessionHost>,
    ) -> Self {
        self.browserSessionHost = Some(browserSessionHost);
        self
    }

    /// Adds a Compose DSL WebView host for rendering tool-provided UI modules.
    #[allow(non_snake_case)]
    pub fn withComposeDslWebViewHost(
        mut self,
        composeDslWebViewHost: Arc<dyn ComposeDslWebViewHost>,
    ) -> Self {
        self.composeDslWebViewHost = Some(composeDslWebViewHost);
        self
    }

    /// Adds a host runtime-event bridge for inbound external events.
    #[allow(non_snake_case)]
    pub fn withHostRuntimeEventHost(
        mut self,
        hostRuntimeEventHost: Arc<dyn HostRuntimeEventHost>,
    ) -> Self {
        self.hostRuntimeEventHost = Some(hostRuntimeEventHost);
        self
    }
}
