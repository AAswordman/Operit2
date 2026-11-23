use std::ffi::OsString;
use std::io;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use operit_host_api::{HostError, HostResult};
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
    RRF_RT_REG_SZ,
};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

struct RegistryKey(HKEY);

impl Drop for RegistryKey {
    /// Closes the per-user WSL registration key.
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}

/// Finds the system WSL launcher without searching the application's working directory or PATH.
pub(super) fn system_executable() -> HostResult<PathBuf> {
    let mut directory = vec![0; 32768];
    let length = unsafe { GetSystemDirectoryW(directory.as_mut_ptr(), directory.len() as u32) };
    if length == 0 || length as usize >= directory.len() {
        return Err(HostError::new(
            "Cannot resolve the Windows system directory",
        ));
    }
    Ok(PathBuf::from(OsString::from_wide(&directory[..length as usize])).join("wsl.exe"))
}

/// Lists WSL registrations for the Windows account running Core without launching a distribution.
pub(super) fn registered_distributions() -> HostResult<Vec<String>> {
    let mut distributions = Vec::new();
    if !system_executable()?.is_file() {
        return Ok(distributions);
    }
    let path: Vec<_> = r"Software\Microsoft\Windows\CurrentVersion\Lxss"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut key = null_mut();
    let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, KEY_READ, &mut key) };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(distributions);
    }
    check_status(status)?;
    let key = RegistryKey(key);
    for index in 0.. {
        let mut subkey = [0u16; 256];
        let mut length = subkey.len() as u32;
        let status = unsafe {
            RegEnumKeyExW(
                key.0,
                index,
                subkey.as_mut_ptr(),
                &mut length,
                null(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            break;
        }
        check_status(status)?;
        if let Some(name) = distribution_name(&key, &subkey)? {
            distributions.push(name);
        }
    }
    distributions.sort_unstable();
    distributions.dedup();
    Ok(distributions)
}

/// Reads the display name from a registration, skipping non-distribution subkeys.
fn distribution_name(key: &RegistryKey, subkey: &[u16]) -> HostResult<Option<String>> {
    let value: Vec<_> = "DistributionName".encode_utf16().chain(Some(0)).collect();
    let mut bytes = 0;
    let status = unsafe {
        RegGetValueW(
            key.0,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut bytes,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    check_status(status)?;
    let mut name = vec![0u16; bytes.div_ceil(2) as usize];
    let status = unsafe {
        RegGetValueW(
            key.0,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            name.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    check_status(status)?;
    let name = String::from_utf16(&name[..bytes as usize / 2])
        .map_err(|error| HostError::new(format!("Invalid WSL distribution name: {error}")))?;
    Ok(Some(name.trim_end_matches('\0').to_owned()))
}

/// Preserves Windows registry error details for the current Core device.
fn check_status(status: u32) -> HostResult<()> {
    if status != ERROR_SUCCESS {
        return Err(HostError::new(format!(
            "Cannot read WSL registrations: {}",
            io::Error::from_raw_os_error(status as i32)
        )));
    }
    Ok(())
}
