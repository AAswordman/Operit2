use operit_js_bridge::javascript::JsLibraries::buildRuntimeBootstrapScript;

const CLEAN_ON_EXIT_ASSIGNMENT: &str =
    r#"var OPERIT_CLEAN_ON_EXIT_DIR = "/app/data/temp/clean_on_exit";"#;

/// Checks the sole production bootstrap without requiring a storage host or identity root.
#[test]
fn bootstrap_exposes_clean_on_exit_as_a_vfs_path() {
    assert!(buildRuntimeBootstrapScript().contains(CLEAN_ON_EXIT_ASSIGNMENT));
}
