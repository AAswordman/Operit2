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
use std::cmp::Ordering as CmpOrdering;
#[cfg(not(target_arch = "wasm32"))]
use std::error::Error;

const GITHUB_RELEASE_OWNER: &str = "AAswordman";
const GITHUB_RELEASE_REPO: &str = "Operit2";
#[cfg(not(target_arch = "wasm32"))]
const GITHUB_API_BASE: &str = "https://api.github.com";
#[cfg(not(target_arch = "wasm32"))]
const DOWNLOAD_THREAD_COUNT: u64 = 6;
#[cfg(not(target_arch = "wasm32"))]
const BUFFER_SIZE: usize = 128 * 1024;

#[derive(Debug, Clone, Default)]
/// Utility for GitHub release based full-update discovery and download.
pub struct GithubReleaseUtil;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Release metadata for a downloadable full update.
pub struct ReleaseInfo {
    pub version: String,
    pub assetName: String,
    pub downloadUrl: String,
    pub releaseNotes: String,
    pub releasePageUrl: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Product, platform, and architecture tuple used to select update assets.
pub struct FullUpdateTarget {
    pub product: String,
    pub platform: String,
    pub arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Result of checking whether a full update is available.
pub enum FullUpdateStatus {
    Available(ReleaseInfo),
    UpToDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Progress event emitted while preparing a full update package.
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
/// High-level stage of full update preparation.
pub enum FullUpdateStage {
    DownloadingPackage,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: PreRelease,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreRelease {
    identifiers: Vec<PreReleaseIdentifier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreReleaseIdentifier {
    Numeric(u64),
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Release channel used for full update selection.
pub enum FullUpdateChannel {
    Stable,
    Preview,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[allow(non_snake_case)]
impl GithubReleaseUtil {
    /// Checks GitHub releases asynchronously for a newer full update.
    pub async fn checkForFullUpdate(
        currentVersion: &str,
        target: FullUpdateTarget,
    ) -> Result<FullUpdateStatus, String> {
        Self::checkForFullUpdateWithAuthHeader(currentVersion, target, None).await
    }

    /// Checks GitHub releases asynchronously with an explicit authorization header.
    pub async fn checkForFullUpdateWithAuthHeader(
        currentVersion: &str,
        target: FullUpdateTarget,
        authorizationHeader: Option<String>,
    ) -> Result<FullUpdateStatus, String> {
        let current = parseVersion(currentVersion)?;
        let channel = channelForCurrentVersion(&current);
        let Some(releaseInfo) = Self::findLatestReleaseInfo(
            GITHUB_RELEASE_OWNER,
            GITHUB_RELEASE_REPO,
            target,
            channel,
            authorizationHeader,
        )
        .await?
        else {
            return Ok(FullUpdateStatus::UpToDate);
        };
        if Self::compareVersions(&releaseInfo.version, currentVersion)? > 0 {
            Ok(FullUpdateStatus::Available(releaseInfo))
        } else {
            Ok(FullUpdateStatus::UpToDate)
        }
    }

    /// Checks GitHub releases synchronously for a newer full update.
    pub fn checkForFullUpdateBlocking(
        currentVersion: &str,
        target: FullUpdateTarget,
    ) -> Result<FullUpdateStatus, String> {
        Self::checkForFullUpdateBlockingWithAuthHeader(currentVersion, target, None)
    }

    /// Checks GitHub releases synchronously with an explicit authorization header.
    pub fn checkForFullUpdateBlockingWithAuthHeader(
        currentVersion: &str,
        target: FullUpdateTarget,
        authorizationHeader: Option<String>,
    ) -> Result<FullUpdateStatus, String> {
        let current = parseVersion(currentVersion)?;
        let channel = channelForCurrentVersion(&current);
        let Some(releaseInfo) = Self::findLatestReleaseInfoBlocking(
            GITHUB_RELEASE_OWNER,
            GITHUB_RELEASE_REPO,
            target,
            channel,
            authorizationHeader.as_deref(),
        )?
        else {
            return Ok(FullUpdateStatus::UpToDate);
        };
        if Self::compareVersions(&releaseInfo.version, currentVersion)? > 0 {
            Ok(FullUpdateStatus::Available(releaseInfo))
        } else {
            Ok(FullUpdateStatus::UpToDate)
        }
    }

    /// Fetches the latest release info for a target asset and channel.
    pub async fn fetchLatestReleaseInfo(
        repoOwner: &str,
        repoName: &str,
        target: FullUpdateTarget,
        channel: FullUpdateChannel,
    ) -> Result<ReleaseInfo, String> {
        Self::fetchLatestReleaseInfoWithAuthHeader(repoOwner, repoName, target, channel, None).await
    }

    /// Fetches the latest release info with an explicit authorization header.
    pub async fn fetchLatestReleaseInfoWithAuthHeader(
        repoOwner: &str,
        repoName: &str,
        target: FullUpdateTarget,
        channel: FullUpdateChannel,
        authorizationHeader: Option<String>,
    ) -> Result<ReleaseInfo, String> {
        let targetAssetName = target.assetName()?;
        match Self::findLatestReleaseInfo(repoOwner, repoName, target, channel, authorizationHeader)
            .await?
        {
            Some(releaseInfo) => Ok(releaseInfo),
            None => Err(missingTargetAssetMessage(
                repoOwner,
                repoName,
                channel,
                &targetAssetName,
            )),
        }
    }

    async fn findLatestReleaseInfo(
        repoOwner: &str,
        repoName: &str,
        target: FullUpdateTarget,
        channel: FullUpdateChannel,
        authorizationHeader: Option<String>,
    ) -> Result<Option<ReleaseInfo>, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = repoOwner;
            let _ = repoName;
            let _ = target;
            let _ = channel;
            let _ = authorizationHeader;
            return Err("full update release query is not available in wasm runtime".to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let owner = repoOwner.to_string();
            let repo = repoName.to_string();
            tokio::task::spawn_blocking(move || {
                Self::findLatestReleaseInfoBlocking(
                    &owner,
                    &repo,
                    target,
                    channel,
                    authorizationHeader.as_deref(),
                )
            })
            .await
            .map_err(|error| error.to_string())?
        }
    }

    /// Downloads a full update package asynchronously and reports progress.
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

    /// Compares two semantic versions.
    pub fn compareVersions(v1: &str, v2: &str) -> Result<i32, String> {
        Ok(orderToI32(compareParsedVersions(
            &parseVersion(v1)?,
            &parseVersion(v2)?,
        )))
    }

    /// Determines the update channel associated with a version string.
    pub fn fullUpdateChannelForVersion(version: &str) -> Result<FullUpdateChannel, String> {
        Ok(channelForCurrentVersion(&parseVersion(version)?))
    }

    /// Fetches the latest release info synchronously for a target asset and channel.
    pub fn fetchLatestReleaseInfoBlocking(
        repoOwner: &str,
        repoName: &str,
        target: FullUpdateTarget,
        channel: FullUpdateChannel,
    ) -> Result<ReleaseInfo, String> {
        Self::fetchLatestReleaseInfoBlockingWithAuthHeader(
            repoOwner, repoName, target, channel, None,
        )
    }

    /// Fetches the latest release info synchronously with an explicit authorization header.
    pub fn fetchLatestReleaseInfoBlockingWithAuthHeader(
        repoOwner: &str,
        repoName: &str,
        target: FullUpdateTarget,
        channel: FullUpdateChannel,
        authorizationHeader: Option<String>,
    ) -> Result<ReleaseInfo, String> {
        let targetAssetName = target.assetName()?;
        match Self::findLatestReleaseInfoBlocking(
            repoOwner,
            repoName,
            target,
            channel,
            authorizationHeader.as_deref(),
        )? {
            Some(releaseInfo) => Ok(releaseInfo),
            None => Err(missingTargetAssetMessage(
                repoOwner,
                repoName,
                channel,
                &targetAssetName,
            )),
        }
    }

    fn findLatestReleaseInfoBlocking(
        repoOwner: &str,
        repoName: &str,
        target: FullUpdateTarget,
        channel: FullUpdateChannel,
        authorizationHeader: Option<&str>,
    ) -> Result<Option<ReleaseInfo>, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = repoOwner;
            let _ = repoName;
            let _ = target;
            let _ = channel;
            let _ = authorizationHeader;
            return Err("full update release query is not available in wasm runtime".to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let targetAssetName = target.assetName()?;
            let url = format!(
                "{GITHUB_API_BASE}/repos/{repoOwner}/{repoName}/releases?page=1&per_page=30"
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
            if let Some(authHeader) = authorizationHeader {
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
            selectLatestReleaseInfo(releases, &targetAssetName, channel)
        }
    }

    /// Downloads a full update package synchronously and reports progress.
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
    /// Creates and validates a full-update target.
    pub fn new(product: &str, platform: &str, arch: &str) -> Result<Self, String> {
        validateTarget(product, platform, arch)?;
        Ok(Self {
            product: product.to_string(),
            platform: platform.to_string(),
            arch: arch.to_string(),
        })
    }

    /// Creates a CLI update target for the current host.
    pub fn cliForCurrentHost() -> Result<Self, String> {
        Self::new("cli", currentDesktopPlatform()?, currentDesktopArch()?)
    }

    /// Creates an app update target for a platform and architecture.
    pub fn app(platform: &str, arch: &str) -> Result<Self, String> {
        Self::new("app", platform, arch)
    }

    /// Builds the expected release asset name for this target.
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
fn parseVersion(v: &str) -> Result<ParsedVersion, String> {
    let s = v.trim().trim_start_matches('v');
    if s.is_empty() {
        return Err("Version is empty".to_string());
    }
    let (without_build, build) = splitOnceOptional(s, '+')?;
    if let Some(build) = build {
        validateBuildMetadata(build)?;
    }
    let (core, prerelease) = splitOnceOptional(without_build, '-')?;
    let coreParts = core.split('.').collect::<Vec<_>>();
    if coreParts.len() != 3 {
        return Err(format!("Version must use major.minor.patch: {v}"));
    }
    Ok(ParsedVersion {
        major: parseNumericIdentifier(coreParts[0], "major")?,
        minor: parseNumericIdentifier(coreParts[1], "minor")?,
        patch: parseNumericIdentifier(coreParts[2], "patch")?,
        prerelease: parsePreRelease(prerelease)?,
    })
}

#[allow(non_snake_case)]
fn splitOnceOptional(value: &str, separator: char) -> Result<(&str, Option<&str>), String> {
    let mut parts = value.split(separator);
    let first = parts
        .next()
        .ok_or_else(|| "Version parser received an empty segment".to_string())?;
    let second = parts.next();
    if parts.next().is_some() {
        return Err(format!(
            "Version contains more than one '{separator}' separator: {value}"
        ));
    }
    Ok((first, second))
}

#[allow(non_snake_case)]
fn parseNumericIdentifier(value: &str, name: &str) -> Result<u64, String> {
    if value.is_empty() {
        return Err(format!("Version {name} number is empty"));
    }
    if value.len() > 1 && value.starts_with('0') {
        return Err(format!("Version {name} number has a leading zero: {value}"));
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("Version {name} number is not numeric: {value}"));
    }
    value
        .parse::<u64>()
        .map_err(|error| format!("Version {name} number is invalid: {error}"))
}

#[allow(non_snake_case)]
fn parsePreRelease(value: Option<&str>) -> Result<PreRelease, String> {
    let Some(value) = value else {
        return Ok(PreRelease {
            identifiers: Vec::new(),
        });
    };
    if value.is_empty() {
        return Err("Version prerelease segment is empty".to_string());
    }
    let mut identifiers = Vec::new();
    for item in value.split('.') {
        if item.is_empty() {
            return Err(format!("Version prerelease identifier is empty: {value}"));
        }
        if !item
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(format!(
                "Version prerelease identifier contains invalid characters: {item}"
            ));
        }
        if item.bytes().all(|byte| byte.is_ascii_digit()) {
            identifiers.push(PreReleaseIdentifier::Numeric(parseNumericIdentifier(
                item,
                "prerelease",
            )?));
        } else {
            identifiers.push(PreReleaseIdentifier::Text(item.to_string()));
        }
    }
    Ok(PreRelease { identifiers })
}

#[allow(non_snake_case)]
fn validateBuildMetadata(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("Version build metadata is empty".to_string());
    }
    for item in value.split('.') {
        if item.is_empty() {
            return Err(format!(
                "Version build metadata identifier is empty: {value}"
            ));
        }
        if !item
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(format!(
                "Version build metadata identifier contains invalid characters: {item}"
            ));
        }
    }
    Ok(())
}

#[allow(non_snake_case)]
fn compareParsedVersions(left: &ParsedVersion, right: &ParsedVersion) -> CmpOrdering {
    left.major
        .cmp(&right.major)
        .then_with(|| left.minor.cmp(&right.minor))
        .then_with(|| left.patch.cmp(&right.patch))
        .then_with(|| comparePreRelease(&left.prerelease, &right.prerelease))
}

#[allow(non_snake_case)]
fn comparePreRelease(left: &PreRelease, right: &PreRelease) -> CmpOrdering {
    if left.identifiers.is_empty() && right.identifiers.is_empty() {
        return CmpOrdering::Equal;
    }
    if left.identifiers.is_empty() {
        return CmpOrdering::Greater;
    }
    if right.identifiers.is_empty() {
        return CmpOrdering::Less;
    }
    let pairCount = left.identifiers.len().min(right.identifiers.len());
    for index in 0..pairCount {
        let order =
            comparePreReleaseIdentifier(&left.identifiers[index], &right.identifiers[index]);
        if order != CmpOrdering::Equal {
            return order;
        }
    }
    left.identifiers.len().cmp(&right.identifiers.len())
}

#[allow(non_snake_case)]
fn comparePreReleaseIdentifier(
    left: &PreReleaseIdentifier,
    right: &PreReleaseIdentifier,
) -> CmpOrdering {
    match (left, right) {
        (PreReleaseIdentifier::Numeric(left), PreReleaseIdentifier::Numeric(right)) => {
            left.cmp(right)
        }
        (PreReleaseIdentifier::Numeric(_), PreReleaseIdentifier::Text(_)) => CmpOrdering::Less,
        (PreReleaseIdentifier::Text(_), PreReleaseIdentifier::Numeric(_)) => CmpOrdering::Greater,
        (PreReleaseIdentifier::Text(left), PreReleaseIdentifier::Text(right)) => left.cmp(right),
    }
}

#[allow(non_snake_case)]
fn orderToI32(order: CmpOrdering) -> i32 {
    match order {
        CmpOrdering::Less => -1,
        CmpOrdering::Equal => 0,
        CmpOrdering::Greater => 1,
    }
}

#[allow(non_snake_case)]
fn channelForCurrentVersion(version: &ParsedVersion) -> FullUpdateChannel {
    if version.prerelease.identifiers.is_empty() {
        FullUpdateChannel::Stable
    } else {
        FullUpdateChannel::Preview
    }
}

#[allow(non_snake_case)]
fn channelForReleaseVersion(version: &ParsedVersion) -> FullUpdateChannel {
    channelForCurrentVersion(version)
}

#[allow(non_snake_case)]
fn channelAccepts(current: FullUpdateChannel, release: FullUpdateChannel) -> bool {
    match current {
        FullUpdateChannel::Stable => release == FullUpdateChannel::Stable,
        FullUpdateChannel::Preview => true,
    }
}

#[allow(non_snake_case)]
fn validateReleaseChannelMarker(
    release: &GitHubRelease,
    parsed: &ParsedVersion,
) -> Result<(), String> {
    let isPrereleaseVersion = !parsed.prerelease.identifiers.is_empty();
    if isPrereleaseVersion && !release.prerelease {
        return Err(format!(
            "Release {} has a prerelease tag but is not marked prerelease on GitHub",
            release.tag_name
        ));
    }
    if !isPrereleaseVersion && release.prerelease {
        return Err(format!(
            "Release {} has a stable tag but is marked prerelease on GitHub",
            release.tag_name
        ));
    }
    Ok(())
}

#[allow(non_snake_case)]
fn selectLatestReleaseInfo(
    releases: Vec<GitHubRelease>,
    targetAssetName: &str,
    channel: FullUpdateChannel,
) -> Result<Option<ReleaseInfo>, String> {
    let mut selected: Option<(ParsedVersion, GitHubRelease, GitHubReleaseAsset)> = None;
    for release in releases {
        if release.draft {
            continue;
        }
        let Some(packageAsset) = release
            .assets
            .iter()
            .find(|asset| asset.name == targetAssetName)
            .cloned()
        else {
            continue;
        };
        let parsed = parseVersion(&release.tag_name)?;
        validateReleaseChannelMarker(&release, &parsed)?;
        if !channelAccepts(channel, channelForReleaseVersion(&parsed)) {
            continue;
        }
        let replace = match selected.as_ref() {
            Some((selectedVersion, _, _)) => {
                compareParsedVersions(&parsed, selectedVersion) == CmpOrdering::Greater
            }
            None => true,
        };
        if replace {
            selected = Some((parsed, release, packageAsset));
        }
    }
    Ok(
        selected.map(|(_, latestRelease, packageAsset)| ReleaseInfo {
            version: latestRelease.tag_name.trim_start_matches('v').to_string(),
            assetName: packageAsset.name.clone(),
            downloadUrl: packageAsset.browser_download_url.clone(),
            releaseNotes: latestRelease.body.unwrap_or_default(),
            releasePageUrl: latestRelease.html_url,
        }),
    )
}

#[allow(non_snake_case)]
fn missingTargetAssetMessage(
    repoOwner: &str,
    repoName: &str,
    channel: FullUpdateChannel,
    targetAssetName: &str,
) -> String {
    format!("No {channel} release found for {repoOwner}/{repoName} asset {targetAssetName}")
}

impl std::fmt::Display for FullUpdateChannel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FullUpdateChannel::Stable => formatter.write_str("stable"),
            FullUpdateChannel::Preview => formatter.write_str("preview"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fetchContentLength(url: &str) -> Result<u64, String> {
    let client = blockingDownloadClient()?;
    let response = client
        .get(url)
        .header(USER_AGENT, "Operit")
        .send()
        .map_err(|error| downloadRequestError("fetch content length", url, &error))?;
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
        .map_err(|error| downloadRequestError("verify range download", url, &error))?;
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
        .map_err(|error| {
            downloadRequestError(&format!("download bytes={start}-{end}"), url, &error)
        })?;
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

#[cfg(not(target_arch = "wasm32"))]
fn downloadRequestError(stage: &str, url: &str, error: &reqwest::Error) -> String {
    let mut message = format!("{stage} failed for {url}: {error}");
    let mut source = error.source();
    while let Some(error) = source {
        message.push_str(": ");
        message.push_str(&error.to_string());
        source = error.source();
    }
    message
}

#[cfg(test)]
mod tests {
    use super::{
        selectLatestReleaseInfo, FullUpdateChannel, FullUpdateTarget, GitHubRelease,
        GitHubReleaseAsset, GithubReleaseUtil,
    };

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
    fn selects_release_by_exact_target_asset() {
        let releases = vec![GitHubRelease {
            tag_name: "v2.0.0-preview.1".to_string(),
            body: Some("CLI preview".to_string()),
            html_url: "https://github.com/AAswordman/Operit2/releases/tag/v2.0.0-preview.1"
                .to_string(),
            draft: false,
            prerelease: true,
            assets: vec![GitHubReleaseAsset {
                name: "operit2-cli-windows-x86_64.zip".to_string(),
                browser_download_url:
                    "https://github.com/AAswordman/Operit2/releases/download/v2.0.0-preview.1/operit2-cli-windows-x86_64.zip"
                        .to_string(),
            }],
        }];

        let cli = selectLatestReleaseInfo(
            releases.clone(),
            "operit2-cli-windows-x86_64.zip",
            FullUpdateChannel::Preview,
        )
        .unwrap();
        assert_eq!(cli.unwrap().version, "2.0.0-preview.1");

        let app = selectLatestReleaseInfo(
            releases,
            "operit2-app-windows-x86_64.zip",
            FullUpdateChannel::Preview,
        )
        .unwrap();
        assert!(app.is_none());
    }

    #[test]
    fn compares_release_tags_with_build_number() {
        assert_eq!(
            GithubReleaseUtil::compareVersions(
                "v2.0.0+20260619.shaabcdef",
                "2.0.0+20260618.sha123456"
            )
            .unwrap(),
            0
        );
        assert!(GithubReleaseUtil::compareVersions("v2.0.1-dev.1", "2.0.0").unwrap() > 0);
    }

    #[test]
    fn compares_release_tags_with_prerelease_order() {
        assert!(
            GithubReleaseUtil::compareVersions("v2.0.0-preview.2", "2.0.0-preview.1").unwrap() > 0
        );
        assert!(GithubReleaseUtil::compareVersions("v2.0.0-rc.1", "2.0.0-preview.9").unwrap() > 0);
        assert!(GithubReleaseUtil::compareVersions("v2.0.0", "2.0.0-rc.9").unwrap() > 0);
        assert!(
            GithubReleaseUtil::compareVersions("v2.0.0-dev.20260619", "2.0.0-preview.1").unwrap()
                < 0
        );
    }

    #[test]
    fn rejects_invalid_release_tags() {
        assert!(GithubReleaseUtil::compareVersions("2.0", "2.0.0").is_err());
        assert!(GithubReleaseUtil::compareVersions("2.0.0-preview..1", "2.0.0").is_err());
        assert!(GithubReleaseUtil::compareVersions("2.0.0+build+extra", "2.0.0").is_err());
    }
}
