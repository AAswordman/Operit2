use std::ffi::OsStr;
use std::os::windows::process::CommandExt;
use std::process::Command;

use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

/// Creates a background command without allocating a Windows console window.
pub(crate) fn background_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}
