use std::future::Future;
use std::pin::Pin;

use operit_link::LocalCoreProxy;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::data::preferences::ActivePromptManager::ActivePromptManager;
use operit_runtime::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_runtime::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::services::ChatServiceCore::ChatServiceCore;

use crate::{begin_chat_message_with_application, ChatSendArgs, ChatSendResult};

pub(super) trait TuiLocalCoreBorrowExt {
    #[allow(non_snake_case)]
    fn borrowApplication(&mut self) -> &mut OperitApplication;

    #[allow(non_snake_case)]
    fn withApplication<R>(&mut self, block: impl FnOnce(&mut OperitApplication) -> R) -> R;

    #[allow(non_snake_case)]
    fn withMainChatCore<R>(&mut self, block: impl FnOnce(&mut ChatServiceCore) -> R) -> R;

    #[allow(non_snake_case)]
    fn withToolHandler<R>(&mut self, block: impl FnOnce(AIToolHandler) -> R) -> R;

    #[allow(non_snake_case)]
    fn withModelConfigManager<R>(&mut self, block: impl FnOnce(ModelConfigManager) -> R) -> R;

    #[allow(non_snake_case)]
    fn withFunctionalConfigManager<R>(
        &mut self,
        block: impl FnOnce(FunctionalConfigManager) -> R,
    ) -> R;

    #[allow(non_snake_case)]
    fn withActivePromptManager<R>(&mut self, block: impl FnOnce(ActivePromptManager) -> R) -> R;

    #[allow(non_snake_case)]
    fn withCharacterCardManager<R>(&mut self, block: impl FnOnce(CharacterCardManager) -> R) -> R;

    #[allow(non_snake_case)]
    fn beginChatMessage(
        &mut self,
        sendArgs: ChatSendArgs,
    ) -> Pin<Box<dyn Future<Output = Result<ChatSendResult, String>> + '_>>;
}

impl TuiLocalCoreBorrowExt for LocalCoreProxy {
    #[allow(non_snake_case)]
    fn borrowApplication(&mut self) -> &mut OperitApplication {
        self.localApplicationMut()
    }

    #[allow(non_snake_case)]
    fn withApplication<R>(&mut self, block: impl FnOnce(&mut OperitApplication) -> R) -> R {
        block(self.borrowApplication())
    }

    #[allow(non_snake_case)]
    fn withMainChatCore<R>(&mut self, block: impl FnOnce(&mut ChatServiceCore) -> R) -> R {
        self.withApplication(|application| {
            block(application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN))
        })
    }

    #[allow(non_snake_case)]
    fn withToolHandler<R>(&mut self, block: impl FnOnce(AIToolHandler) -> R) -> R {
        let context = self.withApplication(|application| application.applicationContext.clone());
        block(AIToolHandler::getInstance(context))
    }

    #[allow(non_snake_case)]
    fn withModelConfigManager<R>(&mut self, block: impl FnOnce(ModelConfigManager) -> R) -> R {
        block(ModelConfigManager::default())
    }

    #[allow(non_snake_case)]
    fn withFunctionalConfigManager<R>(
        &mut self,
        block: impl FnOnce(FunctionalConfigManager) -> R,
    ) -> R {
        block(FunctionalConfigManager::default())
    }

    #[allow(non_snake_case)]
    fn withActivePromptManager<R>(&mut self, block: impl FnOnce(ActivePromptManager) -> R) -> R {
        block(ActivePromptManager::getInstance())
    }

    #[allow(non_snake_case)]
    fn withCharacterCardManager<R>(&mut self, block: impl FnOnce(CharacterCardManager) -> R) -> R {
        block(CharacterCardManager::getInstance())
    }

    #[allow(non_snake_case)]
    fn beginChatMessage(
        &mut self,
        sendArgs: ChatSendArgs,
    ) -> Pin<Box<dyn Future<Output = Result<ChatSendResult, String>> + '_>> {
        Box::pin(async move {
            begin_chat_message_with_application(self.borrowApplication(), sendArgs).await
        })
    }
}
