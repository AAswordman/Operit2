use super::JsEngineTests::newTestJsEngineState;
use crate::javascript::TestJsToolsHost::expect_js_output;
use std::collections::BTreeMap;

#[test]
fn plugin_config_proxy_persists_and_reads_values() {
    super::JsEngineTests::ensure_test_runtime_root();
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.plugin_config_roundtrip = async function(_params) {
            var files = Object.create(null);
            var writeCount = 0;
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
                    writeCount += 1;
                    files[String(path)] = String(content);
                    return { successful: true };
                }
            };

            var config = await PluginConfig.use('roundtrip', { count: 1, name: 'default' });
            config.count = 42;
            config.name = 'saved';
            var beforeFlushWriteCount = writeCount;
            await PluginConfig.flush(config);

            var loaded = await PluginConfig.use('roundtrip', { count: 0, name: 'missing' });
            return {
                count: loaded.count,
                name: loaded.name,
                beforeFlushWriteCount: beforeFlushWriteCount,
                writeCount: writeCount
            };
        };
    "#;
    let mut params = BTreeMap::new();
    params.insert(
        "__operit_package_lang".to_string(),
        serde_json::Value::String("zh-CN".to_string()),
    );

    let output = state.execute_script_function_on_current_thread(
        script,
        "plugin_config_roundtrip",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "plugin config roundtrip execution"),
        "{\"count\":42,\"name\":\"saved\",\"beforeFlushWriteCount\":0,\"writeCount\":1}"
    );
}
