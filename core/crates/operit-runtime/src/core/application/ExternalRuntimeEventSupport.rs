#![cfg(not(target_arch = "wasm32"))]
#![allow(non_snake_case)]

use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::AIToolHandler::AIToolHandler;
use operit_host_api::{
    ExternalRuntimeEvent, ExternalRuntimeEventBusConfig, ExternalRuntimeEventRegistration,
    HostError, HostResult,
};

pub const TOOLPKG_HOST_EVENT: &str = "toolpkg.host_event";
pub const TOOLPKG_PACKAGES_CHANGED_EVENT: &str = "toolpkg.packages.changed";

pub fn startExternalRuntimeEventSupport(
    context: OperitApplicationContext,
    processKind: impl Into<String>,
) -> Result<Box<dyn ExternalRuntimeEventRegistration>, String> {
    let externalRuntimeEventHost = context
        .externalRuntimeEventHost
        .clone()
        .ok_or_else(|| "ExternalRuntimeEventHost is required".to_string())?;
    let handlerContext = context.clone();
    externalRuntimeEventHost
        .startExternalRuntimeEventBus(
            ExternalRuntimeEventBusConfig {
                processKind: processKind.into(),
                capabilities: vec![
                    TOOLPKG_PACKAGES_CHANGED_EVENT.to_string(),
                    TOOLPKG_HOST_EVENT.to_string(),
                ],
                pollInterval: Duration::from_millis(250),
            },
            Arc::new(move |event| handleExternalRuntimeEvent(handlerContext.clone(), event)),
        )
        .map_err(|error| error.to_string())
}

pub fn handleExternalRuntimeEvent(
    context: OperitApplicationContext,
    event: ExternalRuntimeEvent,
) -> HostResult<serde_json::Value> {
    match event.name.as_str() {
        TOOLPKG_PACKAGES_CHANGED_EVENT => handleToolPkgPackagesChanged(context, event),
        TOOLPKG_HOST_EVENT => handleToolPkgHostEvent(context, event),
        _ => Err(HostError::new(format!(
            "unsupported external runtime event: {}",
            event.name
        ))),
    }
}

fn handleToolPkgPackagesChanged(
    context: OperitApplicationContext,
    event: ExternalRuntimeEvent,
) -> HostResult<serde_json::Value> {
    let packageManager = AIToolHandler::getInstance(context).getOrCreatePackageManager();
    packageManager
        .lock()
        .map_err(|error| HostError::new(format!("package manager mutex poisoned: {error}")))?
        .loadAvailablePackages();
    Ok(serde_json::json!({
        "event": event.name,
        "packageManagerReloaded": true,
    }))
}

fn handleToolPkgHostEvent(
    _context: OperitApplicationContext,
    event: ExternalRuntimeEvent,
) -> HostResult<serde_json::Value> {
    let source = event
        .payload
        .get("source")
        .and_then(Value::as_str)
        .ok_or_else(|| HostError::new("toolpkg host event source is required"))?
        .to_string();
    let payload = event
        .payload
        .get("payload")
        .cloned()
        .ok_or_else(|| HostError::new("toolpkg host event payload is required"))?;
    crate::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge::dispatchHostEvent(&source, payload);
    Ok(serde_json::json!({
        "event": event.name,
        "source": source,
    }))
}
