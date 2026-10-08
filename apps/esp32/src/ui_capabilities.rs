#![allow(non_snake_case)]
use esp_idf_svc::http::{server::EspHttpServer, Method};
use esp_idf_svc::io::Write;
use operit_host_api::{HostError, HostResult};

/// Report what the installed renderer can actually execute. No layout upload
/// route or Flash storage is installed for the fixed text-only renderer.
pub fn register(server: &mut EspHttpServer<'static>) -> HostResult<()> {
    server.fn_handler("/ui/capabilities", Method::Get, |request| {
        request.into_response(200, Some("OK"), &[("Content-Type", "application/json"), ("Cache-Control", "no-store")])?
            .write_all(br#"{"protocol":1,"board":"ESP32-2432S028","width":320,"height":240,"dynamicLayout":false,"imagePreview":false}"#)?;
        Ok::<(), esp_idf_svc::io::EspIOError>(())
    }).map_err(|error| HostError::new(format!("UI capabilities: {error}")))?;
    Ok(())
}
