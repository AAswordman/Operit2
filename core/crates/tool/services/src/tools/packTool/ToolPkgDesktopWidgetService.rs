use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use operit_plugin_sdk::toolpkg::ToolPkgComposeDslParser::{ToolPkgComposeDslParser, ToolPkgComposeDslRenderResult};
use operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgDesktopWidget;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::RuntimePackageManager::RuntimePackageManager;

/// Delivers a desktop widget snapshot as a typed object through unchanged CoreLink.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgDesktopWidgetSnapshot {
    pub widget: ToolPkgDesktopWidget,
    pub renderResult: ToolPkgComposeDslRenderResult,
}

static NEXT_RENDER_ID: AtomicU64 = AtomicU64::new(1);

/// Releases the widget's execution ownership on success, failure, or cancellation.
struct ExecutionOwner<'a> {
    manager: &'a RuntimePackageManager,
    context_key: String,
    package_name: String,
}

impl Drop for ExecutionOwner<'_> {
    /// Balances the acquisition even when the host cancels an in-flight refresh.
    fn drop(&mut self) {
        self.manager
            .releaseToolPkgExecutionEngine(&self.context_key, &self.package_name);
    }
}

/// Executes a widget's render route using shared runtime services and no platform branches.
pub(super) async fn render(
    manager: &RuntimePackageManager,
    package_name: &str,
    widget_id: &str,
    instance_id: &str,
    use_english: bool,
) -> Result<ToolPkgDesktopWidgetSnapshot, String> {
    if instance_id.trim().is_empty() {
        return Err("Desktop widget instanceId must not be empty".into());
    }
    let widget = manager
        .getToolPkgDesktopWidgets(use_english)
        .into_iter()
        .find(|widget| widget.containerPackageName == package_name && widget.widgetId == widget_id)
        .ok_or_else(|| {
            format!("Desktop widget is not registered or enabled: {package_name}/{widget_id}")
        })?;
    let route = manager
        .getToolPkgUiRoutes("compose_dsl", use_english)
        .into_iter()
        .find(|route| {
            route.containerPackageName == package_name && route.routeId == widget.renderRouteId
        })
        .ok_or_else(|| {
            format!(
                "Desktop widget render route was not found: {}",
                widget.renderRouteId
            )
        })?;
    let script = manager
        .getToolPkgComposeDslScript(package_name, Some(&route.uiModuleId))
        .ok_or_else(|| format!("Desktop widget script was not found: {}", route.uiModuleId))?;
    let resources = manager.toolPkgTextResources(package_name)?;
    let render_id = NEXT_RENDER_ID.fetch_add(1, Ordering::Relaxed);
    let context_key = format!(
        "toolpkg_widget:{}",
        json!([package_name, widget_id, instance_id, render_id])
    );
    let mut options: BTreeMap<String, Value> = BTreeMap::from([
        ("packageName".into(), json!(package_name)),
        ("toolPkgId".into(), json!(widget.toolPkgId)),
        ("uiModuleId".into(), json!(route.uiModuleId)),
        ("routeInstanceId".into(), json!(instance_id)),
        ("executionContextKey".into(), json!(context_key)),
        ("__operit_script_screen".into(), json!(route.screen)),
        ("moduleSpec".into(), json!(route.moduleSpec)),
        ("state".into(), json!({})),
        ("memo".into(), json!({})),
    ]);
    manager.acquireToolPkgExecutionEngine(&context_key, package_name).await;
    let _owner = ExecutionOwner {
        manager,
        context_key: context_key.clone(),
        package_name: package_name.into(),
    };
    let engine = manager.getToolPkgExecutionEngine(&context_key, package_name);
    let initial = engine
        .execute_compose_dsl_script_async(script, options.clone(), BTreeMap::new(), resources)
        .await
        .map_err(|error| error.to_string())?;
    let mut result = decode_render(initial)?;
    let on_load =
        ToolPkgComposeDslParser::extractActionId(result.pointer("/tree/props/onLoad").cloned());
    if let Some(action_id) = on_load {
        options.insert(
            "state".into(),
            result
                .get("state")
                .ok_or("Desktop widget render result is missing state")?
                .clone(),
        );
        options.insert(
            "memo".into(),
            result
                .get("memo")
                .ok_or("Desktop widget render result is missing memo")?
                .clone(),
        );
        let loaded = engine
            .dispatch_compose_dsl_action_result_async(
                action_id,
                None,
                options,
                BTreeMap::new(),
                None,
            )
            .await
            .map_err(|error| error.to_string())?;
        result = decode_render(loaded)?;
    }
    Ok(ToolPkgDesktopWidgetSnapshot { widget, renderResult: serde_json::from_value(result).map_err(|error| error.to_string())? })
}

/// Rejects failed or malformed renders instead of presenting an older widget snapshot.
fn decode_render(raw: Option<Value>) -> Result<Value, String> {
    let raw = raw.ok_or("Desktop widget returned no render result")?;
    let value = raw;
    if value.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(format!("Desktop widget render failed: {value}"));
    }
    if !value.get("tree").is_some_and(Value::is_object) {
        return Err("Desktop widget render result is missing its tree".into());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::decode_render;

    /// Ensures refresh failures cannot masquerade as usable widget content.
    #[test]
    fn rejects_failed_and_missing_trees() {
        assert!(decode_render(None).is_err());
        assert!(decode_render(Some(serde_json::json!({"success": false, "tree": {}}))).is_err());
        assert!(decode_render(Some(serde_json::json!({"success": true}))).is_err());
        assert!(decode_render(Some(
            serde_json::json!({"tree": {"type": "Text", "props": {}, "children": []}})
        ))
        .is_ok());
    }
}
