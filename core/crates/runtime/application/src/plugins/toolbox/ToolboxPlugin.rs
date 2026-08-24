use crate::plugins::PluginRegistry::OperitPlugin;

pub struct ToolboxPlugin;

impl OperitPlugin for ToolboxPlugin {
    fn id(&self) -> &str {
        "builtin.toolbox"
    }

    fn register(&self) {}
}
