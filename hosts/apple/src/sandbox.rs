//! Reads the running process signature, including when Core is embedded in Flutter.

use operit_host_api::HostIsolation;

/// Describes OS isolation independently of the AI permission mode or build flags.
pub(crate) fn processIsolation() -> HostIsolation {
    #[cfg(target_os = "macos")]
    {
        use std::sync::OnceLock;
        static ISOLATION: OnceLock<HostIsolation> = OnceLock::new();
        ISOLATION
            .get_or_init(|| match macos::isSandboxed() {
                Some(false) => HostIsolation::None,
                // A failed signature query must not advertise unrestricted execution.
                _ => HostIsolation::OsAppSandbox,
            })
            .clone()
    }
    #[cfg(not(target_os = "macos"))]
    {
        HostIsolation::OsAppSandbox
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::{CFString, CFStringRef};
    use security_framework::os::macos::code_signing::{Flags, SecCode};
    use std::ffi::c_void;
    use std::ptr;

    #[link(name = "Security", kind = "framework")]
    extern "C" {
        fn SecCodeCopySigningInformation(
            code: *const c_void,
            flags: u32,
            information: *mut CFDictionaryRef,
        ) -> i32;
        static kSecCodeInfoEntitlementsDict: CFStringRef;
    }

    pub(super) fn isSandboxed() -> Option<bool> {
        let code = SecCode::for_self(Flags::NONE).ok()?;
        let mut information: CFDictionaryRef = ptr::null();
        // Apple's public API accepts both SecCode and SecStaticCode. Copy returns
        // a retained dictionary, released by the Core Foundation wrapper below.
        let status = unsafe {
            SecCodeCopySigningInformation(
                code.as_concrete_TypeRef().cast(),
                1 << 1, // kSecCSSigningInformation
                &mut information,
            )
        };
        if status != 0 || information.is_null() {
            return None;
        }
        let information: CFDictionary =
            unsafe { CFDictionary::wrap_under_create_rule(information) };
        let Some(value) =
            information.find(unsafe { kSecCodeInfoEntitlementsDict }.cast::<c_void>())
        else {
            return Some(false);
        };
        let entitlements =
            unsafe { CFType::wrap_under_get_rule(*value) }.downcast::<CFDictionary>()?;
        let key = CFString::new("com.apple.security.app-sandbox");
        let Some(value) = entitlements.find(key.as_CFTypeRef()) else {
            return Some(false);
        };
        let enabled = unsafe { CFType::wrap_under_get_rule(*value) }.downcast::<CFBoolean>()?;
        Some(bool::from(enabled))
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use crate::AppleFileSystemHost;
    use operit_host_api::{FileSystemHost, HostIsolation};

    /// Checks ordinary native filesystem access, outside runtime/workspace roots.
    #[test]
    fn unsandbox_exposes_native_root_and_reads_and_writes_external_files() {
        if super::processIsolation() != HostIsolation::None {
            return;
        }
        let host = AppleFileSystemHost::new();
        let descriptor = host.environmentDescriptor();
        assert_eq!(descriptor.isolation, HostIsolation::None);
        assert!(descriptor
            .workspaceRoots
            .iter()
            .any(|root| root.physicalRoot == "/" && root.vfsRoot == "/mnt/macos"));
        assert!(host
            .listFiles("/")
            .unwrap()
            .iter()
            .any(|entry| entry.name == "Users"));
        let folder =
            std::env::temp_dir().join(format!("operit-native-fs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&folder).unwrap();
        let file = folder.join("external.txt");
        let path = file.to_str().unwrap();
        host.writeFile(path, "native filesystem", false).unwrap();
        assert_eq!(host.readFile(path).unwrap(), "native filesystem");
        host.deleteFile(path, false).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
}
