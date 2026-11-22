//! Windows ConPTY processes with IOCP-owned output instead of per-session reader threads.

use operit_host_api::{HostError, HostResult};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::File;
use std::os::windows::{ffi::OsStrExt, io::{AsRawHandle, FromRawHandle, OwnedHandle}};
use std::process::Command;
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    Storage::FileSystem::{CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, OPEN_EXISTING, PIPE_ACCESS_INBOUND},
    System::{
        Console::{ClosePseudoConsole, CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON},
        Pipes::{CreateNamedPipeW, CreatePipe, PIPE_REJECT_REMOTE_CLIENTS},
        Threading::{CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
            InitializeProcThreadAttributeList, TerminateProcess, UpdateProcThreadAttribute,
            WaitForSingleObject, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT,
            INFINITE, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, STARTF_USESTDHANDLES, STARTUPINFOEXW},
    },
};

/// Owns the pseudoconsole until its process has stopped and output has drained.
pub(super) struct Process {
    console: Console,
    child: OwnedHandle,
    pub rows: u16,
    pub cols: u16,
}

/// Closes a pseudoconsole on every launch failure and normal session teardown.
struct Console(HPCON);
impl Drop for Console {
    /// Closes ConPTY while its independent IOCP reader can still drain the final output.
    fn drop(&mut self) { unsafe { ClosePseudoConsole(self.0) }; }
}

/// Releases native process attribute storage on both successful and failed launches.
struct Attributes(Vec<usize>);
impl Drop for Attributes {
    /// Deletes the initialized list before its aligned allocation is released.
    fn drop(&mut self) { unsafe { DeleteProcThreadAttributeList(self.0.as_mut_ptr().cast()) }; }
}

impl Process {
    /// Launches the exact interpreter using an overlapped server pipe for terminal output.
    pub fn spawn(command: Command, rows: u16, cols: u16) -> HostResult<(Self, File, OwnedHandle)> {
        let (mut input, mut writer) = (std::ptr::null_mut(), std::ptr::null_mut());
        check(unsafe { CreatePipe(&mut input, &mut writer, std::ptr::null(), 0) })?;
        let input = unsafe { OwnedHandle::from_raw_handle(input) };
        let writer = unsafe { File::from_raw_handle(writer) };
        let name = wide(OsStr::new(&format!(r"\\.\pipe\operit-pty-{}", uuid::Uuid::new_v4())));
        let output = owned(unsafe { CreateNamedPipeW(name.as_ptr(),
            PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_REJECT_REMOTE_CLIENTS, 1, 0, 64 * 1024, 0, std::ptr::null()) })?;
        let guestOutput = owned(unsafe { CreateFileW(name.as_ptr(), 0x4000_0000, 0,
            std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut()) })?;
        let mut console = 0;
        let status = unsafe { CreatePseudoConsole(COORD { X: cols as i16, Y: rows as i16 },
            input.as_raw_handle(), guestOutput.as_raw_handle(), 0, &mut console) };
        if status < 0 { return Err(HostError::new(format!("CreatePseudoConsole failed: {status:#x}"))); }
        let console = Console(console);
        let mut length = 0;
        unsafe { InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut length) };
        let mut attributes = vec![0usize; length.div_ceil(std::mem::size_of::<usize>())];
        check(unsafe { InitializeProcThreadAttributeList(attributes.as_mut_ptr().cast(), 1, 0, &mut length) })?;
        let mut attributes = Attributes(attributes);
        check(unsafe { UpdateProcThreadAttribute(attributes.0.as_mut_ptr().cast(), 0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize, console.0 as *const _,
            std::mem::size_of::<HPCON>(), std::ptr::null_mut(), std::ptr::null()) })?;
        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;
        startup.StartupInfo.hStdOutput = INVALID_HANDLE_VALUE;
        startup.StartupInfo.hStdError = INVALID_HANDLE_VALUE;
        startup.lpAttributeList = attributes.0.as_mut_ptr().cast();
        let mut process: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
        let executable = wide(command.get_program());
        let mut commandline = Vec::new();
        for argument in std::iter::once(command.get_program()).chain(command.get_args()) {
            if !commandline.is_empty() { commandline.push(b' ' as u16); }
            quote(argument, &mut commandline);
        }
        commandline.push(0);
        let directory = command.get_current_dir().map(|path| wide(path.as_os_str()));
        let mut environment = std::env::vars_os().collect::<BTreeMap<_, _>>();
        for (name, value) in command.get_envs() {
            environment.retain(|key, _| !key.eq_ignore_ascii_case(name));
            if let Some(value) = value { environment.insert(name.to_owned(), value.to_owned()); }
        }
        let mut block = Vec::new();
        for (name, value) in environment {
            block.extend(name.encode_wide());
            block.push(b'=' as u16);
            block.extend(value.encode_wide());
            block.push(0);
        }
        block.push(0);
        check(unsafe { CreateProcessW(executable.as_ptr(), commandline.as_mut_ptr(),
            std::ptr::null(), std::ptr::null(), 0,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT, block.as_ptr().cast(),
            directory.as_ref().map_or(std::ptr::null(), |value| value.as_ptr()),
            &startup.StartupInfo, &mut process) })?;
        let child = unsafe { OwnedHandle::from_raw_handle(process.hProcess) };
        drop(unsafe { OwnedHandle::from_raw_handle(process.hThread) });
        Ok((Self { console, child, rows, cols }, writer, output))
    }

    /// Resizes the same pseudoconsole and retains the dimensions used by screen rendering.
    pub fn resize(&mut self, rows: u16, cols: u16) -> HostResult<()> {
        let status = unsafe { ResizePseudoConsole(self.console.0, COORD { X: cols as i16, Y: rows as i16 }) };
        if status < 0 { return Err(HostError::new(format!("ResizePseudoConsole failed: {status:#x}"))); }
        (self.rows, self.cols) = (rows, cols);
        Ok(())
    }

    /// Returns only a completed process status without creating a waiter thread.
    pub fn try_wait(&self) -> HostResult<Option<i32>> {
        match unsafe { WaitForSingleObject(self.child.as_raw_handle(), 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut code = 0;
                check(unsafe { GetExitCodeProcess(self.child.as_raw_handle(), &mut code) })?;
                Ok(Some(code as i32))
            }
            _ => Err(std::io::Error::last_os_error().into()),
        }
    }
}

impl Drop for Process {
    /// Stops the interpreter before closing ConPTY, which wakes its pending overlapped read.
    fn drop(&mut self) {
        unsafe {
            TerminateProcess(self.child.as_raw_handle(), 1);
            WaitForSingleObject(self.child.as_raw_handle(), INFINITE);
        }
    }
}

/// Owns a valid Windows kernel handle immediately after its creating API succeeds.
fn owned(handle: HANDLE) -> HostResult<OwnedHandle> {
    if handle == INVALID_HANDLE_VALUE { return Err(std::io::Error::last_os_error().into()); }
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// Reports the exact native API error without selecting another implementation.
fn check(success: i32) -> HostResult<()> {
    if success == 0 { Err(std::io::Error::last_os_error().into()) } else { Ok(()) }
}

/// Encodes an OS string for Windows APIs while preserving Unicode code units.
fn wide(value: &OsStr) -> Vec<u16> { value.encode_wide().chain(Some(0)).collect() }

/// Quotes one argument according to the Windows process command-line escaping rules.
fn quote(argument: &OsStr, output: &mut Vec<u16>) {
    output.push(b'"' as u16);
    let mut slashes = 0;
    for unit in argument.encode_wide() {
        if unit == b'\\' as u16 { slashes += 1; continue; }
        output.extend(std::iter::repeat_n(b'\\' as u16, slashes * if unit == b'"' as u16 { 2 } else { 1 }));
        if unit == b'"' as u16 { output.push(b'\\' as u16); }
        output.push(unit);
        slashes = 0;
    }
    output.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
    output.push(b'"' as u16);
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::HostRuntimeTaskSchedulerHost;

    /// Closing a silent session wakes and releases its pending IOCP read without a polling timer.
    #[test]
    fn iocp_silent_close_releases_output_reader() {
        use std::os::windows::io::IntoRawHandle;
        use tokio::io::AsyncReadExt;
        let executable = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut command = Command::new(executable);
        command.args(["-NoLogo", "-NoProfile", "-Command", "Start-Sleep -Seconds 120"]);
        let (process, writer, reader) = Process::spawn(command, 24, 80).unwrap();
        let (started, ready) = std::sync::mpsc::sync_channel(1);
        let (finished, complete) = std::sync::mpsc::sync_channel(1);
        operit_host_native_common::NativeHostRuntimeTaskSchedulerHost::new()
            .scheduleHostRuntimeCooperativeAsyncTask("test-iocp-eof", Box::new(move || Box::pin(async move {
                let mut reader = unsafe { tokio::net::windows::named_pipe::NamedPipeServer::from_raw_handle(reader.into_raw_handle()) }.unwrap();
                reader.connect().await.unwrap();
                started.send(()).unwrap();
                let mut bytes = [0u8; 4096];
                loop {
                    match reader.read(&mut bytes).await {
                        Ok(0) => break,
                        Ok(_) => {},
                        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => break,
                        Err(error) => panic!("Unexpected IOCP read error: {error}"),
                    }
                }
                drop(reader);
                finished.send(()).unwrap();
            }))).unwrap();
        ready.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        drop(writer);
        drop(process);
        complete.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    }
}
