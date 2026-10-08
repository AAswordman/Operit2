use std::collections::HashMap;

use operit_host_api::{HostEnvironmentDescriptor, HostPlatform};
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

#[derive(Clone, Debug)]
pub struct SystemPromptOptions {
    pub chat_id: Option<String>,
    pub workspace_path: Option<String>,
    pub workspace_folders: Vec<String>,
    /// VFS roots mapped to absolute host paths by the active file-tool mapper.
    pub workspace_path_mappings: Vec<ResolvedVfsPath>,
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
            workspace_folders: Vec::new(),
            workspace_path_mappings: Vec::new(),
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
    pub enable_group_orchestration_hint: bool,
    pub group_orchestration_role_name: String,
    pub group_participant_names_text: String,
}

pub struct SystemPromptConfig;

impl SystemPromptConfig {
    #[allow(non_snake_case)]
    pub fn applyCustomPrompts(system_prompt: &str, custom_intro_prompt: &str) -> String {
        system_prompt.replace("BEGIN_SELF_INTRODUCTION_SECTION", custom_intro_prompt)
    }

    #[allow(non_snake_case)]
    pub async fn getSystemPrompt(options: SystemPromptOptions) -> String {
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
            &options.workspace_folders,
            &options.workspace_path_mappings,
            &options.host_environment.platform,
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

        let available_tools_en =
            if options.use_tool_call_api || options.tool_exposure_mode == ToolExposureMode::CLI {
                String::new()
            } else {
                format!(
                    "{}{}",
                    SystemToolPrompts::generateMemoryToolsPromptEn(&options.tool_visibility),
                    SystemToolPrompts::generateToolsPromptEnForHost(
                        options.chat_id.clone(),
                        options.has_image_recognition,
                        false,
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
                    .await
                )
            };
        let available_tools_cn =
            if options.use_tool_call_api || options.tool_exposure_mode == ToolExposureMode::CLI {
                String::new()
            } else {
                format!(
                    "{}{}",
                    SystemToolPrompts::generateMemoryToolsPromptCn(&options.tool_visibility),
                    SystemToolPrompts::generateToolsPromptCnForHost(
                        options.chat_id.clone(),
                        options.has_image_recognition,
                        false,
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
                    .await
                )
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
                        if options.use_english {
                            &available_tools_en
                        } else {
                            &available_tools_cn
                        },
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
        }

        collapse_blank_lines(&prompt)
    }

    #[allow(non_snake_case)]
    pub async fn getSystemPromptWithCustomPrompts(
        options: SystemPromptWithCustomOptions,
    ) -> String {
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
            (
                "enableGroupOrchestrationHint".to_string(),
                json!(options.enable_group_orchestration_hint),
            ),
            (
                "groupOrchestrationRoleName".to_string(),
                json!(options.group_orchestration_role_name),
            ),
            (
                "groupParticipantNamesText".to_string(),
                json!(options.group_participant_names_text),
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
                on_hook_timeout: None,
            })
            .await;

        let base_prompt = match before_context.system_prompt.clone() {
            Some(prompt) => prompt,
            None => Self::getSystemPrompt(options.base.clone()).await,
        };
        let mut composed_prompt =
            Self::applyCustomPrompts(&base_prompt, &options.custom_intro_prompt);
        if options.enable_group_orchestration_hint {
            let role_name = if options.group_orchestration_role_name.is_empty() {
                if options.base.use_english {
                    "assistant"
                } else {
                    "助手"
                }
                .to_string()
            } else {
                options.group_orchestration_role_name.clone()
            };
            composed_prompt.push_str(&buildGroupOrchestrationHint(
                options.base.use_english,
                &role_name,
                &options.group_participant_names_text,
            ));
        }

        let compose_context =
            PromptHookRegistry::dispatchSystemPromptComposeHooks(PromptHookContext {
                stage: "compose_system_prompt_sections".to_string(),
                system_prompt: Some(composed_prompt),
                ..before_context
            })
            .await;
        let after_compose_prompt = compose_context.system_prompt.clone().unwrap_or_default();
        let after_context =
            PromptHookRegistry::dispatchSystemPromptComposeHooks(PromptHookContext {
                stage: "after_compose_system_prompt".to_string(),
                system_prompt: Some(after_compose_prompt),
                ..compose_context
            })
            .await;
        after_context.system_prompt.unwrap_or_default()
    }
}

#[allow(non_snake_case)]
fn buildGroupOrchestrationHint(
    use_english: bool,
    role_name: &str,
    participant_names_text: &str,
) -> String {
    if use_english {
        format!(
            "\n\nRole response plan hint:\n- This chat uses a role response planner. After each user message, the system dynamically decides who responds and in what order.\n- Always keep your own role identity. Never reply as another role or imitate another persona.\n- Answer the user's latest request in your own role, optionally considering prior agents' replies.\n- If you have nothing new, reply briefly in your own role.\n\nRole-scoped history hint:\n- Messages prefixed with [From role: xxx] are historical outputs from other role cards.\n- Treat them as reference context only, not as the current user's new request.\n- Stay in role as {role_name}, and do not switch persona to the referenced role.\n\nGroup participants: {participant_names_text}"
        )
    } else {
        format!(
            "\n\n角色回答规划提示：\n- 当前会话启用了角色回答规划，用户每次发言后系统会动态决定谁回答以及回答顺序。\n- 你必须始终牢记并保持你自己的角色身份，严禁使用他人身份回答或模仿其他角色口吻。\n- 用你自己的角色身份回答用户最新请求，可以参考前面角色的回复。\n- 如果没有新的内容，也请用自己的角色简短回应。\n\n角色分视角历史说明：\n- 带有 [From role: xxx] 前缀的内容是其他角色卡的历史输出。\n- 这类内容仅用于上下文参考，不是当前用户的新指令。\n- 你必须保持当前角色身份（{role_name}），不要切换为前缀中的角色。\n\n当前群聊参与者：{participant_names_text}"
        )
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

/// Attachment files remain node-local and ephemeral; metadata does not copy their bytes.
#[allow(non_snake_case)]
fn getAttachmentGuidelines(use_english: bool) -> &'static str {
    if use_english {
        "ATTACHMENT LOCATIONS:\n- An attachment's `node_id` identifies the CoreNode that actually holds its file; it is not necessarily the current execution node. Node metadata does not transfer or synchronize the file.\n- Use content already embedded in the message directly. To access a file, use `list_core_nodes` to check the current node and source reachability. If the source differs, call `switch_core` with the exact `node_id` and wait for continuation on that node before using file tools; switching changes this chat's execution node. Use the attachment's `path` VFS locator for file tools. Client-local physical paths are not exposed in attachment metadata. If no `path` is provided, use embedded content or request re-upload; do not guess a physical path or search for a mapping. Do not search the current device for another device's file.\n- Files under `/app/data/temp/clean_on_exit` are temporary, not Space-synchronized, and may have been cleaned. If the source is unreachable or the file has been cleaned, explain this and request reconnection or re-upload instead of searching unrelated directories. Legacy attachments without `node_id` have an unknown source; do not invent one."
    } else {
        "附件位置：\n- 附件的 `node_id` 是文件实际所在的 CoreNode，不一定是当前执行节点；携带节点信息不代表文件已传输或同步。\n- 消息中已经内嵌的内容可直接使用。需要访问文件时，先用 `list_core_nodes` 确认当前节点和来源节点是否可达；来源不同则用精确的 `node_id` 调用 `switch_core`，等待在目标节点继续执行后再调用文件工具。切换会改变这段聊天的执行节点。文件工具使用附件的 `path` VFS 地址；附件元数据不暴露客户端物理路径。没有 `path` 时，使用内嵌内容或请用户重新上传，不要猜测物理路径或搜索映射。不要在当前设备搜索另一台设备的文件。\n- `/app/data/temp/clean_on_exit` 下的文件是临时附件，不参与 Space 文件同步，可能已经清理。来源不可达或文件已清理时，明确说明并请用户重连或重新上传，不要搜索无关目录。旧附件没有 `node_id` 时来源未知，不得猜测补成当前节点。"
    }
}

/// Builds workspace instructions with every mounted folder visible to the model.
#[allow(non_snake_case)]
fn getWorkspaceGuidelines(
    workspace_path: Option<&str>,
    workspace_folders: &[String],
    workspace_path_mappings: &[ResolvedVfsPath],
    host_platform: &HostPlatform,
    use_english: bool,
    workspace_rule_file: Option<&WorkspaceRuleFile>,
) -> String {
    let Some(workspace_path) = workspace_path else {
        return String::new();
    };
    if workspace_path.trim().is_empty() {
        return String::new();
    }
    let mut seen = std::collections::HashSet::new();
    let mounted_folders = std::iter::once(workspace_path)
        .chain(workspace_folders.iter().map(String::as_str))
        .filter(|folder| !folder.trim().is_empty())
        .filter(|folder| seen.insert(folder.trim_end_matches('/').to_string()))
        .map(|folder| format!("- `{folder}`"))
        .collect::<Vec<_>>()
        .join("\n");
    let base_guidelines = if use_english {
        format!(
            "WORKSPACE GUIDELINES:\n- The current workspace root is `{workspace_path}`.\n- This workspace contains these mounted folders; every listed path belongs to the same workspace:\n{mounted_folders}\n- Treat every listed VFS path as an allowed workspace root; do not limit workspace operations to the first path.\n- File tools accept VFS paths only. Use absolute paths rooted at the relevant listed workspace folder.\n- The workspace collection is under `/app/workspaces`; each workspace must be addressed by its full VFS path.\n- Within the same Space, ordinary files in the workspace body under `/app/workspaces/<workspace-id>/...` are automatically replicated bidirectionally between devices running supported Core versions. Synchronization is eventual and needs connectivity; membership, pairing, or an identical path does not prove a file has arrived. Verify availability on the execution node.\n- External mounted folders (such as `/mnt/...` and `/data/...`), symlinks, and temporary attachments are not automatically included in workspace synchronization. To retain and share such a file, copy it into the workspace body; a mount or symlink alone is not enough.\n- Root listing always shows `/app`; `/mnt` is listed when this host has mounted external entries.\n- `/sdcard` and `/data` are hidden Android aliases that can be opened directly on Android hosts.\n- Relative paths are allowed in project-internal references and terminal commands after selecting the working directory, but not in file-tool path parameters.\n- **Best Practice for Code Modifications**: Before modifying any file, use `grep_code` and `grep_context` to locate and understand relevant code with surrounding context. This ensures you understand the codebase structure before making changes."
        )
    } else {
        format!(
            "工作区指南：\n- 当前工作区根目录是 `{workspace_path}`。\n- 当前工作区包含以下挂载文件夹，所有列出的路径都属于同一个工作区：\n{mounted_folders}\n- 每个列出的 VFS 路径都是允许访问的工作区根目录，不能只使用第一个路径。\n- 文件工具只接受 VFS 路径；操作文件时，请使用以对应工作区文件夹为根的绝对路径。\n- 工作区集合位于 `/app/workspaces`；每个工作区都必须用完整 VFS 路径访问。\n- 同一 Space 内，`/app/workspaces/<workspace-id>/...` 工作区本体中的普通文件，会在运行支持版本 Core 的设备间自动双向复制。同步是最终一致的，需要设备连通；仅配对、成员关系或路径相同不代表文件已到达，使用前应确认当前执行节点的文件已就绪。\n- 外部挂载目录（如 `/mnt/...`、`/data/...`）、软链接和临时附件，不会自动纳入工作区文件同步。需要长期保留并跨端共享的文件，应实际复制进工作区本体；只挂载目录或创建软链接不够。\n- 根目录列表固定展示 `/app`；当前 Host 存在外部挂载项时才展示 `/mnt`。\n- `/sdcard` 和 `/data` 是 Android 隐藏别名，只在 Android Host 上可直接访问。\n- 项目内部引用和已切换工作目录的终端命令可以使用相对路径，但文件工具的路径参数必须使用 VFS 绝对路径。\n- **代码修改最佳实践**：修改任何文件之前，建议组合使用 `grep_code` 与 `grep_context` 定位并理解相关代码及其上下文，避免在未理解项目结构时盲改。"
        )
    };
    let terminal_section =
        buildWorkspaceTerminalPathSection(workspace_path_mappings, host_platform, use_english);
    let rule_section = buildWorkspaceRuleFileSection(workspace_rule_file, use_english);
    [base_guidelines, terminal_section, rule_section]
        .into_iter()
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Distinguishes virtual file-tool paths from terminal-visible filesystem paths.
#[allow(non_snake_case)]
fn buildWorkspaceTerminalPathSection(
    mappings: &[ResolvedVfsPath],
    host_platform: &HostPlatform,
    use_english: bool,
) -> String {
    if *host_platform == HostPlatform::Web {
        return if use_english {
            "TERMINAL WORKSPACE PATHS:\n- The browser terminal runs in an isolated Linux VM. Workspace VFS storage is not mounted into that VM; do not use VFS paths or browser storage keys as terminal directories. Use file tools for workspace files."
        } else {
            "终端工作区路径：\n- 浏览器终端运行在独立的 Linux 虚拟机中，工作区 VFS 存储未挂载到该虚拟机。不要把 VFS 路径或浏览器存储键当作终端目录；请使用文件工具访问工作区文件。"
        }.to_string();
    }
    if mappings.is_empty() {
        return if use_english {
            "TERMINAL WORKSPACE PATHS:\n- No absolute host path mapping is available for this workspace. Do not assume `/app/workspaces` exists in the terminal or guess its physical location; use file tools for workspace files."
        } else {
            "终端工作区路径：\n- 当前工作区没有可用的宿主绝对路径映射。不要假设终端中存在 `/app/workspaces`，也不要猜测其物理位置；请使用文件工具访问工作区文件。"
        }.to_string();
    }
    let paths = mappings.iter().map(|mapping| {
        if *host_platform == HostPlatform::Ohos {
            // QEMU-vroot mounts the native host root at /mnt/host-root.
            if use_english {
                format!("- File tools (VFS): `{}` → Native terminal: `{}`; QEMU-vroot terminal: `/mnt/host-root{}`",
                    mapping.vfsPath, mapping.physicalPath, mapping.physicalPath)
            } else {
                format!("- 文件工具（VFS）：`{}` → 原生终端：`{}`；QEMU-vroot 终端：`/mnt/host-root{}`",
                    mapping.vfsPath, mapping.physicalPath, mapping.physicalPath)
            }
        } else if use_english {
            format!("- File tools (VFS): `{}` → Terminal absolute path: `{}`",
                mapping.vfsPath, mapping.physicalPath)
        } else {
            format!("- 文件工具（VFS）：`{}` → 终端绝对路径：`{}`",
                mapping.vfsPath, mapping.physicalPath)
        }
    }).collect::<Vec<_>>().join("\n");
    if use_english {
        format!("TERMINAL WORKSPACE PATHS (resolved by the host):\n{paths}\n- File tools must continue to use the VFS paths on the left; terminal commands must use the corresponding terminal paths on the right. `/app/workspaces` and `/mnt/...` are virtual paths, not necessarily terminal mount points.\n- Before running project commands, explicitly `cd` to the corresponding terminal directory using your shell's quoting syntax (paths may contain spaces). Do not assume the session's initial or current directory is the workspace.\n- These mappings already locate the workspace; do not search the entire filesystem for it. For a child file, append the same workspace-relative suffix to the corresponding root.")
    } else {
        format!("终端工作区路径（由宿主解析）：\n{paths}\n- 文件工具继续使用左侧 VFS 路径；终端命令必须使用右侧对应的终端路径。`/app/workspaces` 和 `/mnt/...` 是虚拟路径，不一定是终端挂载点。\n- 执行项目命令前，先用当前 shell 的引号语法显式 `cd` 到对应终端目录（路径可能包含空格）。不要假设会话的初始目录或当前目录就是工作区。\n- 上述映射已经定位工作区，不要再全盘搜索工作区位置。访问子文件时，在对应根目录后拼接相同的工作区内相对路径。")
    }
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
        let prompt = SystemPromptConfig::getSystemPrompt(packagePromptOptions(true)).await;

        assert!(prompt.contains("call the use_package function"));
        assert!(!prompt.contains("<tool"));
        assert!(!prompt.contains("<param"));
    }

    /// Verifies text-protocol prompts retain the XML package invocation syntax.
    #[tokio::test(flavor = "current_thread")]
    async fn xmlToolPromptIncludesPackageInvocationSyntax() {
        let prompt = SystemPromptConfig::getSystemPrompt(packagePromptOptions(false)).await;

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
            .await;
            assert!(prompt.contains("node_id"));
            assert!(prompt.contains("switch_core"));
            assert!(prompt.contains("/app/data/temp/clean_on_exit"));
            assert!(!prompt.contains("resolve `id`"));
            assert!(!prompt.contains("物理路径 `id`"));
            assert!(prompt.contains(if use_english {
                "Client-local physical paths are not exposed"
            } else {
                "附件元数据不暴露客户端物理路径"
            }));
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspace_prompt_explains_sync_and_external_mount_boundaries() {
        for use_english in [true, false] {
            let prompt = SystemPromptConfig::getSystemPrompt(SystemPromptOptions {
                use_english,
                workspace_path: Some("/app/workspaces/test".into()),
                ..SystemPromptOptions::default()
            })
            .await;
            if use_english {
                assert!(prompt.contains("automatically replicated bidirectionally"));
                assert!(prompt.contains("Synchronization is eventual"));
                assert!(prompt.contains("External mounted folders"));
                assert!(prompt.contains("copy it into the workspace body"));
            } else {
                assert!(prompt.contains("自动双向复制"));
                assert!(prompt.contains("最终一致"));
                assert!(prompt.contains("外部挂载目录"));
                assert!(prompt.contains("实际复制进工作区本体"));
            }
        }
    }

    /// Verifies every mounted workspace folder is exposed in the model prompt.
    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptListsAllMountedFolders() {
        let prompt = SystemPromptConfig::getSystemPrompt(SystemPromptOptions {
            use_english: true,
            workspace_path: Some("/app/workspaces/test".to_string()),
            workspace_folders: vec![
                "/app/workspaces/test".to_string(),
                "/mnt/windows/d/Code/stm32".to_string(),
            ],
            ..SystemPromptOptions::default()
        })
        .await;

        assert!(prompt.contains("/app/workspaces/test"));
        assert!(prompt.contains("/mnt/windows/d/Code/stm32"));
        assert!(prompt.contains("do not limit workspace operations to the first path"));
    }

    fn workspacePromptOptions(use_english: bool) -> SystemPromptOptions {
        SystemPromptOptions {
            use_english,
            workspace_path: Some("/app/workspaces/main".into()),
            workspace_folders: vec![
                "/app/workspaces/main".into(),
                "/app/workspaces/library".into(),
            ],
            workspace_path_mappings: vec![
                super::ResolvedVfsPath {
                    vfsPath: "/app/workspaces/main".into(),
                    physicalPath: "/Users/test/My Projects/main".into(),
                },
                super::ResolvedVfsPath {
                    vfsPath: "/app/workspaces/library".into(),
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
                    let prompt = SystemPromptConfig::getSystemPrompt(options).await;
                    assert!(prompt.contains("/Users/test/My Projects/main"));
                    assert!(prompt.contains("/Users/test/My Projects/library"));
                    assert!(prompt.contains(if use_english {
                        "terminal commands must use the corresponding terminal paths"
                    } else {
                        "终端命令必须使用右侧对应的终端路径"
                    }));
                    assert!(prompt.contains(if use_english {
                        "do not search the entire filesystem"
                    } else {
                        "不要再全盘搜索"
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
            let prompt = SystemPromptConfig::getSystemPrompt(options).await;
            assert!(prompt.contains("Custom instructions."));
            assert!(prompt.contains("/Users/test/My Projects/main"));
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
                .await
                .contains("/Users/test/My Projects"));
        }
        for path in [None, Some(" ".into())] {
            let mut options = workspacePromptOptions(true);
            options.workspace_path = path;
            assert!(!SystemPromptConfig::getSystemPrompt(options)
                .await
                .contains("TERMINAL WORKSPACE PATHS"));
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptDoesNotGuessUnresolvedTerminalPaths() {
        let mut options = workspacePromptOptions(true);
        options.workspace_path_mappings.clear();
        let prompt = SystemPromptConfig::getSystemPrompt(options).await;
        assert!(prompt.contains("No absolute host path mapping is available"));
        assert!(!prompt.contains("Terminal absolute path:"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptDistinguishesOhosNativeAndVrootPaths() {
        let mut options = workspacePromptOptions(true);
        options.host_environment.platform = super::HostPlatform::Ohos;
        let prompt = SystemPromptConfig::getSystemPrompt(options).await;
        assert!(prompt.contains("Native terminal: `/Users/test/My Projects/main`"));
        assert!(
            prompt.contains("QEMU-vroot terminal: `/mnt/host-root/Users/test/My Projects/main`")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptDoesNotTreatBrowserStorageAsVmDirectories() {
        let mut options = workspacePromptOptions(true);
        options.host_environment.platform = super::HostPlatform::Web;
        let prompt = SystemPromptConfig::getSystemPrompt(options).await;
        assert!(prompt.contains("Workspace VFS storage is not mounted into that VM"));
        assert!(!prompt.contains("/Users/test/My Projects"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workspacePromptIncludesWindowsDrivePathsWithoutRewritingThem() {
        let mut options = workspacePromptOptions(true);
        options.host_environment.platform = super::HostPlatform::Windows;
        options.workspace_path_mappings[0].physicalPath = "D:/My Projects/main".into();
        let prompt = SystemPromptConfig::getSystemPrompt(options).await;
        assert!(prompt.contains("Terminal absolute path: `D:/My Projects/main`"));
    }
}
