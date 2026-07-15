#[cfg(feature = "embedded-web-access")]
use std::fs;
#[cfg(feature = "embedded-web-access")]
use std::path::Path;
use std::path::PathBuf;

#[cfg(feature = "embedded-web-access")]
use include_dir::{include_dir, Dir};

#[cfg(feature = "embedded-web-access")]
static WEB_ACCESS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../web_access/build/bundle");
#[cfg(feature = "embedded-web-access")]
const WEB_ACCESS_VERSION_FILE: &str = "web_access_version.json";

/// Materializes the embedded Web Access bundle into the client data directory.
#[cfg(feature = "embedded-web-access")]
pub(crate) fn materialize_web_access_bundle() -> Result<PathBuf, String> {
    let target = crate::client_paths::link_host_web_access_bundle_dir();
    let embedded_version = embedded_web_access_version()?;
    if is_materialized_web_access_current(&target, embedded_version)? {
        return Ok(target);
    }
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&target).map_err(|error| error.to_string())?;
    materialize_dir(&WEB_ACCESS_DIR, &target)?;
    Ok(target)
}

/// Reports that this binary was compiled without embedded Web Access assets.
#[cfg(not(feature = "embedded-web-access"))]
pub(crate) fn materialize_web_access_bundle() -> Result<PathBuf, String> {
    Err(
        "This operit2 build does not include the Web Access bundle. Pass --web-root <path> to operit2 cli web open."
            .to_string(),
    )
}

/// Reads the embedded Web Access version manifest bytes.
#[cfg(feature = "embedded-web-access")]
fn embedded_web_access_version() -> Result<&'static [u8], String> {
    let file = WEB_ACCESS_DIR
        .get_file(WEB_ACCESS_VERSION_FILE)
        .ok_or_else(|| "embedded Web Access version manifest is missing".to_string())?;
    Ok(file.contents())
}

/// Returns whether the materialized Web Access bundle matches the embedded manifest.
#[cfg(feature = "embedded-web-access")]
fn is_materialized_web_access_current(
    target: &Path,
    embedded_version: &[u8],
) -> Result<bool, String> {
    let version_path = target.join(WEB_ACCESS_VERSION_FILE);
    if !version_path.is_file() {
        return Ok(false);
    }
    let current_version = fs::read(version_path).map_err(|error| error.to_string())?;
    Ok(current_version == embedded_version)
}

/// Copies one embedded directory tree to a filesystem target directory.
#[cfg(feature = "embedded-web-access")]
fn materialize_dir(dir: &Dir<'_>, target: &Path) -> Result<(), String> {
    for file in dir.files() {
        let destination = target.join(file.path());
        let parent = destination
            .parent()
            .ok_or_else(|| format!("invalid bundled web asset path: {}", file.path().display()))?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        fs::write(destination, file.contents()).map_err(|error| error.to_string())?;
    }
    for child in dir.dirs() {
        materialize_dir(child, target)?;
    }
    Ok(())
}
