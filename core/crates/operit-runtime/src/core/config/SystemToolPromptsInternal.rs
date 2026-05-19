use crate::core::config::SystemToolPrompts::{
    SystemToolPromptCategory, ToolParameterSchema, ToolPrompt,
};

pub struct SystemToolPromptsInternal;

impl SystemToolPromptsInternal {
    #[allow(non_snake_case)]
    pub fn internalToolCategoriesEn() -> Vec<SystemToolPromptCategory> {
        vec![
            SystemToolPromptCategory {
                category_name: "Internal Tools".to_string(),
                category_header: String::new(),
                tools: vec![
                    ToolPrompt {
                        name: "execute_shell".to_string(),
                        description: "Execute a device shell command.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![param(
                            "command",
                            "string",
                            "shell command to execute",
                            true,
                            None,
                        )],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "apply_file".to_string(),
                        description: "Applies edits to a file by finding and replacing/deleting a matched content block.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "file path", true, None),
                            param(
                                "environment",
                                "string",
                                "optional, same as read_file environment",
                                false,
                                None,
                            ),
                            param(
                                "type",
                                "string",
                                "operation type: replace | delete | create",
                                true,
                                None,
                            ),
                            param(
                                "old",
                                "string",
                                "the exact content to be matched and replaced/deleted (required for replace/delete)",
                                false,
                                None,
                            ),
                            param(
                                "new",
                                "string",
                                "the new content to insert (required for replace/create)",
                                false,
                                None,
                            ),
                        ],
                        details: [
                            "  - **How it works**:",
                            "    - The tool finds the best fuzzy match of `old` in the current file content (not by line numbers) and applies the requested operation.",
                            "    - You can call this tool multiple times to apply multiple independent edits.",
                            "",
                            "  - **Parameters**:",
                            "    - `type`:",
                            "      - `replace`: replace the matched `old` content with `new`",
                            "      - `delete`: delete the matched `old` content",
                            "      - `create`: create the file when it does not exist (write `new` as full file content)",
                            "    - `old`: required for `replace` / `delete`",
                            "    - `new`: required for `replace` / `create`",
                            "",
                            "  - **CRITICAL RULES**:",
                            "    1. **If you need to rewrite a whole existing file**: do **NOT** use apply_file to overwrite it. Instead, call `delete_file` first, then use `apply_file` with `type=create`.",
                            "    2. **If you need to modify an existing file**: you **MUST** use `type=replace` (or `type=delete`) and provide `old` / `new`. Do **NOT** delete the whole file and rewrite it.",
                        ]
                        .join("\n"),
                        notes: String::new(),
                    },
                ],
                category_footer: String::new(),
            },
            SystemToolPromptCategory {
                category_name: "Internal File Tools".to_string(),
                category_header: String::new(),
                tools: vec![
                    ToolPrompt {
                        name: "read_file_full".to_string(),
                        description:
                            "Read the full content of a file without enforcing size limit."
                                .to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "file path", true, None),
                            param(
                                "environment",
                                "string",
                                "optional, \"android\" (default) or \"linux\"",
                                false,
                                None,
                            ),
                            param("text_only", "boolean", "optional", false, Some("false")),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "read_file_binary".to_string(),
                        description: "Read binary file and return base64 content.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "file path", true, None),
                            param(
                                "environment",
                                "string",
                                "optional, \"android\" (default) or \"linux\"",
                                false,
                                None,
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "write_file".to_string(),
                        description: "Write content to a file.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "file path", true, None),
                            param("content", "string", "file content", true, None),
                            param("append", "boolean", "optional", false, Some("false")),
                            param(
                                "environment",
                                "string",
                                "optional, \"android\" (default) or \"linux\"",
                                false,
                                None,
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "write_file_binary".to_string(),
                        description: "Write base64 content to a binary file.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "file path", true, None),
                            param(
                                "base64Content",
                                "string",
                                "base64 encoded content",
                                true,
                                None,
                            ),
                            param(
                                "environment",
                                "string",
                                "optional, \"android\" (default) or \"linux\"",
                                false,
                                None,
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                ],
                category_footer: String::new(),
            },
            SystemToolPromptCategory {
                category_name: "Extended File Tools".to_string(),
                category_header: String::new(),
                tools: vec![
                    ToolPrompt {
                        name: "file_exists".to_string(),
                        description: "Check if a file or directory exists.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![param(
                            "path",
                            "string",
                            "target path",
                            true,
                            None,
                        )],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "move_file".to_string(),
                        description: "Move or rename a file or directory.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("source", "string", "source path", true, None),
                            param("destination", "string", "destination path", true, None),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "copy_file".to_string(),
                        description:
                            "Copy a file or directory. Supports cross-environment copying between Android and Linux."
                                .to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("source", "string", "source path", true, None),
                            param("destination", "string", "destination path", true, None),
                            param("recursive", "boolean", "boolean", false, Some("false")),
                            param(
                                "source_environment",
                                "string",
                                "optional, \"android\" or \"linux\"",
                                false,
                                Some("\"android\""),
                            ),
                            param(
                                "dest_environment",
                                "string",
                                "optional, \"android\" or \"linux\". For cross-environment copy (e.g., Android → Linux or Linux → Android), specify both source_environment and dest_environment",
                                false,
                                Some("\"android\""),
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "file_info".to_string(),
                        description: "Get detailed information about a file or directory including type, size, permissions, owner, group, and last modified time.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![param(
                            "path",
                            "string",
                            "target path",
                            true,
                            None,
                        )],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "zip_files".to_string(),
                        description: "Compress files or directories.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("source", "string", "path to compress", true, None),
                            param("destination", "string", "output zip file", true, None),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "unzip_files".to_string(),
                        description: "Extract a zip file.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("source", "string", "zip file path", true, None),
                            param("destination", "string", "extract path", true, None),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "open_file".to_string(),
                        description: "Open a file using the system's default application.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![param(
                            "path",
                            "string",
                            "file path",
                            true,
                            None,
                        )],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "share_file".to_string(),
                        description: "Share a file with other applications.".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "file path", true, None),
                            param(
                                "title",
                                "string",
                                "optional share title",
                                false,
                                Some("\"Share File\""),
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                ],
                category_footer: String::new(),
            },
            SystemToolPromptCategory {
                category_name: "Package Internal Tools".to_string(),
                category_header: String::new(),
                tools: vec![ToolPrompt {
                    name: "package_proxy".to_string(),
                    description: "Call a tool provided by an activated package.".to_string(),
                    parameters: String::new(),
                    parameters_structured: vec![
                        param("tool_name", "string", "actual package tool name", true, None),
                        param("params", "object", "target tool arguments as JSON object", true, None),
                    ],
                    details: String::new(),
                    notes: String::new(),
                }],
                category_footer: String::new(),
            },
        ]
    }

    #[allow(non_snake_case)]
    pub fn internalToolCategoriesCn() -> Vec<SystemToolPromptCategory> {
        vec![
            SystemToolPromptCategory {
                category_name: "内部工具".to_string(),
                category_header: String::new(),
                tools: vec![
                    ToolPrompt {
                        name: "execute_shell".to_string(),
                        description: "执行设备 Shell 命令。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![param(
                            "command",
                            "string",
                            "要执行的命令",
                            true,
                            None,
                        )],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "apply_file".to_string(),
                        description: "通过查找并替换/删除匹配的内容块来编辑文件。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "文件路径", true, None),
                            param(
                                "environment",
                                "string",
                                "可选，同 read_file 的 environment",
                                false,
                                None,
                            ),
                            param(
                                "type",
                                "string",
                                "操作类型：replace | delete | create",
                                true,
                                None,
                            ),
                            param(
                                "old",
                                "string",
                                "用于匹配/替换/删除的原始内容（replace/delete必填）",
                                false,
                                None,
                            ),
                            param(
                                "new",
                                "string",
                                "要插入的新内容（replace/create必填）",
                                false,
                                None,
                            ),
                        ],
                        details: [
                            "  - **工作原理**:",
                            "    - 工具会在文件当前内容中对 `old` 做最佳的模糊匹配（不依赖行号），然后执行指定操作。",
                            "    - 你可以多次调用本工具，对同一个文件做多处独立修改。",
                            "",
                            "  - **参数**:",
                            "    - `type`:",
                            "      - `replace`: 用 `new` 替换匹配到的 `old`",
                            "      - `delete`: 删除匹配到的 `old`",
                            "      - `create`: 当文件不存在时创建文件（用 `new` 作为完整文件内容）",
                            "    - `old`: `replace` / `delete` 必填",
                            "    - `new`: `replace` / `create` 必填",
                            "",
                            "  - **关键规则**:",
                            "    1. **如果需要重写整个已存在文件**：不要用 apply_file 直接覆盖。请先 `delete_file`，再使用 `apply_file` 且 `type=create`。",
                            "    2. **如果需要修改已存在文件**：必须用 `type=replace`（或 `type=delete`）并提供 `old/new`（或 `old`）。不要删除整个文件再重写。",
                        ]
                        .join("\n"),
                        notes: String::new(),
                    },
                ],
                category_footer: String::new(),
            },
            SystemToolPromptCategory {
                category_name: "内部文件工具".to_string(),
                category_header: String::new(),
                tools: vec![
                    ToolPrompt {
                        name: "read_file_full".to_string(),
                        description: "读取完整文件内容（不限制大小）。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "文件路径", true, None),
                            param(
                                "environment",
                                "string",
                                "可选，\"android\"（默认）或 \"linux\"",
                                false,
                                None,
                            ),
                            param("text_only", "boolean", "可选", false, Some("false")),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "read_file_binary".to_string(),
                        description: "读取二进制文件并返回 Base64 内容。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "文件路径", true, None),
                            param(
                                "environment",
                                "string",
                                "可选，\"android\"（默认）或 \"linux\"",
                                false,
                                None,
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "write_file".to_string(),
                        description: "写入文件内容。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "文件路径", true, None),
                            param("content", "string", "文件内容", true, None),
                            param("append", "boolean", "可选", false, Some("false")),
                            param(
                                "environment",
                                "string",
                                "可选，\"android\"（默认）或 \"linux\"",
                                false,
                                None,
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "write_file_binary".to_string(),
                        description: "将 Base64 内容写入二进制文件。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "文件路径", true, None),
                            param(
                                "base64Content",
                                "string",
                                "base64 编码内容",
                                true,
                                None,
                            ),
                            param(
                                "environment",
                                "string",
                                "可选，\"android\"（默认）或 \"linux\"",
                                false,
                                None,
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                ],
                category_footer: String::new(),
            },
            SystemToolPromptCategory {
                category_name: "扩展文件工具".to_string(),
                category_header: String::new(),
                tools: vec![
                    ToolPrompt {
                        name: "file_exists".to_string(),
                        description: "检查文件或目录是否存在。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![param(
                            "path",
                            "string",
                            "目标路径",
                            true,
                            None,
                        )],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "move_file".to_string(),
                        description: "移动或重命名文件或目录。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("source", "string", "源路径", true, None),
                            param("destination", "string", "目标路径", true, None),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "copy_file".to_string(),
                        description: "复制文件或目录。支持Android和Linux之间的跨环境复制。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("source", "string", "源路径", true, None),
                            param("destination", "string", "目标路径", true, None),
                            param("recursive", "boolean", "布尔值", false, Some("false")),
                            param(
                                "source_environment",
                                "string",
                                "可选，\"android\"或\"linux\"",
                                false,
                                Some("\"android\""),
                            ),
                            param(
                                "dest_environment",
                                "string",
                                "可选，\"android\"或\"linux\"。跨环境复制（如Android → Linux或Linux → Android）时，需指定source_environment和dest_environment",
                                false,
                                Some("\"android\""),
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "file_info".to_string(),
                        description: "获取文件或目录的详细信息，包括类型、大小、权限、所有者、组和最后修改时间。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![param(
                            "path",
                            "string",
                            "目标路径",
                            true,
                            None,
                        )],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "zip_files".to_string(),
                        description: "压缩文件或目录。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("source", "string", "要压缩的路径", true, None),
                            param("destination", "string", "输出zip文件", true, None),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "unzip_files".to_string(),
                        description: "解压zip文件。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("source", "string", "zip文件路径", true, None),
                            param("destination", "string", "解压路径", true, None),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "open_file".to_string(),
                        description: "使用系统默认应用程序打开文件。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![param(
                            "path",
                            "string",
                            "文件路径",
                            true,
                            None,
                        )],
                        details: String::new(),
                        notes: String::new(),
                    },
                    ToolPrompt {
                        name: "share_file".to_string(),
                        description: "与其他应用程序共享文件。".to_string(),
                        parameters: String::new(),
                        parameters_structured: vec![
                            param("path", "string", "文件路径", true, None),
                            param(
                                "title",
                                "string",
                                "可选的共享标题",
                                false,
                                Some("\"Share File\""),
                            ),
                        ],
                        details: String::new(),
                        notes: String::new(),
                    },
                ],
                category_footer: String::new(),
            },
            SystemToolPromptCategory {
                category_name: "包内部工具".to_string(),
                category_header: String::new(),
                tools: vec![ToolPrompt {
                    name: "package_proxy".to_string(),
                    description: "调用已激活包提供的工具。".to_string(),
                    parameters: String::new(),
                    parameters_structured: vec![
                        param("tool_name", "string", "真实包工具名", true, None),
                        param("params", "object", "目标工具参数 JSON 对象", true, None),
                    ],
                    details: String::new(),
                    notes: String::new(),
                }],
                category_footer: String::new(),
            },
        ]
    }
}

fn param(
    name: &str,
    value_type: &str,
    description: &str,
    required: bool,
    default: Option<&str>,
) -> ToolParameterSchema {
    ToolParameterSchema {
        name: name.to_string(),
        value_type: value_type.to_string(),
        description: description.to_string(),
        required,
        default: default.map(ToOwned::to_owned),
    }
}
