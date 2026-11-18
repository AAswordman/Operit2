//! Platform-owned native filesystem namespaces shared by path ingress and mount resolution.

use crate::HostPlatform;

/// Declares the native path model without inferring it from a path's spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostFileSystemRoots {
    Windows,
    Android,
    Linux,
    Macos,
    Ios,
    Ohos,
    Esp32,
    Browser,
    Unsupported,
}

/// Defines each flat native mount once for ingress, discovery and resolution.
const POSIX_ROOTS: &[(HostFileSystemRoots, &str, &str)] = &[
    (HostFileSystemRoots::Linux, "linux", "/virtual/linux"),
    (HostFileSystemRoots::Macos, "macos", "/virtual/macos"),
    (HostFileSystemRoots::Ios, "ios", "/virtual/ios"),
    (HostFileSystemRoots::Ohos, "ohos", "/virtual/ohos"),
    (HostFileSystemRoots::Esp32, "esp32", "/virtual/esp32"),
];

impl HostFileSystemRoots {
    /// Selects the filesystem layout at the host platform dispatch boundary.
    pub fn current() -> Self {
        Self::forPlatform(&platform::CURRENT)
    }

    /// Defines every platform explicitly, including hosts without native path access.
    pub fn forPlatform(platform: &HostPlatform) -> Self {
        match platform {
            HostPlatform::Windows => Self::Windows,
            HostPlatform::Android => Self::Android,
            HostPlatform::Linux => Self::Linux,
            HostPlatform::Macos => Self::Macos,
            HostPlatform::Ios => Self::Ios,
            HostPlatform::Ohos => Self::Ohos,
            HostPlatform::Esp32 => Self::Esp32,
            HostPlatform::Web => Self::Browser,
            HostPlatform::Other => Self::Unsupported,
        }
    }

    /// Returns the mount accepting native POSIX paths, never a different platform's root.
    pub fn posixVfsRoot(self) -> Result<&'static str, String> {
        match (self, self.posixRoot()) {
            (_, Some((_, root))) => Ok(root),
            (Self::Android, None) => Ok("/virtual/android/root"),
            (Self::Windows, None) => {
                Err("Native Windows paths must include an absolute drive".into())
            }
            (Self::Browser, None) => Err(
                "Browser storage requires an application VFS path or a registered resource mount"
                    .into(),
            ),
            (_, None) => Err("This host does not declare a native filesystem root".into()),
        }
    }

    /// Preserves Android shared-storage aliases only when selecting an Android native directory.
    pub fn nativePosixMount(self, segments: &[&str]) -> Result<(&'static str, usize), String> {
        match (self, segments) {
            (Self::Android, ["sdcard", ..]) => Ok(("/virtual/android/sdcard", 1)),
            (Self::Android, ["storage", "emulated", "0", ..]) => Ok(("/virtual/android/sdcard", 3)),
            _ => Ok((self.posixVfsRoot()?, 0)),
        }
    }

    /// Identifies flat POSIX mounts; Android exposes its root and shared storage separately.
    pub fn posixMountName(self) -> Option<&'static str> {
        self.posixRoot().map(|(name, _)| name)
    }

    /// Recognizes reserved POSIX mount names independently of the active host.
    pub fn isPosixMountName(name: &str) -> bool {
        POSIX_ROOTS.iter().any(|(_, mount, _)| *mount == name)
    }

    /// Reads the native mount from the shared declaration instead of maintaining parallel lists.
    fn posixRoot(self) -> Option<(&'static str, &'static str)> {
        POSIX_ROOTS
            .iter()
            .find(|(platform, _, _)| *platform == self)
            .map(|(_, name, root)| (*name, *root))
    }

    /// Discovers drive roots only for the declared Windows filesystem.
    pub fn windowsDriveExists(self, drive: &str) -> bool {
        self == Self::Windows && std::path::Path::new(&format!("{drive}:/")).exists()
    }

    /// Discovers Android-specific roots only for the declared Android filesystem.
    pub fn androidPathExists(self, path: &str) -> bool {
        self == Self::Android && std::path::Path::new(path).exists()
    }
}

/// Contains build-target dispatch; VFS consumers use the declared layout above.
mod platform {
    use crate::HostPlatform;

    #[cfg(windows)]
    pub(super) const CURRENT: HostPlatform = HostPlatform::Windows;
    #[cfg(target_os = "android")]
    pub(super) const CURRENT: HostPlatform = HostPlatform::Android;
    #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
    pub(super) const CURRENT: HostPlatform = HostPlatform::Linux;
    #[cfg(target_env = "ohos")]
    pub(super) const CURRENT: HostPlatform = HostPlatform::Ohos;
    #[cfg(target_os = "macos")]
    pub(super) const CURRENT: HostPlatform = HostPlatform::Macos;
    #[cfg(target_os = "ios")]
    pub(super) const CURRENT: HostPlatform = HostPlatform::Ios;
    #[cfg(target_os = "espidf")]
    pub(super) const CURRENT: HostPlatform = HostPlatform::Esp32;
    #[cfg(target_arch = "wasm32")]
    pub(super) const CURRENT: HostPlatform = HostPlatform::Web;
    #[cfg(not(any(
        windows,
        target_os = "android",
        target_os = "linux",
        target_env = "ohos",
        target_os = "macos",
        target_os = "ios",
        target_os = "espidf",
        target_arch = "wasm32"
    )))]
    pub(super) const CURRENT: HostPlatform = HostPlatform::Other;
}
