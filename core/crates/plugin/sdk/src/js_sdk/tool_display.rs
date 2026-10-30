//! Display semantics declared by a tool owner and carried separately from model parameters.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

/// Selects a renderer by explicit tool-owner intent, independently of the result's type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolResultRenderer {
    Directory,
    FileDiff,
    #[default]
    #[serde(other)]
    Text,
}

/// Declares localized action text, a semantic icon, and an optional target parameter.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolDisplayDeclaration {
    pub label: BTreeMap<String, String>,
    #[serde(default)]
    pub icon: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_parameter: Option<String>,
    #[serde(default)]
    pub result_renderer: ToolResultRenderer,
}

impl ToolDisplayDeclaration {
    /// Resolves a built-in resource at the declaration boundary; clients receive all available locales.
    pub fn builtin(
        action: &str,
        icon: &str,
        target: Option<&str>,
        result_renderer: ToolResultRenderer,
    ) -> Self {
        static LABELS: OnceLock<BTreeMap<String, BTreeMap<String, String>>> = OnceLock::new();
        let labels = LABELS.get_or_init(|| {
            serde_json::from_str(include_str!("tool_display_labels.json"))
                .expect("valid tool display resources")
        });
        Self {
            label: labels
                .get(action)
                .unwrap_or_else(|| panic!("missing tool display resource: {action}"))
                .clone(),
            icon: icon.to_string(),
            target_parameter: target.map(str::to_string),
            result_renderer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js_sdk::tool_types::BuiltinToolName;
    use crate::JsPackageLoader::JsPackageLoader;

    #[test]
    fn builtin_metadata_is_declared_with_valid_resources() {
        for tool in BuiltinToolName::ALL {
            if let Some(display) = tool.display() {
                assert!(!display.label["en"].is_empty());
                assert!(!display.label["zh"].is_empty());
            }
        }
        let display = BuiltinToolName::ReadFile.display().unwrap();
        assert_eq!(display.target_parameter.as_deref(), Some("path"));
        assert_eq!(display.icon, "file");
        assert_eq!(display.result_renderer, ToolResultRenderer::Text);
        assert_eq!(
            BuiltinToolName::ListFiles
                .display()
                .unwrap()
                .result_renderer,
            ToolResultRenderer::Directory
        );
        assert_eq!(
            BuiltinToolName::ApplyFile
                .display()
                .unwrap()
                .result_renderer,
            ToolResultRenderer::FileDiff
        );
    }

    #[test]
    fn package_metadata_carries_arbitrary_tool_names_and_locales() {
        let package = JsPackageLoader::parse_metadata(r#"{
          "name":"example", "tools":[
            {"name":"z42", "display": {"label":{"fr":"Lire","default":"Read"},"icon":"file","targetParameter":"document","resultRenderer":"directory"}},
            {"name":"delete_everything"}
          ]
        }"#, "").unwrap();
        let display = package.tools[0].display.as_ref().unwrap();
        assert_eq!(display.label["fr"], "Lire");
        assert_eq!(display.result_renderer, ToolResultRenderer::Directory);
        assert_eq!(display.target_parameter.as_deref(), Some("document"));
        assert!(package.tools[1].display.is_none());
        let json = serde_json::to_value(display).unwrap();
        assert_eq!(json["targetParameter"], "document");
    }
    #[test]
    fn invalid_display_is_reported_instead_of_silently_removing_tools() {
        for metadata in [
            r#"{"name":"example","tools":[{"name":"z42","display":{"label":42}}]}"#,
            r#"{"name":"example","states":[{"id":"active","tools":[{"name":"z42","display":{"label":{"en":"Read"},"icon":42}}]}]}"#,
        ] {
            assert!(JsPackageLoader::parse_metadata(metadata, "").is_err());
        }
    }
    #[test]
    fn bundled_terminal_tools_declare_display_in_every_platform_state() {
        let package = JsPackageLoader::parse(include_str!(
            "../../../../../../plugins/packages/buildin/super_admin.ts"
        ))
        .unwrap();
        for tool in package
            .tools
            .iter()
            .chain(package.states.iter().flat_map(|state| &state.tools))
        {
            let display = tool
                .display
                .as_ref()
                .expect("terminal tools own their presentation");
            assert_eq!(display.icon, "terminal");
            let target = display.target_parameter.as_deref().unwrap();
            assert!(tool
                .parameters
                .iter()
                .any(|parameter| parameter.name == target));
        }
    }
}
