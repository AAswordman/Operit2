//! Owns piped subprocesses and cancellable readers for native managed-runtime hosts.
use operit_host_api::{
    HostError, HostResult, ManagedRuntimeProcess, RuntimeCommandOutput, RuntimeProcessRequest,
};
use operit_host_native_common::{scheduleNativeServiceTasks, NativeTaskHandle};
use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

const MANAGED_RUNTIME_STDIO_BUFFER_BYTES: usize = 64 * 1024;

pub struct NativeManagedRuntimeProcess {
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    stdoutRx: Mutex<Receiver<String>>,
    stderrLines: Arc<Mutex<VecDeque<String>>>,
    stopped: Arc<AtomicBool>,
    readers: Mutex<Vec<NativeTaskHandle<()>>>,
}

impl NativeManagedRuntimeProcess {
    /// Takes ownership before any fallible setup so errors also terminate and reap the child.
    pub fn start(child: Child) -> HostResult<Box<dyn ManagedRuntimeProcess>> {
        let (stdoutTx, stdoutRx) = mpsc::channel();
        let stderrLines = Arc::new(Mutex::new(VecDeque::new()));
        let stopped = Arc::new(AtomicBool::new(false));
        let mut process = Self {
            child: Mutex::new(child),
            stdin: Mutex::new(None),
            stdoutRx: Mutex::new(stdoutRx),
            stderrLines: stderrLines.clone(),
            stopped: stopped.clone(),
            readers: Mutex::new(Vec::new()),
        };
        let child = process.child.get_mut().unwrap();
        let missingPipe = || HostError::new("managed runtime stdio pipe unavailable");
        *process.stdin.get_mut().unwrap() = Some(child.stdin.take().ok_or_else(missingPipe)?);
        let stdout = child.stdout.take().ok_or_else(missingPipe)?;
        let stderr = child.stderr.take().ok_or_else(missingPipe)?;
        let stdoutStop = stopped.clone();
        let tasks: Vec<operit_host_api::HostRuntimeTask> = vec![
            Box::new(move || readLines(stdout, &stdoutStop, |line| stdoutTx.send(line).is_ok())),
            Box::new(move || {
                readLines(stderr, &stopped, |line| {
                    let mut lines = stderrLines
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    lines.push_back(line + "\n");
                    while lines.len() > 400 {
                        lines.pop_front();
                    }
                    true
                })
            }),
        ];
        *process.readers.get_mut().unwrap() =
            scheduleNativeServiceTasks("managed-runtime-pipes", tasks)?;
        Ok(Box::new(process))
    }

    pub fn run(mut command: std::process::Command) -> HostResult<RuntimeCommandOutput> {
        let output = command.output()?;
        Ok(RuntimeCommandOutput {
            exitCode: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    fn writeFrames<'a>(&self, lines: impl IntoIterator<Item = &'a str>) -> HostResult<()> {
        let frame: Vec<u8> = lines
            .into_iter()
            .flat_map(|line| line.bytes().chain([b'\n']))
            .collect();
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| HostError::new("stdin mutex poisoned"))?;
        let stdin = stdin
            .as_mut()
            .ok_or_else(|| HostError::new("managed runtime stdin is closed"))?;
        stdin.write_all(&frame)?;
        Ok(stdin.flush()?)
    }

    fn closePipes(&self) {
        self.stopped.store(true, Ordering::Release);
        self.stdin
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let readers = std::mem::take(
            &mut *self
                .readers
                .lock()
                .unwrap_or_else(|error| error.into_inner()),
        );
        for reader in readers {
            let _ = reader.join_timeout(Duration::from_secs(1));
        }
    }
}

impl Drop for NativeManagedRuntimeProcess {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

impl ManagedRuntimeProcess for NativeManagedRuntimeProcess {
    fn writeLine(&self, line: &str) -> HostResult<()> {
        self.writeFrames([line])
    }
    fn writeLines(&self, lines: &[String]) -> HostResult<()> {
        self.writeFrames(lines.iter().map(String::as_str))
    }
    fn readStdoutLine(&self, timeoutMs: u64) -> HostResult<Option<String>> {
        let receiver = self
            .stdoutRx
            .lock()
            .map_err(|_| HostError::new("stdout mutex poisoned"))?;
        Ok(receiver.recv_timeout(Duration::from_millis(timeoutMs)).ok())
    }
    fn drainStderr(&self) -> HostResult<String> {
        Ok(self
            .stderrLines
            .lock()
            .map_err(|_| HostError::new("stderr mutex poisoned"))?
            .drain(..)
            .collect())
    }

    fn isRunning(&self) -> HostResult<bool> {
        let running = self
            .child
            .lock()
            .map_err(|_| HostError::new("child mutex poisoned"))?
            .try_wait()?
            .is_none();
        if !running {
            self.closePipes();
        }
        Ok(running)
    }
    fn kill(&self) -> HostResult<()> {
        let result = (|| -> io::Result<()> {
            let mut child = self.child.lock().unwrap_or_else(|error| error.into_inner());
            if child.try_wait()?.is_none() {
                child.kill()?;
                child.wait()?;
            }
            Ok(())
        })();
        self.closePipes();
        result.map_err(HostError::from)
    }
}

#[cfg(unix)]
trait Pipe: Read + std::os::fd::AsRawFd {}
#[cfg(unix)]
impl<T: Read + std::os::fd::AsRawFd> Pipe for T {}
#[cfg(windows)]
trait Pipe: Read + std::os::windows::io::AsRawHandle {}
#[cfg(windows)]
impl<T: Read + std::os::windows::io::AsRawHandle> Pipe for T {}

/// Poll before each buffered read so an inherited pipe or partial line remains cancellable.
struct PipeReader<'a, T> {
    pipe: T,
    stopped: &'a AtomicBool,
    drainDeadline: Option<Instant>,
}
impl<T: Pipe> Read for PipeReader<'_, T> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            let stopping = self.stopped.load(Ordering::Acquire);
            if stopping
                && Instant::now()
                    >= *self
                        .drainDeadline
                        .get_or_insert_with(|| Instant::now() + Duration::from_millis(100))
            {
                return Ok(0);
            }
            match readChunk(&mut self.pipe, buffer, stopping)? {
                Some(count) => return Ok(count),
                None if stopping => return Ok(0),
                None => {}
            }
        }
    }
}
fn readLines(reader: impl Pipe, stopped: &AtomicBool, mut emit: impl FnMut(String) -> bool) {
    let reader = PipeReader {
        pipe: reader,
        stopped,
        drainDeadline: None,
    };
    for line in BufReader::with_capacity(MANAGED_RUNTIME_STDIO_BUFFER_BYTES, reader).lines() {
        match line {
            Ok(line) => {
                if !emit(line) {
                    break;
                }
            }
            Err(error) if error.kind() != io::ErrorKind::InvalidData => break,
            _ => {}
        }
    }
}

#[cfg(unix)]
fn readChunk(
    reader: &mut impl Pipe,
    buffer: &mut [u8],
    stopping: bool,
) -> io::Result<Option<usize>> {
    let mut descriptor = libc::pollfd {
        fd: reader.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: reader owns the fd and descriptor is initialized for the duration of poll.
    let ready = unsafe { libc::poll(&mut descriptor, 1, if stopping { 0 } else { 100 }) };
    if ready < 0 {
        return Err(io::Error::last_os_error());
    }
    if ready == 0 {
        return Ok(None);
    }
    reader.read(buffer).map(Some)
}

#[cfg(windows)]
fn readChunk(
    reader: &mut impl Pipe,
    buffer: &mut [u8],
    stopping: bool,
) -> io::Result<Option<usize>> {
    use windows_sys::Win32::{Foundation::ERROR_BROKEN_PIPE, System::Pipes::PeekNamedPipe};
    let mut available = 0;
    // SAFETY: reader owns the anonymous-pipe handle; only the valid byte count is requested.
    let ready = unsafe {
        PeekNamedPipe(
            reader.as_raw_handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        )
    };
    if ready == 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
            Ok(Some(0))
        } else {
            Err(error)
        };
    }
    if available == 0 {
        if !stopping {
            std::thread::sleep(Duration::from_millis(10));
        }
        return Ok(None);
    }
    let count = buffer.len().min(available as usize);
    reader.read(&mut buffer[..count]).map(Some)
}

/// Applies the same arguments, environment, and working directory to persistent and one-shot commands.
pub fn runtimeCommand(executable: String, request: RuntimeProcessRequest) -> std::process::Command {
    #[cfg(windows)]
    let mut command = crate::process::background_command(executable);
    #[cfg(unix)]
    let mut command = std::process::Command::new(executable);
    command.args(request.args).envs(request.env);
    if let Some(cwd) = request.cwd {
        command.current_dir(cwd);
    }
    command
}

/// Resolves explicit paths before falling back to PATH, consistently across native hosts.
pub fn ensureExecutablePath(path: &str) -> HostResult<String> {
    if std::path::Path::new(path).exists() {
        return Ok(path.to_owned());
    }
    findExecutable(&[path]).ok_or_else(|| HostError::new(format!("Executable not found: {path}")))
}

pub fn findExecutable(names: &[&str]) -> Option<String> {
    for dir in std::env::split_paths(&std::env::var_os("PATH")?) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
            #[cfg(windows)]
            if candidate.extension().is_none() {
                if let Some(extensions) = std::env::var_os("PATHEXT") {
                    for extension in extensions.to_string_lossy().split(';') {
                        let executable = dir.join(format!("{name}{extension}"));
                        if executable.is_file() {
                            return Some(executable.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use operit_host_native_common::scheduleNativeServiceTask;
    use std::os::unix::net::UnixStream;

    /// An inherited writer or a partial line must not prevent pipe readers from stopping.
    #[test]
    fn cancellation_releases_idle_and_partial_line_readers() {
        for bytes in [b"".as_slice(), b"incomplete line".as_slice()] {
            let (reader, mut writer) = UnixStream::pair().unwrap();
            writer.write_all(bytes).unwrap();
            let stopped = Arc::new(AtomicBool::new(false));
            let readerStop = stopped.clone();
            let (started, ready) = mpsc::channel();
            let task =
                scheduleNativeServiceTask("managed-runtime-reader-cancellation-test", move || {
                    started.send(()).unwrap();
                    readLines(reader, &readerStop, |_| true);
                })
                .unwrap();
            ready.recv_timeout(Duration::from_secs(2)).unwrap();
            stopped.store(true, Ordering::Release);
            let result = task.join_timeout(Duration::from_secs(1));
            drop(writer);
            assert!(result
                .expect("reader waited for the inherited pipe writer")
                .is_ok());
        }
    }
}
