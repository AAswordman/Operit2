use std::collections::HashMap;

use operit_host_api::HostEnvironmentDescriptor;
use serde_json::{json, Value};

use crate::core::chat::hooks::PromptHookRegistry::{PromptHookContext, PromptHookRegistry};
use crate::core::config::SystemToolPrompts::SystemToolPrompts;
use crate::core::tools::climode::CliToolModeSupport::CliToolModeSupport;

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
    pub fn getSystemPrompt(options: SystemPromptOptions) -> String {
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

        if package_system_visible {
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

        collapse_blank_lines(&prompt)
    }

    #[allow(non_snake_case)]
    pub fn getSystemPromptWithCustomPrompts(options: SystemPromptWithCustomOptions) -> String {
        let mut metadata = HashMap::from([
            (
                "workspacePath".to_string(),
                json!(options.base.workspace_path),
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
            });

        let base_prompt = before_context
            .system_prompt
            .clone()
            .unwrap_or_else(|| Self::getSystemPrompt(options.base.clone()));
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
            });
        let after_compose_prompt = compose_context.system_prompt.clone().unwrap_or_default();
        let after_context =
            PromptHookRegistry::dispatchSystemPromptComposeHooks(PromptHookContext {
                stage: "after_compose_system_prompt".to_string(),
                system_prompt: Some(after_compose_prompt),
                ..compose_context
            });
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

#[allow(non_snake_case)]
fn getWorkspaceGuidelines(
    workspace_path: Option<&str>,
    use_english: bool,
    workspace_rule_file: Option<&WorkspaceRuleFile>,
) -> String {
    let Some(workspace_path) = workspace_path else {
        return String::new();
    };
    if workspace_path.trim().is_empty() {
        return String::new();
    }
    let base_guidelines = if use_english {
        format!(
            "WORKSPACE GUIDELINES:\n- The current workspace root is `{workspace_path}`.\n- Treat this exact VFS path as the base path for all workspace file operations.\n- File tools accept VFS paths only. Use absolute paths rooted at `{workspace_path}` for workspace files.\n- The workspace collection is under `/app/workspaces`; each workspace must be addressed by its full VFS path.\n- Root listing always shows `/app`; `/mnt` is listed when this host has mounted external entries.\n- `/sdcard` and `/data` are hidden Android aliases that can be opened directly on Android hosts.\n- Relative paths are only for file contents or project-internal references, not for tool parameters.\n- **Best Practice for Code Modifications**: Before modifying any file, use `grep_code` and `grep_context` to locate and understand relevant code with surrounding context. This ensures you understand the codebase structure before making changes."
        )
    } else {
        format!(
            "工作区指南：\n- 当前工作区根目录是 `{workspace_path}`。\n- 所有工作区文件操作都要把这个精确 VFS 路径当作根路径。\n- 文件工具只接受 VFS 路径；操作工作区文件时，请使用以 `{workspace_path}` 为根的绝对路径。\n- 工作区集合位于 `/app/workspaces`；每个工作区都必须用完整 VFS 路径访问。\n- 根目录列表固定展示 `/app`；当前 Host 存在外部挂载项时才展示 `/mnt`。\n- `/sdcard` 和 `/data` 是 Android 隐藏别名，只在 Android Host 上可直接访问。\n- 相对路径只用于文件内容里的项目内部引用，不用于工具参数。\n- **代码修改最佳实践**：修改任何文件之前，建议组合使用 `grep_code` 与 `grep_context` 定位并理解相关代码及其上下文，避免在未理解项目结构时盲改。"
        )
    };
    let rule_section = buildWorkspaceRuleFileSection(workspace_rule_file, use_english);
    if rule_section.is_empty() {
        base_guidelines
    } else {
        format!("{base_guidelines}\n\n{rule_section}")
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
