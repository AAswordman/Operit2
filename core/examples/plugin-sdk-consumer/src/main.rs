use std::collections::BTreeMap;
use std::sync::Arc;

use operit_plugin_sdk::execution_result::JsExecutionResult;
use operit_plugin_sdk::javascript::{JsExecutionEngine, ToolPkgMainRegistrationCapture};
use operit_plugin_sdk::package::ToolPackage;
use operit_plugin_sdk::toolpkg::ToolPkgManager::{
    ToolPkgAssetSource, ToolPkgExecutionEngineFactory,
};
use operit_plugin_sdk::JsPackageLoader::JsPackageLoader;
use operit_plugin_sdk::PackageManager::{PackageStateResolver, PluginPackageManager};
use serde_json::Value;

/// Demonstrates the JavaScript execution contract implemented by an embedding application.
struct ExampleExecutionEngine;

impl JsExecutionEngine for ExampleExecutionEngine {
    /// Executes one JavaScript function in the embedding application's JavaScript runtime.
    fn execute_script_function(
        &self,
        _script: &str,
        function_name: &str,
        _params: &BTreeMap<String, Value>,
        _env_overrides: &BTreeMap<String, String>,
        _on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        _dispatch_intermediate_on_main: bool,
        _timeout_sec: u64,
    ) -> JsExecutionResult<Option<String>> {
        Ok(Some(format!("executed:{function_name}")))
    }

    /// Captures ToolPkg declarations produced by a registration function.
    fn execute_toolpkg_main_registration_function_with_text_resources(
        &self,
        _script: &str,
        _function_name: &str,
        _params: &BTreeMap<String, Value>,
        _text_resources: Option<Arc<BTreeMap<String, String>>>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        Ok(ToolPkgMainRegistrationCapture::default())
    }

    /// Renders one Compose DSL script.
    fn execute_compose_dsl_script(
        &self,
        _script: &str,
        _runtime_options: &BTreeMap<String, Value>,
        _env_overrides: &BTreeMap<String, String>,
    ) -> JsExecutionResult<Option<String>> {
        Ok(Some(r#"{"tree":{"type":"Text"}}"#.to_string()))
    }

    /// Dispatches one Compose DSL action.
    fn dispatch_compose_dsl_action(
        &self,
        action_id: &str,
        _payload: Option<Value>,
        _runtime_options: &BTreeMap<String, Value>,
        _env_overrides: &BTreeMap<String, String>,
        _on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> JsExecutionResult<Option<String>> {
        Ok(Some(format!("action:{action_id}")))
    }

    /// Releases resources owned by this example engine.
    fn destroy(&self) {}
}

/// Creates isolated execution engines for ToolPkg containers.
struct ExampleExecutionEngineFactory;

impl ToolPkgExecutionEngineFactory for ExampleExecutionEngineFactory {
    /// Creates one execution engine for a ToolPkg runtime.
    fn createToolPkgExecutionEngine(&self) -> Arc<dyn JsExecutionEngine> {
        Arc::new(ExampleExecutionEngine)
    }
}

/// Supplies embedded ToolPkg archives owned by the application.
struct ExampleAssetSource;

impl ToolPkgAssetSource for ExampleAssetSource {
    /// Returns no embedded archive because this example loads a JavaScript package from source.
    fn toolPkgAssetBytes(&self, _assetName: &str) -> Option<Vec<u8>> {
        None
    }
}

/// Selects conditional package state for the current application context.
struct ExamplePackageStateResolver;

impl PackageStateResolver for ExamplePackageStateResolver {
    /// Selects no conditional state for this minimal example.
    fn resolvePackageStateId(&self, _package: &ToolPackage) -> Option<String> {
        None
    }
}

/// Loads and registers a JavaScript package through the public plugin SDK.
fn main() {
    let package_source = r#"
        /* METADATA
        {
          name: third_party_echo,
          displayName: Third-Party Echo,
          description: Minimal package loaded through operit-plugin-sdk,
          tools: [
            {
              name: echo,
              description: Returns the supplied text,
              parameters: [
                { name: text, type: string, required: true }
              ]
            }
          ]
        }
        */
        async function echo(params) {
          return params.text;
        }
    "#;

    let package = JsPackageLoader::parse(package_source)
        .expect("the third-party JavaScript package must be valid");
    let package_name = package.name.clone();

    let mut manager = PluginPackageManager::new(
        Arc::new(ExampleExecutionEngineFactory),
        Arc::new(ExampleAssetSource),
        Arc::new(ExamplePackageStateResolver),
    );
    manager.registerPackage(package);

    let registered = manager.availablePackages().contains_key(&package_name);
    println!("registered package: {package_name}, success={registered}");
}
