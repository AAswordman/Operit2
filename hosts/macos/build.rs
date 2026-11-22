use std::{env, path::PathBuf, process::Command};

/// Builds the native ScreenCaptureKit preview alongside the macOS host, without runtime compilation.
fn main() {
    println!("cargo:rerun-if-changed=src/WindowPreview.swift");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") { return; }
    let architecture = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "aarch64" => "arm64", "x86_64" => "x86_64", other => panic!("Unsupported macOS preview architecture: {other}"),
    };
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("operit-window-preview");
    let status = Command::new("xcrun").args(["swiftc", "-parse-as-library", "-swift-version", "5", "-O", "-target",
        &format!("{architecture}-apple-macos12.3"), "src/WindowPreview.swift", "-o"])
        .arg(output).status().expect("The macOS host requires the Xcode Swift toolchain for native window previews");
    assert!(status.success(), "Native macOS window preview compilation failed");
}
