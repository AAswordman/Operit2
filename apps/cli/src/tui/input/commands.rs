use super::i18n::{TuiLanguage, TuiTextKey};

#[derive(Clone, Copy, Debug)]
pub(super) struct TuiCommandSpec {
    pub(super) name: &'static str,
    pub(super) usage: &'static str,
    pub(super) description_key: TuiTextKey,
}

const COMMAND_SPECS: [TuiCommandSpec; 41] = [
    TuiCommandSpec {
        name: "help",
        usage: "/help",
        description_key: TuiTextKey::CommandHelpDescription,
    },
    TuiCommandSpec {
        name: "new",
        usage: "/new [--character <name>] [--group-card <id>] [--group <name>]",
        description_key: TuiTextKey::CommandNewDescription,
    },
    TuiCommandSpec {
        name: "switch",
        usage: "/switch",
        description_key: TuiTextKey::CommandSwitchDescription,
    },
    TuiCommandSpec {
        name: "resume",
        usage: "/resume",
        description_key: TuiTextKey::CommandResumeDescription,
    },
    TuiCommandSpec {
        name: "max",
        usage: "/max",
        description_key: TuiTextKey::CommandMaxDescription,
    },
    TuiCommandSpec {
        name: "language",
        usage: "/language [en|zh-CN]",
        description_key: TuiTextKey::CommandLanguageDescription,
    },
    TuiCommandSpec {
        name: "model",
        usage: "/model",
        description_key: TuiTextKey::CommandModelDescription,
    },
    TuiCommandSpec {
        name: "model current",
        usage: "/model current",
        description_key: TuiTextKey::CommandModelDescription,
    },
    TuiCommandSpec {
        name: "model list",
        usage: "/model list",
        description_key: TuiTextKey::CommandModelListDescription,
    },
    TuiCommandSpec {
        name: "model choose",
        usage: "/model choose",
        description_key: TuiTextKey::CommandModelChooseDescription,
    },
    TuiCommandSpec {
        name: "model use",
        usage: "/model use <provider-id> <model-id>",
        description_key: TuiTextKey::CommandModelUseDescription,
    },
    TuiCommandSpec {
        name: "model config",
        usage: "/model config",
        description_key: TuiTextKey::CommandModelConfigDescription,
    },
    TuiCommandSpec {
        name: "approval",
        usage: "/approval",
        description_key: TuiTextKey::CommandApprovalDescription,
    },
    TuiCommandSpec {
        name: "approval allow",
        usage: "/approval allow",
        description_key: TuiTextKey::CommandApprovalAllowDescription,
    },
    TuiCommandSpec {
        name: "approval ask",
        usage: "/approval ask",
        description_key: TuiTextKey::CommandApprovalAskDescription,
    },
    TuiCommandSpec {
        name: "approval forbid",
        usage: "/approval forbid",
        description_key: TuiTextKey::CommandApprovalForbidDescription,
    },
    TuiCommandSpec {
        name: "approval tool",
        usage: "/approval tool <tool> <allow|ask|forbid|clear>",
        description_key: TuiTextKey::CommandApprovalToolDescription,
    },
    TuiCommandSpec {
        name: "attach",
        usage: "/attach <path>",
        description_key: TuiTextKey::CommandAttachDescription,
    },
    TuiCommandSpec {
        name: "attachments",
        usage: "/attachments",
        description_key: TuiTextKey::CommandAttachmentsDescription,
    },
    TuiCommandSpec {
        name: "clear-attachments",
        usage: "/clear-attachments",
        description_key: TuiTextKey::CommandClearAttachmentsDescription,
    },
    TuiCommandSpec {
        name: "queue",
        usage: "/queue",
        description_key: TuiTextKey::CommandQueueDescription,
    },
    TuiCommandSpec {
        name: "queue clear",
        usage: "/queue clear",
        description_key: TuiTextKey::CommandQueueClearDescription,
    },
    TuiCommandSpec {
        name: "queue delete",
        usage: "/queue delete <id>",
        description_key: TuiTextKey::CommandQueueDeleteDescription,
    },
    TuiCommandSpec {
        name: "queue edit",
        usage: "/queue edit <id>",
        description_key: TuiTextKey::CommandQueueEditDescription,
    },
    TuiCommandSpec {
        name: "queue send",
        usage: "/queue send <id>",
        description_key: TuiTextKey::CommandQueueSendDescription,
    },
    TuiCommandSpec {
        name: "quit",
        usage: "/quit",
        description_key: TuiTextKey::CommandQuitDescription,
    },
    TuiCommandSpec {
        name: "exit",
        usage: "/exit",
        description_key: TuiTextKey::CommandQuitDescription,
    },
    TuiCommandSpec {
        name: "character",
        usage: "/character",
        description_key: TuiTextKey::CommandCharacterDescription,
    },
    TuiCommandSpec {
        name: "character choose",
        usage: "/character choose",
        description_key: TuiTextKey::CommandCharacterChooseDescription,
    },
    TuiCommandSpec {
        name: "group",
        usage: "/group",
        description_key: TuiTextKey::CommandGroupDescription,
    },
    TuiCommandSpec {
        name: "group choose",
        usage: "/group choose",
        description_key: TuiTextKey::CommandGroupChooseDescription,
    },
    TuiCommandSpec {
        name: "skill",
        usage: "/skill",
        description_key: TuiTextKey::CommandSkillDescription,
    },
    TuiCommandSpec {
        name: "skill toggle",
        usage: "/skill toggle <name>",
        description_key: TuiTextKey::CommandSkillToggleDescription,
    },
    TuiCommandSpec {
        name: "package",
        usage: "/package",
        description_key: TuiTextKey::CommandPackageDescription,
    },
    TuiCommandSpec {
        name: "package toggle",
        usage: "/package toggle <name>",
        description_key: TuiTextKey::CommandPackageToggleDescription,
    },
    TuiCommandSpec {
        name: "plugin",
        usage: "/plugin",
        description_key: TuiTextKey::CommandPluginDescription,
    },
    TuiCommandSpec {
        name: "plugin toggle",
        usage: "/plugin toggle <name>",
        description_key: TuiTextKey::CommandPluginToggleDescription,
    },
    TuiCommandSpec {
        name: "mcp",
        usage: "/mcp",
        description_key: TuiTextKey::CommandMcpDescription,
    },
    TuiCommandSpec {
        name: "mcp toggle",
        usage: "/mcp toggle <name>",
        description_key: TuiTextKey::CommandMcpToggleDescription,
    },
    TuiCommandSpec {
        name: "tag",
        usage: "/tag",
        description_key: TuiTextKey::CommandTagDescription,
    },
    TuiCommandSpec {
        name: "update",
        usage: "/update",
        description_key: TuiTextKey::CommandUpdateDescription,
    },
];

impl TuiCommandSpec {
    pub(super) fn description(self, language: TuiLanguage) -> &'static str {
        language.text().raw(self.description_key)
    }
}

pub(super) fn command_specs() -> &'static [TuiCommandSpec] {
    &COMMAND_SPECS
}

pub(super) fn matching_command_specs(input: &str) -> Vec<TuiCommandSpec> {
    let Some(prefix) = active_command_prefix(input) else {
        return Vec::new();
    };
    command_specs()
        .iter()
        .copied()
        .filter(|spec| {
            if prefix.is_empty() {
                return !spec.name.contains(' ');
            }
            if prefix.chars().any(|ch| ch.is_whitespace()) {
                return spec.name.starts_with(prefix.as_str());
            }
            spec.name
                .split_whitespace()
                .next()
                .map(|name| name.starts_with(prefix.as_str()))
                .unwrap_or(false)
                && !spec.name.contains(' ')
        })
        .collect()
}

pub(super) fn complete_command_input(_input: &str, command: TuiCommandSpec) -> (String, usize) {
    let command_text = command
        .usage
        .split_whitespace()
        .take_while(|part| !part.starts_with('<') && !part.starts_with('['))
        .collect::<Vec<_>>()
        .join(" ");
    let completed = format!("{command_text} ");
    let cursor = completed.chars().count();
    (completed, cursor)
}

fn active_command_prefix(input: &str) -> Option<String> {
    let stripped = input.strip_prefix('/')?;
    if stripped.contains('\n') {
        return None;
    }
    Some(stripped.trim_start().to_ascii_lowercase())
}
