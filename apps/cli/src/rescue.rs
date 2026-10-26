//! CLI entry for the shared offline rescue engine; never starts Core or opens SQLite.
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use base64::{Engine, engine::general_purpose::STANDARD};
use operit_host_data_rescue::{NativeRescueSession, RescueRoots};
use serde_json::json;

/// Parses one explicit option value and rejects missing or repeated arguments.
fn option(args: &[String], key: &str) -> Result<Option<String>, String> {
    let positions: Vec<_> = args.iter().enumerate().filter(|(_, value)| value.as_str() == key).collect();
    match positions.as_slice() {
        [] => Ok(None),
        [(index, _)] => args.get(index + 1).filter(|value| !value.starts_with("--")).cloned().map(Some).ok_or_else(|| format!("Missing value for {key}")),
        _ => Err(format!("Repeated option: {key}")),
    }
}

/// Rejects options that are not part of the explicit rescue command contract.
fn validate_options(args: &[String], command: &str) -> Result<(), String> {
    let allowed = match command {
        "export" | "inspect" | "import" => ["--runtime-root", "--workspace-root", "--confirm"].as_slice(),
        "resume" => ["--runtime-root", "--workspace-root", "--confirm"].as_slice(),
        "describe" => ["--runtime-root", "--workspace-root"].as_slice(),
        _ => return Err("Unknown rescue command; run operit2 rescue --help".into()),
    };
    let takes_path = matches!(command, "export" | "inspect" | "import");
    let mut index = if takes_path { 2 } else { 1 };
    if takes_path && args.get(1).is_none_or(|value| value.starts_with('-')) { return Err("An explicit archive path is required before options".into()); }
    let mut seen = std::collections::BTreeSet::new();
    while index < args.len() {
        let argument = &args[index];
        if !allowed.contains(&argument.as_str()) { return Err(format!("Unexpected rescue argument: {argument}")); }
        if !seen.insert(argument) { return Err(format!("Repeated rescue option: {argument}")); }
        if argument == "--confirm" {
            if !matches!(command, "import" | "resume") { return Err("--confirm is only valid for import and resume".into()); }
            index += 1;
        } else {
            option(args, argument)?;
            index += 2;
        }
    }
    Ok(())
}

/// Removes an owned temporary archive and preserves both operation and cleanup diagnostics.
fn cleanup_archive<T>(session: &mut NativeRescueSession, id: &str, result: Result<T, String>) -> Result<T, String> {
    let cleanup = session.execute(json!({"operation": "discardArchive", "archiveId": id}));
    match (result, cleanup) {
        (Ok(value), Ok(_)) => Ok(value),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(error)) => Err(format!("Archive cleanup failed: {error}")),
        (Err(error), Err(cleanup)) => Err(format!("{error}; archive cleanup failed: {cleanup}")),
    }
}

/// Streams one selected archive into the shared bounded upload protocol.
fn upload(session: &mut NativeRescueSession, path: &str) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let length = file.metadata().map_err(|error| error.to_string())?.len();
    let response = session.execute(json!({"operation": "beginUpload", "byteLength": length}))?;
    let id = response["archiveId"].as_str().ok_or("Upload returned no archive identifier")?.to_string();
    let result = (|| {
        let mut buffer = [0u8; 65536];
        loop {
            let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
            if count == 0 { break; }
            session.execute(json!({"operation": "uploadChunk", "archiveId": id, "bytes": STANDARD.encode(&buffer[..count])}))?;
        }
        session.execute(json!({"operation": "sealUpload", "archiveId": id}))?;
        Ok(())
    })();
    if let Err(error) = result {
        return match session.execute(json!({"operation": "discardArchive", "archiveId": id})) {
            Ok(_) => Err(error),
            Err(cleanup) => Err(format!("{error}; archive cleanup failed: {cleanup}")),
        };
    }
    Ok(id)
}

/// Executes the independent rescue command before any normal CLI bootstrap is reached.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() || args[0] == "--help" || args[0] == "help" {
        println!("operit2 rescue describe\noperit2 rescue export <new.zip>\noperit2 rescue inspect <backup.zip>\noperit2 rescue import <backup.zip> --confirm\noperit2 rescue resume --confirm\nAll commands accept explicit --runtime-root <directory> --workspace-root <directory>. Close every Operit window and background service first.");
        return Ok(());
    }
    validate_options(args, &args[0])?;
    let command = args[0].as_str();
    if command == "describe" && option(args, "--runtime-root")?.is_none() && option(args, "--workspace-root")?.is_none() {
        let session = NativeRescueSession::open(crate::bootstrap::rescue_default_runtime(), None)?;
        let mut startup = session.describe_startup()?;
        let config = crate::bootstrap::rescue_config_root().join("storage.json");
        let bytes = match std::fs::read(&config) {
            Ok(bytes) => Some(STANDARD.encode(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        startup["cliConfigPath"] = json!(config);
        startup["cliConfigBase64"] = json!(bytes);
        println!("{}", serde_json::to_string_pretty(&startup).map_err(|error| error.to_string())?);
        return Ok(());
    }
    let roots = match (option(args, "--runtime-root")?, option(args, "--workspace-root")?) {
        (Some(runtime), Some(workspaces)) => RescueRoots { runtime_root: PathBuf::from(runtime), workspace_root: PathBuf::from(workspaces), cli_config_root: Some(crate::bootstrap::rescue_config_root()) },
        (None, None) if command == "resume" => {
            let default_runtime = crate::bootstrap::rescue_default_runtime();
            let probe = NativeRescueSession::open(default_runtime, None)?;
            let startup = probe.describe_startup()?;
            let selected = startup["recoveryRoots"].as_object().ok_or("No recorded recovery roots are available for the interrupted import")?;
            RescueRoots {
                runtime_root: PathBuf::from(selected["runtimeRoot"].as_str().ok_or("Pending import runtime root is invalid")?),
                workspace_root: PathBuf::from(selected["workspaceRoot"].as_str().ok_or("Pending import workspace root is invalid")?),
                cli_config_root: selected.get("cliConfigRoot").and_then(|value| value.as_str()).map(PathBuf::from),
            }
        }
        (None, None) => crate::bootstrap::rescue_roots()?,
        _ => return Err("Specify --runtime-root and --workspace-root together".into()),
    };
    let default_runtime = crate::bootstrap::rescue_default_runtime();
    let mut session = crate::bootstrap::rescue_session(default_runtime, &roots.runtime_root)?;
    session.execute(json!({"operation": "configure", "roots": roots}))?;
    let value = match command {
        "describe" => session.execute(json!({"operation": "describe"}))?,
        "export" => {
            let destination = args.get(1).filter(|path| !path.starts_with("--")).ok_or("Export destination is required")?;
            let archive = session.execute(json!({"operation": "export"}))?;
            let mut created = false;
            let copied = (|| {
                let reference = archive["fileReference"].as_str().ok_or("Export returned no file reference")?;
                let mut input = File::open(reference).map_err(|error| error.to_string())?;
                let mut output = OpenOptions::new().write(true).create_new(true).open(destination).map_err(|error| error.to_string())?;
                created = true;
                std::io::copy(&mut input, &mut output).map_err(|error| error.to_string())?;
                output.flush().map_err(|error| error.to_string())?;
                output.sync_all().map_err(|error| error.to_string())?;
                Ok(json!({"savedTo": destination, "byteLength": archive["byteLength"]}))
            })();
            let copied = match copied {
                Err(error) if created => match std::fs::remove_file(destination) {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(format!("{error}; incomplete export cleanup failed: {cleanup}")),
                },
                result => result,
            };
            cleanup_archive(&mut session, archive["archiveId"].as_str().ok_or("Export returned no archive identifier")?, copied)?
        }
        "inspect" | "import" => {
            let path = args.get(1).filter(|path| !path.starts_with("--")).ok_or("Backup archive path is required")?;
            if args[0] == "import" && !args.iter().any(|arg| arg == "--confirm") { return Err("Import replaces selected live data and requires --confirm".into()); }
            let id = upload(&mut session, path)?;
            let operation = if args[0] == "import" { "restore" } else { "inspect" };
            let value = session.execute(json!({"operation": operation, "archiveId": id}));
            cleanup_archive(&mut session, &id, value)?
        }
        "resume" => {
            if !args.iter().any(|arg| arg == "--confirm") { return Err("Resuming import publication requires --confirm".into()); }
            session.execute(json!({"operation": "resumeRestore"}))?
        }
        _ => return Err("Unknown rescue command; run operit2 rescue --help".into()),
    };
    println!("{}", serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?);
    Ok(())
}
