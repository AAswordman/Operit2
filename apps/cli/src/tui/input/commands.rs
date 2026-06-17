#[derive(Clone, Copy, Debug)]
pub(super) struct TuiCommandSpec {
    pub(super) name: &'static str,
    pub(super) usage: &'static str,
    pub(super) description: &'static str,
}

const COMMAND_SPECS: [TuiCommandSpec; 25] = [
    TuiCommandSpec {
        name: "help",
        usage: "/help",
        description: "show help",
    },
    TuiCommandSpec {
        name: "new",
        usage: "/new [--character <name>] [--group-card <id>] [--group <name>]",
        description: "create chat",
    },
    TuiCommandSpec {
        name: "switch",
        usage: "/switch",
        description: "toggle chats",
    },
    TuiCommandSpec {
        name: "resume",
        usage: "/resume",
        description: "resume previous chat",
    },
    TuiCommandSpec {
        name: "max",
        usage: "/max",
        description: "toggle max context mode",
    },
    TuiCommandSpec {
        name: "model",
        usage: "/model",
        description: "show chat model binding",
    },
    TuiCommandSpec {
        name: "model current",
        usage: "/model current",
        description: "show chat model binding",
    },
    TuiCommandSpec {
        name: "model list",
        usage: "/model list",
        description: "list models",
    },
    TuiCommandSpec {
        name: "model choose",
        usage: "/model choose",
        description: "choose chat model",
    },
    TuiCommandSpec {
        name: "model use",
        usage: "/model use <provider-id> <model-id>",
        description: "switch chat model binding",
    },
    TuiCommandSpec {
        name: "approval",
        usage: "/approval",
        description: "show tool approval",
    },
    TuiCommandSpec {
        name: "approval allow",
        usage: "/approval allow",
        description: "allow all tools",
    },
    TuiCommandSpec {
        name: "approval ask",
        usage: "/approval ask",
        description: "ask before tools",
    },
    TuiCommandSpec {
        name: "approval forbid",
        usage: "/approval forbid",
        description: "deny all tools",
    },
    TuiCommandSpec {
        name: "approval tool",
        usage: "/approval tool <tool> <allow|ask|forbid|clear>",
        description: "set tool approval",
    },
    TuiCommandSpec {
        name: "attach",
        usage: "/attach <path>",
        description: "queue attachment",
    },
    TuiCommandSpec {
        name: "attachments",
        usage: "/attachments",
        description: "show queued attachments",
    },
    TuiCommandSpec {
        name: "clear-attachments",
        usage: "/clear-attachments",
        description: "clear queued attachments",
    },
    TuiCommandSpec {
        name: "queue",
        usage: "/queue",
        description: "show message queue",
    },
    TuiCommandSpec {
        name: "queue clear",
        usage: "/queue clear",
        description: "clear message queue",
    },
    TuiCommandSpec {
        name: "queue delete",
        usage: "/queue delete <id>",
        description: "delete queued message",
    },
    TuiCommandSpec {
        name: "queue edit",
        usage: "/queue edit <id>",
        description: "edit queued message",
    },
    TuiCommandSpec {
        name: "queue send",
        usage: "/queue send <id>",
        description: "send queued message",
    },
    TuiCommandSpec {
        name: "quit",
        usage: "/quit",
        description: "quit",
    },
    TuiCommandSpec {
        name: "exit",
        usage: "/exit",
        description: "quit",
    },
];

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
