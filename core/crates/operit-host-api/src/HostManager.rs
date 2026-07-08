use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use crate::{
    AudioPlaybackHost, BluetoothHost, BrowserAutomationHost, ComposeDslWebViewHost, FileSystemHost,
    HostEnvironmentDescriptor, HostRuntimeEventHost, HttpHost, ManagedRuntimeHost,
    RuntimeSqliteHost, RuntimeStorageHost, SystemOperationHost, TerminalHost, TtsPlaybackHost,
    TtsSynthesisHost, WebVisitHost,
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
    pub composeDslWebViewHost: Option<Arc<dyn ComposeDslWebViewHost>>,
    pub httpHost: Option<Arc<dyn HttpHost>>,
    pub systemOperationHost: Option<Arc<dyn SystemOperationHost>>,
    pub audioPlaybackHost: Option<Arc<dyn AudioPlaybackHost>>,
    pub bluetoothHost: Option<Arc<dyn BluetoothHost>>,
    pub ttsSynthesisHost: Option<Arc<dyn TtsSynthesisHost>>,
    pub ttsPlaybackHost: Option<Arc<dyn TtsPlaybackHost>>,
    pub managedRuntimeHost: Option<Arc<dyn ManagedRuntimeHost>>,
    pub terminalHost: Option<Arc<dyn TerminalHost>>,
    pub runtimeStorageHost: Option<Arc<dyn RuntimeStorageHost>>,
    pub runtimeSqliteHost: Option<Arc<dyn RuntimeSqliteHost>>,
    pub hostRuntimeEventHost: Option<Arc<dyn HostRuntimeEventHost>>,
    pub hostEnvironment: HostEnvironmentDescriptor,
    pub coreCommandExecutor: Option<CoreCommandExecutor>,
    pub appFilesRoot: Option<PathBuf>,
}

impl HostManager {
    /// Creates a context with no host integrations and the default Android descriptor.
    pub fn new() -> Self {
        Self {
            fileSystemHost: None,
            webVisitHost: None,
            browserAutomationHost: None,
            composeDslWebViewHost: None,
            httpHost: None,
            systemOperationHost: None,
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostRuntimeEventHost: None,
            hostEnvironment: HostEnvironmentDescriptor::android(),
            coreCommandExecutor: None,
            appFilesRoot: None,
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
            composeDslWebViewHost: None,
            httpHost: None,
            systemOperationHost: None,
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostRuntimeEventHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
            appFilesRoot: None,
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
            composeDslWebViewHost: None,
            httpHost: None,
            systemOperationHost: None,
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostRuntimeEventHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
            appFilesRoot: None,
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
            composeDslWebViewHost: None,
            httpHost: None,
            systemOperationHost: Some(systemOperationHost),
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            managedRuntimeHost: None,
            terminalHost: None,
            runtimeStorageHost: None,
            runtimeSqliteHost: None,
            hostRuntimeEventHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
            appFilesRoot: None,
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
            composeDslWebViewHost: None,
            httpHost: Some(httpHost),
            systemOperationHost: Some(systemOperationHost),
            audioPlaybackHost: None,
            bluetoothHost: None,
            ttsSynthesisHost: None,
            ttsPlaybackHost: None,
            managedRuntimeHost: Some(managedRuntimeHost),
            terminalHost: None,
            runtimeStorageHost: Some(runtimeStorageHost),
            runtimeSqliteHost: Some(runtimeSqliteHost),
            hostRuntimeEventHost: None,
            hostEnvironment,
            coreCommandExecutor: None,
            appFilesRoot: None,
        }
    }

    /// Adds a command executor for host-dispatched core commands.
    #[allow(non_snake_case)]
    pub fn withCoreCommandExecutor(mut self, executor: CoreCommandExecutor) -> Self {
        self.coreCommandExecutor = Some(executor);
        self
    }

    /// Sets the application files root used by host-backed storage paths.
    #[allow(non_snake_case)]
    pub fn withAppFilesRoot(mut self, appFilesRoot: PathBuf) -> Self {
        self.appFilesRoot = Some(appFilesRoot);
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

    /// Adds a browser automation host for web interaction tools.
    #[allow(non_snake_case)]
    pub fn withBrowserAutomationHost(
        mut self,
        browserAutomationHost: Arc<dyn BrowserAutomationHost>,
    ) -> Self {
        self.browserAutomationHost = Some(browserAutomationHost);
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
