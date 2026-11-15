//! Application facade for shared updates; package receipts retain VFS paths.
use operit_host_api::HostManager::{defaultHostRuntimeTaskSchedulerHost, HostManager};
use operit_host_api::HttpDownloadControl;
use operit_host_api::{AppInstallTiming, AppOperationData, SystemOperationHost};
use operit_store::PreferencesDataStore::StateFlow;
use operit_util::GithubReleaseUtil::{
    FullUpdateProgressEvent, FullUpdateStage, FullUpdateStatus, FullUpdateTarget,
    GithubReleaseUtil, ReleaseInfo,
};
use operit_util::IncrementalUpdate::{executeUpdate, planUpdate, UpdatePlan};
use operit_vfs::VisualFileSystem::VisualFileSystem;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Removes obsolete updater files once, before runtime initialization is exposed.
pub(crate) fn cleanupCache(context: &HostManager) -> Result<(), String> {
    let Some(host) = &context.systemOperationHost else {
        return Ok(());
    };
    let Some(target) = host.applicationUpdateTarget().map_err(|e| e.to_string())? else {
        return Ok(());
    };
    let target = FullUpdateTarget::fromApplicationTarget(target)?;
    operit_util::IncrementalUpdate::cleanupUpdateCache(
        crate::CORE_VERSION,
        &target.assetNameForVersion(crate::CORE_VERSION)?,
        context,
    )
}

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
    pub stage: String,
    pub receivedBytes: u64,
    pub totalBytes: u64,
}
/// Installer feedback only; the native package path stays inside the core.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUpdateInstallResult {
    pub installState: String,
    pub details: String,
}
/// Exposes one Core decision without package paths or transport URLs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUpdatePlan {
    pub mode: String,
    pub fullAvailable: bool,
    pub incrementalAvailable: bool,
    pub baseVersion: Option<String>,
    pub fullBytes: Option<u64>,
    pub incrementalBytes: Option<u64>,
}
impl ApplicationUpdatePlan {
    /// Converts the private executable plan into presentation data.
    fn fromPlan(plan: &UpdatePlan) -> Self {
        Self {
            mode: match plan.mode {
                operit_util::IncrementalUpdate::UpdateMode::Full => "full",
                operit_util::IncrementalUpdate::UpdateMode::Incremental => "incremental",
                operit_util::IncrementalUpdate::UpdateMode::NeedsBase => "needs_base",
                operit_util::IncrementalUpdate::UpdateMode::Unavailable => "unavailable",
            }
            .into(),
            fullAvailable: plan.fullAvailable,
            incrementalAvailable: plan.incrementalAvailable,
            baseVersion: plan.baseVersion.clone(),
            fullBytes: plan.fullBytes,
            incrementalBytes: plan.incrementalBytes,
        }
    }
}
impl ApplicationUpdateInstallResult {
    /// Converts typed host feedback to the existing presentation protocol.
    fn fromOperation(data: &AppOperationData) -> Self {
        Self {
            installState: data
                .installState
                .as_ref()
                .expect("validated installation state")
                .as_str()
                .into(),
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
    plan: Option<UpdatePlan>,
    installTiming: AppInstallTiming,
    target: Option<FullUpdateTarget>,
}
struct PreparedUpdate {
    version: String,
    receipt: String,
    vfsPath: String,
    installation: Option<AppOperationData>,
}
#[derive(Clone)]
pub(super) struct ApplicationUpdateManager {
    state: Arc<Mutex<UpdateState>>,
    progress: StateFlow<ApplicationUpdateProgress>,
}
struct BusyGuard(Arc<Mutex<UpdateState>>);
impl Drop for BusyGuard {
    /// Releases operation ownership and its active cancellation token.
    fn drop(&mut self) {
        let mut state = self.0.lock().expect("update state");
        state.busy = false;
        state.download = None;
    }
}
impl ApplicationUpdateManager {
    /// Creates empty update state and an observable progress value.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(UpdateState::default())),
            progress: StateFlow::new(ApplicationUpdateProgress {
                stage: "idle".into(),
                receivedBytes: 0,
                totalBytes: 0,
            }),
        }
    }
    /// Serializes update operations until the returned guard is released.
    fn begin(&self) -> Result<BusyGuard, String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        if state.busy {
            return Err("An application update operation is already active".into());
        }
        state.busy = true;
        Ok(BusyGuard(self.state.clone()))
    }
    /// Exposes presentation metadata without leaking package paths or URLs.
    fn info(release: &ReleaseInfo) -> ApplicationUpdateInfo {
        ApplicationUpdateInfo {
            currentVersion: crate::CORE_VERSION.into(),
            version: release.version.clone(),
            releasePageUrl: release.releasePageUrl.clone(),
            releaseNotes: release.releaseNotes.clone(),
        }
    }
    /// Checks shared release rules against the installation identity reported by the Host.
    pub(super) async fn check(
        &self,
        host: Option<Arc<dyn SystemOperationHost>>,
    ) -> Result<Option<ApplicationUpdateInfo>, String> {
        let scheduler = {
            let state = self.state.lock().map_err(|e| e.to_string())?;
            if state.busy || (state.prepared.is_some() && state.release.is_some()) {
                return Ok(state.release.as_ref().map(Self::info));
            }
            let scheduler = defaultHostRuntimeTaskSchedulerHost();
            let now = scheduler.monotonicTimeMillis().map_err(|e| e.to_string())?;
            if state
                .checkedAt
                .is_some_and(|at| now.saturating_sub(at) < 6 * 60 * 60 * 1000)
            {
                return Ok(state.release.as_ref().map(Self::info));
            }
            scheduler
        };
        let _busy = self.begin()?;
        let host = host.ok_or("SystemOperationHost is not registered for application updates")?;
        let Some(target) = host
            .applicationUpdateTarget()
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        let installTiming = target.installTiming;
        let target = FullUpdateTarget::fromApplicationTarget(target)?;
        operit_util::AppLogger::AppLogger::i("ApplicationUpdate", &format!(
            "Checking current={} package={}", crate::CORE_VERSION,
            target.assetNameForVersion(crate::CORE_VERSION)?,
        ));
        let result =
            GithubReleaseUtil::checkForFullUpdate(crate::CORE_VERSION, target.clone()).await?;
        operit_util::AppLogger::AppLogger::i("ApplicationUpdate", &match &result {
            FullUpdateStatus::Available(info) => format!(
                "Available version={} full={} manifest={}", info.version,
                info.fullPackageAvailable, info.updateManifest.is_some(),
            ),
            FullUpdateStatus::UpToDate => "No newer matching package found".into(),
        });
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        state.installTiming = installTiming;
        state.target = Some(target);
        state.release = match result {
            FullUpdateStatus::Available(info) => Some(info),
            FullUpdateStatus::UpToDate => None,
        };
        state.checkedAt = Some(scheduler.monotonicTimeMillis().map_err(|e| e.to_string())?);
        Ok(state.release.as_ref().map(Self::info))
    }
    /// Calculates both transfer routes once when the user requests this update.
    pub(super) async fn plan(
        &self,
        version: String,
        context: HostManager,
    ) -> Result<ApplicationUpdatePlan, String> {
        let _busy = self.begin()?;
        let (release, currentAsset, control) = {
            let mut state = self.state.lock().map_err(|e| e.to_string())?;
            let release = state
                .release
                .clone()
                .filter(|r| r.version == version)
                .ok_or("Update selection expired; check again")?;
            let currentAsset = state
                .target
                .as_ref()
                .ok_or("Update target is missing")?
                .assetNameForVersion(crate::CORE_VERSION)?;
            let control = HttpDownloadControl::new();
            state.download = Some(control.clone());
            state.plan = None;
            (release, currentAsset, control)
        };
        let plan = planUpdate(release, currentAsset, &context, control).await?;
        let result = ApplicationUpdatePlan::fromPlan(&plan);
        self.state.lock().map_err(|e| e.to_string())?.plan = Some(plan);
        Ok(result)
    }

    /// Verifies an original installer against the selected patch and preserves the plan on rejection.
    pub(super) fn supplyBase(
        &self,
        version: String,
        sourcePath: String,
        context: HostManager,
    ) -> Result<ApplicationUpdatePlan, String> {
        let _busy = self.begin()?;
        let (mut plan, control) = {
            let mut state = self.state.lock().map_err(|e| e.to_string())?;
            if !state.release.as_ref().is_some_and(|r| r.version == version) {
                return Err("Update selection expired".into());
            }
            let plan = state.plan.take().ok_or("Update must be planned first")?;
            let control = HttpDownloadControl::new();
            state.download = Some(control.clone());
            (plan, control)
        };
        let result = plan
            .supplyBase(&sourcePath, &context, &control)
            .map(|()| ApplicationUpdatePlan::fromPlan(&plan));
        self.state.lock().map_err(|e| e.to_string())?.plan = Some(plan);
        result
    }

    /// Applies the no-original choice using the same full-package availability decision.
    pub(super) fn chooseFull(&self, version: String) -> Result<ApplicationUpdatePlan, String> {
        let _busy = self.begin()?;
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        if !state.release.as_ref().is_some_and(|r| r.version == version) {
            return Err("Update selection expired".into());
        }
        let plan = state.plan.as_mut().ok_or("Update must be planned first")?;
        plan.chooseFull();
        Ok(ApplicationUpdatePlan::fromPlan(plan))
    }

    /// Executes the stored Core decision and retains the result behind an opaque receipt.
    pub(super) async fn download(
        &self,
        version: String,
        context: HostManager,
    ) -> Result<String, String> {
        let vfs = (&context).vfs()?;
        let _busy = self.begin()?;
        let (plan, control) = {
            let mut state = self.state.lock().map_err(|e| e.to_string())?;
            if !state.release.as_ref().is_some_and(|r| r.version == version) {
                return Err("Update selection expired; check again".into());
            }
            if let Some(prepared) = &state.prepared {
                if prepared.version == version
                    && vfs
                        .fileExists(&prepared.vfsPath)
                        .map(|entry| entry.exists && !entry.isDirectory)?
                {
                    return Ok(prepared.receipt.clone());
                }
            }
            let plan = state
                .plan
                .take()
                .ok_or("Update must be planned before downloading")?;
            let control = HttpDownloadControl::new();
            state.download = Some(control.clone());
            (plan, control)
        };
        self.progress.set_value(ApplicationUpdateProgress {
            stage: "downloading".into(),
            receivedBytes: 0,
            totalBytes: 0,
        });
        let progress = self.progress.clone();
        let path = executeUpdate(plan, &context, control, move |event| match event {
            FullUpdateProgressEvent::DownloadProgress {
                readBytes,
                totalBytes,
                ..
            } => progress.set_value(ApplicationUpdateProgress {
                stage: "downloading".into(),
                receivedBytes: readBytes,
                totalBytes,
            }),
            FullUpdateProgressEvent::StageChanged { stage, .. } => {
                progress.set_value(ApplicationUpdateProgress {
                    stage: match stage {
                        FullUpdateStage::SelectingSource => "selecting_source",
                        FullUpdateStage::ReconstructingPackage => "reconstructing",
                        FullUpdateStage::Ready => "ready",
                        FullUpdateStage::DownloadingPackage => "downloading",
                    }
                    .into(),
                    receivedBytes: 0,
                    totalBytes: 0,
                })
            }
        })
        .await?;
        let receipt = uuid::Uuid::new_v4().to_string();
        self.state.lock().map_err(|e| e.to_string())?.prepared = Some(PreparedUpdate {
            version,
            receipt: receipt.clone(),
            vfsPath: VisualFileSystem::runtimePath(&path)?,
            installation: None,
        });
        Ok(receipt)
    }
    /// Notifies the active request without holding application state across callbacks.
    pub(super) fn cancel(&self) {
        let control = self.state.lock().expect("update state").download.clone();
        if let Some(control) = control {
            control.cancel();
        }
    }
    /// Returns the shared download progress stream.
    pub(super) fn progress(&self) -> StateFlow<ApplicationUpdateProgress> {
        self.progress.clone()
    }
    /// Validates the update receipt and submits its VFS file reference to the installer.
    pub(super) async fn install(
        &self,
        receipt: String,
        context: HostManager,
    ) -> Result<ApplicationUpdateInstallResult, String> {
        let _busy = self.begin()?;
        let (source, timing) = {
            let state = self.state.lock().map_err(|e| e.to_string())?;
            let prepared = state
                .prepared
                .as_ref()
                .filter(|p| p.receipt == receipt)
                .ok_or("UPDATE_PACKAGE_MISSING: update receipt has expired")?;
            if let Some(result) = &prepared.installation {
                if result.installState == Some(operit_host_api::AppInstallState::RestartRequired) {
                    return Ok(ApplicationUpdateInstallResult::fromOperation(result));
                }
            }
            let vfs = (&context).vfs()?;
            let entry = vfs.fileExists(&prepared.vfsPath)?;
            if !entry.exists || entry.isDirectory {
                return Err(
                    "UPDATE_PACKAGE_MISSING: downloaded package was removed; download again".into(),
                );
            }
            (vfs.file(&prepared.vfsPath)?, state.installTiming)
        };
        let host = context
            .systemOperationHost
            .as_deref()
            .ok_or("SystemOperationHost is not registered for application updates")?;
        let data = host
            .installApp(&source, timing)
            .map_err(|error| error.to_string())?;
        if !data.success {
            return Err(data.details);
        }
        if data.installState.is_none() {
            return Err("Application update preparation returned no installation state".into());
        }
        if let Some(prepared) = &mut self.state.lock().map_err(|e| e.to_string())?.prepared {
            prepared.installation = Some(data.clone());
        }
        Ok(ApplicationUpdateInstallResult::fromOperation(&data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Operation ownership and cancellation share one update state.
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
    /// Reuses a receipt addressed through VFS without exposing the native storage root.
    #[tokio::test]
    async fn retry_keeps_native_path_private_and_reuses_receipt() {
        let manager = ApplicationUpdateManager::new();
        let root = std::env::temp_dir().join(format!("operit-receipt-{}", uuid::Uuid::new_v4()));
        let storage = Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
            root.clone(),
            root.join("workspaces"),
        ));
        let context = HostManager::withFileSystemHost(Arc::new(
            operit_host_native_filesystem::PosixFileSystemHost::new(),
        ))
        .withRuntimeStorageHost(storage.clone());
        let path = "runtime/temp/application_updates/receipt/native.pkg";
        operit_host_api::RuntimeStorageHost::writeBytes(storage.as_ref(), path, b"fixture")
            .unwrap();
        {
            let mut state = manager.state.lock().unwrap();
            state.release = Some(ReleaseInfo {
                version: "2.0.0".into(),
                assetName: "native.pkg".into(),
                downloadUrl: "private-native-url".into(),
                releaseNotes: String::new(),
                releasePageUrl: "release-page".into(),
                packageSha256: None,
                fullPackageAvailable: true,
                patchAssetNames: Vec::new(),
                updateManifest: None,
            });
            state.prepared = Some(PreparedUpdate {
                version: "2.0.0".into(),
                receipt: "opaque-receipt".into(),
                vfsPath: VisualFileSystem::runtimePath(path).unwrap(),
                installation: None,
            });
        }
        let info = manager.check(None).await.unwrap().unwrap();
        assert_eq!(info.version, "2.0.0");
        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("private-native-url"));
        assert!(!json.contains("native.pkg"));
        assert_eq!(
            manager
                .download("2.0.0".into(), context.clone())
                .await
                .unwrap(),
            "opaque-receipt"
        );
        assert!(manager.download("3.0.0".into(), context).await.is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Reuses prepared restart feedback without starting another native helper.
    #[tokio::test]
    async fn restart_preparation_is_reused_without_another_host_request() {
        let manager = ApplicationUpdateManager::new();
        manager.state.lock().unwrap().prepared = Some(PreparedUpdate {
            version: "2.0.0".into(),
            receipt: "prepared-receipt".into(),
            vfsPath: "/operitapp/data/temp/application_updates/private-package.msi".into(),
            installation: Some(AppOperationData {
                installState: Some(operit_host_api::AppInstallState::RestartRequired),
                operationType: "install".into(),
                packageName: "private-package.msi".into(),
                success: true,
                details: "Update is ready".into(),
            }),
        });
        let result = manager
            .install("prepared-receipt".into(), HostManager::new())
            .await
            .unwrap();
        assert_eq!(result.installState, "restart_required");
        assert_eq!(result.details, "Update is ready");
    }
}
