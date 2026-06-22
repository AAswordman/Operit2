use std::ops::{Deref, DerefMut};

use operit_core_proxy::GeneratedCoreProxy;
use operit_link::CoreLinkClient;
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;

use crate::create_local_core;

pub(crate) struct CliCore {
    proxy: GeneratedCoreProxy<Box<dyn CoreLinkClient + Send>>,
}

pub(crate) fn cli_core(client: impl CoreLinkClient + Send + 'static) -> CliCore {
    CliCore {
        proxy: GeneratedCoreProxy::new(Box::new(client)),
    }
}

pub(crate) fn local_cli_core() -> Result<CliCore, String> {
    let mut core = create_local_core();
    core.localApplicationMut().onCreate()?;
    let main_core = core
        .localApplicationMut()
        .chatRuntimeHolder
        .getCore(ChatRuntimeSlot::MAIN);
    main_core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    Ok(CliCore {
        proxy: GeneratedCoreProxy::new(Box::new(core)),
    })
}

impl Deref for CliCore {
    type Target = GeneratedCoreProxy<Box<dyn CoreLinkClient + Send>>;

    fn deref(&self) -> &Self::Target {
        &self.proxy
    }
}

impl DerefMut for CliCore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.proxy
    }
}
