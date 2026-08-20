use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use operit_core_proxy::GeneratedCoreProxy;
use operit_link::{CoreEvent, CoreLinkClient, CoreLinkError, CoreRequestId, CoreValue};

pub(super) struct TuiCore {
    proxy: GeneratedCoreProxy<Box<dyn CoreLinkClient + Send>>,
    eventSender: tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    eventReceiver: tokio::sync::mpsc::UnboundedReceiver<CoreEvent>,
    messageWatchTask: Option<tokio::task::JoinHandle<()>>,
    messageWatchChatId: Option<String>,
    messageWatchRequestId: Option<CoreRequestId>,
    messageWatchGeneration: u64,
}

/// Creates a TUI proxy wrapper with an internal event queue.
pub(super) fn tui_core(client: impl CoreLinkClient + Send + 'static) -> TuiCore {
    let (eventSender, eventReceiver) = tokio::sync::mpsc::unbounded_channel();
    TuiCore {
        proxy: GeneratedCoreProxy::new(Box::new(client)),
        eventSender,
        eventReceiver,
        messageWatchTask: None,
        messageWatchChatId: None,
        messageWatchRequestId: None,
        messageWatchGeneration: 0,
    }
}

impl TuiCore {
    #[allow(non_snake_case)]
    /// Watches every zero-argument state flow exposed by the main chat runtime.
    pub(super) async fn watchMainChatGeneratedStateFlows(&mut self) -> Result<(), CoreLinkError> {
        self.proxy
            .chat_runtime_holder_main()
            .watchAllGeneratedStateFlows(self.eventSender.clone())
            .await
    }

    #[allow(non_snake_case)]
    /// Watches message changes for one explicit main chat id.
    pub(super) async fn watchMainChatMessagesFlow(
        &mut self,
        chatId: String,
    ) -> Result<(), CoreLinkError> {
        if self.messageWatchChatId.as_ref() == Some(&chatId) {
            return Ok(());
        }
        self.clearMainChatMessagesWatch();
        self.messageWatchGeneration += 1;
        let requestId = CoreRequestId::new(format!(
            "tui-main-chat-messages-{}",
            self.messageWatchGeneration
        ));
        let mut chatProxy = self.proxy.chat_runtime_holder_main();
        let mut args = BTreeMap::new();
        args.insert(
            "chatId".to_string(),
            operit_link::toCoreValue(Some(chatId.clone())).map_err(|error| {
                CoreLinkError::new("INVALID_ARGS", format!("chatId: {error}"))
            })?,
        );
        let request = operit_link::CoreWatchRequest::new(
            requestId.0.clone(),
            chatProxy.generatedTargetPath().clone(),
            "chatMessagesFlow",
            CoreValue::Map(args),
        );
        let mut stream = chatProxy.generatedClientMut().watch(request).await?;
        let sender = self.eventSender.clone();
        let eventRequestId = requestId.clone();
        self.messageWatchChatId = Some(chatId);
        self.messageWatchRequestId = Some(requestId);
        self.messageWatchTask = Some(tokio::task::spawn_local(async move {
            while let Some(mut event) = stream.recv().await {
                event.requestId = Some(eventRequestId.clone());
                let _ = sender.send(event);
            }
        }));
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Stops the active main chat message watch.
    pub(super) fn clearMainChatMessagesWatch(&mut self) {
        if let Some(task) = self.messageWatchTask.take() {
            task.abort();
        }
        self.messageWatchChatId = None;
        self.messageWatchRequestId = None;
    }

    #[allow(non_snake_case)]
    /// Reports whether an event belongs to the active main chat message watch.
    pub(super) fn isActiveMainChatMessagesEvent(&self, event: &CoreEvent) -> bool {
        event.propertyName == "chatMessagesFlow"
            && event.requestId.as_ref() == self.messageWatchRequestId.as_ref()
    }

    #[allow(non_snake_case)]
    /// Drains all queued generated Core events.
    pub(super) fn drainEvents(&mut self) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.eventReceiver.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Deref for TuiCore {
    type Target = GeneratedCoreProxy<Box<dyn CoreLinkClient + Send>>;

    fn deref(&self) -> &Self::Target {
        &self.proxy
    }
}

impl DerefMut for TuiCore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.proxy
    }
}
