//! Bound overlapping local Space working sets on the constrained device.
//! Inbound RPCs and reply delivery must stay independent: local work can await them.
use tokio::sync::{Semaphore, SemaphorePermit};

static WORK: Semaphore = Semaphore::const_new(1);

pub async fn acquire() -> SemaphorePermit<'static> {
    WORK.acquire().await.expect("Space work semaphore stays open")
}

pub fn try_acquire() -> Option<SemaphorePermit<'static>> {
    WORK.try_acquire().ok()
}

pub fn is_busy() -> bool {
    WORK.available_permits() == 0
}
