use std::env;
use std::path::PathBuf;

pub(crate) fn client_root_dir() -> PathBuf {
    platform_files_root_dir().join("client")
}

pub(crate) fn access_dir() -> PathBuf {
    client_root_dir().join("access")
}

pub(crate) fn link_sessions_path() -> PathBuf {
    access_dir().join("link_sessions.json")
}

pub(crate) fn link_server_sessions_path() -> PathBuf {
    access_dir().join("link_server_sessions.json")
}

pub(crate) fn web_access_config_path() -> PathBuf {
    client_root_dir().join("web_access").join("web_access.json")
}

pub(crate) fn web_access_state_path() -> PathBuf {
    client_root_dir()
        .join("web_access")
        .join("web_access_state.json")
}

pub(crate) fn web_access_bundle_dir() -> PathBuf {
    client_root_dir().join("web_access").join("flutter_web")
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
