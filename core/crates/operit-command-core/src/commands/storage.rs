use std::fs;
use std::path::{Path, PathBuf};

use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_util::RuntimeStoreRoot::RuntimeStoreRootConfig;

/// Runs storage path inspection and migration commands.
pub fn run_storage_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match args.get(0).map(String::as_str) {
        Some("paths") if args.len() == 1 => print_storage_paths(application, output),
        Some("migrate") => migrate_storage_roots(application, &args[1..], output),
        _ => {
            print_storage_usage(output);
            Ok(())
        }
    }
}

/// Prints the active data, runtime, and workspace root directories.
fn print_storage_paths(
    application: &OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let config = storage_root_config(application)?;
    output.push_stdout_line(format!("dataRoot={}", config.data_root.display()));
    output.push_stdout_line(format!("runtimeRoot={}", config.runtime_root.display()));
    output.push_stdout_line(format!("workspaceRoot={}", config.workspace_root.display()));
    Ok(())
}

/// Migrates runtime and workspace directories to explicit target roots.
fn migrate_storage_roots(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let plan = parse_migrate_args(args)?;
    let current = storage_root_config(application)?;

    migrate_named_root("runtime", &current.runtime_root, &plan.runtime_root, output)?;
    migrate_named_root(
        "workspace",
        &current.workspace_root,
        &plan.workspace_root,
        output,
    )?;

    output.push_stdout_line(format!("dataRoot={}", current.data_root.display()));
    output.push_stdout_line(format!("runtimeRoot={}", plan.runtime_root.display()));
    output.push_stdout_line(format!("workspaceRoot={}", plan.workspace_root.display()));
    output.push_stdout_line("storageConfig=updated");
    Ok(())
}

/// Returns the root configuration exposed by the active runtime storage host.
fn storage_root_config(application: &OperitApplication) -> Result<RuntimeStoreRootConfig, String> {
    let host = application
        .hostManager
        .runtimeStorageHost
        .as_ref()
        .ok_or_else(|| "RuntimeStorageHost is not registered".to_string())?;
    let data_root = host
        .rootDir()
        .ok_or_else(|| "RuntimeStorageHost data root is not configured".to_string())?;
    let runtime_root = host
        .runtimeRootDir()
        .ok_or_else(|| "RuntimeStorageHost runtime root is not configured".to_string())?;
    let workspace_root = host
        .workspaceRootDir()
        .ok_or_else(|| "RuntimeStorageHost workspace root is not configured".to_string())?;
    Ok(RuntimeStoreRootConfig::new(
        data_root,
        runtime_root,
        workspace_root,
    ))
}

#[derive(Debug)]
struct StorageMigrationPlan {
    runtime_root: PathBuf,
    workspace_root: PathBuf,
}

/// Parses storage migration target root arguments.
fn parse_migrate_args(args: &[String]) -> Result<StorageMigrationPlan, String> {
    let mut runtime_root = None;
    let mut workspace_root = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--runtime" => {
                index += 1;
                runtime_root = Some(parse_target_root(args.get(index), "--runtime")?);
            }
            "--workspace" => {
                index += 1;
                workspace_root = Some(parse_target_root(args.get(index), "--workspace")?);
            }
            value => return Err(format!("unknown storage migrate argument: {value}")),
        }
        index += 1;
    }
    Ok(StorageMigrationPlan {
        runtime_root: runtime_root.ok_or_else(|| {
            "usage: operit2 storage migrate --runtime <path> --workspace <path>".to_string()
        })?,
        workspace_root: workspace_root.ok_or_else(|| {
            "usage: operit2 storage migrate --runtime <path> --workspace <path>".to_string()
        })?,
    })
}

/// Parses and validates one target root argument.
fn parse_target_root(value: Option<&String>, flag: &str) -> Result<PathBuf, String> {
    let value = value
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing path after {flag}"))?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("{flag} path must be absolute: {value}"));
    }
    Ok(path)
}

/// Migrates one named storage root into a target directory.
fn migrate_named_root(
    name: &str,
    source: &Path,
    target: &Path,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if source == target {
        output.push_stdout_line(format!("{name}=unchanged\t{}", target.display()));
        return Ok(());
    }
    if !source.exists() {
        fs::create_dir_all(target).map_err(|error| error.to_string())?;
        output.push_stdout_line(format!("{name}=created\t{}", target.display()));
        return Ok(());
    }
    if source.is_file() {
        return Err(format!(
            "{name} source root must be a directory: {}",
            source.display()
        ));
    }
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    copy_directory_contents(source, target)?;
    output.push_stdout_line(format!(
        "{name}=migrated\t{}\t{}",
        source.display(),
        target.display()
    ));
    Ok(())
}

/// Copies every entry under one directory into another directory.
fn copy_directory_contents(source: &Path, target: &Path) -> Result<(), String> {
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        copy_storage_entry(&source_path, &target_path)?;
    }
    Ok(())
}

/// Copies one file-system entry into a target path.
fn copy_storage_entry(source: &Path, target: &Path) -> Result<(), String> {
    let metadata = fs::metadata(source).map_err(|error| error.to_string())?;
    if metadata.is_dir() {
        fs::create_dir_all(target).map_err(|error| error.to_string())?;
        return copy_directory_contents(source, target);
    }
    if metadata.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(source, target).map_err(|error| error.to_string())?;
        return Ok(());
    }
    Err(format!(
        "unsupported storage entry type: {}",
        source.display()
    ))
}

/// Prints storage command usage.
fn print_storage_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 storage paths");
    output.push_stdout_line("operit2 storage migrate --runtime <path> --workspace <path>");
}
