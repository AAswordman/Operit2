use std::future::Future;

use crate::output::CoreCommandOutput;
use operit_tools::tools::packTool::RuntimePackageManager::{
    ResolvedToolPkgCoreCommand, RuntimePackageManager, ToolPkgCoreCommandCatalog,
    ToolPkgCoreCommandExecutionResult, ToolPkgCoreCommandInfo,
};
use operit_tools::tools::AIToolHandler::AIToolHandler;

use super::catalog::{resolve_root, RootCommandOwner, BUILTIN_ROOTS};

/// Owns one completed registry snapshot for both owner resolution and provider execution.
pub(super) struct ReadyCommandRegistry {
    manager: RuntimePackageManager,
    catalog: ToolPkgCoreCommandCatalog,
}

impl ReadyCommandRegistry {
    /// Waits for cold-start loading before capturing the sole manager and command directory snapshot.
    pub(super) async fn load(tool_handler: &AIToolHandler) -> Result<Self, String> {
        let manager =
            RuntimePackageManager::readySnapshot(tool_handler.getOrCreatePackageManager()).await?;
        let catalog = manager.getToolPkgCoreCommandCatalog(false)?;
        Ok(Self { manager, catalog })
    }

    /// Selects one inherent Core handler or immutable registered handler from this exact snapshot.
    pub(super) fn resolve(
        &self,
        command_name: &str,
    ) -> Result<RootCommandOwner<ResolvedToolPkgCoreCommand>, String> {
        resolve_root(command_name, |name, builtin_names| {
            self.catalog.resolve(name, builtin_names)
        })
    }

    /// Lists enabled registered roots and checks their ownership against the same builtin directory.
    pub(super) fn plugin_commands(&self) -> Result<Vec<ToolPkgCoreCommandInfo>, String> {
        let builtin_names = BUILTIN_ROOTS
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>();
        let mut commands = Vec::new();
        for registration in self.catalog.registrations() {
            if !registration.enabled {
                continue;
            }
            let selected = self
                .catalog
                .resolve(&registration.command.name, &builtin_names)?
                .ok_or_else(|| {
                    format!(
                        "registered command disappeared from its immutable catalog: {}",
                        registration.command.name
                    )
                })?;
            commands.push(selected.info().clone());
        }
        Ok(commands)
    }

    /// Executes only the selected immutable handler token on the manager that supplied its catalog.
    pub(super) async fn execute(
        &self,
        selected: &ResolvedToolPkgCoreCommand,
        args: &[String],
        output: &mut CoreCommandOutput,
    ) -> Result<(), String> {
        execute_with_provider(
            selected,
            &selected.info().name,
            args,
            output,
            |handler, args, json_mode| {
                self.manager
                    .executeResolvedToolPkgCoreCommand(handler, args, json_mode)
            },
        )
        .await
    }
}

/// Executes an explicitly addressed plugin root through the already completed invocation snapshot.
pub(super) async fn run_delegated_command(
    registry: &ReadyCommandRegistry,
    command_name: &str,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match registry.resolve(command_name)? {
        RootCommandOwner::Plugin(selected) => registry.execute(&selected, args, output).await,
        RootCommandOwner::Core(root) => Err(format!(
            "command /{} is owned by Core; plugin exec requires a registered plugin command",
            root.name()
        )),
    }
}

/// Invokes exactly one selected provider and preserves its error and explicit output contract.
async fn execute_with_provider<'a, Handler, Provider, Execution>(
    selected: &'a Handler,
    command_name: &'a str,
    args: &'a [String],
    output: &mut CoreCommandOutput,
    provider: Provider,
) -> Result<(), String>
where
    Provider: FnOnce(&'a Handler, &'a [String], bool) -> Execution,
    Execution: Future<Output = Result<ToolPkgCoreCommandExecutionResult, String>>,
{
    let result = provider(selected, args, output.isJsonMode()).await?;
    if output.isJsonMode() {
        let json = result.json.ok_or_else(|| {
            format!("plugin command /{command_name} did not return a JSON result")
        })?;
        output.setJsonStdout(json);
    } else {
        output.push_stdout(result.stdout);
        output.push_stderr(result.stderr);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::future::ready;

    /// Runs an isolated provider future without initializing application storage or plugins.
    fn run<Execution: Future>(execution: Execution) -> Execution::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("command test runtime must initialize")
            .block_on(execution)
    }

    /// Verifies the immutable selected token, arguments, and JSON mode reach exactly one provider call.
    #[test]
    fn selected_handler_executes_once_with_exact_arguments() {
        for json_mode in [false, true] {
            let selected = ("arbitrary.plugin", "handler-728");
            let args = vec![
                "show".to_string(),
                "record with spaces".to_string(),
                String::new(),
            ];
            let calls = Cell::new(0);
            let mut output = CoreCommandOutput::new();
            output.setJsonMode(json_mode);
            run(execute_with_provider(
                &selected,
                "new-root-728",
                &args,
                &mut output,
                |handler, actual_args, actual_json| {
                    calls.set(calls.get() + 1);
                    assert!(std::ptr::eq(handler, &selected));
                    assert_eq!(actual_args, args.as_slice());
                    assert_eq!(actual_json, json_mode);
                    ready(Ok(ToolPkgCoreCommandExecutionResult {
                        stdout: "provider output\n".to_string(),
                        stderr: "provider diagnostic\n".to_string(),
                        json: Some(serde_json::json!({"owner": handler.0, "handler": handler.1})),
                    }))
                },
            ))
            .expect("the selected provider must execute");
            assert_eq!(calls.get(), 1);
            if json_mode {
                output
                    .finalizeJson()
                    .expect("structured result must finalize");
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&output.stdout).unwrap(),
                    serde_json::json!({"owner": selected.0, "handler": selected.1})
                );
                assert_eq!(output.stderr, "");
            } else {
                assert_eq!(output.stdout, "provider output\n");
                assert_eq!(output.stderr, "provider diagnostic\n");
            }
        }
    }

    /// Verifies the selected handler owns its own empty-argument usage without a second lookup.
    #[test]
    fn empty_subcommands_reach_selected_handler() {
        let selected = "selected.handler";
        let calls = Cell::new(0);
        let mut output = CoreCommandOutput::new();
        run(execute_with_provider(
            &selected,
            "custom-root",
            &[],
            &mut output,
            |handler, args, json_mode| {
                calls.set(calls.get() + 1);
                assert_eq!(*handler, selected);
                assert!(args.is_empty());
                assert!(!json_mode);
                ready(Ok(ToolPkgCoreCommandExecutionResult {
                    stdout: "registered usage\n".to_string(),
                    stderr: String::new(),
                    json: None,
                }))
            },
        ))
        .expect("the selected handler owns its help");
        assert_eq!(calls.get(), 1);
        assert_eq!(output.stdout, "registered usage\n");
    }

    /// Verifies original provider failures terminate execution without another provider or lookup.
    #[test]
    fn provider_errors_are_propagated_once() {
        for json_mode in [false, true] {
            let calls = Cell::new(0);
            let mut output = CoreCommandOutput::new();
            output.setJsonMode(json_mode);
            let result = run(execute_with_provider(
                &"handler-728",
                "custom-root",
                &[],
                &mut output,
                |_, _, mode| {
                    calls.set(calls.get() + 1);
                    assert_eq!(mode, json_mode);
                    ready(Err("original provider storage error".to_string()))
                },
            ));
            assert_eq!(result, Err("original provider storage error".to_string()));
            assert_eq!(calls.get(), 1);
            assert_eq!(output.stdout, "");
            assert_eq!(output.stderr, "");
        }
    }

    /// Verifies JSON mode cannot synthesize a structured result from provider stdout.
    #[test]
    fn json_requires_the_selected_provider_document() {
        let mut output = CoreCommandOutput::new();
        output.setJsonMode(true);
        let result = run(execute_with_provider(
            &"handler-728",
            "custom-root",
            &[],
            &mut output,
            |_, _, _| {
                ready(Ok(ToolPkgCoreCommandExecutionResult {
                    stdout: "{\"ok\":true}".to_string(),
                    stderr: "provider diagnostic".to_string(),
                    json: None,
                }))
            },
        ));
        assert_eq!(
            result,
            Err("plugin command /custom-root did not return a JSON result".to_string())
        );
        assert_eq!(output.stdout, "");
        assert_eq!(output.stderr, "");
    }

    /// Verifies structured application errors remain the selected provider's unchanged JSON document.
    #[test]
    fn structured_provider_error_documents_are_preserved() {
        let expected = serde_json::json!({"ok": false, "error": {"code": "record_not_found", "message": "exact error"}});
        let mut output = CoreCommandOutput::new();
        output.setJsonMode(true);
        run(execute_with_provider(
            &"handler-728",
            "custom-root",
            &[],
            &mut output,
            |_, _, json_mode| {
                assert!(json_mode);
                ready(Ok(ToolPkgCoreCommandExecutionResult {
                    stdout: "provider text must not replace JSON".to_string(),
                    stderr: "exact error".to_string(),
                    json: Some(expected.clone()),
                }))
            },
        ))
        .expect("the structured error is a provider result");
        output.finalizeJson().expect("error document must finalize");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&output.stdout).unwrap(),
            expected
        );
        assert_eq!(output.stderr, "");
    }
}
