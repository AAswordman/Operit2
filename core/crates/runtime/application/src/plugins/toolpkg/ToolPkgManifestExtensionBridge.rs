use std::collections::BTreeSet;
use std::sync::{Mutex, OnceLock};

use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgManifestExtensionRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::{
    ToolPkgContainerRuntime, ToolPkgManifestRequirement,
};
use operit_util::GithubReleaseUtil::GithubReleaseUtil;
use operit_util::ChainLogger::{self, PLUGIN_CHAIN};
use serde_json::{Map, Value};

use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;

static MANIFEST_EXTENSION_HANDLERS: OnceLock<Mutex<Vec<ToolPkgManifestExtensionRegistration>>> =
    OnceLock::new();
static DISPATCHED_MANIFEST_EXTENSIONS: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();

/// Bridges manifest extension declarations into their required ToolPkg owners.
pub struct ToolPkgManifestExtensionBridge;

impl ToolPkgManifestExtensionBridge {
    /// Synchronizes active handlers and dispatches each newly visible manifest extension once.
    #[allow(non_snake_case)]
    pub fn syncAndDispatch(
        runtime: &ToolPkgBridgeRuntime,
        activeContainers: &[ToolPkgContainerRuntime],
    ) {
        let handlers = activeContainers
            .iter()
            .flat_map(|container| {
                container
                    .manifestExtensionHandlers
                    .iter()
                    .map(|handler| ToolPkgManifestExtensionRegistration {
                        containerPackageName: container.packageName.clone(),
                        extensionKey: handler.key.clone(),
                        functionName: handler.function.clone(),
                        functionSource: handler.functionSource.clone(),
                    })
            })
            .collect::<Vec<_>>();
        *MANIFEST_EXTENSION_HANDLERS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("toolpkg manifest extension mutex poisoned") = handlers.clone();

        let targets = activeContainers;
        for handler in handlers {
            for target in targets.iter().filter(|target| {
                target.packageName != handler.containerPackageName
                    && target.requires.iter().any(|requirement| {
                        requirementMatchesHandler(requirement, &handler.containerPackageName, &target.version)
                    })
                    && target
                        .manifestExtensions
                        .contains_key(&handler.extensionKey)
            }) {
                dispatchManifestExtension(runtime, &handler, target);
            }
        }
    }
}

/// Dispatches one manifest extension to its declared prerequisite handler.
#[allow(non_snake_case)]
fn dispatchManifestExtension(
    runtime: &ToolPkgBridgeRuntime,
    handler: &ToolPkgManifestExtensionRegistration,
    target: &ToolPkgContainerRuntime,
) {
    let runtime = runtime.to_owned();
    let handler = handler.to_owned();
    let target = target.to_owned();
    super::ToolPkgHookBridgeSupport::scheduleToolPkgNotification(
        "operit-toolpkg-notification",
        move || {
            Box::pin(async move {
                let runtime = &runtime;
                let handler = &handler;
                let target = &target;
                let extension = target
                    .manifestExtensions
                    .get(&handler.extensionKey)
                    .cloned()
                    .expect("manifest extension target must contain the selected key");
                let dispatchKey = format!(
                    "{}:{}:{}:{}:{}:{}:{}:{}",
                    handler.containerPackageName,
                    handler.extensionKey,
                    handler.functionName,
                    handler.functionSource.as_deref().unwrap_or_default(),
                    target.packageName,
                    target.version,
                    target.sourcePath,
                    serde_json::to_string(&extension)
                        .expect("manifest extension must remain JSON serializable")
                );
                {
                    let dispatched = DISPATCHED_MANIFEST_EXTENSIONS
                        .get_or_init(|| Mutex::new(BTreeSet::new()))
                        .lock()
                        .expect("toolpkg manifest extension dispatch mutex poisoned");
                    if dispatched.contains(&dispatchKey) {
                        return;
                    }
                }

                let mut payload = Map::new();
                payload.insert(
                    "extensionKey".to_string(),
                    Value::String(handler.extensionKey.clone()),
                );
                payload.insert(
                    "sourceToolPkgId".to_string(),
                    Value::String(target.packageName.clone()),
                );
                payload.insert(
                    "sourceVersion".to_string(),
                    Value::String(target.version.clone()),
                );
                payload.insert("extension".to_string(), extension);
                payload.insert(
                    "manifestExtensions".to_string(),
                    serde_json::to_value(&target.manifestExtensions)
                        .expect("manifest extensions must remain JSON serializable"),
                );

                let manager = runtime.package_manager();
                ChainLogger::info(
                    PLUGIN_CHAIN,
                    "plugin.toolpkg.manifest_extension.dispatch.start",
                    &[
                        ("handler", handler.containerPackageName.clone()),
                        ("extension", handler.extensionKey.clone()),
                        ("source", target.packageName.clone()),
                    ],
                );
                match manager
                    .runToolPkgMainHook(
                        &handler.containerPackageName,
                        &handler.functionName,
                        "toolpkg_manifest_extension",
                        Some("toolpkg_manifest_extension"),
                        Some(&handler.extensionKey),
                        handler.functionSource.as_deref(),
                        Value::Object(payload),
                        None,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(_) => {
                        DISPATCHED_MANIFEST_EXTENSIONS
                            .get_or_init(|| Mutex::new(BTreeSet::new()))
                            .lock()
                            .expect("toolpkg manifest extension dispatch mutex poisoned")
                            .insert(dispatchKey);
                        ChainLogger::info(
                            PLUGIN_CHAIN,
                            "plugin.toolpkg.manifest_extension.dispatch.done",
                            &[
                                ("handler", handler.containerPackageName.clone()),
                                ("extension", handler.extensionKey.clone()),
                                ("source", target.packageName.clone()),
                            ],
                        );
                    }
                    Err(error) => ChainLogger::error(
                        PLUGIN_CHAIN,
                        "plugin.toolpkg.manifest_extension.dispatch.error",
                        &[
                            ("handler", handler.containerPackageName.clone()),
                            ("extension", handler.extensionKey.clone()),
                            ("source", target.packageName.clone()),
                            ("error", error),
                        ],
                    ),
                }
            })
        },
    );
}

/// Checks one dependency declaration against the required handler package and source version.
#[allow(non_snake_case)]
fn requirementMatchesHandler(
    requirement: &ToolPkgManifestRequirement,
    handlerPackageName: &str,
    sourceVersion: &str,
) -> bool {
    if !requirement.id.eq_ignore_ascii_case(handlerPackageName) {
        return false;
    }
    if let Some(minVersion) = requirement.minVersion.as_deref() {
        let order = match GithubReleaseUtil::compareVersions(sourceVersion, minVersion) {
            Ok(order) => order,
            Err(_) => return false,
        };
        if order < 0 { return false; }
    }
    if let Some(maxVersion) = requirement.maxVersion.as_deref() {
        let order = match GithubReleaseUtil::compareVersions(sourceVersion, maxVersion) {
            Ok(order) => order,
            Err(_) => return false,
        };
        if order > 0 { return false; }
    }
    true
}
