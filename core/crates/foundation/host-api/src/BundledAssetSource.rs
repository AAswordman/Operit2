//! Build-selected asset storage: CLI bytes or application-owned packaged resources.

use std::borrow::Cow;

/// Describes immutable bytes without eagerly opening application resources.
#[derive(Clone, Copy)]
pub enum BundledAssetSource {
    Embedded(&'static [u8]),
    Packaged { key: &'static str, length: usize },
}

impl BundledAssetSource {
    /// Returns the build-time length without reading the resource.
    pub fn len(&self) -> usize {
        match self {
            Self::Embedded(bytes) => bytes.len(),
            Self::Packaged { length, .. } => *length,
        }
    }

    /// Borrows CLI bytes or reads exactly one resource through the application Host.
    pub fn read(&self) -> Result<Cow<'static, [u8]>, String> {
        match self {
            Self::Embedded(bytes) => Ok(Cow::Borrowed(bytes)),
            Self::Packaged { key, length } => {
                crate::BundledResourceHost::validateResourceKey(key).map_err(|error| error.to_string())?;
                let bytes = crate::HostManager::defaultBundledResourceHost()?
                    .read(key).map_err(|error| format!("Cannot read bundled resource {key}: {error}"))?;
                if bytes.len() != *length {
                    return Err(format!("Bundled resource length mismatch: {key}"));
                }
                Ok(Cow::Owned(bytes))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    use crate::{HostError, HostResult, BundledResourceHost::BundledResourceHost};

    struct Resources(Arc<AtomicUsize>);

    impl BundledResourceHost for Resources {
        /// Counts actual reads and preserves missing-resource failures.
        fn read(&self, key: &str) -> HostResult<Vec<u8>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            match key {
                "plugins/demo.toolpkg" => Ok(vec![1, 2, 3]),
                _ => Err(HostError::new("Resource not found")),
            }
        }
    }

    /// Keeps embedded data borrowed and packaged data lazy, with strict keys and lengths.
    #[test]
    fn resources_are_lazy_and_failures_remain_errors() {
        let embedded = BundledAssetSource::Embedded(&[4, 5]);
        assert!(matches!(embedded.read().unwrap(), Cow::Borrowed(&[4, 5])));
        let reads = Arc::new(AtomicUsize::new(0));
        crate::HostManager::setDefaultBundledResourceHost(Arc::new(Resources(reads.clone())));
        let asset = BundledAssetSource::Packaged { key: "plugins/demo.toolpkg", length: 3 };
        assert_eq!(asset.len(), 3);
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        assert_eq!(asset.read().unwrap().as_ref(), &[1, 2, 3]);
        assert!(BundledAssetSource::Packaged { key: "plugins/demo.toolpkg", length: 4 }.read().is_err());
        assert!(BundledAssetSource::Packaged { key: "plugins/missing.toolpkg", length: 0 }.read().is_err());
        assert_eq!(reads.load(Ordering::SeqCst), 3);
        for key in ["", "../secret", "/absolute", "a/../b", "a//b", "a\\b", "a:b"] {
            assert!(BundledAssetSource::Packaged { key, length: 0 }.read().is_err());
        }
        assert_eq!(reads.load(Ordering::SeqCst), 3);
    }
}
