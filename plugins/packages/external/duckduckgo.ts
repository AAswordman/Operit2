/*
METADATA
{
    "api_version": "2.0.0",
    "name": "duckduckgo",

    "display_name": {
        "zh": "DuckDuckGo 搜索",
        "en": "DuckDuckGo Search"
    },
    "description": { "zh": "使用DuckDuckGo进行网络搜索和内容抓取。", "en": "Use DuckDuckGo for web search and content extraction." },
    "category": "Search",
    "tools": [
        {
            "name": "search",
            "description": { "zh": "执行DuckDuckGo搜索并返回格式化的结果。", "en": "Run a DuckDuckGo search and return formatted results." },
            "parameters": [
                {
                    "name": "query",
                    "description": { "zh": "搜索查询字符串", "en": "Search query string." },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "max_results",
                    "description": { "zh": "返回的最大结果数 (默认: 10)", "en": "Maximum number of results to return (default: 10)." },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "fetch_content",
            "description": { "zh": "从网页URL抓取和解析内容。", "en": "Fetch and parse content from a webpage URL." },
            "parameters": [
                {
                    "name": "url",
                    "description": { "zh": "要抓取内容的网页URL", "en": "Webpage URL to fetch and parse." },
                    "type": "string",
                    "required": true
                }
            ]
        }
    ]
}*/

const duckduckgo = (function () {
    const client = OkHttp.newClient();
    const BASE_URL = "https://html.duckduckgo.com/html/";

    /**
     * A simple rate limiter to avoid sending too many requests.
     */
    class RateLimiter {
        private requests: number[] = [];
        constructor(private requestsPerMinute: number = 30) { }

        async acquire(): Promise<void> {
            const now = Date.now();
            // Remove requests older than 1 minute (60000 ms)
            this.requests = this.requests.filter(reqTime => now - reqTime < 60000);

            if (this.requests.length >= this.requestsPerMinute) {
                const timeSinceFirstRequest = now - this.requests[0];
                const waitTime = 60000 - timeSinceFirstRequest;
                if (waitTime > 0) {
                    await new Promise(resolve => setTimeout(resolve, waitTime));
                }
            }

            this.requests.push(Date.now());
        }
    }

    const searchRateLimiter = new RateLimiter(30);
    const fetchRateLimiter = new RateLimiter(20);

    /**
     * Decodes basic HTML entities.
     * @param text The text to decode.
     * @returns Decoded text.
     */
    function decodeHtmlEntities(text: string): string {
        return text.replace(/&amp;/g, '&')
            .replace(/&lt;/g, '<')
            .replace(/&gt;/g, '>')
            .replace(/&quot;/g, '"')
            .replace(/&#39;/g, "'");
    }

    /**
     * Search DuckDuckGo and return formatted results.
     * @param params Search parameters including query and max_results.
     * @returns A formatted string of search results.
     */
    async function search(params: { query: string; max_results?: string }): Promise<string> {
        const { query } = params;
        let max_results = 10;
        if (params.max_results) {
            const parsedMaxResults = parseInt(params.max_results, 10);
            if (!isNaN(parsedMaxResults)) {
                max_results = parsedMaxResults;
            }
        }

        if (!query) {
            throw new Error("查询不能为空");
        }

        await searchRateLimiter.acquire();
        console.log(`正在从DuckDuckGo搜索: ${query}`);

        const request = client.newRequest()
            .url(BASE_URL)
            .method('POST')
            .body({ q: query, b: '', kl: '' }, 'form')
            .headers({
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
            });

        const response = await request.build().execute();
        if (!response.isSuccessful()) {
            throw new Error(`HTTP 错误! 状态码: ${response.statusCode}`);
        }

        const html = response.content;
        const results: { title: string; link: string; snippet: string; position: number }[] = [];
        const resultRegex = /<h2 class="result__title">[\s\S]*?<a.*?href="([^"]+)".*?>([\s\S]+?)<\/a>[\s\S]*?<\/h2>[\s\S]*?<a class="result__snippet".*?>([\s\S]+?)<\/a>/g;

        let match;
        while ((match = resultRegex.exec(html)) !== undefined) {
            if (results.length >= max_results) {
                break;
            }

            let link = match[1];
            if (link.includes('y.js')) {
                continue; // Skip ads
            }

            if (link.startsWith('//duckduckgo.com/l/?uddg=')) {
                try {
                    link = decodeURIComponent(link.split('uddg=')[1].split('&')[0]);
                } catch (e) {
                    console.error(`URL 解码失败: ${link}`);
                }
            }

            const title = decodeHtmlEntities(match[2].replace(/<[^>]*>/g, '').trim());
            const snippet = decodeHtmlEntities(match[3].replace(/<[^>]*>/g, '').trim());

            results.push({ title, link, snippet, position: results.length + 1 });
        }

        console.log(`成功找到 ${results.length} 个结果`);
        return format_results_for_llm(results);
    }

    /**
     * Fetch and parse content from a webpage URL.
     * @param params Parameters including the URL to fetch.
     * @returns The cleaned text content of the webpage.
     */
    async function fetch_content(params: { url: string }): Promise<string> {
        const { url } = params;
        if (!url) {
            throw new Error("URL不能为空");
        }

        await fetchRateLimiter.acquire();
        console.log(`正在抓取内容: ${url}`);

        try {
            const request = client.newRequest()
                .url(url)
                .method('GET')
                .headers({ "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36" });

            const response = await request.build().execute();
            if (!response.isSuccessful()) {
                throw new Error(`无法访问网页 (${response.statusCode})`);
            }

            let text = response.content;
            text = text.replace(/<style[^>]*>[\s\S]*?<\/style>/gi, '');
            text = text.replace(/<script[^>]*>[\s\S]*?<\/script>/gi, '');
            text = text.replace(/<nav[^>]*>[\s\S]*?<\/nav>/gi, '');
            text = text.replace(/<header[^>]*>[\s\S]*?<\/header>/gi, '');
            text = text.replace(/<footer[^>]*>[\s\S]*?<\/footer>/gi, '');
            text = text.replace(/<[^>]+>/g, ' ');
            text = text.replace(/\s+/g, ' ').trim();

            if (text.length > 8000) {
                text = text.substring(0, 8000) + "... [内容被截断]";
            }

            console.log(`成功抓取并解析内容 (${text.length} 字符)`);
            return text;
        } catch (error) {
            console.error(`从 ${url} 抓取内容时出错: ${error.message}`);
            return `错误: 从网页抓取内容时发生意外错误 (${error.message})`;
        }
    }

    /**
     * Formats search results into a readable string for LLMs.
     * @param results The list of search results.
     * @returns A formatted string.
     */
    function format_results_for_llm(results: { title: string; link: string; snippet: string; position: number }[]): string {
        if (!results || results.length === 0) {
            return "没有为您的搜索查询找到结果。这可能是由于DuckDuckGo的机器人检测或查询没有匹配项。请尝试重新措辞您的搜索或在几分钟后重试。";
        }

        const output = results.map(r =>
            `${r.position}. ${r.title}\n   URL: ${r.link}\n   摘要: ${r.snippet}`
        );

        return `找到 ${results.length} 个搜索结果:\n\n${output.join('\n\n')}`;
    }

    /**
     * Wraps function calls for standardized success/error handling.
     */
    async function duckduckgo_wrap<T>(
        func: (params: any) => Promise<any>,
        params: any,
        successMessage: string,
        failMessage: string
    ): Promise<void> {
        try {
            console.log(`开始执行函数: ${func.name || '匿名函数'}`);
            const result = await func(params);
            complete({ success: true, message: successMessage, data: result });
        } catch (error) {
            console.error(`函数 ${func.name || '匿名函数'} 执行失败! 错误: ${error.message}`);
            complete({ success: false, message: `${failMessage}: ${error.message}`, error_stack: error.stack });
        }
    }

    return {
        search: (params: any) => duckduckgo_wrap(search, params, '搜索完成', '搜索失败'),
        fetch_content: (params: any) => duckduckgo_wrap(fetch_content, params, '内容抓取完成', '内容抓取失败'),
    };
})();

exports.search = duckduckgo.search;
exports.fetch_content = duckduckgo.fetch_content; 
