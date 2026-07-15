# operit-plugin-sdk

Public Rust contracts for Operit plugin developers.

The crate owns the complete publishable plugin surface:

- standalone JavaScript package manifests, parsing, state, and package management;
- the stable JavaScript-to-Rust tool execution request and result contract;
- ToolPkg archive loading, manifests, runtime models, package queries, resources, and hooks;
- Compose DSL render models and result parsing;
- JavaScript execution-engine interfaces used by ToolPkg and package runtimes.

It intentionally contains no runtime host, MCP, skill, provider, storage, or built-in tool
implementation. Applications implement the SDK traits and inject their own host behavior:

- `JsExecutionHost` executes real tools and supplies package-facing host capabilities.
- `JsExecutionProvider` supplies the concrete JavaScript engine implementation.
- `JsPackageRuntime` supplies package state, language, and shared ToolPkg engines.
- `ToolPkgExecutionEngineFactory` creates JavaScript engines.
- `ToolPkgAssetSource` supplies embedded ToolPkg archives.
- `PackageStateResolver` selects conditional JavaScript package states.
- `ToolPkgPackageHost` supplies ToolPkg persistence and host file access.

## Entry Points

- `JsPackageLoader::JsPackageLoader`: loads and parses standalone `.js` and `.ts` packages.
- `JsTools::getJsToolsDefinition`: returns the stable JavaScript `Tools` namespace installed by
  JavaScript engine implementations.
- `JsExecutionScriptBuilder`: returns the fixed JavaScript package execution prelude, runtime
  bridge, and invocation wrapper.
- `javascript::JsExecutionHost`: receives fixed JSON tool requests and owns environment,
  configuration, ToolPkg resource, package-management, tool-name, and IPC behavior.
- `javascript::JsExecutionProvider`: creates engines and package executors using only
  caller-owned SDK contracts.
- `javascript::JsPackageExecutor`: executes package JavaScript through one bound package and
  host context.
- `javascript::JsPackageToolCallRequest` and `JsPackageToolCallResult`: define the stable
  Rust-to-JavaScript package execution envelope.
- `javascript::JsPackageRuntime`: supplies package definitions, package language, active state,
  ToolPkg subpackage metadata, and shared execution engines.
- `PackageManager::PluginPackageManager`: registers, enables, activates, removes, and resolves
  JavaScript packages and ToolPkg containers.
- `toolpkg::ToolPkgManager::ToolPkgManager`: owns ToolPkg runtime metadata, resources, listeners,
  and container-scoped execution-engine instances. Callers supply the container separately from
  the opaque context key, and container cleanup destroys every context recorded under that owner.
- `toolpkg::ToolPkgPackageService::ToolPkgPackageService`: exposes container details, UI routes,
  workspace templates, resources, and Compose DSL scripts through a caller-implemented host.
- `toolpkg::ToolPkgLoader::ToolPkgLoader`: loads external or embedded `.toolpkg` archives.
- `toolpkg::ToolPkgRegistrationBridge`: returns the fixed `registerToolPkg` JavaScript API.
- `toolpkg::ToolPkgParser`: exposes manifest, runtime, hook, and archive models.
- `toolpkg::ToolPkgComposeDslParser`: parses Compose DSL render trees, state, memo values, and
  action identifiers.
- `toolpkg::ToolPkgComposeDslBridge`: returns the fixed JavaScript DSL context installed by engine
  implementations.
- `toolpkg::ToolPkgComposeDslRuntimeScript`: wraps screen scripts with render, rerender, and action
  entry points.
- `toolpkg::ToolPkgHooks::ToolPkgHookDispatcher`: lets an embedding application trigger package
  hooks when host events occur.
- `javascript::JsExecutionEngine`: host-supplied JavaScript execution contract.

## JavaScript Execution Contract

JavaScript packages call the fixed `toolCall` interface. The JavaScript bridge converts that call
into `JsToolCallRequest` without depending on Operit's internal tool types. The embedding
application implements `JsExecutionHost` and owns permission checks, tool lookup, execution,
environment access, plugin configuration paths, ToolPkg resources, package state, tool-name
resolution, and ToolPkg IPC. Every method is required; the SDK contains no host fallback behavior.

`JsToolCallResultData::Value` preserves JSON values. `JsToolCallResultData::Binary` lets the
JavaScript bridge choose its transport encoding without exposing host-specific result types.

A JavaScript engine crate implements `JsExecutionProvider`. The top-level application constructs
that provider and injects it into the tool system. Package execution is represented by
`JsPackageExecutor`, which binds one `JsPackageRuntime` and one `JsExecutionHost` without exposing
engine-specific types.

## Rust Package Execution Runtime

Rust callers invoke JavaScript package tools through `JsPackageToolCallRequest` and receive
`JsPackageToolCallResult`. The JavaScript bridge depends on `JsPackageRuntime`, so the embedding
application keeps ownership of package storage, state selection, language resolution, and ToolPkg
engine lifetimes.

```rust
use std::sync::Arc;

use operit_plugin_sdk::javascript::{
    JsExecutionEngine, JsPackageRuntime,
};
use operit_plugin_sdk::package::ToolPackage;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSubpackageRuntime;

struct ApplicationPackages;

impl JsPackageRuntime for ApplicationPackages {
    /// Returns the language exposed to package JavaScript.
    fn package_language(&self) -> Result<String, String> {
        Ok("en-US".to_string())
    }

    /// Returns one package definition from application-owned storage.
    fn package(&self, package_name: &str) -> Option<ToolPackage> {
        todo!("load package {package_name} from application storage")
    }

    /// Returns the selected conditional package state.
    fn active_package_state_id(&self, package_name: &str) -> Option<String> {
        todo!("resolve active state for {package_name}")
    }

    /// Resolves ToolPkg subpackage metadata.
    fn resolve_toolpkg_subpackage(
        &self,
        package_name: &str,
    ) -> Option<ToolPkgSubpackageRuntime> {
        todo!("resolve ToolPkg subpackage {package_name}")
    }

    /// Returns the shared ToolPkg engine for one explicitly owned execution context.
    fn toolpkg_execution_engine(
        &self,
        context_key: &str,
        container_package_name: &str,
    ) -> Arc<dyn JsExecutionEngine> {
        todo!("return {context_key} owned by {container_package_name}")
    }
}
```
