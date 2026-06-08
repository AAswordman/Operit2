use super::JsEngineState;
use std::collections::BTreeMap;

#[test]
fn plugin_config_proxy_persists_and_reads_values() {
    let mut state = JsEngineState::new(None);
    let script = r#"
        exports.plugin_config_roundtrip = async function(_params) {
            var files = Object.create(null);
            globalThis.__operit_call_runtime_ref.getPluginConfigDir = function() {
                return '/plugin-config-test';
            };
            Tools.Files = {
                exists: async function(path) {
                    return {
                        exists: Object.prototype.hasOwnProperty.call(files, String(path))
                    };
                },
                read: async function(path) {
                    return {
                        content: files[String(path)] || ''
                    };
                },
                mkdir: async function(_path, _recursive) {
                    return { successful: true };
                },
                write: async function(path, content, _append) {
                    files[String(path)] = String(content);
                    return { successful: true };
                }
            };

            var config = await PluginConfig.use('roundtrip', { count: 1, name: 'default' });
            config.count = 42;
            config.name = 'saved';
            await Promise.resolve();
            await Promise.resolve();

            var loaded = await PluginConfig.use('roundtrip', { count: 0, name: 'missing' });
            return {
                count: loaded.count,
                name: loaded.name
            };
        };
    "#;
    let mut params = BTreeMap::new();
    params.insert(
        "__operit_package_lang".to_string(),
        serde_json::Value::String("zh-CN".to_string()),
    );

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "plugin_config_roundtrip",
        &params,
        None,
    );

    assert_eq!(output.as_deref(), Some("{\"count\":42,\"name\":\"saved\"}"));
}
