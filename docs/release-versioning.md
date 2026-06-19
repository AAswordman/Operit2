# Operit2 Release Versioning

This document is the source of truth for Operit2 app and CLI release versions.

## Version Format

Operit2 uses SemVer-compatible versions:

```text
stable:  2.0.0
preview: 2.0.0-preview.1
rc:      2.0.0-rc.1
dev:     2.0.0-dev.20260619
```

Git tags use the same version with a leading `v`:

```text
v2.0.0-preview.1
v2.0.0-rc.1
v2.0.0
```

Build metadata is allowed and does not change update ordering:

```text
2.0.0+20260619.shaabcdef
```

Do not use build metadata as a release counter. Use prerelease identifiers for public preview builds.

## Ordering

Version ordering follows SemVer precedence:

```text
2.0.0-dev.20260619 < 2.0.0-preview.1 < 2.0.0-preview.2 < 2.0.0-rc.1 < 2.0.0
```

Stable versions are newer than prerelease versions with the same major, minor, and patch numbers.

## Channels

Updater channel is derived from the current installed version:

```text
current stable version      -> stable channel
current prerelease version  -> preview channel
```

Stable channel accepts only stable GitHub releases.

Preview channel accepts prerelease and stable GitHub releases.

## GitHub Releases

Release tags and GitHub release flags must agree:

```text
v2.0.0-preview.1  -> GitHub prerelease = true
v2.0.0-rc.1       -> GitHub prerelease = true
v2.0.0            -> GitHub prerelease = false
```

Draft releases are ignored by the updater.

The release script derives the GitHub prerelease flag from the tag version. Prerelease tags publish as GitHub prereleases. Stable tags publish as stable GitHub releases.

The first public Operit2 preview should use:

```text
v2.0.0-preview.1
```

The first stable Operit2 release should use:

```text
v2.0.0
```

## Package Assets

Release asset names are fixed by product, platform, and architecture. They do not include the version number.

```text
operit2-cli-windows-x86_64.zip
operit2-cli-linux-x86_64.tar.gz
operit2-cli-macos-aarch64.tar.gz
operit2-app-android-arm64-v8a.apk
operit2-app-macos-x86_64.tar.gz
```

The version lives in the Git tag and package metadata.

A GitHub Release may contain only the selected scope's assets. The updater selects releases by exact target asset name, so an app client does not treat a CLI-only release as an app update.

## Version Sources

The Operit2 release version is the full SemVer value used by the updater:

```text
apps/cli/Cargo.toml                         package.version
core/crates/operit-runtime/Cargo.toml       package.version
```

These two values must be identical. They must not include build metadata. The CLI uses its Cargo package version as `cliVersion`. The app exposes `coreVersion` from `operit-runtime`.

Flutter platform metadata is separate from the Operit2 release version:

```text
apps/flutter/app/pubspec.yaml               version: major.minor.patch+buildNumber
```

For `2.0.0-preview.1`, the Flutter platform version is:

```text
2.0.0+1
```

The release script uses the full Operit2 SemVer for Git tags and updater metadata. The Flutter `major.minor.patch` must match the Operit2 release version. Flutter `buildNumber` is only platform package metadata for Android, Windows, Linux, and macOS builds.

For App and full publish runs, the release script builds with the current Flutter `buildNumber` and then increments `apps/flutter/app/pubspec.yaml` by 1 for the next App package. `--build-only` and CLI-only runs do not change the Flutter `buildNumber`.

## Release Script

The release script is:

```text
tools/release/release.py
```

GitHub publish credentials live in:

```text
tools/release/secrets/github.env
```

That directory is ignored by git.

Default publish command on Windows:

```powershell
.\.venv\Scripts\python.exe tools\release\release.py
```

Release scope:

```powershell
# CLI/TUI only
.\.venv\Scripts\python.exe tools\release\release.py --scope cli

# App only
.\.venv\Scripts\python.exe tools\release\release.py --scope app

# App and CLI/TUI
.\.venv\Scripts\python.exe tools\release\release.py --scope full
```

The script reads GitHub credentials from:

```text
d:\Code\prog\assistance\tools\github\.env
```

Required keys:

```text
GITHUB_TOKEN
GITHUB_API_URL
```

The default release repository is:

```text
AAswordman/Operit2
```

Use `--repo owner/name` to publish another repository.

It must enforce these rules:

- Read Cargo package versions with TOML parsing.
- Read Flutter platform build metadata from `pubspec.yaml`.
- Reject mismatched CLI and runtime release versions.
- Reject Cargo and runtime release versions that include build metadata.
- Reject Flutter platform versions whose `major.minor.patch` differs from the Cargo release version.
- Reject a `--tag` value that differs from the Cargo release version.
- Derive the GitHub prerelease flag from the release version.
- Build and upload only the selected `--scope` assets.
- Increment Flutter `buildNumber` by 1 after successful App/full publish builds.
- Publish releases through the GitHub REST API with `GITHUB_TOKEN`.
- Check an existing GitHub release's prerelease flag before uploading assets.

For the first public preview:

```text
apps/cli/Cargo.toml                    version = "2.0.0-preview.1"
core/crates/operit-runtime/Cargo.toml  version = "2.0.0-preview.1"
apps/flutter/app/pubspec.yaml          version: 2.0.0+1
GitHub tag                             v2.0.0-preview.1
```

## Runtime Implementation

The updater implementation is in:

```text
core/crates/operit-runtime/src/util/GithubReleaseUtil.rs
```

The implementation must keep these rules:

- Parse versions strictly as `major.minor.patch[-prerelease][+build]`.
- Ignore build metadata during version comparison.
- Select releases by updater channel.
- Match assets by fixed asset name.
- Reject mismatched GitHub prerelease flags.
