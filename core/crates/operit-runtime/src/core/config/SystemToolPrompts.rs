use std::collections::{BTreeSet, HashMap};
use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::chat::hooks::PromptHookRegistry::{PromptHookContext, PromptHookRegistry};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolParameterSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: String,
    pub description: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolPrompt {
    pub name: String,
    pub description: String,
    pub parameters: String,
    #[serde(rename = "parametersStructured")]
    pub parameters_structured: Vec<ToolParameterSchema>,
    pub details: String,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemToolPromptCategory {
    #[serde(rename = "categoryName")]
    pub category_name: String,
    #[serde(rename = "categoryHeader")]
    pub category_header: String,
    pub tools: Vec<ToolPrompt>,
    #[serde(rename = "categoryFooter")]
    pub category_footer: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManageableToolPrompt {
    pub category_name: String,
    pub name: String,
    pub description: String,
}

pub struct SystemToolPrompts;

impl Display for ToolParameterSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.default {
            Some(default) => write!(
                f,
                "- {} ({}, {}, default={}): {}",
                self.name,
                self.value_type,
                if self.required { "required" } else { "optional" },
                default,
                self.description
            ),
            None => write!(
                f,
                "- {} ({}, {}): {}",
                self.name,
                self.value_type,
                if self.required { "required" } else { "optional" },
                self.description
            ),
        }
    }
}

impl Display for ToolPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "### {}", self.name)?;
        writeln!(f, "{}", self.description)?;
        if !self.parameters_structured.is_empty() {
            writeln!(f, "Parameters:")?;
            for parameter in &self.parameters_structured {
                writeln!(f, "{}", parameter)?;
            }
        } else if !self.parameters.is_empty() {
            writeln!(f, "Parameters: {}", self.parameters)?;
        }
        if !self.details.is_empty() {
            writeln!(f, "{}", self.details)?;
        }
        if !self.notes.is_empty() {
            writeln!(f, "{}", self.notes)?;
        }
        Ok(())
    }
}

impl Display for SystemToolPromptCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "## {}", self.category_name)?;
        if !self.category_header.is_empty() {
            writeln!(f, "{}", self.category_header)?;
        }
        for tool in &self.tools {
            writeln!(f)?;
            write!(f, "{}", tool)?;
        }
        if !self.category_footer.is_empty() {
            writeln!(f)?;
            write!(f, "{}", self.category_footer)?;
        }
        Ok(())
    }
}

impl SystemToolPrompts {
    #[allow(non_snake_case)]
    pub fn getAIAllCategoriesEn(
        has_backend_image_recognition: bool,
        chat_model_has_direct_image: bool,
        has_backend_audio_recognition: bool,
        has_backend_video_recognition: bool,
        chat_model_has_direct_audio: bool,
        chat_model_has_direct_video: bool,
        saf_bookmark_names: &[String],
    ) -> Vec<SystemToolPromptCategory> {
        let expose_intent = (has_backend_image_recognition && !chat_model_has_direct_image)
            || (has_backend_audio_recognition && !chat_model_has_direct_audio)
            || (has_backend_video_recognition && !chat_model_has_direct_video);
        let mut file_system = file_system_tools_en();
        file_system.tools = adjust_read_file_tool(
            file_system.tools,
            expose_intent,
            false,
            false,
            false,
            buildSafBookmarksSectionEn(saf_bookmark_names),
            "Read the content of a file. For media files, you can also provide an 'intent' parameter to use a backend recognition model for analysis.",
        );
        vec![basic_tools_en(), file_system, http_tools_en(), memory_tools_en()]
    }

    #[allow(non_snake_case)]
    pub fn getAllCategoriesEn(
        has_backend_image_recognition: bool,
        chat_model_has_direct_image: bool,
        has_backend_audio_recognition: bool,
        has_backend_video_recognition: bool,
        chat_model_has_direct_audio: bool,
        chat_model_has_direct_video: bool,
        saf_bookmark_names: &[String],
    ) -> Vec<SystemToolPromptCategory> {
        let mut categories = Self::getAIAllCategoriesEn(
            has_backend_image_recognition,
            chat_model_has_direct_image,
            has_backend_audio_recognition,
            has_backend_video_recognition,
            chat_model_has_direct_audio,
            chat_model_has_direct_video,
            saf_bookmark_names,
        );
        categories.extend(internal_tool_categories_en());
        categories
    }

    #[allow(non_snake_case)]
    pub fn getAIAllCategoriesCn(
        has_backend_image_recognition: bool,
        chat_model_has_direct_image: bool,
        has_backend_audio_recognition: bool,
        has_backend_video_recognition: bool,
        chat_model_has_direct_audio: bool,
        chat_model_has_direct_video: bool,
        saf_bookmark_names: &[String],
    ) -> Vec<SystemToolPromptCategory> {
        let expose_intent = (has_backend_image_recognition && !chat_model_has_direct_image)
            || (has_backend_audio_recognition && !chat_model_has_direct_audio)
            || (has_backend_video_recognition && !chat_model_has_direct_video);
        let mut file_system = file_system_tools_cn();
        file_system.tools = adjust_read_file_tool(
            file_system.tools,
            expose_intent,
            false,
            false,
            false,
            buildSafBookmarksSectionCn(saf_bookmark_names),
            "读取文件内容。对于媒体文件，你也可以提供 intent 参数，使用后端识别模型进行分析。",
        );
        vec![basic_tools_cn(), file_system, http_tools_cn(), memory_tools_cn()]
    }

    #[allow(non_snake_case)]
    pub fn getAllCategoriesCn(
        has_backend_image_recognition: bool,
        chat_model_has_direct_image: bool,
        has_backend_audio_recognition: bool,
        has_backend_video_recognition: bool,
        chat_model_has_direct_audio: bool,
        chat_model_has_direct_video: bool,
        saf_bookmark_names: &[String],
    ) -> Vec<SystemToolPromptCategory> {
        let mut categories = Self::getAIAllCategoriesCn(
            has_backend_image_recognition,
            chat_model_has_direct_image,
            has_backend_audio_recognition,
            has_backend_video_recognition,
            chat_model_has_direct_audio,
            chat_model_has_direct_video,
            saf_bookmark_names,
        );
        categories.extend(internal_tool_categories_cn());
        categories
    }

    #[allow(non_snake_case)]
    pub fn getManageableToolPrompts(use_english: bool) -> Vec<ManageableToolPrompt> {
        let base_categories = if use_english {
            vec![basic_tools_en(), file_system_tools_en(), http_tools_en(), memory_tools_en()]
        } else {
            vec![basic_tools_cn(), file_system_tools_cn(), http_tools_cn(), memory_tools_cn()]
        };
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        for category in base_categories {
            for tool in category.tools {
                if seen.insert(tool.name.clone()) {
                    result.push(ManageableToolPrompt {
                        category_name: category.category_name.clone(),
                        name: tool.name,
                        description: tool.description,
                    });
                }
            }
        }
        result
    }

    #[allow(non_snake_case)]
    pub fn generateMemoryToolsPromptEn(tool_visibility: &HashMap<String, bool>) -> String {
        applyToolVisibility(vec![memory_tools_en()], tool_visibility)
            .first()
            .map(ToString::to_string)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn generateMemoryToolsPromptCn(tool_visibility: &HashMap<String, bool>) -> String {
        applyToolVisibility(vec![memory_tools_cn()], tool_visibility)
            .first()
            .map(ToString::to_string)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn generateToolsPromptEn(
        chat_id: Option<String>,
        has_backend_image_recognition: bool,
        include_memory_tools: bool,
        chat_model_has_direct_image: bool,
        has_backend_audio_recognition: bool,
        has_backend_video_recognition: bool,
        chat_model_has_direct_audio: bool,
        chat_model_has_direct_video: bool,
        saf_bookmark_names: &[String],
        tool_visibility: &HashMap<String, bool>,
        hook_metadata: HashMap<String, Value>,
    ) -> String {
        let mut categories = Self::getAIAllCategoriesEn(
            has_backend_image_recognition,
            chat_model_has_direct_image,
            has_backend_audio_recognition,
            has_backend_video_recognition,
            chat_model_has_direct_audio,
            chat_model_has_direct_video,
            saf_bookmark_names,
        );
        if !include_memory_tools {
            categories.retain(|category| category.category_name != "Memory and Memory Library Tools");
        }
        compose_tool_prompt(chat_id, true, include_memory_tools, categories, tool_visibility, hook_metadata)
    }

    #[allow(non_snake_case)]
    pub fn generateToolsPromptCn(
        chat_id: Option<String>,
        has_backend_image_recognition: bool,
        include_memory_tools: bool,
        chat_model_has_direct_image: bool,
        has_backend_audio_recognition: bool,
        has_backend_video_recognition: bool,
        chat_model_has_direct_audio: bool,
        chat_model_has_direct_video: bool,
        saf_bookmark_names: &[String],
        tool_visibility: &HashMap<String, bool>,
        hook_metadata: HashMap<String, Value>,
    ) -> String {
        let mut categories = Self::getAIAllCategoriesCn(
            has_backend_image_recognition,
            chat_model_has_direct_image,
            has_backend_audio_recognition,
            has_backend_video_recognition,
            chat_model_has_direct_audio,
            chat_model_has_direct_video,
            saf_bookmark_names,
        );
        if !include_memory_tools {
            categories.retain(|category| category.category_name != "记忆与记忆库工具");
        }
        compose_tool_prompt(chat_id, false, include_memory_tools, categories, tool_visibility, hook_metadata)
    }
}

#[allow(non_snake_case)]
fn buildSafBookmarksSectionEn(saf_bookmark_names: &[String]) -> String {
    let names: BTreeSet<String> = saf_bookmark_names.iter().map(|name| name.trim().to_string()).filter(|name| !name.is_empty()).collect();
    if names.is_empty() {
        return String::new();
    }
    let listed = names.into_iter().map(|name| format!("repo:{name}")).collect::<Vec<_>>().join(", ");
    format!(
        "\n\n**Attached Local Storage Repository:**\n- environment (optional): you can also use `environment=\"repo:<repositoryName>\"` to operate in an attached local storage repository.\n- Paths are absolute (e.g., `/`, `/work/index.html`).\n- Available repositories: {listed}"
    )
}

#[allow(non_snake_case)]
fn buildSafBookmarksSectionCn(saf_bookmark_names: &[String]) -> String {
    let names: BTreeSet<String> = saf_bookmark_names.iter().map(|name| name.trim().to_string()).filter(|name| !name.is_empty()).collect();
    if names.is_empty() {
        return String::new();
    }
    let listed = names.into_iter().map(|name| format!("repo:{name}")).collect::<Vec<_>>().join("、");
    format!(
        "\n\n**附加本地储存仓库：**\n- environment（可选）：也可以使用 `environment=\"repo:<仓库名>\"` 在附加本地储存仓库中操作。\n- 路径使用绝对路径（例如 `/`、`/work/index.html`）。\n- 当前可用仓库：{listed}"
    )
}

#[allow(non_snake_case)]
fn applyToolVisibility(categories: Vec<SystemToolPromptCategory>, tool_visibility: &HashMap<String, bool>) -> Vec<SystemToolPromptCategory> {
    if tool_visibility.is_empty() {
        return categories;
    }
    categories
        .into_iter()
        .filter_map(|mut category| {
            category.tools.retain(|tool| tool_visibility.get(&tool.name).copied().unwrap_or(true));
            if category.tools.is_empty() {
                None
            } else {
                Some(category)
            }
        })
        .collect()
}

fn compose_tool_prompt(
    chat_id: Option<String>,
    use_english: bool,
    include_memory_tools: bool,
    categories: Vec<SystemToolPromptCategory>,
    tool_visibility: &HashMap<String, bool>,
    hook_metadata: HashMap<String, Value>,
) -> String {
    let visible_categories = applyToolVisibility(categories, tool_visibility);
    let available_tools = buildToolHookPayload(&visible_categories);
    let mut metadata = HashMap::from([
        ("includeMemoryTools".to_string(), json!(include_memory_tools)),
        ("toolVisibility".to_string(), json!(tool_visibility)),
    ]);
    metadata.extend(hook_metadata);

    let before_context = PromptHookRegistry::dispatchToolPromptComposeHooks(PromptHookContext {
        stage: "before_compose_tool_prompt".to_string(),
        chat_id: chat_id.clone(),
        function_type: None,
        prompt_function_type: None,
        use_english: Some(use_english),
        raw_input: None,
        processed_input: None,
        chat_history: Vec::new(),
        prepared_history: Vec::new(),
        system_prompt: None,
        tool_prompt: None,
        model_parameters: Vec::new(),
        available_tools,
        metadata,
    });
    let mut prompt = before_context
        .tool_prompt
        .clone()
        .unwrap_or_else(|| renderToolPromptFromAvailableTools(&before_context.available_tools));
    let filter_context = PromptHookRegistry::dispatchToolPromptComposeHooks(PromptHookContext {
        stage: "filter_tool_prompt_items".to_string(),
        tool_prompt: Some(prompt),
        ..before_context
    });
    prompt = filter_context
        .tool_prompt
        .clone()
        .unwrap_or_else(|| renderToolPromptFromAvailableTools(&filter_context.available_tools));
    let after_context = PromptHookRegistry::dispatchToolPromptComposeHooks(PromptHookContext {
        stage: "after_compose_tool_prompt".to_string(),
        tool_prompt: Some(prompt),
        ..filter_context
    });
    let after_available_tools = after_context.available_tools.clone();
    after_context
        .tool_prompt
        .unwrap_or_else(|| renderToolPromptFromAvailableTools(&after_available_tools))
}

#[allow(non_snake_case)]
fn buildToolHookPayload(categories: &[SystemToolPromptCategory]) -> Vec<HashMap<String, Value>> {
    categories
        .iter()
        .flat_map(|category| {
            category.tools.iter().map(move |tool| {
                HashMap::from([
                    ("categoryName".to_string(), json!(category.category_name)),
                    ("categoryHeader".to_string(), json!(category.category_header)),
                    ("categoryFooter".to_string(), json!(category.category_footer)),
                    ("name".to_string(), json!(tool.name)),
                    ("description".to_string(), json!(tool.description)),
                    ("parameters".to_string(), json!(tool.parameters)),
                    ("details".to_string(), json!(tool.details)),
                    ("notes".to_string(), json!(tool.notes)),
                    ("parametersStructured".to_string(), json!(tool.parameters_structured)),
                ])
            })
        })
        .collect()
}

#[allow(non_snake_case)]
fn renderToolPromptFromAvailableTools(available_tools: &[HashMap<String, Value>]) -> String {
    if available_tools.is_empty() {
        return String::new();
    }
    buildToolPromptCategories(available_tools)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[allow(non_snake_case)]
fn buildToolPromptCategories(available_tools: &[HashMap<String, Value>]) -> Vec<SystemToolPromptCategory> {
    let mut categories: Vec<SystemToolPromptCategory> = Vec::new();
    for item in available_tools {
        let category_name = string_field(item, "categoryName");
        let tool_name = string_field(item, "name");
        let description = string_field(item, "description");
        let category_index = categories.iter().position(|category| category.category_name == category_name);
        let tool = ToolPrompt {
            name: tool_name,
            description,
            parameters: string_field(item, "parameters"),
            parameters_structured: parseToolParameterSchemas(item.get("parametersStructured")),
            details: string_field(item, "details"),
            notes: string_field(item, "notes"),
        };
        match category_index {
            Some(index) => categories[index].tools.push(tool),
            None => categories.push(SystemToolPromptCategory {
                category_name,
                category_header: string_field(item, "categoryHeader"),
                tools: vec![tool],
                category_footer: string_field(item, "categoryFooter"),
            }),
        }
    }
    categories
}

#[allow(non_snake_case)]
fn parseToolParameterSchemas(value: Option<&Value>) -> Vec<ToolParameterSchema> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| serde_json::from_value::<ToolParameterSchema>(item.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

fn string_field(item: &HashMap<String, Value>, key: &str) -> String {
    item.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn category(name: &str, tools: Vec<ToolPrompt>) -> SystemToolPromptCategory {
    SystemToolPromptCategory {
        category_name: name.to_string(),
        category_header: String::new(),
        tools,
        category_footer: String::new(),
    }
}

fn tool(name: &str, description: &str, parameters: Vec<ToolParameterSchema>) -> ToolPrompt {
    ToolPrompt {
        name: name.to_string(),
        description: description.to_string(),
        parameters: String::new(),
        parameters_structured: parameters,
        details: String::new(),
        notes: String::new(),
    }
}

fn param(name: &str, value_type: &str, description: &str, required: bool, default: Option<&str>) -> ToolParameterSchema {
    ToolParameterSchema {
        name: name.to_string(),
        value_type: value_type.to_string(),
        description: description.to_string(),
        required,
        default: default.map(ToOwned::to_owned),
    }
}

fn basic_tools_en() -> SystemToolPromptCategory {
    category("Available tools", vec![
        tool("sleep", "Demonstration tool that pauses briefly.", vec![param("duration_ms", "integer", "milliseconds, default 1000, >= 0", false, Some("1000"))]),
        tool("use_package", "Activate a package for use in the current session.", vec![param("package_name", "string", "name of the package to activate", true, None)]),
    ])
}

fn basic_tools_cn() -> SystemToolPromptCategory {
    category("可用工具", vec![
        tool("sleep", "演示工具，短暂暂停。", vec![param("duration_ms", "integer", "毫秒，默认1000，>= 0", false, Some("1000"))]),
        tool("use_package", "在当前会话中激活包。", vec![param("package_name", "string", "要激活的包名", true, None)]),
    ])
}

fn file_system_tools_en() -> SystemToolPromptCategory {
    category("File System Tools", vec![
        tool("list_files", "List files in a directory.", vec![param("path", "string", "e.g. \"/sdcard/Download\"", true, None), param("environment", "string", "optional, same as read_file environment", false, None)]),
        tool("read_file", "Read the content of a file. For image files (jpg, jpeg, png, gif, bmp), it automatically extracts text using OCR.", vec![
            param("path", "string", "file path", true, None),
            param("environment", "string", "optional, execution environment. Values: \"android\" (default, Android file system) | \"linux\" (local Ubuntu 24 terminal environment via proot; Linux paths like /home/... /etc/hosts) | \"repo:<repositoryName>\" (attached local storage repository)", false, None),
            param("intent", "string", "optional, your question about the media/file (used for backend recognition)", false, None),
            param("direct_image", "boolean", "optional, when true: return an <link type=\"image\"> tag for models that support vision", false, None),
            param("direct_audio", "boolean", "optional, when true: return an <link type=\"audio\"> tag for models that support audio", false, None),
            param("direct_video", "boolean", "optional, when true: return an <link type=\"video\"> tag for models that support video", false, None),
        ]),
        tool("read_file_part", "Read file content by line range.", vec![param("path", "string", "file path", true, None), param("environment", "string", "optional, same as read_file environment", false, None), param("start_line", "integer", "starting line number, 1-indexed", false, Some("1")), param("end_line", "integer", "ending line number, 1-indexed, inclusive, optional", false, Some("start_line + 99"))]),
        tool("create_file", "Create a new file by delegating to apply_file with type=create.", vec![param("path", "string", "file path", true, None), param("new", "string", "full file content for the new file", true, None), param("environment", "string", "optional, same as read_file environment", false, None)]),
        tool("edit_file", "Edit an existing file by delegating to apply_file with type=replace.", vec![param("path", "string", "file path", true, None), param("old", "string", "the exact content to be matched and replaced", true, None), param("new", "string", "the new content to insert", true, None), param("environment", "string", "optional, same as read_file environment", false, None)]),
        tool("delete_file", "Delete a file or directory.", vec![param("path", "string", "target path", true, None), param("environment", "string", "optional, same as read_file environment", false, None), param("recursive", "boolean", "boolean", false, Some("false"))]),
        tool("make_directory", "Create a directory.", vec![param("path", "string", "directory path", true, None), param("environment", "string", "optional, same as read_file environment", false, None), param("create_parents", "boolean", "boolean", false, Some("false"))]),
        tool("find_files", "Search for files matching a pattern.", vec![param("path", "string", "search path, for Android use /sdcard/..., for Linux use /home/... or /etc/...", true, None), param("environment", "string", "optional, same as read_file environment", false, None), param("pattern", "string", "search pattern, e.g. \"*.jpg\"", true, None), param("max_depth", "integer", "optional, controls depth of subdirectory search, -1=unlimited", false, None), param("use_path_pattern", "boolean", "boolean", false, Some("false")), param("case_insensitive", "boolean", "boolean", false, Some("false"))]),
        tool("grep_code", "Search code content matching a regex pattern in files. Returns matches with surrounding context lines.", vec![param("path", "string", "search path", true, None), param("environment", "string", "optional, same as read_file environment", false, None), param("pattern", "string", "regex pattern", true, None), param("file_pattern", "string", "file filter", false, Some("\"*\"")), param("case_insensitive", "boolean", "boolean", false, Some("false")), param("context_lines", "integer", "lines of context before/after match", false, Some("3")), param("max_results", "integer", "max matches", false, Some("100"))]),
        tool("grep_context", "Search for relevant content based on intent/context understanding. Supports directory and file modes. Uses semantic relevance scoring.", vec![param("path", "string", "directory or file path", true, None), param("environment", "string", "optional, same as read_file environment", false, None), param("intent", "string", "intent or context description string", true, None), param("file_pattern", "string", "file filter for directory mode", false, Some("\"*\"")), param("max_results", "integer", "maximum items to return", false, Some("10"))]),
        tool("download_file", "Download a file from the internet. Two modes: url+destination, or visit_key plus link_number/image_number plus destination.", vec![param("url", "string", "optional, file URL", false, None), param("visit_key", "string", "optional, visitKey from a previous visit_web result", false, None), param("link_number", "integer", "optional, 1-based link index from Results", false, None), param("image_number", "integer", "optional, 1-based image index from Images", false, None), param("destination", "string", "save path", true, None), param("environment", "string", "optional, same as read_file environment", false, None), param("headers", "string", "optional HTTP headers as JSON object string", false, None)]),
    ])
}

fn file_system_tools_cn() -> SystemToolPromptCategory {
    category("文件系统工具", vec![
        tool("list_files", "列出目录中的文件。", vec![param("path", "string", "例如\"/sdcard/Download\"", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None)]),
        tool("read_file", "读取文件内容。对于图片文件(jpg, jpeg, png, gif, bmp)，自动使用OCR提取文本。", vec![
            param("path", "string", "文件路径", true, None),
            param("environment", "string", "可选，执行环境。取值：\"android\"（默认，Android文件系统）| \"linux\"（本地Ubuntu 24终端环境，通过proot实现）| \"repo:<仓库名>\"（附加本地储存仓库）", false, None),
            param("intent", "string", "可选，用户对媒体/文件的问题（用于后端识别模型）", false, None),
            param("direct_image", "boolean", "可选，为true时：返回<link type=\"image\">标签供支持识图的模型直接查看", false, None),
            param("direct_audio", "boolean", "可选，为true时：返回<link type=\"audio\">标签供支持音频的模型直接处理", false, None),
            param("direct_video", "boolean", "可选，为true时：返回<link type=\"video\">标签供支持视频的模型直接处理", false, None),
        ]),
        tool("read_file_part", "按行号范围读取文件内容。", vec![param("path", "string", "文件路径", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None), param("start_line", "integer", "起始行号，从1开始", false, Some("1")), param("end_line", "integer", "结束行号，从1开始，包括该行，可选", false, Some("start_line + 99"))]),
        tool("create_file", "通过委托给 apply_file 且 type=create 来创建新文件。", vec![param("path", "string", "文件路径", true, None), param("new", "string", "新文件的完整内容", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None)]),
        tool("edit_file", "通过委托给 apply_file 且 type=replace 来编辑已存在文件。", vec![param("path", "string", "文件路径", true, None), param("old", "string", "用于匹配并替换的原始内容", true, None), param("new", "string", "要插入的新内容", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None)]),
        tool("delete_file", "删除文件或目录。", vec![param("path", "string", "目标路径", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None), param("recursive", "boolean", "布尔值", false, Some("false"))]),
        tool("make_directory", "创建目录。", vec![param("path", "string", "目录路径", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None), param("create_parents", "boolean", "布尔值", false, Some("false"))]),
        tool("find_files", "搜索匹配模式的文件。", vec![param("path", "string", "搜索路径，Android用/sdcard/...，Linux用/home/...或/etc/...", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None), param("pattern", "string", "搜索模式，例如\"*.jpg\"", true, None), param("max_depth", "integer", "可选，控制子目录搜索深度，-1=无限", false, None), param("use_path_pattern", "boolean", "布尔值", false, Some("false")), param("case_insensitive", "boolean", "布尔值", false, Some("false"))]),
        tool("grep_code", "在文件中搜索匹配正则表达式的代码内容，返回带上下文的匹配结果。", vec![param("path", "string", "搜索路径", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None), param("pattern", "string", "正则表达式模式", true, None), param("file_pattern", "string", "文件过滤", false, Some("\"*\"")), param("case_insensitive", "boolean", "布尔值", false, Some("false")), param("context_lines", "integer", "匹配行前后的上下文行数", false, Some("3")), param("max_results", "integer", "最大匹配数", false, Some("100"))]),
        tool("grep_context", "基于意图/上下文理解搜索相关内容。支持目录模式和文件模式，使用语义相关性评分。", vec![param("path", "string", "目录或文件路径", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None), param("intent", "string", "意图或上下文描述字符串", true, None), param("file_pattern", "string", "目录模式下的文件过滤", false, Some("\"*\"")), param("max_results", "integer", "返回的最大项数", false, Some("10"))]),
        tool("download_file", "从互联网下载文件。有两种用法：url+destination，或 visit_key 加 link_number/image_number 加 destination。", vec![param("url", "string", "可选, 文件URL", false, None), param("visit_key", "string", "可选, 上一次 visit_web 返回的 visitKey", false, None), param("link_number", "integer", "可选, Results 中的链接编号（从1开始）", false, None), param("image_number", "integer", "可选, Images 中的图片编号（从1开始）", false, None), param("destination", "string", "保存路径", true, None), param("environment", "string", "可选，同 read_file 的 environment", false, None), param("headers", "string", "可选：HTTP请求头，JSON对象字符串", false, None)]),
    ])
}

fn http_tools_en() -> SystemToolPromptCategory {
    category("HTTP Tools", vec![tool("visit_web", "Visit a webpage and extract information (including optional image links). Supports url mode and visit_key + link_number continuation mode.", vec![param("url", "string", "optional, webpage URL", false, None), param("visit_key", "string", "optional, string, the visitKey from a previous visit_web result", false, None), param("link_number", "integer", "optional, int, 1-based index of the link to follow", false, None), param("include_image_links", "boolean", "optional, boolean, when true include extracted image links", false, None), param("headers", "string", "optional HTTP headers as JSON object string", false, None), param("user_agent_preset", "string", "optional, quick select user agent: desktop/android", false, None), param("user_agent", "string", "optional, full custom user agent override", false, None)])])
}

fn http_tools_cn() -> SystemToolPromptCategory {
    category("HTTP工具", vec![tool("visit_web", "访问网页并提取信息（可选包含图片链接）。支持 url 访问新页面，也支持 visit_key + link_number 继续访问结果链接。", vec![param("url", "string", "可选, 网页URL", false, None), param("visit_key", "string", "可选, 字符串, 上一次 visit_web 返回的 visitKey", false, None), param("link_number", "integer", "可选, 整数, 要继续访问的链接编号（从1开始）", false, None), param("include_image_links", "boolean", "可选, boolean, 为 true 时额外包含图片链接列表", false, None), param("headers", "string", "可选：HTTP请求头，JSON对象字符串", false, None), param("user_agent_preset", "string", "可选：UA预设，快速选择：desktop/android", false, None), param("user_agent", "string", "可选：完整自定义UA", false, None)])])
}

fn memory_tools_en() -> SystemToolPromptCategory {
    let mut category = category("Memory and Memory Library Tools", vec![
        tool("query_memory", "Searches the memory library for relevant memories and document chunks.", vec![param("query", "string", "the search query", true, None), param("folder_path", "string", "optional, the specific folder path to search within", false, None), param("start_time", "string", "optional, local-time string in YYYY-MM-DD or YYYY-MM-DD HH:mm format", false, None), param("end_time", "string", "optional, local-time string in YYYY-MM-DD or YYYY-MM-DD HH:mm format", false, None), param("snapshot_id", "string", "optional, reusable snapshot id", false, None), param("threshold", "number", "optional, number >= 0", false, Some("0")), param("limit", "integer", "optional, maximum number of results", false, Some("20"))]),
        tool("get_memory_by_title", "Retrieves a memory by exact title, including document content or selected chunks.", vec![param("title", "string", "required, the exact title of the memory", true, None), param("chunk_index", "integer", "optional, read a specific chunk by its number", false, None), param("chunk_range", "string", "optional, read a range of chunks in start-end format", false, None), param("query", "string", "optional, search inside the document", false, None), param("limit", "integer", "optional, maximum number of chunks", false, Some("20"))]),
    ]);
    category.category_footer = "\nNote: The memory library and user personality profile may be updated automatically after the current reply is finalized. If you need to manage memories immediately or update user preferences, use the appropriate tools directly.".to_string();
    category
}

fn memory_tools_cn() -> SystemToolPromptCategory {
    let mut category = category("记忆与记忆库工具", vec![
        tool("query_memory", "从记忆库中搜索相关记忆和文档分块。", vec![param("query", "string", "搜索查询", true, None), param("folder_path", "string", "可选, 要搜索的特定文件夹路径", false, None), param("start_time", "string", "可选, 本地时间字符串，格式支持 YYYY-MM-DD 或 YYYY-MM-DD HH:mm", false, None), param("end_time", "string", "可选, 本地时间字符串，格式支持 YYYY-MM-DD 或 YYYY-MM-DD HH:mm", false, None), param("snapshot_id", "string", "可选, 可复用快照 id", false, None), param("threshold", "number", "可选, number >= 0", false, Some("0")), param("limit", "integer", "可选, 返回结果的最大数量", false, Some("20"))]),
        tool("get_memory_by_title", "通过精确标题检索记忆，可读取完整内容或文档分块。", vec![param("title", "string", "必需, 记忆的精确标题", true, None), param("chunk_index", "integer", "可选, 读取特定编号的分块", false, None), param("chunk_range", "string", "可选, 读取分块范围，格式为 起始-结束", false, None), param("query", "string", "可选, 在文档内搜索", false, None), param("limit", "integer", "可选, 最大分块数量", false, Some("20"))]),
    ]);
    category.category_footer = "\n注意：记忆库和用户人格画像可能会在当前回复完成后自动更新。若需要立即管理记忆或更新用户偏好，请直接使用对应工具。".to_string();
    category
}

fn internal_tool_categories_en() -> Vec<SystemToolPromptCategory> {
    vec![category("Internal Tools", vec![tool("package_proxy", "Call a tool provided by an activated package.", vec![param("tool_name", "string", "actual package tool name", true, None), param("params", "object", "target tool arguments as JSON object", true, None)])])]
}

fn internal_tool_categories_cn() -> Vec<SystemToolPromptCategory> {
    vec![category("内部工具", vec![tool("package_proxy", "调用已激活包提供的工具。", vec![param("tool_name", "string", "真实包工具名", true, None), param("params", "object", "目标工具参数 JSON 对象", true, None)])])]
}

fn adjust_read_file_tool(
    tools: Vec<ToolPrompt>,
    expose_intent: bool,
    expose_direct_image: bool,
    expose_direct_audio: bool,
    expose_direct_video: bool,
    saf_bookmarks_section: String,
    adjusted_description: &str,
) -> Vec<ToolPrompt> {
    tools
        .into_iter()
        .map(|mut tool| {
            if tool.name == "read_file" {
                tool.parameters_structured.retain(|parameter| match parameter.name.as_str() {
                    "intent" => expose_intent,
                    "direct_image" => expose_direct_image,
                    "direct_audio" => expose_direct_audio,
                    "direct_video" => expose_direct_video,
                    _ => true,
                });
                if expose_intent {
                    tool.description = format!("{adjusted_description}{saf_bookmarks_section}");
                } else {
                    tool.description = format!("{}{}", tool.description, saf_bookmarks_section);
                }
            }
            tool
        })
        .collect()
}
