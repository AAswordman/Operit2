#![allow(non_snake_case)]
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use operit_host_api::{HostError, HostResult};

unsafe extern "C" {
    fn operit_store_begin(length: usize) -> i32;
    fn operit_store_write(data: *const u8, length: usize) -> i32;
    fn operit_store_finish() -> i32;
    fn operit_store_abort();
    fn operit_store_revision() -> u32;
}

/// Streams into the inactive flash slot. LVGL adopts a committed package on its own task.
pub fn register(server: &mut EspHttpServer<'static>, token: String) -> HostResult<()> {
    server.fn_handler("/ui/capabilities",Method::Get,move |request| {
        let revision=unsafe{operit_store_revision()};
        let body=format!("{{\"protocol\":1,\"board\":\"ESP32-2432S028\",\"width\":320,\"height\":240,\"maxPackageBytes\":28672,\"revision\":{revision}}}");
        request.into_response(200,Some("OK"),&[("Content-Type","application/json"),("Cache-Control","no-store")])?.write_all(body.as_bytes())?;
        Ok::<(),esp_idf_svc::io::EspIOError>(())
    }).map_err(|e|HostError::new(format!("ui capabilities: {e}")))?;
    server
        .fn_handler("/ui/package", Method::Post, move |mut request| {
            // Use the existing device pairing token. Never allow a cross-origin web form.
            let authorized = request.header("X-Operit-Studio") == Some("1")
                && (token.is_empty()
                    || request.header("Authorization") == Some(format!("Bearer {token}").as_str()));
            let length = request
                .header("Content-Length")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            if !authorized {
                request
                    .into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Device Edge token required")?;
                return Ok::<(), esp_idf_svc::io::EspIOError>(());
            }
            if unsafe { operit_store_begin(length) } != 0 {
                request
                    .into_response(409, Some("Conflict"), &[])?
                    .write_all(b"Runtime missing, upload busy or package too large")?;
                return Ok(());
            }
            let mut remaining = length;
            let mut chunk = [0u8; 512];
            let mut good = true;
            while remaining > 0 {
                let cap = remaining.min(chunk.len());
                match request.read(&mut chunk[..cap]) {
                    Ok(0) | Err(_) => {
                        good = false;
                        break;
                    }
                    Ok(n) => {
                        if unsafe { operit_store_write(chunk.as_ptr(), n) } != 0 {
                            good = false;
                            break;
                        }
                        remaining -= n;
                    }
                }
            }
            if good {
                good = unsafe { operit_store_finish() } == 0;
            }
            if !good {
                unsafe { operit_store_abort() };
            }
            request
                .into_response(
                    if good { 202 } else { 422 },
                    Some(if good { "Accepted" } else { "Invalid package" }),
                    &[("Content-Type", "application/json")],
                )?
                .write_all(if good {
                    b"{\"accepted\":true}"
                } else {
                    b"{\"accepted\":false}"
                })?;
            Ok(())
        })
        .map_err(|e| HostError::new(format!("ui deploy: {e}")))?;
    Ok(())
}
