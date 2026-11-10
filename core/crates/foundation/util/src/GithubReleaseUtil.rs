use crate::UpdateHttp::{request as requestUpdateHttp, UpdateHttpEvent};
use operit_host_api::HostManager::{defaultHttpHost, HostManager};
use operit_host_api::{
    ApplicationUpdatePackage, HttpDownloadControl, HttpRequestData, RuntimeStorageHost,
    RuntimeStorageWriteSession, SystemOperationHost,
};
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use std::cmp::Ordering as CmpOrdering;

const GITHUB_RELEASE_OWNER: &str = "AAswordman";
const GITHUB_RELEASE_REPO: &str = "Operit2";
const GITHUB_RELEASE_PAGE_SIZE: usize = 30;
const GITHUB_API_BASE: &str = "https://api.github.com";

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

#[cfg(test)]
#[path = "GithubReleaseDownloadTests.rs"]
mod downloadTests;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Product, platform, and architecture tuple used to select update assets.
pub struct FullUpdateTarget {
    pub product: String,
    pub platform: String,
    pub arch: String,
    macosInstallerType: Option<String>,
    windowsMsi: bool,
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

    /// Reads release pages asynchronously through the configured HTTP Host.
    async fn findLatestReleaseInfo(
        repoOwner: &str,
        repoName: &str,
        target: FullUpdateTarget,
        channel: FullUpdateChannel,
        authorizationHeader: Option<String>,
    ) -> Result<Option<ReleaseInfo>, String> {
        let authorizationHeader = authorizationHeader.as_deref();
        selectLatestReleaseInfoFromPages(&target, channel, |page| async move {
            let url = format!(
                "{GITHUB_API_BASE}/repos/{repoOwner}/{repoName}/releases?page={page}&per_page={GITHUB_RELEASE_PAGE_SIZE}"
            );
            let mut request = updateHttpRequest(&url, "GET");
            request.headers = vec![
                ("User-Agent".into(), "Operit-MCP-Client".into()),
                ("Accept".into(), "application/vnd.github.v3+json".into()),
            ];
            if let Some(header) = authorizationHeader {
                request.headers.push(("Authorization".into(), header.to_string()));
            }
            let bytes = Arc::new(Mutex::new(Vec::new()));
            let body = bytes.clone();
            requestUpdateHttp(defaultHttpHost(), request, HttpDownloadControl::new(), move |event| {
                if let UpdateHttpEvent::Bytes(chunk) = event {
                    let mut bytes = body.lock().map_err(|error| error.to_string())?;
                    if bytes.len().saturating_add(chunk.len()) > 16 * 1024 * 1024 {
                        return Err("Release metadata exceeds the response size limit".into());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Ok(())
            }).await?;
            let releases: Vec<GitHubRelease> = serde_json::from_slice(
                &bytes.lock().map_err(|error| error.to_string())?
            ).map_err(|error| error.to_string())?;
            Ok(releases)
        }).await
    }

    /// Downloads into virtual runtime storage and returns its logical storage path.
    pub async fn downloadAndPrepareFullUpdateWithProgress<F>(
        packageUrl: String,
        packageFileName: String,
        context: &HostManager,
        onEvent: F,
    ) -> Result<String, String>
    where
        F: Fn(FullUpdateProgressEvent) + Send + Sync + 'static,
    {
        Self::downloadAndPrepareFullUpdateControlled(
            packageUrl,
            packageFileName,
            context,
            HttpDownloadControl::new(),
            onEvent,
        )
        .await
    }

    /// The app, CLI and TUI share host HTTP, cancellation and package validation.
    pub async fn downloadAndPrepareFullUpdateControlled<F>(
        packageUrl: String,
        packageFileName: String,
        context: &HostManager,
        control: HttpDownloadControl,
        onEvent: F,
    ) -> Result<String, String>
    where
        F: Fn(FullUpdateProgressEvent) + Send + Sync + 'static,
    {
        validateUpdatePackageSource(&packageUrl, &packageFileName)?;
        ensureDownloadActive(&control)?;
        let storage = context
            .runtimeStorageHost
            .clone()
            .ok_or("RuntimeStorageHost is required for updates")?;
        let attempt = format!(
            "{}/{}",
            crate::RuntimeStorageLayout::RUNTIME_APPLICATION_UPDATES_DIR_PATH,
            uuid::Uuid::new_v4()
        );
        if storage
            .exists(&attempt)
            .map_err(|error| error.to_string())?
        {
            return Err("Update attempt storage path already exists".into());
        }
        let mut cleanup = DownloadAttempt {
            storage,
            path: attempt.clone(),
            published: false,
        };
        let packageFile = format!("{attempt}/{packageFileName}");
        let events = Arc::new(onEvent);
        events(FullUpdateProgressEvent::StageChanged {
            stage: FullUpdateStage::DownloadingPackage,
            message: "Downloading full update package".into(),
        });
        ensureDownloadActive(&control)?;
        let progress = events.clone();
        downloadUpdatePackage(
            &packageUrl,
            &packageFile,
            &packageFileName,
            context,
            control.clone(),
            move |event| progress(event),
        )
        .await?;
        ensureDownloadActive(&control)?;
        events(FullUpdateProgressEvent::StageChanged {
            stage: FullUpdateStage::Ready,
            message: "Full update package is ready".into(),
        });
        ensureDownloadActive(&control)?;
        cleanup.published = true;
        Ok(packageFile)
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
            macosInstallerType: None,
            windowsMsi: false,
        })
    }

    /// Resolves installation identity through the local system host.
    pub fn appForHost(host: &dyn SystemOperationHost) -> Result<Option<Self>, String> {
        host.applicationUpdateTarget()
            .map_err(|error| error.to_string())?
            .map(Self::fromApplicationTarget)
            .transpose()
    }

    /// Maps host-owned package identities to the shared release naming rules.
    pub fn fromApplicationTarget(
        target: operit_host_api::ApplicationUpdateTarget,
    ) -> Result<Self, String> {
        match target.package {
            ApplicationUpdatePackage::AndroidApk => Self::app("android", &target.arch),
            ApplicationUpdatePackage::WindowsZip => Self::app("windows", &target.arch),
            ApplicationUpdatePackage::WindowsMsi => {
                let mut result = Self::app("windows", &target.arch)?;
                if result.arch != "x86_64" {
                    return Err("Windows MSI releases are only available for x86_64".into());
                }
                result.windowsMsi = true;
                Ok(result)
            }
            ApplicationUpdatePackage::LinuxTarGz => Self::app("linux", &target.arch),
            ApplicationUpdatePackage::MacosPkg { buildType } => {
                Self::app("macos", &target.arch)?.withMacosInstaller(&buildType)
            }
        }
    }
    /// Creates an app update target for a platform and architecture.
    pub fn app(platform: &str, arch: &str) -> Result<Self, String> {
        Self::new("app", platform, arch)
    }

    /// Selects the matching signed macOS installer instead of the legacy ZIP.
    pub fn withMacosInstaller(mut self, buildType: &str) -> Result<Self, String> {
        if self.product != "app"
            || self.platform != "macos"
            || !matches!(buildType, "sandbox" | "unsandbox")
        {
            return Err(format!("Invalid macOS App installer type: {buildType}"));
        }
        self.macosInstallerType = Some(buildType.to_string());
        Ok(self)
    }

    /// Builds the release asset name for the installed package version.
    pub fn assetName(&self) -> Result<String, String> {
        self.assetNameForVersion(env!("CARGO_PKG_VERSION"))
    }

    /// Builds the exact release filename from its full semantic version.
    pub fn assetNameForVersion(&self, releaseVersion: &str) -> Result<String, String> {
        validateTarget(&self.product, &self.platform, &self.arch)?;
        parseVersion(releaseVersion)?;
        let arch = match (self.product.as_str(), self.platform.as_str()) {
            ("app", "macos") => "universal",
            _ => self.arch.as_str(),
        };
        if let Some(buildType) = &self.macosInstallerType {
            return Ok(format!(
                "operit2-app-macos-universal-{buildType}-{}.pkg",
                releaseVersion.trim().trim_start_matches('v'),
            ));
        }
        Ok(format!(
            "operit2-{}-{}-{}-{}.{}",
            self.product,
            self.platform,
            arch,
            releaseVersion.trim().trim_start_matches('v'),
            if self.windowsMsi {
                "msi"
            } else {
                packageExtension(&self.product, &self.platform)?
            }
        ))
    }
}

/// Validates supported product, platform, and architecture tuples.
#[allow(non_snake_case)]
fn validateTarget(product: &str, platform: &str, arch: &str) -> Result<(), String> {
    match product {
        "app" | "cli" => {}
        _ => return Err(format!("Unsupported update product: {product}")),
    }
    match platform {
        "windows" | "linux" | "macos" | "android" | "ios" | "ohos" => {}
        _ => return Err(format!("Unsupported update platform: {platform}")),
    }
    if product == "cli" && matches!(platform, "android" | "ios" | "ohos") {
        return Err(format!(
            "Operit2 CLI update assets are not published for {platform}"
        ));
    }
    match platform {
        "ios" | "ohos" => match arch {
            "arm64" => Ok(()),
            _ => Err(format!("Unsupported {platform} update arch: {arch}")),
        },
        "macos" if product == "app" && arch == "universal" => Ok(()),
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

/// Returns the extension used by the platform packaging entry point.
#[allow(non_snake_case)]
fn packageExtension(product: &str, platform: &str) -> Result<&'static str, String> {
    match (product, platform) {
        ("app", "android") => Ok("apk"),
        ("app", "ohos") => Ok("hap"),
        ("app", "windows" | "macos" | "ios") | ("cli", "windows") => Ok("zip"),
        ("app", "linux") | ("cli", "linux" | "macos") => Ok("tar.gz"),
        _ => Err(format!("Unsupported update target: {product}/{platform}")),
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

/// Loads older release pages only until a compatible target package is found.
#[allow(non_snake_case)]
async fn selectLatestReleaseInfoFromPages<F, Fut>(
    target: &FullUpdateTarget,
    channel: FullUpdateChannel,
    mut fetchPage: F,
) -> Result<Option<ReleaseInfo>, String>
where
    F: FnMut(u32) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<GitHubRelease>, String>>,
{
    let mut page = 1;
    loop {
        let batch = fetchPage(page).await?;
        let isLastPage = batch.len() < GITHUB_RELEASE_PAGE_SIZE;
        if let Some(release) = selectLatestReleaseInfo(batch, target, channel)? {
            return Ok(Some(release));
        }
        if isLastPage {
            return Ok(None);
        }
        page += 1;
    }
}

/// Selects the newest release containing the exact target asset.
#[allow(non_snake_case)]
fn selectLatestReleaseInfo(
    releases: Vec<GitHubRelease>,
    target: &FullUpdateTarget,
    channel: FullUpdateChannel,
) -> Result<Option<ReleaseInfo>, String> {
    let mut selected: Option<(ParsedVersion, GitHubRelease, GitHubReleaseAsset)> = None;
    for release in releases {
        if release.draft {
            continue;
        }
        let expectedAssetName = target.assetNameForVersion(&release.tag_name)?;
        let Some(packageAsset) = release
            .assets
            .iter()
            .find(|asset| asset.name == expectedAssetName)
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

/// Rejects cancellation before opening or publishing a package.
fn ensureDownloadActive(control: &HttpDownloadControl) -> Result<(), String> {
    if control.isCancelled() {
        Err("UPDATE_CANCELLED".into())
    } else {
        Ok(())
    }
}

struct DownloadAttempt {
    storage: Arc<dyn RuntimeStorageHost>,
    path: String,
    published: bool,
}
impl Drop for DownloadAttempt {
    /// Removes only this attempt on errors, cancellation or a dropped future.
    fn drop(&mut self) {
        if !self.published {
            let _ = self.storage.delete(&self.path, true);
        }
    }
}

/// Validates the release source before creating a native file. Shared by app and CLI downloads.
fn validateUpdatePackageSource(url: &str, name: &str) -> Result<(), String> {
    validatePackageFileName(name)?;
    let prefix = "https://github.com/AAswordman/Operit2/releases/download/";
    let path = url
        .strip_prefix(prefix)
        .ok_or("Update URL must reference this project's HTTPS release assets")?;
    let (tag, asset) = path.split_once('/').ok_or("Missing release tag/asset")?;
    if tag.is_empty() || asset != name || !name.starts_with("operit2-") || name.contains(['?', '#'])
    {
        return Err("Invalid update release asset".into());
    }
    Ok(())
}

/// Rejects HTML error pages and mismatched archives before any installer sees them.
fn validateDownloadedUpdatePackage(prefix: &[u8], name: &str) -> Result<(), String> {
    let length = if name.ends_with(".msi") { 8 } else { 4 };
    if prefix.len() < length {
        return Err("Downloaded update package is too short".into());
    }
    let valid = if name.ends_with(".pkg") {
        prefix[..4] == *b"xar!"
    } else if name.ends_with(".msi") {
        prefix[..8] == [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]
    } else if name.ends_with(".tar.gz") {
        prefix[..2] == [0x1f, 0x8b]
    } else if [".zip", ".apk", ".hap", ".ipa"]
        .iter()
        .any(|ext| name.ends_with(ext))
    {
        prefix[..4] == *b"PK\x03\x04"
    } else {
        false
    };
    if valid {
        Ok(())
    } else {
        Err("Downloaded content is not a valid update package".into())
    }
}

/// Builds update network policy once for requests routed through Host HTTP.
pub(crate) fn updateHttpRequest(url: &str, method: &str) -> HttpRequestData {
    HttpRequestData {
        url: url.into(),
        method: method.into(),
        headers: vec![("User-Agent".into(), "Operit".into())],
        body: Vec::new(),
        formFields: Vec::new(),
        fileParts: Vec::new(),
        connectTimeoutSeconds: 30,
        readTimeoutSeconds: 120,
        followRedirects: true,
        ignoreSsl: false,
        proxyHost: String::new(),
        proxyPort: 0,
    }
}

struct StagedUpdate {
    writer: Option<Box<dyn RuntimeStorageWriteSession>>,
    prefix: Vec<u8>,
    received: u64,
    total: u64,
}
impl Drop for StagedUpdate {
    /// Explicitly discards uncommitted data on every Host, including browser storage.
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            let _ = writer.discard();
        }
    }
}

/// Streams into a Host-owned virtual storage session and commits only a verified package.
async fn downloadUpdatePackage<F>(
    url: &str,
    out: &str,
    name: &str,
    context: &HostManager,
    control: HttpDownloadControl,
    onEvent: F,
) -> Result<(), String>
where
    F: Fn(FullUpdateProgressEvent) + Send + Sync + 'static,
{
    let writes = context
        .runtimeStorageWriteHost
        .as_ref()
        .ok_or("RuntimeStorageWriteHost is required for updates")?;
    let http = context
        .httpHost
        .clone()
        .ok_or("HttpHost is required for updates")?;
    let scheduler = context
        .hostRuntimeTaskSchedulerHost
        .clone()
        .ok_or("Host clock is required for updates")?;
    let state = Arc::new(Mutex::new(StagedUpdate {
        writer: Some(
            writes
                .createWriteSession(out)
                .map_err(|error| error.to_string())?,
        ),
        prefix: Vec::with_capacity(8),
        received: 0,
        total: 0,
    }));
    let sink = state.clone();
    let mut lastTime = scheduler
        .monotonicTimeMillis()
        .map_err(|error| error.to_string())?;
    let active = control.clone();
    requestUpdateHttp(
        http,
        updateHttpRequest(url, "GET"),
        control.clone(),
        move |event| {
            ensureDownloadActive(&active)?;
            let mut state = sink.lock().map_err(|error| error.to_string())?;
            match event {
                UpdateHttpEvent::Head(head) => {
                    state.total = head
                        .headers
                        .iter()
                        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .ok_or("Update response has no Content-Length")?
                        .1
                        .parse::<u64>()
                        .map_err(|error| error.to_string())?;
                    if state.total == 0 {
                        return Err("Update package Content-Length must be positive".into());
                    }
                }
                UpdateHttpEvent::Bytes(bytes) => {
                    let next = state
                        .received
                        .checked_add(bytes.len() as u64)
                        .ok_or("Update byte count overflow")?;
                    if next > state.total {
                        return Err("Update package exceeds Content-Length".into());
                    }
                    state
                        .writer
                        .as_mut()
                        .expect("active update session")
                        .writeChunk(&bytes)
                        .map_err(|error| error.to_string())?;
                    let prefixBytes = bytes.len().min(8 - state.prefix.len());
                    state.prefix.extend_from_slice(&bytes[..prefixBytes]);
                    let now = scheduler
                        .monotonicTimeMillis()
                        .map_err(|error| error.to_string())?;
                    let speed = (next - state.received).saturating_mul(1000)
                        / now.saturating_sub(lastTime).max(1);
                    lastTime = now;
                    state.received = next;
                    let event = FullUpdateProgressEvent::DownloadProgress {
                        readBytes: state.received,
                        totalBytes: state.total,
                        speedBytesPerSec: speed,
                    };
                    drop(state);
                    onEvent(event);
                }
            }
            Ok(())
        },
    )
    .await?;
    ensureDownloadActive(&control)?;
    let mut state = state.lock().map_err(|error| error.to_string())?;
    if state.received != state.total || state.total == 0 {
        return Err("Update package is incomplete".into());
    }
    validateDownloadedUpdatePackage(&state.prefix, name)?;
    state
        .writer
        .take()
        .expect("active update session")
        .commit()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        selectLatestReleaseInfo, selectLatestReleaseInfoFromPages, FullUpdateChannel,
        FullUpdateTarget, GitHubRelease, GitHubReleaseAsset, GithubReleaseUtil,
        GITHUB_RELEASE_PAGE_SIZE,
    };

    #[test]
    fn shared_update_source_validation_rejects_foreign_or_traversing_assets() {
        let name = "operit2-app-linux-x86_64-2.0.0.tar.gz";
        assert!(super::validateUpdatePackageSource(
            &format!("https://github.com/AAswordman/Operit2/releases/download/v2.0.0/{name}"),
            name
        )
        .is_ok());
        assert!(super::validateUpdatePackageSource("https://evil.example/install", name).is_err());
        assert!(super::validateUpdatePackageSource(
            "https://github.com/AAswordman/Operit2/releases/download/v2/a",
            "../a"
        )
        .is_err());
        assert!(super::validateUpdatePackageSource(
            "https://github.com/AAswordman/Operit2/releases/download/v2/wrong",
            name
        )
        .is_err());
    }
    #[test]
    fn shared_package_validation_rejects_html_and_accepts_native_headers() {
        for (name, header) in [
            ("app.pkg", &b"xar!"[..]),
            ("cli.tar.gz", &b"\x1f\x8bxx"[..]),
            ("app.apk", &b"PK\x03\x04"[..]),
            ("app.msi", &b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1"[..]),
        ] {
            assert!(super::validateDownloadedUpdatePackage(b"<html>error</html>", name).is_err());
            assert!(super::validateDownloadedUpdatePackage(b"", name).is_err());
            super::validateDownloadedUpdatePackage(header, name).unwrap();
        }
    }

    #[test]
    fn macos_installer_matches_distribution_and_channel() {
        for buildType in ["sandbox", "unsandbox"] {
            let target = FullUpdateTarget::app("macos", "aarch64")
                .unwrap()
                .withMacosInstaller(buildType)
                .unwrap();
            assert_eq!(
                target.assetNameForVersion("v2.0.0-preview.35").unwrap(),
                format!("operit2-app-macos-universal-{buildType}-2.0.0-preview.35.pkg")
            );
            let other = FullUpdateTarget::app("macos", "aarch64")
                .unwrap()
                .withMacosInstaller(if buildType == "sandbox" {
                    "unsandbox"
                } else {
                    "sandbox"
                })
                .unwrap();
            let releases = vec![
                release_for_target("2.0.0-preview.36", &other),
                release_for_target("2.0.0-preview.35", &target),
            ];
            let selected =
                selectLatestReleaseInfo(releases.clone(), &target, FullUpdateChannel::Preview)
                    .unwrap()
                    .unwrap();
            assert_eq!(selected.version, "2.0.0-preview.35");
            assert!(
                selectLatestReleaseInfo(releases, &target, FullUpdateChannel::Stable)
                    .unwrap()
                    .is_none()
            );
        }
        assert!(FullUpdateTarget::new("cli", "linux", "x86_64")
            .unwrap()
            .withMacosInstaller("sandbox")
            .is_err());
        assert!(FullUpdateTarget::app("macos", "aarch64")
            .unwrap()
            .withMacosInstaller("other")
            .is_err());
    }

    /// Builds a release containing one exact platform package for discovery tests.
    fn release_for_target(version: &str, target: &FullUpdateTarget) -> GitHubRelease {
        GitHubRelease {
            tag_name: format!("v{version}"),
            body: None,
            html_url: format!("https://example.com/releases/v{version}"),
            draft: false,
            prerelease: !super::parseVersion(version)
                .unwrap()
                .prerelease
                .identifiers
                .is_empty(),
            assets: vec![GitHubReleaseAsset {
                name: target.assetNameForVersion(version).unwrap(),
                browser_download_url: "https://example.com/update".to_string(),
            }],
        }
    }

    /// Keeps MSI-managed apps on MSI assets even when a newer ZIP release exists.
    #[test]
    fn windows_installation_identity_selects_the_exact_package_format() {
        use operit_host_api::{ApplicationUpdatePackage, ApplicationUpdateTarget};
        let msi = FullUpdateTarget::fromApplicationTarget(ApplicationUpdateTarget {
            installTiming: operit_host_api::AppInstallTiming::Immediate,
            arch: "x86_64".into(),
            package: ApplicationUpdatePackage::WindowsMsi,
        })
        .unwrap();
        let zip = FullUpdateTarget::fromApplicationTarget(ApplicationUpdateTarget {
            installTiming: operit_host_api::AppInstallTiming::Immediate,
            arch: "x86_64".into(),
            package: ApplicationUpdatePackage::WindowsZip,
        })
        .unwrap();
        let releases = vec![
            release_for_target("2.0.0-preview.32", &zip),
            release_for_target("2.0.0-preview.31", &msi),
        ];
        let selected = selectLatestReleaseInfo(releases.clone(), &msi, FullUpdateChannel::Preview)
            .unwrap()
            .unwrap();
        assert_eq!(
            selected.assetName,
            "operit2-app-windows-x86_64-2.0.0-preview.31.msi"
        );
        assert_eq!(
            selectLatestReleaseInfo(releases, &zip, FullUpdateChannel::Preview)
                .unwrap()
                .unwrap()
                .version,
            "2.0.0-preview.32"
        );
        assert!(selectLatestReleaseInfo(
            vec![release_for_target("2.0.0-preview.32", &zip)],
            &msi,
            FullUpdateChannel::Preview
        )
        .unwrap()
        .is_none());
        assert!(
            FullUpdateTarget::fromApplicationTarget(ApplicationUpdateTarget {
                installTiming: operit_host_api::AppInstallTiming::Immediate,
                arch: "aarch64".into(),
                package: ApplicationUpdatePackage::WindowsMsi,
            })
            .is_err()
        );
    }

    /// Finds a compatible older release beyond a full page of other architectures.
    #[tokio::test]
    async fn finds_older_target_package_on_next_page() {
        let target = FullUpdateTarget::new("app", "windows", "aarch64").unwrap();
        let other = FullUpdateTarget::new("app", "windows", "x86_64").unwrap();
        let mut requested_pages = Vec::new();
        let selected =
            selectLatestReleaseInfoFromPages(&target, FullUpdateChannel::Preview, |page| {
                std::future::ready({
                    requested_pages.push(page);
                    match page {
                        1 => Ok(vec![
                            release_for_target("2.0.0-preview.30", &other);
                            GITHUB_RELEASE_PAGE_SIZE
                        ]),
                        2 => Ok(vec![release_for_target("2.0.0-preview.20", &target)]),
                        _ => panic!("unexpected release page {page}"),
                    }
                })
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(requested_pages, vec![1, 2]);
        assert_eq!(selected.version, "2.0.0-preview.20");
        assert_eq!(
            selected.assetName,
            target.assetNameForVersion(&selected.version).unwrap()
        );
        assert!(
            GithubReleaseUtil::compareVersions(&selected.version, "2.0.0-preview.19").unwrap() > 0
        );
        assert!(
            GithubReleaseUtil::compareVersions(&selected.version, "2.0.0-preview.21").unwrap() < 0
        );
    }

    /// Stops after the first matching page even when that page is full.
    #[tokio::test]
    async fn stops_loading_pages_when_target_package_is_found() {
        let target = FullUpdateTarget::new("cli", "linux", "aarch64").unwrap();
        let selected =
            selectLatestReleaseInfoFromPages(&target, FullUpdateChannel::Preview, |page| {
                std::future::ready({
                    assert_eq!(page, 1, "older pages must not be fetched after a match");
                    let mut releases = vec![
                        release_for_target("2.0.0-preview.9", &target);
                        GITHUB_RELEASE_PAGE_SIZE
                    ];
                    releases[1] = release_for_target("2.0.0-preview.16", &target);
                    Ok(releases)
                })
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(selected.version, "2.0.0-preview.16");
    }

    /// Loads older pages only when the installed channel rejects the current page.
    #[tokio::test]
    async fn loads_older_pages_only_for_compatible_channel() {
        let target = FullUpdateTarget::new("cli", "linux", "aarch64").unwrap();
        for (channel, expected, expected_pages) in [
            (FullUpdateChannel::Preview, "2.1.0-preview.1", vec![1]),
            (FullUpdateChannel::Stable, "2.0.1", vec![1, 2]),
        ] {
            let mut requested_pages = Vec::new();
            let selected = selectLatestReleaseInfoFromPages(&target, channel, |page| {
                std::future::ready({
                    requested_pages.push(page);
                    match page {
                        1 => Ok(vec![
                            release_for_target("2.1.0-preview.1", &target);
                            GITHUB_RELEASE_PAGE_SIZE
                        ]),
                        2 => Ok(vec![release_for_target("2.0.1", &target)]),
                        _ => panic!("unexpected release page {page}"),
                    }
                })
            })
            .await
            .unwrap()
            .unwrap();
            assert_eq!(selected.version, expected);
            assert_eq!(requested_pages, expected_pages);
        }
    }

    /// Exhausts a full final page before reporting that no compatible package exists.
    #[tokio::test]
    async fn returns_no_package_after_exhausting_pages() {
        let target = FullUpdateTarget::new("app", "windows", "aarch64").unwrap();
        let other = FullUpdateTarget::new("cli", "windows", "aarch64").unwrap();
        let mut requested_pages = Vec::new();
        let selected =
            selectLatestReleaseInfoFromPages(&target, FullUpdateChannel::Preview, |page| {
                std::future::ready({
                    requested_pages.push(page);
                    match page {
                        1 => Ok(vec![
                            release_for_target("2.0.0", &other);
                            GITHUB_RELEASE_PAGE_SIZE
                        ]),
                        2 => Ok(Vec::new()),
                        _ => panic!("unexpected release page {page}"),
                    }
                })
            })
            .await
            .unwrap();
        assert_eq!(requested_pages, vec![1, 2]);
        assert!(selected.is_none());
    }

    /// Reports a network error encountered while loading a required older page.
    #[tokio::test]
    async fn propagates_later_page_errors() {
        let target = FullUpdateTarget::new("cli", "windows", "x86_64").unwrap();
        let other = FullUpdateTarget::new("cli", "windows", "aarch64").unwrap();
        let error = selectLatestReleaseInfoFromPages(&target, FullUpdateChannel::Preview, |page| {
            std::future::ready(match page {
                1 => Ok(vec![
                    release_for_target("2.0.0", &other);
                    GITHUB_RELEASE_PAGE_SIZE
                ]),
                2 => Err("HTTP 403".to_string()),
                _ => panic!("unexpected release page {page}"),
            })
        })
        .await
        .unwrap_err();
        assert_eq!(error, "HTTP 403");
    }

    /// Builds installed-version CLI names for the supported desktop targets.
    #[test]
    fn builds_cli_asset_names_from_release_spec() {
        let windows = FullUpdateTarget::new("cli", "windows", "x86_64").unwrap();
        assert_eq!(
            windows.assetName().unwrap(),
            format!(
                "operit2-cli-windows-x86_64-{}.zip",
                env!("CARGO_PKG_VERSION")
            )
        );

        let linux = FullUpdateTarget::new("cli", "linux", "aarch64").unwrap();
        assert_eq!(
            linux.assetName().unwrap(),
            format!(
                "operit2-cli-linux-aarch64-{}.tar.gz",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    /// Builds installed-version App names with the packaged platform architecture.
    #[test]
    fn builds_app_asset_names_from_release_spec() {
        let android = FullUpdateTarget::new("app", "android", "arm64-v8a").unwrap();
        assert_eq!(
            android.assetName().unwrap(),
            format!(
                "operit2-app-android-arm64-v8a-{}.apk",
                env!("CARGO_PKG_VERSION")
            )
        );

        let macos = FullUpdateTarget::new("app", "macos", "x86_64").unwrap();
        assert_eq!(
            macos.assetName().unwrap(),
            format!(
                "operit2-app-macos-universal-{}.zip",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn rejects_unsupported_targets() {
        assert!(FullUpdateTarget::new("cli", "android", "arm64-v8a").is_err());
        assert!(FullUpdateTarget::new("app", "windows", "x86").is_err());
    }

    /// Selects a release by its own version and exact product target.
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
                name: "operit2-cli-windows-x86_64-2.0.0-preview.1.zip".to_string(),
                browser_download_url:
                    "https://github.com/AAswordman/Operit2/releases/download/v2.0.0-preview.1/operit2-cli-windows-x86_64-2.0.0-preview.1.zip"
                        .to_string(),
            }],
        }];

        let cli = selectLatestReleaseInfo(
            releases.clone(),
            &FullUpdateTarget::new("cli", "windows", "x86_64").unwrap(),
            FullUpdateChannel::Preview,
        )
        .unwrap();
        assert_eq!(cli.unwrap().version, "2.0.0-preview.1");

        let app = selectLatestReleaseInfo(
            releases,
            &FullUpdateTarget::new("app", "windows", "x86_64").unwrap(),
            FullUpdateChannel::Preview,
        )
        .unwrap();
        assert!(app.is_none());
    }

    /// Matches full versions, products, and architectures for every packaged platform.
    #[test]
    fn selects_all_platforms_by_exact_versioned_asset() {
        let targets = [
            ("app", "windows", "x86_64", "x86_64", "zip"),
            ("app", "windows", "aarch64", "aarch64", "zip"),
            ("app", "linux", "x86_64", "x86_64", "tar.gz"),
            ("app", "linux", "aarch64", "aarch64", "tar.gz"),
            ("app", "macos", "x86_64", "universal", "zip"),
            ("app", "macos", "aarch64", "universal", "zip"),
            ("app", "macos", "universal", "universal", "zip"),
            ("app", "android", "arm64-v8a", "arm64-v8a", "apk"),
            ("app", "android", "armeabi-v7a", "armeabi-v7a", "apk"),
            ("app", "android", "x86_64", "x86_64", "apk"),
            ("app", "ios", "arm64", "arm64", "zip"),
            ("app", "ohos", "arm64", "arm64", "hap"),
            ("cli", "windows", "x86_64", "x86_64", "zip"),
            ("cli", "windows", "aarch64", "aarch64", "zip"),
            ("cli", "linux", "x86_64", "x86_64", "tar.gz"),
            ("cli", "linux", "aarch64", "aarch64", "tar.gz"),
            ("cli", "macos", "x86_64", "x86_64", "tar.gz"),
            ("cli", "macos", "aarch64", "aarch64", "tar.gz"),
        ];
        for (product, platform, arch, packagedArch, extension) in targets {
            let target = FullUpdateTarget::new(product, platform, arch).unwrap();
            for version in ["2.0.0-preview.16", "2.0.0"] {
                let stem = format!("operit2-{product}-{platform}-{packagedArch}");
                let expected = format!("{stem}-{version}.{extension}");
                assert_eq!(target.assetNameForVersion(version).unwrap(), expected);
                for (name, accepted) in [
                    (expected.clone(), true),
                    (format!("{expected}.sig"), false),
                    (format!("{stem}-1.9.0.{extension}"), false),
                    (format!("{stem}.{extension}"), false),
                    (
                        format!("operit2-other-{platform}-{packagedArch}-{version}.{extension}"),
                        false,
                    ),
                ] {
                    let release = GitHubRelease {
                        tag_name: format!("v{version}"),
                        body: None,
                        html_url: "https://example.com/release".to_string(),
                        draft: false,
                        prerelease: version == "2.0.0-preview.16",
                        assets: vec![GitHubReleaseAsset {
                            name,
                            browser_download_url: "https://example.com/update".to_string(),
                        }],
                    };
                    let selected =
                        selectLatestReleaseInfo(vec![release], &target, FullUpdateChannel::Preview)
                            .unwrap();
                    assert_eq!(selected.is_some(), accepted, "{expected}");
                    if let Some(info) = selected {
                        assert_eq!(info.assetName, expected);
                        assert_eq!(info.version, version);
                    }
                }
            }
        }
    }

    /// Orders matching packages by release SemVer rather than lexical filenames.
    #[test]
    fn selects_newest_versioned_package_for_channel() {
        let target = FullUpdateTarget::new("cli", "linux", "aarch64").unwrap();
        let releases = ["2.0.0-preview.9", "2.0.0-preview.16", "2.0.0"]
            .into_iter()
            .map(|version| GitHubRelease {
                tag_name: format!("v{version}"),
                body: None,
                html_url: "https://example.com/release".to_string(),
                draft: false,
                prerelease: version != "2.0.0",
                assets: vec![GitHubReleaseAsset {
                    name: target.assetNameForVersion(version).unwrap(),
                    browser_download_url: "https://example.com/update".to_string(),
                }],
            })
            .collect::<Vec<_>>();
        for channel in [FullUpdateChannel::Preview, FullUpdateChannel::Stable] {
            let selected = selectLatestReleaseInfo(releases.clone(), &target, channel)
                .unwrap()
                .unwrap();
            assert_eq!(selected.version, "2.0.0");
        }
        let selected =
            selectLatestReleaseInfo(releases[..2].to_vec(), &target, FullUpdateChannel::Preview)
                .unwrap()
                .unwrap();
        assert_eq!(selected.version, "2.0.0-preview.16");
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
