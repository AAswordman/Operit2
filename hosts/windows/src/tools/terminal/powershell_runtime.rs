use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadFileRequest, HttpDownloadRequest,
    HttpHost,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::WindowsHttpHost;

const DOWNLOAD_DEADLINE: Duration = Duration::from_secs(600);

#[derive(Deserialize)]
struct Release {
    version: String,
    x86_64: Archive,
    aarch64: Archive,
}

#[derive(Deserialize)]
struct Archive {
    suffix: String,
    bytes: u64,
    sha256: String,
}

/// Installs the pinned interpreter on first use and reuses it across launches.
pub(super) fn executable(cache: &Path) -> HostResult<PathBuf> {
    let release: Release = serde_json::from_str(include_str!("powershell_release.json"))
        .map_err(|error| HostError::new(format!("Invalid PowerShell release metadata: {error}")))?;
    let archive = match std::env::consts::ARCH {
        "x86_64" => release.x86_64,
        "aarch64" => release.aarch64,
        arch => {
            return Err(HostError::new(format!(
                "Unsupported PowerShell architecture: {arch}"
            )))
        }
    };
    let version = release.version;
    let name = format!("PowerShell-{version}-win-{}", archive.suffix);
    let url =
        format!("https://github.com/PowerShell/PowerShell/releases/download/v{version}/{name}.zip");
    let cache_key = format!("{version}-{}", archive.suffix);
    install(cache, &cache_key, &archive.sha256, |target| {
        download(
            &WindowsHttpHost::new(),
            &url,
            target,
            archive.bytes,
            DOWNLOAD_DEADLINE,
        )
    })
    .map_err(|error| HostError::new(format!("PowerShell 7 setup failed: {}", error.message)))
}

/// Serializes installation and publishes only an extracted, verified runtime.
fn install(
    cache: &Path,
    cache_key: &str,
    sha256: &str,
    download: impl FnOnce(&Path) -> HostResult<()>,
) -> HostResult<PathBuf> {
    let runtime = cache.join(cache_key);
    let executable = runtime.join("pwsh.exe");
    if executable.is_file() {
        return Ok(executable);
    }
    fs::create_dir_all(cache)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(cache.join(format!("{cache_key}.lock")))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            return Err(HostError::new("PowerShell 7 setup is already in progress. Open the terminal again when it finishes."));
        }
        Err(TryLockError::Error(error)) => return Err(error.into()),
    }
    if executable.is_file() {
        return Ok(executable);
    }
    let archive = cache.join(format!("{cache_key}.zip"));
    download(&archive)?;
    let staging = tempfile::tempdir_in(cache)?;
    let extracted = staging.path().join("runtime");
    let result = extract_verified(&archive, sha256, &extracted);
    fs::remove_file(&archive)?;
    result?;
    fs::rename(&extracted, &runtime)?;
    Ok(executable)
}

/// Uses the native resumable downloader with connection, idle and total deadlines.
fn download(
    http: &dyn HttpHost,
    url: &str,
    target: &Path,
    bytes: u64,
    deadline: Duration,
) -> HostResult<()> {
    let target = target
        .to_str()
        .ok_or_else(|| HostError::new("Invalid PowerShell download path"))?;
    let control = HttpDownloadControl::default();
    let cancel = control.clone();
    let (finished, wait) = mpsc::channel::<()>();
    let timer = thread::Builder::new()
        .name("powershell-download-deadline".into())
        .spawn(move || {
            if wait.recv_timeout(deadline) == Err(mpsc::RecvTimeoutError::Timeout) {
                cancel.cancel();
            }
        })?;
    let result = http.downloadFiles(
        HttpDownloadRequest {
            downloadId: "powershell-runtime".into(),
            files: vec![HttpDownloadFileRequest {
                fileId: "powershell-zip".into(),
                url: url.into(),
                targetPath: target.into(),
                headers: Vec::new(),
                expectedBytes: bytes,
            }],
            maxConcurrency: 1,
            connectTimeoutSeconds: 15,
            readTimeoutSeconds: 30,
            followRedirects: true,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        },
        control.clone(),
        Arc::new(|_| {}),
    );
    drop(finished);
    timer
        .join()
        .map_err(|_| HostError::new("PowerShell download deadline worker stopped"))?;
    if control.isCancelled() {
        return Err(HostError::timeout(
            "PowerShell 7 download timed out. Open the terminal again to resume.",
        ));
    }
    result.map(|_| ()).map_err(|error| {
        HostError::new(format!(
            "PowerShell 7 download failed. Check your connection and open the terminal again. {}",
            error.message
        ))
    })
}

/// Verifies the ZIP before extracting any files into the unpublished staging directory.
fn extract_verified(archive: &Path, sha256: &str, destination: &Path) -> HostResult<()> {
    let mut file = File::open(archive)?;
    let mut hash = Sha256::new();
    io::copy(&mut file, &mut hash)?;
    if format!("{:x}", hash.finalize()) != sha256 {
        return Err(HostError::new(
            "PowerShell 7 download failed SHA-256 verification",
        ));
    }
    ZipArchive::new(file)
        .and_then(|mut zip| zip.extract(destination))
        .map_err(|error| HostError::new(format!("Cannot extract PowerShell 7: {error}")))?;
    if !destination.join("pwsh.exe").is_file() {
        return Err(HostError::new("PowerShell 7 archive is missing pwsh.exe"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Instant;

    /// Publishes the interpreter exactly once, including across separate callers.
    #[test]
    fn verified_install_is_atomic_and_reused() {
        let cache = tempfile::tempdir().unwrap();
        let fixture = cache.path().join("fixture.zip");
        let mut zip = zip::ZipWriter::new(File::create(&fixture).unwrap());
        zip.start_file("pwsh.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"interpreter fixture").unwrap();
        zip.finish().unwrap();
        let hash = format!("{:x}", Sha256::digest(fs::read(&fixture).unwrap()));
        let key = "7.6.10-x64";
        let exe = install(cache.path(), key, &hash, |target| {
            let attempt = install(cache.path(), key, &hash, |_| panic!("duplicate download"));
            assert!(attempt.unwrap_err().message.contains("already in progress"));
            assert!(!cache.path().join(key).exists());
            fs::copy(&fixture, target)?;
            Ok(())
        })
        .unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"interpreter fixture");
        assert_eq!(
            install(cache.path(), key, &hash, |_| panic!("cache hit downloaded")).unwrap(),
            exe
        );
        assert!(!cache.path().join(format!("{key}.zip")).exists());
    }

    /// Keeps a checksum failure out of the executable cache.
    #[test]
    fn bad_archive_is_removed_without_publishing() {
        let cache = tempfile::tempdir().unwrap();
        let error = install(cache.path(), "bad", "wrong-sha256", |target| {
            fs::write(target, b"corrupt")?;
            Ok(())
        })
        .unwrap_err();
        assert!(error.message.contains("SHA-256"));
        assert!(!cache.path().join("bad").exists());
        assert!(!cache.path().join("bad.zip").exists());
    }

    /// Interrupts a stalled response and resumes its retained bytes on the next request.
    #[test]
    fn stalled_download_times_out_and_resumes() {
        let cache = tempfile::tempdir().unwrap();
        let target = cache.path().join("runtime.zip");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/test", listener.local_addr().unwrap());
        let (resume, after_deadline) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            assert!(!request_headers(&mut stream).contains("range:"));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabc")
                .unwrap();
            after_deadline.recv_timeout(Duration::from_secs(5)).unwrap();
            drop(stream);

            let (mut stream, _) = listener.accept().unwrap();
            assert!(request_headers(&mut stream).contains("range: bytes=3-"));
            stream.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 3-5/6\r\nConnection: close\r\n\r\ndef").unwrap();
        });
        let http = WindowsHttpHost::new();
        let start = Instant::now();
        let error = download(&http, &url, &target, 6, Duration::from_secs(1)).unwrap_err();
        assert!(error.message.contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(5));
        assert!(!target.exists());
        resume.send(()).unwrap();
        download(&http, &url, &target, 6, Duration::from_secs(5)).unwrap();
        assert_eq!(fs::read(target).unwrap(), b"abcdef");
        server.join().unwrap();
    }

    /// Reads a complete local test request while bounding the socket wait.
    fn request_headers(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut headers = Vec::new();
        let mut byte = [0u8; 1];
        while !headers.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            headers.extend(byte);
        }
        String::from_utf8(headers).unwrap().to_ascii_lowercase()
    }

    /// Exercises the real upstream asset, extraction, and interpreter startup on Windows.
    #[test]
    #[ignore = "downloads the pinned PowerShell release from GitHub"]
    fn upstream_distribution_starts() {
        let cache = tempfile::tempdir().unwrap();
        let path = executable(cache.path()).unwrap();
        let output = std::process::Command::new(&path)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "1,2,3 | Measure-Object -Sum | Select-Object -ExpandProperty Sum",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "6");
        assert_eq!(executable(cache.path()).unwrap(), path);
    }
}
