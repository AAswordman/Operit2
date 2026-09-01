use crate::output::CoreCommandOutput;
use operit_util::GithubReleaseUtil::{FullUpdateStatus, FullUpdateTarget, GithubReleaseUtil};

/// Runs full-update commands.
pub fn run_update_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    if args.is_empty() {
        print_update_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "run" => run_update_run(args, output),
        "check" => run_update_check(args, output),
        "target" => run_update_target(args, output),
        _ => {
            print_update_usage(output);
            Ok(())
        }
    }
}

/// Checks for an update and downloads the matching package when available.
fn run_update_run(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    let usage = "usage: operit2 update run <current-version> <app|cli> <windows|linux|macos|android> <arch>";
    if args.len() != 5 {
        return Err(usage.to_string());
    }
    let currentVersion = args.get(1).ok_or_else(|| usage.to_string())?;
    let target = parseTarget(args.get(2), args.get(3), args.get(4), usage)?;
    let packageName = target.assetName()?;
    let channel = GithubReleaseUtil::fullUpdateChannelForVersion(currentVersion)?;
    let channelText = channel.to_string();
    match GithubReleaseUtil::checkForFullUpdateBlocking(currentVersion, target)? {
        FullUpdateStatus::Available(info) => {
            let workDir = std::env::temp_dir().join("operit2").join("full_update");
            let packagePath = GithubReleaseUtil::downloadAndPrepareFullUpdateBlocking(
                &info.downloadUrl,
                &info.assetName,
                &workDir,
                |_| {},
            )?;
            let packagePathText = packagePath.display().to_string();
            output.push_stdout_line("Update package downloaded.");
            output.push_stdout_line(format!("Current version: {currentVersion}"));
            output.push_stdout_line(format!("Channel: {channelText}"));
            output.push_stdout_line(format!("Latest version: {}", info.version));
            output.push_stdout_line(format!("Package: {}", info.assetName));
            output.push_stdout_line(format!("Package path: {packagePathText}"));
            output.push_stdout_line(format!("Release page: {}", info.releasePageUrl));
            output.setJsonStdout(serde_json::json!({
                "status": "downloaded",
                "currentVersion": currentVersion,
                "channel": channelText,
                "latestVersion": info.version,
                "package": info.assetName,
                "packagePath": packagePathText,
                "releasePageUrl": info.releasePageUrl,
            }));
        }
        FullUpdateStatus::UpToDate => {
            output.push_stdout_line("Already up to date.");
            output.push_stdout_line(format!("Current version: {currentVersion}"));
            output.push_stdout_line(format!("Channel: {channelText}"));
            output.push_stdout_line(format!("Package: {packageName}"));
            output.setJsonStdout(serde_json::json!({
                "status": "up-to-date",
                "currentVersion": currentVersion,
                "channel": channelText,
                "package": packageName,
            }));
        }
    }
    Ok(())
}

/// Checks whether a full update is available.
fn run_update_check(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    let usage = "usage: operit2 update check <current-version> <app|cli> <windows|linux|macos|android> <arch>";
    if args.len() != 5 {
        return Err(usage.to_string());
    }
    let currentVersion = args.get(1).ok_or_else(|| usage.to_string())?;
    let target = parseTarget(args.get(2), args.get(3), args.get(4), usage)?;
    let packageName = target.assetName()?;
    let channel = GithubReleaseUtil::fullUpdateChannelForVersion(currentVersion)?;
    let channelText = channel.to_string();
    match GithubReleaseUtil::checkForFullUpdateBlocking(currentVersion, target)? {
        FullUpdateStatus::Available(info) => {
            output.push_stdout_line("Update available.");
            output.push_stdout_line(format!("Current version: {currentVersion}"));
            output.push_stdout_line(format!("Channel: {channelText}"));
            output.push_stdout_line(format!("Latest version: {}", info.version));
            output.push_stdout_line(format!("Package: {}", info.assetName));
            output.push_stdout_line(format!("Download URL: {}", info.downloadUrl));
            output.push_stdout_line(format!("Release page: {}", info.releasePageUrl));
            output.setJsonStdout(serde_json::json!({
                "status": "available",
                "currentVersion": currentVersion,
                "channel": channelText,
                "latestVersion": info.version,
                "package": info.assetName,
                "downloadUrl": info.downloadUrl,
                "releasePageUrl": info.releasePageUrl,
            }));
        }
        FullUpdateStatus::UpToDate => {
            output.push_stdout_line("Already up to date.");
            output.push_stdout_line(format!("Current version: {currentVersion}"));
            output.push_stdout_line(format!("Channel: {channelText}"));
            output.push_stdout_line(format!("Package: {packageName}"));
            output.setJsonStdout(serde_json::json!({
                "status": "up-to-date",
                "currentVersion": currentVersion,
                "channel": channelText,
                "package": packageName,
            }));
        }
    }
    Ok(())
}

/// Prints the package target resolved from explicit platform arguments.
fn run_update_target(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    let usage = "usage: operit2 update target <app|cli> <windows|linux|macos|android> <arch>";
    if args.len() != 4 {
        return Err(usage.to_string());
    }
    let target = parseTarget(args.get(1), args.get(2), args.get(3), usage)?;
    let packageName = target.assetName()?;
    output.push_stdout_line(format!("Product: {}", target.product));
    output.push_stdout_line(format!("Platform: {}", target.platform));
    output.push_stdout_line(format!("Arch: {}", target.arch));
    output.push_stdout_line(format!("Package: {packageName}"));
    output.setJsonStdout(serde_json::json!({
        "product": target.product,
        "platform": target.platform,
        "arch": target.arch,
        "package": packageName,
    }));
    Ok(())
}

/// Parses and validates a full-update target.
fn parseTarget(
    product: Option<&String>,
    platform: Option<&String>,
    arch: Option<&String>,
    usage: &str,
) -> Result<FullUpdateTarget, String> {
    FullUpdateTarget::new(
        product.ok_or_else(|| usage.to_string())?,
        platform.ok_or_else(|| usage.to_string())?,
        arch.ok_or_else(|| usage.to_string())?,
    )
}

/// Prints update command usage.
fn print_update_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 update run <current-version> <app|cli> <windows|linux|macos|android> <arch>",
        "operit2 update check <current-version> <app|cli> <windows|linux|macos|android> <arch>",
        "operit2 update target <app|cli> <windows|linux|macos|android> <arch>",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
