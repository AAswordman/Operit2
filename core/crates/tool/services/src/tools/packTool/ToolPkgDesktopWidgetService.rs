use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use super::ToolPkgComposeDslProgramStore::ToolPkgComposeDslProgramStore;
use super::ToolPkgComposeDslSession::{
    ToolPkgComposeDslCommand, ToolPkgComposeDslEventStream, ToolPkgComposeDslSession,
};
use operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgDesktopWidget;
use operit_util::stream::ReverseStream::ReverseStream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::RuntimePackageManager::RuntimePackageManager;

/// Delivers a desktop widget snapshot as a typed object through unchanged CoreLink.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgDesktopWidgetSnapshot {
    pub widget: ToolPkgDesktopWidget,
    pub renderUpdate: crate::tools::packTool::ToolPkgComposeDslProtocol::ToolPkgComposeDslTransaction,
}

static NEXT_RENDER_ID: AtomicU64 = AtomicU64::new(1);

/// Releases the widget's execution ownership on success, failure, or cancellation.
struct ExecutionOwner {
    manager: operit_plugin_sdk::toolpkg::ToolPkgManager::ToolPkgManager,
    session: Option<ToolPkgComposeDslSession>,
    context_key: String,
    package_name: String,
}

impl Drop for ExecutionOwner {
    /// Balances the acquisition even when the host cancels an in-flight refresh.
    fn drop(&mut self) {
        if let Some(session) = &self.session {
            session.close();
        }
        self.manager
            .releaseToolPkgExecutionEngine(&self.context_key, &self.package_name);
    }
}

/// Executes a widget's render route using shared runtime services and no platform branches.
pub(super) fn render(
    manager: &RuntimePackageManager,
    package_name: &str,
    widget_id: &str,
    instance_id: &str,
    use_english: bool,
) -> impl std::future::Future<Output = Result<ToolPkgDesktopWidgetSnapshot, String>> + 'static {
    let engines = manager.toolPkgManager().clone();
    let prepared = (|| {
        if instance_id.trim().is_empty() {
            return Err("Desktop widget instanceId must not be empty".into());
        }
        let widget = manager
            .getToolPkgDesktopWidgets(use_english)
            .into_iter()
            .find(|widget| {
                widget.containerPackageName == package_name && widget.widgetId == widget_id
            })
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
            .getToolPkgComposeDslScript(package_name, Some(&route.uiModuleId));
        let render_id = NEXT_RENDER_ID.fetch_add(1, Ordering::Relaxed);
        let context_key = format!(
            "toolpkg_widget:{}",
            json!([package_name, widget_id, instance_id, render_id])
        );
        let options: BTreeMap<String, Value> = BTreeMap::from([
            ("packageName".into(), json!(package_name)),
            ("toolPkgId".into(), json!(widget.toolPkgId)),
            ("uiModuleId".into(), json!(route.uiModuleId)),
            ("routeInstanceId".into(), json!(instance_id)),
            ("executionContextKey".into(), json!(context_key)),
            ("__operit_script_screen".into(), json!(route.screen)),
            ("moduleSpec".into(), json!(route.moduleSpec)),
        ]);
        let resources = manager.toolPkgTextResources(package_name);
        Ok::<_, String>((
            widget,
            script,
            options,
            context_key,
            package_name.to_string(),
            resources,
        ))
    })();
    async move {
        let (widget, script, options, context_key, package_name, resources) = prepared?;
        let script = script.await.ok_or("Desktop widget script was not found")?;
        let resources = resources.await?;
        engines
            .acquireToolPkgExecutionEngineAsync(&context_key, &package_name)
            .await?;
        let mut owner = ExecutionOwner {
            manager: engines,
            session: None,
            context_key: context_key.clone(),
            package_name: package_name.clone(),
        };
        let engine = owner
            .manager
            .findToolPkgExecutionEngine(&context_key, &package_name)
            .ok_or("Desktop widget execution context has been released")?;
        let session = ToolPkgComposeDslSession::new(engine, resources);
        owner.session = Some(session.clone());
        let mut updates = session.updates()?;
        let (mut sender, commands) = ReverseStream::channel();
        let rendering = async {
            let work = async {
                let mut store = ToolPkgComposeDslProgramStore::default();
                sender
                    .send(ToolPkgComposeDslCommand {
                        requestId: "render".into(),
                        operation: "render".into(),
                        script: Some(script),
                        actionId: None,
                        payload: None,
                        runtimeOptions: options.clone(),
                        envOverrides: BTreeMap::new(),
                    })
                    .await?;
                collect_command(&mut updates, &mut store, "render").await?;
                let on_load = store
                    .view(store.root()?)?
                    .props
                    .get("onLoad")
                    .and_then(|value| value.get("__actionId"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if let Some(action_id) = on_load {
                    sender
                        .send(ToolPkgComposeDslCommand {
                            requestId: "load".into(),
                            operation: "action".into(),
                            script: None,
                            actionId: Some(action_id),
                            payload: None,
                            runtimeOptions: options,
                            envOverrides: BTreeMap::new(),
                        })
                        .await?;
                    collect_command(&mut updates, &mut store, "load").await?;
                }
                Ok::<_, String>(ToolPkgDesktopWidgetSnapshot {
                    widget,
                    renderUpdate: store.snapshot()?.encode()?,
                })
            }
            .await;
            sender.close();
            work
        };
        let (submission, snapshot) = tokio::join!(session.submit(commands), rendering);
        submission?;
        snapshot
    }
}

/// Collects one command's commits through the same session stream used by interactive hosts.
async fn collect_command(
    updates: &mut ToolPkgComposeDslEventStream,
    store: &mut ToolPkgComposeDslProgramStore,
    request: &str,
) -> Result<(), String> {
    loop {
        let event = updates
            .recv()
            .await
            .ok_or("Desktop widget Compose session closed before completion")?;
        if let Some(error) = event.error {
            return Err(error);
        }
        if !event.navigationCommands.is_empty() {
            return Err("Desktop widget rendering cannot navigate".into());
        }
        if let Some(update) = event.update {
            store.apply(update)?;
        }
        if event.requestId == request && event.phase == "complete" {
            return Ok(());
        }
    }
}
