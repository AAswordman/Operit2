#[derive(Clone, Copy, Debug)]
pub(super) struct TuiCommandSpec {
    pub(super) name: &'static str,
    pub(super) usage: &'static str,
    pub(super) description: &'static str,
}

const COMMAND_SPECS: [TuiCommandSpec; 13] = [
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
        description: "list model configs",
    },
    TuiCommandSpec {
        name: "model use",
        usage: "/model use <config-id> [model-index]",
        description: "switch chat model binding",
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

pub(super) fn complete_command_input(_input: &str, command_name: &str) -> (String, usize) {
    let completed = format!("/{command_name} ");
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
