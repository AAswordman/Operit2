//! Authenticated patch graph discovery and minimum-download routes.
use super::*;
use std::collections::{BTreeMap, HashSet};

pub(super) struct ChainPackage {
    pub release: ReleaseInfo,
    pub manifest: UpdateManifest,
}

#[derive(Clone, Default)]
pub(super) struct PatchRoute {
    pub steps: Vec<(usize, usize)>,
    pub bytes: u64,
}

/// Extracts the installed version only from the selected platform's exact asset family.
pub(super) fn currentVersion(release: &ReleaseInfo, currentAsset: &str) -> Result<String, String> {
    let (prefix, extension) = release
        .assetName
        .split_once(&format!("-{}.", release.version))
        .ok_or("Invalid update asset version")?;
    let version = currentAsset
        .strip_prefix(&format!("{prefix}-"))
        .and_then(|name| name.strip_suffix(&format!(".{extension}")))
        .ok_or("Installed package belongs to a different update target")?;
    GithubReleaseUtil::compareVersions(version, &release.version)?;
    Ok(version.to_owned())
}

/// Reads only historical versions referenced by published edges, without downloading package bytes.
pub(super) async fn discover(
    release: &ReleaseInfo,
    manifest: Option<&UpdateManifest>,
    current: &str,
    context: &HostManager,
    control: HttpDownloadControl,
) -> Result<Vec<ChainPackage>, String> {
    let Some(manifest) = manifest else {
        return Ok(Vec::new());
    };
    let mut packages = vec![ChainPackage {
        release: release.clone(),
        manifest: manifest.clone(),
    }];
    let mut visited = HashSet::from([release.version.clone(), current.to_owned()]);
    let stable = GithubReleaseUtil::fullUpdateChannelForVersion(current)?
        == crate::GithubReleaseUtil::FullUpdateChannel::Stable;
    let mut cursor = 0;
    while cursor < packages.len() {
        let package = &packages[cursor];
        let mut versions = Vec::new();
        for patch in &package.manifest.patches {
            if !package.release.patchAssetNames.contains(&patch.assetName)
                || GithubReleaseUtil::compareVersions(&patch.baseVersion, current)? <= 0
                || (stable
                    && GithubReleaseUtil::fullUpdateChannelForVersion(&patch.baseVersion)?
                        != crate::GithubReleaseUtil::FullUpdateChannel::Stable)
                || !visited.insert(patch.baseVersion.clone())
            {
                continue;
            }
            if visited.len() > 66 {
                return Err("Incremental update graph exceeds 64 historical versions".into());
            }
            versions.push(patch.baseVersion.clone());
        }
        for version in versions {
            ensureDownloadActive(&control)?;
            let asset = release
                .assetName
                .replace(&format!("-{}.", release.version), &format!("-{version}."));
            if let Some(historical) =
                GithubReleaseUtil::fetchPatchRelease(&version, &asset, context, control.clone())
                    .await?
            {
                if let Some(manifest) = fetchManifest(&historical, context, control.clone()).await?
                {
                    packages.push(ChainPackage {
                        release: historical,
                        manifest,
                    });
                }
            }
        }
        cursor += 1;
    }
    Ok(packages)
}

/// Finds the cheapest route per original identity in the strictly increasing version DAG.
pub(super) fn routes(
    packages: &[ChainPackage],
    current: &str,
    fullAvailable: bool,
) -> Result<Vec<PatchRoute>, String> {
    let Some(target) = packages.first() else {
        return Ok(Vec::new());
    };
    let mut reachable = BTreeMap::from([(
        (
            target.manifest.version.clone(),
            target.manifest.sha256.clone(),
            target.manifest.size,
        ),
        PatchRoute::default(),
    )]);
    let mut order: Vec<_> = (0..packages.len()).collect();
    order.sort_by(|left, right| {
        GithubReleaseUtil::compareVersions(
            &packages[*right].manifest.version,
            &packages[*left].manifest.version,
        )
        .expect("validated manifest versions")
        .cmp(&0)
    });
    for packageIndex in order {
        let package = &packages[packageIndex];
        let key = (
            package.manifest.version.clone(),
            package.manifest.sha256.clone(),
            package.manifest.size,
        );
        let Some(tail) = reachable.get(&key).cloned() else {
            continue;
        };
        for (patchIndex, patch) in package.manifest.patches.iter().enumerate() {
            if !package.release.patchAssetNames.contains(&patch.assetName)
                || GithubReleaseUtil::compareVersions(&patch.baseVersion, current)? < 0
            {
                continue;
            }
            let bytes = tail
                .bytes
                .checked_add(patch.size)
                .ok_or("Incremental route size overflow")?;
            if fullAvailable && bytes >= target.manifest.size {
                continue;
            }
            let mut steps = vec![(packageIndex, patchIndex)];
            steps.extend_from_slice(&tail.steps);
            let key = (
                patch.baseVersion.clone(),
                patch.baseSha256.clone(),
                patch.baseSize,
            );
            let replace = reachable
                .get(&key)
                .map(|old| (bytes, steps.len()) < (old.bytes, old.steps.len()))
                .unwrap_or(true);
            if replace {
                reachable.insert(key, PatchRoute { steps, bytes });
            }
        }
    }
    let mut routes: Vec<_> = reachable
        .into_iter()
        .filter(|((version, _, _), route)| version == current && !route.steps.is_empty())
        .map(|(_, route)| route)
        .collect();
    routes.sort_by_key(|route| (route.bytes, route.steps.len()));
    Ok(routes)
}
