/* METADATA
{
    "api_version": "2.0.0",
    "name": "google_search",

    "display_name": {
        "zh": "Google 搜索",
        "en": "Google Search"
    },
    "description": {
        "zh": "提供 Google 普通搜索与 Google Scholar 学术搜索能力，支持设置语言与返回条数。",
        "en": "Google web search and Google Scholar search, with configurable language and result count."
    },
    "enabledByDefault": false,
    "category": "Search",
    "tools": [
        {
            "name": "search_web",
            "description": { "zh": "执行 Google 普通搜索，返回网页搜索结果。", "en": "Perform a regular Google search and return web results." },
            "parameters": [
                { "name": "query", "description": { "zh": "搜索关键词", "en": "Search query" }, "type": "string", "required": true },
                { "name": "max_results", "description": { "zh": "返回结果数量，默认 10，最大 20", "en": "Number of results to return (default: 10, max: 20)" }, "type": "number", "required": false },
                { "name": "language", "description": { "zh": "界面语言参数，默认 en", "en": "Interface language (default: en)" }, "type": "string", "required": false },
                { "name": "region", "description": { "zh": "地区参数，例如 us、cn。默认 us", "en": "Region parameter, e.g. us, cn (default: us)" }, "type": "string", "required": false },
                { "name": "includeLinks", "description": { "zh": "是否在结果中包含可点击的链接列表，默认为false。", "en": "Whether to include a clickable link list in results (default: false)" }, "type": "boolean", "required": false }
            ]
        },
        {
            "name": "search_scholar",
            "description": { "zh": "执行 Google Scholar 学术搜索，返回学术文献结果。", "en": "Perform a Google Scholar search and return academic results." },
            "parameters": [
                { "name": "query", "description": { "zh": "搜索关键词", "en": "Search query" }, "type": "string", "required": true },
                { "name": "max_results", "description": { "zh": "返回结果数量，默认 10，最大 20", "en": "Number of results to return (default: 10, max: 20)" }, "type": "number", "required": false },
                { "name": "language", "description": { "zh": "界面语言参数，默认 en", "en": "Interface language (default: en)" }, "type": "string", "required": false },
                { "name": "includeLinks", "description": { "zh": "是否在结果中包含可点击的链接列表，默认为false。", "en": "Whether to include a clickable link list in results (default: false)" }, "type": "boolean", "required": false }
            ]
        },
        {
            "name": "search_scholar_mirror",
            "description": { "zh": "通过镜像站执行 Google Scholar 学术搜索，以绕过人机验证。", "en": "Use a mirror site to perform Google Scholar searches to bypass CAPTCHA checks." },
            "parameters": [
                { "name": "query", "description": { "zh": "搜索关键词", "en": "Search query" }, "type": "string", "required": true },
                { "name": "max_results", "description": { "zh": "返回结果数量，默认 10，最大 20", "en": "Number of results to return (default: 10, max: 20)" }, "type": "number", "required": false },
                { "name": "language", "description": { "zh": "界面语言参数，默认 en", "en": "Interface language (default: en)" }, "type": "string", "required": false },
                { "name": "includeLinks", "description": { "zh": "是否在结果中包含可点击的链接列表，默认为false。", "en": "Whether to include a clickable link list in results (default: false)" }, "type": "boolean", "required": false }
            ]
        }
    ]
}*/

const googleSearch = (function () {
    type SearchParams = {
        query: string;
        max_results?: number;
        language?: string;
        region?: string;
        includeLinks?: boolean;
    };

    type ScholarSearchParams = {
        query: string;
        max_results?: number;
        language?: string;
        includeLinks?: boolean;
    };

    type GoogleSearchResult = {
        title: string;
        url: string;
        snippet: string;
        position: number;
    };

    const GOOGLE_SEARCH_URL = "https://www.google.com/search";
    const GOOGLE_SCHOLAR_URL = "https://scholar.google.com/scholar";
    const GOOGLE_SCHOLAR_MIRRORS = [
        "https://xs.cntpj.com/scholar",
    ];
    const MAX_RESULTS = 20;

    function buildUrl(base: string, params: Record<string, string>): string {
        const queryString = Object.entries(params)
            .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
            .join("&");
        return `${base}?${queryString}`;
    }

    function getHostname(rawUrl: string): string {
        const match = rawUrl.trim().match(/^[a-zA-Z][a-zA-Z\d+\-.]*:\/\/([^/?#]+)/);
        if (!match || !match[1]) {
            return rawUrl;
        }
        const authority = match[1];
        const host = authority.includes("@") ? authority.split("@").pop() || authority : authority;
        return host.replace(/:\d+$/, "");
    }

    async function fetchHtmlViaWebVisit(url: string): Promise<VisitWebResultData> {
        const result = await Tools.Net.visit(url);
        // The result can be a string if the underlying tool returns a simple string.
        // We'll normalize it to a VisitWebResultData object.
        if (typeof result === "string") {
            return {
                url: url,
                title: "",
                content: result,
                metadata: {},
                links: [],
                imageLinks: [],
                contentTruncated: false,
                toString: () => result,
            };
        }
        return result;
    }

    // 解析相关逻辑已移除，直接返回 visit 的纯文本结果

    async function performSearch(url: string, includeLinks: boolean = false, sourceName: string) {
        try {
            const result = await fetchHtmlViaWebVisit(url);
            const content = result.content || '';

            let parts: string[] = [];
            if (result.visitKey) {
                parts.push(`visit_key: ${result.visitKey}`);
            }
            if (includeLinks && result.links && result.links.length > 0) {
                const linksLines = result.links.map((link, index) => `[${index + 1}] ${link.text}`);
                parts.push(linksLines.join('\n'));
            }
            parts.push(content);

            return {
                success: true,
                message: `${sourceName} 搜索成功`,
                data: parts.join('\n\n')
            };
        } catch (error: any) {
            return {
                success: false,
                message: `${sourceName} 搜索失败: ${error.message}`
            };
        }
    }

    async function searchWeb(params: SearchParams) {
        if (!params.query || params.query.trim() === "") {
            throw new Error("请提供有效的 query 参数。");
        }
        const maxResults = Math.min(Math.max(params.max_results || 10, 1), MAX_RESULTS);
        const language = params.language || "en";
        const region = params.region || "us";
        const url = buildUrl(GOOGLE_SEARCH_URL, {
            q: params.query,
            hl: language,
            gl: region,
            num: String(maxResults),
            pws: "0",
        });

        return performSearch(url, params.includeLinks, "Google");
    }

    async function searchScholar(params: ScholarSearchParams) {
        if (!params.query || params.query.trim() === "") {
            throw new Error("请提供有效的 query 参数。");
        }
        const maxResults = Math.min(Math.max(params.max_results || 10, 1), MAX_RESULTS);
        const language = params.language || "en";
        const url = buildUrl(GOOGLE_SCHOLAR_URL, {
            q: params.query,
            hl: language,
            as_sdt: "0,5",
            num: String(maxResults)
        });

        return performSearch(url, params.includeLinks, "Google Scholar");
    }

    async function searchScholarMirror(params: ScholarSearchParams) {
        if (!params.query || params.query.trim() === "") {
            throw new Error("请提供有效的 query 参数。");
        }
        const maxResults = Math.min(Math.max(params.max_results || 10, 1), MAX_RESULTS);
        const language = params.language || "en";

        const mirrorUrls = GOOGLE_SCHOLAR_MIRRORS.map(mirror => buildUrl(mirror, {
            q: params.query,
            hl: language,
            as_sdt: "0,5",
            num: String(maxResults)
        }));

        if (mirrorUrls.length === 0) {
            return {
                success: false,
                message: "没有可用的 Google Scholar 镜像地址。"
            };
        }

        let lastError: any = null;

        for (const currentUrl of mirrorUrls) {
            try {
                const searchResult = await performSearch(currentUrl, params.includeLinks, `Google Scholar 镜像 (${getHostname(currentUrl)})`);
                if (searchResult.success && searchResult.data) {
                    // Check for CAPTCHA in the content
                    if (searchResult.data.includes("recaptcha") || searchResult.data.includes("人机身份验证")) {
                        throw new Error("CAPTCHA required");
                    }
                    return searchResult;
                }
                // If performSearch itself fails, it will throw and be caught below.
            } catch (error: any) {
                lastError = error;
                console.log(`Attempt with ${currentUrl} failed: ${error.message}`);
            }
        }

        return {
            success: false,
            message: `Google Scholar 镜像搜索在尝试所有镜像后失败: ${lastError?.message || 'Unknown error'}`
        };
    }

    async function main() {
        console.log("--- Testing Web Search ---");
        const webResult = await searchWeb({ query: "TypeScript" });
        console.log(JSON.stringify(webResult, null, 2));

        console.log("\n--- Testing Scholar Search ---");
        const scholarResult = await searchScholar({ query: "Large Language Models" });
        console.log(JSON.stringify(scholarResult, null, 2));

        console.log("\n--- Testing Scholar Mirror Search ---");
        const scholarMirrorResult = await searchScholarMirror({ query: "Quantum Computing" });
        console.log(JSON.stringify(scholarMirrorResult, null, 2));
    }

    function wrap(coreFunction: (params: any) => Promise<any>) {
        return async (params: any) => {
            // The core function expects the params object directly.
            return coreFunction(params);
        };
    }

    return {
        search_web: searchWeb,
        search_scholar: searchScholar,
        search_scholar_mirror: searchScholarMirror,
        main,
        wrap,
    };
})();

exports.search_web = googleSearch.wrap(googleSearch.search_web);
exports.search_scholar = googleSearch.wrap(googleSearch.search_scholar);
exports.search_scholar_mirror = googleSearch.wrap(googleSearch.search_scholar_mirror);
exports.main = googleSearch.main;

