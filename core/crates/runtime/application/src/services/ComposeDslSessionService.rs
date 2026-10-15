use operit_host_api::HostManager::HostManager;
use operit_tools::tools::packTool::ToolPkgComposeDslSession::{ToolPkgComposeDslSession, ToolPkgComposeDslCommand, ToolPkgComposeDslEventStream, ToolPkgComposeDslEvent};
use operit_util::stream::ReverseStream::ReverseStream;
use operit_util::stream::Stream::{Stream, CollectFuture};

/// Exposes page-owned objects using the existing automatically generated stream service contract.
pub struct ComposeDslSessionService;

impl ComposeDslSessionService {
    /// Constructs the application facade; session ownership remains with its package manager.
    pub fn getInstance(_context: &HostManager) -> Self { Self }

    /// Attaches the single downstream update consumer to a live session capability.
    pub fn updates(&self, sessionId: String) -> ComposeDslEventStream {
        ComposeDslEventStream { stream: ToolPkgComposeDslSession::resolve(&sessionId).and_then(|session| session.updates()) }
    }

    /// Attaches the upstream command owner without modifying the CoreLink transport.
    pub async fn submit(&self, sessionId: String, commands: ReverseStream<ToolPkgComposeDslCommand>) -> Result<(), String> {
        ToolPkgComposeDslSession::resolve(&sessionId)?.submit(commands).await
    }

    /// Validates a command before admission without starting work or changing session state.
    pub fn validateCommand(&self, command: ToolPkgComposeDslCommand) -> Result<(), String> {
        if command.requestId.is_empty() { return Err("Compose command requires a request id".into()); }
        match command.operation.as_str() {
            "render" if command.script.is_some() => Ok(()),
            "action" if command.actionId.is_some() => Ok(()),
            _ => Err("Compose command requires an explicit render script or action id".into()),
        }
    }

    /// Closes the session before releasing the existing package execution lease.
    pub fn close(&self, sessionId: String) -> Result<(), String> {
        ToolPkgComposeDslSession::resolve(&sessionId)?.close();
        Ok(())
    }
}

/// Exposes the downstream item type directly to the existing automatic schema scanner.
pub struct ComposeDslEventStream {
    stream: Result<ToolPkgComposeDslEventStream, String>,
}

impl Stream for ComposeDslEventStream {
    type Item = ToolPkgComposeDslEvent;

    /// Delivers an attachment error explicitly and terminates an invalid stream.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        Box::pin(async move {
            match &mut self.stream {
                Ok(stream) => stream.collect(collector).await,
                Err(error) => collector(ToolPkgComposeDslEvent {
                    requestId: String::new(), phase: "sessionError".into(), update: None,
                    actionResult: None, navigationCommands: Vec::new(), error: Some(error.clone()),
                }),
            }
        })
    }
}
