//! Application facade for the shared app/CLI release pipeline. Native paths stay here.
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HttpDownloadControl;
use operit_plugin_sdk::js_sdk::results::{AppOperationData, ToolResultData};
use operit_store::PreferencesDataStore::StateFlow;
use operit_tools::ToolExecutionManager::{AITool, ToolParameter};
use operit_util::GithubReleaseUtil::{
    FullUpdateProgressEvent, FullUpdateStatus, FullUpdateTarget, GithubReleaseUtil, ReleaseInfo,
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// Presentation metadata only; the native download URL/path is never sent to the UI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUpdateInfo {
    pub currentVersion: String,
    pub version: String,
    pub releasePageUrl: String,
    pub releaseNotes: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUpdateProgress {
    pub receivedBytes: u64,
    pub totalBytes: u64,
}
/// Installer feedback only; the install_app package path stays inside the core.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUpdateInstallResult {
    pub installState: String,
    pub details: String,
}
impl ApplicationUpdateInstallResult {
    fn fromOperation(data: &AppOperationData) -> Self {
        Self {
            installState: data
                .installState
                .clone()
                .expect("validated installation state"),
            details: data.details.clone(),
        }
    }
}
#[derive(Default)]
struct UpdateState {
    busy: bool,
    checkedAt: Option<u64>,
    release: Option<ReleaseInfo>,
    download: Option<HttpDownloadControl>,
    prepared: Option<PreparedUpdate>,
}
struct PreparedUpdate {
    receipt: String,
    path: PathBuf,
    installation: Option<AppOperationData>,
}
#[derive(Clone)]
pub(super) struct ApplicationUpdateManager {
    state: Arc<Mutex<UpdateState>>,
    progress: StateFlow<ApplicationUpdateProgress>,
}
struct BusyGuard(Arc<Mutex<UpdateState>>);
impl Drop for BusyGuard {
    fn drop(&mut self) {
        let mut state = self.0.lock().expect("update state");
        state.busy = false;
        state.download = None;
    }
}
impl ApplicationUpdateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(UpdateState::default())),
            progress: StateFlow::new(ApplicationUpdateProgress {
                receivedBytes: 0,
                totalBytes: 0,
            }),
        }
    }
    fn begin(&self) -> Result<BusyGuard, String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        if state.busy {
            return Err("An application update operation is already active".into());
        }
        state.busy = true;
        Ok(BusyGuard(self.state.clone()))
    }
    fn info(release: &ReleaseInfo) -> ApplicationUpdateInfo {
        ApplicationUpdateInfo {
            currentVersion: crate::CORE_VERSION.into(),
            version: release.version.clone(),
            releasePageUrl: release.releasePageUrl.clone(),
            releaseNotes: release.releaseNotes.clone(),
        }
    }
    pub(super) async fn check(&self) -> Result<Option<ApplicationUpdateInfo>, String> {
        let scheduler = defaultHostRuntimeTaskSchedulerHost();
        let now = scheduler.monotonicTimeMillis().map_err(|e| e.to_string())?;
        {
            let state = self.state.lock().map_err(|e| e.to_string())?;
            if state.busy
                || state.prepared.is_some()
                || state
                    .checkedAt
                    .is_some_and(|at| now.saturating_sub(at) < 6 * 60 * 60 * 1000)
            {
                return Ok(state.release.as_ref().map(Self::info));
            }
        }
        let _busy = self.begin()?;
        let Some(target) = FullUpdateTarget::appForCurrentHost()? else {
            return Ok(None);
        };
        let result = GithubReleaseUtil::checkForFullUpdate(crate::CORE_VERSION, target).await?;
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        state.release = match result {
            FullUpdateStatus::Available(info) => Some(info),
            FullUpdateStatus::UpToDate => None,
        };
        state.checkedAt = Some(scheduler.monotonicTimeMillis().map_err(|e| e.to_string())?);
        Ok(state.release.as_ref().map(Self::info))
    }
    pub(super) async fn download(&self, version: String) -> Result<String, String> {
        let _busy = self.begin()?;
        let (release, control) = {
            let mut state = self.state.lock().map_err(|e| e.to_string())?;
            let release = state
                .release
                .clone()
                .filter(|r| r.version == version)
                .ok_or("Update selection expired; check again")?;
            if let Some(prepared) = &state.prepared {
                if prepared.path.is_file() {
                    return Ok(prepared.receipt.clone());
                }
            }
            let control = HttpDownloadControl::new();
            state.download = Some(control.clone());
            (release, control)
        };
        self.progress.set_value(ApplicationUpdateProgress {
            receivedBytes: 0,
            totalBytes: 0,
        });
        let progress = self.progress.clone();
        // Uses the runtime owner's configured storage root, including Android's private app directory.
        let root = operit_util::RuntimeStoreRoot::default_runtime_store_root_config()
            .ok_or("Runtime storage root is unavailable")?
            .runtime_root
            .join("temp/application_updates");
        let path = GithubReleaseUtil::downloadAndPrepareFullUpdateControlled(
            release.downloadUrl,
            release.assetName,
            root,
            control,
            move |event| {
                if let FullUpdateProgressEvent::DownloadProgress {
                    readBytes,
                    totalBytes,
                    ..
                } = event
                {
                    progress.set_value(ApplicationUpdateProgress {
                        receivedBytes: readBytes,
                        totalBytes,
                    });
                }
            },
        )
        .await?;
        let receipt = uuid::Uuid::new_v4().to_string();
        self.state.lock().map_err(|e| e.to_string())?.prepared = Some(PreparedUpdate {
            receipt: receipt.clone(),
            path,
            installation: None,
        });
        Ok(receipt)
    }
    pub(super) fn cancel(&self) {
        if let Some(control) = &self.state.lock().expect("update state").download {
            control.cancel();
        }
    }
    pub(super) fn progress(&self) -> StateFlow<ApplicationUpdateProgress> {
        self.progress.clone()
    }
    pub(super) async fn install(
        &self,
        receipt: String,
        mut tools: operit_tools::tools::AIToolHandler::AIToolHandler,
    ) -> Result<ApplicationUpdateInstallResult, String> {
        let manager = self;

        let _busy = manager.begin()?;
        let path = {
            let state = manager.state.lock().map_err(|e| e.to_string())?;
            let prepared = state
                .prepared
                .as_ref()
                .filter(|p| p.receipt == receipt)
                .ok_or("UPDATE_PACKAGE_MISSING: update receipt has expired")?;
            if let Some(result) = &prepared.installation {
                if result.installState.as_deref() == Some("restart_required") {
                    return Ok(ApplicationUpdateInstallResult::fromOperation(result));
                }
            }
            if !prepared.path.is_file() {
                return Err(
                    "UPDATE_PACKAGE_MISSING: downloaded package was removed; download again".into(),
                );
            }
            prepared
                .path
                .to_str()
                .ok_or("Installer path is not UTF-8")?
                .to_owned()
        };
        let result = tools
            .executeTool(AITool {
                name: "install_app".into(),
                parameters: vec![ToolParameter {
                    name: "path".into(),
                    value: path,
                }],
            })
            .await;
        if !result.success {
            return Err(result
                .error
                .unwrap_or_else(|| "Native installation failed".into()));
        }
        let ToolResultData::AppOperationData(data) = result.result else {
            return Err("install_app returned no installation state".into());
        };
        if !data.success {
            return Err(data.details);
        }
        if !matches!(
            data.installState.as_deref(),
            Some("permission_required" | "installer_opened" | "restart_required" | "completed")
        ) {
            return Err("install_app returned an unknown installation state".into());
        }
        if let Some(prepared) = &mut manager.state.lock().map_err(|e| e.to_string())?.prepared {
            prepared.installation = Some(data.clone());
        }
        Ok(ApplicationUpdateInstallResult::fromOperation(&data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operation_guard_and_cancel_share_one_native_task() {
        let manager = ApplicationUpdateManager::new();
        let guard = manager.begin().unwrap();
        assert!(manager.begin().is_err());
        let control = HttpDownloadControl::new();
        manager.state.lock().unwrap().download = Some(control.clone());
        manager.cancel();
        assert!(control.isCancelled());
        drop(guard);
        assert!(!manager.state.lock().unwrap().busy);
        assert!(manager.state.lock().unwrap().download.is_none());
        assert!(manager.begin().is_ok());
    }
    #[tokio::test]
    async fn retry_keeps_native_path_private_and_reuses_receipt() {
        let manager = ApplicationUpdateManager::new();
        let path = std::env::temp_dir().join(format!("operit-receipt-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"fixture").unwrap();
        {
            let mut state = manager.state.lock().unwrap();
            state.release = Some(ReleaseInfo {
                version: "2.0.0".into(),
                assetName: "native.pkg".into(),
                downloadUrl: "private-native-url".into(),
                releaseNotes: String::new(),
                releasePageUrl: "release-page".into(),
            });
            state.prepared = Some(PreparedUpdate {
                receipt: "opaque-receipt".into(),
                path: path.clone(),
                installation: None,
            });
        }
        let info = manager.check().await.unwrap().unwrap();
        assert_eq!(info.version, "2.0.0");
        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("private-native-url"));
        assert!(!json.contains("native.pkg"));
        assert_eq!(
            manager.download("2.0.0".into()).await.unwrap(),
            "opaque-receipt"
        );
        assert!(manager.download("3.0.0".into()).await.is_err());
        std::fs::remove_file(path).unwrap();
    }
}
