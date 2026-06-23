#[allow(non_snake_case)]
pub fn inferToolPkgIpcRuntimeFromContextKey(contextKey: &str) -> String {
    let lower = contextKey.trim().to_ascii_lowercase();
    if lower.starts_with("toolpkg_main:") {
        "main".to_string()
    } else if lower.starts_with("toolpkg_ui:") {
        "ui".to_string()
    } else if lower.starts_with("toolpkg_sandbox:") {
        "sandbox".to_string()
    } else if lower.starts_with("toolpkg_provider:") {
        "provider".to_string()
    } else {
        String::new()
    }
}

#[allow(non_snake_case)]
pub fn toolPkgIpcDispatchFunctionSource() -> String {
    r#"
        async function(params) {
            var dispatch = globalThis.__operitInvokeToolPkgIpcLocal;
            if (typeof dispatch !== 'function') {
                throw new Error('ToolPkg.ipc runtime is unavailable in target context');
            }
            var payloadJson =
                params && typeof params.__operit_toolpkg_ipc_payload_json === 'string'
                    ? params.__operit_toolpkg_ipc_payload_json
                    : 'null';
            var payload;
            try {
                payload = JSON.parse(payloadJson);
            } catch (error) {
                throw new Error(
                    'ToolPkg.ipc payload JSON is invalid: ' +
                    String(error && error.message ? error.message : error)
                );
            }
            var channel =
                params && typeof params.__operit_toolpkg_ipc_channel === 'string'
                    ? params.__operit_toolpkg_ipc_channel.trim()
                    : '';
            if (!channel) {
                throw new Error('ToolPkg.ipc channel is required');
            }
            var callerContextKey =
                params && typeof params.__operit_toolpkg_ipc_caller_context_key === 'string'
                    ? params.__operit_toolpkg_ipc_caller_context_key
                    : '';
            var currentContextKey =
                params && typeof params.__operit_execution_context_key === 'string'
                    ? params.__operit_execution_context_key
                    : '';
            var packageTarget =
                params && typeof params.__operit_ui_package_name === 'string'
                    ? params.__operit_ui_package_name
                    : '';
            var currentRuntime =
                params && typeof params.__operit_toolpkg_runtime_kind === 'string'
                    ? params.__operit_toolpkg_runtime_kind.trim()
                    : '';
            return await dispatch(channel, payload, {
                channel: channel,
                callerContextKey: callerContextKey,
                currentContextKey: currentContextKey,
                currentRuntime: currentRuntime,
                packageTarget: packageTarget
            });
        }
    "#
    .trim()
    .to_string()
}
