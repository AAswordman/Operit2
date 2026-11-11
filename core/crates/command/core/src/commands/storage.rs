use operit_vfs::{VfsPath, VisualFileSystem::VisualFileSystem};
use std::path::{Path, PathBuf};

use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_util::RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH;
/// Describes virtual storage roots exposed by the command interface.
struct StoragePaths {
    runtime_root: PathBuf,
    workspace_root: PathBuf,
}

/// Runs storage path inspection and migration commands.
pub fn run_storage_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match args.get(0).map(String::as_str) {
        Some("paths") if args.len() == 1 => print_storage_paths(application, output),
        Some("mounts") => run_mount_command(application, &args[1..], output),
        Some("migrate") => migrate_storage_roots(application, &args[1..], output),
        _ => {
            print_storage_usage(output);
            Ok(())
        }
    }
}

/// Registers native folders or platform resource backends without adding VFS special cases.
fn run_mount_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let storage = application
        .hostManager
        .runtimeStorageHost
        .clone()
        .ok_or("RuntimeStorageHost is not registered for mount commands")?;
    let registry = operit_vfs::MountRegistry::MountRegistry::withStorage(storage);
    match args.first().map(String::as_str) {
        Some("list") if args.len() == 1 => {
            output.setJsonStdout(serde_json::json!({"mounts": registry.list()?}));
        }
        Some("add") if args.len() == 5 => {
            let mount = registry.register(&args[1], &args[2], &args[3], &args[4])?;
            output.push_stdout_line(mount.vfsPath());
            output.setJsonStdout(serde_json::json!({"path": mount.vfsPath(), "mount": mount}));
        }
        Some("remove") if args.len() == 2 => {
            registry.remove(&args[1])?;
            output.setJsonStdout(serde_json::json!({"removed": args[1]}));
        }
        _ => return Err("usage: storage mounts list | add <namespace> <backend> <root> <name> | remove <vfs-path>".into()),
    }
    Ok(())
}

/// Prints the active runtime and workspace root directories.
fn print_storage_paths(
    application: &OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let config = storage_root_config(application)?;
    let runtimeRoot = config.runtime_root.display().to_string();
    let workspaceRoot = config.workspace_root.display().to_string();
    let locations = application.hostManager.vfs()?.storageLocations()?;
    output.push_stdout_line(format!("Runtime root: {runtimeRoot}"));
    output.push_stdout_line(format!("Workspace root: {workspaceRoot}"));
    for location in &locations {
        output.push_stdout_line(format!(
            "{} = {} ({})",
            location.vfsPath, location.displayPath, location.locationVfsPath
        ));
    }
    output.setJsonStdout(serde_json::json!({
        "runtimeRoot": runtimeRoot,
        "workspaceRoot": workspaceRoot,
        "locations": locations,
    }));
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
    let filesystem = application.hostManager.vfs()?;

    let runtimeOperation = migrate_named_root(
        &filesystem,
        RUNTIME_ROOT_DIR_PATH,
        &current.runtime_root,
        &plan.runtime_root,
        output,
    )?;
    let workspaceOperation = migrate_named_root(
        &filesystem,
        "workspace",
        &current.workspace_root,
        &plan.workspace_root,
        output,
    )?;

    let runtimeRoot = plan.runtime_root.display().to_string();
    let workspaceRoot = plan.workspace_root.display().to_string();
    output.push_stdout_line(format!("Runtime root: {runtimeRoot}"));
    output.push_stdout_line(format!("Workspace root: {workspaceRoot}"));
    output.push_stdout_line(format!("runtimeRoot={runtimeRoot}"));
    output.push_stdout_line(format!("workspaceRoot={workspaceRoot}"));
    output.push_stdout_line("storageConfig=updated");
    output.setJsonStdout(serde_json::json!({
        "runtimeRoot": runtimeRoot,
        "workspaceRoot": workspaceRoot,
        "storageConfig": "updated",
        "operations": [runtimeOperation, workspaceOperation],
    }));
    Ok(())
}

/// Returns the root configuration exposed by the active runtime storage host.
fn storage_root_config(application: &OperitApplication) -> Result<StoragePaths, String> {
    application.hostManager.vfs()?;
    Ok(StoragePaths {
        runtime_root: "/operitapp/data".into(),
        workspace_root: "/operitapp/workspaces".into(),
    })
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
    Ok(PathBuf::from(VfsPath::storageLocationPath(value)?))
}

/// Migrates one named storage root into a target directory.
fn migrate_named_root(
    filesystem: &VisualFileSystem,
    name: &str,
    source: &Path,
    target: &Path,
    output: &mut CoreCommandOutput,
) -> Result<serde_json::Value, String> {
    let sourcePath = source.display().to_string();
    let targetPath = target.display().to_string();
    if filesystem.isWithin(&sourcePath, &targetPath)?
        && filesystem.isWithin(&targetPath, &sourcePath)?
    {
        output.push_stdout_line(format!("{name}: already using {targetPath}"));
        return Ok(serde_json::json!({
            "name": name,
            "status": "unchanged",
            "source": sourcePath,
            "target": targetPath,
        }));
    }
    if !filesystem.fileExists(&sourcePath)?.exists {
        filesystem.makeDirectory(&targetPath, true)?;
        output.push_stdout_line(format!("{name}: created {targetPath}"));
        return Ok(serde_json::json!({
            "name": name,
            "status": "created",
            "source": sourcePath,
            "target": targetPath,
        }));
    }
    if !filesystem.fileExists(&sourcePath)?.isDirectory {
        return Err(format!(
            "{name} source root must be a directory: {}",
            source.display()
        ));
    }
    if filesystem.isWithin(&sourcePath, &targetPath)? {
        return Err("storage migration target cannot be inside its source".into());
    }
    filesystem.makeDirectory(&targetPath, true)?;
    filesystem.copyFile(&sourcePath, &targetPath, true)?;
    output.push_stdout_line(format!("{name}: migrated {sourcePath} -> {targetPath}"));
    Ok(serde_json::json!({
        "name": name,
        "status": "migrated",
        "source": sourcePath,
        "target": targetPath,
    }))
}

/// Prints storage command usage.
fn print_storage_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 storage paths",
        "operit2 storage mounts list",
        "operit2 storage mounts add <namespace> <backend> <root> <name>",
        "operit2 storage mounts remove <vfs-path>",
        "operit2 storage migrate --runtime <path> --workspace <path>",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}

#[cfg(test)]
mod tests {
    use super::parse_target_root;

    /// Accepts a storage picker location at the management boundary and enters the VFS.
    #[test]
    fn selected_storage_locations_enter_the_mounted_namespace() {
        let selected = "D:\\用户目录\\Operit data".to_string();
        let target = parse_target_root(Some(&selected), "--runtime").unwrap();
        assert_eq!(
            target.to_str().unwrap(),
            "/virtual/windows/d/用户目录/Operit data"
        );
        let mounted = "/virtual/windows/d/用户目录/Operit data".to_string();
        assert_eq!(
            parse_target_root(Some(&mounted), "--runtime").unwrap(),
            target
        );
        let alias = "/operitapp/data".to_string();
        assert_eq!(
            parse_target_root(Some(&alias), "--runtime")
                .unwrap()
                .to_str(),
            Some(alias.as_str())
        );
        assert!(parse_target_root(Some(&"relative".to_string()), "--runtime").is_err());
    }
}
