use crate::data::preferences::ApiPreferences::ApiPreferences;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;

/// Tracks the single total deadline shared by one ToolPkg pre-hook dispatch chain.
pub struct ToolPkgPreHookTimeout {
    deadlineMillis: i64,
}

impl ToolPkgPreHookTimeout {
    /// Creates a deadline from the persisted ToolPkg pre-hook timeout preference.
    pub fn fromPreferences() -> Result<Self, String> {
        let seconds = ApiPreferences::getInstance()
            .getToolPkgPreHookTimeoutSeconds()
            .map_err(|error| format!("Before-hook timeout preference read failed: {error}"))?;
        Ok(Self::fromSeconds(seconds))
    }

    /// Creates a deadline with the supplied whole-second duration.
    pub fn fromSeconds(seconds: i32) -> Self {
        let durationMillis = i64::from(seconds.clamp(1, 60)) * 1000;
        Self {
            deadlineMillis: currentTimeMillis() + durationMillis,
        }
    }

    /// Creates a deadline only for explicitly declared pre-action hook stages.
    pub fn forStage(stage: &str) -> Result<Option<Self>, String> {
        match stage {
            "before_process"
            | "before_prepare_history"
            | "before_finalize_prompt"
            | "before_send_to_model"
            | "before_prepare_summary_prompt"
            | "before_compose_system_prompt"
            | "compose_system_prompt_sections"
            | "build_tool_prompt"
            | "before_compose_tool_prompt"
            | "filter_tool_prompt_items"
            | "filter_tool_call_tools" => Self::fromPreferences().map(Some),
            _ => Ok(None),
        }
    }

    /// Executes a hook under its stage policy and propagates notified deadline failures.
    pub async fn runHook(
        budget: Option<&Self>,
        manager: &RuntimePackageManager,
        packageName: &str,
        functionName: &str,
        event: &str,
        stage: &str,
        hookId: &str,
        functionSource: Option<&str>,
        payload: serde_json::Value,
    ) -> Result<Option<String>, String> {
        match budget {
            Some(budget) => {
                let timeout = budget.remainingForHook(manager, packageName, hookId, stage)?;
                let result = manager
                    .runToolPkgMainHookWithTimeoutMillis(
                        packageName,
                        functionName,
                        event,
                        Some(stage),
                        Some(hookId),
                        functionSource,
                        payload,
                        None,
                        None,
                        None,
                        timeout,
                    )
                    .await?;
                if budget.hasExpired() {
                    return Err(manager.reportToolPkgHookTimeout(packageName, hookId, stage));
                }
                Ok(result)
            }
            None => {
                manager
                    .runToolPkgMainHook(
                        packageName,
                        functionName,
                        event,
                        Some(stage),
                        Some(hookId),
                        functionSource,
                        payload,
                        None,
                        None,
                        None,
                    )
                    .await
            }
        }
    }

    /// Returns the timeout milliseconds derived from the remaining shared deadline.
    #[allow(non_snake_case)]
    pub fn remainingTimeoutMillis(&self) -> Option<u64> {
        let remainingMillis = self.deadlineMillis - currentTimeMillis();
        if remainingMillis <= 0 {
            return None;
        }
        let millis = u64::try_from(remainingMillis)
            .expect("ToolPkg pre-hook remaining timeout must fit into u64 milliseconds");
        Some(millis)
    }

    /// Reports an exhausted before-hook chain before declining to start another handler.
    #[allow(non_snake_case)]
    pub fn remainingForHook(
        &self,
        manager: &RuntimePackageManager,
        packageName: &str,
        hookId: &str,
        event: &str,
    ) -> Result<u64, String> {
        self.remainingTimeoutMillis()
            .ok_or_else(|| manager.reportToolPkgHookTimeout(packageName, hookId, event))
    }

    /// Reports whether the shared pre-hook deadline has elapsed.
    #[allow(non_snake_case)]
    pub fn hasExpired(&self) -> bool {
        currentTimeMillis() >= self.deadlineMillis
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::ToolPkgPreHookTimeout;

    /// Keeps after-event notifications and custom notification stages outside the before-hook deadline policy.
    #[test]
    fn notification_stages_do_not_create_deadlines() {
        for stage in [
            "after_process",
            "after_prepare_history",
            "after_compose_system_prompt",
            "after_compose_tool_prompt",
            "after_generate_summary",
            "custom_notification",
        ] {
            assert!(
                ToolPkgPreHookTimeout::forStage(stage).unwrap().is_none(),
                "{stage}"
            );
        }
    }

    /// Verifies a shared deadline cannot be renewed between hook invocations.
    #[test]
    fn shared_deadline_expires_for_all_remaining_hooks() {
        let budget = ToolPkgPreHookTimeout::fromSeconds(1);
        assert!(budget.remainingTimeoutMillis().is_some());

        thread::sleep(Duration::from_millis(1100));

        assert!(budget.hasExpired());
        assert_eq!(budget.remainingTimeoutMillis(), None);
    }
}
