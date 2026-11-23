//! Automatic source selection for authenticated GitHub release assets on every Host.
use crate::GithubReleaseUtil::{
    ensureDownloadActive, updateHttpRequest, validateDownloadedUpdatePackage,
    validateUpdatePackageSource,
};
use crate::UpdateHttp::{requestOptional, UpdateHttpEvent};
use futures_util::{stream::FuturesUnordered, StreamExt};
use operit_host_api::{HostManager::HostManager, HttpDownloadControl};
use std::sync::{Arc, Mutex};

// Shared with Operit 1's GithubReleaseUtil.kt. Availability is measured per artifact.
const MIRRORS: &[&str] = &[
    "https://ghfast.top/",
    "https://ghproxy.com/",
    "https://ghproxy.net/",
    "https://mirror.ghproxy.com/",
    "https://gh-proxy.com/",
    "https://hub.gitmirror.com/",
    "https://github.moeyy.xyz/",
    "https://github.abskoop.workers.dev/",
    "https://gh.h233.eu.org/",
    "https://ghproxy.1888866.xyz/",
    "https://ghproxy.cfd/",
    "https://github.boki.moe/",
    "https://gh-proxy.net/",
    "https://gh.jasonzeng.dev/",
    "https://gh.monlor.com/",
    "https://fastgit.cc/",
    "https://github.tbedu.top/",
    "https://firewall.lxstd.org/",
    "https://github.ednovas.xyz/",
    "https://ghfile.geekertao.top/",
    "https://gh.chjina.com/",
    "https://ghpxy.hwinzniej.top/",
    "https://cdn.crashmc.com/",
    "https://git.yylx.win/",
    "https://gitproxy.mrhjx.cn/",
    "https://ghproxy.cxkpro.top/",
    "https://gh.xxooo.cf/",
    "https://github.limoruirui.com/",
    "https://gh.llkk.cc/",
    "https://down.npee.cn/?",
    "https://gh.nxnow.top/",
    "https://gh.zwy.one/",
    "https://ghproxy.monkeyray.net/",
    "https://gh.xx9527.cn/",
];
const SAMPLE_BYTES: usize = 64 * 1024;
const PROBE_TIMEOUT_MS: u64 = 3_000;
const SAMPLE_COMPLETE: &str = "UPDATE_SOURCE_SAMPLE_COMPLETE";

/// Derives candidates for a caller-validated public GitHub release asset URL.
fn sources(url: &str) -> Vec<String> {
    std::iter::once(url.to_string())
        .chain(MIRRORS.iter().map(|prefix| format!("{prefix}{url}")))
        .collect()
}

/// Checks sample bytes so HTML gateways never win a speed test.
fn validateSample(prefix: &[u8], name: &str) -> Result<(), String> {
    if name.ends_with(".update.json") {
        if prefix
            .iter()
            .copied()
            .find(|byte| !byte.is_ascii_whitespace())
            == Some(b'{')
        {
            return Ok(());
        }
        return Err("Download source returned invalid update metadata".into());
    }
    validateDownloadedUpdatePackage(prefix, name)
}

/// Measures useful bytes including connection time, with a bounded sample and Host-owned deadline.
async fn probe(
    url: &str,
    context: &HostManager,
    control: HttpDownloadControl,
    expectedRangeTotal: Option<u64>,
    validate: impl Fn(&[u8]) -> Result<(), String> + Send + 'static,
) -> Result<Option<u64>, String> {
    let http = context
        .httpHost
        .clone()
        .ok_or("HttpHost is required for updates")?;
    let clock = context
        .hostRuntimeTaskSchedulerHost
        .clone()
        .ok_or("Host clock is required for updates")?;
    let started = clock.monotonicTimeMillis().map_err(|e| e.to_string())?;
    let sample = Arc::new(Mutex::new((Vec::with_capacity(8), 0usize)));
    let sink = sample.clone();
    let mut request = updateHttpRequest(url, "GET");
    request.connectTimeoutSeconds = 3;
    request.readTimeoutSeconds = 3;
    request.headers.extend([
        ("Range".into(), format!("bytes=0-{}", SAMPLE_BYTES - 1)),
        ("Accept-Encoding".into(), "identity".into()),
    ]);
    let pending = requestOptional(http, request, control.clone(), move |event| {
        match event {
            UpdateHttpEvent::Head(head) => {
                if let Some(total) = expectedRangeTotal {
                    let range = format!("bytes 0-{}/{total}", SAMPLE_BYTES - 1);
                    if head.statusCode != 206 || !head.headers.iter().any(|(key, value)| {
                        key.eq_ignore_ascii_case("content-range") && value == &range
                    }) {
                        return Err("Download source does not serve the expected archive byte range".into());
                    }
                }
                if !matches!(head.statusCode, 200 | 206) {
                    return Err(format!(
                        "Unexpected download source status {}",
                        head.statusCode
                    ));
                }
                if head.headers.iter().any(|(key, value)| {
                    key.eq_ignore_ascii_case("content-type")
                        && value
                            .split(';')
                            .next()
                            .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("text/html"))
                }) {
                    return Err("Download source returned HTML".into());
                }
            }
            UpdateHttpEvent::Bytes(bytes) => {
                let mut sample = sink.lock().map_err(|e| e.to_string())?;
                let length = bytes.len().min(8 - sample.0.len());
                sample.0.extend_from_slice(&bytes[..length]);
                sample.1 = sample.1.saturating_add(bytes.len()).min(SAMPLE_BYTES);
                if sample.1 == SAMPLE_BYTES {
                    return Err(SAMPLE_COMPLETE.into());
                }
            }
        }
        Ok(())
    });
    let result = tokio::select! {
        biased;
        result = pending => result,
        result = clock.waitForHostRuntimeDelay(PROBE_TIMEOUT_MS) => {
            result.map_err(|e| e.to_string())?;
            if sample.lock().map_err(|e| e.to_string())?.1 >= 8 {
                Err(SAMPLE_COMPLETE.into())
            } else {
                Err("Download source timed out".into())
            }
        }
    };
    ensureDownloadActive(&control)?;
    match result {
        Ok(false) => return Ok(None),
        Err(error) if error != SAMPLE_COMPLETE => return Err(error),
        _ => {}
    }
    let sample = sample.lock().map_err(|e| e.to_string())?;
    validate(&sample.0)?;
    let elapsed = clock
        .monotonicTimeMillis()
        .map_err(|e| e.to_string())?
        .saturating_sub(started)
        .max(1);
    Ok(Some((sample.1 as u64).saturating_mul(1000) / elapsed))
}

/// Ranks reachable sources once; failed probes and later failed downloads are never retried in the attempt.
pub(crate) async fn ranked(
    url: &str,
    name: &str,
    context: &HostManager,
    control: HttpDownloadControl,
    optional: bool,
) -> Result<Vec<String>, String> {
    validateUpdatePackageSource(url, name)?;
    let name = name.to_owned();
    rankSources(url, context, control, optional, None, move |prefix| validateSample(prefix, &name)).await
}

/// Selects the fastest range-capable source for a host-pinned Termux XZ release asset.
pub async fn fastestTermuxRootfsSource(
    url: &str,
    expectedBytes: u64,
    context: &HostManager,
    control: HttpDownloadControl,
) -> Result<String, String> {
    let path = url.strip_prefix("https://github.com/termux/proot-distro/releases/download/")
        .ok_or("Rootfs URL must reference a Termux GitHub release asset")?;
    if expectedBytes < SAMPLE_BYTES as u64 || !path.ends_with(".tar.xz") || path.contains(['?', '#']) {
        return Err("Invalid Termux rootfs release asset".into());
    }
    rankSources(url, context, control, false, Some(expectedBytes), |prefix| {
        if prefix.starts_with(b"\xfd7zXZ\0") { Ok(()) } else { Err("Download source did not return an XZ archive".into()) }
    }).await?.into_iter().next().ok_or_else(|| "No reachable rootfs download source".into())
}

/// Measures all candidates concurrently and ranks their effective throughput.
async fn rankSources(
    url: &str,
    context: &HostManager,
    control: HttpDownloadControl,
    optional: bool,
    expectedRangeTotal: Option<u64>,
    validate: impl Fn(&[u8]) -> Result<(), String> + Clone + Send + 'static,
) -> Result<Vec<String>, String> {
    ensureDownloadActive(&control)?;
    let mut probes = FuturesUnordered::new();
    for source in sources(url) {
        let control = control.clone();
        let validate = validate.clone();
        probes.push(async move {
            let result = probe(&source, context, control, expectedRangeTotal, validate).await;
            (source, result)
        });
    }
    let mut available = Vec::new();
    let mut absent = false;
    let mut originalError = String::new();
    while let Some((source, result)) = probes.next().await {
        ensureDownloadActive(&control)?;
        match result {
            Ok(Some(speed)) => available.push((source, speed)),
            Ok(None) if source == url => {
                absent = true;
                originalError = "HTTP 404/410: update asset is unavailable".into();
            }
            Err(error) if source == url => originalError = error,
            _ => {}
        }
    }
    if available.is_empty() && !(optional && absent) {
        return Err(format!(
            "{originalError}; no reachable download source"
        ));
    }
    available.sort_by_key(|(_, speed)| std::cmp::Reverse(*speed));
    Ok(available.into_iter().map(|(source, _)| source).collect())
}
