use std::ops::{Deref, DerefMut};

use operit_core_proxy::GeneratedCoreProxy;
use operit_link::{CoreEvent, CoreLinkClient, CoreLinkError};

pub(super) struct TuiCore {
    proxy: GeneratedCoreProxy<Box<dyn CoreLinkClient + Send>>,
    eventSender: tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    eventReceiver: tokio::sync::mpsc::UnboundedReceiver<CoreEvent>,
}

pub(super) fn tui_core(client: impl CoreLinkClient + Send + 'static) -> TuiCore {
    let (eventSender, eventReceiver) = tokio::sync::mpsc::unbounded_channel();
    TuiCore {
        proxy: GeneratedCoreProxy::new(Box::new(client)),
        eventSender,
        eventReceiver,
    }
}

impl TuiCore {
    #[allow(non_snake_case)]
    pub(super) async fn watchMainChatGeneratedStateFlows(&mut self) -> Result<(), CoreLinkError> {
        self.proxy
            .chat_runtime_holder_main()
            .watchAllGeneratedStateFlows(self.eventSender.clone())
            .await
    }

    #[allow(non_snake_case)]
    pub(super) async fn watchMainChatResponseStream(
        &mut self,
        chatId: String,
    ) -> Result<(), CoreLinkError> {
        let mut stream = self
            .proxy
            .chat_runtime_holder_main()
            .getResponseStream(chatId)
            .await?;
        let sender = self.eventSender.clone();
        tokio::spawn(async move {
            while let Some(event) = stream.recv().await {
                let _ = sender.send(event);
            }
        });
        Ok(())
    }

    #[allow(non_snake_case)]
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
