use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkClient, CoreLinkError,
    CoreLinkSharedClient, CoreObjectPath, CorePushItem, CorePushRequest, CoreWatchRequest,
};
use operit_link_access::{LinkAccessRoute, LinkAccessStore, PairedRemoteSession};
use std::sync::Arc;

use crate::LocalCoreProxy;

/// Stores the transport selected when one client-owned Link push stream opens.
#[derive(Clone)]
pub enum RuntimeCorePushTarget {
    Local {
        request: CorePushRequest,
    },
    Remote {
        session: PairedRemoteSession,
        remotePushId: String,
    },
}

/// Routes incoming Core Link traffic through the runtime-owned Link routing configuration.
pub struct RuntimeCoreRouter {
    localCore: Arc<LocalCoreProxy>,
    linkAccessStore: LinkAccessStore,
}

enum RuntimeCoreTarget {
    Local,
    Remote(PairedRemoteSession),
}

impl RuntimeCoreRouter {
    /// Creates a router over the local Core and its runtime-owned Link Access records.
    pub fn new(localCore: Arc<LocalCoreProxy>) -> Self {
        let linkAccessStore = LinkAccessStore::new(localCore.runtimeStorageHost());
        Self {
            localCore,
            linkAccessStore,
        }
    }

    /// Opens a push stream and binds it to the transport selected at open time.
    #[allow(non_snake_case)]
    pub async fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<RuntimeCorePushTarget, CoreLinkError> {
        match self.resolveTarget(&request.targetPath).await? {
            RuntimeCoreTarget::Local => Ok(RuntimeCorePushTarget::Local { request }),
            RuntimeCoreTarget::Remote(session) => {
                let remotePushId = session
                    .pushOpen(request)
                    .await
                    .map_err(CoreLinkError::internal)?;
                Ok(RuntimeCorePushTarget::Remote {
                    session,
                    remotePushId,
                })
            }
        }
    }

    /// Sends one ordered input item through the push stream's bound transport.
    #[allow(non_snake_case)]
    pub async fn pushItem(
        &self,
        target: &RuntimeCorePushTarget,
        item: CorePushItem,
    ) -> Result<(), CoreLinkError> {
        match target {
            RuntimeCorePushTarget::Local { request } => {
                let response = CoreLinkSharedClient::call(
                    self.localCore.as_ref(),
                    request.itemCall(item.sequence, item.args),
                )
                .await;
                response.result.map(|_| ())
            }
            RuntimeCorePushTarget::Remote {
                session,
                remotePushId,
            } => session
                .pushItem(CorePushItem {
                    pushId: remotePushId.clone(),
                    sequence: item.sequence,
                    args: item.args,
                })
                .await
                .map_err(CoreLinkError::internal),
        }
    }

    /// Closes a push stream through the transport selected when it opened.
    #[allow(non_snake_case)]
    pub async fn closePush(&self, target: RuntimeCorePushTarget) -> Result<(), CoreLinkError> {
        match target {
            RuntimeCorePushTarget::Local { .. } => Ok(()),
            RuntimeCorePushTarget::Remote {
                session,
                remotePushId,
            } => session
                .pushClose(remotePushId)
                .await
                .map_err(CoreLinkError::internal),
        }
    }

    /// Resolves the current runtime-owned route into a local Core or paired remote session.
    async fn resolveTarget(
        &self,
        targetPath: &CoreObjectPath,
    ) -> Result<RuntimeCoreTarget, CoreLinkError> {
        if crate::generated_is_local_runtime_control_path(targetPath) {
            return Ok(RuntimeCoreTarget::Local);
        }
        let config = self
            .linkAccessStore
            .initializeRoutingConfig()
            .map_err(CoreLinkError::internal)?;
        match config.route {
            LinkAccessRoute::Local => Ok(RuntimeCoreTarget::Local),
            LinkAccessRoute::Remote { sessionName } => self.resolveRemoteTarget(&sessionName),
        }
    }

    /// Resolves one persisted paired session name into an authenticated remote transport.
    fn resolveRemoteTarget(&self, sessionName: &str) -> Result<RuntimeCoreTarget, CoreLinkError> {
        let sessions = self
            .linkAccessStore
            .outboundSessions()
            .map_err(CoreLinkError::internal)?;
        let record = sessions.get(sessionName).cloned().ok_or_else(|| {
            CoreLinkError::new(
                "REMOTE_SESSION_NOT_FOUND",
                format!("paired remote session is not available: {sessionName}"),
            )
        })?;
        let session = PairedRemoteSession::fromRecord(record).map_err(CoreLinkError::internal)?;
        Ok(RuntimeCoreTarget::Remote(session))
    }
}

#[async_trait(?Send)]
impl CoreLinkClient for RuntimeCoreRouter {
    /// Executes a one-shot request through the route selected by runtime state.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self, request).await
    }

    /// Reads a watch snapshot through the route selected by runtime state.
    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self, request).await
    }

    /// Opens a watch stream through the route selected by runtime state.
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        CoreLinkSharedClient::watch(self, request).await
    }
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for RuntimeCoreRouter {
    /// Executes a one-shot request through the current local or remote route.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        match self.resolveTarget(&request.targetPath).await {
            Ok(RuntimeCoreTarget::Local) => {
                CoreLinkSharedClient::call(self.localCore.as_ref(), request).await
            }
            Ok(RuntimeCoreTarget::Remote(session)) => match session.call(request).await {
                Ok(response) => response,
                Err(error) => CoreCallResponse::err(requestId, CoreLinkError::internal(error)),
            },
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    /// Reads a watch snapshot through the current local or remote route.
    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        match self.resolveTarget(&request.targetPath).await? {
            RuntimeCoreTarget::Local => {
                CoreLinkSharedClient::watchSnapshot(self.localCore.as_ref(), request).await
            }
            RuntimeCoreTarget::Remote(session) => session
                .watchSnapshot(request)
                .await
                .map_err(CoreLinkError::internal),
        }
    }

    /// Opens a watch stream through the current local or remote route.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        match self.resolveTarget(&request.targetPath).await? {
            RuntimeCoreTarget::Local => {
                CoreLinkSharedClient::watch(self.localCore.as_ref(), request).await
            }
            RuntimeCoreTarget::Remote(session) => session
                .watch(request)
                .await
                .map_err(CoreLinkError::internal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies runtime Link control endpoints remain local under every route selection.
    #[test]
    fn runtime_control_paths_are_local() {
        assert!(crate::generated_is_local_runtime_control_path(&CoreObjectPath::parse(
            "linkAccessStore"
        )));
        assert!(crate::generated_is_local_runtime_control_path(&CoreObjectPath::parse(
            "runtimeRemoteLinkService"
        )));
        assert!(!crate::generated_is_local_runtime_control_path(&CoreObjectPath::parse(
            "application"
        )));
    }
}
