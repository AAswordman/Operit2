use super::i18n::{TuiLanguage, TuiTextKey};

#[derive(Clone, Copy, Debug)]
pub(super) struct TuiCommandSpec {
    pub(super) name: &'static str,
    pub(super) usage: &'static str,
    pub(super) description_key: TuiTextKey,
    /// Keyword-value options completing in any order after the command
    /// (e.g. `new character <name> group <name>`). Empty for path-style
    /// commands; only the entry whose `name` equals the command word
    /// declares them.
    pub(super) options: &'static [&'static str],
}

/// Describes one plugin-provided slash command returned by Core.
#[derive(Clone, Debug)]
pub(super) struct TuiPluginCommandSpec {
    pub(super) name: String,
    pub(super) usage: String,
    pub(super) description: String,
}

/// Represents either a built-in or runtime-provided command suggestion.
#[derive(Clone, Debug)]
pub(super) enum TuiCommandSuggestion {
    Builtin(TuiCommandSpec),
    Plugin(TuiPluginCommandSpec),
}

impl TuiCommandSuggestion {
    /// Returns the command name without its leading slash.
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Builtin(spec) => spec.name,
            Self::Plugin(spec) => &spec.name,
        }
    }

    /// Returns the command usage shown in the completion popup.
    pub(super) fn usage(&self) -> &str {
        match self {
            Self::Builtin(spec) => spec.usage,
            Self::Plugin(spec) => &spec.usage,
        }
    }

    /// Returns the localized or plugin-provided command description.
    pub(super) fn description(&self, language: TuiLanguage) -> String {
        match self {
            Self::Builtin(spec) => spec.description(language).to_string(),
            Self::Plugin(spec) => spec.description.clone(),
        }
    }
}

const COMMAND_SPECS: [TuiCommandSpec; 75] = [
    TuiCommandSpec {
        name: "help",
        usage: "/help",
        description_key: TuiTextKey::CommandHelpDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "new",
        usage: "/new [character <name>] [group-card <id>] [group <name>]",
        description_key: TuiTextKey::CommandNewDescription,
        options: &["character", "group-card", "group"],
    },
    TuiCommandSpec {
        name: "new character",
        usage: "/new character <name>",
        description_key: TuiTextKey::CommandNewCharacterDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "new group-card",
        usage: "/new group-card <id>",
        description_key: TuiTextKey::CommandNewGroupCardDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "new group",
        usage: "/new group <name>",
        description_key: TuiTextKey::CommandNewGroupDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "switch",
        usage: "/switch",
        description_key: TuiTextKey::CommandSwitchDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "resume",
        usage: "/resume",
        description_key: TuiTextKey::CommandResumeDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "language",
        usage: "/language [en|zh-CN]",
        description_key: TuiTextKey::CommandLanguageDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "language en",
        usage: "/language en",
        description_key: TuiTextKey::CommandLanguageEnDescription,
        options: &[],
    },
    TuiCommandSpec {
        // Lowercase so the lowercased input prefix matches; display keeps the
        // canonical "zh-CN" spelling in `usage`.
        name: "language zh-cn",
        usage: "/language zh-CN",
        description_key: TuiTextKey::CommandLanguageZhDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network",
        usage: "/network",
        description_key: TuiTextKey::CommandNetworkDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network show",
        usage: "/network show",
        description_key: TuiTextKey::CommandNetworkShowDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network bootstrap",
        usage: "/network bootstrap",
        description_key: TuiTextKey::CommandNetworkBootstrapDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network audit",
        usage: "/network audit",
        description_key: TuiTextKey::CommandNetworkAuditDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network doctor",
        usage: "/network doctor",
        description_key: TuiTextKey::CommandNetworkDoctorDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network devices",
        usage: "/network devices",
        description_key: TuiTextKey::CommandNetworkDevicesDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network identities",
        usage: "/network identities",
        description_key: TuiTextKey::CommandNetworkIdentitiesDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity list",
        usage: "/network identity list",
        description_key: TuiTextKey::CommandNetworkIdentityListDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity define",
        usage: "/network identity define <name> <capability>...",
        description_key: TuiTextKey::CommandNetworkIdentityDefineDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity set",
        usage: "/network identity set <device> <identity>",
        description_key: TuiTextKey::CommandNetworkIdentitySetDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network identity clear",
        usage: "/network identity clear <device>",
        description_key: TuiTextKey::CommandNetworkIdentityClearDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network admit",
        usage: "/network admit <device>",
        description_key: TuiTextKey::CommandNetworkAdmitDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network remove",
        usage: "/network remove <device>",
        description_key: TuiTextKey::CommandNetworkRemoveDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network disconnect",
        usage: "/network disconnect <device>",
        description_key: TuiTextKey::CommandNetworkDisconnectDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network policy list",
        usage: "/network policy list",
        description_key: TuiTextKey::CommandNetworkPolicyListDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network policy set",
        usage: "/network policy set <key> <value>",
        description_key: TuiTextKey::CommandNetworkPolicySetDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network token",
        usage: "/network token",
        description_key: TuiTextKey::CommandNetworkTokenDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network prompts",
        usage: "/network prompts",
        description_key: TuiTextKey::CommandNetworkPromptsDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network requests",
        usage: "/network requests",
        description_key: TuiTextKey::CommandNetworkRequestsDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network approve",
        usage: "/network approve <device|request-id> <assignment-version>",
        description_key: TuiTextKey::CommandNetworkApproveDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network reject",
        usage: "/network reject <device|request-id> <assignment-version>",
        description_key: TuiTextKey::CommandNetworkRejectDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network discover",
        usage: "/network discover",
        description_key: TuiTextKey::CommandNetworkDiscoverDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network pair",
        usage: "/network pair <address> <http|ws|tcp|serial|bluetooth> [--token <token>]",
        description_key: TuiTextKey::CommandNetworkPairDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network pair-confirm",
        usage: "/network pair-confirm [pairing-id] <code>",
        description_key: TuiTextKey::CommandNetworkPairConfirmDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network pair-cancel",
        usage: "/network pair-cancel [pairing-id]",
        description_key: TuiTextKey::CommandNetworkPairCancelDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network peers",
        usage: "/network peers",
        description_key: TuiTextKey::CommandNetworkPeersDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network unpair",
        usage: "/network unpair <node-id|name>",
        description_key: TuiTextKey::CommandNetworkUnpairDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network join",
        usage: "/network join <node-id|name>",
        description_key: TuiTextKey::CommandNetworkJoinDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network joins",
        usage: "/network joins",
        description_key: TuiTextKey::CommandNetworkJoinsDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network join-cancel",
        usage: "/network join-cancel <request-id|node-id>",
        description_key: TuiTextKey::CommandNetworkJoinCancelDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "network leave",
        usage: "/network leave",
        description_key: TuiTextKey::CommandNetworkLeaveDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "model",
        usage: "/model",
        description_key: TuiTextKey::CommandModelDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "model current",
        usage: "/model current",
        description_key: TuiTextKey::CommandModelDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "model list",
        usage: "/model list",
        description_key: TuiTextKey::CommandModelListDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "model choose",
        usage: "/model choose",
        description_key: TuiTextKey::CommandModelChooseDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "model use",
        usage: "/model use <provider-id> <model-id>",
        description_key: TuiTextKey::CommandModelUseDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "model config",
        usage: "/model config",
        description_key: TuiTextKey::CommandModelConfigDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "approval",
        usage: "/approval",
        description_key: TuiTextKey::CommandApprovalDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "approval allow",
        usage: "/approval allow",
        description_key: TuiTextKey::CommandApprovalAllowDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "approval ask",
        usage: "/approval ask",
        description_key: TuiTextKey::CommandApprovalAskDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "approval forbid",
        usage: "/approval forbid",
        description_key: TuiTextKey::CommandApprovalForbidDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "attach",
        usage: "/attach <path>",
        description_key: TuiTextKey::CommandAttachDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "attachments",
        usage: "/attachments",
        description_key: TuiTextKey::CommandAttachmentsDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "clear-attachments",
        usage: "/clear-attachments",
        description_key: TuiTextKey::CommandClearAttachmentsDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "queue",
        usage: "/queue",
        description_key: TuiTextKey::CommandQueueDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "queue clear",
        usage: "/queue clear",
        description_key: TuiTextKey::CommandQueueClearDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "queue delete",
        usage: "/queue delete <id>",
        description_key: TuiTextKey::CommandQueueDeleteDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "queue edit",
        usage: "/queue edit <id>",
        description_key: TuiTextKey::CommandQueueEditDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "queue send",
        usage: "/queue send <id>",
        description_key: TuiTextKey::CommandQueueSendDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "quit",
        usage: "/quit",
        description_key: TuiTextKey::CommandQuitDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "exit",
        usage: "/exit",
        description_key: TuiTextKey::CommandQuitDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "character",
        usage: "/character",
        description_key: TuiTextKey::CommandCharacterDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "character choose",
        usage: "/character choose",
        description_key: TuiTextKey::CommandCharacterChooseDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "group",
        usage: "/group",
        description_key: TuiTextKey::CommandGroupDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "group choose",
        usage: "/group choose",
        description_key: TuiTextKey::CommandGroupChooseDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "skill",
        usage: "/skill",
        description_key: TuiTextKey::CommandSkillDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "skill toggle",
        usage: "/skill toggle <name>",
        description_key: TuiTextKey::CommandSkillToggleDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "package",
        usage: "/package",
        description_key: TuiTextKey::CommandPackageDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "package toggle",
        usage: "/package toggle <name>",
        description_key: TuiTextKey::CommandPackageToggleDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "plugin",
        usage: "/plugin",
        description_key: TuiTextKey::CommandPluginDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "plugin toggle",
        usage: "/plugin toggle <name>",
        description_key: TuiTextKey::CommandPluginToggleDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "mcp",
        usage: "/mcp",
        description_key: TuiTextKey::CommandMcpDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "mcp toggle",
        usage: "/mcp toggle <name>",
        description_key: TuiTextKey::CommandMcpToggleDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "tag",
        usage: "/tag",
        description_key: TuiTextKey::CommandTagDescription,
        options: &[],
    },
    TuiCommandSpec {
        name: "update",
        usage: "/update",
        description_key: TuiTextKey::CommandUpdateDescription,
        options: &[],
    },
];

impl TuiCommandSpec {
    pub(super) fn description(self, language: TuiLanguage) -> &'static str {
        language.text().raw(self.description_key)
    }
}

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
    }];
    for alternative in alternatives {
        let literal = alternative.split_whitespace().next().unwrap_or(alternative);
        expanded.push(TuiPluginCommandSpec {
            name: format!("{name} {literal}"),
            usage: format!("/{name} {alternative}"),
            description: description.clone(),
        });
    }
    expanded
}

pub(super) fn command_specs() -> &'static [TuiCommandSpec] {
    &COMMAND_SPECS
}

/// Returns the keyword-value options declared for a command word, if any.
/// Shared with the command parser so completion and dispatch stay in sync.
pub(super) fn keyword_options_for(command: &str) -> Option<&'static [&'static str]> {
    command_specs()
        .iter()
        .find(|spec| spec.name == command && !spec.options.is_empty())
        .map(|spec| spec.options)
}

/// Keyword-value options combine freely and each takes a free-form value, so
/// plain prefix matching cannot suggest the remaining options once a value is
/// typed. Match by the keywords the input already consumes instead.
fn match_keyword_options_prefix(
    prefix: &str,
    command: &str,
    options: &[&str],
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
    let trailing_space = stripped.ends_with(' ');
    let tokens: Vec<&str> = stripped.split_whitespace().collect();
    if trailing_space {
        // A bare keyword is waiting for its value; a completed value means
        // the remaining keywords are up next.
        !tokens.last().is_some_and(|last| options.contains(last)) && !tokens.contains(&keyword)
    } else if let Some(partial) = tokens.last() {
        keyword.starts_with(partial) && !tokens.contains(&keyword)
    } else {
        !tokens.contains(&keyword)
    }
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
        .copied()
        .map(TuiCommandSuggestion::Builtin)
        .collect::<Vec<_>>();
    suggestions.extend(
        plugin_commands
            .iter()
            .cloned()
            .map(TuiCommandSuggestion::Plugin),
    );
    suggestions
        .into_iter()
        .filter(|spec| {
            let command = prefix.split_whitespace().next().unwrap_or(prefix.as_str());
            if let Some(options) = keyword_options_for(command) {
                return match_keyword_options_prefix(&prefix, command, options, spec);
            }
            if prefix.is_empty() {
                return !spec.name().contains(' ');
            }
            if prefix.chars().any(|ch| ch.is_whitespace()) {
                return spec.name().starts_with(prefix.as_str());
            }
            spec.name()
                .split_whitespace()
                .next()
                .map(|name| name.starts_with(prefix.as_str()))
                .unwrap_or(false)
                && !spec.name().contains(' ')
        })
        .collect()
}

/// Completes the command prefix using the selected suggestion usage.
pub(super) fn complete_command_input(
    input: &str,
    command: &TuiCommandSuggestion,
) -> (String, usize) {
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
    // Option-style entries such as `new group-card` are appended after the
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
        assert_eq!(suggestion_names("/go", &plugins), vec!["goal".to_string()]);
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
        assert_eq!(
            builtin_names("/new g"),
            vec!["new group-card".to_string(), "new group".to_string()]
        );
        assert_eq!(
            builtin_names("/new character Alice "),
            vec!["new group-card".to_string(), "new group".to_string()]
        );
        assert_eq!(
            builtin_names("/new character Alice group-card 5 "),
            vec!["new group".to_string()]
        );
        assert_eq!(
            builtin_names("/new character Alice group-card 5 group Heroes "),
            Vec::<String>::new()
        );
    }

    #[test]
    fn new_value_positions_stay_clear_of_suggestions() {
        assert_eq!(builtin_names("/new character "), Vec::<String>::new());
        assert_eq!(builtin_names("/new cha"), vec!["new character".to_string()]);
    }

    #[test]
    fn language_value_matches_case_insensitively() {
        assert_eq!(
            builtin_names("/language ZH"),
            vec!["language zh-cn".to_string()]
        );
    }

    #[test]
    fn network_linking_subcommands_are_suggested() {
        assert_eq!(
            builtin_names("/network token"),
            vec!["network token".to_string()]
        );
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
}
