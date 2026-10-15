//! Shared package-registry readiness; waiting never loads, retries, or rescans packages.

use std::sync::{Arc, Mutex};
use tokio::sync::watch;

/// Identifies the exact lifecycle of the canonical package registration snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageRegistryLoadState {
    NotScheduled,
    Scheduled,
    Loading,
    Ready,
    Failed(String),
}

/// Keeps readiness and its error shared across package-manager snapshots.
#[derive(Clone)]
pub struct PackageRegistryReadiness {
    inner: Arc<PackageRegistryReadinessInner>,
}

struct PackageRegistryReadinessInner {
    state: watch::Sender<PackageRegistryLoadState>,
    transition: Mutex<()>,
}

impl PackageRegistryReadiness {
    /// Creates an uninitialized registry lifecycle without loading any packages.
    pub(super) fn new() -> Self {
        let (state, _) = watch::channel(PackageRegistryLoadState::NotScheduled);
        Self {
            inner: Arc::new(PackageRegistryReadinessInner {
                state,
                transition: Mutex::new(()),
            }),
        }
    }

    /// Claims the initial startup task once; repeated startup notifications never schedule another scan.
    pub fn claim_initial_load(&self) -> Result<bool, String> {
        let _guard = self
            .inner
            .transition
            .lock()
            .map_err(|error| error.to_string())?;
        let state = self.inner.state.borrow().clone();
        match state {
            PackageRegistryLoadState::NotScheduled => {
                self.inner
                    .state
                    .send_replace(PackageRegistryLoadState::Scheduled);
                Ok(true)
            }
            PackageRegistryLoadState::Scheduled
            | PackageRegistryLoadState::Loading
            | PackageRegistryLoadState::Ready => Ok(false),
            PackageRegistryLoadState::Failed(error) => Err(error),
        }
    }

    /// Announces an explicitly requested source scan before its registration snapshot is changed.
    pub fn begin_scan(&self) {
        self.inner
            .state
            .send_replace(PackageRegistryLoadState::Loading);
    }

    /// Publishes the existing load outcome after the canonical registration snapshot has been installed.
    pub fn complete_scan(&self, outcome: Result<(), String>) {
        let state = match outcome {
            Ok(()) => PackageRegistryLoadState::Ready,
            Err(error) => PackageRegistryLoadState::Failed(error),
        };
        self.inner.state.send_replace(state);
    }

    /// Publishes the original startup scheduler or loader error to every waiting command.
    pub fn fail(&self, error: String) {
        self.inner
            .state
            .send_replace(PackageRegistryLoadState::Failed(error));
    }

    /// Reads the exact current lifecycle state without inventing a ready registry.
    pub fn state(&self) -> PackageRegistryLoadState {
        self.inner.state.borrow().clone()
    }

    /// Requires a completed canonical snapshot before a caller clones or enumerates it.
    pub fn require_ready(&self) -> Result<(), String> {
        match self.state() {
            PackageRegistryLoadState::Ready => Ok(()),
            PackageRegistryLoadState::Failed(error) => Err(error),
            PackageRegistryLoadState::NotScheduled => {
                Err("Package registry startup has not been scheduled".to_string())
            }
            PackageRegistryLoadState::Scheduled | PackageRegistryLoadState::Loading => {
                Err("Package registry load has not completed".to_string())
            }
        }
    }

    /// Waits for the owning startup task without holding a manager mutex or initiating package work.
    pub async fn wait_until_ready(&self) -> Result<(), String> {
        let mut receiver = self.inner.state.subscribe();
        loop {
            let state = receiver.borrow().clone();
            match state {
                PackageRegistryLoadState::Ready => return Ok(()),
                PackageRegistryLoadState::Failed(error) => return Err(error),
                PackageRegistryLoadState::NotScheduled => {
                    return Err("Package registry startup has not been scheduled".to_string())
                }
                PackageRegistryLoadState::Scheduled | PackageRegistryLoadState::Loading => {
                    receiver
                        .changed()
                        .await
                        .map_err(|error| error.to_string())?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rejects an unscheduled registry instead of pretending that an empty catalog is ready.
    #[tokio::test(flavor = "current_thread")]
    async fn unscheduled_registry_is_an_error() {
        let readiness = PackageRegistryReadiness::new();
        assert_eq!(
            readiness.wait_until_ready().await.unwrap_err(),
            "Package registry startup has not been scheduled"
        );
    }

    /// Allows only one owner to claim initial loading and makes ready snapshots observable to clones.
    #[tokio::test(flavor = "current_thread")]
    async fn startup_claim_and_completion_are_shared() {
        let readiness = PackageRegistryReadiness::new();
        let snapshot = readiness.clone();
        assert!(readiness.claim_initial_load().unwrap());
        assert!(!snapshot.claim_initial_load().unwrap());
        let waiter = tokio::spawn(async move { snapshot.wait_until_ready().await });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        readiness.begin_scan();
        readiness.complete_scan(Ok(()));
        assert_eq!(waiter.await.unwrap(), Ok(()));
        assert!(!readiness.claim_initial_load().unwrap());
    }

    /// Broadcasts the original scheduler failure without retrying the loader or hiding its text.
    #[tokio::test(flavor = "current_thread")]
    async fn failed_startup_rejects_every_waiter_and_future_claim() {
        let readiness = PackageRegistryReadiness::new();
        assert!(readiness.claim_initial_load().unwrap());
        let first = readiness.clone();
        let second = readiness.clone();
        let first = tokio::spawn(async move { first.wait_until_ready().await });
        let second = tokio::spawn(async move { second.wait_until_ready().await });
        readiness.fail("canonical scheduler failure".to_string());
        assert_eq!(
            first.await.unwrap().unwrap_err(),
            "canonical scheduler failure"
        );
        assert_eq!(
            second.await.unwrap().unwrap_err(),
            "canonical scheduler failure"
        );
        assert_eq!(
            readiness.claim_initial_load().unwrap_err(),
            "canonical scheduler failure"
        );
    }

    /// Exposes an actual loader issue even when a source scan registered some unrelated packages.
    #[tokio::test(flavor = "current_thread")]
    async fn source_scan_error_is_not_partial_success() {
        let readiness = PackageRegistryReadiness::new();
        readiness.begin_scan();
        readiness.complete_scan(Err("invalid toolpkg manifest".to_string()));
        assert_eq!(
            readiness.wait_until_ready().await.unwrap_err(),
            "invalid toolpkg manifest"
        );
        assert_eq!(
            readiness.require_ready().unwrap_err(),
            "invalid toolpkg manifest"
        );
    }
}
