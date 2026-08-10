#[derive(Clone, Copy)]
pub struct PluginAsset {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

pub static BUILTIN_PLUGIN_ASSETS: &[PluginAsset] = &[
];

pub static BUNDLED_EXTERNAL_PLUGIN_ASSETS: &[PluginAsset] = &[
];
