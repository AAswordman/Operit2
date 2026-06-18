use std::env;
use std::path::PathBuf;

pub(crate) fn client_root_dir() -> PathBuf {
    platform_files_root_dir().join("client")
}

pub(crate) fn link_dir() -> PathBuf {
    client_root_dir().join("link")
}

pub(crate) fn link_sessions_path() -> PathBuf {
    link_dir().join("outbound_sessions.json")
}

pub(crate) fn link_server_sessions_path() -> PathBuf {
    link_dir().join("inbound_sessions.json")
}

pub(crate) fn link_host_config_path() -> PathBuf {
    link_dir().join("host_config.json")
}

pub(crate) fn link_host_state_path() -> PathBuf {
    link_dir().join("host_state.json")
}

pub(crate) fn link_host_device_id_path() -> PathBuf {
    link_dir().join("host_device_id")
}

pub(crate) fn link_host_web_access_bundle_dir() -> PathBuf {
    link_dir().join("web_access_bundle")
}

#[cfg(windows)]
fn platform_files_root_dir() -> PathBuf {
    let appdata =
        env::var_os("APPDATA").expect("APPDATA is required for Operit2 CLI client storage");
    PathBuf::from(appdata).join("app.operit").join("Operit2")
}

#[cfg(target_os = "linux")]
fn platform_files_root_dir() -> PathBuf {
    let home = env::var_os("HOME").expect("HOME is required for Operit2 CLI client storage");
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("app.operit")
        .join("Operit2")
}

#[cfg(target_os = "macos")]
fn platform_files_root_dir() -> PathBuf {
    let home = env::var_os("HOME").expect("HOME is required for Operit2 CLI client storage");
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("app.operit")
        .join("Operit2")
}
