/* METADATA
{
    "api_version": "2.0.0",
  "name": "zhipu_search",
  "display_name": {
    "zh": "智谱搜索",
    "en": "Zhipu Search"
  },
  "description": {
    "zh": "智谱 AI 独立网络搜索 API，返回结构化搜索结果。",
    "en": "Zhipu AI standalone web search API with structured results."
  },
  "author": "浮生一梦",
  "env": [
    {
      "name": "ZHIPU_SEARCH_API_KEY",
      "description": {
        "zh": "智谱搜索专用 API Key（与生图 Key 独立）",
        "en": "Zhipu Search API Key (independent from draw Key)"
      },
      "required": false
    }
  ],
  "category": "Search",
  "tools": [
    {
      "name": "search",
      "description": {
        "zh": "使用智谱 Web Search API 进行搜索",
        "en": "Search using Zhipu Web Search API"
      },
      "parameters": [
        { "name": "query", "description": { "zh": "搜索关键词", "en": "Search query" }, "type": "string", "required": true },
        { "name": "api_key", "description": { "zh": "智谱 API Key（可选，不传则读取环境变量）", "en": "Zhipu API Key" }, "type": "string", "required": false },
        { "name": "engine", "description": { "zh": "搜索引擎：search_std/search_pro/search_pro_sogou/search_pro_quark", "en": "Search engine" }, "type": "string", "required": false },
        { "name": "count", "description": { "zh": "返回结果数 (1-50)，默认 10", "en": "Result count (1-50)" }, "type": "number", "required": false },
        { "name": "recency", "description": { "zh": "时间范围：oneDay/oneWeek/oneMonth/oneYear/noLimit", "en": "Time range" }, "type": "string", "required": false },
        { "name": "content_size", "description": { "zh": "内容长度：medium/high", "en": "Content size" }, "type": "string", "required": false }
      ]
    },
    {
      "name": "test",
      "description": {
        "zh": "测试 API 连接",
        "en": "Test API connection"
      },
      "parameters": []
    }
  ]
}*/

const zhipuSearch = (function () {
    const API_URL = "https://open.bigmodel.cn/api/paas/v4/web_search";
    const TIMEOUT = 60000;
    const client = OkHttp.newBuilder()
        .connectTimeout(TIMEOUT)
        .readTimeout(TIMEOUT)
        .writeTimeout(TIMEOUT)
        .build();

    type SearchParams = {
        query: string;
        api_key?: string;
        engine?: string;
        count?: number;
        recency?: string;
        content_size?: string;
    };

    type ZhipuSearchItem = {
        title?: string;
        content?: string;
        link?: string;
        media?: string;
        icon?: string;
        publish_date?: string;
        refer?: string;
    };

    type ZhipuSearchResponse = {
        id?: string;
        search_result?: ZhipuSearchItem[];
        search_intent?: string[];
    };

    type SearchResult = {
        success: true;
        query: string;
        id?: string;
        intent: string | null;
        results: ZhipuSearchItem[];
        count: number;
    };

    type TestResult = {
        success: true;
        latency: number;
        id?: string;
    };

    function getApiKey(providedKey?: string): string {
        if (providedKey) return providedKey;
        let key = getEnv("ZHIPU_SEARCH_API_KEY");
        if (key) return key;
        key = getEnv("DEFAULT_API_KEY");
        return key || "";
    }

    async function httpPost(body: Record<string, any>, apiKey?: string | null): Promise<ZhipuSearchResponse> {
        const key = getApiKey(apiKey || undefined);
        if (!key) {
            throw new Error("未设置 API Key，请配置 ZHIPU_SEARCH_API_KEY 环境变量或在调用时传入 api_key 参数");
        }

        const request = client
            .newRequest()
            .url(API_URL)
            .method("POST")
            .header("Authorization", "Bearer " + key)
            .header("Content-Type", "application/json")
            .body(JSON.stringify(body), "json");

        const response = await request.build().execute();
        const content = response.content;

        if (!response.isSuccessful()) {
            throw new Error("HTTP " + response.statusCode + ": " + content);
        }

        return JSON.parse(content) as ZhipuSearchResponse;
    }

    async function search(params: SearchParams): Promise<SearchResult> {
        const body: Record<string, any> = {
            search_query: params.query,
            search_engine: params.engine || "search_std",
            search_intent: true,
            count: params.count || 10,
            content_size: params.content_size || "medium"
        };

        if (params.recency) {
            body.search_recency_filter = params.recency;
        }

        const result = await httpPost(body, params.api_key);

        let results: ZhipuSearchItem[] = [];
        if (result.search_result && Array.isArray(result.search_result)) {
            results = result.search_result.map((item) => ({
                title: item.title,
                content: item.content,
                link: item.link,
                media: item.media,
                icon: item.icon,
                publish_date: item.publish_date,
                refer: item.refer
            }));
        }

        let intent: string | null = null;
        if (result.search_intent && result.search_intent.length > 0) {
            intent = result.search_intent[0] || null;
        }

        return {
            success: true,
            query: params.query,
            id: result.id,
            intent,
            results,
            count: results.length
        };
    }

    async function test(): Promise<TestResult> {
        const body = {
            search_query: "hi",
            search_engine: "search_std",
            search_intent: false,
            count: 1
        };

        const start = Date.now();
        const result = await httpPost(body, null);
        const latency = Date.now() - start;

        return {
            success: true,
            latency,
            id: result.id
        };
    }

    async function searchWrapper(params: SearchParams) {
        try {
            const result = await search(params);
            complete({
                success: true,
                message: "搜索完成，找到 " + result.count + " 条结果",
                data: result
            });
        } catch (error: any) {
            complete({
                success: false,
                message: "搜索失败：" + error.message,
                error_stack: error.stack
            });
        }
    }

    async function testWrapper() {
        try {
            const result = await test();
            complete({
                success: true,
                message: "连接成功，延迟 " + result.latency + "ms",
                data: result
            });
        } catch (error: any) {
            complete({
                success: false,
                message: "测试失败：" + error.message,
                error_stack: error.stack
            });
        }
    }

    return {
        search: searchWrapper,
        test: testWrapper
    };
})();

exports.search = zhipuSearch.search;
exports.test = zhipuSearch.test;
