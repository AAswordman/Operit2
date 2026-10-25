//! OpenHarmony host assembly for the Flutter bridge.

use std::path::PathBuf;
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_ohos_native::{
    createRuntimeHostManager, OhosBluetoothHost, OhosManagedRuntimeHost, OhosRuntimeStorageHost,
    OhosTerminalHost,
};
use operit_link::LinkDeviceInfo;

use super::{install_owner_media, BridgeStartup, StartupMetadata};
use crate::FlutterHostAdapters::FlutterWebVisitBridge;
use crate::FlutterOwnerCapabilities::{
    ownerBluetooth, ownerFileOpen, ownerFileShare, ownerScreenshot, ownerSystemOperation,
};

/// Creates OpenHarmony hosts with every owner callback fixed during assembly.
pub(crate) fn create_host_context(startup: &BridgeStartup) -> Result<HostManager, String> {
    let StartupMetadata::OpenHarmonyDevice { systemLanguageCode, .. } = &startup.metadata else {
        return Err("OpenHarmony startup requires its owner device metadata".to_string());
    };
    if systemLanguageCode.trim().is_empty() {
        return Err("OpenHarmony owner system language is empty".to_string());
    }
    let language = systemLanguageCode.clone();
    let terminal = Arc::new(
        OhosTerminalHost::new(startup.runtimeRoot.clone(), startup.workspaceRoot.clone())
            .map_err(|error| error.message)?,
    );
    let managed = Arc::new(OhosManagedRuntimeHost::new(
        terminal.clone(),
        startup.workspaceRoot.clone(),
    ));
    let ocrTerminal = terminal.clone();
    let ocrRoot = startup.runtimeRoot.clone();
    let context = createRuntimeHostManager(
        startup.runtimeRoot.clone(),
        startup.workspaceRoot.clone(),
        Arc::new(FlutterWebVisitBridge::new()),
        Arc::new(ownerFileOpen),
        Arc::new(ownerFileShare),
        Arc::new(move || Ok(language.clone())),
        Arc::new(ownerScreenshot),
        Arc::new(move |path, language, quality| {
            operit_host_ohos_native::documents::recognizeText(
                ocrTerminal.as_ref(),
                &ocrRoot,
                path,
                language,
                quality,
            )
        }),
        Arc::new(ownerSystemOperation::<serde_json::Value>),
        managed,
    )
    .withTerminalHost(terminal)
    .withBluetoothHost(Arc::new(OhosBluetoothHost::fromController(Arc::new(
        ownerBluetooth,
    ))));
    Ok(install_owner_media(context, false))
}

/// Reads owner-supplied identity before the runtime interaction channel exists.
pub(crate) fn startup_device_info(
    context: &HostManager,
    metadata: &StartupMetadata,
) -> Result<LinkDeviceInfo, String> {
    let StartupMetadata::OpenHarmonyDevice { model, .. } = metadata else {
        return Err("OpenHarmony startup requires its owner device metadata".to_string());
    };
    if model.trim().is_empty() {
        return Err("OpenHarmony owner device model is empty".to_string());
    }
    Ok(LinkDeviceInfo {
        platform: context.hostEnvironment.id.clone(),
        model: model.clone(),
        pairingIncarnation: None,
    })
}

/// OpenHarmony receives storage roots from its application owner.
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Err("The OpenHarmony owner must supply its application storage roots".to_string())
}

/// OpenHarmony has no process-global bridge registration to release here.
pub(crate) fn release_host() {}

impl crate::OperitFlutterBridge {
    /// Starts OpenHarmony using storage roots and device metadata supplied by its owner.
    pub(crate) fn new_with_storage_roots(
        runtimeRoot: PathBuf,
        workspaceRoot: PathBuf,
        systemLanguageCode: String,
        model: String,
    ) -> Result<Self, String> {
        crate::PlatformRuntimeFactory::startBridge(BridgeStartup {
            runtimeRoot,
            workspaceRoot,
            metadata: StartupMetadata::OpenHarmonyDevice { model, systemLanguageCode },
        })
    }
}

/// Creates only the platform secret-store adapter without starting Core or owner services.
pub(crate) fn rescue_secret_store(
    default_runtime: &std::path::Path,
) -> Result<Option<std::sync::Arc<dyn operit_host_api::HostSecretStore>>, String> {
    Ok(None)
}

/// Retains the OHOS owner handle before a platform worker starts blocking document work.
#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_ohos_retain(
    handle: *const crate::OperitFlutterBridge,
) -> *const crate::OperitFlutterBridge {
    if !handle.is_null() {
        Arc::increment_strong_count(handle);
    }
    handle
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PdfRenderRequest {
    input: PathBuf,
    outputDirectory: PathBuf,
    pages: Option<Vec<u32>>,
    scale: f64,
}

/// Renders PDF pages on the N-API worker through the runtime's existing terminal host.
#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_ohos_render_pdf(
    handle: *const crate::OperitFlutterBridge,
    request: *const std::ffi::c_char,
) -> *mut std::ffi::c_char {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> Result<Vec<String>, String> {
            let bridge = handle.as_ref().ok_or("OHOS runtime handle is null")?;
            if request.is_null() {
                return Err("PDF request is null".to_string());
            }
            let request: PdfRenderRequest = serde_json::from_str(
                std::ffi::CStr::from_ptr(request)
                    .to_str()
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            let terminal = bridge
                .localCore
                .hostManager()
                .terminalHost
                .as_ref()
                .ok_or("OHOS terminal host is missing")?;
            operit_host_ohos_native::documents::renderPdf(
                terminal.as_ref(),
                &request.input,
                &request.outputDirectory,
                request.pages,
                request.scale,
            )
            .map_err(|error| error.message)
        },
    ));
    let response = match result {
        Ok(Ok(pages)) => serde_json::json!({"ok": true, "pages": pages}),
        Ok(Err(error)) => serde_json::json!({"ok": false, "error": error}),
        Err(_) => serde_json::json!({"ok": false, "error": "PDF renderer panicked"}),
    };
    std::ffi::CString::new(response.to_string())
        .expect("JSON contains no raw NUL")
        .into_raw()
}
