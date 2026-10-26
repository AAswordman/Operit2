use std::sync::{Mutex, OnceLock};

use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};
use serde::{Deserialize, Serialize};

pub const PLUGIN_LOAD_STATUS_WAITING: &str = "waiting";
pub const PLUGIN_LOAD_STATUS_LOADING: &str = "loading";
pub const PLUGIN_LOAD_STATUS_SUCCESS: &str = "success";
pub const PLUGIN_LOAD_STATUS_FAILED: &str = "failed";

pub const PLUGIN_LOAD_KIND_PACKAGE: &str = "package";
pub const PLUGIN_LOAD_KIND_PLUGIN: &str = "plugin";
pub const PLUGIN_LOAD_KIND_MCP: &str = "mcp";

#[derive(Default)]
struct PluginLoadingSessionState {
    generation: u64,
    active: bool,
}

static PLUGIN_LOADING_SESSION_STATE: OnceLock<Mutex<PluginLoadingSessionState>> = OnceLock::new();

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

/// Returns the synchronized state for the current plugin loading session.
fn pluginLoadingSessionState() -> &'static Mutex<PluginLoadingSessionState> {
    PLUGIN_LOADING_SESSION_STATE.get_or_init(|| Mutex::new(PluginLoadingSessionState::default()))
}

/// Returns the next unique generation for a plugin loading session.
fn nextPluginLoadingGeneration(current: u64) -> u64 {
    current
        .checked_add(1)
        .expect("plugin loading session generation exhausted")
}

/// Returns whether a delayed task still owns a completed loading session.
fn canHideCompletedPluginLoadingSession(
    session: &PluginLoadingSessionState,
    progress: &PluginLoadingProgress,
    generation: u64,
) -> bool {
    !session.active
        && session.generation == generation
        && progress.visible
        && (progress.phase == "complete_success" || progress.phase == "complete_with_failures")
}

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
    pluginLoadingSessionState()
        .lock()
        .expect("plugin loading session mutex poisoned")
        .active
}

/// Starts a visible loading session and returns its generation token.
pub fn showPluginLoading() -> u64 {
    let mut session = pluginLoadingSessionState()
        .lock()
        .expect("plugin loading session mutex poisoned");
    session.generation = nextPluginLoadingGeneration(session.generation);
    session.active = true;
    let generation = session.generation;
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
    generation
}

/// Hides the overlay without stopping the in-flight load session.
pub fn skipPluginLoading() {
    let _session = pluginLoadingSessionState()
        .lock()
        .expect("plugin loading session mutex poisoned");
    updatePluginLoadingProgress(|current| {
        current.visible = false;
        current.forceExpanded = false;
    });
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
        if current.plugins.iter().any(|item| item.id == id) {
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

/// Corrects the category of an overlay row after its source has been identified.
pub fn setPluginLoadingItemKind(id: &str, kind: &str) {
    updatePluginLoadingProgress(|current| {
        if let Some(item) = current.plugins.iter_mut().find(|item| item.id == id) {
            item.kind = kind.to_string();
        }
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
            item.message = message
                .lines()
                .next()
                .unwrap_or(message)
                .chars()
                .take(160)
                .collect();
        }
    });
}

/// Finishes the matching load session and schedules its overlay hide.
pub fn completePluginLoadingSession(generation: u64) {
    let mut session = pluginLoadingSessionState()
        .lock()
        .expect("plugin loading session mutex poisoned");
    if !session.active || session.generation != generation {
        return;
    }
    session.active = false;
    let current = pluginLoadingProgressFlow().value();
    if !current.visible {
        return;
    }
    let hasFailures = current
        .plugins
        .iter()
        .any(|item| item.status == PLUGIN_LOAD_STATUS_FAILED);
    if current.plugins.is_empty() {
        updatePluginLoadingProgress(|progress| {
            progress.visible = false;
            progress.forceExpanded = false;
        });
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
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleDelayedHostRuntimeTask(
            "plugin-loading-overlay-hide",
            120,
            Box::new(move || hidePluginLoadingIfSessionComplete(generation)),
        )
        .expect("plugin loading overlay hide task must be scheduled");
}

/// Hides a completed session only while its generation still owns the overlay.
fn hidePluginLoadingIfSessionComplete(generation: u64) {
    let session = pluginLoadingSessionState()
        .lock()
        .expect("plugin loading session mutex poisoned");
    let current = pluginLoadingProgressFlow().value();
    if !canHideCompletedPluginLoadingSession(&session, &current, generation) {
        return;
    }
    updatePluginLoadingProgress(|progress| {
        progress.visible = false;
        progress.forceExpanded = false;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a progress snapshot for delayed-hide ownership assertions.
    fn progressSnapshot(phase: &str) -> PluginLoadingProgress {
        PluginLoadingProgress {
            visible: true,
            forceExpanded: false,
            progress: 1.0,
            phase: phase.to_string(),
            currentTask: String::new(),
            pluginsStarted: 1,
            pluginsTotal: 1,
            plugins: Vec::new(),
        }
    }

    /// Verifies that a newer active session blocks an older hide task.
    #[test]
    fn delayedHideRejectsNewActiveSession() {
        let session = PluginLoadingSessionState {
            generation: 2,
            active: true,
        };
        let progress = progressSnapshot("complete_success");
        assert!(!canHideCompletedPluginLoadingSession(
            &session, &progress, 1
        ));
    }

    /// Verifies that a completed generation can hide its own overlay.
    #[test]
    fn delayedHideAcceptsCompletedGeneration() {
        let session = PluginLoadingSessionState {
            generation: 2,
            active: false,
        };
        let progress = progressSnapshot("complete_with_failures");
        assert!(canHideCompletedPluginLoadingSession(&session, &progress, 2));
    }
}
