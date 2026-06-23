use std::collections::{BTreeMap, BTreeSet};

use operit_host_api::HostEnvironmentDescriptor;

use operit_context::OperitApplicationContext::OperitApplicationContext;
use crate::core::config::SystemToolPrompts::{
    SystemToolPromptCategory as ConfigSystemToolPromptCategory, SystemToolPrompts,
    ToolPrompt as ConfigToolPrompt,
};
use crate::core::tools::packTool::PackageManager::{CachedMcpToolInfo, PackageManager};
use crate::core::tools::ToolPackage::{PackageTool, PackageToolParameter, ToolPackage};
use crate::data::mcp::MCPLocalServer::MCPLocalServer;
use crate::data::model::ModelConfigData::ApiProviderType;
use crate::data::model::ToolPrompt::{SystemToolPromptCategory, ToolParameterSchema, ToolPrompt};
use crate::data::preferences::CharacterCardToolAccessResolver::ResolvedCharacterCardToolAccess;
use crate::data::skill::SkillRepository::SkillRepository;

pub const SEARCH_TOOL_NAME: &str = "search";
pub const PROXY_TOOL_NAME: &str = "proxy";
pub const PACKAGE_PROXY_TOOL_NAME: &str = "package_proxy";
const DEFAULT_SEARCH_LIMIT: i32 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolExposureMode {
    FULL,
    CLI,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HiddenToolSourceKind {
    BUILTIN,
    INTERNAL,
    PACKAGE,
    MCP,
    ACTIVATION,
}

impl HiddenToolSourceKind {
    pub fn label(&self, useEnglish: bool) -> String {
        match self {
            HiddenToolSourceKind::BUILTIN => {
                if useEnglish { "built-in" } else { "内置" }.to_string()
            }
            HiddenToolSourceKind::INTERNAL => {
                if useEnglish { "internal" } else { "内部" }.to_string()
            }
            HiddenToolSourceKind::PACKAGE => if useEnglish { "package" } else { "包" }.to_string(),
            HiddenToolSourceKind::MCP => if useEnglish { "mcp" } else { "MCP" }.to_string(),
            HiddenToolSourceKind::ACTIVATION => {
                if useEnglish { "activation" } else { "激活" }.to_string()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenToolCatalogEntry {
    pub target_tool_name: String,
    pub display_name: String,
    pub description: String,
    pub parameter_hints: Vec<String>,
    pub source_kind: HiddenToolSourceKind,
    pub keywords: Vec<String>,
    pub suggested_params_json: Option<String>,
}

pub struct CliToolModeSupport;

impl ToolExposureMode {
    pub fn resolve(provider_type: ApiProviderType) -> Self {
        match provider_type {
            ApiProviderType::LMSTUDIO
            | ApiProviderType::OLLAMA
            | ApiProviderType::OPENAI_LOCAL
            | ApiProviderType::MNN
            | ApiProviderType::LLAMA_CPP => Self::CLI,
            _ => Self::FULL,
        }
    }
}

impl CliToolModeSupport {
    #[allow(non_snake_case)]
    pub fn isCliPublicTool(toolName: &str) -> bool {
        matches!(toolName.trim(), SEARCH_TOOL_NAME | PROXY_TOOL_NAME)
    }

    #[allow(non_snake_case)]
    pub fn isReservedProxyTarget(toolName: &str) -> bool {
        matches!(
            toolName.trim(),
            SEARCH_TOOL_NAME | PROXY_TOOL_NAME | PACKAGE_PROXY_TOOL_NAME
        )
    }

    #[allow(non_snake_case)]
    pub fn defaultSearchLimit() -> i32 {
        DEFAULT_SEARCH_LIMIT
    }

    #[allow(non_snake_case)]
    pub fn buildCliPublicToolPrompts(useEnglish: bool) -> Vec<ToolPrompt> {
        if useEnglish {
            vec![
                ToolPrompt {
                    name: SEARCH_TOOL_NAME.to_string(),
                    description: "Search the hidden tool catalog only. Use this first to discover hidden tool names and parameter shapes.".to_string(),
                    parameters: String::new(),
                    parametersStructured: Some(vec![
                        ToolParameterSchema {
                            name: "query".to_string(),
                            r#type: "string".to_string(),
                            description: "tool capability or hidden tool name to search for".to_string(),
                            required: true,
                            default: None,
                        },
                        ToolParameterSchema {
                            name: "limit".to_string(),
                            r#type: "integer".to_string(),
                            description: "optional, max results to return".to_string(),
                            required: false,
                            default: Some(DEFAULT_SEARCH_LIMIT.to_string()),
                        },
                    ]),
                    details: String::new(),
                    notes: String::new(),
                },
                ToolPrompt {
                    name: PROXY_TOOL_NAME.to_string(),
                    description: "Execute a hidden tool after you discover its target tool name and parameter shape via search.".to_string(),
                    parameters: String::new(),
                    parametersStructured: Some(vec![
                        ToolParameterSchema {
                            name: "tool_name".to_string(),
                            r#type: "string".to_string(),
                            description: "hidden target tool name, for example read_file or packageName:toolName".to_string(),
                            required: true,
                            default: None,
                        },
                        ToolParameterSchema {
                            name: "params".to_string(),
                            r#type: "object".to_string(),
                            description: "JSON object of parameters to forward to the hidden target tool".to_string(),
                            required: true,
                            default: None,
                        },
                    ]),
                    details: String::new(),
                    notes: String::new(),
                },
            ]
        } else {
            vec![
                ToolPrompt {
                    name: SEARCH_TOOL_NAME.to_string(),
                    description: "仅搜索隐藏工具目录。先用它发现隐藏工具名和参数形态。".to_string(),
                    parameters: String::new(),
                    parametersStructured: Some(vec![
                        ToolParameterSchema {
                            name: "query".to_string(),
                            r#type: "string".to_string(),
                            description: "要搜索的工具能力或隐藏工具名".to_string(),
                            required: true,
                            default: None,
                        },
                        ToolParameterSchema {
                            name: "limit".to_string(),
                            r#type: "integer".to_string(),
                            description: "可选，返回的最大结果数".to_string(),
                            required: false,
                            default: Some(DEFAULT_SEARCH_LIMIT.to_string()),
                        },
                    ]),
                    details: String::new(),
                    notes: String::new(),
                },
                ToolPrompt {
                    name: PROXY_TOOL_NAME.to_string(),
                    description: "在 search 发现目标工具名和参数形态后，代理执行隐藏工具。"
                        .to_string(),
                    parameters: String::new(),
                    parametersStructured: Some(vec![
                        ToolParameterSchema {
                            name: "tool_name".to_string(),
                            r#type: "string".to_string(),
                            description: "隐藏目标工具名，例如 read_file 或 packageName:toolName"
                                .to_string(),
                            required: true,
                            default: None,
                        },
                        ToolParameterSchema {
                            name: "params".to_string(),
                            r#type: "object".to_string(),
                            description: "转发给隐藏目标工具的 JSON 参数对象".to_string(),
                            required: true,
                            default: None,
                        },
                    ]),
                    details: String::new(),
                    notes: String::new(),
                },
            ]
        }
    }

    #[allow(non_snake_case)]
    pub fn buildCliModePrompt(useEnglish: bool) -> String {
        let intro = if useEnglish {
            r#"CLI TOOL MODE
- Only two public tools are available: `search` and `proxy`.
- `search` only searches the hidden tool catalog. It does not read files, search code, or browse the web.
- All real capabilities are hidden behind `proxy`.
- Do not call hidden tools directly. Use `search` first, then call `proxy` with the discovered target tool name and JSON params."#
        } else {
            r#"CLI 工具模式
- 当前只有两个公开工具：`search` 和 `proxy`。
- `search` 只搜索隐藏工具目录，不会直接读文件、搜代码或访问网页。
- 所有真实能力都隐藏在 `proxy` 后面。
- 不要直接调用隐藏工具。先用 `search`，再用发现到的目标工具名和 JSON 参数调用 `proxy`。"#
        };
        let category = SystemToolPromptCategory {
            categoryName: if useEnglish {
                "Public tools"
            } else {
                "公开工具"
            }
            .to_string(),
            categoryHeader: String::new(),
            tools: Self::buildCliPublicToolPrompts(useEnglish),
            categoryFooter: String::new(),
        };
        format!("{intro}\n\n{category}")
    }

    #[allow(non_snake_case)]
    pub fn buildHiddenToolCatalog(
        context: &OperitApplicationContext,
        packageManager: &PackageManager,
        useEnglish: bool,
        roleCardToolAccess: &ResolvedCharacterCardToolAccess,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> Vec<HiddenToolCatalogEntry> {
        let categories = Self::buildBuiltinAndInternalCategories(useEnglish, hostEnvironment);
        let builtinToolNames = Self::buildBuiltinToolNameSet(useEnglish, hostEnvironment);
        let mut entries = BTreeMap::new();

        for category in categories {
            for tool in category.tools {
                if tool.name == "use_package" {
                    continue;
                }
                if Self::isReservedProxyTarget(&tool.name) || Self::isCliPublicTool(&tool.name) {
                    continue;
                }
                if !Self::isToolNameAllowedForRoleCard(&tool.name, None, roleCardToolAccess) {
                    continue;
                }

                let sourceKind = if builtinToolNames.contains(&tool.name) {
                    HiddenToolSourceKind::BUILTIN
                } else {
                    HiddenToolSourceKind::INTERNAL
                };
                let entry = HiddenToolCatalogEntry {
                    target_tool_name: tool.name.clone(),
                    display_name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameter_hints: Self::buildParameterHints(&tool),
                    source_kind: sourceKind,
                    keywords: vec![category.category_name.clone()],
                    suggested_params_json: None,
                };
                entries.entry(Self::catalogKey(&entry)).or_insert(entry);
            }
        }

        let enabledPackages = packageManager
            .getEnabledPackageNames()
            .into_iter()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .filter(|name| !packageManager.isToolPkgContainer(name))
            .filter(|name| roleCardToolAccess.isExternalSourceAllowed(name))
            .collect::<Vec<_>>();

        for packageName in enabledPackages {
            let Some(toolPackage) = packageManager.getEffectivePackageTools(&packageName) else {
                continue;
            };
            if toolPackage.tools.is_empty() {
                Self::addActivationEntry(
                    &mut entries,
                    &packageName,
                    &toolPackage.description.resolve(useEnglish),
                    "package",
                    HiddenToolSourceKind::ACTIVATION,
                );
            } else {
                Self::addPackageToolEntries(
                    &mut entries,
                    &packageName,
                    &toolPackage,
                    useEnglish,
                    HiddenToolSourceKind::PACKAGE,
                    "package",
                );
            }
        }

        let skillPackages = SkillRepository::getInstance(context).getAiVisibleSkillPackages();
        for (skillName, skillPackage) in skillPackages {
            if !roleCardToolAccess.isExternalSourceAllowed(&skillName) {
                continue;
            }
            Self::addActivationEntry(
                &mut entries,
                &skillName,
                &skillPackage.description,
                "skill",
                HiddenToolSourceKind::ACTIVATION,
            );
        }

        let mcpServers = packageManager.getAvailableServerPackages();
        let mcpLocalServer = MCPLocalServer::getInstance(context);
        for (serverName, serverConfig) in mcpServers {
            if !roleCardToolAccess.isExternalSourceAllowed(&serverName) {
                continue;
            }
            let cachedTools = mcpLocalServer
                .getCachedTools(&serverName)
                .unwrap_or_default();
            if cachedTools.is_empty() {
                Self::addActivationEntry(
                    &mut entries,
                    &serverName,
                    &serverConfig.description,
                    "mcp",
                    HiddenToolSourceKind::ACTIVATION,
                );
                continue;
            }
            Self::addCachedMcpToolEntries(
                &mut entries,
                &serverName,
                &serverConfig.description,
                &cachedTools,
            );
        }

        entries.into_values().collect()
    }

    #[allow(non_snake_case)]
    pub fn searchHiddenToolCatalog(
        catalog: &[HiddenToolCatalogEntry],
        query: &str,
        limit: i32,
    ) -> Vec<HiddenToolCatalogEntry> {
        let normalizedQuery = normalize(query);
        if normalizedQuery.is_empty() {
            return Vec::new();
        }

        let terms = normalizedQuery
            .split(' ')
            .filter(|term| !term.trim().is_empty())
            .map(|term| term.to_string())
            .collect::<Vec<_>>();
        let mut ranked = catalog
            .iter()
            .filter_map(|entry| {
                let score = scoreEntry(entry, &normalizedQuery, &terms);
                if score <= 0 {
                    None
                } else {
                    Some((score, entry.clone()))
                }
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.target_tool_name.cmp(&right.1.target_tool_name))
                .then_with(|| left.1.display_name.cmp(&right.1.display_name))
        });
        let boundedLimit = limit.clamp(1, 20) as usize;
        ranked
            .into_iter()
            .take(boundedLimit)
            .map(|(_, entry)| entry)
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn formatSearchResults(
        query: &str,
        results: &[HiddenToolCatalogEntry],
        useEnglish: bool,
    ) -> String {
        if results.is_empty() {
            return if useEnglish {
                format!(
                    "No hidden tools matched \"{query}\". Try a broader capability keyword, then call proxy with a discovered target tool name."
                )
            } else {
                format!(
                    "没有隐藏工具匹配“{query}”。请尝试更宽泛的能力关键词，然后再用发现到的目标工具名调用 proxy。"
                )
            };
        }

        let mut output = String::new();
        if useEnglish {
            output.push_str(&format!("Hidden tool search results for \"{query}\":\n"));
        } else {
            output.push_str(&format!("“{query}”的隐藏工具搜索结果：\n"));
        }
        for (index, entry) in results.iter().enumerate() {
            output.push_str(&format!(
                "{}. `{}` [{}]\n",
                index + 1,
                entry.display_name,
                entry.source_kind.label(useEnglish)
            ));
            output.push_str("   ");
            if entry.description.trim().is_empty() {
                output.push_str(if useEnglish {
                    "No description."
                } else {
                    "无描述。"
                });
                output.push('\n');
            } else {
                output.push_str(&entry.description);
                output.push('\n');
            }
            output.push_str("   ");
            if useEnglish {
                output.push_str(&format!("Target: `{}`\n", entry.target_tool_name));
            } else {
                output.push_str(&format!("目标工具：`{}`\n", entry.target_tool_name));
            }
            match &entry.suggested_params_json {
                Some(params) if !params.trim().is_empty() => {
                    output.push_str("   ");
                    if useEnglish {
                        output.push_str(&format!("Params hint: `{params}`\n"));
                    } else {
                        output.push_str(&format!("参数示例：`{params}`\n"));
                    }
                }
                _ if !entry.parameter_hints.is_empty() => {
                    output.push_str("   ");
                    if useEnglish {
                        output.push_str("Params: ");
                    } else {
                        output.push_str("参数：");
                    }
                    output.push_str(&entry.parameter_hints.join("; "));
                    output.push('\n');
                }
                _ => {}
            }
        }
        output.trim_end().to_string()
    }

    #[allow(non_snake_case)]
    pub fn buildCliTopLevelRestrictionErrorMessage(
        attemptedToolName: &str,
        useEnglish: bool,
    ) -> String {
        if useEnglish {
            format!(
                "Tool '{attemptedToolName}' is hidden in CLI tool mode. Use 'search' to find the hidden target tool, then call 'proxy'."
            )
        } else {
            format!(
                "工具“{attemptedToolName}”在 CLI 工具模式下是隐藏的。请先用 `search` 查找隐藏目标工具，再调用 `proxy`。"
            )
        }
    }

    #[allow(non_snake_case)]
    pub fn buildCliModeUnavailableMessage(useEnglish: bool) -> String {
        if useEnglish {
            "This tool is only available in CLI tool mode.".to_string()
        } else {
            "该工具仅在 CLI 工具模式下可用。".to_string()
        }
    }

    #[allow(non_snake_case)]
    pub fn buildProxyTargetUnavailableMessage(targetToolName: &str, useEnglish: bool) -> String {
        if useEnglish {
            format!(
                "Hidden target tool '{targetToolName}' is unavailable. Use 'search' first to discover a valid hidden tool name and params."
            )
        } else {
            format!(
                "隐藏目标工具“{targetToolName}”不可用。请先用 `search` 发现有效的隐藏工具名和参数。"
            )
        }
    }

    #[allow(non_snake_case)]
    pub fn buildReservedProxyTargetMessage(targetToolName: &str, useEnglish: bool) -> String {
        if useEnglish {
            format!("Hidden target tool '{targetToolName}' is reserved and cannot be called through proxy.")
        } else {
            format!("隐藏目标工具“{targetToolName}”是保留目标，不能通过 proxy 调用。")
        }
    }

    #[allow(non_snake_case)]
    pub fn buildRoleAccessDeniedMessage(useEnglish: bool) -> String {
        if useEnglish {
            "The current role card is not allowed to access this hidden tool.".to_string()
        } else {
            "当前角色卡无权访问这个隐藏工具。".to_string()
        }
    }

    #[allow(non_snake_case)]
    pub fn isToolNameAllowedForRoleCard(
        toolName: &str,
        usePackageSourceName: Option<&str>,
        roleCardToolAccess: &ResolvedCharacterCardToolAccess,
    ) -> bool {
        if toolName == "use_package" {
            if !roleCardToolAccess.isBuiltinToolAllowed("use_package") {
                return false;
            }
            return usePackageSourceName
                .map(str::trim)
                .map(|sourceName| {
                    sourceName.is_empty() || roleCardToolAccess.isExternalSourceAllowed(sourceName)
                })
                .unwrap_or(true);
        }
        if toolName.contains(':') {
            let sourceName = toolName.split(':').next().unwrap_or("").trim();
            return sourceName.is_empty() || roleCardToolAccess.isExternalSourceAllowed(sourceName);
        }
        roleCardToolAccess.isBuiltinToolAllowed(toolName)
    }

    #[allow(non_snake_case)]
    fn buildBuiltinAndInternalCategories(
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> Vec<ConfigSystemToolPromptCategory> {
        if useEnglish {
            SystemToolPrompts::getAllCategoriesEnForHost(
                false,
                false,
                false,
                false,
                false,
                false,
                &[],
                hostEnvironment,
            )
        } else {
            SystemToolPrompts::getAllCategoriesCnForHost(
                false,
                false,
                false,
                false,
                false,
                false,
                &[],
                hostEnvironment,
            )
        }
    }

    #[allow(non_snake_case)]
    fn buildBuiltinToolNameSet(
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> BTreeSet<String> {
        let categories = if useEnglish {
            SystemToolPrompts::getAIAllCategoriesEnForHost(
                false,
                false,
                false,
                false,
                false,
                false,
                &[],
                hostEnvironment,
            )
        } else {
            SystemToolPrompts::getAIAllCategoriesCnForHost(
                false,
                false,
                false,
                false,
                false,
                false,
                &[],
                hostEnvironment,
            )
        };
        categories
            .into_iter()
            .flat_map(|category| category.tools)
            .map(|tool| tool.name)
            .collect()
    }

    #[allow(non_snake_case)]
    fn buildParameterHints(tool: &ConfigToolPrompt) -> Vec<String> {
        if !tool.parameters_structured.is_empty() {
            return tool
                .parameters_structured
                .iter()
                .map(|parameter| {
                    Self::buildParameterHint(
                        &parameter.name,
                        &parameter.description,
                        &parameter.value_type,
                        parameter.required,
                    )
                })
                .collect();
        }
        tool.parameters
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect()
    }

    #[allow(non_snake_case)]
    fn buildParameterHint(
        name: &str,
        description: &str,
        valueType: &str,
        required: bool,
    ) -> String {
        let requiredText = if required { "required" } else { "optional" };
        format!("{name} [{valueType}, {requiredText}]: {description}")
    }

    #[allow(non_snake_case)]
    fn addActivationEntry(
        entries: &mut BTreeMap<String, HiddenToolCatalogEntry>,
        displayName: &str,
        description: &str,
        keywordTag: &str,
        sourceKind: HiddenToolSourceKind,
    ) {
        let entry = HiddenToolCatalogEntry {
            target_tool_name: "use_package".to_string(),
            display_name: displayName.to_string(),
            description: description.to_string(),
            parameter_hints: vec![format!("package_name [string, required]: {displayName}")],
            source_kind: sourceKind,
            keywords: vec![
                keywordTag.to_string(),
                "use_package".to_string(),
                "activate".to_string(),
            ],
            suggested_params_json: Some(format!("{{\"package_name\":\"{displayName}\"}}")),
        };
        entries.entry(Self::catalogKey(&entry)).or_insert(entry);
    }

    #[allow(non_snake_case)]
    fn addPackageToolEntries(
        entries: &mut BTreeMap<String, HiddenToolCatalogEntry>,
        prefix: &str,
        toolPackage: &ToolPackage,
        useEnglish: bool,
        sourceKind: HiddenToolSourceKind,
        keywordTag: &str,
    ) {
        for packageTool in toolPackage.tools.iter().filter(|tool| !tool.advice) {
            let targetToolName = format!("{prefix}:{}", packageTool.name);
            let entry = HiddenToolCatalogEntry {
                target_tool_name: targetToolName.clone(),
                display_name: targetToolName,
                description: Self::packageToolDescription(packageTool, useEnglish),
                parameter_hints: packageTool
                    .parameters
                    .iter()
                    .map(|parameter| Self::packageToolParameterHint(parameter, useEnglish))
                    .collect(),
                source_kind: sourceKind.clone(),
                keywords: vec![
                    prefix.to_string(),
                    keywordTag.to_string(),
                    toolPackage.name.clone(),
                ],
                suggested_params_json: None,
            };
            entries.entry(Self::catalogKey(&entry)).or_insert(entry);
        }
    }

    #[allow(non_snake_case)]
    fn packageToolDescription(packageTool: &PackageTool, useEnglish: bool) -> String {
        packageTool.description.resolve(useEnglish)
    }

    #[allow(non_snake_case)]
    fn packageToolParameterHint(parameter: &PackageToolParameter, useEnglish: bool) -> String {
        Self::buildParameterHint(
            &parameter.name,
            &parameter.description.resolve(useEnglish),
            &parameter.parameter_type,
            parameter.required,
        )
    }

    #[allow(non_snake_case)]
    fn addCachedMcpToolEntries(
        entries: &mut BTreeMap<String, HiddenToolCatalogEntry>,
        serverName: &str,
        serverDescription: &str,
        cachedTools: &[CachedMcpToolInfo],
    ) {
        for cachedTool in cachedTools {
            let toolName = cachedTool.name.trim();
            if toolName.is_empty() {
                continue;
            }
            let targetToolName = format!("{serverName}:{toolName}");
            let entry = HiddenToolCatalogEntry {
                target_tool_name: targetToolName.clone(),
                display_name: targetToolName,
                description: if cachedTool.description.trim().is_empty() {
                    serverDescription.to_string()
                } else {
                    cachedTool.description.clone()
                },
                parameter_hints: Self::buildCachedMcpParameterHints(&cachedTool.inputSchema),
                source_kind: HiddenToolSourceKind::MCP,
                keywords: vec![
                    serverName.to_string(),
                    "mcp".to_string(),
                    "cached".to_string(),
                ],
                suggested_params_json: None,
            };
            entries.entry(Self::catalogKey(&entry)).or_insert(entry);
        }
    }

    #[allow(non_snake_case)]
    fn buildCachedMcpParameterHints(inputSchemaJson: &str) -> Vec<String> {
        let Ok(schema) = serde_json::from_str::<serde_json::Value>(inputSchemaJson) else {
            return Vec::new();
        };
        let Some(properties) = schema.get("properties").and_then(|value| value.as_object()) else {
            return Vec::new();
        };
        let requiredNames = schema
            .get("required")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        properties
            .iter()
            .map(|(name, parameterObject)| {
                let valueType = parameterObject
                    .get("type")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or("string");
                let description = parameterObject
                    .get("description")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                Self::buildParameterHint(name, description, valueType, requiredNames.contains(name))
            })
            .collect()
    }

    #[allow(non_snake_case)]
    fn catalogKey(entry: &HiddenToolCatalogEntry) -> String {
        format!(
            "{:?}:{}:{}",
            entry.source_kind, entry.target_tool_name, entry.display_name
        )
    }
}

#[allow(non_snake_case)]
fn scoreEntry(entry: &HiddenToolCatalogEntry, normalizedQuery: &str, terms: &[String]) -> i32 {
    let displayName = normalize(&entry.display_name);
    let targetName = normalize(&entry.target_tool_name);
    let description = normalize(&entry.description);
    let params = normalize(&entry.parameter_hints.join(" "));
    let keywords = normalize(&entry.keywords.join(" "));

    let mut score = 0;
    if displayName == normalizedQuery || targetName == normalizedQuery {
        score += 300;
    }
    if displayName.starts_with(normalizedQuery) || targetName.starts_with(normalizedQuery) {
        score += 140;
    }
    if displayName.contains(normalizedQuery) || targetName.contains(normalizedQuery) {
        score += 100;
    }
    if description.contains(normalizedQuery) || keywords.contains(normalizedQuery) {
        score += 40;
    }
    if params.contains(normalizedQuery) {
        score += 25;
    }

    let mut matchedTerms = 0;
    for term in terms {
        let mut termMatched = false;
        if displayName.contains(term) || targetName.contains(term) {
            score += 40;
            termMatched = true;
        }
        if keywords.contains(term) {
            score += 16;
            termMatched = true;
        }
        if description.contains(term) {
            score += 12;
            termMatched = true;
        }
        if params.contains(term) {
            score += 8;
            termMatched = true;
        }
        if termMatched {
            matchedTerms += 1;
        }
    }
    if matchedTerms == terms.len() && !terms.is_empty() {
        score += 30;
    }

    score
}

fn normalize(value: &str) -> String {
    let mut normalized = String::new();
    let mut previousSpace = false;
    for ch in value.chars() {
        if ch.is_alphanumeric() || matches!(ch, ':' | '_' | '.' | '/' | '-') {
            for lower in ch.to_lowercase() {
                normalized.push(lower);
            }
            previousSpace = false;
        } else if !previousSpace {
            normalized.push(' ');
            previousSpace = true;
        }
    }
    normalized.trim().to_string()
}
