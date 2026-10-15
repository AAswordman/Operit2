use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use operit_host_api::FileSystemHost;
use operit_plugin_sdk::toolpkg::{ToolPkgParser::ToolPkgArchiveParser, ToolPkgProtection};

type TextResources = Arc<BTreeMap<String, String>>;
type PackageResources = Arc<Mutex<Option<CachedResources>>>;

/// Retains requested texts and an optional complete snapshot for one package revision.
struct CachedResources {
    signature: String,
    texts: BTreeMap<String, String>,
    snapshot: Option<TextResources>,
}

/// Shares revision-scoped text resources, never JavaScript state, across package callers.
#[derive(Default)]
pub(super) struct ToolPkgTextResourceCache {
    packages: Mutex<BTreeMap<String, PackageResources>>,
}

impl ToolPkgTextResourceCache {
    /// Captures a package slot so invalidation also detaches requests already loading.
    fn package_slot(&self, package: &str) -> Result<PackageResources, String> {
        Ok(self
            .packages
            .lock()
            .map_err(|error| error.to_string())?
            .entry(package.into())
            .or_default()
            .clone())
    }

    /// Loads the complete snapshot only for callers that explicitly request all resources.
    pub(super) fn get_or_load(
        &self,
        package: &str,
        signature: &str,
        load: impl FnOnce() -> Result<BTreeMap<String, String>, String>,
    ) -> Result<(TextResources, bool), String> {
        let slot = self.package_slot(package)?;
        let mut entry = slot.lock().map_err(|error| error.to_string())?;
        if let Some(cached) = &*entry {
            if cached.signature == signature {
                if let Some(resources) = &cached.snapshot {
                    return Ok((resources.clone(), true));
                }
            }
        }
        let resources = Arc::new(load()?);
        *entry = Some(CachedResources {
            signature: signature.into(),
            texts: BTreeMap::new(),
            snapshot: Some(resources.clone()),
        });
        Ok((resources, false))
    }

    /// Loads exactly one requested text while sharing revision and invalidation with full snapshots.
    pub(super) fn get_or_load_text(
        &self,
        package: &str,
        signature: &str,
        path: &str,
        load: impl FnOnce() -> Result<String, String>,
    ) -> Result<(String, bool), String> {
        let slot = self.package_slot(package)?;
        let mut entry = slot.lock().map_err(|error| error.to_string())?;
        if let Some(cached) = &*entry {
            if cached.signature == signature {
                if let Some(resources) = &cached.snapshot {
                    let text = resources.get(path).ok_or_else(|| {
                        format!("ToolPkg text resource is unavailable: {package}/{path}")
                    })?;
                    return Ok((text.clone(), true));
                }
                if let Some(text) = cached.texts.get(path) {
                    return Ok((text.clone(), true));
                }
            }
        }
        let text = load()?;
        if !entry.as_ref().is_some_and(|cached| cached.signature == signature) {
            *entry = Some(CachedResources {
                signature: signature.into(),
                texts: BTreeMap::new(),
                snapshot: None,
            });
        }
        entry
            .as_mut()
            .expect("loaded revision must be installed")
            .texts
            .insert(path.into(), text.clone());
        Ok((text, false))
    }

    /// Detaches in-flight loads too, so they cannot republish an invalidated snapshot.
    pub(super) fn invalidate(&self, package: &str) {
        self.packages
            .lock()
            .expect("text resource cache mutex poisoned")
            .remove(package);
    }

    /// Removes every cached read form for packages no longer owned by the registry.
    pub(super) fn retain_packages(&self, packages: &BTreeSet<String>) {
        self.packages
            .lock()
            .expect("text resource cache mutex poisoned")
            .retain(|name, _| packages.contains(name));
    }

    /// Invalidates both individual texts and full snapshots during registry reload.
    pub(super) fn clear(&self) {
        self.packages
            .lock()
            .expect("text resource cache mutex poisoned")
            .clear();
    }
}

/// Reads from one validated cache directory without reentering package cache validation.
pub(super) fn read_text_resources(
    host: &dyn FileSystemHost,
    directory: &Path,
) -> Result<BTreeMap<String, String>, String> {
    fn collect(
        host: &dyn FileSystemHost,
        root: &Path,
        directory: &Path,
        resources: &mut BTreeMap<String, String>,
    ) -> Result<(), String> {
        for entry in host
            .listFiles(&directory.to_string_lossy())
            .map_err(|error| error.to_string())?
        {
            if entry.name.is_empty()
                || entry.name == "."
                || entry.name == ".."
                || entry.name.contains(['/', '\\'])
            {
                return Err(format!("Invalid ToolPkg resource name: {}", entry.name));
            }
            let path = directory.join(&entry.name);
            if entry.isDirectory {
                collect(host, root, &path, resources)?;
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            if relative == super::TOOLPKG_CACHE_SIGNATURE_FILE {
                continue;
            }
            let normalized = ToolPkgArchiveParser::normalizeResourcePath(&relative)
                .ok_or_else(|| format!("Invalid ToolPkg resource path: {relative}"))?;
            let bytes = host
                .readFileBytes(&path.to_string_lossy())
                .map_err(|error| format!("Cannot read ToolPkg resource {relative}: {error}"))?;
            let bytes = ToolPkgProtection::decryptIfNeeded(&bytes)
                .map_err(|error| format!("Cannot decrypt ToolPkg resource {relative}: {error}"))?;
            if let Ok(text) = String::from_utf8(bytes) {
                resources.insert(normalized.to_ascii_lowercase(), text);
            }
        }
        Ok(())
    }
    let mut resources = BTreeMap::new();
    collect(host, directory, directory, &mut resources)?;
    Ok(resources)
}

#[cfg(test)]
#[path = "tests/ToolPkgTextResourceCacheTests.rs"]
mod tests;
