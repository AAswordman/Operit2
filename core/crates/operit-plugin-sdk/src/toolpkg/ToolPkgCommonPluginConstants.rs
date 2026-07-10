pub const TOOLPKG_RUNTIME_COMPOSE_DSL: &str = "compose_dsl";

pub const TOOLPKG_EVENT_APPLICATION_ON_CREATE: &str = "application_on_create";
pub const TOOLPKG_EVENT_APPLICATION_ON_FOREGROUND: &str = "application_on_foreground";
pub const TOOLPKG_EVENT_APPLICATION_ON_BACKGROUND: &str = "application_on_background";
pub const TOOLPKG_EVENT_APPLICATION_ON_LOW_MEMORY: &str = "application_on_low_memory";
pub const TOOLPKG_EVENT_APPLICATION_ON_TRIM_MEMORY: &str = "application_on_trim_memory";
pub const TOOLPKG_EVENT_APPLICATION_ON_TERMINATE: &str = "application_on_terminate";
pub const TOOLPKG_EVENT_ACTIVITY_ON_CREATE: &str = "activity_on_create";
pub const TOOLPKG_EVENT_ACTIVITY_ON_START: &str = "activity_on_start";
pub const TOOLPKG_EVENT_ACTIVITY_ON_RESUME: &str = "activity_on_resume";
pub const TOOLPKG_EVENT_ACTIVITY_ON_PAUSE: &str = "activity_on_pause";
pub const TOOLPKG_EVENT_ACTIVITY_ON_STOP: &str = "activity_on_stop";
pub const TOOLPKG_EVENT_ACTIVITY_ON_DESTROY: &str = "activity_on_destroy";
pub const TOOLPKG_EVENT_MESSAGE_PROCESSING: &str = "toolpkg_message_processing";
pub const TOOLPKG_EVENT_XML_RENDER: &str = "toolpkg_xml_render";
pub const TOOLPKG_EVENT_INPUT_MENU_TOGGLE: &str = "toolpkg_input_menu_toggle";
pub const TOOLPKG_EVENT_CHAT_INPUT: &str = "toolpkg_chat_input";
pub const TOOLPKG_EVENT_CHAT_VIEW: &str = "toolpkg_chat_view";
pub const TOOLPKG_EVENT_NAVIGATION_ENTRY_ACTION: &str = "toolpkg_navigation_entry_action";
pub const TOOLPKG_EVENT_HOST_EVENT: &str = "toolpkg_host_event";
pub const TOOLPKG_EVENT_TOOL_LIFECYCLE: &str = "toolpkg_tool_lifecycle";
pub const TOOLPKG_EVENT_PROMPT_INPUT: &str = "toolpkg_prompt_input";
pub const TOOLPKG_EVENT_PROMPT_HISTORY: &str = "toolpkg_prompt_history";
pub const TOOLPKG_EVENT_PROMPT_ESTIMATE_HISTORY: &str = "toolpkg_prompt_estimate_history";
pub const TOOLPKG_EVENT_SYSTEM_PROMPT_COMPOSE: &str = "toolpkg_system_prompt_compose";
pub const TOOLPKG_EVENT_TOOL_PROMPT_COMPOSE: &str = "toolpkg_tool_prompt_compose";
pub const TOOLPKG_EVENT_PROMPT_FINALIZE: &str = "toolpkg_prompt_finalize";
pub const TOOLPKG_EVENT_PROMPT_ESTIMATE_FINALIZE: &str = "toolpkg_prompt_estimate_finalize";
pub const TOOLPKG_EVENT_SUMMARY_GENERATE: &str = "toolpkg_summary_generate";
pub const TOOLPKG_EVENT_AI_PROVIDER_LIST_MODELS: &str = "toolpkg_ai_provider_list_models";
pub const TOOLPKG_EVENT_AI_PROVIDER_SEND_MESSAGE: &str = "toolpkg_ai_provider_send_message";
pub const TOOLPKG_EVENT_AI_PROVIDER_TEST_CONNECTION: &str = "toolpkg_ai_provider_test_connection";
pub const TOOLPKG_EVENT_AI_PROVIDER_CALCULATE_INPUT_TOKENS: &str =
    "toolpkg_ai_provider_calculate_input_tokens";

pub const TOOLPKG_REGISTRATION_TOOLBOX_UI_MODULE: &str = "registerToolPkgToolboxUiModule";
pub const TOOLPKG_REGISTRATION_UI_ROUTE: &str = "registerToolPkgUiRoute";
pub const TOOLPKG_REGISTRATION_NAVIGATION_ENTRY: &str = "registerToolPkgNavigationEntry";
pub const TOOLPKG_REGISTRATION_DESKTOP_WIDGET: &str = "registerToolPkgDesktopWidget";
pub const TOOLPKG_REGISTRATION_APP_LIFECYCLE_HOOK: &str = "registerToolPkgAppLifecycleHook";
pub const TOOLPKG_REGISTRATION_MESSAGE_PROCESSING_PLUGIN: &str =
    "registerToolPkgMessageProcessingPlugin";
pub const TOOLPKG_REGISTRATION_XML_RENDER_PLUGIN: &str = "registerToolPkgXmlRenderPlugin";
pub const TOOLPKG_REGISTRATION_INPUT_MENU_TOGGLE_PLUGIN: &str =
    "registerToolPkgInputMenuTogglePlugin";
pub const TOOLPKG_REGISTRATION_CHAT_INPUT_HOOK: &str = "registerToolPkgChatInputHook";
pub const TOOLPKG_REGISTRATION_CHAT_VIEW_HOOK: &str = "registerToolPkgChatViewHook";
pub const TOOLPKG_REGISTRATION_HOST_EVENT_HOOK: &str = "registerToolPkgHostEventHook";
pub const TOOLPKG_REGISTRATION_TOOL_LIFECYCLE_HOOK: &str = "registerToolPkgToolLifecycleHook";
pub const TOOLPKG_REGISTRATION_PROMPT_INPUT_HOOK: &str = "registerToolPkgPromptInputHook";
pub const TOOLPKG_REGISTRATION_PROMPT_HISTORY_HOOK: &str = "registerToolPkgPromptHistoryHook";
pub const TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_HISTORY_HOOK: &str =
    "registerToolPkgPromptEstimateHistoryHook";
pub const TOOLPKG_REGISTRATION_SYSTEM_PROMPT_COMPOSE_HOOK: &str =
    "registerToolPkgSystemPromptComposeHook";
pub const TOOLPKG_REGISTRATION_TOOL_PROMPT_COMPOSE_HOOK: &str =
    "registerToolPkgToolPromptComposeHook";
pub const TOOLPKG_REGISTRATION_PROMPT_FINALIZE_HOOK: &str = "registerToolPkgPromptFinalizeHook";
pub const TOOLPKG_REGISTRATION_PROMPT_ESTIMATE_FINALIZE_HOOK: &str =
    "registerToolPkgPromptEstimateFinalizeHook";
pub const TOOLPKG_REGISTRATION_SUMMARY_GENERATE_HOOK: &str = "registerToolPkgSummaryGenerateHook";
pub const TOOLPKG_REGISTRATION_AI_PROVIDER: &str = "registerToolPkgAiProvider";

pub const TOOLPKG_NAV_SURFACE_TOOLBOX: &str = "toolbox";
pub const TOOLPKG_NAV_SURFACE_MAIN_SIDEBAR_PLUGINS: &str = "main_sidebar_plugins";

/// Builds the globally unique route id used by a ToolPkg UI route.
#[allow(non_snake_case)]
pub fn buildToolPkgRouteId(containerPackageName: &str, uiRouteId: &str) -> String {
    format!("toolpkg:{containerPackageName}:ui:{uiRouteId}")
}
