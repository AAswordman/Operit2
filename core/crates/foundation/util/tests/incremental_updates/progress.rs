use super::*;
use operit_util::GithubReleaseUtil::{
    FullUpdateProgressEvent, FullUpdateStage, UpdatePreparationPhase,
};
use std::sync::Mutex;

/// Runs the public update route and checks each phase's counters independently of downloads.
pub(super) async fn phases(mut fixture: Fixture, expected: &[UpdatePreparationPhase]) {
    let (release, _, current) = fixture.network(true, false);
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = events.clone();
    let path = prepareUpdate(
        release,
        current,
        &fixture.context,
        HttpDownloadControl::new(),
        move |event| collected.lock().unwrap().push(event),
    )
    .await
    .unwrap();
    assert_eq!(
        fixture
            .context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .readBytes(&path)
            .unwrap(),
        fixture.target
    );
    let events = events.lock().unwrap();
    let mut phases = Vec::new();
    let mut last = None;
    for event in events.iter() {
        if let FullUpdateProgressEvent::PreparationProgress {
            phase,
            completedBytes,
            totalBytes,
        } = event
        {
            assert!(completedBytes <= totalBytes);
            if let Some((previous_phase, previous, total)) = last {
                if previous_phase == *phase && total == *totalBytes {
                    assert!(
                        completedBytes >= &previous,
                        "phase progress moved backwards"
                    );
                } else if previous_phase != *phase {
                    assert_eq!(previous, total, "previous phase never completed");
                }
            }
            if phases.last() != Some(phase) {
                phases.push(*phase);
            }
            last = Some((*phase, *completedBytes, *totalBytes));
        }
    }
    assert_eq!(phases, expected);
    let (_, done, total) = last.unwrap();
    assert!(total > 0);
    assert_eq!(done, total);
    assert!(matches!(
        events.last(),
        Some(FullUpdateProgressEvent::StageChanged {
            stage: FullUpdateStage::Ready,
            ..
        })
    ));
}

/// Every portable binary patch exposes verification, merge and final retention work.
#[tokio::test]
async fn binary_update_reports_local_progress_before_ready() {
    phases(
        Fixture::new(),
        &[
            UpdatePreparationPhase::VerifyingBase,
            UpdatePreparationPhase::VerifyingPatch,
            UpdatePreparationPhase::ApplyingPatch,
            UpdatePreparationPhase::SavingPackage,
        ],
    )
    .await;
}

/// Cancellation from a local progress callback must remove partial output and suppress readiness.
#[tokio::test]
async fn local_progress_cancellation_keeps_installer_unpublished() {
    let mut fixture = Fixture::new();
    let (release, _, current) = fixture.network(true, false);
    let control = HttpDownloadControl::new();
    let cancelled = control.clone();
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = events.clone();
    let error = prepareUpdate(release, current, &fixture.context, control, move |event| {
        if matches!(
            event,
            FullUpdateProgressEvent::PreparationProgress {
                phase: UpdatePreparationPhase::ApplyingPatch,
                ..
            }
        ) {
            cancelled.cancel();
        }
        collected.lock().unwrap().push(event);
    })
    .await
    .unwrap_err();
    assert_eq!(error, "UPDATE_CANCELLED");
    assert!(!events.lock().unwrap().iter().any(|event| matches!(
        event,
        FullUpdateProgressEvent::StageChanged {
            stage: FullUpdateStage::Ready,
            ..
        }
    )));
    let jobs = fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .list("runtime/temp/application_updates")
        .unwrap();
    assert!(jobs.iter().all(|entry| entry.path.ends_with("/test")));
}
