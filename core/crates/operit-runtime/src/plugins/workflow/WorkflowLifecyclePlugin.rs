use crate::plugins::PluginRegistry::OperitPlugin;

pub struct WorkflowLifecyclePlugin;

impl OperitPlugin for WorkflowLifecyclePlugin {
    fn id(&self) -> &str {
        "builtin.workflow.lifecycle"
    }

    fn register(&self) {}
}
