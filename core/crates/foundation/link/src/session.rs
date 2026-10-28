//! Transport-independent execution of the three Link operation families.
//!
//! Authentication, pairing and Space authorization belong to the node, before
//! this boundary. This dispatcher neither trusts transport metadata nor grants
//! permissions. Its client must already represent the authorized execution scope.
#![allow(non_snake_case)]

use std::{collections::BTreeMap, future::poll_fn, ops::Bound, task::Poll};
use crate::{CoreEventStream, CoreLinkClient, CoreLinkError, CoreLinkPushRequestMessage,
    CoreLinkPushResponse, CoreLinkPushSession, CoreLinkRequest, CoreLinkResponse,
    CoreLinkWatchRequest, CoreLinkWatchResponse, CoreRequestId};

struct Push {
    session: Box<dyn CoreLinkPushSession>,
    nextSequence: u64,
}

/// One admitted connection's subscriptions and input streams. Dropping it releases
/// those resources; it never installs a global route or starts application tasks.
/// The dispatcher has no socket, platform or node-role dependency.
pub struct CoreLinkSession<C> {
    client: C,
    watches: BTreeMap<CoreRequestId, CoreEventStream>,
    pushes: BTreeMap<String, Push>,
    maxStreams: usize,
    lastWatch: Option<CoreRequestId>,
}

impl<C: CoreLinkClient> CoreLinkSession<C> {
    /// The deployment supplies its resource bound, independently of the protocol.
    pub fn new(client: C, maxStreams: usize) -> Self {
        Self { client, watches: BTreeMap::new(), pushes: BTreeMap::new(),
            maxStreams, lastWatch: None }
    }

    fn ensureCapacity(&self) -> Result<(), CoreLinkError> {
        if self.watches.len() + self.pushes.len() >= self.maxStreams {
            Err(CoreLinkError::new("LINK_STREAM_LIMIT", "Connection stream limit reached"))
        } else { Ok(()) }
    }

    /// Execute requests in connection order. The caller must not cancel an
    /// in-flight dispatch and reuse the connection: service operations may have
    /// side effects. Closing the connection instead drops all remaining streams.
    pub async fn dispatch(&mut self, request: CoreLinkRequest) -> CoreLinkResponse {
        match request {
            CoreLinkRequest::Call(request) => {
                let context = format!("Link call {}::{} [{}]", request.target, request.methodName, request.requestId.0);
                let requestId = request.requestId.clone();
                let response = crate::isolation::future(&context, async { Ok(self.client.call(request).await) }).await;
                CoreLinkResponse::Call(response.unwrap_or_else(|error| crate::CoreCallResponse::err(requestId, error)))
            },
            CoreLinkRequest::Watch(request) => self.watch(request).await,
            CoreLinkRequest::Push(request) => self.push(request).await,
        }
    }

    async fn watch(&mut self, request: CoreLinkWatchRequest) -> CoreLinkResponse {
        let requestId = match &request {
            CoreLinkWatchRequest::Snapshot(r) | CoreLinkWatchRequest::Open(r) => r.requestId.clone(),
            CoreLinkWatchRequest::Close { requestId } => requestId.clone(),
        };
        let result = match request {
            CoreLinkWatchRequest::Snapshot(request) => crate::isolation::future(&format!("Link watch snapshot {}", requestId.0), self.client.watchSnapshot(request)).await
                .map(CoreLinkWatchResponse::Snapshot),
            CoreLinkWatchRequest::Open(request) => {
                if self.watches.contains_key(&requestId) {
                    Err(CoreLinkError::new("LINK_DUPLICATE_WATCH", "Watch id is already open"))
                } else if let Err(error) = self.ensureCapacity() { Err(error) }
                else {
                    match crate::isolation::future(&format!("Link watch open {}", requestId.0), self.client.watch(request)).await {
                        Ok(stream) => {
                            self.watches.insert(requestId.clone(), stream);
                            Ok(CoreLinkWatchResponse::Opened)
                        },
                        Err(error) => Err(error),
                    }
                }
            },
            CoreLinkWatchRequest::Close { .. } => {
                // Close is idempotent, including after a source-ended notification.
                self.watches.remove(&requestId);
                Ok(CoreLinkWatchResponse::Closed)
            },
        };
        CoreLinkResponse::Watch { requestId, result }
    }

    async fn push(&mut self, request: CoreLinkPushRequestMessage) -> CoreLinkResponse {
        let pushId = match &request {
            CoreLinkPushRequestMessage::Open(r) => r.requestId.0.clone(),
            CoreLinkPushRequestMessage::Item(r) => r.pushId.clone(),
            CoreLinkPushRequestMessage::Close { pushId } => pushId.clone(),
        };
        let result = match request {
            CoreLinkPushRequestMessage::Open(request) => {
                if self.pushes.contains_key(&pushId) {
                    Err(CoreLinkError::new("LINK_DUPLICATE_PUSH", "Push id is already open"))
                } else if let Err(error) = self.ensureCapacity() { Err(error) }
                else {
                    match crate::isolation::future(&format!("Link push open {pushId}"), self.client.openPush(request)).await {
                        Ok(session) => {
                            self.pushes.insert(pushId.clone(), Push { session, nextSequence: 0 });
                            Ok(CoreLinkPushResponse::Opened)
                        },
                        Err(error) => Err(error),
                    }
                }
            },
            CoreLinkPushRequestMessage::Item(item) => {
                match self.pushes.get_mut(&pushId) {
                    None => Err(CoreLinkError::new("LINK_PUSH_NOT_FOUND", "Push is not open")),
                    Some(push) if item.sequence != push.nextSequence =>
                        Err(CoreLinkError::new("LINK_PUSH_SEQUENCE", "Push item is out of order")),
                    Some(push) => {
                        match push.nextSequence.checked_add(1) {
                            None => Err(CoreLinkError::new("LINK_PUSH_SEQUENCE", "Push sequence exhausted")),
                            Some(next) => match crate::isolation::future(&format!("Link push item {pushId} sequence {}", item.sequence), push.session.send(item.args)).await {
                                Ok(()) => {
                                    push.nextSequence = next;
                                    Ok(CoreLinkPushResponse::ItemAccepted { sequence: item.sequence })
                                },
                                Err(error) => {
                                    // Delivery may have partially executed. Do not allow retry
                                    // on the same stream to duplicate service-side effects.
                                    self.pushes.remove(&pushId);
                                    Err(error)
                                },
                            },
                        }
                    },
                }
            },
            CoreLinkPushRequestMessage::Close { .. } => match self.pushes.remove(&pushId) {
                Some(push) => crate::isolation::future(&format!("Link push close {pushId}"), push.session.close()).await.map(|()| CoreLinkPushResponse::Closed),
                None => Err(CoreLinkError::new("LINK_PUSH_NOT_FOUND", "Push is not open")),
            },
        };
        CoreLinkResponse::Push { pushId, result }
    }

    pub fn hasWatches(&self) -> bool { !self.watches.is_empty() }

    /// Drains each ready stream's queued prefix in order, including its terminal notification.
    pub async fn nextWatchBatch(&mut self) -> Option<Vec<CoreLinkResponse>> {
        use tokio::sync::mpsc::error::TryRecvError;
        let mut batch = vec![self.nextWatchEvent().await?];
        self.watches.retain(|requestId, stream| {
            for _ in 0..=stream.pendingCount() {
                let received = crate::isolation::polling(&format!("Link watch {requestId:?} event"), || stream.try_recv());
                let result = match received {
                    Err(error) => {
                        batch.push(CoreLinkResponse::Watch { requestId: requestId.clone(), result: Err(error) });
                        return false;
                    },
                    Ok(result) => result,
                };
                let result = match result {
                    Ok(event) => CoreLinkWatchResponse::Event(event),
                    Err(TryRecvError::Empty) => return true,
                    Err(TryRecvError::Disconnected) => CoreLinkWatchResponse::Closed,
                };
                let closed = matches!(result, CoreLinkWatchResponse::Closed);
                batch.push(CoreLinkResponse::Watch { requestId: requestId.clone(), result: Ok(result) });
                if closed { return false; }
            }
            true
        });
        Some(batch)
    }

    /// Multiplex watch events fairly without spawning per-watch tasks or polling
    /// on a timer. Cancellation before readiness does not consume an event.
    /// Returns None only when this connection has no open watches.
    pub async fn nextWatchEvent(&mut self) -> Option<CoreLinkResponse> {
        if self.watches.is_empty() { return None; }
        let (requestId, event) = poll_fn(|cx| {
            let start = self.lastWatch.clone().map_or(Bound::Unbounded, Bound::Excluded);
            for (id, stream) in self.watches.range_mut((start, Bound::Unbounded)) {
                match crate::isolation::polling(&format!("Link watch {} event", id.0), || stream.poll_recv(cx)) {
                    Ok(Poll::Ready(event)) => return Poll::Ready((id.clone(), Ok(event))),
                    Ok(Poll::Pending) => {},
                    Err(error) => return Poll::Ready((id.clone(), Err(error))),
                }
            }
            if let Some(last) = &self.lastWatch {
                for (id, stream) in self.watches.range_mut(..=last.clone()) {
                    match crate::isolation::polling(&format!("Link watch {} event", id.0), || stream.poll_recv(cx)) {
                        Ok(Poll::Ready(event)) => return Poll::Ready((id.clone(), Ok(event))),
                        Ok(Poll::Pending) => {},
                        Err(error) => return Poll::Ready((id.clone(), Err(error))),
                    }
                }
            }
            Poll::Pending
        }).await;
        self.lastWatch = Some(requestId.clone());
        let result = match event {
            Err(error) => {
                self.watches.remove(&requestId);
                return Some(CoreLinkResponse::Watch { requestId, result: Err(error) });
            },
            Ok(Some(event)) => CoreLinkWatchResponse::Event(event),
            Ok(None) => {
                self.watches.remove(&requestId);
                CoreLinkWatchResponse::Closed
            },
        };
        Some(CoreLinkResponse::Watch { requestId, result: Ok(result) })
    }
}

#[cfg(test)]
mod tests;
