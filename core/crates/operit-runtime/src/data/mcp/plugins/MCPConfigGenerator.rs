use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::data::mcp::plugins::MCPProjectAnalyzer::{ProjectStructure, ProjectType};

pub struct MCPConfigGenerator;

impl MCPConfigGenerator {
    #[allow(non_snake_case)]
    pub fn generateMcpConfig(
        &self,
        pluginId: &str,
        projectStructure: &ProjectStructure,
        environmentVariables: BTreeMap<String, String>,
        pluginDirPath: Option<&str>,
    ) -> String {
        let mut finalConfigJson = projectStructure
            .configExample
            .as_ref()
            .and_then(|config| serde_json::from_str::<Value>(config).ok())
            .filter(|value| value.get("mcpServers").and_then(Value::as_object).is_some());

        if finalConfigJson.is_none() || matches!(projectStructure.r#type, ProjectType::TYPESCRIPT) {
            let mut configJson = finalConfigJson.unwrap_or_else(|| Value::Object(Map::new()));
            if configJson
                .get("mcpServers")
                .and_then(Value::as_object)
                .is_none()
            {
                configJson["mcpServers"] = Value::Object(Map::new());
            }
            let serverName = pluginId;
            let mcpServers = configJson
                .get_mut("mcpServers")
                .and_then(Value::as_object_mut)
                .expect("mcpServers object created");
            if !mcpServers.contains_key(serverName) {
                mcpServers.insert(serverName.to_string(), Value::Object(Map::new()));
            }
            let serverJson = mcpServers
                .get_mut(serverName)
                .and_then(Value::as_object_mut)
                .expect("server object created");

            match projectStructure.r#type {
                ProjectType::PYTHON => {
                    if !serverJson.contains_key("command") {
                        let pythonCommand = pluginDirPath
                            .map(|path| format!("{path}/venv/bin/python"))
                            .unwrap_or_else(|| "python".to_string());
                        serverJson.insert("command".to_string(), Value::String(pythonCommand));
                    }
                    if !serverJson.contains_key("args") {
                        let moduleName = projectStructure
                            .pythonPackageName
                            .clone()
                            .or_else(|| projectStructure.mainPythonModule.clone())
                            .unwrap_or_else(|| pluginId.replace('-', "_").to_ascii_lowercase());
                        serverJson.insert(
                            "args".to_string(),
                            Value::Array(vec![
                                Value::String("-m".to_string()),
                                Value::String(moduleName),
                            ]),
                        );
                    }
                }
                ProjectType::TYPESCRIPT => {
                    serverJson.insert("command".to_string(), Value::String("node".to_string()));
                    let outDir = projectStructure
                        .tsConfigOutDir
                        .clone()
                        .unwrap_or_else(|| "dist".to_string());
                    let rootDir = projectStructure
                        .tsConfigRootDir
                        .clone()
                        .unwrap_or_else(|| "src".to_string());
                    let compiledPath = projectStructure
                        .mainTsFile
                        .as_ref()
                        .map(|mainTsFile| compiledTsPath(mainTsFile, &rootDir, &outDir))
                        .unwrap_or_else(|| format!("{outDir}/index.js"));
                    serverJson.insert(
                        "args".to_string(),
                        Value::Array(vec![Value::String(compiledPath)]),
                    );
                }
                ProjectType::NODEJS => {
                    if !serverJson.contains_key("command") {
                        serverJson.insert("command".to_string(), Value::String("node".to_string()));
                    }
                    if !serverJson.contains_key("args") {
                        let mainFile = projectStructure
                            .mainJsFile
                            .clone()
                            .unwrap_or_else(|| "index.js".to_string());
                        serverJson.insert(
                            "args".to_string(),
                            Value::Array(vec![Value::String(mainFile)]),
                        );
                    }
                }
                ProjectType::UNKNOWN => {
                    if !serverJson.contains_key("command") {
                        let pythonCommand = pluginDirPath
                            .map(|path| format!("{path}/venv/bin/python"))
                            .unwrap_or_else(|| "python".to_string());
                        serverJson.insert("command".to_string(), Value::String(pythonCommand));
                    }
                    if !serverJson.contains_key("args") {
                        serverJson.insert(
                            "args".to_string(),
                            Value::Array(vec![
                                Value::String("-m".to_string()),
                                Value::String(pluginId.replace('-', "_").to_ascii_lowercase()),
                            ]),
                        );
                    }
                }
            }

            if !serverJson.contains_key("autoApprove") {
                serverJson.insert("autoApprove".to_string(), Value::Array(Vec::new()));
            }
            if !environmentVariables.is_empty() {
                serverJson.insert(
                    "env".to_string(),
                    Value::Object(
                        environmentVariables
                            .into_iter()
                            .map(|(key, value)| (key, Value::String(value)))
                            .collect(),
                    ),
                );
            } else if !serverJson.contains_key("env") {
                serverJson.insert("env".to_string(), Value::Object(Map::new()));
            }
            finalConfigJson = Some(configJson);
        }

        serde_json::to_string_pretty(&finalConfigJson.unwrap_or_else(|| {
            serde_json::json!({
                "mcpServers": {}
            })
        }))
        .unwrap_or_else(|_| "{\"mcpServers\":{}}".to_string())
    }
}

#[allow(non_snake_case)]
fn compiledTsPath(mainTsFile: &str, rootDir: &str, outDir: &str) -> String {
    let normalizedRootDir = if rootDir.is_empty() { "" } else { rootDir };
    if normalizedRootDir.is_empty() {
        let relativePath = mainTsFile.trim_start_matches("src/");
        return format!("{outDir}/{}", relativePath.replace(".ts", ".js"));
    }
    if let Some(relative) = mainTsFile.strip_prefix(&format!("{normalizedRootDir}/")) {
        return format!("{outDir}/{}", relative.replace(".ts", ".js"));
    }
    if mainTsFile.starts_with("src/") && normalizedRootDir == "src" {
        return format!(
            "{outDir}/{}",
            mainTsFile.trim_start_matches("src/").replace(".ts", ".js")
        );
    }
    format!("{outDir}/{}", mainTsFile.replace(".ts", ".js"))
}
