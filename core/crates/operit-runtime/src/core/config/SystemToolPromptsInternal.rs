use operit_host_api::HostEnvironmentDescriptor;

use crate::core::config::SystemToolPrompts::{
    SystemToolPromptCategory, SystemToolPrompts, ToolParameterSchema, ToolPrompt,
};

pub struct SystemToolPromptsInternal;

impl SystemToolPromptsInternal {
    #[allow(non_snake_case)]
    pub fn internalToolCategoriesEn() -> Vec<SystemToolPromptCategory> {
        Self::internalToolCategoriesEnForHost(&HostEnvironmentDescriptor::android())
    }

    #[allow(non_snake_case)]
    pub fn internalToolCategoriesEnForHost(
        host_environment: &HostEnvironmentDescriptor,
    ) -> Vec<SystemToolPromptCategory> {
        internalToolCategoriesEnSource()
            .into_iter()
            .map(|category| {
                SystemToolPrompts::applyHostEnvironmentToCategory(category, host_environment, true)
            })
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn internalToolCategoriesCn() -> Vec<SystemToolPromptCategory> {
        Self::internalToolCategoriesCnForHost(&HostEnvironmentDescriptor::android())
    }

    #[allow(non_snake_case)]
    pub fn internalToolCategoriesCnForHost(
        host_environment: &HostEnvironmentDescriptor,
    ) -> Vec<SystemToolPromptCategory> {
        internalToolCategoriesCnSource()
            .into_iter()
            .map(|category| {
                SystemToolPrompts::applyHostEnvironmentToCategory(category, host_environment, false)
            })
            .collect()
    }
}

fn internalToolCategoriesEnSource() -> Vec<SystemToolPromptCategory> {
    vec![
        category(
            "Internal Tools",
            "",
            vec![
                tool(
                    "get_terminal_info",
                    "Get terminal platform info and supported terminal types.",
                    "",
                    vec![],
                    "",
                    "",
                ),
                tool(
                    "apply_file",
                    "Applies edits to a file by finding and replacing/deleting a matched content block.",
                    "",
                    vec![
                        param("path", "string", "file path", true, None),
                        param("type", "string", "operation type: replace | delete | create", true, None),
                        param("old", "string", "the exact content to be matched and replaced/deleted (required for replace/delete)", false, None),
                        param("new", "string", "the new content to insert (required for replace/create)", false, None)
                    ],
                    "\n  - **How it works**:\n    - The tool finds the best fuzzy match of `old` in the current file content (not by line numbers) and applies the requested operation.\n    - You can call this tool multiple times to apply multiple independent edits.\n\n  - **Parameters**:\n    - `type`:\n      - `replace`: replace the matched `old` content with `new`\n      - `delete`: delete the matched `old` content\n      - `create`: create the file when it does not exist (write `new` as full file content)\n    - `old`: required for `replace` / `delete`\n    - `new`: required for `replace` / `create`\n\n  - **CRITICAL RULES**:\n    1. **If you need to rewrite a whole existing file**: do **NOT** use apply_file to overwrite it. Instead, call `delete_file` first, then use `apply_file` with `type=create`.\n    2. **If you need to modify an existing file**: you **MUST** use `type=replace` (or `type=delete`) and provide `old` / `new`. Do **NOT** delete the whole file and rewrite it.\n",
                    "",
                ),
                tool(
                    "create_terminal_session",
                    "Create or get a terminal session.",
                    "",
                    vec![
                        param("session_name", "string", "terminal session name", true, None),
                        param("type", "string", "optional terminal type. Linux host supports linux. Windows host supports bash and powershell.", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_in_terminal_session",
                    "Execute a command in a terminal session and collect full output.",
                    "",
                    vec![
                        param("session_id", "string", "terminal session id", true, None),
                        param("command", "string", "command to execute", true, None),
                        param("timeout_ms", "integer", "optional, command timeout in milliseconds", false, Some("1800000".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_in_terminal_session_streaming",
                    "Execute a command in a terminal session and stream output.",
                    "",
                    vec![
                        param("session_id", "string", "terminal session id", true, None),
                        param("command", "string", "command to execute", true, None),
                        param("timeout_ms", "integer", "optional, command timeout in milliseconds", false, Some("1800000".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_hidden_terminal_command",
                    "Execute a command in a hidden non-PTY terminal executor. Commands using the same executor_key reuse the same hidden login context and are not shown in the visible terminal UI.",
                    "",
                    vec![
                        param("command", "string", "command to execute", true, None),
                        param("type", "string", "optional terminal type. Linux host supports linux. Windows host supports bash and powershell.", false, None),
                        param("executor_key", "string", "optional, hidden executor key used to reuse the same background shell context", false, Some("default".to_string())),
                        param("timeout_ms", "integer", "optional, command timeout in milliseconds", false, Some("120000".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "input_in_terminal_session",
                    "Write input to a terminal session. At least one of input or control is required. Typical usage is sending input first, then control=enter to submit.",
                    "",
                    vec![
                        param("session_id", "string", "terminal session id", true, None),
                        param("input", "string", "text to write to the terminal (can include newlines)", false, None),
                        param("control", "string", "control key or modifier (e.g. enter/tab/esc/up/down/left/right/home/end/pageup/pagedown, or ctrl with input=c for Ctrl+C)", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "close_terminal_session",
                    "Close a terminal session.",
                    "",
                    vec![
                        param("session_id", "string", "terminal session id", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_terminal_session_screen",
                    "Get only the current visible PTY screen content for a terminal session (single screen, no scrollback/history).",
                    "",
                    vec![
                        param("session_id", "string", "terminal session id", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "music_play",
                    "Play audio inside the app using the built-in music player.",
                    "",
                    vec![
                        param("source", "string", "audio source", true, None),
                        param("source_type", "string", "source type: path | url | uri", true, None),
                        param("title", "string", "optional display title", false, None),
                        param("artist", "string", "optional display artist", false, None),
                        param("loop", "boolean", "optional, repeat this track", false, None),
                        param("volume", "number", "optional, 0 to 1", false, None),
                        param("start_position_ms", "integer", "optional start position in milliseconds", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "music_pause",
                    "Pause the current app music playback.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "music_resume",
                    "Resume the current app music playback.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "music_stop",
                    "Stop the current app music playback.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "music_seek",
                    "Seek the current app music playback.",
                    "",
                    vec![
                        param("position_ms", "integer", "target position in milliseconds", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "music_set_volume",
                    "Set the current app music playback volume.",
                    "",
                    vec![
                        param("volume", "number", "volume from 0 to 1", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "music_status",
                    "Get the current app music playback status.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "browser_click",
                    "Click an element on the current page by browser_snapshot ref, including refs inside same-origin iframes.",
                    "",
                    vec![
                        param("ref", "string", "target element ref from browser_snapshot output; provide ref or selector", false, None),
                        param("selector", "string", "optional CSS selector used when ref is not available", false, None),
                        param("element", "string", "optional, human-readable element description", false, None),
                        param("doubleClick", "boolean", "optional, perform a double click instead of a single click", false, Some("false".to_string())),
                        param("button", "string", "optional mouse button: left/right/middle", false, Some("left".to_string())),
                        param("modifiers", "array", "optional modifier keys array: Alt/Control/ControlOrMeta/Meta/Shift", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_close",
                    "Close the current browser tab. Closing the last tab also closes the browser overlay.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "browser_close_all",
                    "Close all browser tabs. This also closes the browser overlay.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "browser_console_messages",
                    "Read browser console messages for the current page.",
                    "",
                    vec![
                        param("level", "string", "optional console level: error/warning/info/debug", false, Some("info".to_string())),
                        param("filename", "string", "optional output file name for large results", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_drag",
                    "Perform drag and drop between two page elements.",
                    "",
                    vec![
                        param("startElement", "string", "human-readable source element description", true, None),
                        param("startRef", "string", "source element ref from browser_snapshot output", true, None),
                        param("endElement", "string", "human-readable target element description", true, None),
                        param("endRef", "string", "target element ref from browser_snapshot output", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_evaluate",
                    "Evaluate a JavaScript function on the page or on a target element.",
                    "",
                    vec![
                        param("function", "string", "() => { ... } or (element) => { ... }", true, None),
                        param("element", "string", "optional, human-readable element description", false, None),
                        param("ref", "string", "optional target element ref; required when element is provided", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_file_upload",
                    "Upload one or multiple files to the active file chooser. Omit paths to cancel the chooser.",
                    "",
                    vec![
                        param("paths", "array", "optional absolute file paths", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_fill_form",
                    "Fill multiple form fields on the current page.",
                    "",
                    vec![
                        param("fields", "array", "array of field objects with name/type/value plus ref or selector", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_handle_dialog",
                    "Accept or dismiss the currently open dialog.",
                    "",
                    vec![
                        param("accept", "boolean", "true to accept, false to dismiss", true, None),
                        param("promptText", "string", "optional prompt text when handling a prompt dialog", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_hover",
                    "Hover over an element on the current page.",
                    "",
                    vec![
                        param("element", "string", "optional, human-readable element description", false, None),
                        param("ref", "string", "target element ref from browser_snapshot output", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_navigate",
                    "Navigate the active browser tab to a URL. If no tab exists yet, the first tab is created automatically.",
                    "",
                    vec![
                        param("url", "string", "target URL", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_navigate_back",
                    "Go back in the current tab history.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "browser_network_requests",
                    "Read network requests recorded for the current page.",
                    "",
                    vec![
                        param("includeStatic", "boolean", "optional, include static asset requests", false, Some("false".to_string())),
                        param("filename", "string", "optional output file name for large results", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_press_key",
                    "Press a keyboard key in the current page.",
                    "",
                    vec![
                        param("key", "string", "key name, for example ArrowLeft or a", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_resize",
                    "Resize the browser viewport.",
                    "",
                    vec![
                        param("width", "number", "viewport width", true, None),
                        param("height", "number", "viewport height", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_run_code",
                    "Run a Playwright-style code snippet against the current tab.",
                    "",
                    vec![
                        param("code", "string", "Playwright-style JavaScript snippet", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_select_option",
                    "Select option values in a dropdown element.",
                    "",
                    vec![
                        param("element", "string", "optional, human-readable element description", false, None),
                        param("ref", "string", "target select element ref from browser_snapshot output", true, None),
                        param("values", "array", "option values or visible texts to select", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_snapshot",
                    "Capture a structured accessibility-style snapshot of the current page, including same-origin iframe content.",
                    "",
                    vec![
                        param("filename", "string", "optional output snapshot file name", false, None),
                        param("selector", "string", "optional root element selector for a partial snapshot", false, None),
                        param("depth", "integer", "optional snapshot tree depth limit", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_take_screenshot",
                    "Take a screenshot of the current page or of a specific element.",
                    "",
                    vec![
                        param("type", "string", "optional image type: png or jpeg", false, Some("png".to_string())),
                        param("filename", "string", "optional output file name", false, None),
                        param("element", "string", "optional element description; when present ref is required", false, None),
                        param("ref", "string", "optional element ref; when present element is required", false, None),
                        param("fullPage", "boolean", "optional full-page capture; cannot be used with element screenshots", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_type",
                    "Type text into an editable element.",
                    "",
                    vec![
                        param("element", "string", "optional, human-readable element description", false, None),
                        param("ref", "string", "target element ref from browser_snapshot output", true, None),
                        param("text", "string", "text to type", true, None),
                        param("submit", "boolean", "optional, press Enter after typing", false, Some("false".to_string())),
                        param("slowly", "boolean", "optional, type character by character", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_wait_for",
                    "Wait for text to appear, disappear, or for a duration to pass.",
                    "",
                    vec![
                        param("time", "number", "optional wait duration in seconds", false, None),
                        param("text", "string", "optional text that must appear", false, None),
                        param("textGone", "string", "optional text that must disappear", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_tabs",
                    "List, create, select, or close browser tabs using 0-based indexes.",
                    "",
                    vec![
                        param("action", "string", "one of: list, create, select, close", true, None),
                        param("index", "integer", "optional tab index used by select or close", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_intent",
                    "Execute an Android Intent (activity/broadcast/service).",
                    "",
                    vec![
                        param("action", "string", "optional, intent action", false, None),
                        param("uri", "string", "optional, data URI", false, None),
                        param("package", "string", "optional, package name", false, None),
                        param("component", "string", "optional, component in \"package/class\" format", false, None),
                        param("type", "string", "optional, one of activity/broadcast/service", false, Some("activity".to_string())),
                        param("flags", "string", "optional, JSON array string of int flags (or a single int)", false, None),
                        param("extras", "string", "optional, JSON object string for extras", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "send_broadcast",
                    "Send a broadcast intent.",
                    "",
                    vec![
                        param("action", "string", "required, broadcast action", true, None),
                        param("uri", "string", "optional, data URI", false, None),
                        param("package", "string", "optional, package name", false, None),
                        param("component", "string", "optional, component in \"package/class\" format", false, None),
                        param("extras", "string", "optional, JSON object string for extras", false, None),
                        param("extra_key", "string", "optional, a single string extra key", false, None),
                        param("extra_value", "string", "optional, a single string extra value", false, None),
                        param("extra_key2", "string", "optional, second string extra key", false, None),
                        param("extra_value2", "string", "optional, second string extra value", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "device_info",
                    "Get device information.",
                    "",
                    Vec::new(),
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "Extended Memory Tools",
            "",
            vec![
                tool(
                    "create_memory",
                    "Creates a new memory node in the library. Use this when you want to save important information for future reference.",
                    "",
                    vec![
                        param("target_owner_key", "string", "required, memory owner key such as character:<character-id> or shared:<shared-id>", true, None),
                        param("title", "string", "required, string", true, None),
                        param("content", "string", "required, string", true, None),
                        param("content_type", "string", "optional", false, Some("\"text/plain\"".to_string())),
                        param("source", "string", "optional", false, Some("\"ai_created\"".to_string())),
                        param("folder_path", "string", "optional", false, Some("\"\"".to_string())),
                        param("tags", "string", "optional, comma-separated string", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "update_memory",
                    "Updates an existing memory node by title. Use this to modify an existing memory's content or metadata.",
                    "",
                    vec![
                        param("target_owner_key", "string", "required, memory owner key such as character:<character-id> or shared:<shared-id>", true, None),
                        param("old_title", "string", "required, string to identify the memory", true, None),
                        param("new_title", "string", "optional, string, new title if renaming", false, None),
                        param("content", "string", "optional, string", false, None),
                        param("content_type", "string", "optional, string", false, None),
                        param("source", "string", "optional, string", false, None),
                        param("credibility", "number", "optional, float 0-1", false, None),
                        param("importance", "number", "optional, float 0-1", false, None),
                        param("folder_path", "string", "optional, string", false, None),
                        param("tags", "string", "optional, comma-separated string", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "delete_memory",
                    "Deletes a memory node from the library by title. Use with caution as this operation is irreversible.",
                    "",
                    vec![
                        param("target_owner_key", "string", "required, memory owner key such as character:<character-id> or shared:<shared-id>", true, None),
                        param("title", "string", "required, string to identify the memory", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "link_memories",
                    "Creates a semantic link between two memories in the library. Use this to establish relationships between related concepts, facts, or pieces of information. This helps build a knowledge graph structure for better memory retrieval and understanding.",
                    "",
                    vec![
                        param("target_owner_key", "string", "required, memory owner key such as character:<character-id> or shared:<shared-id>", true, None),
                        param("source_title", "string", "required, string, the title of the source memory", true, None),
                        param("target_title", "string", "required, string, the title of the target memory", true, None),
                        param("link_type", "string", "optional, string, the type of relationship such as \"related\", \"causes\", \"explains\", \"part_of\", \"contradicts\", etc.", false, Some("\"related\"".to_string())),
                        param("weight", "number", "optional, float 0.0-1.0, the strength of the link with 1.0 being strongest", false, Some("0.7".to_string())),
                        param("description", "string", "optional, string, additional context about the relationship", false, Some("\"\"".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "query_memory_links",
                    "Queries links in the memory graph. Supports filtering by link_id, source_title, target_title, and link_type. Use this before updating/deleting links to precisely identify targets.",
                    "",
                    vec![
                        param("target_owner_key", "string", "required, memory owner key such as character:<character-id> or shared:<shared-id>", true, None),
                        param("link_id", "integer", "optional, exact link id", false, None),
                        param("source_title", "string", "optional, exact source memory title", false, None),
                        param("target_title", "string", "optional, exact target memory title", false, None),
                        param("link_type", "string", "optional, relation type filter", false, None),
                        param("limit", "integer", "optional, int 1-200, maximum links to return", false, Some("20".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "update_user_preferences",
                    "Updates USER.md directly. Use this when stable information about the user or their working style should be remembered in the user profile markdown.",
                    "",
                    vec![
                        param("target_owner_key", "string", "required, memory owner key such as character:<character-id> or shared:<shared-id>", true, None),
                        param("content", "string", "required, complete updated USER.md markdown content", true, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "Extended HTTP Tools",
            "",
            vec![
                tool(
                    "http_request",
                    "Send HTTP request.",
                    "",
                    vec![
                        param("url", "string", "url", true, None),
                        param("method", "string", "GET/POST/PUT/DELETE", true, None),
                        param("headers", "string", "headers", false, None),
                        param("body", "string", "body", false, None),
                        param("body_type", "string", "json/form/text/xml", false, None),
                        param("ignore_ssl", "boolean", "ignore https certificate verification, true/false", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "multipart_request",
                    "Upload files.",
                    "",
                    vec![
                        param("url", "string", "url", true, None),
                        param("method", "string", "POST/PUT", true, None),
                        param("headers", "string", "headers", false, None),
                        param("form_data", "string", "form_data", false, None),
                        param("files", "string", "JSON array string. Each item is an object: {\"field_name\": string, \"file_path\": string, \"content_type\"?: string, \"file_name\"?: string}", false, None),
                        param("ignore_ssl", "boolean", "ignore https certificate verification, true/false", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "manage_cookies",
                    "Manage cookies.",
                    "",
                    vec![
                        param("action", "string", "get/set/clear", true, None),
                        param("domain", "string", "domain", false, None),
                        param("cookies", "string", "cookies", false, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "Extended File Tools",
            "",
            vec![
                tool(
                    "file_exists",
                    "Check if a file or directory exists.",
                    "",
                    vec![
                        param("path", "string", "target path", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "move_file",
                    "Move or rename a file or directory.",
                    "",
                    vec![
                        param("source", "string", "source path", true, None),
                        param("destination", "string", "destination path", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "copy_file",
                    "Copy a file or directory through VFS paths.",
                    "",
                    vec![
                        param("source", "string", "source path", true, None),
                        param("destination", "string", "destination path", true, None),
                        param("recursive", "boolean", "boolean", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "file_info",
                    "Get detailed information about a file or directory including type, size, permissions, owner, group, and last modified time.",
                    "",
                    vec![
                        param("path", "string", "target path", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "zip_files",
                    "Compress files or directories.",
                    "",
                    vec![
                        param("source", "string", "path to compress", true, None),
                        param("destination", "string", "output zip file", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "unzip_files",
                    "Extract a zip file.",
                    "",
                    vec![
                        param("source", "string", "zip file path", true, None),
                        param("destination", "string", "extract path", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "open_file",
                    "Open a file using the system's default application.",
                    "",
                    vec![
                        param("path", "string", "file path", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "share_file",
                    "Share a file with other applications.",
                    "",
                    vec![
                        param("path", "string", "file path", true, None),
                        param("title", "string", "optional share title", false, Some("\"Share File\"".to_string()))
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "Chat Tools",
            "",
            vec![
                tool(
                    "start_chat_service",
                    "Start the floating chat service.",
                    "",
                    vec![
                        param("initial_mode", "string", "optional, initial floating mode: WINDOW, BALL, VOICE_BALL, FULLSCREEN, RESULT_DISPLAY, SCREEN_OCR", false, None),
                        param("auto_enter_voice_chat", "boolean", "optional, if true then enter voice mode automatically when opening FULLSCREEN", false, Some("false".to_string())),
                        param("wake_launched", "boolean", "optional, true if launched by wake word so UI can adjust behavior", false, Some("false".to_string())),
                        param("timeout_ms", "integer", "optional, auto close the floating window after this timeout (milliseconds). <=0 disables auto-exit.", false, None),
                        param("keep_if_exists", "boolean", "optional, if true and service already running, do not force floating window mode change", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "stop_chat_service",
                    "Stop the floating chat service.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "create_new_chat",
                    "Create a new chat.",
                    "",
                    vec![
                        param("group", "string", "optional group name for the new chat", false, None),
                        param("set_as_current_chat", "boolean", "optional, whether to switch to the new chat (default true)", false, None),
                        param("character_card_id", "string", "optional, character card id to bind for the new chat", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "list_chats",
                    "List chats (supports filtering and sorting).",
                    "",
                    vec![
                        param("query", "string", "optional, title keyword filter", false, None),
                        param("match", "string", "optional, contains | exact | regex (default contains)", false, None),
                        param("limit", "integer", "optional, max results (default 50)", false, None),
                        param("sort_by", "string", "optional, updatedAt | createdAt | messageCount (default updatedAt)", false, None),
                        param("sort_order", "string", "optional, asc | desc (default desc)", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "find_chat",
                    "Find a chat by title and return its info.",
                    "",
                    vec![
                        param("query", "string", "title keyword/regex", true, None),
                        param("match", "string", "optional, contains | exact | regex (default contains)", false, None),
                        param("index", "integer", "optional, pick Nth match (default 0)", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "agent_status",
                    "Check a chat's input processing status.",
                    "",
                    vec![
                        param("chat_id", "string", "target chat id", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "switch_chat",
                    "Switch to a chat.",
                    "",
                    vec![
                        param("chat_id", "string", "target chat id", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "update_chat_title",
                    "Update a chat title.",
                    "",
                    vec![
                        param("chat_id", "string", "target chat id", true, None),
                        param("title", "string", "new chat title", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "delete_chat",
                    "Delete a chat by id.",
                    "",
                    vec![
                        param("chat_id", "string", "target chat id", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "send_message_to_ai",
                    "Send a user message to AI.",
                    "",
                    vec![
                        param("message", "string", "message content", true, None),
                        param("chat_id", "string", "optional, target chat id", false, None),
                        param("runtime", "string", "optional, runtime slot for this send: main | floating (default floating)", false, None),
                        param("role_card_id", "string", "optional, role card id to use for this send", false, None),
                        param("sender_name", "string", "optional, display name of the sender when AI sends as user", false, None),
                        param("persist_turn", "boolean", "optional, whether this user/AI turn should be persisted to chat history; default true", false, None),
                        param("notify_reply", "boolean", "optional, override whether this turn sends reply-completed notification", false, None),
                        param("hide_user_message", "boolean", "optional, hide user message content in UI and show a placeholder marker while keeping original content in history/context", false, None),
                        param("disable_warning", "boolean", "optional, suppress AI-generated warning markup for this turn; when true, warning-driven retry branches stop instead of continuing", false, None),
                        param("timeout_ms", "integer", "optional, maximum wait time in milliseconds for this send, including response-stream acquisition and AI reply; default 180000", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "list_character_cards",
                    "List all role cards.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "get_chat_messages",
                    "Get messages from a specific chat (cross-chat history read).",
                    "",
                    vec![
                        param("chat_id", "string", "target chat id", true, None),
                        param("order", "string", "optional, asc/desc (default desc)", false, None),
                        param("limit", "integer", "optional, number of messages to return (default 20, max 200)", false, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "Internal File Tools",
            "",
            vec![
                tool(
                    "read_file_full",
                    "Read the full content of a file without enforcing size limit.",
                    "",
                    vec![
                        param("path", "string", "file path", true, None),
                        param("text_only", "boolean", "optional", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "read_file_binary",
                    "Read binary file and return base64 content.",
                    "",
                    vec![
                        param("path", "string", "file path", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "write_file",
                    "Write content to a file.",
                    "",
                    vec![
                        param("path", "string", "file path", true, None),
                        param("content", "string", "file content", true, None),
                        param("append", "boolean", "optional", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "write_file_binary",
                    "Write base64 content to a binary file.",
                    "",
                    vec![
                        param("path", "string", "file path", true, None),
                        param("base64Content", "string", "base64 encoded content", true, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "Internal UI Tools",
            "",
            vec![
                tool(
                    "get_page_info",
                    "Get current page/window UI information.",
                    "",
                    vec![
                        param("format", "string", "optional, xml/json", false, Some("xml".to_string())),
                        param("detail", "string", "optional", false, Some("summary".to_string())),
                        param("display", "string", "optional, display id for multi-display", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "tap",
                    "Tap at screen coordinates.",
                    "",
                    vec![
                        param("x", "integer", "x coordinate", true, None),
                        param("y", "integer", "y coordinate", true, None),
                        param("display", "string", "optional, display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "long_press",
                    "Long press at screen coordinates.",
                    "",
                    vec![
                        param("x", "integer", "x coordinate", true, None),
                        param("y", "integer", "y coordinate", true, None),
                        param("display", "string", "optional, display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "swipe",
                    "Swipe from start to end coordinates.",
                    "",
                    vec![
                        param("start_x", "integer", "start x", true, None),
                        param("start_y", "integer", "start y", true, None),
                        param("end_x", "integer", "end x", true, None),
                        param("end_y", "integer", "end y", true, None),
                        param("duration", "integer", "optional, duration in ms", false, Some("300".to_string())),
                        param("display", "string", "optional, display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "click_element",
                    "Click a UI element by resource id / class name / content description / bounds.",
                    "",
                    vec![
                        param("resourceId", "string", "optional", false, None),
                        param("className", "string", "optional", false, None),
                        param("contentDesc", "string", "optional", false, None),
                        param("bounds", "string", "optional, format: [left,top][right,bottom]", false, None),
                        param("partialMatch", "boolean", "optional, enable partial match for selectors", false, Some("false".to_string())),
                        param("index", "integer", "optional", false, Some("0".to_string())),
                        param("display", "string", "optional, display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "set_input_text",
                    "Set input text in focused field.",
                    "",
                    vec![
                        param("text", "string", "text to input (can be empty to clear)", true, None),
                        param("display", "string", "optional, display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "press_key",
                    "Press a key via keyevent.",
                    "",
                    vec![
                        param("key_code", "string", "key code, e.g. KEYCODE_HOME", true, None),
                        param("display", "string", "optional, display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "capture_screenshot",
                    "Capture a screenshot and return a file path.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "run_ui_subagent",
                    "Run a lightweight UI automation subagent.",
                    "",
                    vec![
                        param("intent", "string", "task description", true, None),
                        param("max_steps", "integer", "optional", false, Some("20".to_string())),
                        param("agent_id", "string", "optional, reuse agent session id. If omitted or 'default', uses the main screen. If provided and not 'default', the requested virtual screen session must be active/available; otherwise the run fails.", false, None),
                        param("target_app", "string", "optional, target app package name", false, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "Software Settings Tools",
            "",
            vec![
                tool(
                    "read_environment_variable",
                    "Read current value of an environment variable by key.",
                    "",
                    vec![
                        param("key", "string", "environment variable key", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "write_environment_variable",
                    "Write an environment variable by key; empty value clears it.",
                    "",
                    vec![
                        param("key", "string", "environment variable key", true, None),
                        param("value", "string", "optional, value to write; empty clears the key", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_cli_command",
                    "Execute an Operit CLI command by argument array and return stdout/stderr output.",
                    "",
                    vec![
                        param("args", "string", "JSON string array of CLI arguments, matching Tools.SoftwareSettings.exec(args)", true, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "Internal System Tools",
            "",
            vec![
                tool(
                    "close_all_virtual_displays",
                    "Close all virtual display overlays.",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "modify_system_setting",
                    "Modify a system setting.",
                    "",
                    vec![
                        param("setting", "string", "setting key (alias: key)", true, None),
                        param("value", "string", "setting value", true, None),
                        param("namespace", "string", "optional, system/secure/global", false, Some("system".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_system_setting",
                    "Get a system setting.",
                    "",
                    vec![
                        param("setting", "string", "setting key (alias: key)", true, None),
                        param("namespace", "string", "optional, system/secure/global", false, Some("system".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "install_app",
                    "Request app installation through the current host.",
                    "",
                    vec![
                        param("path", "string", "installer file path for the current host, for example Android APK, Windows MSI/MSIX/EXE, or Linux desktop installer file", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "uninstall_app",
                    "Request app uninstallation through the current host.",
                    "",
                    vec![
                        param("package_name", "string", "host app/package identifier, for example Android package name, Windows Appx/MSIX package name or MSI product code, or Linux Flatpak app id/local .desktop file", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "list_installed_apps",
                    "List installed apps.",
                    "",
                    vec![
                        param("include_system_apps", "boolean", "optional", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "start_app",
                    "Start an app through the current host.",
                    "",
                    vec![
                        param("package_name", "string", "host app/package identifier or executable name", true, None),
                        param("activity", "string", "optional, Android activity class name", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "stop_app",
                    "Stop an app background process through the current host.",
                    "",
                    vec![
                        param("package_name", "string", "host app/package identifier or process name", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_notifications",
                    "Get current host notifications when the host exposes a notification history/listener API.",
                    "",
                    vec![
                        param("limit", "integer", "optional", false, Some("10".to_string())),
                        param("include_ongoing", "boolean", "optional", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_app_usage_time",
                    "Get foreground or process usage time from the current host. On Android this uses Usage Access; desktop hosts report the data exposed by the platform host.",
                    "",
                    vec![
                        param("package_name", "string", "optional, exact app package name to query", false, None),
                        param("since_hours", "integer", "optional, look back this many hours", false, Some("24".to_string())),
                        param("limit", "integer", "optional, max apps to return when package_name is not provided", false, Some("10".to_string())),
                        param("include_system_apps", "boolean", "optional, include system apps when package_name is not provided", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "toast",
                    "Show a short toast/message on the current host.",
                    "",
                    vec![
                        param("message", "string", "toast text", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "send_notification",
                    "Send a notification using the AI reply completion notification channel.",
                    "",
                    vec![
                        param("title", "string", "optional", false, None),
                        param("message", "string", "notification body", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_device_location",
                    "Get current host device location.",
                    "",
                    vec![
                        param("timeout", "integer", "optional, seconds", false, Some("10".to_string())),
                        param("high_accuracy", "boolean", "optional", false, Some("false".to_string())),
                        param("include_address", "boolean", "optional", false, Some("true".to_string()))
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
    ]
}

fn internalToolCategoriesCnSource() -> Vec<SystemToolPromptCategory> {
    vec![
        category(
            "内部工具",
            "",
            vec![
                tool(
                    "get_terminal_info",
                    "获取终端平台信息和支持的终端类型。",
                    "",
                    vec![],
                    "",
                    "",
                ),
                tool(
                    "apply_file",
                    "通过查找并替换/删除匹配的内容块来编辑文件。",
                    "",
                    vec![
                        param("path", "string", "文件路径", true, None),
                        param("type", "string", "操作类型：replace | delete | create", true, None),
                        param("old", "string", "用于匹配/替换/删除的原始内容（replace/delete必填）", false, None),
                        param("new", "string", "要插入的新内容（replace/create必填）", false, None)
                    ],
                    "\n  - **工作原理**:\n    - 工具会在文件当前内容中对 `old` 做最佳的模糊匹配（不依赖行号），然后执行指定操作。\n    - 你可以多次调用本工具，对同一个文件做多处独立修改。\n\n  - **参数**:\n    - `type`:\n      - `replace`: 用 `new` 替换匹配到的 `old`\n      - `delete`: 删除匹配到的 `old`\n      - `create`: 当文件不存在时创建文件（用 `new` 作为完整文件内容）\n    - `old`: `replace` / `delete` 必填\n    - `new`: `replace` / `create` 必填\n\n  - **关键规则**:\n    1. **如果需要重写整个已存在文件**：不要用 apply_file 直接覆盖。请先 `delete_file`，再使用 `apply_file` 且 `type=create`。\n    2. **如果需要修改已存在文件**：必须用 `type=replace`（或 `type=delete`）并提供 `old/new`（或 `old`）。不要删除整个文件再重写。\n",
                    "",
                ),
                tool(
                    "create_terminal_session",
                    "创建或获取终端会话。",
                    "",
                    vec![
                        param("session_name", "string", "终端会话名称", true, None),
                        param("type", "string", "可选终端类型。Linux host 支持 linux。Windows host 支持 bash 和 powershell。", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_in_terminal_session",
                    "在终端会话中执行命令，并一次性返回完整输出。",
                    "",
                    vec![
                        param("session_id", "string", "终端会话 ID", true, None),
                        param("command", "string", "要执行的命令", true, None),
                        param("timeout_ms", "integer", "可选，超时时间（毫秒）", false, Some("1800000".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_in_terminal_session_streaming",
                    "在终端会话中执行命令，并流式返回输出。",
                    "",
                    vec![
                        param("session_id", "string", "终端会话 ID", true, None),
                        param("command", "string", "要执行的命令", true, None),
                        param("timeout_ms", "integer", "可选，超时时间（毫秒）", false, Some("1800000".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_hidden_terminal_command",
                    "在隐藏的非 PTY 终端执行器中执行命令。使用相同 executor_key 的命令会复用同一个后台登录上下文，且不会显示在可见终端 UI 中。",
                    "",
                    vec![
                        param("command", "string", "要执行的命令", true, None),
                        param("type", "string", "可选终端类型。Linux host 支持 linux。Windows host 支持 bash 和 powershell。", false, None),
                        param("executor_key", "string", "可选，用于复用同一个后台 shell 上下文的隐藏执行器 key", false, Some("default".to_string())),
                        param("timeout_ms", "integer", "可选，超时时间（毫秒）", false, Some("120000".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "input_in_terminal_session",
                    "向终端会话写入输入。input 与 control 至少传一个。通常先发送 input，再发送 control=enter 提交内容。",
                    "",
                    vec![
                        param("session_id", "string", "终端会话 ID", true, None),
                        param("input", "string", "要写入终端的文本（可包含换行）", false, None),
                        param("control", "string", "控制键或修饰键（如 enter/tab/esc/up/down/left/right/home/end/pageup/pagedown，或 control=ctrl 且 input=c 表示 Ctrl+C）", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "close_terminal_session",
                    "关闭终端会话。",
                    "",
                    vec![
                        param("session_id", "string", "终端会话 ID", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_terminal_session_screen",
                    "获取终端会话当前可见 PTY 屏幕内容（仅一屏，不包含历史滚动缓冲）。",
                    "",
                    vec![
                        param("session_id", "string", "终端会话 ID", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "music_play",
                    "使用应用内置音乐播放器播放音频。",
                    "",
                    vec![
                        param("source", "string", "音频来源", true, None),
                        param("source_type", "string", "来源类型：path | url | uri", true, None),
                        param("title", "string", "可选，显示标题", false, None),
                        param("artist", "string", "可选，显示艺术家", false, None),
                        param("loop", "boolean", "可选，循环当前曲目", false, None),
                        param("volume", "number", "可选，0 到 1", false, None),
                        param("start_position_ms", "integer", "可选，开始播放位置，单位毫秒", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "music_pause",
                    "暂停当前应用内音乐播放。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "music_resume",
                    "继续当前应用内音乐播放。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "music_stop",
                    "停止当前应用内音乐播放。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "music_seek",
                    "跳转当前应用内音乐播放位置。",
                    "",
                    vec![
                        param("position_ms", "integer", "目标位置，单位毫秒", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "music_set_volume",
                    "设置当前应用内音乐播放音量。",
                    "",
                    vec![
                        param("volume", "number", "音量，0 到 1", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "music_status",
                    "获取当前应用内音乐播放状态。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "browser_click",
                    "按 browser_snapshot 的 ref 点击当前页面元素，包括同源 iframe 内的 ref。",
                    "",
                    vec![
                        param("ref", "string", "来自 browser_snapshot 输出的目标元素 ref；ref 和 selector 至少提供一个", false, None),
                        param("selector", "string", "可选，ref 不可用时使用的 CSS 选择器", false, None),
                        param("element", "string", "可选，人类可读元素描述", false, None),
                        param("doubleClick", "boolean", "可选，是否双击", false, Some("false".to_string())),
                        param("button", "string", "可选鼠标按键：left/right/middle", false, Some("left".to_string())),
                        param("modifiers", "array", "可选修饰键数组：Alt/Control/ControlOrMeta/Meta/Shift", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_close",
                    "关闭当前浏览器 tab。关闭最后一个 tab 时也会关闭浏览器浮窗。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "browser_close_all",
                    "关闭全部浏览器 tab，并关闭浏览器浮窗。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "browser_console_messages",
                    "读取当前页面的浏览器控制台消息。",
                    "",
                    vec![
                        param("level", "string", "可选，控制台级别：error/warning/info/debug", false, Some("info".to_string())),
                        param("filename", "string", "可选，大结果输出文件名", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_drag",
                    "在两个页面元素之间执行拖拽。",
                    "",
                    vec![
                        param("startElement", "string", "源元素的人类可读描述", true, None),
                        param("startRef", "string", "源元素 ref", true, None),
                        param("endElement", "string", "目标元素的人类可读描述", true, None),
                        param("endRef", "string", "目标元素 ref", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_evaluate",
                    "在页面上或目标元素上执行 JavaScript 函数。",
                    "",
                    vec![
                        param("function", "string", "() => { ... } 或 (element) => { ... }", true, None),
                        param("element", "string", "可选，人类可读元素描述", false, None),
                        param("ref", "string", "可选，目标元素 ref；提供 element 时必须同时提供", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_file_upload",
                    "向当前 file chooser 上传一个或多个文件。不传 paths 时取消选择器。",
                    "",
                    vec![
                        param("paths", "array", "可选，绝对文件路径数组", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_fill_form",
                    "批量填写当前页面的多个表单字段。",
                    "",
                    vec![
                        param("fields", "array", "字段对象数组，每项包含 name/type/value 以及 ref 或 selector", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_handle_dialog",
                    "接受或取消当前打开的对话框。",
                    "",
                    vec![
                        param("accept", "boolean", "true 表示接受，false 表示取消", true, None),
                        param("promptText", "string", "可选，处理 prompt 时输入的文本", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_hover",
                    "悬停到当前页面的目标元素上。",
                    "",
                    vec![
                        param("element", "string", "可选，人类可读元素描述", false, None),
                        param("ref", "string", "目标元素 ref", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_navigate",
                    "让当前活动 tab 跳转到指定 URL。若当前没有 tab，会自动创建首个 tab。",
                    "",
                    vec![
                        param("url", "string", "目标 URL", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_navigate_back",
                    "在当前 tab 历史中后退。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "browser_network_requests",
                    "读取当前页面记录到的网络请求。",
                    "",
                    vec![
                        param("includeStatic", "boolean", "可选，是否包含静态资源请求", false, Some("false".to_string())),
                        param("filename", "string", "可选，大结果输出文件名", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_press_key",
                    "在当前页面按下一个键盘按键。",
                    "",
                    vec![
                        param("key", "string", "按键名，例如 ArrowLeft 或 a", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_resize",
                    "调整浏览器视口大小。",
                    "",
                    vec![
                        param("width", "number", "视口宽度", true, None),
                        param("height", "number", "视口高度", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_run_code",
                    "运行 Playwright 风格的代码片段。",
                    "",
                    vec![
                        param("code", "string", "Playwright 风格 JavaScript 代码片段", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_select_option",
                    "在下拉元素中选择一个或多个选项值。",
                    "",
                    vec![
                        param("element", "string", "可选，人类可读元素描述", false, None),
                        param("ref", "string", "来自 browser_snapshot 输出的目标下拉元素 ref", true, None),
                        param("values", "array", "要选择的值或可见文本数组", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_snapshot",
                    "抓取当前页面的结构化无障碍风格快照，包括同源 iframe 内容。",
                    "",
                    vec![
                        param("filename", "string", "可选，输出快照文件名", false, None),
                        param("selector", "string", "可选，局部快照的根元素选择器", false, None),
                        param("depth", "integer", "可选，快照树深度限制", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_take_screenshot",
                    "截取当前页面或特定元素的截图。",
                    "",
                    vec![
                        param("type", "string", "可选，图片类型：png 或 jpeg", false, Some("png".to_string())),
                        param("filename", "string", "可选，输出文件名", false, None),
                        param("element", "string", "可选，元素描述；提供时必须同时提供 ref", false, None),
                        param("ref", "string", "可选，元素 ref；提供时必须同时提供 element", false, None),
                        param("fullPage", "boolean", "可选，是否整页截图；元素截图时不可使用", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_type",
                    "向可编辑元素输入文本。",
                    "",
                    vec![
                        param("element", "string", "可选，人类可读元素描述", false, None),
                        param("ref", "string", "来自 browser_snapshot 输出的目标元素 ref", true, None),
                        param("text", "string", "要输入的文本", true, None),
                        param("submit", "boolean", "可选，输入后是否按 Enter 提交", false, Some("false".to_string())),
                        param("slowly", "boolean", "可选，是否逐字符输入", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_wait_for",
                    "等待文本出现、消失，或等待指定时长。",
                    "",
                    vec![
                        param("time", "number", "可选，等待秒数", false, None),
                        param("text", "string", "可选，等待出现的文本", false, None),
                        param("textGone", "string", "可选，等待消失的文本", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "browser_tabs",
                    "使用 0-based 索引列出、创建、切换或关闭浏览器 tab。",
                    "",
                    vec![
                        param("action", "string", "list/create/select/close 之一", true, None),
                        param("index", "integer", "可选，select 或 close 使用的 tab 索引", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_intent",
                    "执行 Android Intent（activity/broadcast/service）。",
                    "",
                    vec![
                        param("action", "string", "可选，Intent action", false, None),
                        param("uri", "string", "可选，data URI", false, None),
                        param("package", "string", "可选，包名", false, None),
                        param("component", "string", "可选，\"package/class\" 格式", false, None),
                        param("type", "string", "可选，activity/broadcast/service", false, Some("activity".to_string())),
                        param("flags", "string", "可选，flag 整数数组 JSON 字符串（或单个整数）", false, None),
                        param("extras", "string", "可选，extras 的 JSON 对象字符串", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "send_broadcast",
                    "发送广播 Intent。",
                    "",
                    vec![
                        param("action", "string", "必填，广播 action", true, None),
                        param("uri", "string", "可选，data URI", false, None),
                        param("package", "string", "可选，包名", false, None),
                        param("component", "string", "可选，\"package/class\" 格式", false, None),
                        param("extras", "string", "可选，extras 的 JSON 对象字符串", false, None),
                        param("extra_key", "string", "可选，单个字符串 extra 的 key", false, None),
                        param("extra_value", "string", "可选，单个字符串 extra 的 value", false, None),
                        param("extra_key2", "string", "可选，第二个字符串 extra 的 key", false, None),
                        param("extra_value2", "string", "可选，第二个字符串 extra 的 value", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "device_info",
                    "获取设备信息。",
                    "",
                    Vec::new(),
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "拓展记忆工具",
            "",
            vec![
                tool(
                    "create_memory",
                    "在记忆库中创建新的记忆节点。当你想保存重要信息供将来参考时使用。",
                    "",
                    vec![
                        param("target_owner_key", "string", "必需，记忆 owner key，例如 character:<character-id> 或 shared:<shared-id>", true, None),
                        param("title", "string", "必需, 字符串", true, None),
                        param("content", "string", "必需, 字符串", true, None),
                        param("content_type", "string", "可选", false, Some("\"text/plain\"".to_string())),
                        param("source", "string", "可选", false, Some("\"ai_created\"".to_string())),
                        param("folder_path", "string", "可选", false, Some("\"\"".to_string())),
                        param("tags", "string", "可选, 逗号分隔的字符串", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "update_memory",
                    "通过标题更新现有的记忆节点。用于修改现有记忆的内容或元数据。",
                    "",
                    vec![
                        param("target_owner_key", "string", "必需，记忆 owner key，例如 character:<character-id> 或 shared:<shared-id>", true, None),
                        param("old_title", "string", "必需, 字符串，用于识别记忆", true, None),
                        param("new_title", "string", "可选, 字符串, 重命名时的新标题", false, None),
                        param("content", "string", "可选, 字符串", false, None),
                        param("content_type", "string", "可选, 字符串", false, None),
                        param("source", "string", "可选, 字符串", false, None),
                        param("credibility", "number", "可选, 浮点数 0-1", false, None),
                        param("importance", "number", "可选, 浮点数 0-1", false, None),
                        param("folder_path", "string", "可选, 字符串", false, None),
                        param("tags", "string", "可选, 逗号分隔的字符串", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "delete_memory",
                    "通过标题从记忆库中删除记忆节点。谨慎使用，此操作不可逆。",
                    "",
                    vec![
                        param("target_owner_key", "string", "必需，记忆 owner key，例如 character:<character-id> 或 shared:<shared-id>", true, None),
                        param("title", "string", "必需, 字符串，用于识别记忆", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "link_memories",
                    "在记忆库中的两个记忆之间创建语义链接。用于建立相关概念、事实或信息片段之间的关系。这有助于构建知识图谱结构，以便更好地检索和理解记忆。",
                    "",
                    vec![
                        param("target_owner_key", "string", "必需，记忆 owner key，例如 character:<character-id> 或 shared:<shared-id>", true, None),
                        param("source_title", "string", "必需, 字符串, 源记忆的标题", true, None),
                        param("target_title", "string", "必需, 字符串, 目标记忆的标题", true, None),
                        param("link_type", "string", "可选, 字符串, 关系类型，如\"related\"（相关）、\"causes\"（导致）、\"explains\"（解释）、\"part_of\"（部分）、\"contradicts\"（矛盾）等", false, Some("\"related\"".to_string())),
                        param("weight", "number", "可选, 浮点数 0.0-1.0, 链接强度，1.0表示最强", false, Some("0.7".to_string())),
                        param("description", "string", "可选, 字符串, 关于关系的额外上下文", false, Some("\"\"".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "query_memory_links",
                    "查询记忆图谱中的链接。支持按 link_id、source_title、target_title、link_type 过滤。适合在更新/删除链接前先精确定位目标。",
                    "",
                    vec![
                        param("target_owner_key", "string", "必需，记忆 owner key，例如 character:<character-id> 或 shared:<shared-id>", true, None),
                        param("link_id", "integer", "可选, 精确链接ID", false, None),
                        param("source_title", "string", "可选, 源记忆精确标题", false, None),
                        param("target_title", "string", "可选, 目标记忆精确标题", false, None),
                        param("link_type", "string", "可选, 关系类型过滤", false, None),
                        param("limit", "integer", "可选, 整数 1-200, 返回链接数量上限", false, Some("20".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "update_user_preferences",
                    "直接更新 USER.md。当稳定的用户信息或用户工作方式需要写入用户画像 markdown 时使用。",
                    "",
                    vec![
                        param("target_owner_key", "string", "必需，记忆 owner key，例如 character:<character-id> 或 shared:<shared-id>", true, None),
                        param("content", "string", "必需，更新后的完整 USER.md markdown 内容", true, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "拓展 HTTP 工具",
            "",
            vec![
                tool(
                    "http_request",
                    "发送HTTP请求。",
                    "",
                    vec![
                        param("url", "string", "url", true, None),
                        param("method", "string", "GET/POST/PUT/DELETE", true, None),
                        param("headers", "string", "headers", false, None),
                        param("body", "string", "body", false, None),
                        param("body_type", "string", "json/form/text/xml", false, None),
                        param("ignore_ssl", "boolean", "是否忽略HTTPS证书校验，true/false", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "multipart_request",
                    "上传文件。",
                    "",
                    vec![
                        param("url", "string", "url", true, None),
                        param("method", "string", "POST/PUT", true, None),
                        param("headers", "string", "headers", false, None),
                        param("form_data", "string", "form_data", false, None),
                        param("files", "string", "JSON数组字符串。每个元素是对象: {\"field_name\": 字符串, \"file_path\": 字符串, 可选 \"content_type\": 字符串, 可选 \"file_name\": 字符串}", false, None),
                        param("ignore_ssl", "boolean", "是否忽略HTTPS证书校验，true/false", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "manage_cookies",
                    "管理cookies。",
                    "",
                    vec![
                        param("action", "string", "get/set/clear", true, None),
                        param("domain", "string", "domain", false, None),
                        param("cookies", "string", "cookies", false, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "拓展文件工具",
            "",
            vec![
                tool(
                    "file_exists",
                    "检查文件或目录是否存在。",
                    "",
                    vec![
                        param("path", "string", "目标路径", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "move_file",
                    "移动或重命名文件或目录。",
                    "",
                    vec![
                        param("source", "string", "源路径", true, None),
                        param("destination", "string", "目标路径", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "copy_file",
                    "通过 VFS 路径复制文件或目录。",
                    "",
                    vec![
                        param("source", "string", "源路径", true, None),
                        param("destination", "string", "目标路径", true, None),
                        param("recursive", "boolean", "布尔值", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "file_info",
                    "获取文件或目录的详细信息，包括类型、大小、权限、所有者、组和最后修改时间。",
                    "",
                    vec![
                        param("path", "string", "目标路径", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "zip_files",
                    "压缩文件或目录。",
                    "",
                    vec![
                        param("source", "string", "要压缩的路径", true, None),
                        param("destination", "string", "输出zip文件", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "unzip_files",
                    "解压zip文件。",
                    "",
                    vec![
                        param("source", "string", "zip文件路径", true, None),
                        param("destination", "string", "解压路径", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "open_file",
                    "使用系统默认应用程序打开文件。",
                    "",
                    vec![
                        param("path", "string", "文件路径", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "share_file",
                    "与其他应用程序共享文件。",
                    "",
                    vec![
                        param("path", "string", "文件路径", true, None),
                        param("title", "string", "可选的共享标题", false, Some("\"Share File\"".to_string()))
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "对话工具",
            "",
            vec![
                tool(
                    "start_chat_service",
                    "启动对话服务（悬浮窗）。",
                    "",
                    vec![
                        param("initial_mode", "string", "可选，初始悬浮模式：WINDOW, BALL, VOICE_BALL, FULLSCREEN, RESULT_DISPLAY, SCREEN_OCR", false, None),
                        param("auto_enter_voice_chat", "boolean", "可选，为 true 时在打开 FULLSCREEN 时自动进入语音模式", false, Some("false".to_string())),
                        param("wake_launched", "boolean", "可选，若由唤醒词启动则为 true，以便 UI 调整行为", false, Some("false".to_string())),
                        param("timeout_ms", "integer", "可选，超时后自动关闭悬浮窗（毫秒），<=0 禁用自动关闭", false, None),
                        param("keep_if_exists", "boolean", "可选，若服务已在运行则不强制切换悬浮窗模式", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "stop_chat_service",
                    "停止对话服务（悬浮窗）。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "create_new_chat",
                    "创建新的对话。",
                    "",
                    vec![
                        param("group", "string", "新对话分组名（可选）", false, None),
                        param("set_as_current_chat", "boolean", "可选，是否切换到新对话（默认 true）", false, None),
                        param("character_card_id", "string", "可选，创建对话时绑定的角色卡 ID", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "list_chats",
                    "列出所有对话（支持筛选与排序）。",
                    "",
                    vec![
                        param("query", "string", "可选，标题关键字筛选", false, None),
                        param("match", "string", "可选，contains | exact | regex（默认 contains）", false, None),
                        param("limit", "integer", "可选，最多返回条数（默认 50）", false, None),
                        param("sort_by", "string", "可选，updatedAt | createdAt | messageCount（默认 updatedAt）", false, None),
                        param("sort_order", "string", "可选，asc | desc（默认 desc）", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "find_chat",
                    "按标题查找对话并返回其信息。",
                    "",
                    vec![
                        param("query", "string", "标题关键字/正则", true, None),
                        param("match", "string", "可选，contains | exact | regex（默认 contains）", false, None),
                        param("index", "integer", "可选，选择第 N 个匹配（默认 0）", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "agent_status",
                    "查询对话的输入处理状态。",
                    "",
                    vec![
                        param("chat_id", "string", "目标对话 ID", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "switch_chat",
                    "切换到指定对话。",
                    "",
                    vec![
                        param("chat_id", "string", "目标对话 ID", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "update_chat_title",
                    "更新对话标题。",
                    "",
                    vec![
                        param("chat_id", "string", "目标对话 ID", true, None),
                        param("title", "string", "新的对话标题", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "delete_chat",
                    "按 ID 删除对话。",
                    "",
                    vec![
                        param("chat_id", "string", "目标对话 ID", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "send_message_to_ai",
                    "向 AI 发送消息。",
                    "",
                    vec![
                        param("message", "string", "消息内容", true, None),
                        param("chat_id", "string", "可选，目标对话 ID", false, None),
                        param("runtime", "string", "可选，本次发送使用的 runtime：main | floating（默认 floating）", false, None),
                        param("role_card_id", "string", "可选，本次发送使用的角色卡 ID", false, None),
                        param("sender_name", "string", "可选，当以用户身份发送时的显示名称", false, None),
                        param("persist_turn", "boolean", "可选，本轮用户消息与 AI 回复是否持久化到聊天历史，默认 true", false, None),
                        param("notify_reply", "boolean", "可选，覆盖本轮是否发送回复完成通知", false, None),
                        param("hide_user_message", "boolean", "可选，仅在 UI 中隐藏用户消息正文并显示占位标记，同时保留原文进入历史与上下文", false, None),
                        param("disable_warning", "boolean", "可选，关闭本轮 AI 生成的 warning 标记；为 true 时，依赖 warning 继续重试的分支会直接停止", false, None),
                        param("timeout_ms", "integer", "可选，本次发送的最长等待时间（毫秒），覆盖响应流获取与 AI 回复等待；默认 180000", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "list_character_cards",
                    "列出所有角色卡。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "get_chat_messages",
                    "读取指定对话的消息内容（跨话题读取）。",
                    "",
                    vec![
                        param("chat_id", "string", "目标对话 ID", true, None),
                        param("order", "string", "可选，asc/desc（默认 desc）", false, None),
                        param("limit", "integer", "可选，返回消息条数（默认20，最大200）", false, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "内部文件工具",
            "",
            vec![
                tool(
                    "read_file_full",
                    "读取完整文件内容（不限制大小）。",
                    "",
                    vec![
                        param("path", "string", "文件路径", true, None),
                        param("text_only", "boolean", "可选", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "read_file_binary",
                    "读取二进制文件并返回 Base64 内容。",
                    "",
                    vec![
                        param("path", "string", "文件路径", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "write_file",
                    "写入文件内容。",
                    "",
                    vec![
                        param("path", "string", "文件路径", true, None),
                        param("content", "string", "文件内容", true, None),
                        param("append", "boolean", "可选", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "write_file_binary",
                    "将 Base64 内容写入二进制文件。",
                    "",
                    vec![
                        param("path", "string", "文件路径", true, None),
                        param("base64Content", "string", "Base64 编码内容", true, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "内部 UI 工具",
            "",
            vec![
                tool(
                    "get_page_info",
                    "获取当前页面/窗口 UI 信息。",
                    "",
                    vec![
                        param("format", "string", "可选，xml/json", false, Some("xml".to_string())),
                        param("detail", "string", "可选", false, Some("summary".to_string())),
                        param("display", "string", "可选，多屏 display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "tap",
                    "点击屏幕坐标。",
                    "",
                    vec![
                        param("x", "integer", "x 坐标", true, None),
                        param("y", "integer", "y 坐标", true, None),
                        param("display", "string", "可选，多屏 display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "long_press",
                    "长按屏幕坐标。",
                    "",
                    vec![
                        param("x", "integer", "x 坐标", true, None),
                        param("y", "integer", "y 坐标", true, None),
                        param("display", "string", "可选，多屏 display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "swipe",
                    "执行滑动手势。",
                    "",
                    vec![
                        param("start_x", "integer", "起始 x", true, None),
                        param("start_y", "integer", "起始 y", true, None),
                        param("end_x", "integer", "结束 x", true, None),
                        param("end_y", "integer", "结束 y", true, None),
                        param("duration", "integer", "可选，持续时间（毫秒）", false, Some("300".to_string())),
                        param("display", "string", "可选，多屏 display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "click_element",
                    "点击 UI 元素（resourceId / className / contentDesc / bounds）。",
                    "",
                    vec![
                        param("resourceId", "string", "可选", false, None),
                        param("className", "string", "可选", false, None),
                        param("contentDesc", "string", "可选", false, None),
                        param("bounds", "string", "可选，格式：[left,top][right,bottom]", false, None),
                        param("partialMatch", "boolean", "可选，是否启用部分匹配", false, Some("false".to_string())),
                        param("index", "integer", "可选", false, Some("0".to_string())),
                        param("display", "string", "可选，多屏 display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "set_input_text",
                    "设置输入框文本（可传空字符串以清空）。",
                    "",
                    vec![
                        param("text", "string", "要输入的文本", true, None),
                        param("display", "string", "可选，多屏 display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "press_key",
                    "按下按键（keyevent）。",
                    "",
                    vec![
                        param("key_code", "string", "按键码，例如 KEYCODE_HOME", true, None),
                        param("display", "string", "可选，多屏 display id", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "capture_screenshot",
                    "截取屏幕截图并返回文件路径。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "run_ui_subagent",
                    "运行轻量 UI 自动化子代理。",
                    "",
                    vec![
                        param("intent", "string", "任务描述", true, None),
                        param("max_steps", "integer", "可选", false, Some("20".to_string())),
                        param("agent_id", "string", "可选，可复用的 agent 会话 ID。不传或传 'default' 时使用主屏幕；传入且不为 'default' 时表示请求使用对应的虚拟屏幕会话，虚拟屏幕必须处于可用状态，否则本次运行将失败。", false, None),
                        param("target_app", "string", "可选，目标应用包名", false, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "软件设置工具",
            "",
            vec![
                tool(
                    "read_environment_variable",
                    "按 key 读取环境变量当前值。",
                    "",
                    vec![
                        param("key", "string", "环境变量名", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "write_environment_variable",
                    "按 key 写入环境变量；value 为空时清除该变量。",
                    "",
                    vec![
                        param("key", "string", "环境变量名", true, None),
                        param("value", "string", "可选，写入值；空值清除该变量", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "execute_cli_command",
                    "按参数数组执行 Operit CLI 命令，并返回 stdout/stderr 输出。",
                    "",
                    vec![
                        param("args", "string", "CLI 参数 JSON 字符串数组，对应 Tools.SoftwareSettings.exec(args)", true, None)
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
        category(
            "内部系统工具",
            "",
            vec![
                tool(
                    "close_all_virtual_displays",
                    "关闭所有虚拟屏幕。",
                    "",
                    Vec::new(),
                    "",
                    "",
                ),
                tool(
                    "modify_system_setting",
                    "修改系统设置。",
                    "",
                    vec![
                        param("setting", "string", "设置项 key（别名：key）", true, None),
                        param("value", "string", "设置值", true, None),
                        param("namespace", "string", "可选，system/secure/global", false, Some("system".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_system_setting",
                    "获取系统设置。",
                    "",
                    vec![
                        param("setting", "string", "设置项 key（别名：key）", true, None),
                        param("namespace", "string", "可选，system/secure/global", false, Some("system".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "install_app",
                    "通过当前 host 请求安装应用（需要用户确认）。",
                    "",
                    vec![
                        param("path", "string", "当前 host 的安装文件路径，例如 Android APK、Windows MSI/MSIX/EXE 或 Linux 桌面安装文件", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "uninstall_app",
                    "通过当前 host 请求卸载应用（需要用户确认）。",
                    "",
                    vec![
                        param("package_name", "string", "host 应用/包标识，例如 Android 包名、Windows Appx/MSIX 包名或 MSI 产品码、Linux Flatpak 应用 ID 或本地 .desktop 文件", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "list_installed_apps",
                    "列出已安装应用。",
                    "",
                    vec![
                        param("include_system_apps", "boolean", "可选", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "start_app",
                    "通过当前 host 启动应用。",
                    "",
                    vec![
                        param("package_name", "string", "host 应用/包标识或可执行文件名", true, None),
                        param("activity", "string", "可选，Android Activity 类名", false, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "stop_app",
                    "通过当前 host 停止应用后台进程。",
                    "",
                    vec![
                        param("package_name", "string", "host 应用/包标识或进程名", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_notifications",
                    "在当前 host 暴露通知历史/监听接口时获取通知。",
                    "",
                    vec![
                        param("limit", "integer", "可选", false, Some("10".to_string())),
                        param("include_ongoing", "boolean", "可选", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_app_usage_time",
                    "读取当前 host 的前台或进程使用时长。Android 使用 Usage Access；桌面 host 返回平台 host 能提供的数据。",
                    "",
                    vec![
                        param("package_name", "string", "可选，精确应用包名", false, None),
                        param("since_hours", "integer", "可选，向前统计多少小时", false, Some("24".to_string())),
                        param("limit", "integer", "可选，不传 package_name 时最多返回多少个应用", false, Some("10".to_string())),
                        param("include_system_apps", "boolean", "可选，不传 package_name 时是否包含系统应用", false, Some("false".to_string()))
                    ],
                    "",
                    "",
                ),
                tool(
                    "toast",
                    "在当前 host 显示短提示。",
                    "",
                    vec![
                        param("message", "string", "Toast 文本", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "send_notification",
                    "使用 AI 回复完成的通知通道发送通知。",
                    "",
                    vec![
                        param("title", "string", "可选", false, None),
                        param("message", "string", "通知内容", true, None)
                    ],
                    "",
                    "",
                ),
                tool(
                    "get_device_location",
                    "获取当前 host 的设备位置信息。",
                    "",
                    vec![
                        param("timeout", "integer", "可选，超时（秒）", false, Some("10".to_string())),
                        param("high_accuracy", "boolean", "可选", false, Some("false".to_string())),
                        param("include_address", "boolean", "可选", false, Some("true".to_string()))
                    ],
                    "",
                    "",
                )
            ],
            "",
        ),
    ]
}

fn category(
    category_name: &str,
    category_header: &str,
    tools: Vec<ToolPrompt>,
    category_footer: &str,
) -> SystemToolPromptCategory {
    SystemToolPromptCategory {
        category_name: category_name.to_string(),
        category_header: category_header.to_string(),
        tools,
        category_footer: category_footer.to_string(),
    }
}

fn tool(
    name: &str,
    description: &str,
    parameters: &str,
    parameters_structured: Vec<ToolParameterSchema>,
    details: &str,
    notes: &str,
) -> ToolPrompt {
    ToolPrompt {
        name: name.to_string(),
        description: description.to_string(),
        parameters: parameters.to_string(),
        parameters_structured,
        details: details.to_string(),
        notes: notes.to_string(),
    }
}

fn param(
    name: &str,
    value_type: &str,
    description: &str,
    required: bool,
    default: Option<String>,
) -> ToolParameterSchema {
    ToolParameterSchema {
        name: name.to_string(),
        value_type: value_type.to_string(),
        description: description.to_string(),
        required,
        default,
    }
}
