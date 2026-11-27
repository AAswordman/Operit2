//! Owns the native resource manager supplied by the OpenHarmony application.

use std::ffi::{c_char, c_int, c_long, c_void, CString};
use operit_host_api::{HostError, HostResult, BundledResourceHost::{BundledResourceHost, validateResourceKey}};

#[link(name = "rawfile.z")]
extern "C" {
    fn OH_ResourceManager_OpenRawFile(manager: *mut c_void, name: *const c_char) -> *mut c_void;
    fn OH_ResourceManager_GetRawFileSize(file: *mut c_void) -> c_long;
    fn OH_ResourceManager_ReadRawFile(file: *mut c_void, buffer: *mut c_void, length: usize) -> c_int;
    fn OH_ResourceManager_CloseRawFile(file: *mut c_void);
    fn OH_ResourceManager_ReleaseNativeResourceManager(manager: *mut c_void);
}

pub struct OhosBundledResourceHost {
    manager: usize,
    lock: std::sync::Mutex<()>,
}

impl OhosBundledResourceHost {
    /// Takes exclusive ownership of a native resource manager retained by the application owner.
    /// The pointer must come from OH_ResourceManager_InitNativeResourceManager.
    pub unsafe fn new(manager: *mut c_void) -> HostResult<Self> {
        if manager.is_null() {
            return Err(HostError::new("OpenHarmony bundled resource manager is null"));
        }
        Ok(Self { manager: manager as usize, lock: std::sync::Mutex::new(()) })
    }
}

impl Drop for OhosBundledResourceHost {
    /// Releases the application resource manager after its final reader is dropped.
    fn drop(&mut self) {
        unsafe { OH_ResourceManager_ReleaseNativeResourceManager(self.manager as *mut c_void) }
    }
}

struct RawFile(*mut c_void);

impl Drop for RawFile {
    /// Closes the resource handle on every read outcome.
    fn drop(&mut self) {
        unsafe { OH_ResourceManager_CloseRawFile(self.0) }
    }
}

impl BundledResourceHost for OhosBundledResourceHost {
    /// Reads the packaged Flutter asset on the calling Core worker.
    fn read(&self, key: &str) -> HostResult<Vec<u8>> {
        validateResourceKey(key)?;
        let _guard = self.lock.lock().map_err(|_| HostError::new("Resource manager lock is poisoned"))?;
        let path = CString::new(format!("flutter_assets/{key}")).map_err(|error| HostError::new(error.to_string()))?;
        unsafe {
            let file = OH_ResourceManager_OpenRawFile(self.manager as *mut c_void, path.as_ptr());
            if file.is_null() {
                return Err(HostError::new(format!("Bundled resource not found: {key}")));
            }
            let file = RawFile(file);
            let length = usize::try_from(OH_ResourceManager_GetRawFileSize(file.0))
                .map_err(|_| HostError::new("Invalid bundled resource length"))?;
            let mut bytes = vec![0; length];
            let mut offset = 0;
            while offset < length {
                let read = OH_ResourceManager_ReadRawFile(file.0, bytes[offset..].as_mut_ptr().cast(), (length - offset).min(1024 * 1024));
                if read <= 0 {
                    return Err(HostError::new(format!("Incomplete bundled resource read: {key}")));
                }
                offset += read as usize;
            }
            Ok(bytes)
        }
    }
}
