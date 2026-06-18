use std::fs;
use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir};

static WEB_ACCESS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../flutter/app/build/web");

pub(crate) fn materialize_web_access_bundle() -> Result<PathBuf, String> {
    let target = crate::client_paths::link_host_web_access_bundle_dir();
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&target).map_err(|error| error.to_string())?;
    materialize_dir(&WEB_ACCESS_DIR, &target)?;
    Ok(target)
}

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
