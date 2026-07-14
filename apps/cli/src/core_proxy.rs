use std::ops::{Deref, DerefMut};

use operit_core_proxy::GeneratedCoreProxy;
use operit_host_api::HostManager::HostManager;
use operit_link::CoreLinkClient;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;

use crate::create_local_core;

pub(crate) struct CliCore {
    proxy: GeneratedCoreProxy<Box<dyn CoreLinkClient + Send>>,
    localHostManager: Option<HostManager>,
}

pub(crate) fn cli_core(client: impl CoreLinkClient + Send + 'static) -> CliCore {
    CliCore {
        proxy: GeneratedCoreProxy::new(Box::new(client)),
        localHostManager: None,
    }
}

pub(crate) fn local_cli_core() -> Result<CliCore, String> {
    let mut core = create_local_core();
    core.localApplicationMut().onCreate()?;
    let localHostManager = core.localApplicationMut().hostManager.clone();
    {
        let application = core.localApplicationMut();
        let enhanced_ai_service = EnhancedAIService::new(
            application.toolHandler.clone(),
            application.providerRuntimeContext.clone(),
        );
        let mut holder = application
            .chatRuntimeHolder
            .try_lock()
            .map_err(|_| "Chat runtime holder is busy".to_string())?;
        holder.getCore(ChatRuntimeSlot::MAIN).enhancedAiService = Some(enhanced_ai_service);
    }
    Ok(CliCore {
        proxy: GeneratedCoreProxy::new(Box::new(core)),
        localHostManager: Some(localHostManager),
    })
}

impl CliCore {
    /// Returns the host context owned by an in-process CLI runtime.
    pub(crate) fn localHostManager(&self) -> Result<&HostManager, String> {
        self.localHostManager
            .as_ref()
            .ok_or_else(|| "this CLI command requires an in-process runtime".to_string())
    }
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
