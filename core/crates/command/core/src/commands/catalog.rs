//! Core-owned command declarations and generic root ownership resolution.

/// Identifies an inherent Core command without accepting unknown names as Core handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BuiltinRoot {
    Extension,
    Tool,
    Package,
    Plugin,
    Skill,
    Mcp,
    Market,
    Host,
    Log,
    LocalModels,
    Prefs,
    Approval,
    Model,
    Chat,
    Workspace,
    Storage,
    Stt,
    Update,
    Usage,
}

/// Declares only inherent Core commands; plugin roots come exclusively from registered metadata.
pub(super) const BUILTIN_ROOTS: &[(&str, BuiltinRoot)] = &[
    ("extension", BuiltinRoot::Extension),
    ("tool", BuiltinRoot::Tool),
    ("package", BuiltinRoot::Package),
    ("plugin", BuiltinRoot::Plugin),
    ("skill", BuiltinRoot::Skill),
    ("mcp", BuiltinRoot::Mcp),
    ("market", BuiltinRoot::Market),
    ("host", BuiltinRoot::Host),
    ("log", BuiltinRoot::Log),
    ("local-models", BuiltinRoot::LocalModels),
    ("prefs", BuiltinRoot::Prefs),
    ("approval", BuiltinRoot::Approval),
    ("model", BuiltinRoot::Model),
    ("chat", BuiltinRoot::Chat),
    ("workspace", BuiltinRoot::Workspace),
    ("storage", BuiltinRoot::Storage),
    ("stt", BuiltinRoot::Stt),
    ("update", BuiltinRoot::Update),
    ("usage", BuiltinRoot::Usage),
];

impl BuiltinRoot {
    /// Returns the canonical root spelling from the inherent Core directory.
    pub(super) fn name(self) -> &'static str {
        BUILTIN_ROOTS
            .iter()
            .find_map(|(name, root)| (*root == self).then_some(*name))
            .expect("every inherent Core command must have a directory entry")
    }

    /// Returns the original extension kind only for Core-owned scope aliases.
    pub(super) fn scope_alias_kind(self) -> Option<&'static str> {
        match self {
            Self::Plugin | Self::Package | Self::Skill | Self::Mcp => Some(self.name()),
            _ => None,
        }
    }
}

/// Carries exactly one selected builtin handler or immutable registered plugin handler token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RootCommandOwner<Plugin> {
    Core(BuiltinRoot),
    Plugin(Plugin),
}

/// Resolves both directory namespaces before any handler executes, preserving catalog failures.
pub(super) fn resolve_root<Plugin, Resolve>(
    command_name: &str,
    resolve_plugin: Resolve,
) -> Result<RootCommandOwner<Plugin>, String>
where
    Resolve: FnOnce(&str, &[&str]) -> Result<Option<Plugin>, String>,
{
    let name = command_name.trim();
    if name.is_empty()
        || name
            .chars()
            .any(|value| value == '/' || value.is_whitespace())
    {
        return Err(format!("invalid root command name: {command_name}"));
    }
    let builtin_names = BUILTIN_ROOTS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    let builtin = BUILTIN_ROOTS
        .iter()
        .find_map(|(builtin_name, root)| builtin_name.eq_ignore_ascii_case(name).then_some(*root));
    let plugin = resolve_plugin(name, &builtin_names)?;
    match (builtin, plugin) {
        (Some(root), None) => Ok(RootCommandOwner::Core(root)),
        (None, Some(command)) => Ok(RootCommandOwner::Plugin(command)),
        (Some(_), Some(_)) => Err(format!(
            "plugin command /{name} collides with builtin Core command: {name}"
        )),
        (None, None) => Err(format!(
            "unknown root command: {name}; use 'operit2 plugin commands' to discover registered commands"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Builds dynamic registration fixtures with explicit enabled states for resolver seam tests.
    fn resolve_fixture(
        command_name: &str,
        registrations: &[(&str, &str, bool)],
        resolutions: &Cell<usize>,
    ) -> Result<RootCommandOwner<String>, String> {
        resolve_root(command_name, |name, builtin_names| {
            resolutions.set(resolutions.get() + 1);
            let candidates = registrations
                .iter()
                .filter(|(registered, _, _)| registered.eq_ignore_ascii_case(name))
                .collect::<Vec<_>>();
            if !candidates.is_empty()
                && builtin_names
                    .iter()
                    .any(|builtin| builtin.eq_ignore_ascii_case(name))
            {
                return Err(format!(
                    "plugin command /{name} collides with builtin Core command: {name}"
                ));
            }
            match candidates.as_slice() {
                [] => Ok(None),
                [(_, owner, false)] => {
                    Err(format!("plugin command is disabled: /{name} ({owner})"))
                }
                [(_, owner, true)] => Ok(Some((*owner).to_string())),
                _ => Err(format!("duplicate registered plugin command /{name}")),
            }
        })
    }

    /// Verifies arbitrary newly registered roots become usable without a Core routing match.
    #[test]
    fn arbitrary_registered_roots_resolve_once() {
        for name in ["new-root-728", "plan", "goal", "custom.automation"] {
            let calls = Cell::new(0);
            let registration = [(name, "fixture.plugin", true)];
            assert_eq!(
                resolve_fixture(name, &registration, &calls),
                Ok(RootCommandOwner::Plugin("fixture.plugin".to_string()))
            );
            assert_eq!(calls.get(), 1);
        }
    }

    /// Verifies exact case-insensitive root lookup does not classify similarly spelled unknown names.
    #[test]
    fn registered_lookup_is_exact_and_case_insensitive() {
        let registrations = [("new-root-728", "fixture.plugin", true)];
        assert_eq!(
            resolve_fixture("NEW-ROOT-728", &registrations, &Cell::new(0)),
            Ok(RootCommandOwner::Plugin("fixture.plugin".to_string()))
        );
        assert!(resolve_fixture("new-root-728-extra", &registrations, &Cell::new(0)).is_err());
    }

    /// Verifies disabled registrations reject routing before a provider can execute.
    #[test]
    fn disabled_registration_is_not_a_core_command() {
        let calls = Cell::new(0);
        assert_eq!(
            resolve_fixture(
                "new-root-728",
                &[("new-root-728", "fixture.plugin", false)],
                &calls
            ),
            Err("plugin command is disabled: /new-root-728 (fixture.plugin)".to_string())
        );
        assert_eq!(calls.get(), 1);
    }

    /// Verifies duplicate registrations preserve their catalog error instead of selecting an owner.
    #[test]
    fn duplicate_registration_is_an_error() {
        assert_eq!(
            resolve_fixture(
                "new-root-728",
                &[
                    ("new-root-728", "first.plugin", true),
                    ("NEW-ROOT-728", "second.plugin", true)
                ],
                &Cell::new(0),
            ),
            Err("duplicate registered plugin command /new-root-728".to_string())
        );
    }

    /// Verifies builtin and plugin names cannot silently override one another.
    #[test]
    fn builtin_plugin_collision_is_an_error() {
        assert_eq!(
            resolve_fixture("chat", &[("chat", "fixture.plugin", true)], &Cell::new(0)),
            Err("plugin command /chat collides with builtin Core command: chat".to_string())
        );
    }

    /// Verifies the generic seam rejects ambiguous ownership even when a resolver omits validation.
    #[test]
    fn ambiguous_owner_is_never_executed() {
        let selected = resolve_root("chat", |_, _| Ok(Some("fixture.plugin")));
        assert_eq!(
            selected,
            Err("plugin command /chat collides with builtin Core command: chat".to_string())
        );
    }

    /// Verifies every unrelated builtin retains its original inherent handler identity.
    #[test]
    fn inherent_core_commands_keep_their_handlers() {
        for (name, expected) in BUILTIN_ROOTS {
            let calls = Cell::new(0);
            assert_eq!(
                resolve_fixture(name, &[], &calls),
                Ok(RootCommandOwner::Core(*expected))
            );
            assert_eq!(calls.get(), 1);
        }
    }

    /// Verifies an absent name is a routing error, not a guessed builtin handler.
    #[test]
    fn unknown_root_is_an_error() {
        assert_eq!(
            resolve_fixture("unregistered-root-391", &[], &Cell::new(0)),
            Err("unknown root command: unregistered-root-391; use 'operit2 plugin commands' to discover registered commands".to_string())
        );
    }

    /// Verifies empty and invalid root grammar cannot reach a catalog or handler.
    #[test]
    fn invalid_names_do_not_resolve() {
        for name in ["", "  ", "/root", "root with spaces"] {
            let calls = Cell::new(0);
            assert!(resolve_fixture(name, &[], &calls).is_err());
            assert_eq!(calls.get(), 0);
        }
    }

    /// Verifies startup, registry, and storage errors stop routing without a second lookup.
    #[test]
    fn catalog_errors_are_preserved_once() {
        let calls = Cell::new(0);
        let result = resolve_root::<String, _>("custom-root", |_, _| {
            calls.set(calls.get() + 1);
            Err("registry load failed: source error".to_string())
        });
        assert_eq!(
            result,
            Err("registry load failed: source error".to_string())
        );
        assert_eq!(calls.get(), 1);
    }

    /// Verifies scope aliases remain limited to their original inherent Core extension families.
    #[test]
    fn scope_aliases_preserve_original_kinds() {
        for root in [
            BuiltinRoot::Plugin,
            BuiltinRoot::Package,
            BuiltinRoot::Skill,
            BuiltinRoot::Mcp,
        ] {
            assert_eq!(root.scope_alias_kind(), Some(root.name()));
        }
        assert_eq!(BuiltinRoot::Chat.scope_alias_kind(), None);
        assert_eq!(BuiltinRoot::Extension.scope_alias_kind(), None);
    }
}
