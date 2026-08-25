use crate::output::CoreCommandOutput;
use operit_util::GithubReleaseUtil::{FullUpdateStatus, FullUpdateTarget, GithubReleaseUtil};

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

fn run_update_run(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    let usage = "usage: operit2 update run <current-version> <app|cli> <windows|linux|macos|android> <arch>";
    if args.len() != 5 {
        return Err(usage.to_string());
    }
    let currentVersion = args.get(1).ok_or_else(|| usage.to_string())?;
    let target = parseTarget(args.get(2), args.get(3), args.get(4), usage)?;
    let packageName = target.assetName()?;
    let channel = GithubReleaseUtil::fullUpdateChannelForVersion(currentVersion)?;
    match GithubReleaseUtil::checkForFullUpdateBlocking(currentVersion, target)? {
        FullUpdateStatus::Available(info) => {
            let workDir = std::env::temp_dir().join("operit2").join("full_update");
            let packagePath = GithubReleaseUtil::downloadAndPrepareFullUpdateBlocking(
                &info.downloadUrl,
                &info.assetName,
                &workDir,
                |_| {},
            )?;
            output.push_stdout_line("status=downloaded");
            output.push_stdout_line(format!("currentVersion={currentVersion}"));
            output.push_stdout_line(format!("channel={channel}"));
            output.push_stdout_line(format!("latestVersion={}", info.version));
            output.push_stdout_line(format!("package={}", info.assetName));
            output.push_stdout_line(format!("packagePath={}", packagePath.display()));
            output.push_stdout_line(format!("releasePageUrl={}", info.releasePageUrl));
        }
        FullUpdateStatus::UpToDate => {
            output.push_stdout_line("status=up-to-date");
            output.push_stdout_line(format!("currentVersion={currentVersion}"));
            output.push_stdout_line(format!("channel={channel}"));
            output.push_stdout_line(format!("package={packageName}"));
        }
    }
    Ok(())
}

fn run_update_check(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    let usage = "usage: operit2 update check <current-version> <app|cli> <windows|linux|macos|android> <arch>";
    if args.len() != 5 {
        return Err(usage.to_string());
    }
    let currentVersion = args.get(1).ok_or_else(|| usage.to_string())?;
    let target = parseTarget(args.get(2), args.get(3), args.get(4), usage)?;
    let packageName = target.assetName()?;
    let channel = GithubReleaseUtil::fullUpdateChannelForVersion(currentVersion)?;
    match GithubReleaseUtil::checkForFullUpdateBlocking(currentVersion, target)? {
        FullUpdateStatus::Available(info) => {
            output.push_stdout_line("status=available");
            output.push_stdout_line(format!("currentVersion={currentVersion}"));
            output.push_stdout_line(format!("channel={channel}"));
            output.push_stdout_line(format!("latestVersion={}", info.version));
            output.push_stdout_line(format!("package={}", info.assetName));
            output.push_stdout_line(format!("downloadUrl={}", info.downloadUrl));
            output.push_stdout_line(format!("releasePageUrl={}", info.releasePageUrl));
        }
        FullUpdateStatus::UpToDate => {
            output.push_stdout_line("status=up-to-date");
            output.push_stdout_line(format!("currentVersion={currentVersion}"));
            output.push_stdout_line(format!("channel={channel}"));
            output.push_stdout_line(format!("package={packageName}"));
        }
    }
    Ok(())
}

fn run_update_target(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    let usage = "usage: operit2 update target <app|cli> <windows|linux|macos|android> <arch>";
    if args.len() != 4 {
        return Err(usage.to_string());
    }
    let target = parseTarget(args.get(1), args.get(2), args.get(3), usage)?;
    let packageName = target.assetName()?;
    output.push_stdout_line(format!("product={}", target.product));
    output.push_stdout_line(format!("platform={}", target.platform));
    output.push_stdout_line(format!("arch={}", target.arch));
    output.push_stdout_line(format!("package={packageName}"));
    Ok(())
}

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

fn print_update_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line(
        "operit2 update run <current-version> <app|cli> <windows|linux|macos|android> <arch>",
    );
    output.push_stdout_line(
        "operit2 update check <current-version> <app|cli> <windows|linux|macos|android> <arch>",
    );
    output.push_stdout_line("operit2 update target <app|cli> <windows|linux|macos|android> <arch>");
}
