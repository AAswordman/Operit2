//! Host 回调到 async receive 的有界缓冲；不包含业务消息或配对状态。
use operit_host_api::{HostResult, HostRuntimeTaskSchedulerHost};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch, Mutex};

pub(super) struct Inbox {
    maxMessageBytes: usize,
    sender: mpsc::Sender<Vec<u8>>,
    receiver: Mutex<mpsc::Receiver<Vec<u8>>>,
    terminal: watch::Sender<Option<Result<(), String>>>,
}
impl Inbox {
    pub fn new(maxMessageBytes: usize) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel(32);
        Arc::new(Self {
            maxMessageBytes,
            sender,
            receiver: Mutex::new(receiver),
            terminal: watch::channel(None).0,
        })
    }
    pub fn finish(&self, result: Result<(), String>) {
        self.terminal.send_if_modified(|state| {
            if state.is_none() {
                *state = Some(result);
                true
            } else {
                false
            }
        });
    }
    pub fn put(&self, bytes: Vec<u8>) {
        // 回调不能阻塞 Host 的网络线程；超限终止连接，不丢包后继续。
        if bytes.len() > self.maxMessageBytes + 4
            || self.sender.try_send(bytes).is_err()
        {
            self.finish(Err("Peer receive queue exceeded limit".into()));
        }
    }
    pub async fn read(&self) -> Result<Option<Vec<u8>>, String> {
        let mut rx = self.receiver.lock().await;
        let mut end = self.terminal.subscribe();
        loop {
            if let Some(Err(error)) = end.borrow().clone() {
                return Err(error);
            }
            if let Ok(bytes) = rx.try_recv() {
                return Ok(Some(bytes));
            }
            if let Some(result) = end.borrow().clone() {
                return result.map(|_| None);
            }
            tokio::select! {
                biased;
                _ = end.changed() => {},
                value = rx.recv() => return Ok(value),
            }
        }
    }
    pub fn status(&self) -> Result<(), String> {
        match self.terminal.borrow().clone() {
            None => Ok(()),
            Some(Err(e)) => Err(e),
            Some(Ok(())) => Err("Peer connection closed".into()),
        }
    }
}
/// 同步 Host 能力由 Host 的任务调度执行，不能阻塞 runtime executor。
pub(super) async fn hostTask<T: Send + 'static>(
    scheduler: &Arc<dyn HostRuntimeTaskSchedulerHost>,
    task: impl FnOnce() -> HostResult<T> + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = oneshot::channel();
    scheduler
        .scheduleHostRuntimeTask(
            "peer-transport-io",
            Box::new(move || {
                let _ = tx.send(task());
            }),
        )
        .map_err(|e| e.to_string())?;
    rx.await
        .map_err(|_| "Host transport task cancelled".to_string())?
        .map_err(|e| e.to_string())
}
