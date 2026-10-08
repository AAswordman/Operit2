//! Generic registered-command ownership; this catalog does not execute handlers.

use super::ToolPkgCoreCommandInfo;
use operit_plugin_sdk::toolpkg::ToolPkgParser::{
    ToolPkgContainerRuntime, ToolPkgCoreCommandRuntime,
};
use std::collections::BTreeSet;

/// Describes a real command declaration including its current activation state.
#[derive(Clone, Debug)]
pub struct ToolPkgCoreCommandRegistrationInfo {
    pub command: ToolPkgCoreCommandInfo,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
struct Registration {
    info: ToolPkgCoreCommandInfo,
    enabled: bool,
    declaration: ToolPkgCoreCommandRuntime,
}

/// Owns the uniquely selected declaration; execution never resolves its name again.
#[derive(Clone, Debug)]
pub struct ResolvedToolPkgCoreCommand {
    registration: Registration,
}

impl ResolvedToolPkgCoreCommand {
    /// Returns immutable metadata identifying the single selected command owner.
    pub fn info(&self) -> &ToolPkgCoreCommandInfo {
        &self.registration.info
    }

    /// Borrows the exact handler declaration selected by the catalog.
    pub(super) fn declaration(&self) -> &ToolPkgCoreCommandRuntime {
        &self.registration.declaration
    }
}

/// Captures enabled and disabled declarations from one completed canonical registry snapshot.
#[derive(Clone, Debug)]
pub struct ToolPkgCoreCommandCatalog {
    registrations: Vec<Registration>,
}

impl ToolPkgCoreCommandCatalog {
    /// Builds the command directory from actual package runtimes, never from a business root allow-list.
    pub(super) fn new(
        containers: Vec<ToolPkgContainerRuntime>,
        enabled_containers: BTreeSet<String>,
        use_english: bool,
    ) -> Self {
        let mut registrations = Vec::new();
        for container in containers {
            let enabled = enabled_containers.contains(&container.packageName);
            for declaration in container.coreCommands {
                let info = ToolPkgCoreCommandInfo {
                    containerPackageName: container.packageName.clone(),
                    commandId: declaration.id.clone(),
                    name: declaration.name.clone(),
                    title: declaration.title.resolve(use_english),
                    description: declaration.description.resolve(use_english),
                    usage: declaration.usage.clone(),
                };
                registrations.push(Registration {
                    info,
                    enabled,
                    declaration,
                });
            }
        }
        registrations.sort_by(|left, right| {
            left.info
                .name
                .to_ascii_lowercase()
                .cmp(&right.info.name.to_ascii_lowercase())
                .then_with(|| {
                    left.info
                        .containerPackageName
                        .cmp(&right.info.containerPackageName)
                })
        });
        Self { registrations }
    }

    /// Lists every actual declaration with its explicit enabled state for generic root routing and help.
    pub fn registrations(&self) -> Vec<ToolPkgCoreCommandRegistrationInfo> {
        self.registrations
            .iter()
            .map(|registration| ToolPkgCoreCommandRegistrationInfo {
                command: registration.info.clone(),
                enabled: registration.enabled,
            })
            .collect()
    }

    /// Resolves one exact root name and rejects disabled, duplicate, or builtin-colliding ownership.
    pub fn resolve(
        &self,
        command_name: &str,
        builtin_command_names: &[&str],
    ) -> Result<Option<ResolvedToolPkgCoreCommand>, String> {
        let name = command_name.trim();
        if name.is_empty() {
            return Err("Command name is empty".to_string());
        }
        let candidates = self
            .registrations
            .iter()
            .filter(|registration| registration.info.name.eq_ignore_ascii_case(name))
            .collect::<Vec<_>>();
        if !candidates.is_empty()
            && builtin_command_names
                .iter()
                .any(|builtin| builtin.eq_ignore_ascii_case(name))
        {
            return Err(format!(
                "plugin command /{name} collides with builtin Core command: {name}"
            ));
        }
        match candidates.as_slice() {
            [] => Ok(None),
            [registration] => {
                if !registration.enabled {
                    return Err(format!(
                        "plugin command is disabled: /{name} ({})",
                        registration.info.containerPackageName
                    ));
                }
                Ok(Some(ResolvedToolPkgCoreCommand {
                    registration: (**registration).clone(),
                }))
            }
            registrations => {
                let owners = registrations
                    .iter()
                    .map(|registration| registration.info.containerPackageName.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(format!(
                    "duplicate registered plugin command /{name}: {owners}"
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_plugin_sdk::package::LocalizedText;

    /// Creates an explicit registration fixture with the real runtime handler record.
    fn registration(name: &str, owner: &str, enabled: bool) -> Registration {
        Registration {
            info: ToolPkgCoreCommandInfo {
                containerPackageName: owner.to_string(),
                commandId: "handler-id".to_string(),
                name: name.to_string(),
                title: name.to_string(),
                description: "description".to_string(),
                usage: "usage".to_string(),
            },
            enabled,
            declaration: ToolPkgCoreCommandRuntime {
                id: "handler-id".to_string(),
                name: name.to_string(),
                title: LocalizedText {
                    values: std::collections::HashMap::from([(
                        "default".to_string(),
                        name.to_string(),
                    )]),
                },
                description: LocalizedText {
                    values: std::collections::HashMap::from([(
                        "default".to_string(),
                        "description".to_string(),
                    )]),
                },
                usage: "usage".to_string(),
                function: "handler".to_string(),
                functionSource: None,
            },
        }
    }

    /// Exposes arbitrary registered names without enumerating migrated business roots.
    #[test]
    fn arbitrary_command_name_is_resolved_once() {
        let catalog = ToolPkgCoreCommandCatalog {
            registrations: vec![registration("custom-command", "plugin.owner", true)],
        };
        let resolved = catalog
            .resolve("CUSTOM-COMMAND", &["plugin", "chat"])
            .unwrap()
            .unwrap();
        assert_eq!(resolved.info().containerPackageName, "plugin.owner");
        assert_eq!(resolved.declaration().function, "handler");
    }

    /// Distinguishes an absent registration from a registered but disabled provider.
    #[test]
    fn absent_and_disabled_names_have_distinct_outcomes() {
        let catalog = ToolPkgCoreCommandCatalog {
            registrations: vec![registration("custom-command", "plugin.owner", false)],
        };
        assert!(catalog.resolve("unregistered", &[]).unwrap().is_none());
        assert_eq!(
            catalog.resolve("custom-command", &[]).unwrap_err(),
            "plugin command is disabled: /custom-command (plugin.owner)"
        );
    }

    /// Rejects builtin ownership collisions before any handler can execute.
    #[test]
    fn builtin_collision_is_an_error() {
        let catalog = ToolPkgCoreCommandCatalog {
            registrations: vec![registration("chat", "plugin.owner", true)],
        };
        assert_eq!(
            catalog.resolve("chat", &["chat"]).unwrap_err(),
            "plugin command /chat collides with builtin Core command: chat"
        );
    }

    /// Rejects duplicate declarations case-insensitively instead of choosing the first provider.
    #[test]
    fn duplicate_ownership_is_an_error() {
        let catalog = ToolPkgCoreCommandCatalog {
            registrations: vec![
                registration("custom-command", "first.owner", true),
                registration("CUSTOM-COMMAND", "second.owner", true),
            ],
        };
        assert_eq!(
            catalog.resolve("custom-command", &[]).unwrap_err(),
            "duplicate registered plugin command /custom-command: first.owner, second.owner"
        );
    }
}
