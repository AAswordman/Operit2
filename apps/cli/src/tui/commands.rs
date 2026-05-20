#[derive(Clone, Copy, Debug)]
pub(super) struct TuiCommandSpec {
    pub(super) name: &'static str,
    pub(super) usage: &'static str,
    pub(super) description: &'static str,
}

const COMMAND_SPECS: [TuiCommandSpec; 8] = [
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
        .filter(|spec| spec.name.starts_with(prefix))
        .collect()
}

pub(super) fn complete_command_input(input: &str, command_name: &str) -> (String, usize) {
    let chars = input.chars().collect::<Vec<_>>();
    let token_end = chars
        .iter()
        .position(|ch| ch.is_whitespace())
        .unwrap_or(chars.len());
    let rest = chars[token_end..].iter().collect::<String>();
    let completed = format!("/{command_name} ");
    let cursor = completed.chars().count();
    (format!("{completed}{}", rest.trim_start()), cursor)
}

fn active_command_prefix(input: &str) -> Option<&str> {
    let stripped = input.strip_prefix('/')?;
    if stripped.chars().any(|ch| ch.is_whitespace()) {
        return None;
    }
    Some(stripped)
}
