use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};
use serde::{Deserialize, Serialize};

const PLUGIN_LOADING_COMPLETE_PREVIEW_DELAY_MS: u64 = 120;

pub const PLUGIN_LOAD_STATUS_WAITING: &str = "waiting";
pub const PLUGIN_LOAD_STATUS_LOADING: &str = "loading";
pub const PLUGIN_LOAD_STATUS_SUCCESS: &str = "success";
pub const PLUGIN_LOAD_STATUS_FAILED: &str = "failed";

pub const PLUGIN_LOAD_KIND_PACKAGE: &str = "package";
pub const PLUGIN_LOAD_KIND_MCP: &str = "mcp";

static PLUGIN_LOADING_SESSION_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
/// One package or plugin row in the startup loading overlay.
pub struct PluginLoadingItem {
    pub id: String,
    pub displayName: String,
    pub kind: String,
    pub status: String,
    pub message: String,
    pub logText: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
/// Published runtime package and plugin loading overlay state.
pub struct PluginLoadingProgress {
    pub visible: bool,
    pub forceExpanded: bool,
    pub progress: f32,
    pub phase: String,
    pub currentTask: String,
    pub pluginsStarted: i32,
    pub pluginsTotal: i32,
    pub plugins: Vec<PluginLoadingItem>,
}

impl PluginLoadingProgress {
    /// Returns the idle overlay state used when no load session is running.
    fn idle() -> Self {
        Self {
            visible: false,
            forceExpanded: false,
            progress: 0.0,
            phase: "idle".to_string(),
            currentTask: String::new(),
            pluginsStarted: 0,
            pluginsTotal: 0,
            plugins: Vec::new(),
        }
    }
}

static PLUGIN_LOADING_PROGRESS_FLOW: OnceLock<MutableStateFlow<PluginLoadingProgress>> =
    OnceLock::new();

fn pluginLoadingProgressFlow() -> &'static MutableStateFlow<PluginLoadingProgress> {
    PLUGIN_LOADING_PROGRESS_FLOW.get_or_init(|| mutableStateFlow(PluginLoadingProgress::idle()))
}

fn updatePluginLoadingProgress<F>(update: F)
where
    F: FnOnce(&mut PluginLoadingProgress),
{
    let mut current = pluginLoadingProgressFlow().value();
    update(&mut current);
    refreshDerivedCounts(&mut current);
    pluginLoadingProgressFlow().set_value(current);
}

fn refreshDerivedCounts(progress: &mut PluginLoadingProgress) {
    progress.pluginsTotal = progress.plugins.len() as i32;
    progress.pluginsStarted = progress
        .plugins
        .iter()
        .filter(|item| item.status == PLUGIN_LOAD_STATUS_SUCCESS)
        .count() as i32;
    let finished = progress
        .plugins
        .iter()
        .filter(|item| {
            item.status == PLUGIN_LOAD_STATUS_SUCCESS || item.status == PLUGIN_LOAD_STATUS_FAILED
        })
        .count();
    progress.progress = if progress.plugins.is_empty() {
        0.0
    } else {
        finished as f32 / progress.plugins.len() as f32
    };
    progress.currentTask = progress
        .plugins
        .iter()
        .find(|item| item.status == PLUGIN_LOAD_STATUS_LOADING)
        .map(|item| item.displayName.clone())
        .unwrap_or_default();
}

/// Observes runtime package and plugin loading overlay state.
pub fn observePluginLoadingProgress() -> StateFlow<PluginLoadingProgress> {
    pluginLoadingProgressFlow().asStateFlow()
}

/// Returns whether a package/plugin load session is currently running.
pub fn pluginLoadingSessionActive() -> bool {
    PLUGIN_LOADING_SESSION_ACTIVE.load(Ordering::SeqCst)
}

/// Starts a visible loading session for packages and plugins.
pub fn showPluginLoading() {
    PLUGIN_LOADING_SESSION_ACTIVE.store(true, Ordering::SeqCst);
    pluginLoadingProgressFlow().set_value(PluginLoadingProgress {
        visible: true,
        forceExpanded: false,
        progress: 0.0,
        phase: "loading".to_string(),
        currentTask: String::new(),
        pluginsStarted: 0,
        pluginsTotal: 0,
        plugins: Vec::new(),
    });
}

/// Hides the overlay without stopping the in-flight load session.
pub fn skipPluginLoading() {
    updatePluginLoadingProgress(|current| {
        current.visible = false;
        current.forceExpanded = false;
    });
}

/// Hides the overlay after a completed session unless a newer load started.
fn hidePluginLoadingIfSessionComplete() {
    if PLUGIN_LOADING_SESSION_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    let current = pluginLoadingProgressFlow().value();
    if current.visible
        && (current.phase == "complete_success" || current.phase == "complete_with_failures")
    {
        skipPluginLoading();
    }
}

/// Creates one waiting overlay row for a package or plugin.
pub fn pluginLoadingItem(id: String, displayName: String, kind: String) -> PluginLoadingItem {
    PluginLoadingItem {
        id,
        displayName,
        kind,
        status: PLUGIN_LOAD_STATUS_WAITING.to_string(),
        message: String::new(),
        logText: String::new(),
    }
}

/// Ensures one overlay row exists so later status updates can find it.
pub fn ensurePluginLoadingItem(id: &str, displayName: &str, kind: &str) {
    updatePluginLoadingProgress(|current| {
        if current
            .plugins
            .iter()
            .any(|item| item.id == id)
        {
            return;
        }
        current.plugins.push(pluginLoadingItem(
            id.to_string(),
            displayName.to_string(),
            kind.to_string(),
        ));
        current.phase = "loading".to_string();
    });
}

/// Marks one overlay row as currently loading.
pub fn markPluginLoadingItemLoading(id: &str, displayName: Option<&str>) {
    updatePluginLoadingProgress(|current| {
        if let Some(item) = current.plugins.iter_mut().find(|item| item.id == id) {
            item.status = PLUGIN_LOAD_STATUS_LOADING.to_string();
            if let Some(displayName) = displayName {
                item.displayName = displayName.to_string();
            }
            item.message = String::new();
        }
        current.phase = "loading".to_string();
    });
}

/// Marks one overlay row as loaded.
pub fn markPluginLoadingItemSuccess(id: &str, displayName: Option<&str>) {
    updatePluginLoadingProgress(|current| {
        if let Some(item) = current.plugins.iter_mut().find(|item| item.id == id) {
            item.status = PLUGIN_LOAD_STATUS_SUCCESS.to_string();
            if let Some(displayName) = displayName {
                item.displayName = displayName.to_string();
            }
            item.message = "success".to_string();
        }
    });
}

/// Marks one overlay row as failed and keeps the overlay expanded.
pub fn markPluginLoadingItemFailed(id: &str, message: &str, logText: &str) {
    updatePluginLoadingProgress(|current| {
        if let Some(item) = current.plugins.iter_mut().find(|item| item.id == id) {
            item.status = PLUGIN_LOAD_STATUS_FAILED.to_string();
            item.message = message.to_string();
            if !logText.is_empty() {
                item.logText = logText.to_string();
            }
        }
        current.forceExpanded = true;
        current.phase = "complete_with_failures".to_string();
    });
}

/// Appends one log line to an overlay row and uses it as the brief status.
pub fn appendPluginLoadingItemLog(id: &str, message: &str) {
    if message.trim().is_empty() {
        return;
    }
    updatePluginLoadingProgress(|current| {
        if let Some(item) = current.plugins.iter_mut().find(|item| item.id == id) {
            if item.logText.is_empty() {
                item.logText = message.to_string();
            } else {
                item.logText.push('\n');
                item.logText.push_str(message);
            }
            item.message = message.lines().next().unwrap_or(message).chars().take(160).collect();
        }
    });
}

/// Finishes the current load session and hides the overlay when every item succeeded.
pub fn completePluginLoadingSession() {
    PLUGIN_LOADING_SESSION_ACTIVE.store(false, Ordering::SeqCst);
    let current = pluginLoadingProgressFlow().value();
    if !current.visible {
        return;
    }
    let hasFailures = current
        .plugins
        .iter()
        .any(|item| item.status == PLUGIN_LOAD_STATUS_FAILED);
    if current.plugins.is_empty() {
        skipPluginLoading();
        return;
    }
    updatePluginLoadingProgress(|progress| {
        progress.progress = 1.0;
        progress.currentTask = String::new();
        if hasFailures {
            progress.phase = "complete_with_failures".to_string();
        } else {
            progress.phase = "complete_success".to_string();
        }
        progress.forceExpanded = false;
    });
    let scheduled = defaultHostRuntimeTaskSchedulerHost().scheduleDelayedHostRuntimeTask(
        "operit-plugin-loading-complete",
        PLUGIN_LOADING_COMPLETE_PREVIEW_DELAY_MS,
        Box::new(hidePluginLoadingIfSessionComplete),
    );
    if scheduled.is_err() {
        skipPluginLoading();
    }
}
