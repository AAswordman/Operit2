use std::collections::HashMap;

use operit_host_api::HostEnvironmentDescriptor;
use operit_tools::files::PathMapper::ResolvedVfsPath;
use serde_json::{json, Value};

use crate::chat::config::SystemToolPrompts::SystemToolPrompts;
use crate::chat::hooks::PromptHookRegistry::{PromptHookContext, PromptHookRegistry};
use operit_tools::tools::climode::CliToolModeSupport::CliToolModeSupport;

const TOOL_USAGE_GUIDELINES_EN: &str = r#"When calling a tool, the user will see your response, and then will automatically send the tool results back to you in a follow-up message.

To use a tool, use this format in your response:

<tool name="tool_name">
<param name="parameter_name">parameter_value</param>
</tool>

When outputting XML (e.g., <tool>), insert a newline before it and ensure the opening tag starts at the beginning of a line.

Based on user needs, proactively select the most appropriate tool or combination of tools. For complex tasks, you can break down the problem and use different tools step by step to solve it. After using each tool, clearly explain the execution results and suggest the next steps."#;

const TOOL_USAGE_GUIDELINES_CN: &str = r#"调用工具时，用户会看到你的响应，然后会自动将工具结果发送回给你。

使用工具时，请使用以下格式：

<tool name="tool_name">
<param name="parameter_name">parameter_value</param>
</tool>

输出XML（如 <tool>）时，必须在XML前换行，并确保起始标签位于行首。

根据用户需求，主动选择最合适的工具或工具组合。对于复杂任务，你可以分解问题并使用不同的工具逐步解决。使用每个工具后，清楚地解释执行结果并建议下一步。"#;

const PACKAGE_SYSTEM_GUIDELINES_EN: &str = r#"PACKAGE SYSTEM
- Some additional functionality is available through packages
- To use a package, simply activate it with:
  <tool name="use_package">
  <param name="package_name">package_name_here</param>
  </tool>
- This will show you all the tools in the package and how to use them
- Only after activating a package, you can use its tools directly"#;

const PACKAGE_SYSTEM_GUIDELINES_CN: &str = r#"包系统：
- 一些额外功能通过包提供
- 要使用包，只需激活它：
  <tool name="use_package">
  <param name="package_name">package_name_here</param>
  </tool>
- 这将显示包中的所有工具及其使用方法
- 只有在激活包后，才能直接使用其工具"#;

const PACKAGE_SYSTEM_GUIDELINES_TOOL_CALL_EN: &str = r#"PACKAGE SYSTEM
- Some additional functionality is available through packages
- To use a package, call the use_package function with the package_name parameter
- If use_package for a package has appeared earlier in this chat, treat that package as activated
- For package tools, call package_proxy:
  - Set tool_name to the actual package tool name (e.g. packageName:toolName)
  - Put target tool arguments in params as a JSON object"#;

const PACKAGE_SYSTEM_GUIDELINES_TOOL_CALL_CN: &str = r#"包系统：
- 一些额外功能通过包提供
- 要使用包，调用 use_package 函数并传入 package_name 参数
- 只要本次聊天中该包曾出现过 use_package，就视为该包已激活
- 调用包工具请使用 package_proxy：
  - tool_name 填写真实工具名（例如 packageName:toolName）
  - 将目标工具参数放入 params（JSON对象）"#;

pub const SYSTEM_PROMPT_TEMPLATE: &str = r#"BEGIN_SELF_INTRODUCTION_SECTION

WORKSPACE_GUIDELINES_SECTION

TOOL_USAGE_GUIDELINES_SECTION

PACKAGE_SYSTEM_GUIDELINES_SECTION

ACTIVE_PACKAGES_SECTION

AVAILABLE_TOOLS_SECTION"#;

pub const SYSTEM_PROMPT_TEMPLATE_CN: &str = r#"BEGIN_SELF_INTRODUCTION_SECTION

WORKSPACE_GUIDELINES_SECTION

TOOL_USAGE_GUIDELINES_SECTION

PACKAGE_SYSTEM_GUIDELINES_SECTION

ACTIVE_PACKAGES_SECTION

AVAILABLE_TOOLS_SECTION"#;

pub const SUBTASK_AGENT_PROMPT_TEMPLATE: &str = r#"BEHAVIOR GUIDELINES:
- You are a subtask-focused AI agent. Your only goal is to complete the assigned task efficiently and accurately.
- You have no memory of past conversations, user preferences, or personality. You must not exhibit any emotion or personality.
- **TOOL SCHEDULING**: All tools may be called either in parallel or sequentially. Choose whichever best fits the task. The tool system will decide and handle execution conflicts automatically.
- **Summarize and Conclude**: If the task requires using tools to gather information (e.g., reading files, searching), you **MUST** process that information and provide a concise, conclusive summary as your final output. Do not output raw data. Your final answer is the only thing passed to the next agent.
- Be concise and factual. Avoid lengthy explanations.

TOOL_USAGE_GUIDELINES_SECTION

PACKAGE_SYSTEM_GUIDELINES_SECTION

ACTIVE_PACKAGES_SECTION

AVAILABLE_TOOLS_SECTION"#;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolExposureMode {
    FULL,
    CLI,
}

#[derive(Clone, Debug, Default)]
pub struct PackageInfo {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Default)]
pub struct WorkspaceRuleFile {
    pub name: String,
    pub content: String,
}

/// Paths returned by the selected terminal Host, including mapping failures.
#[derive(Clone, Debug)]
pub struct TerminalPathMapping {
    pub terminal: String,
    pub terminal_type: String,
    pub vfs_path: String,
    pub path: Result<String, String>,
}

#[derive(Clone, Debug)]
pub struct SystemPromptOptions {
    pub chat_id: Option<String>,
    pub workspace_path: Option<String>,
    pub output_path: Option<String>,
    pub workspace_folders: Vec<String>,
    /// VFS roots mapped to absolute host paths by the active file-tool mapper.
    pub workspace_path_mappings: Vec<ResolvedVfsPath>,
    pub terminal_path_mappings: Vec<TerminalPathMapping>,
    pub saf_bookmark_names: Vec<String>,
    pub use_english: bool,
    pub custom_system_prompt_template: String,
    pub enable_tools: bool,
    pub has_image_recognition: bool,
    pub chat_model_has_direct_image: bool,
    pub has_audio_recognition: bool,
    pub has_video_recognition: bool,
    pub chat_model_has_direct_audio: bool,
    pub chat_model_has_direct_video: bool,
    pub use_tool_call_api: bool,
    pub tool_exposure_mode: ToolExposureMode,
    pub tool_visibility: HashMap<String, bool>,
    pub enabled_packages: Vec<PackageInfo>,
    pub mcp_servers: Vec<PackageInfo>,
    pub skill_packages: Vec<PackageInfo>,
    pub workspace_rule_file: Option<WorkspaceRuleFile>,
    pub external_storage_path: String,
    pub app_files_path: String,
    pub host_environment: HostEnvironmentDescriptor,
    pub hook_metadata: HashMap<String, Value>,
}

impl Default for SystemPromptOptions {
    fn default() -> Self {
        Self {
            chat_id: None,
            workspace_path: None,
            output_path: None,
            workspace_folders: Vec::new(),
            workspace_path_mappings: Vec::new(),
            terminal_path_mappings: Vec::new(),
            saf_bookmark_names: Vec::new(),
            use_english: false,
            custom_system_prompt_template: String::new(),
            enable_tools: true,
            has_image_recognition: false,
            chat_model_has_direct_image: false,
            has_audio_recognition: false,
            has_video_recognition: false,
            chat_model_has_direct_audio: false,
            chat_model_has_direct_video: false,
            use_tool_call_api: false,
            tool_exposure_mode: ToolExposureMode::FULL,
            tool_visibility: HashMap::new(),
            enabled_packages: Vec::new(),
            mcp_servers: Vec::new(),
            skill_packages: Vec::new(),
            workspace_rule_file: None,
            external_storage_path: "/sdcard".to_string(),
            app_files_path: String::new(),
            host_environment: HostEnvironmentDescriptor::android(),
            hook_metadata: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SystemPromptWithCustomOptions {
    pub base: SystemPromptOptions,
    pub custom_intro_prompt: String,
}

pub struct SystemPromptConfig;

impl SystemPromptConfig {
    #[allow(non_snake_case)]
    pub fn applyCustomPrompts(system_prompt: &str, custom_intro_prompt: &str) -> String {
        system_prompt.replace("BEGIN_SELF_INTRODUCTION_SECTION", custom_intro_prompt)
    }

    #[allow(non_snake_case)]
    /// Stops prompt construction on a registered tool-policy failure.
    pub async fn getSystemPrompt(options: SystemPromptOptions) -> Result<String, String> {
        let package_system_visible = options.tool_exposure_mode == ToolExposureMode::FULL
            && options.enable_tools
            && options
                .tool_visibility
                .get("use_package")
                .copied()
                .unwrap_or(true);
        let mut packages_section = String::new();
        let has_packages = package_system_visible
            && (!options.enabled_packages.is_empty()
                || !options.mcp_servers.is_empty()
                || !options.skill_packages.is_empty());

        if has_packages {
            packages_section.push_str("Available packages:\n");
            for package in options
                .enabled_packages
                .iter()
                .chain(options.mcp_servers.iter())
                .chain(options.skill_packages.iter())
            {
                if package.description.is_empty() {
                    packages_section.push_str(&format!("- {}\n", package.name));
                } else {
                    packages_section
                        .push_str(&format!("- {} : {}\n", package.name, package.description));
                }
            }
        } else if package_system_visible {
            packages_section.push_str("No packages are currently available.\n");
        }

        if package_system_visible && !options.use_tool_call_api {
            packages_section.push('\n');
            packages_section.push_str("To use a package:\n");
            packages_section.push_str("<tool name=\"use_package\"><param name=\"package_name\">package_name_here</param></tool>\n");
        }

        let template_to_use = if !options.custom_system_prompt_template.is_empty() {
            options.custom_system_prompt_template.clone()
        } else if options.use_english {
            SYSTEM_PROMPT_TEMPLATE.to_string()
        } else {
            SYSTEM_PROMPT_TEMPLATE_CN.to_string()
        };

        let workspace_guidelines = getWorkspaceGuidelines(
            options.workspace_path.as_deref(),
            options.output_path.as_deref(),
            &options.workspace_folders,
            &options.terminal_path_mappings,
            options.use_english,
            options.workspace_rule_file.as_ref(),
        );

        let mut prompt = template_to_use
            .replace(
                "ACTIVE_PACKAGES_SECTION",
                if options.enable_tools {
                    &packages_section
                } else {
                    ""
                },
            )
            .replace("WORKSPACE_GUIDELINES_SECTION", &workspace_guidelines);

        let available_tools =
            if options.use_tool_call_api || options.tool_exposure_mode == ToolExposureMode::CLI {
                String::new()
            } else if options.use_english {
                SystemToolPrompts::generateToolsPromptEnForHost(
                    options.chat_id.clone(),
                    options.has_image_recognition,
                    options.chat_model_has_direct_image,
                    options.has_audio_recognition,
                    options.has_video_recognition,
                    options.chat_model_has_direct_audio,
                    options.chat_model_has_direct_video,
                    &options.saf_bookmark_names,
                    &options.host_environment,
                    &options.tool_visibility,
                    options.hook_metadata.clone(),
                )
                .await?
            } else {
                SystemToolPrompts::generateToolsPromptCnForHost(
                    options.chat_id.clone(),
                    options.has_image_recognition,
                    options.chat_model_has_direct_image,
                    options.has_audio_recognition,
                    options.has_video_recognition,
                    options.chat_model_has_direct_audio,
                    options.chat_model_has_direct_video,
                    &options.saf_bookmark_names,
                    &options.host_environment,
                    &options.tool_visibility,
                    options.hook_metadata.clone(),
                )
                .await?
            };

        if options.enable_tools {
            if options.tool_exposure_mode == ToolExposureMode::CLI {
                prompt = prompt
                    .replace(
                        "TOOL_USAGE_GUIDELINES_SECTION",
                        &build_cli_mode_prompt(options.use_english),
                    )
                    .replace("PACKAGE_SYSTEM_GUIDELINES_SECTION", "")
                    .replace("ACTIVE_PACKAGES_SECTION", "")
                    .replace("AVAILABLE_TOOLS_SECTION", "");
            } else if options.use_tool_call_api {
                let package_guidelines = if options.use_english {
                    PACKAGE_SYSTEM_GUIDELINES_TOOL_CALL_EN
                } else {
                    PACKAGE_SYSTEM_GUIDELINES_TOOL_CALL_CN
                };
                prompt = prompt
                    .replace("TOOL_USAGE_GUIDELINES_SECTION", "")
                    .replace(
                        "PACKAGE_SYSTEM_GUIDELINES_SECTION",
                        if package_system_visible {
                            package_guidelines
                        } else {
                            ""
                        },
                    )
                    .replace("AVAILABLE_TOOLS_SECTION", "");
            } else {
                prompt = prompt
                    .replace(
                        "TOOL_USAGE_GUIDELINES_SECTION",
                        if options.use_english {
                            TOOL_USAGE_GUIDELINES_EN
                        } else {
                            TOOL_USAGE_GUIDELINES_CN
                        },
                    )
                    .replace(
                        "PACKAGE_SYSTEM_GUIDELINES_SECTION",
                        if package_system_visible {
                            if options.use_english {
                                PACKAGE_SYSTEM_GUIDELINES_EN
                            } else {
                                PACKAGE_SYSTEM_GUIDELINES_CN
                            }
                        } else {
                            ""
                        },
                    )
                    .replace(
                        "AVAILABLE_TOOLS_SECTION",
                        &available_tools,
                    );
            }
        } else {
            prompt = prompt
                .replace("TOOL_USAGE_GUIDELINES_SECTION", "")
                .replace("PACKAGE_SYSTEM_GUIDELINES_SECTION", "")
                .replace("AVAILABLE_TOOLS_SECTION", "")
                .replace(&workspace_guidelines, "");
        }

        // Custom templates may omit the placeholder, but workspace context is
        // still required to use tools correctly.
        if options.enable_tools
            && !workspace_guidelines.is_empty()
            && !template_to_use.contains("WORKSPACE_GUIDELINES_SECTION")
        {
            prompt.push_str("\n\n");
            prompt.push_str(&workspace_guidelines);
        }

        if options.enable_tools {
            prompt.push_str("\n\n");
            prompt.push_str(getAttachmentGuidelines(options.use_english));
            prompt.push_str("\n\n");
            prompt.push_str(&getFileSyncGuidelines(options.use_english));
        }

        Ok(collapse_blank_lines(&prompt))
    }

    #[allow(non_snake_case)]
    /// Propagates tool-policy failure through custom system-prompt construction.
    pub async fn getSystemPromptWithCustomPrompts(
        options: SystemPromptWithCustomOptions,
    ) -> Result<String, String> {
        let mut metadata = HashMap::from([
            (
                "workspacePath".to_string(),
                json!(options.base.workspace_path),
            ),
            (
                "workspaceFolders".to_string(),
                json!(options.base.workspace_folders),
            ),
            (
                "workspacePathMappings".to_string(),
                json!(options
                    .base
                    .workspace_path_mappings
                    .iter()
                    .map(|mapping| json!({
                        "vfsPath": mapping.vfsPath,
                        "physicalPath": mapping.physicalPath,
                    }))
                    .collect::<Vec<_>>()),
            ),
            (
                "hostEnvironment".to_string(),
                json!(options.base.host_environment.id.clone()),
            ),
            (
                "safBookmarkNames".to_string(),
                json!(options.base.saf_bookmark_names),
            ),
            (
                "customSystemPromptTemplate".to_string(),
                json!(options.base.custom_system_prompt_template),
            ),
            (
                "customIntroPrompt".to_string(),
                json!(options.custom_intro_prompt),
            ),
            ("enableTools".to_string(), json!(options.base.enable_tools)),
            (
                "hasImageRecognition".to_string(),
                json!(options.base.has_image_recognition),
            ),
            (
                "chatModelHasDirectImage".to_string(),
                json!(options.base.chat_model_has_direct_image),
            ),
            (
                "hasAudioRecognition".to_string(),
                json!(options.base.has_audio_recognition),
            ),
            (
                "hasVideoRecognition".to_string(),
                json!(options.base.has_video_recognition),
            ),
            (
                "chatModelHasDirectAudio".to_string(),
                json!(options.base.chat_model_has_direct_audio),
            ),
            (
                "chatModelHasDirectVideo".to_string(),
                json!(options.base.chat_model_has_direct_video),
            ),
            (
                "useToolCallApi".to_string(),
                json!(options.base.use_tool_call_api),
            ),
            (
                "toolExposureMode".to_string(),
                json!(format!("{:?}", options.base.tool_exposure_mode)),
            ),
            (
                "toolVisibility".to_string(),
                json!(options.base.tool_visibility),
            ),
        ]);
        metadata.extend(options.base.hook_metadata.clone());

        let before_context =
            PromptHookRegistry::dispatchSystemPromptComposeHooks(PromptHookContext {
                stage: "before_compose_system_prompt".to_string(),
                chat_id: options.base.chat_id.clone(),
                function_type: None,
                prompt_function_type: None,
                use_english: Some(options.base.use_english),
                raw_input: None,
                processed_input: None,
                chat_history: Vec::new(),
                prepared_history: Vec::new(),
                system_prompt: None,
                tool_prompt: None,
                model_parameters: Vec::new(),
                available_tools: Vec::new(),
                metadata,
            })
            .await?;

        let base_prompt = match before_context.system_prompt.clone() {
            Some(prompt) => prompt,
            None => Self::getSystemPrompt(options.base.clone()).await?,
        };
        let composed_prompt =
            Self::applyCustomPrompts(&base_prompt, &options.custom_intro_prompt);
        let compose_context =
            PromptHookRegistry::dispatchSystemPromptComposeHooks(PromptHookContext {
                stage: "compose_system_prompt_sections".to_string(),
                system_prompt: Some(composed_prompt),
                ..before_context
            })
            .await?;
        let after_compose_prompt = compose_context.system_prompt.clone().unwrap_or_default();
        let after_context =
            PromptHookRegistry::dispatchSystemPromptComposeHooks(PromptHookContext {
                stage: "after_compose_system_prompt".to_string(),
                system_prompt: Some(after_compose_prompt),
                ..compose_context
            })
            .await?;
        Ok(after_context.system_prompt.unwrap_or_default())
    }
}

#[allow(non_snake_case)]
fn buildWorkspaceRuleFileSection(
    rule_file: Option<&WorkspaceRuleFile>,
    use_english: bool,
) -> String {
    let Some(rule_file) = rule_file else {
        return String::new();
    };
    if rule_file.name.trim().is_empty() || rule_file.content.trim().is_empty() {
        return String::new();
    }
    if use_english {
        format!(
            "WORKSPACE ROOT RULE FILE:\n- The workspace root contains `{}`. Treat the following content as project-specific workspace instructions.\n<workspace_rule_file name=\"{}\">\n{}\n</workspace_rule_file>",
            rule_file.name, rule_file.name, rule_file.content
        )
    } else {
        format!(
            "工作区根目录规则文件：\n- 工作区根目录存在 `{}`，请将以下内容视为当前项目的工作区专属指令。\n<workspace_rule_file name=\"{}\">\n{}\n</workspace_rule_file>",
            rule_file.name, rule_file.name, rule_file.content
        )
    }
}

/// Describes attachment paths from the same directory used by file tools and terminals.
#[allow(non_snake_case)]
fn getAttachmentGuidelines(use_english: bool) -> &'static str {
    if use_english {
        "ATTACHMENTS:\n- Attachment `path` is relative to `.`. File tools and terminal sessions use this same base. Read supplied relative attachment paths directly, including `..`; use embedded content directly.\n- Small attachments contain the file; large attachments use `.operit-ref.json` with the source `node_id` and VFS path.\n- Check the current node first. For a missing file, use `list_core_nodes` and access its source with file tools. For terminal access, use `switch_core` and the source Host's resolved path.\n- If the path is missing, use embedded content or request re-upload. For terminal startup errors, report the error and use file tools."
    } else {
        "附件：\n- 附件 `path` 相对于 `.`，文件工具与终端使用同一个基准。直接读取附件提供的相对路径，支持 `..`；内嵌内容直接使用。\n- 小附件保存文件本体；大附件通过 `.operit-ref.json` 记录来源 `node_id` 和 VFS 路径。\n- 先读取当前节点；文件缺失时，通过 `list_core_nodes` 检查来源并用文件工具读取。终端访问来源文件时，使用 `switch_core` 和来源 Host 解析的路径。\n- 路径缺失时使用内嵌内容或请求重新上传；终端启动失败时报告错误并使用文件工具。"
    }
}

/// Keeps sync policy instructions tied to the implementation's actual default schema.
#[allow(non_snake_case)]
fn getFileSyncGuidelines(use_english: bool) -> String {
    let example = serde_json::to_string_pretty(&operit_store::FileSyncPolicy::FileSyncPolicy::default())
        .expect("The default file sync policy must serialize");
    let guidance = if use_english {
        "FILE SYNC POLICY:
- Read `.operit/sync.json` before installing dependencies, building, downloading or generating files in bulk. Preserve existing rules and limits; add exclusions first.
- Use valid JSON: `version` is 1; `ignoreFiles` lists relative ignore files; `.gitignore` includes nested rules; `exclude` uses Gitignore patterns (`dir/`, `*.tmp`, `!path`).
- Exclude dependency, build, cache, log, temporary and bulk-data directories (e.g. `.venv/`, `.cache/`, `coverage/`). Keep `.operit/` and final deliverables available. External mounts and symlinks are managed at their source.
- `maxFileBytes` and `maxTotalBytes` limit automatic copying per file and root, in bytes. Oversized files share `.operit-ref.json` locators; originals stay at the source. Policy changes affect subsequent copying and retain existing copies.
- Default policy; merge changes into the existing file:"
    } else {
        "文件同步：
- 安装依赖、构建、下载或批量生成文件前，读取 `.operit/sync.json`，保留已有规则和限额，先补充排除项。
- 使用合法 JSON：`version` 为 1；`ignoreFiles` 填相对路径，`.gitignore` 包含各级规则；`exclude` 使用 Gitignore 语法（`dir/`、`*.tmp`、`!path`）。
- 按目录排除依赖、构建、缓存、日志、临时文件和批量数据（如 `.venv/`、`.cache/`、`coverage/`）；保留 `.operit/` 与最终交付文件。外部挂载和软链接由来源管理。
- `maxFileBytes`、`maxTotalBytes` 分别限制单文件和目录的自动复制字节量。超限文件分享 `.operit-ref.json`，原件留在来源。规则调整作用于后续复制，已有副本保留。
- 默认配置如下，修改时合并已有内容："
    };
    format!("{guidance}
```json
{example}
```")
}

#[allow(non_snake_case)]
fn getWorkspaceGuidelines(
    workspace_path: Option<&str>,
    output_path: Option<&str>,
    workspace_folders: &[String],
    terminal_path_mappings: &[TerminalPathMapping],
    use_english: bool,
    workspace_rule_file: Option<&WorkspaceRuleFile>,
) -> String {
    let Some(workspace_path) = workspace_path.filter(|p| !p.trim().is_empty()) else {
        let Some(output) = output_path else { return String::new(); };
        let text = if use_english {
            "CONVERSATION OUTPUT DIRECTORY:\n- `.` is the conversation output area.\n- File tools and terminal sessions start from `.`. Use relative paths for attachments, outputs and Markdown links."
        } else {
            "对话产出目录：\n- `.` 是对话产出区域。\n- 文件工具与终端从 `.` 开始，附件、产出及 Markdown 引用使用相对路径。"
        };
        return format!("{text}\n\n{}", buildWorkspaceTerminalPathSection(terminal_path_mappings, output, use_english, true));
    };
    let mut seen = std::collections::HashSet::new();
    let mounted_folders = std::iter::once(workspace_path)
        .chain(workspace_folders.iter().map(String::as_str))
        .filter(|folder| !folder.trim().is_empty())
        .filter_map(|folder| operit_tools::files::PathMapper::PathMapper::relativeFileToolPath(workspace_path, folder).ok())
        .filter(|folder| seen.insert(folder.clone()))
        .map(|folder| format!("- `{folder}`"))
        .collect::<Vec<_>>()
        .join("\n");
    let base_guidelines = if use_english {
        format!(
            "WORKSPACE:\n- `.` is the primary workspace. Folder paths:\n{mounted_folders}\n- File tools and terminal sessions start from `.`. Use relative paths for attachments, outputs and Markdown links. Use Host-resolved paths for terminal access to other mounts.\n- `nodeName:path` addresses another device. Files synchronize eventually; verify arrival before use."
        )
    } else {
        format!(
            "工作区：\n- `.` 是主工作区。文件夹路径：\n{mounted_folders}\n- 文件工具与终端从 `.` 开始，附件、产出及 Markdown 引用使用相对路径。终端访问其他挂载时使用 Host 解析的路径。\n- `nodeName:path` 指向其他设备。文件最终一致同步，使用前确认已到达。"
        )
    };
    let terminal_section = buildWorkspaceTerminalPathSection(terminal_path_mappings, workspace_path, use_english, false);
    let rule_section = buildWorkspaceRuleFileSection(workspace_rule_file, use_english);
    [base_guidelines, terminal_section, rule_section].into_iter()
        .filter(|section| !section.is_empty()).collect::<Vec<_>>().join("\n\n")
}

/// Describes terminal availability at `.` and resolved paths for additional mounts.
#[allow(non_snake_case)]
fn buildWorkspaceTerminalPathSection(
    mappings: &[TerminalPathMapping],
    primary: &str,
    use_english: bool,
    is_output: bool,
) -> String {
    let header = match (use_english, is_output) {
        (true, true) => "TERMINAL OUTPUT DIRECTORY",
        (true, false) => "TERMINAL WORKSPACE PATHS",
        (false, true) => "终端产出目录",
        (false, false) => "终端工作区路径",
    };
    if mappings.is_empty() {
        return format!("{header}:\n{}", if use_english {
            "- Use file tools; terminal mapping is unavailable."
        } else {
            "- 当前使用文件工具，终端路径映射待配置。"
        });
    }
    let paths = mappings.iter().map(|mapping| {
        let label = format!("{}/{}", mapping.terminal, mapping.terminal_type);
        let relative = operit_tools::files::PathMapper::PathMapper::relativeFileToolPath(primary, &mapping.vfs_path)
            .unwrap_or_else(|_| mapping.vfs_path.clone());
        match &mapping.path {
            Ok(_) if relative == "." => format!("- `{label}`: `.`"),
            Ok(path) => format!("- `{label}`: `{relative}` → `{path}`"),
            Err(error) => format!("- `{label}`: `{relative}`: {error}"),
        }
    }).collect::<Vec<_>>().join("\n");
    let guidance = if use_english {
        "Each selected Host starts its sessions at `.`. Record `pwd` before `cd`; file tools keep their original base. For mapping errors, report the error and use file tools."
    } else {
        "所选 Host 从 `.` 启动终端。`cd` 前记录 `pwd`，文件工具始终使用原基准。映射失败时报告错误并使用文件工具。"
    };
    format!("{header} (Host):\n{paths}\n- {guidance}")
}

fn build_cli_mode_prompt(use_english: bool) -> String {
    CliToolModeSupport::buildCliModePrompt(use_english)
}

fn collapse_blank_lines(input: &str) -> String {
    let mut output = String::new();
    let mut blank_count = 0usize;
    for line in input.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                output.push('\n');
            }
        } else {
            blank_count = 0;
            output.push_str(line);
            output.push('\n');
        }
    }
    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{PackageInfo, SystemPromptConfig, SystemPromptOptions};

    /// Creates package-enabled prompt options for one tool transport mode.
    fn packagePromptOptions(useToolCallApi: bool) -> SystemPromptOptions {
        SystemPromptOptions {
            use_english: true,
            use_tool_call_api: useToolCallApi,
            enabled_packages: vec![PackageInfo {
                name: "browser".to_string(),
                description: "Browser automation".to_string(),
            }],
            ..SystemPromptOptions::default()
        }
    }

    /// Verifies native tool-call prompts never advertise the text XML protocol.
    #[tokio::test(flavor = "current_thread")]
    async fn nativeToolCallPromptExcludesXmlToolSyntax() {
        let prompt = SystemPromptConfig::getSystemPrompt(packagePromptOptions(true)).await.expect("Tool-prompt policies must succeed in this test");

        assert!(prompt.contains("call the use_package function"));
        assert!(!prompt.contains("<tool"));
        assert!(!prompt.contains("<param"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn native_package_guidelines_advertise_json_objects_in_both_languages() {
        for english in [true, false] {
            let mut options = packagePromptOptions(true);
            options.use_english = english;
            let prompt = SystemPromptConfig::getSystemPrompt(options).await.unwrap();
            assert!(prompt.contains("JSON"));
            assert!(!prompt.contains("YAML"));
        }
    }

    /// Verifies text-protocol prompts retain the XML package invocation syntax.
    #[tokio::test(flavor = "current_thread")]
    async fn xmlToolPromptIncludesPackageInvocationSyntax() {
        let prompt = SystemPromptConfig::getSystemPrompt(packagePromptOptions(false)).await.expect("Tool-prompt policies must succeed in this test");

        assert!(prompt.contains("<tool name=\"use_package\">"));
        assert!(prompt.contains("<param name=\"package_name\">"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn attachment_origin_guidelines_are_visible_without_a_workspace_in_both_languages() {
        for use_english in [true, false] {
            let prompt = SystemPromptConfig::getSystemPrompt(SystemPromptOptions {
                use_english,
                custom_system_prompt_template: "Custom instructions".into(),
                ..SystemPromptOptions::default()
            })
            .await.expect("Tool-prompt policies must succeed in this test");
            assert!(prompt.contains("node_id"));
            assert!(prompt.contains("switch_core"));
            assert!(prompt.contains(".operit/sync.json"));
            assert!(prompt.contains(".operit-ref.json"));
            assert!(!prompt.contains("resolve `id`"));
            assert!(!prompt.contains("物理路径 `id`"));
            assert!(prompt.contains(if use_english {
                "Check the current node first"
            } else {
                "先读取当前节点"
            }));
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspace_prompt_explains_sync_and_external_mount_boundaries() {
        for use_english in [true, false] {
            let prompt = SystemPromptConfig::getSystemPrompt(SystemPromptOptions {
                use_english,
                workspace_path: Some("/operitapp/workspaces/test".into()),
                ..SystemPromptOptions::default()
            })
            .await.expect("Tool-prompt policies must succeed in this test");
            if use_english {
                assert!(prompt.contains("synchronize eventually"));
                assert!(prompt.contains("External mounts and symlinks are managed at their source"));
                assert!(prompt.contains(".operit/sync.json"));
            } else {
                assert!(prompt.contains("最终一致"));
                assert!(prompt.contains("外部挂载和软链接由来源管理"));
                assert!(prompt.contains(".operit/sync.json"));
            }
        }
    }

    /// Workspaces, unbound output areas and custom templates all carry executable policy guidance.
    #[tokio::test(flavor = "current_thread")]
    async fn sync_policy_instructions_cover_schema_and_proactive_exclusion_in_both_languages() {
        let defaults = serde_json::to_string_pretty(&operit_store::FileSyncPolicy::FileSyncPolicy::default()).unwrap();
        for use_english in [true, false] {
            for workspace in [None, Some("/operitapp/workspaces/project".to_string())] {
                let prompt = SystemPromptConfig::getSystemPrompt(SystemPromptOptions {
                    use_english,
                    workspace_path: workspace,
                    output_path: Some("/operitapp/outputs/family".into()),
                    custom_system_prompt_template: "Custom instructions".into(),
                    ..SystemPromptOptions::default()
                }).await.unwrap();
                assert!(prompt.contains(&format!("```json\n{defaults}\n```")));
                for field in ["version", "ignoreFiles", "exclude", "maxFileBytes", "maxTotalBytes"] {
                    assert!(prompt.contains(field), "missing policy field {field}");
                }
                for pattern in ["node_modules/", "build/", ".gitignore", ".venv/", ".cache/", "coverage/", "!path"] {
                    assert!(prompt.contains(pattern), "missing exclusion guidance {pattern}");
                }
                assert!(prompt.contains(if use_english { "add exclusions first" } else { "先补充排除项" }));
                assert!(prompt.contains(if use_english { "Exclude dependency, build, cache, log, temporary and bulk-data directories" } else { "按目录排除" }));
                assert!(prompt.contains(if use_english { "Preserve existing rules and limits" } else { "保留已有规则和限额" }));
                assert!(prompt.contains(if use_english { "Keep `.operit/` and final deliverables available" } else { "保留 `.operit/` 与最终交付文件" }));
            }
        }
    }

    /// Verifies every mounted workspace folder is exposed in the model prompt.
    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptListsAllMountedFolders() {
        let prompt = SystemPromptConfig::getSystemPrompt(SystemPromptOptions {
            use_english: true,
            workspace_path: Some("/operitapp/workspaces/test".to_string()),
            workspace_folders: vec![
                "/operitapp/workspaces/test".to_string(),
                "/virtual/windows/d/Code/stm32".to_string(),
            ],
            ..SystemPromptOptions::default()
        })
        .await.expect("Tool-prompt policies must succeed in this test");

        assert!(prompt.contains("`.` is the primary workspace"));
        assert!(prompt.contains("../../../virtual/windows/d/Code/stm32"));
        assert!(prompt.contains("Use relative paths for attachments, outputs and Markdown"));
        assert!(!prompt.contains("/operitapp/workspaces/test"));
        assert!(!prompt.contains("not in file-tool path parameters"));
    }

    fn workspacePromptOptions(use_english: bool) -> SystemPromptOptions {
        SystemPromptOptions {
            use_english,
            workspace_path: Some("/operitapp/workspaces/main".into()),
            workspace_folders: vec![
                "/operitapp/workspaces/main".into(),
                "/operitapp/workspaces/library".into(),
            ],
            terminal_path_mappings: vec![
                super::TerminalPathMapping { terminal: "native".into(), terminal_type: "shell".into(), vfs_path: "/operitapp/workspaces/main".into(), path: Ok("/Users/test/My Projects/main".into()) },
                super::TerminalPathMapping { terminal: "native".into(), terminal_type: "shell".into(), vfs_path: "/operitapp/workspaces/library".into(), path: Ok("/Users/test/My Projects/library".into()) },
            ],
            workspace_path_mappings: vec![
                super::ResolvedVfsPath {
                    vfsPath: "/operitapp/workspaces/main".into(),
                    physicalPath: "/Users/test/My Projects/main".into(),
                },
                super::ResolvedVfsPath {
                    vfsPath: "/operitapp/workspaces/library".into(),
                    physicalPath: "/Users/test/My Projects/library".into(),
                },
            ],
            ..SystemPromptOptions::default()
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptIncludesTerminalMappingsInBothLanguagesAndToolModes() {
        for use_english in [false, true] {
            for mode in [super::ToolExposureMode::FULL, super::ToolExposureMode::CLI] {
                for use_tool_call_api in [false, true] {
                    let mut options = workspacePromptOptions(use_english);
                    options.tool_exposure_mode = mode.clone();
                    options.use_tool_call_api = use_tool_call_api;
                    let prompt = SystemPromptConfig::getSystemPrompt(options).await.expect("Tool-prompt policies must succeed in this test");
                    assert!(prompt.contains("`native/shell`: `.`"));
                    assert!(prompt.contains("/Users/test/My Projects/library"));
                    assert!(prompt.contains(if use_english {
                        "Each selected Host starts its sessions at `.`"
                    } else {
                        "所选 Host 从 `.` 启动终端"
                    }));
                }
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptSurvivesCustomTemplatesWithOrWithoutPlaceholder() {
        for template in [
            "Custom instructions.",
            "Custom instructions.\nWORKSPACE_GUIDELINES_SECTION",
        ] {
            let mut options = workspacePromptOptions(true);
            options.custom_system_prompt_template = template.into();
            let prompt = SystemPromptConfig::getSystemPrompt(options).await.expect("Tool-prompt policies must succeed in this test");
            assert!(prompt.contains("Custom instructions."));
            assert!(prompt.contains("`native/shell`: `.`"));
            assert_eq!(prompt.matches("TERMINAL WORKSPACE PATHS").count(), 1);
            assert!(!prompt.contains("WORKSPACE_GUIDELINES_SECTION"));
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptOmitsPathsWhenToolsAreDisabledOrWorkspaceIsUnbound() {
        for template in ["Custom instructions.", "WORKSPACE_GUIDELINES_SECTION"] {
            let mut options = workspacePromptOptions(true);
            options.custom_system_prompt_template = template.into();
            options.enable_tools = false;
            assert!(!SystemPromptConfig::getSystemPrompt(options)
                .await.expect("Tool-prompt policies must succeed in this test")
                .contains("/Users/test/My Projects"));
        }
        for path in [None, Some(" ".into())] {
            let mut options = workspacePromptOptions(true);
            options.workspace_path = path;
            assert!(!SystemPromptConfig::getSystemPrompt(options)
                .await.expect("Tool-prompt policies must succeed in this test")
                .contains("TERMINAL WORKSPACE PATHS"));
        }
    }

    /// Every tool protocol and custom template must retain the relative-path contract.
    #[tokio::test(flavor = "current_thread")]
    async fn chat_file_prompt_uses_the_same_relative_base_without_attachment_searches() {
        for use_english in [false, true] {
            for workspace_path in [None, Some("/operitapp/workspaces/project".to_string())] {
                for (use_tool_call_api, tool_exposure_mode) in [
                    (false, super::ToolExposureMode::FULL),
                    (true, super::ToolExposureMode::FULL),
                    (false, super::ToolExposureMode::CLI),
                ] {
                    for template in ["", "Custom instructions"] {
                        let prompt = SystemPromptConfig::getSystemPrompt(SystemPromptOptions {
                            use_english,
                            workspace_path: workspace_path.clone(),
                            output_path: Some("/operitapp/outputs/family".into()),
                            use_tool_call_api,
                            tool_exposure_mode: tool_exposure_mode.clone(),
                            custom_system_prompt_template: template.into(),
                            ..SystemPromptOptions::default()
                        }).await.expect("Tool-prompt policies must succeed in this test");
                        assert!(prompt.contains(if use_english {
                            "File tools and terminal sessions use this same base"
                        } else {
                            "文件工具与终端使用同一个基准"
                        }));
                        assert!(prompt.contains(if use_english {
                            "Read supplied relative attachment paths directly"
                        } else {
                            "直接读取附件提供的相对路径"
                        }));
                        assert!(prompt.contains(if use_english {
                            "For terminal startup errors"
                        } else {
                            "终端启动失败时报告错误"
                        }));
                        assert!(!prompt.contains("Relative file paths require a workspace"));
                        assert!(!prompt.contains("Without a workspace, list `/`"));
                        assert!(!prompt.contains("文件相对路径需要绑定工作区"));
                        assert!(!prompt.contains("没有工作区时，可列出 `/`"));
                    }
                }
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptDoesNotGuessUnresolvedTerminalPaths() {
        let mut options = workspacePromptOptions(true);
        options.terminal_path_mappings.clear();
        let prompt = SystemPromptConfig::getSystemPrompt(options).await.expect("Tool-prompt policies must succeed in this test");
        assert!(prompt.contains("terminal mapping is unavailable"));
        assert!(!prompt.contains("Terminal absolute path:"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptUsesExactHostMappingsForEachTerminal() {
        let mut options = workspacePromptOptions(true);
        options.terminal_path_mappings = vec![
            super::TerminalPathMapping { terminal: "native".into(), terminal_type: "shell".into(), vfs_path: "/operitapp/workspaces/main".into(), path: Ok("/native/custom/project".into()) },
            super::TerminalPathMapping { terminal: "guest".into(), terminal_type: "bash".into(), vfs_path: "/operitapp/workspaces/main".into(), path: Ok("/guest/custom/project".into()) },
            super::TerminalPathMapping { terminal: "unavailable".into(), terminal_type: "shell".into(), vfs_path: "/operitapp/workspaces/main".into(), path: Err("Document mounts cannot be terminal directories".into()) },
        ];
        let prompt = SystemPromptConfig::getSystemPrompt(options).await.unwrap();
        assert!(prompt.contains("`native/shell`: `.`"));
        assert!(prompt.contains("`guest/bash`: `.`"));
        assert!(prompt.contains("Document mounts cannot be terminal directories"));
        assert!(!prompt.contains("/Users/test/My Projects"));
        assert!(!prompt.contains("/mnt/host-root"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptIncludesHostDrivePathsWithoutRewritingThem() {
        let mut options = workspacePromptOptions(true);
        options.terminal_path_mappings[1].path = Ok("D:/My Projects/library".into());
        let prompt = SystemPromptConfig::getSystemPrompt(options).await.unwrap();
        assert!(prompt.contains("`../library` → `D:/My Projects/library`"));
    }

    struct LanguageHookProbe {
        expected: bool,
        events: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl crate::chat::hooks::PromptHookRegistry::ToolPromptComposeHook for LanguageHookProbe {
        /// Identifies the isolated language-selection regression hook.
        fn id(&self) -> &str { "test.system-prompt.selected-language" }

        /// Rejects construction in the unused language before the prompt can be returned.
        fn on_event_async<'a>(
            &'a self,
            context: &'a crate::chat::hooks::PromptHookRegistry::PromptHookContext,
        ) -> crate::chat::hooks::PromptHookRegistry::ToolPromptHookFuture<'a> {
            Box::pin(async move {
                if context.chat_id.as_deref() != Some("selected-language-test") {
                    return Ok(None);
                }
                if context.use_english != Some(self.expected) {
                    return Err("The unused tool-prompt language was dispatched".into());
                }
                self.events.lock().unwrap().push(context.stage.clone());
                Ok(None)
            })
        }
    }

    /// Generates exactly one four-stage tool prompt and never invokes policies in the other language.
    #[tokio::test(flavor = "current_thread")]
    async fn get_system_prompt_dispatches_only_selected_language() {
        use crate::chat::hooks::PromptHookRegistry::PromptHookRegistry;
        for use_english in [false, true] {
            let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            PromptHookRegistry::registerToolPromptComposeHook(std::sync::Arc::new(LanguageHookProbe {
                expected: use_english, events: events.clone(),
            }));
            let result = SystemPromptConfig::getSystemPrompt(SystemPromptOptions {
                chat_id: Some("selected-language-test".into()),
                use_english,
                ..SystemPromptOptions::default()
            }).await;
            PromptHookRegistry::unregisterToolPromptComposeHook("test.system-prompt.selected-language");
            result.expect("Only the selected language may run tool-prompt policies");
            assert_eq!(*events.lock().unwrap(), [
                "build_tool_prompt", "before_compose_tool_prompt",
                "filter_tool_prompt_items", "after_compose_tool_prompt",
            ]);
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn unbound_output_directory_uses_a_concise_relative_base() {
        for use_english in [true, false] {
            let prompt = SystemPromptConfig::getSystemPrompt(SystemPromptOptions {
                use_english, output_path: Some("/operitapp/outputs/family".into()),
                ..SystemPromptOptions::default()
            }).await.unwrap();
            assert!(!prompt.contains("/operitapp/outputs/family"));
            assert!(prompt.contains("`.`"));
            assert!(prompt.contains(".operit-ref.json"));
            assert!(prompt.contains(if use_english { "`.` is the conversation output area." } else { "`.` 是对话产出区域。" }));
            assert!(!prompt.contains(if use_english { "Sessions start in the primary workspace" } else { "会话从主工作区启动" }));
        }
    }

}
