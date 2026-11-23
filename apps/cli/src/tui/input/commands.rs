use super::i18n::{TuiLanguage, TuiTextKey};

/// Where an argument's completion candidates come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ArgSource {
    /// No candidates; the value is typed free-form.
    None,
    /// Live chat ids from the session list.
    ChatId,
    /// Space member device ids from the topology snapshot.
    DeviceId,
    /// File names in the process working directory.
    FilePath,
}

/// The kind of candidates offered at one argument position.
#[derive(Clone, Copy, Debug)]
pub(super) enum TuiCommandArgKind {
    /// Fixed literal candidates, completed verbatim.
    Enum(&'static [&'static str]),
    /// Free-form value; dynamic sources refine it when available.
    Free,
}

/// One positional argument of a builtin command.
#[derive(Clone, Copy, Debug)]
pub(super) struct TuiCommandArgSpec {
    /// Placeholder label without brackets (rendered `<label>`/`[label]`).
    pub(super) label: &'static str,
    pub(super) kind: TuiCommandArgKind,
    /// Required arguments render `<>`, optional ones `[]`.
    pub(super) required: bool,
    pub(super) source: ArgSource,
}

/// One keyword-value option pair (only `/new` today). Options complete in
/// any order and each takes one free-form value.
#[derive(Clone, Copy, Debug)]
pub(super) struct TuiCommandOption {
    pub(super) keyword: &'static str,
    /// Value placeholder label without brackets.
    pub(super) value_label: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TuiCommandSpec {
    /// Space-separated command path without the leading slash.
    pub(super) name: &'static str,
    pub(super) description_key: TuiTextKey,
    /// Positional arguments after the command path.
    pub(super) args: &'static [TuiCommandArgSpec],
    /// Keyword-value options completing in any order after the command.
    pub(super) options: &'static [TuiCommandOption],
}

/// One declared argument of a plugin command (structured protocol).
#[derive(Clone, Debug)]
pub(super) struct TuiPluginArgSpec {
    /// Placeholder label without brackets.
    pub(super) label: String,
    /// Fixed literal candidates; empty means free-form.
    pub(super) options: Vec<String>,
    pub(super) required: bool,
}

/// Describes one plugin-provided slash command returned by Core.
#[derive(Clone, Debug)]
pub(super) struct TuiPluginCommandSpec {
    pub(super) name: String,
    pub(super) usage: String,
    pub(super) description: String,
    /// Structured argument declarations; empty for legacy commands whose
    /// completion falls back to the usage string.
    pub(super) args: Vec<TuiPluginArgSpec>,
}

/// One argument-value completion candidate.
#[derive(Clone, Debug)]
pub(super) struct TuiValueSuggestion {
    pub(super) text: String,
    /// Source label shown beside the candidate ("chat", "device", "file").
    pub(super) hint: String,
}

/// Completion candidates for dynamic argument sources, gathered from live
/// app state by the caller.
pub(super) struct ValueContext {
    pub(super) chat_ids: Vec<String>,
    pub(super) device_ids: Vec<String>,
}

/// Represents either a built-in, runtime-provided, or argument-value
/// completion suggestion.
#[derive(Clone, Debug)]
pub(super) enum TuiCommandSuggestion {
    Builtin(TuiCommandSpec),
    Plugin(TuiPluginCommandSpec),
    Value(TuiValueSuggestion),
}

/// Returns whether a usage token renders as a parameter placeholder.
pub(super) fn is_param_token(token: &str) -> bool {
    token.starts_with('<') || token.starts_with('[')
}

fn render_arg(arg: &TuiCommandArgSpec) -> String {
    if arg.required {
        format!("<{}>", arg.label)
    } else {
        format!("[{}]", arg.label)
    }
}

fn render_plugin_arg(arg: &TuiPluginArgSpec) -> String {
    if arg.required {
        format!("<{}>", arg.label)
    } else {
        format!("[{}]", arg.label)
    }
}

impl TuiCommandSuggestion {
    /// Returns the command name without its leading slash; value
    /// suggestions return their literal text.
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Builtin(spec) => spec.name,
            Self::Plugin(spec) => &spec.name,
            Self::Value(value) => &value.text,
        }
    }

    /// Returns the full usage line (or the literal value candidate).
    pub(super) fn usage(&self) -> String {
        match self {
            Self::Builtin(spec) => spec.usage(),
            Self::Plugin(spec) => spec.usage.clone(),
            Self::Value(value) => value.text.clone(),
        }
    }

    /// Splits the usage into `(token, is_parameter)` pairs so the popup can
    /// color placeholders differently from command words.
    pub(super) fn usage_tokens(&self) -> Vec<(String, bool)> {
        self.usage()
            .split_whitespace()
            .map(|token| (token.to_string(), is_param_token(token)))
            .collect()
    }

    /// Returns the localized or plugin-provided command description.
    pub(super) fn description(&self, language: TuiLanguage) -> String {
        match self {
            Self::Builtin(spec) => spec.description(language).to_string(),
            Self::Plugin(spec) => spec.description.clone(),
            Self::Value(value) => value.hint.clone(),
        }
    }
}

impl TuiCommandSpec {
    /// Generates the usage line from the structured declaration, so the
    /// popup, ghost hint and dispatcher share one source of truth.
    pub(super) fn usage(&self) -> String {
        let mut usage = format!("/{}", self.name);
        for option in self.options {
            usage.push_str(&format!(" [{} <{}>]", option.keyword, option.value_label));
        }
        for arg in self.args {
            usage.push(' ');
            usage.push_str(&render_arg(arg));
        }
        usage
    }

    pub(super) fn description(self, language: TuiLanguage) -> &'static str {
        language.text().raw(self.description_key)
    }
}

const COMMAND_SPECS: &[TuiCommandSpec] = &[
    TuiCommandSpec {
        name: "help",
        description_key: TuiTextKey::CommandHelpDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "diff",
        description_key: TuiTextKey::CommandDiffDescription,
        args: &[TuiCommandArgSpec {
            label: "file",
            kind: TuiCommandArgKind::Free,
            required: false,
            source: ArgSource::FilePath,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "new",
        description_key: TuiTextKey::CommandNewDescription,
        args: &[],
        options: &[
            TuiCommandOption {
                keyword: "source",
                value_label: "chat-id",
            },
            TuiCommandOption {
                keyword: "input",
                value_label: "json-object",
            },
        ],
    },
    TuiCommandSpec {
        name: "new source",
        description_key: TuiTextKey::CommandNewDescription,
        args: &[TuiCommandArgSpec {
            label: "chat-id",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::ChatId,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "new input",
        description_key: TuiTextKey::CommandNewDescription,
        args: &[TuiCommandArgSpec {
            label: "json-object",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "switch",
        description_key: TuiTextKey::CommandSwitchDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "resume",
        description_key: TuiTextKey::CommandResumeDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "language",
        description_key: TuiTextKey::CommandLanguageDescription,
        args: &[TuiCommandArgSpec {
            label: "en|zh-CN",
            kind: TuiCommandArgKind::Enum(&["en", "zh-CN"]),
            required: false,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network",
        description_key: TuiTextKey::CommandNetworkDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network show",
        description_key: TuiTextKey::CommandNetworkShowDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network bootstrap",
        description_key: TuiTextKey::CommandNetworkBootstrapDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network audit",
        description_key: TuiTextKey::CommandNetworkAuditDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network doctor",
        description_key: TuiTextKey::CommandNetworkDoctorDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network devices",
        description_key: TuiTextKey::CommandNetworkDevicesDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network identities",
        description_key: TuiTextKey::CommandNetworkIdentitiesDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity list",
        description_key: TuiTextKey::CommandNetworkIdentityListDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity define",
        description_key: TuiTextKey::CommandNetworkIdentityDefineDescription,
        args: &[
            TuiCommandArgSpec {
                label: "name",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
            TuiCommandArgSpec {
                label: "capability...",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity update",
        description_key: TuiTextKey::CommandNetworkIdentityUpdateDescription,
        args: &[
            TuiCommandArgSpec {
                label: "identity",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
            TuiCommandArgSpec {
                label: "name",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
            TuiCommandArgSpec {
                label: "capability...",
                kind: TuiCommandArgKind::Free,
                required: false,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity delete",
        description_key: TuiTextKey::CommandNetworkIdentityDeleteDescription,
        args: &[TuiCommandArgSpec {
            label: "identity",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity set",
        description_key: TuiTextKey::CommandNetworkIdentitySetDescription,
        args: &[
            TuiCommandArgSpec {
                label: "device",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::DeviceId,
            },
            TuiCommandArgSpec {
                label: "identity",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity clear",
        description_key: TuiTextKey::CommandNetworkIdentityClearDescription,
        args: &[TuiCommandArgSpec {
            label: "device",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::DeviceId,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network name",
        description_key: TuiTextKey::CommandNetworkNameDescription,
        args: &[TuiCommandArgSpec {
            label: "name",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network admit",
        description_key: TuiTextKey::CommandNetworkAdmitDescription,
        args: &[TuiCommandArgSpec {
            label: "device",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::DeviceId,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network remove",
        description_key: TuiTextKey::CommandNetworkRemoveDescription,
        args: &[TuiCommandArgSpec {
            label: "device",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::DeviceId,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network disconnect",
        description_key: TuiTextKey::CommandNetworkDisconnectDescription,
        args: &[TuiCommandArgSpec {
            label: "device",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::DeviceId,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network policy list",
        description_key: TuiTextKey::CommandNetworkPolicyListDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network policy set",
        description_key: TuiTextKey::CommandNetworkPolicySetDescription,
        args: &[
            TuiCommandArgSpec {
                label: "key",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
            TuiCommandArgSpec {
                label: "value",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "network prompts",
        description_key: TuiTextKey::CommandNetworkPromptsDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network requests",
        description_key: TuiTextKey::CommandNetworkRequestsDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network approve",
        description_key: TuiTextKey::CommandNetworkApproveDescription,
        args: &[
            TuiCommandArgSpec {
                label: "device|request-id",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
            TuiCommandArgSpec {
                label: "assignment-version",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "network reject",
        description_key: TuiTextKey::CommandNetworkRejectDescription,
        args: &[
            TuiCommandArgSpec {
                label: "device|request-id",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
            TuiCommandArgSpec {
                label: "assignment-version",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "network discover",
        description_key: TuiTextKey::CommandNetworkDiscoverDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network pair",
        description_key: TuiTextKey::CommandNetworkPairDescription,
        args: &[
            TuiCommandArgSpec {
                label: "address",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
            TuiCommandArgSpec {
                label: "http|ws|tcp|serial|bluetooth",
                kind: TuiCommandArgKind::Enum(&["http", "ws", "tcp", "serial", "bluetooth"]),
                required: true,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "network pair-confirm",
        description_key: TuiTextKey::CommandNetworkPairConfirmDescription,
        args: &[
            TuiCommandArgSpec {
                label: "pairing-id",
                kind: TuiCommandArgKind::Free,
                required: false,
                source: ArgSource::None,
            },
            TuiCommandArgSpec {
                label: "code",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "network pair-cancel",
        description_key: TuiTextKey::CommandNetworkPairCancelDescription,
        args: &[TuiCommandArgSpec {
            label: "pairing-id",
            kind: TuiCommandArgKind::Free,
            required: false,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network peers",
        description_key: TuiTextKey::CommandNetworkPeersDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network unpair",
        description_key: TuiTextKey::CommandNetworkUnpairDescription,
        args: &[TuiCommandArgSpec {
            label: "node-id|name",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::DeviceId,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network join",
        description_key: TuiTextKey::CommandNetworkJoinDescription,
        args: &[TuiCommandArgSpec {
            label: "node-id|name",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::DeviceId,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network joins",
        description_key: TuiTextKey::CommandNetworkJoinsDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network join-cancel",
        description_key: TuiTextKey::CommandNetworkJoinCancelDescription,
        args: &[TuiCommandArgSpec {
            label: "request-id|node-id",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "network listen",
        description_key: TuiTextKey::CommandNetworkListenDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network listen show",
        description_key: TuiTextKey::CommandNetworkListenShowDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "network endpoint",
        description_key: TuiTextKey::CommandNetworkEndpointDescription,
        args: &[
            TuiCommandArgSpec {
                label: "node-id|name",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::DeviceId,
            },
            TuiCommandArgSpec {
                label: "address",
                kind: TuiCommandArgKind::Free,
                required: false,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "network leave",
        description_key: TuiTextKey::CommandNetworkLeaveDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "model",
        description_key: TuiTextKey::CommandModelDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "model current",
        description_key: TuiTextKey::CommandModelDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "model list",
        description_key: TuiTextKey::CommandModelListDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "model choose",
        description_key: TuiTextKey::CommandModelChooseDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "model use",
        description_key: TuiTextKey::CommandModelUseDescription,
        args: &[
            TuiCommandArgSpec {
                label: "provider-id",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
            TuiCommandArgSpec {
                label: "model-id",
                kind: TuiCommandArgKind::Free,
                required: true,
                source: ArgSource::None,
            },
        ],
        options: &[],
    },
    TuiCommandSpec {
        name: "model config",
        description_key: TuiTextKey::CommandModelConfigDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "approval",
        description_key: TuiTextKey::CommandApprovalDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "approval allow",
        description_key: TuiTextKey::CommandApprovalAllowDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "approval ask",
        description_key: TuiTextKey::CommandApprovalAskDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "approval forbid",
        description_key: TuiTextKey::CommandApprovalForbidDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "attach",
        description_key: TuiTextKey::CommandAttachDescription,
        args: &[TuiCommandArgSpec {
            label: "path",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::FilePath,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "attachments",
        description_key: TuiTextKey::CommandAttachmentsDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "clear-attachments",
        description_key: TuiTextKey::CommandClearAttachmentsDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "queue",
        description_key: TuiTextKey::CommandQueueDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "queue clear",
        description_key: TuiTextKey::CommandQueueClearDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "queue delete",
        description_key: TuiTextKey::CommandQueueDeleteDescription,
        args: &[TuiCommandArgSpec {
            label: "id",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "queue edit",
        description_key: TuiTextKey::CommandQueueEditDescription,
        args: &[TuiCommandArgSpec {
            label: "id",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "queue send",
        description_key: TuiTextKey::CommandQueueSendDescription,
        args: &[TuiCommandArgSpec {
            label: "id",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "quit",
        description_key: TuiTextKey::CommandQuitDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "exit",
        description_key: TuiTextKey::CommandQuitDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "character",
        description_key: TuiTextKey::CommandCharacterDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "character choose",
        description_key: TuiTextKey::CommandCharacterChooseDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "group",
        description_key: TuiTextKey::CommandGroupDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "group choose",
        description_key: TuiTextKey::CommandGroupChooseDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "skill",
        description_key: TuiTextKey::CommandSkillDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "package",
        description_key: TuiTextKey::CommandPackageDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "package toggle",
        description_key: TuiTextKey::CommandPackageToggleDescription,
        args: &[TuiCommandArgSpec {
            label: "name",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "plugin",
        description_key: TuiTextKey::CommandPluginDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "plugin toggle",
        description_key: TuiTextKey::CommandPluginToggleDescription,
        args: &[TuiCommandArgSpec {
            label: "name",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "mcp",
        description_key: TuiTextKey::CommandMcpDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "mcp toggle",
        description_key: TuiTextKey::CommandMcpToggleDescription,
        args: &[TuiCommandArgSpec {
            label: "name",
            kind: TuiCommandArgKind::Free,
            required: true,
            source: ArgSource::None,
        }],
        options: &[],
    },
    TuiCommandSpec {
        name: "status",
        description_key: TuiTextKey::CommandStatusDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "log",
        description_key: TuiTextKey::CommandLogDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "tag",
        description_key: TuiTextKey::CommandTagDescription,
        args: &[],
        options: &[],
    },
    TuiCommandSpec {
        name: "update",
        description_key: TuiTextKey::CommandUpdateDescription,
        args: &[],
        options: &[],
    },
];

/// How many file candidates the directory source offers at most.
const FILE_CANDIDATE_LIMIT: usize = 64;

/// Expands a plugin usage string following the `/name <a|b c|d>` or
/// `/name [a|b c|d]` subcommand convention into one suggestion per
/// alternative, so plugin commands complete like builtin path commands.
/// The parent entry keeps a shortened usage; usages outside the convention
/// are returned unchanged as a single entry.
pub(super) fn expand_plugin_command(
    name: String,
    usage: String,
    description: String,
) -> Vec<TuiPluginCommandSpec> {
    let single = || {
        vec![TuiPluginCommandSpec {
            name: name.clone(),
            usage: usage.clone(),
            description: description.clone(),
            args: Vec::new(),
        }]
    };
    let command_prefix = format!("/{name}");
    let Some(rest) = usage.strip_prefix(&command_prefix) else {
        return single();
    };
    let rest = rest.trim();
    if !rest.starts_with('<') && !rest.starts_with('[') {
        return single();
    }
    if !rest.ends_with('>') && !rest.ends_with(']') {
        return single();
    }
    let inner = &rest[1..rest.len() - 1];
    let alternatives: Vec<&str> = inner.split('|').map(str::trim).filter(|a| !a.is_empty()).collect();
    if alternatives.len() < 2 {
        return single();
    }
    // Alternatives must start with literal words; `<value>`-first usages
    // describe plain arguments, not subcommands.
    if alternatives
        .iter()
        .any(|a| a.starts_with('<') || a.starts_with('['))
    {
        return single();
    }
    let mut expanded = vec![TuiPluginCommandSpec {
        name: name.clone(),
        usage: command_prefix,
        description: description.clone(),
        args: Vec::new(),
    }];
    for alternative in alternatives {
        let literal = alternative.split_whitespace().next().unwrap_or(alternative);
        expanded.push(TuiPluginCommandSpec {
            name: format!("{name} {literal}"),
            usage: format!("/{name} {alternative}"),
            description: description.clone(),
            args: Vec::new(),
        });
    }
    expanded
}

pub(super) fn command_specs() -> &'static [TuiCommandSpec] {
    COMMAND_SPECS
}

/// Returns the keyword-value options declared for a command word, if any.
/// Shared with the command parser so completion and dispatch stay in sync.
pub(super) fn keyword_options_for(command: &str) -> Option<&'static [TuiCommandOption]> {
    command_specs()
        .iter()
        .find(|spec| spec.name == command && !spec.options.is_empty())
        .map(|spec| spec.options)
}

/// Builds the `/network` dispatcher usage line from the command table, so
/// new subcommands cannot drift out of the error text.
pub(super) fn network_command_usage() -> String {
    let mut words: Vec<&str> = Vec::new();
    for spec in command_specs() {
        let Some(rest) = spec.name.strip_prefix("network ") else {
            continue;
        };
        let Some(first) = rest.split_whitespace().next() else {
            continue;
        };
        if !words.contains(&first) {
            words.push(first);
        }
    }
    format!("network subcommands: <{}>", words.join("|"))
}

/// Keyword-value options combine freely and each takes a free-form value, so
/// plain prefix matching cannot suggest the remaining options once a value is
/// typed. Match by the keywords the input already consumes instead.
fn match_keyword_options_prefix(
    prefix: &str,
    command: &str,
    options: &[TuiCommandOption],
    spec: &TuiCommandSuggestion,
) -> bool {
    let name = spec.name();
    if name == command {
        return !prefix.contains(' ');
    }
    let command_prefix = format!("{command} ");
    let Some(keyword) = name.strip_prefix(&command_prefix) else {
        return false;
    };
    let Some(stripped) = prefix.strip_prefix(&command_prefix) else {
        return false;
    };
    let keyword_used = |token: &str| options.iter().any(|option| option.keyword == token);
    let trailing_space = stripped.ends_with(' ');
    let tokens: Vec<&str> = stripped.split_whitespace().collect();
    if trailing_space {
        // A bare keyword is waiting for its value; a completed value means
        // this keyword is consumed and the remaining ones are up next.
        !tokens.iter().any(|token| *token == keyword)
            && !tokens.last().is_some_and(|last| keyword_used(last))
    } else if let Some(partial) = tokens.last() {
        keyword.starts_with(partial) && !tokens.iter().any(|token| *token == keyword)
    } else {
        !tokens.iter().any(|t| keyword_used(t))
    }
}

/// Splits a lowercased input prefix into whitespace tokens.
fn prefix_tokens(prefix: &str) -> Vec<&str> {
    prefix.split_whitespace().collect()
}

pub(super) fn matching_command_specs(
    input: &str,
    plugin_commands: &[TuiPluginCommandSpec],
) -> Vec<TuiCommandSuggestion> {
    let Some(prefix) = active_command_prefix(input) else {
        return Vec::new();
    };
    let mut suggestions = command_specs()
        .iter()
        .cloned()
        .map(TuiCommandSuggestion::Builtin)
        .collect::<Vec<_>>();
    suggestions.extend(
        plugin_commands
            .iter()
            .cloned()
            .map(TuiCommandSuggestion::Plugin),
    );
    let tokens = prefix_tokens(&prefix);
    let last_partial = !prefix.ends_with(' ');
    let command = tokens.first().copied().unwrap_or(prefix.as_str());
    if let Some(options) = keyword_options_for(command) {
        return suggestions
            .into_iter()
            .filter(|spec| match_keyword_options_prefix(&prefix, command, options, spec))
            .collect();
    }
    suggestions
        .into_iter()
        .filter(|spec| {
            if tokens.is_empty() {
                return !spec.name().contains(' ');
            }
            if last_partial && tokens.len() == 1 {
                // A single partial word lists bare commands only.
                let first = spec.name().split_whitespace().next().unwrap_or_default();
                return !spec.name().contains(' ') && first.starts_with(tokens[0]);
            }
            let name_words: Vec<&str> = spec.name().split_whitespace().collect();
            if tokens.len() > name_words.len() {
                return false;
            }
            let complete = tokens.len() - usize::from(last_partial);
            if name_words[..complete]
                .iter()
                .zip(&tokens[..complete])
                .any(|(word, typed)| word != typed)
            {
                return false;
            }
            if last_partial {
                // The partially typed word must prefix the next path word.
                name_words[complete].starts_with(tokens[complete])
            } else {
                // A trailing space means the path must extend beyond the
                // typed words; exact matches drop out.
                name_words.len() > tokens.len()
            }
        })
        .collect()
}

/// Returns argument-value suggestions for the active argument position, or
/// an empty list when the position has no candidates (free-form values stay
/// silent, matching the old behavior). Later positions work too: the spec
/// whose path is the longest prefix of the typed tokens owns the position.
pub(super) fn value_suggestions(
    input: &str,
    plugin_commands: &[TuiPluginCommandSpec],
    context: &ValueContext,
) -> Vec<TuiCommandSuggestion> {
    let Some(prefix) = active_command_prefix(input) else {
        return Vec::new();
    };
    let tokens = prefix_tokens(&prefix);
    if tokens.is_empty() {
        return Vec::new();
    }
    let trailing_space = prefix.ends_with(' ');
    let mut all = command_specs()
        .iter()
        .cloned()
        .map(TuiCommandSuggestion::Builtin)
        .collect::<Vec<_>>();
    all.extend(
        plugin_commands
            .iter()
            .cloned()
            .map(TuiCommandSuggestion::Plugin),
    );
    // Longest command path first, so `/network pair` wins over `/network`.
    all.sort_by_key(|suggestion| {
        std::cmp::Reverse(suggestion.name().split_whitespace().count())
    });
    for spec in &all {
        let name_words: Vec<&str> = spec.name().split_whitespace().collect();
        if name_words.len() > tokens.len()
            || !name_words.iter().zip(&tokens).all(|(word, typed)| word == typed)
        {
            continue;
        }
        let consumed = tokens.len() - name_words.len();
        let (arg_index, partial) = if trailing_space {
            (consumed, String::new())
        } else {
            (
                consumed.saturating_sub(1),
                tokens.last().copied().unwrap_or_default().to_string(),
            )
        };
        let candidates = match spec {
            TuiCommandSuggestion::Builtin(builtin) => {
                // Keyword options own their value positions; they complete
                // via the path popup, not value candidates.
                if !builtin.options.is_empty() {
                    continue;
                }
                builtin
                    .args
                    .get(arg_index)
                    .map(|arg| arg_candidates(arg.label, arg.kind, arg.source, &partial, context))
                    .unwrap_or_default()
            }
            TuiCommandSuggestion::Plugin(plugin) => plugin
                .args
                .get(arg_index)
                .map(|arg| {
                    plugin_arg_candidates(&arg.label, &arg.options, arg.required, &partial, context)
                })
                .unwrap_or_default(),
            TuiCommandSuggestion::Value(_) => Vec::new(),
        };
        if !candidates.is_empty() {
            return candidates;
        }
    }
    Vec::new()
}

/// Builds candidates for one argument position: fixed enum options first,
/// then dynamic source values, all filtered by the typed prefix.
fn arg_candidates(
    label: &str,
    kind: TuiCommandArgKind,
    source: ArgSource,
    partial: &str,
    context: &ValueContext,
) -> Vec<TuiCommandSuggestion> {
    let mut candidates = Vec::new();
    let partial = partial.to_ascii_lowercase();
    if let TuiCommandArgKind::Enum(options) = kind {
        candidates.extend(options.iter().filter(|option| {
            partial.is_empty() || option.to_ascii_lowercase().starts_with(&partial)
        }).map(|option| TuiCommandSuggestion::Value(TuiValueSuggestion {
            text: (*option).to_string(),
            hint: label.to_string(),
        })));
    }
    match source {
        ArgSource::ChatId => {
            for chat_id in &context.chat_ids {
                if partial.is_empty() || chat_id.to_ascii_lowercase().starts_with(&partial) {
                    candidates.push(TuiCommandSuggestion::Value(TuiValueSuggestion {
                        text: chat_id.clone(),
                        hint: "chat".to_string(),
                    }));
                }
            }
        }
        ArgSource::DeviceId => {
            for device_id in &context.device_ids {
                if partial.is_empty() || device_id.to_ascii_lowercase().starts_with(&partial) {
                    candidates.push(TuiCommandSuggestion::Value(TuiValueSuggestion {
                        text: device_id.clone(),
                        hint: "device".to_string(),
                    }));
                }
            }
        }
        ArgSource::FilePath => {
            for file_name in directory_candidates(&partial) {
                candidates.push(TuiCommandSuggestion::Value(TuiValueSuggestion {
                    text: file_name,
                    hint: "file".to_string(),
                }));
            }
        }
        ArgSource::None => {}
    }
    candidates
}

/// Builds candidates for one structured plugin argument.
fn plugin_arg_candidates(
    label: &str,
    options: &[String],
    required: bool,
    partial: &str,
    context: &ValueContext,
) -> Vec<TuiCommandSuggestion> {
    let _ = required;
    let _ = context;
    let partial = partial.to_ascii_lowercase();
    options
        .iter()
        .filter(|option| partial.is_empty() || option.to_ascii_lowercase().starts_with(&partial))
        .map(|option| {
            TuiCommandSuggestion::Value(TuiValueSuggestion {
                text: option.clone(),
                hint: label.to_string(),
            })
        })
        .collect()
}

/// Lists file names in the working directory matching the partial prefix.
fn directory_candidates(partial: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(".") else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .filter(|name| partial.is_empty() || name.to_ascii_lowercase().starts_with(&partial))
        .take(FILE_CANDIDATE_LIMIT)
        .collect();
    names.sort();
    names
}

/// Completes the command prefix using the selected suggestion usage.
pub(super) fn complete_command_input(
    input: &str,
    command: &TuiCommandSuggestion,
) -> (String, usize) {
    if let TuiCommandSuggestion::Value(value) = command {
        return complete_value_input(input, &value.text);
    }
    let command_text = command
        .usage()
        .split_whitespace()
        .take_while(|part| !part.starts_with('<') && !part.starts_with('['))
        .collect::<Vec<_>>()
        .join(" ");
    let lowercased = command_text.to_ascii_lowercase();
    let typed = format!(
        "/{}",
        input
            .strip_prefix('/')
            .unwrap_or(input)
            .trim()
            .to_ascii_lowercase()
    );
    // Option-style entries such as `new source` are appended after the
    // keywords and values already typed; everything else replaces the buffer.
    let completed =
        if typed == lowercased || lowercased.starts_with(&typed) {
            command_text
        } else {
            let keyword = command_text.rsplit(' ').next().unwrap_or_default();
            format!("{} {keyword}", input.trim_end())
        };
    let completed = format!("{completed} ");
    let cursor = completed.chars().count();
    (completed, cursor)
}

/// Completes one argument value candidate by replacing the trailing partial
/// token (or appending after a trailing space).
fn complete_value_input(input: &str, text: &str) -> (String, usize) {
    let mut completed = if input.ends_with(' ') {
        input.to_string()
    } else {
        match input.rsplit_once(' ') {
            Some((head, _)) => format!("{head} "),
            None => String::new(),
        }
    };
    completed.push_str(text);
    completed.push(' ');
    let cursor = completed.chars().count();
    (completed, cursor)
}

/// Returns the remaining parameter tokens of the command the buffer has
/// matched, for the muted ghost hint after the typed text. The spec whose
/// path is the longest prefix of the typed tokens wins; parameters already
/// consuming typed values drop out of the hint.
pub(super) fn ghost_hint(
    input: &str,
    plugin_commands: &[TuiPluginCommandSpec],
) -> Option<String> {
    if input.contains('\n') || !input.starts_with('/') {
        return None;
    }
    let stripped = input.strip_prefix('/').unwrap_or(input);
    let stripped = stripped.trim_start();
    if stripped.is_empty() {
        return None;
    }
    let tokens: Vec<String> = stripped
        .split_whitespace()
        .map(|token| token.to_ascii_lowercase())
        .collect();
    if tokens.is_empty() {
        return None;
    }
    let mut all = command_specs()
        .iter()
        .cloned()
        .map(TuiCommandSuggestion::Builtin)
        .collect::<Vec<_>>();
    all.extend(
        plugin_commands
            .iter()
            .cloned()
            .map(TuiCommandSuggestion::Plugin),
    );
    all.sort_by_key(|suggestion| {
        std::cmp::Reverse(suggestion.name().split_whitespace().count())
    });
    let trailing_space = input.ends_with(' ');
    for spec in &all {
        let name_words: Vec<&str> = spec.name().split_whitespace().collect();
        if name_words.len() > tokens.len()
            || !name_words.iter().zip(&tokens).all(|(word, typed)| word == typed)
        {
            continue;
        }
        // A partially typed last token means the path itself is still being
        // typed unless the path is shorter than the tokens (values present).
        if !trailing_space && name_words.len() == tokens.len() {
            let exact = name_words
                .iter()
                .zip(&tokens)
                .all(|(word, typed)| word == typed);
            let _ = exact;
        }
        // Parameters already consuming typed values drop out.
        let skip = tokens.len().max(name_words.len());
        let usage = spec.usage();
        let usage_tokens: Vec<&str> = usage.split_whitespace().collect();
        let remaining: Vec<&str> = usage_tokens
            .into_iter()
            .skip(skip)
            .filter(|token| is_param_token(token))
            .collect();
        if remaining.is_empty() {
            return None;
        }
        let mut hint = remaining.join(" ");
        if !input.ends_with(' ') {
            hint = format!(" {hint}");
        }
        return Some(hint);
    }
    None
}

fn active_command_prefix(input: &str) -> Option<String> {
    let stripped = input.strip_prefix('/')?;
    if stripped.contains('\n') {
        return None;
    }
    Some(stripped.trim_start().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin_names(input: &str) -> Vec<String> {
        matching_command_specs(input, &[])
            .into_iter()
            .map(|suggestion| suggestion.name().to_string())
            .collect()
    }

    fn suggestion_names(input: &str, plugins: &[TuiPluginCommandSpec]) -> Vec<String> {
        matching_command_specs(input, plugins)
            .into_iter()
            .map(|suggestion| suggestion.name().to_string())
            .collect()
    }

    fn value_texts(input: &str) -> Vec<String> {
        value_suggestions(input, &[], &ValueContext {
            chat_ids: vec!["chat-1".to_string(), "chat-2".to_string()],
            device_ids: vec!["core-1".to_string()],
        })
        .into_iter()
        .map(|suggestion| suggestion.name().to_string())
        .collect()
    }

    fn goal_plugin() -> Vec<TuiPluginCommandSpec> {
        expand_plugin_command(
            "goal".to_string(),
            "/goal [objective|edit <objective>|pause|resume|clear]".to_string(),
            "task goal".to_string(),
        )
    }

    #[test]
    fn plugin_enum_usage_expands_into_path_suggestions() {
        let plugins = goal_plugin();
        assert_eq!(
            suggestion_names("/goal ", &plugins),
            vec![
                "goal objective".to_string(),
                "goal edit".to_string(),
                "goal pause".to_string(),
                "goal resume".to_string(),
                "goal clear".to_string(),
            ]
        );
        assert_eq!(
            suggestion_names("/goal e", &plugins),
            vec!["goal edit".to_string()]
        );
        assert_eq!(
            suggestion_names("/go", &plugins),
            vec!["goal".to_string()]
        );
    }

    #[test]
    fn plugin_usages_outside_the_enum_convention_stay_single_entries() {
        let expand = |usage: &str| {
            expand_plugin_command("goal".to_string(), usage.to_string(), "d".to_string())
        };
        assert_eq!(expand("/goal <path>").len(), 1);
        assert_eq!(expand("/goal [edit <objective>]").len(), 1);
        assert_eq!(expand("/goal [a|<b>|c]").len(), 1);
        assert_eq!(expand("/goal [a]").len(), 1);
        assert_eq!(expand("/goalist [a|b]").len(), 1);
        assert_eq!(expand("/goal").len(), 1);

        let expanded = expand("/goal [a|b]");
        assert_eq!(expanded.len(), 3);
        assert_eq!(expanded[0].usage, "/goal");
        assert_eq!(expanded[1].name, "goal a");
        assert_eq!(expanded[2].usage, "/goal b");
    }

    #[test]
    fn builtin_usage_lines_are_generated_from_args() {
        let pair = command_specs()
            .iter()
            .find(|spec| spec.name == "network pair")
            .unwrap();
        assert_eq!(pair.usage(), "/network pair <address> <http|ws|tcp|serial|bluetooth>");
        let diff = command_specs()
            .iter()
            .find(|spec| spec.name == "diff")
            .unwrap();
        assert_eq!(diff.usage(), "/diff [file]");
        let new = command_specs()
            .iter()
            .find(|spec| spec.name == "new")
            .unwrap();
        assert_eq!(
            new.usage(),
            "/new [source <chat-id>] [input <json-object>]"
        );
    }

    #[test]
    fn top_level_prefix_lists_bare_commands_only() {
        assert_eq!(
            builtin_names("/ne"),
            vec!["new".to_string(), "network".to_string()]
        );
    }

    #[test]
    fn trailing_space_lists_subcommands_like_model() {
        let names = builtin_names("/model ");
        assert!(names.contains(&"model use".to_string()));
        assert!(!names.contains(&"model".to_string()));

        let names = builtin_names("/network ");
        assert!(names.contains(&"network show".to_string()));
        assert!(names.contains(&"network identity define".to_string()));
        assert!(!names.contains(&"network".to_string()));
    }

    #[test]
    fn deep_prefix_narrows_network_identity_subcommands() {
        assert_eq!(
            builtin_names("/network identity s"),
            vec!["network identity set".to_string()]
        );
    }

    #[test]
    fn new_options_complete_after_trailing_space() {
        assert_eq!(builtin_names("/new s"), vec!["new source".to_string()]);
        assert_eq!(
            builtin_names("/new source parent "),
            vec!["new input".to_string()]
        );
        assert_eq!(
            builtin_names("/new source parent input {} "),
            Vec::<String>::new()
        );
    }

    #[test]
    fn new_value_positions_stay_clear_of_suggestions() {
        assert_eq!(builtin_names("/new source "), Vec::<String>::new());
        assert_eq!(builtin_names("/new inp"), vec!["new input".to_string()]);
    }

    #[test]
    fn language_value_completes_case_insensitively() {
        assert_eq!(value_texts("/language ZH"), vec!["zh-CN".to_string()]);
        assert_eq!(value_texts("/language ").len(), 2);
        assert_eq!(builtin_names("/language ZH"), Vec::<String>::new());
    }

    #[test]
    fn network_listener_settings_commands_are_suggested() {
        assert_eq!(
            builtin_names("/network listen"),
            vec![
                "network listen".to_string(),
                "network listen show".to_string()
            ]
        );
        assert_eq!(
            builtin_names("/network endpoint"),
            vec!["network endpoint".to_string()]
        );
    }

    #[test]
    fn network_linking_subcommands_are_suggested() {
        assert_eq!(
            builtin_names("/network prompts"),
            vec!["network prompts".to_string()]
        );
        assert_eq!(
            builtin_names("/network requests"),
            vec!["network requests".to_string()]
        );
        assert_eq!(
            builtin_names("/network approve"),
            vec!["network approve".to_string()]
        );
        assert_eq!(
            builtin_names("/network reject"),
            vec!["network reject".to_string()]
        );
    }

    #[test]
    fn headless_panel_commands_are_suggested() {
        assert_eq!(builtin_names("/sta"), vec!["status".to_string()]);
        assert_eq!(builtin_names("/lo"), vec!["log".to_string()]);
        assert_eq!(
            builtin_names("/network na"),
            vec!["network name".to_string()]
        );
        assert_eq!(
            builtin_names("/network identity u"),
            vec!["network identity update".to_string()]
        );
        assert_eq!(
            builtin_names("/network identity del"),
            vec!["network identity delete".to_string()]
        );
        assert!(!builtin_names("/skill ").contains(&"skill toggle".to_string()));
    }

    #[test]
    fn network_linking_value_positions_stay_clear_of_suggestions() {
        assert_eq!(
            builtin_names("/network pro"),
            vec!["network prompts".to_string()]
        );
        // Request ids and assignment versions are free-form values, not path
        // segments, so completing past them must not suggest more commands.
        assert_eq!(
            builtin_names("/network approve req-1 "),
            Vec::<String>::new()
        );
        assert_eq!(
            builtin_names("/network reject req-1 3"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn transport_enum_completes_case_insensitively() {
        assert_eq!(
            value_texts("/network pair host WS"),
            vec!["ws".to_string()]
        );
        assert_eq!(
            value_texts("/network pair host ").len(),
            5
        );
    }

    #[test]
    fn chat_and_device_sources_feed_value_candidates() {
        assert_eq!(
            value_texts("/new source chat-"),
            vec!["chat-1".to_string(), "chat-2".to_string()]
        );
        assert_eq!(
            value_texts("/network admit core-"),
            vec!["core-1".to_string()]
        );
    }

    #[test]
    fn ghost_hint_shows_remaining_parameters() {
        assert_eq!(
            ghost_hint("/network pair ", &[]).as_deref(),
            Some("<address> <http|ws|tcp|serial|bluetooth>")
        );
        // A consumed first argument drops out of the hint.
        assert_eq!(
            ghost_hint("/network pair host ", &[]).as_deref(),
            Some("<http|ws|tcp|serial|bluetooth>")
        );
        assert_eq!(
            ghost_hint("/language ", &[]).as_deref(),
            Some("[en|zh-CN]")
        );
        assert_eq!(ghost_hint("/language en", &[]).as_deref(), None);
        assert_eq!(
            ghost_hint("/new ", &[]).as_deref(),
            Some("[source <chat-id>] [input <json-object>]")
        );
        assert_eq!(ghost_hint("plain text", &[]).as_deref(), None);
    }

    #[test]
    fn value_completion_replaces_the_partial_token() {
        let (completed, cursor) = complete_value_input("/language zh", "zh-CN");
        assert_eq!(completed, "/language zh-CN ");
        assert_eq!(cursor, completed.chars().count());
        let (appended, _) = complete_value_input("/network pair host ", "ws");
        assert_eq!(appended, "/network pair host ws ");
    }

    #[test]
    fn network_usage_lists_every_first_level_subcommand() {
        let usage = network_command_usage();
        assert!(usage.starts_with("network subcommands: <"));
        for expected in ["show", "identity", "pair", "listen", "leave"] {
            assert!(usage.contains(expected), "missing {expected} in {usage}");
        }
    }
}
