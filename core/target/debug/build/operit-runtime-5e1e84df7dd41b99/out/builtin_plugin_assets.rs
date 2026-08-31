#[derive(Clone, Copy)]
pub struct PluginAsset {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

pub static BUILTIN_PLUGIN_ASSETS: &[PluginAsset] = &[
    PluginAsset { name: "browser.js", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/buildin/browser.js") },
    PluginAsset { name: "extended_chat.js", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/buildin/extended_chat.js") },
    PluginAsset { name: "extended_memory_tools.js", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/buildin/extended_memory_tools.js") },
    PluginAsset { name: "operit_editor.js", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/buildin/operit_editor.js") },
    PluginAsset { name: "super_admin.js", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/buildin/super_admin.js") },
    PluginAsset { name: "thinking_guidance.toolpkg", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/buildin/thinking_guidance.toolpkg") },
];

pub static BUNDLED_EXTERNAL_PLUGIN_ASSETS: &[PluginAsset] = &[
    PluginAsset { name: "context_limiter.toolpkg", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/external/context_limiter.toolpkg") },
    PluginAsset { name: "dino_runner.toolpkg", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/external/dino_runner.toolpkg") },
    PluginAsset { name: "message_insert.toolpkg", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/external/message_insert.toolpkg") },
];
