use crate::commands::util::read_content_arg;
use crate::output::CoreCommandOutput;
use operit_host_api::{TerminalProfile, TerminalProfileSettings};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::services::RuntimeTerminalService::RuntimeTerminalService;
use serde_json::json;

/// Manages the same device-owned terminal registry and installation jobs as the GUI.
pub async fn run_terminal_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() || matches!(args, [flag] if flag == "help" || flag == "--help") {
        let usage = [
            "operit2 terminal info",
            "operit2 terminal support",
            "operit2 terminal profiles",
            "operit2 terminal save <settings-json|@vfs-file>",
            "operit2 terminal add <profile-json|@vfs-file>",
            "operit2 terminal update <profile-json|@vfs-file>",
            "operit2 terminal default <profile-id>",
            "operit2 terminal delete <profile-id>",
            "operit2 terminal distributions",
            "operit2 terminal install <distribution-id> <instance-name>",
            "operit2 terminal installs",
            "operit2 terminal cancel <install-id>",
            "operit2 terminal sessions",
            "operit2 terminal open <session-name> <vfs-directory> [profile-id]",
            "operit2 terminal input <session-id> <text|@vfs-file>",
            "operit2 terminal screen <session-id>",
            "operit2 terminal close <session-id>",
        ];
        for line in usage {
            output.push_stdout_line(line);
        }
        output.setJsonStdout(json!({"usage": usage}));
        return Ok(());
    }
    if application.hostManager.terminalHost.is_none() {
        return Err("This host does not register a terminal".into());
    }
    let service = RuntimeTerminalService::getInstance(&application.hostManager);
    let value = match args {
        [command] if command == "info" => json!(service.terminalInfo()?),
        [command] if command == "support" => json!(service.terminalProfileSupport()?),
        [command] if command == "profiles" => json!(service.terminalProfileSettings()?),
        [command, input] if command == "save" => {
            let settings: TerminalProfileSettings =
                serde_json::from_str(&read_content_arg(&application.hostManager, input)?)
                    .map_err(|error| error.to_string())?;
            json!(service.saveTerminalProfileSettings(settings)?)
        }
        [command, input] if command == "add" || command == "update" => {
            let profile: TerminalProfile =
                serde_json::from_str(&read_content_arg(&application.hostManager, input)?)
                    .map_err(|error| error.to_string())?;
            let mut settings = service.terminalProfileSettings()?;
            edit_profile(&mut settings, command, profile)?;
            json!(service.saveTerminalProfileSettings(settings)?)
        }
        [command, id] if command == "default" || command == "delete" => {
            let mut settings = service.terminalProfileSettings()?;
            select_or_delete(&mut settings, command, id)?;
            json!(service.saveTerminalProfileSettings(settings)?)
        }
        [command] if command == "distributions" => json!(service.terminalDistributions()?),
        [command, distribution, name] if command == "install" => {
            let started =
                service.startTerminalDistributionInstall(distribution.clone(), name.clone())?;
            loop {
                let job = service
                    .terminalDistributionInstalls()?
                    .into_iter()
                    .find(|job| job.installId == started.installId)
                    .ok_or("Terminal installation disappeared")?;
                match job.state.as_str() {
                    "completed" => break json!(job),
                    "failed" | "cancelled" => {
                        return Err(match job.error {
                            Some(error) => format!("Terminal installation {}: {error}", job.state),
                            None => format!("Terminal installation {}", job.state),
                        })
                    }
                    _ => tokio::time::sleep(std::time::Duration::from_millis(200)).await,
                }
            }
        }
        [command] if command == "installs" => json!(service.terminalDistributionInstalls()?),
        [command, id] if command == "cancel" => {
            service.cancelTerminalDistributionInstall(id.clone())?;
            json!({"installId": id, "cancelRequested": true})
        }
        [command] if command == "sessions" => json!(service.listTerminalSessions()?),
        [command, name, directory, rest @ ..] if command == "open" && rest.len() <= 1 => {
            let settings = service.terminalProfileSettings()?;
            let id = rest.first().unwrap_or(&settings.defaultProfileId);
            let profile = settings
                .profiles
                .iter()
                .find(|profile| profile.id == *id)
                .ok_or_else(|| format!("Terminal profile does not exist: {id}"))?;
            let session_id = service
                .startTerminalPty(
                    name.clone(),
                    format!("profile:{}", profile.id),
                    profile.terminalType.clone(),
                    directory.clone(),
                    24,
                    80,
                )
                .await?;
            json!({"sessionId": session_id, "profileId": profile.id})
        }
        [command, id, input] if command == "input" => {
            let input = read_content_arg(&application.hostManager, input)?;
            json!({"sessionId": id, "written": service.inputTerminalSession(id.clone(), input)?})
        }
        [command, id] if command == "screen" => {
            json!(service.getTerminalSessionScreen(id.clone())?)
        }
        [command, id] if command == "close" => {
            service.closeTerminalPty(id.clone())?;
            json!({"sessionId": id, "closed": true})
        }
        _ => return Err("Invalid terminal arguments; use 'operit2 terminal help'".into()),
    };
    output
        .push_stdout_line(serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?);
    output.setJsonStdout(value);
    Ok(())
}

/// Distinguishes creation from replacement without implicitly changing the default or revision.
fn edit_profile(
    settings: &mut TerminalProfileSettings,
    command: &str,
    profile: TerminalProfile,
) -> Result<(), String> {
    let index = settings
        .profiles
        .iter()
        .position(|entry| entry.id == profile.id);
    match (command, index) {
        ("add", None) => settings.profiles.push(profile),
        ("update", Some(index)) => settings.profiles[index] = profile,
        ("add", Some(_)) => return Err(format!("Terminal profile already exists: {}", profile.id)),
        _ => return Err(format!("Terminal profile does not exist: {}", profile.id)),
    }
    Ok(())
}

/// Rejects unknown identities and deletion of the active default before saving.
fn select_or_delete(
    settings: &mut TerminalProfileSettings,
    command: &str,
    id: &str,
) -> Result<(), String> {
    let index = settings
        .profiles
        .iter()
        .position(|entry| entry.id == id)
        .ok_or_else(|| format!("Terminal profile does not exist: {id}"))?;
    if command == "default" {
        settings.defaultProfileId = id.into();
    } else {
        if settings.defaultProfileId == id {
            return Err("Select another default before deleting this terminal profile".into());
        }
        settings.profiles.remove(index);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Verifies independent instances retain the explicit default and stale-write revision.
    #[test]
    fn mutations_preserve_identity_default_and_revision() {
        let profile = TerminalProfile {
            id: "ps5".into(),
            name: "PowerShell 5".into(),
            terminal: "native".into(),
            terminalType: "powershell".into(),
            options: BTreeMap::new(),
        };
        let mut settings = TerminalProfileSettings {
            revision: 17,
            defaultProfileId: profile.id.clone(),
            profiles: vec![profile.clone()],
        };
        let mut second = profile.clone();
        second.id = "ps7".into();
        edit_profile(&mut settings, "add", second.clone()).unwrap();
        assert_eq!(settings.defaultProfileId, "ps5");
        assert!(edit_profile(&mut settings, "add", second.clone()).is_err());
        assert!(select_or_delete(&mut settings, "delete", "ps5").is_err());
        assert!(select_or_delete(&mut settings, "default", "missing").is_err());
        select_or_delete(&mut settings, "default", "ps7").unwrap();
        select_or_delete(&mut settings, "delete", "ps5").unwrap();
        assert_eq!(settings.profiles, vec![second]);
        assert_eq!(settings.revision, 17);
    }
}
