#[derive(Clone, Copy)]
pub struct PluginAsset {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

pub static BUILTIN_PLUGIN_ASSETS: &[PluginAsset] = &[
];

pub static BUNDLED_EXTERNAL_PLUGIN_ASSETS: &[PluginAsset] = &[
    PluginAsset { name: "context_limiter.toolpkg", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/external/context_limiter.toolpkg") },
    PluginAsset { name: "dino_runner.toolpkg", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/external/dino_runner.toolpkg") },
    PluginAsset { name: "message_insert.toolpkg", bytes: include_bytes!("/home/runner/work/Operit2/Operit2/core/crates/runtime/application/assets/plugins/external/message_insert.toolpkg") },
];
