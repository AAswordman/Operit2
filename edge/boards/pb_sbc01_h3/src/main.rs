#![allow(non_snake_case)]

#[cfg(target_os = "linux")]
mod app;
#[cfg(target_os = "linux")]
mod config;

#[cfg(target_os = "linux")]
use std::process::ExitCode;

/// Starts the PB_SBC01_H3 hardware controller on Linux.
#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> ExitCode {
    match app::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Stops non-Linux targets from launching the PB_SBC01_H3 controller.
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("operit-pb-sbc01-h3 requires Linux");
    std::process::exit(1);
}
