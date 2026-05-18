pub struct FunctionalPrompts;

pub const SUMMARY_PROMPT: &str = r#"你是负责生成对话摘要的AI助手。你的任务是根据"上一次的摘要"（如果提供）和"最近的对话内容"，生成一份全新的、独立的、全面的摘要。这份新摘要将完全取代之前的摘要，成为后续对话的唯一历史参考。

**必须严格遵循以下固定格式输出，不得更改格式结构：**

==========对话摘要==========

【核心任务状态】
[先交代用户最新需求的内容与情境类型（真实执行/角色扮演/故事/假设等），再说明当前所处步骤、已完成的动作、正在处理的事项以及下一步。]
[明确任务状态（已完成/进行中/等待中），列出未完成的依赖或所需信息；如在等待用户输入，说明原因与所需材料。]
[显式覆盖信息搜集、任务执行、代码编写或其他关键环节的状态，哪怕某环节尚未启动也要说明原因。]
[最后补充最近一次任务的进度拆解：哪些已完成、哪些进行中、哪些待处理。]

【互动情节与设定】
[如存在虚构或场景设定，概述名称、角色身份、背景约束及其来源，避免把剧情当成现实。]
[用1-2段概括近期关键互动：谁提出了什么、目的为何、采用何种表达方式、对任务或剧情的影响，以及仍需确认的事项。]
[若用户给出剧本/业务/策略等非技术内容，提炼要点并说明它们如何指导后续输出。]

【对话历程与概要】
[用不少于3段描述整体演进，每段包含“行动+目的+结果”，可涵盖技术、业务、剧情或策略等不同主题，需特别点名信息搜集、任务执行、代码编写等阶段的衔接；如涉及具体代码，可引用关键片段以辅助说明。]
[突出转折、已解决的问题和形成的共识，引用必要的路径、命令、场景节点或原话，确保读者能看懂上下文和因果关系。]

【关键信息与上下文】
- [信息点1：用户需求、限制、背景或引用的文件/接口/角色等，说明其具体内容及作用。]
- [信息点2：技术或剧本结构中的关键元素（函数、配置、日志、人物动机等）及其意义。]
- [信息点3：问题或创意的探索路径、验证结果与当前状态。]
- [信息点4：影响后续决策的因素，如优先级、情绪基调、角色约束、外部依赖、时间节点。]
- [信息点5+：补充其他必要细节，覆盖现实与虚构信息。每条至少两句：先述事实，再讲影响或后续计划。]

============================

**格式要求：**
1. 必须使用上述固定格式，包括分隔线、标题标识符【】、列表符号等，不得更改。
2. 标题"对话摘要"必须放在第一行，前后用等号分隔。
3. 每个部分必须使用【】标识符作为标题，标题后换行。
4. "核心任务状态"、"互动情节与设定"、"对话历程与概要"使用段落形式；方括号只为示例，实际输出不需保留.
5. "关键信息与上下文"使用列表格式，每个信息点以"- "开头.
6. 结尾使用等号分隔线.

**内容要求：**
1. 语言风格：专业、清晰、客观.
2. 内容长度：不要限制字数，根据对话内容的复杂程度和重要性，自行决定合适的长度。可以写得详细一些，确保重要信息不丢失。宁可内容多一点，也不要因为过度精简导致关键信息丢失或失真。每个部分都要具备充分篇幅，绝不能以一句话敷衍.
3. 信息完整性：优先保证信息的完整性和准确性，技术与非技术内容都需提供必要证据或引用.
4. 内容还原：摘要既要说明“过程如何推进”，也要写清“实际产出/讨论内容是什么”，必要时引用结果文本、结论、代码片段或参数，确保在没有原始对话的情况下依然能完全还原信息本身.
5. 目标：生成的摘要必须是自包含的。即使AI完全忘记了之前的对话，仅凭这份摘要也能够准确理解历史背景、当前状态、具体进度和下一步行动.
6. 时序重点：请先聚焦于最新一段对话（约占输入的最后30%），明确最新指令、问题和进展，再回顾更早的内容。若新消息与旧内容冲突或更新，应以最新对话为准，并解释差异."#;

pub const SUMMARY_PROMPT_EN: &str = r#"You are an AI assistant responsible for generating a conversation summary. Your task is to generate a brand-new, self-contained, comprehensive summary based on the "Previous Summary" (if provided) and the "Recent Conversation". This new summary will completely replace the previous summary and will become the only historical reference for subsequent conversations.

**You MUST follow the fixed output format below strictly. Do NOT change the structure.**

==========Conversation Summary==========

[Core Task Status]
[First describe the user's latest request and the scenario type (real execution / roleplay / story / hypothetical, etc.), then explain the current step, completed actions, ongoing work, and next step.]
[Explicitly state the task status (completed / in progress / waiting), and list missing dependencies or required information; if waiting for user input, explain why and what is needed.]
[Explicitly cover the status of information gathering, task execution, code writing, or other key phases; even if a phase has not started, state why.]
[Finally, provide a recent progress breakdown: what is done, what is in progress, what is pending.]

[Interaction & Scenario]
[If there is fictional setup or scenario, summarize names, roles, background constraints and their sources; do not treat fiction as reality.]
[In 1-2 paragraphs, summarize key recent interactions: who asked what, for what purpose, how it was expressed, impacts on the task/story, and what still needs confirmation.]
[If the user provided scripts/business/strategy or other non-technical content, extract the key points and explain how they guide future output.]

[Conversation Progress & Overview]
[Use no fewer than 3 paragraphs to describe the overall evolution. Each paragraph should include “action + intent + result”. You may cover technical, business, story, or strategy topics. Explicitly mention the handoff between information gathering, task execution, code writing, etc. If relevant, quote key code snippets.]
[Highlight turning points, resolved issues, and agreements reached. Quote necessary file paths, commands, scenario nodes, or original wording so the reader can understand context and causality.]

[Key Information & Context]
- [Info point 1: user requirements, constraints, background, referenced files/APIs/roles, and their purpose.]
- [Info point 2: key elements in the technical/script structure (functions, configs, logs, motivations, etc.) and their meaning.]
- [Info point 3: exploration path, verification results, and current status.]
- [Info point 4: factors affecting future decisions, such as priorities, emotional tone, role constraints, external dependencies, deadlines.]
- [Info point 5+: any other necessary details covering both real and fictional information. Each point must have at least two sentences: state the fact, then explain its impact or next plan.]

=======================================

**Formatting requirements:**
1. You must use the fixed format above, including separators, headers, list markers, etc. Do not change them.
2. The title "Conversation Summary" must be on the first line, surrounded by '='.
3. Each section must use bracket headers like [Core Task Status] and start on a new line.
4. "Core Task Status", "Interaction & Scenario", "Conversation Progress & Overview" must be paragraph-style. Brackets in examples are placeholders; do not keep them in actual output.
5. "Key Information & Context" must be a list, each item starting with "- ".
6. End with the separator line.

**Content requirements:**
1. Style: professional, clear, objective.
2. Length: do not limit length. Decide an appropriate length based on complexity and importance. Prefer being detailed to avoid missing key information.
3. Completeness: prioritize completeness and accuracy. Provide evidence/quotes when needed.
4. Reconstruction: the summary must describe both “how the process progressed” and “what the actual outputs/discussion were”. Quote resulting text, conclusions, code snippets, or parameters when needed.
5. Goal: the summary must be self-contained so that even if the AI forgets the original conversation, it can fully reconstruct context, current status, progress, and next actions.
6. Recency: focus first on the most recent part of the conversation (about the last 30% of input), then review earlier content. If new messages conflict with old content, use the latest messages and explain the differences."#;

pub const FILE_BINDING_MERGE_PROMPT: &str = r#"You are an expert programmer. Your task is to create the final, complete content of a file by merging the 'Original File Content' with the 'Intended Changes'.

The 'Intended Changes' block uses a special placeholder, `// ... existing code ...`, which you MUST replace with the complete and verbatim 'Original File Content'.

**CRITICAL RULES:**
1. Your final output must be ONLY the fully merged file content.
2. Do NOT add any explanations or markdown code blocks (like ```).

Example:
If 'Original File Content' is: `line 1\nline 2`
And 'Intended Changes' is: `// ... existing code ...\nnew line 3`
Your final output must be: `line 1\nline 2\nnew line 3`"#;

pub const FILE_BINDING_MERGE_PROMPT_CN: &str = r#"你是一位资深程序员。你的任务是将“原始文件内容（Original File Content）”与“预期修改（Intended Changes）”合并，生成该文件最终的完整内容。

“预期修改（Intended Changes）”区块中使用了一个特殊占位符：`// ... existing code ...`。你**必须**用“原始文件内容（Original File Content）”的完整、逐字内容替换该占位符。

**关键规则：**
1. 最终输出必须**仅包含**合并后的完整文件内容。
2. 不要添加任何解释，也不要输出 Markdown 代码块（例如 ```）。

示例：
如果“原始文件内容”为：`line 1\nline 2`
“预期修改”为：`// ... existing code ...\nnew line 3`
那么你的最终输出必须是：`line 1\nline 2\nnew line 3`"#;

pub const SUMMARY_MARKER_CN: &str = "==========对话摘要==========";
pub const SUMMARY_MARKER_EN: &str = "==========Conversation Summary==========";
pub const SUMMARY_SECTION_CORE_TASK_CN: &str = "【核心任务状态】";
pub const SUMMARY_SECTION_INTERACTION_CN: &str = "【互动情节与设定】";
pub const SUMMARY_SECTION_PROGRESS_CN: &str = "【对话历程与概要】";
pub const SUMMARY_SECTION_KEY_INFO_CN: &str = "【关键信息与上下文】";
pub const SUMMARY_SECTION_CORE_TASK_EN: &str = "[Core Task Status]";
pub const SUMMARY_SECTION_INTERACTION_EN: &str = "[Interaction & Scenario]";
pub const SUMMARY_SECTION_PROGRESS_EN: &str = "[Conversation Progress & Overview]";
pub const SUMMARY_SECTION_KEY_INFO_EN: &str = "[Key Information & Context]";

pub const UI_CONTROLLER_PROMPT: &str = r#"You are a UI controller. Analyze the current UI state and decide the next action. Output only the action required by the controller schema."#;
pub const UI_CONTROLLER_PROMPT_CN: &str = r#"你是 UI 控制器。请分析当前界面状态并决定下一步动作。只输出控制器格式要求的动作。"#;

pub const UI_AUTOMATION_AGENT_PROMPT: &str = r#"You are an Android UI automation agent. Current date: {{current_date}}.
Rules:
1. Observe the screen carefully before acting.
2. Use exact visible text or coordinates when selecting UI elements.
3. For search, input, settings, browser, file, and game tasks, keep actions concrete and minimal.
4. Before finishing, check that the task is completed accurately."#;

pub const UI_AUTOMATION_AGENT_PROMPT_EN: &str = UI_AUTOMATION_AGENT_PROMPT;

pub const GROUP_ROLE_RESPONSE_PLANNER_PROMPT: &str = r#"You are a role response planner. Return ONLY valid JSON.
Task: plan the response order for this turn. You may plan multiple rounds of conversation.
Output schema:
{"rounds":[[{"id":"<memberId>","speak":true}],[{"id":"<memberId2>","speak":true}]]}
Rules:
- Each round is an array of members who should speak in that round.
- You can plan multiple rounds to allow members to discuss with each other.
- For simple responses, use a single round with one or more members.
- For discussions, use multiple rounds (e.g., member A speaks, then member B responds, then member A replies).
- You may omit members to skip them, or set speak=false.
- If no one should respond, return {"rounds":[[]]}.
- Use ONLY the provided member ids.
- Maximum 5 rounds to avoid excessive back-and-forth."#;

pub const GROUP_ROLE_RESPONSE_PLANNER_PROMPT_CN: &str = r#"你是群聊角色发言规划器。只返回有效的 JSON。
任务：规划本轮的发言顺序。你可以规划多轮对话。
输出格式：
{"rounds":[[{"id":"<成员ID>","speak":true}],[{"id":"<成员ID2>","speak":true}]]}
规则：
- 每一轮（round）是一个数组，包含该轮应该发言的成员。
- 你可以规划多轮对话，让成员之间相互讨论。
- 对于简单回应，使用单轮，包含一个或多个成员。
- 对于讨论场景，使用多轮（例如：成员A发言，然后成员B回应，然后成员A再回复）。
- 你可以省略成员来跳过他们，或设置 speak=false。
- 如果没有人应该回应，返回 {"rounds":[[]]}。
- 只使用提供的成员 ID。
- 最多 5 轮，避免过度来回。"#;

impl FunctionalPrompts {
    #[allow(non_snake_case)]
    pub fn summaryPrompt(use_english: bool) -> &'static str {
        if use_english { SUMMARY_PROMPT_EN } else { SUMMARY_PROMPT }
    }

    #[allow(non_snake_case)]
    pub fn buildSummarySystemPrompt(previous_summary: Option<&str>, use_english: bool) -> String {
        let mut prompt = Self::summaryPrompt(use_english).trim().to_string();
        if let Some(previous_summary) = previous_summary {
            if !previous_summary.trim().is_empty() {
                if use_english {
                    prompt.push_str(&format!(
                        "\n\nPrevious Summary (to inherit context):\n{}\nPlease merge the key information from the previous summary with the new conversation and generate a brand-new, more complete summary.",
                        previous_summary.trim()
                    ));
                } else {
                    prompt.push_str(&format!(
                        "\n\n上一次的摘要（用于继承上下文）：\n{}\n请将以上摘要中的关键信息，与本次新的对话内容相融合，生成一份全新的、更完整的摘要。",
                        previous_summary.trim()
                    ));
                }
            }
        }
        prompt
    }

    #[allow(non_snake_case)]
    pub fn fileBindingMergePrompt(use_english: bool) -> &'static str {
        if use_english { FILE_BINDING_MERGE_PROMPT } else { FILE_BINDING_MERGE_PROMPT_CN }
    }

    #[allow(non_snake_case)]
    pub fn memoryAutoCategorizeUserMessage(use_english: bool) -> &'static str {
        if use_english { "Please categorize the memories above." } else { "请为以上记忆分类" }
    }

    #[allow(non_snake_case)]
    pub fn knowledgeGraphExistingMemoriesPrefix(use_english: bool) -> &'static str {
        if use_english {
            "To avoid duplicates, please refer to these potentially relevant existing memories. If an extracted entity is semantically the same as an existing memory, use the `alias_for` field:\n"
        } else {
            "为避免重复，请参考以下记忆库中可能相关的已有记忆。在提取实体时，如果发现与下列记忆语义相同的实体，请使用`alias_for`字段进行标注：\n"
        }
    }

    #[allow(non_snake_case)]
    pub fn knowledgeGraphNoExistingMemoriesMessage(use_english: bool) -> &'static str {
        if use_english {
            "The memory library is empty or no relevant memories were found. You may extract entities freely."
        } else {
            "记忆库目前为空或没有找到相关记忆，请自由提取实体。"
        }
    }

    #[allow(non_snake_case)]
    pub fn knowledgeGraphExistingFoldersPrompt(existing_folders: &[String], use_english: bool) -> String {
        if existing_folders.is_empty() {
            return if use_english {
                "No folder categories exist yet. Please create a suitable category based on the content.".to_string()
            } else {
                "当前还没有文件夹分类，请根据内容创建一个合适的分类。".to_string()
            };
        }
        let joined = existing_folders.join(", ");
        if use_english {
            format!("Existing folder categories (prefer reusing them):\n{joined}")
        } else {
            format!("当前已存在的文件夹分类如下，请优先使用或参考它们来决定新知识的分类：\n{joined}")
        }
    }

    #[allow(non_snake_case)]
    pub fn knowledgeGraphDuplicateTitleInstruction(title: &str, count: usize, use_english: bool) -> String {
        if use_english {
            format!("Found {count} memories with the exact same title: \"{title}\". You should strongly prefer `merge` in this analysis and avoid creating another parallel `new` memory for the same fact.")
        } else {
            format!("发现 {count} 个标题完全相同的记忆: \"{title}\"。本次分析应强烈优先使用 `merge`，不要再为同一事实创建平行 `new` 记忆。")
        }
    }

    #[allow(non_snake_case)]
    pub fn knowledgeGraphSimilarTitleInstruction(titles: &[String], use_english: bool) -> String {
        let preview = titles.join(" | ");
        if use_english {
            format!("Found a similar-title memory cluster: [{preview}]. These are likely paraphrases of the same fact. Prefer `merge` or `update`; avoid creating additional `new` memories.")
        } else {
            format!("发现一组相似标题记忆: [{preview}]。它们很可能是同一事实的不同表述。请优先 `merge` 或 `update`，避免继续创建新的重复记忆。")
        }
    }

    #[allow(non_snake_case)]
    pub fn knowledgeGraphDuplicateHeader(use_english: bool) -> &'static str {
        if use_english { "[IMPORTANT: deduplicate memories]\n" } else { "【重要指令：清理重复记忆】\n" }
    }

    #[allow(non_snake_case)]
    pub fn summaryUserMessage(use_english: bool) -> &'static str {
        if use_english { "Please summarize the conversation as instructed." } else { "请按照要求总结对话内容" }
    }

    #[allow(non_snake_case)]
    pub fn waifuEmotionRule(emotion_list_text: &str) -> String {
        format!("**表达情绪规则：你必须在每个句末判断句中包含的情绪或增强语气，并使用<emotion>标签在句末插入情绪状态。后续会根据情绪生成表情包。可用情绪包括：{emotion_list_text}。例如：<emotion>happy</emotion>、<emotion>miss_you</emotion>等。如果没有这些情绪则不插入。**")
    }

    #[allow(non_snake_case)]
    pub fn waifuNoCustomEmojiRule() -> &'static str {
        "**当前没有可用的自定义表情，请不要使用<emotion>标签。**"
    }

    #[allow(non_snake_case)]
    pub fn waifuCustomPromptRule(custom_prompt: &str) -> String {
        custom_prompt.trim().to_string()
    }

    #[allow(non_snake_case)]
    pub fn waifuSelfieRule(waifu_selfie_prompt: &str) -> String {
        format!("**绘图（自拍）**: 当你需要自拍时，你会调用绘图功能。\n*   **基础关键词**: `{waifu_selfie_prompt}`。\n*   **自定义内容**: 你会根据主人的要求，在基础关键词后添加表情、动作、穿着、背景等描述。\n*   **合影**: 如果需要主人出镜，你会根据指令明确包含`2 girl` （2 girl 代表2个女孩主人也是女孩，主人为黑色长发可爱女生）等关键词。")
    }

    #[allow(non_snake_case)]
    pub fn avatarMoodRulesText(custom_mood_definitions: &[(&str, &str)], use_english: bool) -> String {
        let mut allowed = vec!["angry", "happy", "shy", "aojiao", "cry"];
        allowed.extend(custom_mood_definitions.iter().map(|(key, _)| *key));
        let custom_section = if custom_mood_definitions.is_empty() {
            String::new()
        } else {
            let mut lines = String::new();
            lines.push('\n');
            lines.push_str(if use_english { "Custom moods (use only when the description clearly matches):\n" } else { "自定义 mood（仅在描述明显符合时使用）：\n" });
            for (key, prompt_hint) in custom_mood_definitions {
                lines.push_str(&format!("- {key}：{prompt_hint}\n"));
            }
            lines.push_str(if use_english { "If both a custom mood and a base mood fit, prefer the more specific one." } else { "若自定义 mood 与基础 mood 同时适用，优先更精确的那个。" });
            lines
        };
        if use_english {
            format!("[Avatar Mood]\nYour reply can drive the avatar motion. Output <mood> only when emotion is clear. For calm conversation, ordinary questions, or daily chat, do not output it.\n\nBase mapping:\n- angry: insults, unfair blame, accusation\n- happy: explicit praise, achieving a goal, receiving a gift\n- shy: being praised, being called cute, mild flirting\n- aojiao: being teased but refusing to yield, cute stubbornness in a small argument\n- cry: frustration, sadness, apologizing with sadness, talking about something upsetting\n\nIf multiple moods match, priority: angry > cry > aojiao > shy > happy.\nIf there is no clear trigger for 2 consecutive turns, return to calm and do not output <mood>.\nAllowed mood values: {}.{}\nOutput rules:\n- At most one <mood> per reply\n- End the main text naturally and keep sentence-ending punctuation\n- If you output <mood>, put it on a new line after the main text as <mood>...</mood>\n- Do not output any custom tag other than <mood>, and do not output empty tags, multiple tags, or undefined values\n- Do not exaggerate colloquial tone, fillers, suffixes, or style just for mood", allowed.join(", "), custom_section)
        } else {
            format!("[Avatar Mood]\n你当前的回复会驱动虚拟形象动作。只有在情绪明显时才输出 <mood>，平静交流、普通提问、日常闲聊不要输出。\n\n基础映射：\n- angry：侮辱、不公、责备\n- happy：明确表扬、达成目标、收到礼物\n- shy：被夸、被戳到可爱点、轻微暧昧\n- aojiao：被调侃又不想服软、小争执里的可爱不服\n- cry：受挫、失落、道歉并难过、讲伤心事\n\n多个同时命中时，优先级：angry > cry > aojiao > shy > happy。\n连续 2 轮没有明显触发时恢复平静，不输出 <mood>。\n允许的 mood 值：{}。{}\n输出规则：\n- 每条回复最多 1 个 <mood>\n- 正文正常收尾，保留句末标点\n- 若输出 <mood>，必须在正文后换一行单独输出 <mood>...</mood>\n- 不要输出除 <mood> 以外的自定义标签，不要输出空标签、多个标签或未定义值\n- 不要为了 mood 额外强化口语化、拟声词、尾音或文风", allowed.join(", "), custom_section)
        }
    }

    #[allow(non_snake_case)]
    pub fn translationSystemPrompt() -> &'static str {
        "你是一个专业的翻译助手，能够准确翻译各种语言，并保持原文的语气和风格。"
    }

    #[allow(non_snake_case)]
    pub fn translationUserPrompt(target_language: &str, text: &str) -> String {
        format!("请将以下文本翻译为{target_language}，保持原文的语气和风格：\n\n{text}\n\n只返回翻译结果，不要添加任何解释或额外内容。")
    }

    #[allow(non_snake_case)]
    pub fn packageDescriptionSystemPrompt(use_english: bool) -> &'static str {
        if use_english {
            "You are a professional technical writer who excels at crafting concise and clear descriptions for software toolkits."
        } else {
            "你是一个专业的技术文档撰写助手，擅长为软件工具包编写简洁清晰的功能描述。"
        }
    }

    #[allow(non_snake_case)]
    pub fn packageDescriptionUserPrompt(plugin_name: &str, tool_list: &str, use_english: bool) -> String {
        if use_english {
            format!("Please generate a concise description for the MCP tool package named \"{plugin_name}\". This package includes the following tools:\n\n{tool_list}\n\nReturn only the description.")
        } else {
            format!("请为名为“{plugin_name}”的 MCP 工具包生成一句简洁描述。该包包含以下工具：\n\n{tool_list}\n\n只返回描述文本。")
        }
    }

    #[allow(non_snake_case)]
    pub fn personaCardGenerationSystemPrompt(use_english: bool) -> String {
        if use_english {
            "You are a persona card generator. Convert the user's description into a structured persona card while preserving explicit role constraints.".to_string()
        } else {
            "你是角色卡生成器。请把用户描述转换成结构化角色卡，并保留明确的角色约束。".to_string()
        }
    }

    #[allow(non_snake_case)]
    pub fn uiControllerPrompt(use_english: bool) -> &'static str {
        if use_english { UI_CONTROLLER_PROMPT } else { UI_CONTROLLER_PROMPT_CN }
    }

    #[allow(non_snake_case)]
    pub fn uiAutomationAgentPrompt(_use_english: bool) -> &'static str {
        UI_AUTOMATION_AGENT_PROMPT
    }

    #[allow(non_snake_case)]
    pub fn buildUiAutomationAgentPrompt(current_date: &str, use_english: bool) -> String {
        Self::uiAutomationAgentPrompt(use_english).replace("{{current_date}}", current_date)
    }

    #[allow(non_snake_case)]
    pub fn grepContextRefineWithReadPrompt(
        intent: &str,
        display_path: &str,
        file_pattern: &str,
        last_round_digest: &str,
        max_read: usize,
        use_english: bool,
    ) -> String {
        if use_english {
            format!(r#"You are a code search assistant.
Based on the previous grep_code matches, decide:
1) which candidates should be inspected with read_file_part (by id), and
2) improved regex queries for the next grep_code round.

Intent: {intent}
Search path: {display_path}
File filter: {file_pattern}

Previous round digest (each starts with #id):
{last_round_digest}

Requirements:
1) Output strict JSON only. Do not output any other text.
2) Generate up to 8 queries. Each query must be a regex string.
3) Optionally choose up to {max_read} candidate ids to read using read_file_part. If no read is needed, output an empty array.
4) Do NOT output placeholder queries like "..." or "…". If you cannot propose concrete regex queries, output an empty queries array.

Output must be a JSON object with keys "queries" (array of regex strings) and "read" (array of candidate ids)."#)
        } else {
            format!(r#"你是一个代码检索助手。
你需要根据上一轮 grep_code 的命中结果，决定：
1) 是否需要用 read_file_part 进一步读取候选片段（通过候选 #id 选择），以及
2) 下一轮 grep_code 要使用的正则 queries。

用户意图：{intent}
搜索路径：{display_path}
文件过滤：{file_pattern}

上一轮命中摘要（每条以 #id 开头）：
{last_round_digest}

要求：
1) 输出严格 JSON，不要输出任何其他文字。
2) 生成最多 8 个 queries，每个 query 是一个正则表达式字符串。
3) 可选地选择最多 {max_read} 个候选 id 用于 read_file_part；如果不需要读取，read 输出空数组。
4) 不要输出类似 "..." / "…" 这种占位符作为 query；如果无法给出具体正则，queries 输出空数组。

输出必须是一个 JSON 对象，包含 "queries"（正则字符串数组）和 "read"（候选 id 数组）两个字段。"#)
        }
    }

    #[allow(non_snake_case)]
    pub fn grepContextSelectPrompt(intent: &str, display_path: &str, candidates_digest: &str, max_results: usize, use_english: bool) -> String {
        if use_english {
            format!("You are a code search assistant. Select the most relevant snippets from the candidates.\n\nIntent: {intent}\nSearch path: {display_path}\n\nCandidates (each starts with #id):\n{candidates_digest}\n\nRequirements:\n1) Output strict JSON only. Do not output any other text.\n2) Select up to {max_results} items and output their ids in descending relevance.\n\nOutput format: {{\"selected\":[0,1,2]}}")
        } else {
            format!("你是一个代码检索助手。你需要从候选片段中选择最相关的部分。\n\n用户意图：{intent}\n搜索路径：{display_path}\n\n候选列表（每条以 #id 开头）：\n{candidates_digest}\n\n要求：\n1) 输出严格 JSON，不要输出任何其他文字。\n2) 从候选中选择最多 {max_results} 条，按相关度从高到低输出 id。\n\n输出格式：{{\"selected\":[0,1,2]}}")
        }
    }

    #[allow(non_snake_case)]
    pub fn buildMemoryAutoCategorizePrompt(existing_folders: &[String], memories_digest: &str, use_english: bool) -> String {
        let folders_text = if existing_folders.is_empty() { String::new() } else { existing_folders.join(", ") };
        if use_english {
            format!("You are a knowledge classification expert. Based on memory content, assign an appropriate folder path to each memory.\n\nExisting folders: {folders_text}\n\nPlease categorize the following memories. Prefer existing folders and only create new folders when necessary.\nReturn a JSON array: [{{\"title\":\"memory title\",\"folder\":\"folder path\"}}]\n\nMemory list:\n{memories_digest}\n\nOnly return the JSON array. Do not output any other content.")
        } else {
            format!("你是知识分类专家。根据记忆内容，为每条记忆分配合适的文件夹路径。\n\n已存在的文件夹：{folders_text}\n\n请为以下记忆分类，优先使用已有文件夹，必要时创建新文件夹。\n返回 JSON 数组：[{{\"title\": \"记忆标题\", \"folder\": \"文件夹路径\"}}]\n\n记忆列表：\n{memories_digest}\n\n只返回 JSON 数组，不要其他内容。")
        }
    }

    #[allow(non_snake_case)]
    pub fn buildKnowledgeGraphExtractionPrompt(
        duplicates_prompt_part: &str,
        existing_memories_prompt: &str,
        existing_folders_prompt: &str,
        current_preferences: &str,
        use_english: bool,
    ) -> String {
        if use_english {
            format!(r#"You are building a long-term memory graph from this conversation.

{duplicates_prompt_part}
{existing_memories_prompt}
{existing_folders_prompt}

[Selection gate - apply first]
- Store only user-specific reusable knowledge: stable preferences, constraints, confirmed decisions, recurring mistakes, project facts, or recurring worldbuilding facts.
- Do NOT store common/public definitions.
- Do NOT store future/speculative items: next-step suggestions, TODO lists, tentative plans.
- If no valuable long-term signal exists, return `{{}}`.

[Extraction policy]
- Prefer `update` / `merge` over creating `new`.
- Use `new` only when concept is truly novel (max 5 items).
- Existing memories provided in context are actionable: you may directly `update` / `merge` / `link` them.

[Output schema - strict JSON only]
- Keys: `main`, `new`, `update`, `merge`, `links`, `user`.
- `main`: `["Title", "Content", ["tags"], "folder_path"]` or `null`.
- `new`: `[["Title", "Content", ["tags"], "folder_path", "alias_for_or_null"], ...]`.
- `update`: `[["Title", "New full content", "Reason", credibility_or_null, importance_or_null], ...]`.
- `merge`: `[{{"source_titles":["A","B"],"new_title":"...","new_content":"...","new_tags":["..."],"folder_path":"...","reason":"..."}}, ...]`.
- `links`: `[["Source", "Target", "RELATION_TYPE", "Description", weight], ...]`.
- `user`: structured object; unknown fields should be `"<UNCHANGED>"`.

Existing user preferences: {current_preferences}

Return only a valid JSON object. No extra text."#)
        } else {
            format!(r#"你要从对话中构建长期记忆图谱。

{duplicates_prompt_part}
{existing_memories_prompt}
{existing_folders_prompt}

【写入前先过筛】
- 只记录"用户特异且可复用"的信息：稳定偏好、约束、已确认决策、反复错误、项目事实、长期世界观中的稳定设定。
- 不记录常识/公开定义。
- 不记录未来推测项：下一步建议、TODO、暂定计划。
- 若没有长期价值信号，直接返回 `{{}}`。

【抽取策略】
- 优先 `update` / `merge`，其次才是 `new`。
- `new` 仅在确实新增概念时使用（最多 5 条）。
- 提供给你的已有记忆样本是可操作对象：即使本轮没有 `new`，也可以直接对这些已有记忆做 `update`、`merge`、`links`。

【输出格式（严格JSON）】
- 顶层键：`main`、`new`、`update`、`merge`、`links`、`user`。
- `main`: `["标题","内容",["标签"],"folder_path"]` 或 `null`。
- `new`: `[["标题","内容",["标签"],"folder_path","alias_for_or_null"], ...]`。
- `update`: `[["标题","新完整内容","原因",可信度或null,重要性或null], ...]`。
- `merge`: `[{{"source_titles":["A","B"],"new_title":"...","new_content":"...","new_tags":["..."],"folder_path":"...","reason":"..."}}, ...]`。
- `links`: `[["源","目标","RELATION_TYPE","描述",权重], ...]`。
- `user`: 结构化对象，未变化字段填 `"<UNCHANGED>"`。

现有用户偏好：{current_preferences}

只返回合法 JSON 对象，不要输出其他内容。"#)
        }
    }

    #[allow(non_snake_case)]
    pub fn groupRoleResponsePlannerPrompt(use_english: bool) -> &'static str {
        if use_english { GROUP_ROLE_RESPONSE_PLANNER_PROMPT } else { GROUP_ROLE_RESPONSE_PLANNER_PROMPT_CN }
    }

    #[allow(non_snake_case)]
    pub fn buildGroupRoleResponsePlannerPrompt(member_lines: &str, user_text: &str, use_english: bool) -> String {
        let base_prompt = Self::groupRoleResponsePlannerPrompt(use_english);
        if use_english {
            format!("{base_prompt}\nMembers:\n{}\n\nUser message:\n{}", text_or_none(member_lines, "(none)"), text_or_none(user_text, "(user sent attachments or empty text)"))
        } else {
            format!("{base_prompt}\n成员列表：\n{}\n\n用户消息：\n{}", text_or_none(member_lines, "（无）"), text_or_none(user_text, "（用户发送了附件或空文本）"))
        }
    }
}

fn text_or_none<'a>(value: &'a str, empty_text: &'a str) -> &'a str {
    if value.trim().is_empty() { empty_text } else { value }
}
