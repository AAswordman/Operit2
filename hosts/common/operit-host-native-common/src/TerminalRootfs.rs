//! On-demand ARM64 root filesystems; archives are never part of the application bundle.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadFileRequest, HttpDownloadRequest,
    HttpHost, TerminalDistribution, TerminalInstallProgress, TerminalInstallProgressCallback,
    TerminalProfile,
};
use sha2::{Digest, Sha256};

use crate::NativeHttpHost;

const MAX_ROOTFS_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_ROOTFS_ENTRIES: usize = 500_000;

/// Lists pinned proot-distro archives with publisher-provided SHA-256 digests.
pub fn distributions() -> Vec<TerminalDistribution> {
    [
        (
            "ubuntu-24_04",
            "Ubuntu 24.04 LTS",
            "v4.18.0",
            "ubuntu-noble-aarch64-pd-v4.18.0.tar.xz",
            "91acaa786b8e2fbba56a9fd0f8a1188cee482b5c7baeed707b29ddaa9a294daa",
            64_133_552,
            "",
        ),
        (
            "debian-13",
            "Debian 13 (trixie)",
            "v4.29.0",
            "debian-trixie-aarch64-pd-v4.29.0.tar.xz",
            "3834a11cbc6496935760bdc20cca7e2c25724d0cd8f5e4926da8fd5ca1857918",
            35_409_704,
            "",
        ),
        (
            "fedora-42",
            "Fedora 42",
            "v4.29.0",
            "fedora-aarch64-pd-v4.29.0.tar.xz",
            "0f683e7c3b250660b490479f0c1c57e7450082f6ddc81375ddc863e611a76c65",
            33_788_404,
            "Upstream reports this image incompatible with Android 15 and later.",
        ),
    ]
    .into_iter()
    .map(
        |(id, name, release, file, sha256, downloadBytes, notice)| TerminalDistribution {
            id: id.into(),
            name: name.into(),
            architecture: "aarch64".into(),
            url: format!(
                "https://github.com/termux/proot-distro/releases/download/{release}/{file}"
            ),
            sha256: sha256.into(),
            downloadBytes,
            notice: notice.into(),
        },
    )
    .collect()
}

/// Fails promptly at each streaming boundary when the user cancels installation.
fn checkCancelled(control: &HttpDownloadControl) -> HostResult<()> {
    if control.isCancelled() {
        return Err(HostError::new("Terminal installation cancelled"));
    }
    Ok(())
}

/// Publishes one installation phase using small immutable progress messages.
fn report(progress: &TerminalInstallProgressCallback, state: &str, bytes: u64, total: u64) {
    progress(TerminalInstallProgress {
        state: state.into(),
        downloadedBytes: bytes,
        totalBytes: total,
    });
}

/// Downloads, verifies and publishes a fresh rootfs instance below the owning host's private root.
pub fn install(
    root: &Path,
    distributionId: &str,
    instanceId: &str,
    name: &str,
    terminal: &str,
    terminalType: &str,
    control: HttpDownloadControl,
    progress: TerminalInstallProgressCallback,
) -> HostResult<TerminalProfile> {
    if instanceId.is_empty()
        || !instanceId
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err(HostError::new("Invalid terminal instance identifier"));
    }
    if name.trim().is_empty() || name.contains('\0') {
        return Err(HostError::new(
            "Terminal instance name must not be blank or contain NUL",
        ));
    }
    let distribution = distributions()
        .into_iter()
        .find(|entry| entry.id == distributionId)
        .ok_or_else(|| HostError::new("Terminal distribution is not registered"))?;
    fs::create_dir_all(root)?;
    let root = root.canonicalize()?;
    let target = root.join(instanceId);
    let staging = root.join(format!(".{instanceId}.installing"));
    if target.exists() {
        return Err(HostError::new("Terminal instance already exists"));
    }
    fs::create_dir(&staging)?;
    let result = (|| {
        checkCancelled(&control)?;
        let archivePath = staging.join("archive.tar.xz");
        let downloadProgress = progress.clone();
        NativeHttpHost::new().downloadFiles(
            HttpDownloadRequest {
                downloadId: instanceId.into(),
                files: vec![HttpDownloadFileRequest {
                    fileId: instanceId.into(),
                    url: distribution.url.clone(),
                    targetPath: archivePath.to_string_lossy().into_owned(),
                    headers: Vec::new(),
                    expectedBytes: distribution.downloadBytes,
                }],
                maxConcurrency: 1,
                connectTimeoutSeconds: 30,
                readTimeoutSeconds: 120,
                followRedirects: true,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            },
            control.clone(),
            std::sync::Arc::new(move |value| {
                report(
                    &downloadProgress,
                    "downloading",
                    value.downloadedBytes,
                    value.totalBytes,
                );
            }),
        )?;
        report(
            &progress,
            "verifying",
            distribution.downloadBytes,
            distribution.downloadBytes,
        );
        verifyArchive(&archivePath, &distribution.sha256, &control)?;
        report(
            &progress,
            "extracting",
            distribution.downloadBytes,
            distribution.downloadBytes,
        );
        let extracted = staging.join("rootfs");
        fs::create_dir(&extracted)?;
        let decoder = lzma_rust2::XzReader::new(File::open(&archivePath)?, true);
        extract(decoder, &extracted, &control)?;
        validateArm64(&extracted)?;
        // DNS is provided explicitly; downloaded images may point resolv.conf at a systemd runtime.
        let resolver = extracted.join("etc/resolv.conf");
        if fs::symlink_metadata(&resolver).is_ok() {
            fs::remove_file(&resolver)?;
        }
        fs::write(&resolver, "nameserver 1.1.1.1\nnameserver 8.8.8.8\n")?;
        fs::create_dir_all(extracted.join("root"))?;
        checkCancelled(&control)?;
        fs::rename(&extracted, &target)?;
        Ok(TerminalProfile {
            id: instanceId.into(),
            name: name.trim().into(),
            terminal: terminal.into(),
            terminalType: terminalType.into(),
            options: BTreeMap::from([("rootfs".into(), target.to_string_lossy().into_owned())]),
        })
    })();
    // Both paths are literal children of a canonical host-owned directory; never accept user paths here.
    let cleanup = fs::remove_dir_all(&staging);
    match (result, cleanup) {
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(cleanup)) => Err(HostError::new(format!(
            "{}; cannot clean installation staging: {cleanup}",
            error.message
        ))),
        (Ok(_), Err(error)) => {
            discard(&root, instanceId).map_err(|cleanup| {
                HostError::new(format!(
                    "Cannot clean installation staging: {error}; {}",
                    cleanup.message
                ))
            })?;
            Err(HostError::new(format!(
                "Cannot clean installation staging: {error}"
            )))
        }
        (Ok(profile), Ok(())) => Ok(profile),
    }
}

/// Rolls back only a direct child instance in the canonical host-owned rootfs directory.
pub fn discard(root: &Path, instanceId: &str) -> HostResult<()> {
    if instanceId.is_empty()
        || !instanceId
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err(HostError::new("Invalid terminal instance identifier"));
    }
    let root = root.canonicalize()?;
    let target = root.join(instanceId);
    if fs::symlink_metadata(&target)?.file_type().is_symlink() {
        return Err(HostError::new(
            "Terminal rollback target must not be a symlink",
        ));
    }
    let resolved = target.canonicalize()?;
    if resolved.parent() != Some(root.as_path()) {
        return Err(HostError::new(
            "Terminal rollback target escapes the rootfs directory",
        ));
    }
    fs::remove_dir_all(resolved).map_err(Into::into)
}

/// Verifies the complete archive incrementally before parsing its compressed contents.
fn verifyArchive(path: &Path, expected: &str, control: &HttpDownloadControl) -> HostResult<()> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        checkCancelled(control)?;
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if format!("{:x}", digest.finalize()) != expected {
        return Err(HostError::new("Terminal rootfs SHA-256 mismatch"));
    }
    Ok(())
}

/// Removes the archive's single distribution prefix and rejects traversal or absolute member paths.
fn memberPath(path: &Path) -> HostResult<PathBuf> {
    let mut components = path.components().filter(|part| *part != Component::CurDir);
    if !matches!(components.next(), Some(Component::Normal(_))) {
        return Err(HostError::new(
            "Rootfs archive requires a distribution directory prefix",
        ));
    }
    let mut result = PathBuf::new();
    for part in components {
        match part {
            Component::Normal(value) => result.push(value),
            _ => return Err(HostError::new("Rootfs archive member escapes its instance")),
        }
    }
    Ok(result)
}

/// Resolves guest link targets lexically inside the new instance without following host symlinks.
fn guestLink(member: &Path, target: &Path) -> HostResult<PathBuf> {
    let mut result = if target.has_root() {
        PathBuf::new()
    } else {
        member
            .parent()
            .ok_or_else(|| HostError::new("Rootfs link has no parent"))?
            .to_path_buf()
    };
    for part in target.components() {
        match part {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(value) => result.push(value),
            Component::ParentDir if result.pop() => {}
            _ => return Err(HostError::new("Rootfs link escapes its instance")),
        }
    }
    Ok(result)
}

/// Extracts regular data first and links last so archive links cannot redirect writes outside staging.
fn extract(reader: impl Read, root: &Path, control: &HttpDownloadControl) -> HostResult<()> {
    let mut archive = tar::Archive::new(reader);
    let mut links = Vec::new();
    let mut modes = Vec::new();
    let mut expanded = 0u64;
    let mut prefix = None;
    for (index, entry) in archive.entries()?.enumerate() {
        checkCancelled(control)?;
        if index >= MAX_ROOTFS_ENTRIES {
            return Err(HostError::new("Rootfs archive has too many entries"));
        }
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let first = path.components().find(|part| *part != Component::CurDir);
        match (&prefix, first) {
            (None, Some(Component::Normal(value))) => prefix = Some(value.to_os_string()),
            (Some(expected), Some(Component::Normal(value))) if expected == value => {}
            _ => {
                return Err(HostError::new(
                    "Rootfs archive must have one distribution directory",
                ))
            }
        }
        let relative = memberPath(&path)?;
        if relative.as_os_str().is_empty() || relative.starts_with("dev") {
            continue;
        }
        let destination = root.join(&relative);
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            fs::create_dir_all(&destination)?;
            modes.push((destination, entry.header().mode()?));
        } else if kind.is_file() {
            expanded = expanded
                .checked_add(entry.size())
                .ok_or_else(|| HostError::new("Rootfs size overflow"))?;
            if expanded > MAX_ROOTFS_BYTES {
                return Err(HostError::new("Rootfs exceeds installation size limit"));
            }
            fs::create_dir_all(
                destination
                    .parent()
                    .ok_or_else(|| HostError::new("Invalid rootfs file"))?,
            )?;
            let mut output = File::options()
                .create_new(true)
                .write(true)
                .open(&destination)?;
            let mut buffer = [0; 64 * 1024];
            loop {
                checkCancelled(control)?;
                let count = entry.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                output.write_all(&buffer[..count])?;
            }
            modes.push((destination, entry.header().mode()?));
        } else if kind.is_symlink() || kind.is_hard_link() {
            let target = entry
                .link_name()?
                .ok_or_else(|| HostError::new("Rootfs link has no target"))?
                .into_owned();
            let resolved = if kind.is_hard_link() {
                memberPath(&target)?
            } else {
                guestLink(&relative, &target)?
            };
            links.push((relative, resolved, kind.is_hard_link()));
        } else {
            return Err(HostError::new("Unsupported rootfs archive entry"));
        }
    }
    // Hardlinks must reference existing regular files before any symbolic links are created.
    for (member, target, _) in links.iter().filter(|(_, _, hard)| *hard) {
        let source = root.join(target);
        if !fs::symlink_metadata(&source)?.is_file() {
            return Err(HostError::new(
                "Rootfs hardlink target is not a regular file",
            ));
        }
        fs::create_dir_all(
            root.join(member)
                .parent()
                .ok_or_else(|| HostError::new("Invalid rootfs hardlink"))?,
        )?;
        fs::hard_link(source, root.join(member))?;
    }
    for (member, target, _) in links.iter().filter(|(_, _, hard)| !*hard) {
        let parent = member
            .parent()
            .ok_or_else(|| HostError::new("Invalid rootfs symlink"))?;
        fs::create_dir_all(root.join(parent))?;
        // Every symlink parent must still be an ordinary directory, including after earlier links.
        let parentCanonical = root.join(parent).canonicalize()?;
        if !parentCanonical.starts_with(root.canonicalize()?) {
            return Err(HostError::new("Rootfs symlink parent escapes staging"));
        }
        let mut relativeTarget = PathBuf::new();
        for _ in parent.components() {
            relativeTarget.push("..");
        }
        relativeTarget.push(target);
        createSymlink(&relativeTarget, &root.join(member))?;
    }
    for (path, mode) in modes.into_iter().rev() {
        setMode(&path, mode & 0o777)?;
    }
    Ok(())
}

/// Checks that Bash is a real little-endian ELF64 AArch64 executable after guest links resolve.
fn validateArm64(root: &Path) -> HostResult<()> {
    let executable = root.join("bin/bash").canonicalize()?;
    if !executable.starts_with(root.canonicalize()?) {
        return Err(HostError::new("Rootfs Bash escapes its instance"));
    }
    let mut header = [0u8; 20];
    File::open(executable)?.read_exact(&mut header)?;
    if &header[..6] != b"\x7fELF\x02\x01" || u16::from_le_bytes([header[18], header[19]]) != 183 {
        return Err(HostError::new(
            "Terminal rootfs must contain an ARM64 Linux Bash executable",
        ));
    }
    if !root.join("etc").is_dir() {
        return Err(HostError::new("Rootfs has no /etc directory"));
    }
    Ok(())
}

/// Creates a guest link on Unix hosts that support rootfs installation.
#[cfg(unix)]
fn createSymlink(target: &Path, path: &Path) -> HostResult<()> {
    std::os::unix::fs::symlink(target, path).map_err(Into::into)
}

/// Rejects rootfs symlink installation on hosts without Unix filesystem semantics.
#[cfg(not(unix))]
fn createSymlink(_target: &Path, _path: &Path) -> HostResult<()> {
    Err(HostError::new("Rootfs symlinks require a Unix host"))
}

/// Restores executable and directory permissions while removing special mode bits.
#[cfg(unix)]
fn setMode(path: &Path, mode: u32) -> HostResult<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(Into::into)
}

/// Leaves platform permissions intact when checking installer logic on a non-Unix host.
#[cfg(not(unix))]
fn setMode(_path: &Path, _mode: u32) -> HostResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a small real tar filesystem with a caller-selected ELF machine identifier.
    fn archive(machine: u16) -> Vec<u8> {
        let mut archive = tar::Builder::new(Vec::new());
        let mut elf = vec![0; 64];
        elf[..6].copy_from_slice(b"\x7fELF\x02\x01");
        elf[18..20].copy_from_slice(&machine.to_le_bytes());
        for (name, bytes) in [
            ("distro/bin/bash", elf.as_slice()),
            ("distro/etc/os-release", b"ID=test"),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            archive.append_data(&mut header, name, bytes).unwrap();
        }
        archive.into_inner().unwrap()
    }

    /// Accepts a complete ARM64 filesystem and rejects an x86-64 executable before publication.
    #[test]
    fn validates_real_arm64_shell_after_extraction() {
        for (machine, accepted) in [(183, true), (62, false)] {
            let directory = tempfile::tempdir().unwrap();
            extract(
                archive(machine).as_slice(),
                directory.path(),
                &HttpDownloadControl::new(),
            )
            .unwrap();
            assert_eq!(validateArm64(directory.path()).is_ok(), accepted);
        }
    }

    /// Rejects unsafe member names and keeps both absolute and relative guest links inside the guest.
    #[test]
    fn confines_archive_paths_and_guest_links() {
        for path in [
            "/etc/passwd",
            "distro/../../outside",
            "distro/bin/../../../outside",
        ] {
            assert!(memberPath(Path::new(path)).is_err(), "{path}");
        }
        assert_eq!(
            memberPath(Path::new("distro/usr/bin/bash")).unwrap(),
            Path::new("usr/bin/bash")
        );
        assert_eq!(
            guestLink(Path::new("bin/sh"), Path::new("/usr/bin/bash")).unwrap(),
            Path::new("usr/bin/bash")
        );
        assert_eq!(
            guestLink(Path::new("bin/sh"), Path::new("../usr/bin/bash")).unwrap(),
            Path::new("usr/bin/bash")
        );
        assert!(guestLink(Path::new("bin/sh"), Path::new("../../outside")).is_err());
    }

    /// Checks whole-file digests and cancellation before any archive content can be installed.
    #[test]
    fn rejects_checksum_mismatch_and_cancelled_extraction() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("archive");
        fs::write(&path, b"archive bytes").unwrap();
        let control = HttpDownloadControl::new();
        assert!(verifyArchive(&path, &"0".repeat(64), &control).is_err());
        let digest = format!("{:x}", Sha256::digest(b"archive bytes"));
        verifyArchive(&path, &digest, &control).unwrap();
        control.cancel();
        assert!(verifyArchive(&path, &digest, &control).is_err());
        let extracted = directory.path().join("rootfs");
        fs::create_dir(&extracted).unwrap();
        assert!(extract(archive(183).as_slice(), &extracted, &control).is_err());
        assert_eq!(fs::read_dir(&extracted).unwrap().count(), 0);
    }

    /// Cancels before network activity and removes the private staging directory completely.
    #[test]
    fn cancellation_removes_staging_without_publishing_an_instance() {
        let directory = tempfile::tempdir().unwrap();
        let control = HttpDownloadControl::new();
        control.cancel();
        let result = install(
            directory.path(),
            "ubuntu-24_04",
            "instance_a",
            "Ubuntu A",
            "proot",
            "bash",
            control,
            std::sync::Arc::new(|_| panic!("cancelled install must not download")),
        );
        assert!(result.unwrap_err().message.contains("cancelled"));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    /// Confirms absolute guest symlinks are rewritten within the installed instance on Unix.
    #[cfg(unix)]
    #[test]
    fn preserves_guest_symlinks_without_host_absolute_targets() {
        let directory = tempfile::tempdir().unwrap();
        let mut builder = tar::Builder::new(Vec::new());
        let mut elf = vec![0; 64];
        elf[..6].copy_from_slice(b"\x7fELF\x02\x01");
        elf[18..20].copy_from_slice(&183u16.to_le_bytes());
        let mut header = tar::Header::new_gnu();
        header.set_size(64);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, "distro/usr/bin/bash", elf.as_slice())
            .unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_mode(0o777);
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_cksum();
        builder
            .append_link(&mut header, "distro/bin", "/usr/bin")
            .unwrap();
        let bytes = builder.into_inner().unwrap();
        extract(
            bytes.as_slice(),
            directory.path(),
            &HttpDownloadControl::new(),
        )
        .unwrap();
        assert_eq!(
            fs::read_link(directory.path().join("bin")).unwrap(),
            Path::new("usr/bin")
        );
        assert!(directory
            .path()
            .join("bin/bash")
            .canonicalize()
            .unwrap()
            .starts_with(directory.path()));
    }
}
