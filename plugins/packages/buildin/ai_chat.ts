/* METADATA
{
  "name": "ai_chat",

  "display_name": {
      "zh": "AI 相互对话",
      "en": "AI-to-AI Chat"
  },
  "description": {
    "zh": "调用AI模型API实现AI之间智能对话互动。",
    "en": "Call an AI model API to enable interactive conversations between AIs."
  },
  "env": ["AI_API_BASE_URL", "AI_API_KEY", "AI_MODEL_NAME"],
  "category": "Chat",
  "tools": [{
    "name": "chat_completion",
    "display": {"label": {"zh": "调用 AI 对话", "en": "Request AI completion"}},
    "description": {
      "zh": "发送消息给AI模型并获取回复。支持超时保护（默认30秒）。从根源上禁止分点列表输出。",
      "en": "Send messages to an AI model and get responses. Includes timeout protection (default: 30s). Enforces non-bulleted output from the source."
    },
    "parameters": [
      {"name": "messages", "description": {"zh": "消息数组或字符串", "en": "Message array or a string"}, "type": "any", "required": true},
      {"name": "system_prompt", "description": {"zh": "系统提示词（会自动追加禁止分点指令）", "en": "System prompt (the system will automatically append non-bulleted instructions)"}, "type": "string", "required": false},
      {"name": "temperature", "description": {"zh": "温度参数(0.0-2.0)", "en": "Temperature (0.0-2.0)"}, "type": "number", "required": false, "default": 0.7},
      {"name": "max_tokens", "description": {"zh": "最大生成长度", "en": "Maximum generation length"}, "type": "number", "required": false},
      {"name": "timeout", "description": {"zh": "超时时间（毫秒）", "en": "Timeout (milliseconds)"}, "type": "number", "required": false, "default": 30000}
    ]
  }]
 } */


const aiModelInteraction = (function () {
    type ChatRole = 'system' | 'user' | 'assistant' | 'function';

    type ChatMessage = {
        role: ChatRole;
        content: string;
        name?: string;
        function_call?: {
            name: string;
            arguments?: string;
        };
    };

    type ChatCompletionToolParams = {
        messages: ChatMessage[] | string;
        message?: string;
        system_prompt?: string;
        temperature?: number;
        max_tokens?: number;
        timeout?: number;
        functions?: any[];
    };

    type UniversalHttpResponse = {
        status: number;
        statusText: string;
        headers?: Record<string, string>;
        body: any;
    };

    async function universalHttpRequest(
        url: string,
        method: string = 'POST',
        headers: Record<string, string> = {},
        body: any = null,
        responseType: 'json' | 'text' = 'json',
        timeout: number = 30000
    ): Promise<UniversalHttpResponse> {
        const timeoutPromise: Promise<never> = new Promise((_, reject) =>
            setTimeout(() => reject(new Error(`请求超时：${timeout}ms`)), timeout)
        );

        const requestPromise: Promise<UniversalHttpResponse> = (async () => {
            if (typeof OkHttp === 'undefined') {
                throw new Error('OkHttp 不可用');
            }

            const client = OkHttp.newBuilder()
                .connectTimeout(timeout)
                .readTimeout(timeout)
                .writeTimeout(timeout)
                .build();

            const requestBuilder = client.newRequest()
                .url(url)
                .method(method.toUpperCase());

            if (headers && Object.keys(headers).length > 0) {
                requestBuilder.headers(headers);
            }

            const upperMethod = method.toUpperCase();
            if (body !== null && body !== undefined && upperMethod !== 'GET' && upperMethod !== 'HEAD') {
                if (typeof body === 'string') {
                    requestBuilder.body(body, 'text');
                } else {
                    requestBuilder.body(body, 'json');
                }
            }

            const response: OkHttpResponse = await requestBuilder.build().execute();
            const resultBody = responseType === 'json' ? response.json() : response.content;
            return {
                status: response.statusCode,
                statusText: response.statusMessage || '',
                headers: response.headers,
                body: resultBody,
            };
        })();

        return Promise.race([requestPromise, timeoutPromise]);
    }

    function cleanText(text: unknown): string {
        if (!text || typeof text !== 'string') return String(text ?? '');

        let cleaned: string = text;

        // 处理转义与实体
        cleaned = cleaned.replace(/\|["\\]?n/g, '\n');
        cleaned = cleaned.replace(/\\\\([\\nrt"'&])/g, (m, c) => ({ n: '\n', r: '\r', t: '\t', '"': '"', "'": "'", "&": "&" }[c] || m));
        cleaned = cleaned.replace(/\\u([0-9A-Fa-f]{4})/g, (_, h) => String.fromCharCode(parseInt(h, 16)));

        // 一次性替换HTML实体
        const entities = { quot: '"', amp: '&', lt: '<', gt: '>', nbsp: ' ', '#39': "'", apos: "'" };
        cleaned = cleaned.replace(/&(\w+|#\d+);/g, (m, e) => entities[e] || m);

        // 清理代码块
        cleaned = cleaned.replace(/```[\w-]*\s*\n([\s\S]*?)```/g, '$1').replace(/```/g, '').replace(/`([^`]+)`/g, '$1');

        // 格式化空白与分点
        cleaned = cleaned.replace(/[ \t]{2,}/g, ' ');
        cleaned = cleaned.split('\n').map(l => l.replace(/^[\s\uFEFF\xA0\u3000\u200B-\u200D]+/g, '')).join('\n');
        cleaned = cleaned.replace(/\n{3,}/g, '\n\n').trim();
        cleaned = cleaned.replace(/^\s*[-•●]\s+|^\s*\d+\.\s+|^\s*\d+\)\s+|^\s*\([a-zA-Z]\)\s+/gm, '');

        return cleaned || text;
    }

    function getConfig(varName: string, defaultValue: string | null = null): string {
        try {
            const value = getEnv(varName);
            if (!value || value === `YOUR_${varName}`) {
                if (defaultValue !== null) return defaultValue;
                throw new Error(`${varName} 未配置`);
            }
            return value.trim();
        } catch (e) {
            if (defaultValue !== null) return defaultValue;
            throw new Error(`${varName} 未配置`);
        }
    }

    function getFullConfig() {
        try {
            return {
                apiBaseUrl: getConfig('AI_API_BASE_URL', ''),
                apiKey: getConfig('AI_API_KEY', ''),
                modelName: getConfig('AI_MODEL_NAME', ''),
                timeout: 30000
            };
        } catch {
            return { apiBaseUrl: '', apiKey: '', modelName: '', timeout: 30000 };
        }
    }

    function joinUrl(baseUrl: string, path: string): string {
        const normalizedBase = baseUrl.endsWith('/') ? baseUrl : `${baseUrl}/`;
        const normalizedPath = path.startsWith('/') ? path.slice(1) : path;
        return `${normalizedBase}${normalizedPath}`;
    }

    async function tryEndpoints(baseUrl: string, payload: Record<string, any>, config: { apiKey: string }, timeout: number): Promise<UniversalHttpResponse> {
        if (baseUrl.includes('chat/completions')) {
            return await universalHttpRequest(
                baseUrl,
                'POST',
                {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${config.apiKey}`
                },
                payload,
                'json',
                timeout
            );
        }

        const endpoints = ['v1/chat/completions', 'chat/completions'];
        let lastError;

        for (const endpoint of endpoints) {
            try {
                const url = joinUrl(baseUrl, endpoint);
                const response = await universalHttpRequest(
                    url,
                    'POST',
                    {
                        'Content-Type': 'application/json',
                        'Authorization': `Bearer ${config.apiKey}`
                    },
                    payload,
                    'json',
                    timeout
                );
                if (response.status === 200) return response;
                lastError = new Error(`端点 ${endpoint} 返回 ${response.status}`);
            } catch (error) {
                lastError = error;
            }
        }

        throw lastError || new Error('所有端点尝试失败');
    }

    async function chat_completion_logic(rawInput: ChatCompletionToolParams | string, timeout: number) {
        if (rawInput === null || rawInput === undefined) {
            throw new Error('参数错误: 输入参数不能为空');
        }

        const internalParams: {
            messages: ChatMessage[];
            system_prompt?: string;
            temperature: number;
            max_tokens?: number;
            functions?: any[];
        } = {
            messages: [],
            system_prompt: undefined,
            temperature: 0.7,
            max_tokens: undefined,
            functions: undefined,
        };

        // 处理输入参数
        if (typeof rawInput === 'string') {
            internalParams.messages = [{ role: 'user', content: rawInput }];
        } else if (typeof rawInput === 'object') {
            const hasMessageField = 'message' in rawInput;
            const hasMessagesField = 'messages' in rawInput;

            if (hasMessagesField) {
                const messagesValue = (rawInput as ChatCompletionToolParams).messages;
                if (Array.isArray(messagesValue)) {
                    internalParams.messages = messagesValue as ChatMessage[];
                } else if (typeof messagesValue === 'string') {
                    internalParams.messages = [{ role: 'user', content: messagesValue }];
                } else {
                    throw new Error(`'messages' 必须是数组或字符串`);
                }
            } else if (hasMessageField) {
                const messageValue = (rawInput as ChatCompletionToolParams).message;
                if (typeof messageValue !== 'string') {
                    throw new Error(`'message' 必须是字符串`);
                }
                internalParams.messages = [{ role: 'user', content: messageValue }];
            } else {
                throw new Error(`对象必须包含 'message' 或 'messages' 字段`);
            }

            internalParams.system_prompt = (rawInput as ChatCompletionToolParams).system_prompt ? String((rawInput as ChatCompletionToolParams).system_prompt) : undefined;
            internalParams.temperature = (rawInput as ChatCompletionToolParams).temperature !== undefined ? Number((rawInput as ChatCompletionToolParams).temperature) : 0.7;
            if (isNaN(internalParams.temperature)) internalParams.temperature = 0.7;

            internalParams.max_tokens = (rawInput as ChatCompletionToolParams).max_tokens !== undefined ? Number((rawInput as ChatCompletionToolParams).max_tokens) : undefined;
            if (internalParams.max_tokens !== undefined && isNaN(internalParams.max_tokens)) {
                internalParams.max_tokens = undefined;
            }

            internalParams.functions = Array.isArray((rawInput as ChatCompletionToolParams).functions) ? (rawInput as ChatCompletionToolParams).functions : undefined;
        } else {
            throw new Error(`不支持的参数类型 '${typeof rawInput}'`);
        }

        // 验证messages
        if (!Array.isArray(internalParams.messages) || internalParams.messages.length === 0) {
            throw new Error('messages必须是有效数组且不为空');
        }

        internalParams.messages.forEach((msg, idx) => {
            if (!msg || typeof msg !== 'object' || !msg.role || typeof msg.content !== 'string') {
                throw new Error(`消息 #${idx} 格式无效`);
            }
        });

        // 获取配置
        const config = getFullConfig();
        if (!config.apiBaseUrl || !config.apiKey) {
            throw new Error('AI_API_BASE_URL 和 AI_API_KEY 必须配置');
        }

        // 构建消息数组
        const antiListInstruction = "【重要指令】你必须以连续段落的方式回答，严禁使用任何分点、列表、编号或项目符号格式。";
        const finalMessages = [
            {
                role: 'system',
                content: internalParams.system_prompt
                    ? internalParams.system_prompt + "\n\n" + antiListInstruction
                    : antiListInstruction
            },
            ...internalParams.messages
        ];

        // 构建请求
        const payload: Record<string, any> = {
            model: String(config.modelName || 'gpt-3.5-turbo'),
            messages: finalMessages,
            temperature: Number(internalParams.temperature)
        };

        if (internalParams.max_tokens !== undefined && !isNaN(internalParams.max_tokens)) {
            payload.max_tokens = Number(internalParams.max_tokens);
        }
        if (internalParams.functions) payload.functions = internalParams.functions;

        Object.keys(payload).forEach(key => payload[key] === undefined && delete payload[key]);

        // 发送请求
        let response;
        try {
            response = await tryEndpoints(config.apiBaseUrl, payload, config, timeout);
        } catch {
            response = await universalHttpRequest(
                config.apiBaseUrl,
                'POST',
                {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${config.apiKey}`
                },
                payload,
                'json',
                timeout
            );
        }

        if (response.status !== 200) {
            let errorMsg = `API请求失败: ${response.status}`;
            try {
                const errorData = typeof response.body === 'string' ? JSON.parse(response.body) : response.body;
                errorMsg += ` - ${errorData.error?.message || JSON.stringify(errorData)}`;
            } catch {
                errorMsg += ` - ${response.statusText}`;
            }
            throw new Error(errorMsg);
        }

        const result = typeof response.body === 'string' ? JSON.parse(response.body) : response.body;
        if (!result.choices?.[0]) {
            throw new Error('API返回格式异常');
        }

        const choice = result.choices[0];
        const reply: any = {
            message: choice.message,
            finish_reason: choice.finish_reason,
            usage: result.usage || null,
            model: result.model || config.modelName,
            id: result.id,
            created: result.created
        };

        if (choice.message.function_call) {
            reply.is_function_call = true;
            reply.function_name = choice.message.function_call.name;
            reply.function_arguments = JSON.parse(choice.message.function_call.arguments || '{}');
        }

        return reply;
    }

    async function chat_completion_impl(params: ChatCompletionToolParams | string) {
        const normalizedParams: ChatCompletionToolParams = typeof params === 'object'
            ? (params as ChatCompletionToolParams)
            : { messages: String(params) };

        const timeout = normalizedParams.timeout || 30000;

        const result = await Promise.race([
            chat_completion_logic(normalizedParams, timeout),
            new Promise<never>((_, reject) =>
                setTimeout(() => reject(new Error(`操作超时：${timeout}ms`)), timeout)
            )
        ]);

        const rawReply = result.message?.content;
        const cleanedReply = cleanText(rawReply);

        return {
            success: true,
            message: "AI回复获取成功！",
            data: result,
            reply: cleanedReply,
            raw_reply: rawReply,
            usage: result.usage,
            anti_list_applied: true
        };
    }

    async function wrapToolExecution<T>(func: (params: any) => Promise<T>, params?: any) {
        try {
            const result = await func(params || {});
            complete(result);
        } catch (error: any) {
            console.error(`Tool ${func.name} failed unexpectedly`, error);
            complete({
                success: false,
                message: `AI对话失败: ${String(error && error.message ? error.message : error)}`,
                error_stack: error && error.stack
            });
        }
    }

    async function chat_completion(params?: ChatCompletionToolParams | string) {
        return await wrapToolExecution(chat_completion_impl, params);
    }

    async function main(params?: { message?: string; temperature?: number; timeout?: number }) {
        const config = getFullConfig();
        if (!config.apiBaseUrl || !config.apiKey) {
            return {
                success: false,
                message: 'AI_API_BASE_URL 和 AI_API_KEY 必须配置',
                config
            };
        }

        return await chat_completion_impl({
            messages: params?.message || 'Hello! Please respond in a continuous paragraph without any list or bullet points.',
            temperature: params?.temperature ?? 0.2,
            timeout: params?.timeout ?? 15000,
        });
    }

    return {
        chat_completion,
        single_message: chat_completion,
        test_connection: async (params?: { timeout?: number }) => wrapToolExecution(
            async () => await chat_completion_impl({
                messages: "Hello! Please respond in a continuous paragraph without any list or bullet points.",
                temperature: 0.1,
                timeout: params?.timeout ?? 10000
            }),
            {}
        ),
        main: (params?: { message?: string; temperature?: number; timeout?: number }) => wrapToolExecution(main, params),
        getConfig: getFullConfig,
        _makeHttpRequest: universalHttpRequest,
        _cleanText: cleanText
    };
})();

// 导出工具函数（CommonJS）
exports.chat_completion = aiModelInteraction.chat_completion;
exports.single_message = aiModelInteraction.single_message;
exports.test_connection = aiModelInteraction.test_connection;
exports.getConfig = aiModelInteraction.getConfig;
exports.main = aiModelInteraction.main;
