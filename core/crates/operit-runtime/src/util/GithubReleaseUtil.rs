#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::header::{ACCEPT, CONTENT_LENGTH, RANGE, USER_AGENT};
use serde::Deserialize;

#[cfg(not(target_arch = "wasm32"))]
use crate::data::preferences::GitHubAuthPreferences::GitHubAuthPreferences;

const GITHUB_RELEASE_OWNER: &str = "AAswordman";
const GITHUB_RELEASE_REPO: &str = "Operit2";
#[cfg(not(target_arch = "wasm32"))]
const GITHUB_API_BASE: &str = "https://api.github.com";
#[cfg(not(target_arch = "wasm32"))]
const DOWNLOAD_THREAD_COUNT: u64 = 6;
#[cfg(not(target_arch = "wasm32"))]
const BUFFER_SIZE: usize = 128 * 1024;

#[derive(Debug, Clone, Default)]
pub struct GithubReleaseUtil;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub assetName: String,
    pub downloadUrl: String,
    pub releaseNotes: String,
    pub releasePageUrl: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullUpdateTarget {
    pub product: String,
    pub platform: String,
    pub arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullUpdateStatus {
    Available(ReleaseInfo),
    UpToDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullUpdateProgressEvent {
    StageChanged {
        stage: FullUpdateStage,
        message: String,
    },
    DownloadProgress {
        readBytes: u64,
        totalBytes: u64,
        speedBytesPerSec: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullUpdateStage {
    DownloadingPackage,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedVersion {
    major: i32,
    minor: i32,
    patch: i32,
    patchIndex: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[allow(non_snake_case)]
impl GithubReleaseUtil {
    pub async fn checkForFullUpdate(
        currentVersion: &str,
        target: FullUpdateTarget,
    ) -> Result<FullUpdateStatus, String> {
        let releaseInfo =
            Self::fetchLatestReleaseInfo(GITHUB_RELEASE_OWNER, GITHUB_RELEASE_REPO, target).await?;
        if Self::compareVersions(&releaseInfo.version, currentVersion) > 0 {
            Ok(FullUpdateStatus::Available(releaseInfo))
        } else {
            Ok(FullUpdateStatus::UpToDate)
        }
    }

    pub fn checkForFullUpdateBlocking(
        currentVersion: &str,
        target: FullUpdateTarget,
    ) -> Result<FullUpdateStatus, String> {
        let releaseInfo = Self::fetchLatestReleaseInfoBlocking(
            GITHUB_RELEASE_OWNER,
            GITHUB_RELEASE_REPO,
            target,
        )?;
        if Self::compareVersions(&releaseInfo.version, currentVersion) > 0 {
            Ok(FullUpdateStatus::Available(releaseInfo))
        } else {
            Ok(FullUpdateStatus::UpToDate)
        }
    }

    pub async fn fetchLatestReleaseInfo(
        repoOwner: &str,
        repoName: &str,
        target: FullUpdateTarget,
    ) -> Result<ReleaseInfo, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = repoOwner;
            let _ = repoName;
            let _ = target;
            return Err("full update release query is not available in wasm runtime".to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let owner = repoOwner.to_string();
            let repo = repoName.to_string();
            tokio::task::spawn_blocking(move || {
                Self::fetchLatestReleaseInfoBlocking(&owner, &repo, target)
            })
            .await
            .map_err(|error| error.to_string())?
        }
    }

    pub async fn downloadAndPrepareFullUpdateWithProgress<F>(
        packageUrl: String,
        packageFileName: String,
        workDir: PathBuf,
        onEvent: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(FullUpdateProgressEvent) + Send + Sync + 'static,
    {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = packageUrl;
            let _ = packageFileName;
            let _ = workDir;
            let _ = onEvent;
            return Err(
                "full update package download is not available in wasm runtime".to_string(),
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::task::spawn_blocking(move || {
                Self::downloadAndPrepareFullUpdateBlocking(
                    &packageUrl,
                    &packageFileName,
                    &workDir,
                    onEvent,
                )
            })
            .await
            .map_err(|error| error.to_string())?
        }
    }

    pub fn compareVersions(v1: &str, v2: &str) -> i32 {
        let p1 = parseVersion(v1);
        let p2 = parseVersion(v2);
        if p1.major != p2.major {
            return p1.major.cmp(&p2.major) as i32;
        }
        if p1.minor != p2.minor {
            return p1.minor.cmp(&p2.minor) as i32;
        }
        if p1.patch != p2.patch {
            return p1.patch.cmp(&p2.patch) as i32;
        }
        p1.patchIndex.cmp(&p2.patchIndex) as i32
    }

    pub fn fetchLatestReleaseInfoBlocking(
        repoOwner: &str,
        repoName: &str,
        target: FullUpdateTarget,
    ) -> Result<ReleaseInfo, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = repoOwner;
            let _ = repoName;
            let _ = target;
            return Err("full update release query is not available in wasm runtime".to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let targetAssetName = target.assetName()?;
            let url = format!(
                "{GITHUB_API_BASE}/repos/{repoOwner}/{repoName}/releases?page=1&per_page=1"
            );
            let client = Client::builder()
                .connect_timeout(Duration::from_secs(30))
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|error| error.to_string())?;
            let mut request = client
                .get(url)
                .header(USER_AGENT, "Operit-MCP-Client")
                .header(ACCEPT, "application/vnd.github.v3+json");
            if let Some(authHeader) = GitHubAuthPreferences::getInstance().getAuthorizationHeader()
            {
                request = request.header("Authorization", authHeader);
            }
            let response = request.send().map_err(|error| error.to_string())?;
            let status = response.status();
            if !status.is_success() {
                return Err(format!(
                    "HTTP {}: {}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or("")
                ));
            }
            let releases = response
                .json::<Vec<GitHubRelease>>()
                .map_err(|error| error.to_string())?;
            let latestRelease = releases
                .into_iter()
                .next()
                .ok_or_else(|| format!("No releases found for {repoOwner}/{repoName}"))?;
            let packageAsset = latestRelease
                .assets
                .iter()
                .find(|asset| asset.name == targetAssetName)
                .ok_or_else(|| {
                    format!(
                        "Release {} is missing asset {}",
                        latestRelease.tag_name, targetAssetName
                    )
                })?;
            Ok(ReleaseInfo {
                version: latestRelease.tag_name.trim_start_matches('v').to_string(),
                assetName: packageAsset.name.clone(),
                downloadUrl: packageAsset.browser_download_url.clone(),
                releaseNotes: latestRelease.body.unwrap_or_default(),
                releasePageUrl: latestRelease.html_url,
            })
        }
    }

    pub fn downloadAndPrepareFullUpdateBlocking<F>(
        packageUrl: &str,
        packageFileName: &str,
        workDir: &Path,
        onEvent: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(FullUpdateProgressEvent) + Send + Sync + 'static,
    {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = packageUrl;
            let _ = packageFileName;
            let _ = workDir;
            let _ = onEvent;
            return Err(
                "full update package download is not available in wasm runtime".to_string(),
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if workDir.exists() {
                fs::remove_dir_all(workDir).map_err(|error| error.to_string())?;
            }
            fs::create_dir_all(workDir).map_err(|error| error.to_string())?;
            validatePackageFileName(packageFileName)?;
            let packageFile = workDir.join(packageFileName);
            onEvent(FullUpdateProgressEvent::StageChanged {
                stage: FullUpdateStage::DownloadingPackage,
                message: "Downloading full update package".to_string(),
            });
            let totalBytes = fetchContentLength(packageUrl)?;
            verifyRangeDownloadSupported(packageUrl)?;
            downloadToFileMultiThread(packageUrl, &packageFile, totalBytes, onEvent)?;
            Ok(packageFile)
        }
    }
}

#[allow(non_snake_case)]
impl FullUpdateTarget {
    pub fn new(product: &str, platform: &str, arch: &str) -> Result<Self, String> {
        validateTarget(product, platform, arch)?;
        Ok(Self {
            product: product.to_string(),
            platform: platform.to_string(),
            arch: arch.to_string(),
        })
    }

    pub fn cliForCurrentHost() -> Result<Self, String> {
        Self::new("cli", currentDesktopPlatform()?, currentDesktopArch()?)
    }

    pub fn app(platform: &str, arch: &str) -> Result<Self, String> {
        Self::new("app", platform, arch)
    }

    pub fn assetName(&self) -> Result<String, String> {
        validateTarget(&self.product, &self.platform, &self.arch)?;
        Ok(format!(
            "operit2-{}-{}-{}.{}",
            self.product,
            self.platform,
            self.arch,
            packageExtension(&self.product, &self.platform)?
        ))
    }
}

#[allow(non_snake_case)]
fn validateTarget(product: &str, platform: &str, arch: &str) -> Result<(), String> {
    match product {
        "app" | "cli" => {}
        _ => return Err(format!("Unsupported update product: {product}")),
    }
    match platform {
        "windows" | "linux" | "macos" | "android" => {}
        _ => return Err(format!("Unsupported update platform: {platform}")),
    }
    if product == "cli" && platform == "android" {
        return Err("Operit2 CLI update assets are not published for Android".to_string());
    }
    match platform {
        "android" => match arch {
            "arm64-v8a" | "armeabi-v7a" | "x86_64" => Ok(()),
            _ => Err(format!("Unsupported Android update arch: {arch}")),
        },
        "windows" | "linux" | "macos" => match arch {
            "x86_64" | "aarch64" => Ok(()),
            _ => Err(format!("Unsupported desktop update arch: {arch}")),
        },
        _ => Err(format!("Unsupported update platform: {platform}")),
    }
}

#[allow(non_snake_case)]
fn packageExtension(product: &str, platform: &str) -> Result<&'static str, String> {
    match (product, platform) {
        ("app", "android") => Ok("apk"),
        ("app", "windows") | ("cli", "windows") => Ok("zip"),
        ("app", "linux") | ("app", "macos") | ("cli", "linux") | ("cli", "macos") => Ok("tar.gz"),
        _ => Err(format!("Unsupported update target: {product}/{platform}")),
    }
}

#[allow(non_snake_case)]
fn currentDesktopPlatform() -> Result<&'static str, String> {
    match std::env::consts::OS {
        "windows" => Ok("windows"),
        "linux" => Ok("linux"),
        "macos" => Ok("macos"),
        other => Err(format!("Unsupported desktop update platform: {other}")),
    }
}

#[allow(non_snake_case)]
fn currentDesktopArch() -> Result<&'static str, String> {
    match std::env::consts::ARCH {
        "x86_64" => Ok("x86_64"),
        "aarch64" => Ok("aarch64"),
        other => Err(format!("Unsupported desktop update arch: {other}")),
    }
}

#[allow(non_snake_case)]
fn validatePackageFileName(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Update package file name is empty".to_string());
    }
    if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(format!("Invalid update package file name: {name}"));
    }
    Ok(())
}

#[allow(non_snake_case)]
fn parseVersion(v: &str) -> ParsedVersion {
    let s = v.trim().trim_start_matches('v');
    let plusIdx = s.find('+');
    let base = match plusIdx {
        Some(index) => &s[..index],
        None => s,
    };
    let patchIndex = match plusIdx {
        Some(index) => s[index + 1..].parse::<i32>().ok().unwrap_or(0),
        None => 0,
    };
    let parts = base.split('.').collect::<Vec<_>>();
    ParsedVersion {
        major: parts
            .get(0)
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0),
        minor: parts
            .get(1)
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0),
        patch: parts
            .get(2)
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0),
        patchIndex,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fetchContentLength(url: &str) -> Result<u64, String> {
    let client = blockingDownloadClient()?;
    let response = client
        .get(url)
        .header(USER_AGENT, "Operit")
        .send()
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "HTTP {}: {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        ));
    }
    let totalBytes = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            "Server must provide a valid Content-Length for 6-thread download".to_string()
        })?;
    if totalBytes == 0 {
        return Err("Server must provide a valid Content-Length for 6-thread download".to_string());
    }
    Ok(totalBytes)
}

#[cfg(not(target_arch = "wasm32"))]
fn verifyRangeDownloadSupported(url: &str) -> Result<(), String> {
    let client = blockingDownloadClient()?;
    let response = client
        .get(url)
        .header(USER_AGENT, "Operit")
        .header(RANGE, "bytes=0-0")
        .send()
        .map_err(|error| error.to_string())?;
    if response.status().as_u16() != 206 {
        return Err(
            "Server does not support HTTP Range (required for 6-thread download)".to_string(),
        );
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn downloadToFileMultiThread<F>(
    url: &str,
    out: &Path,
    totalBytes: u64,
    onEvent: F,
) -> Result<(), String>
where
    F: Fn(FullUpdateProgressEvent) + Send + Sync + 'static,
{
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out)
        .map_err(|error| error.to_string())?;
    file.set_len(totalBytes)
        .map_err(|error| error.to_string())?;
    drop(file);

    let downloadedBytes = Arc::new(AtomicU64::new(0));
    let finished = Arc::new(AtomicBool::new(false));
    let onEvent = Arc::new(onEvent);
    onEvent(FullUpdateProgressEvent::DownloadProgress {
        readBytes: 0,
        totalBytes,
        speedBytesPerSec: 0,
    });

    let reporterDownloaded = downloadedBytes.clone();
    let reporterFinished = finished.clone();
    let reporterEvent = onEvent.clone();
    let reporter = thread::spawn(move || {
        let mut lastBytes = 0;
        let mut lastAt = Instant::now();
        while !reporterFinished.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(300));
            let currentBytes = reporterDownloaded.load(Ordering::SeqCst);
            let elapsedMs = lastAt.elapsed().as_millis() as u64;
            let elapsedMs = elapsedMs.max(1);
            let delta = currentBytes.saturating_sub(lastBytes);
            let speed = delta.saturating_mul(1000) / elapsedMs;
            reporterEvent(FullUpdateProgressEvent::DownloadProgress {
                readBytes: currentBytes,
                totalBytes,
                speedBytesPerSec: speed,
            });
            lastBytes = currentBytes;
            lastAt = Instant::now();
        }
    });

    let ranges = splitRanges(totalBytes, DOWNLOAD_THREAD_COUNT);
    let url = Arc::new(url.to_string());
    let out = Arc::new(out.to_path_buf());
    let mut workers = Vec::new();
    for (start, end) in ranges {
        let workerUrl = url.clone();
        let workerOut = out.clone();
        let workerDownloaded = downloadedBytes.clone();
        workers.push(thread::spawn(move || {
            downloadRangeToFile(&workerUrl, &workerOut, start, end, &workerDownloaded)
        }));
    }

    let mut firstError = None;
    for worker in workers {
        match worker.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                if firstError.is_none() {
                    firstError = Some(error);
                }
            }
            Err(_) => {
                if firstError.is_none() {
                    firstError = Some("download worker panicked".to_string());
                }
            }
        }
    }
    finished.store(true, Ordering::SeqCst);
    let _ = reporter.join();

    if let Some(error) = firstError {
        return Err(error);
    }
    onEvent(FullUpdateProgressEvent::DownloadProgress {
        readBytes: totalBytes,
        totalBytes,
        speedBytesPerSec: 0,
    });
    onEvent(FullUpdateProgressEvent::StageChanged {
        stage: FullUpdateStage::Ready,
        message: "Full update package is ready".to_string(),
    });
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn splitRanges(totalBytes: u64, threadCount: u64) -> Vec<(u64, u64)> {
    let mut ranges = Vec::new();
    let baseSize = totalBytes / threadCount;
    let remainder = totalBytes % threadCount;
    let mut start = 0;
    for index in 0..threadCount {
        let chunkSize = baseSize + u64::from(index < remainder);
        if chunkSize == 0 {
            continue;
        }
        let end = start + chunkSize - 1;
        ranges.push((start, end));
        start = end + 1;
    }
    ranges
}

#[cfg(not(target_arch = "wasm32"))]
fn downloadRangeToFile(
    url: &str,
    out: &Path,
    start: u64,
    end: u64,
    downloadedBytes: &AtomicU64,
) -> Result<(), String> {
    let client = blockingDownloadClient()?;
    let mut response = client
        .get(url)
        .header(USER_AGENT, "Operit")
        .header(RANGE, format!("bytes={start}-{end}"))
        .send()
        .map_err(|error| error.to_string())?;
    if response.status().as_u16() != 206 {
        return Err(format!(
            "HTTP {}: Range request failed for bytes={start}-{end}",
            response.status().as_u16()
        ));
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(out)
        .map_err(|error| error.to_string())?;
    use std::io::Seek;
    file.seek(std::io::SeekFrom::Start(start))
        .map_err(|error| error.to_string())?;
    let mut buffer = [0u8; BUFFER_SIZE];
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
        downloadedBytes.fetch_add(read as u64, Ordering::SeqCst);
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn blockingDownloadClient() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{FullUpdateTarget, GithubReleaseUtil};

    #[test]
    fn builds_cli_asset_names_from_release_spec() {
        let windows = FullUpdateTarget::new("cli", "windows", "x86_64").unwrap();
        assert_eq!(
            windows.assetName().unwrap(),
            "operit2-cli-windows-x86_64.zip"
        );

        let linux = FullUpdateTarget::new("cli", "linux", "aarch64").unwrap();
        assert_eq!(
            linux.assetName().unwrap(),
            "operit2-cli-linux-aarch64.tar.gz"
        );
    }

    #[test]
    fn builds_app_asset_names_from_release_spec() {
        let android = FullUpdateTarget::new("app", "android", "arm64-v8a").unwrap();
        assert_eq!(
            android.assetName().unwrap(),
            "operit2-app-android-arm64-v8a.apk"
        );

        let macos = FullUpdateTarget::new("app", "macos", "x86_64").unwrap();
        assert_eq!(
            macos.assetName().unwrap(),
            "operit2-app-macos-x86_64.tar.gz"
        );
    }

    #[test]
    fn rejects_unsupported_targets() {
        assert!(FullUpdateTarget::new("cli", "android", "arm64-v8a").is_err());
        assert!(FullUpdateTarget::new("app", "windows", "x86").is_err());
    }

    #[test]
    fn compares_release_tags_with_build_number() {
        assert!(GithubReleaseUtil::compareVersions("v1.0.0+2", "1.0.0+1") > 0);
        assert!(GithubReleaseUtil::compareVersions("v1.0.1+1", "1.0.0+99") > 0);
        assert_eq!(GithubReleaseUtil::compareVersions("v1.0.0+1", "1.0.0+1"), 0);
    }
}
