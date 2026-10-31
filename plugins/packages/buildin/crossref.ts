/* METADATA
{
    "name": "crossref",
    "display_name": {
        "zh": "Crossref 学术文献查询",
        "en": "Crossref Academic Literature Search"
    },
    "description": {
        "zh": "Crossref 学术文献查询工具，提供 DOI 查询、关键词搜索、作者搜索等功能，帮助用户查找和获取学术文章元数据。",
        "en": "Crossref scholarly literature search tools: query by DOI, keyword, author, title, ISSN, and retrieve publication metadata."
    },
    "category": "Search",
    "enabledByDefault": true,
    "tools": [
        {
            "name": "search_by_doi",
            "description": { "zh": "通过 DOI (Digital Object Identifier) 查询文章的详细信息", "en": "Query article details by DOI (Digital Object Identifier)." },
            "parameters": [
                {
                    "name": "doi",
                    "description": { "zh": "文章的 DOI 标识符，例如 '10.1038/nature12373'", "en": "DOI identifier, e.g. '10.1038/nature12373'" },
                    "type": "string",
                    "required": true
                }
            ]
        },
        {
            "name": "search_by_keyword",
            "description": { "zh": "通过关键词搜索学术文章", "en": "Search scholarly articles by keyword." },
            "parameters": [
                {
                    "name": "query",
                    "description": { "zh": "搜索关键词", "en": "Search query keyword(s)" },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "rows",
                    "description": { "zh": "返回结果数量，默认 10，最大 100", "en": "Number of results to return (default: 10, max: 100)" },
                    "type": "number",
                    "required": false
                },
                {
                    "name": "sort",
                    "description": { "zh": "排序方式，可选值：'relevance'(相关性), 'score'(评分), 'updated'(更新时间), 'deposited'(提交时间), 'indexed'(索引时间), 'published'(发布时间)", "en": "Sort mode. Options: 'relevance', 'score', 'updated', 'deposited', 'indexed', 'published'." },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "order",
                    "description": { "zh": "排序顺序，可选值：'asc'(升序), 'desc'(降序)，默认 'desc'", "en": "Sort order: 'asc' or 'desc' (default: 'desc')." },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "search_by_author",
            "description": { "zh": "通过作者名字搜索文章", "en": "Search articles by author name." },
            "parameters": [
                {
                    "name": "author",
                    "description": { "zh": "作者名字", "en": "Author name" },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "rows",
                    "description": { "zh": "返回结果数量，默认 10，最大 100", "en": "Number of results to return (default: 10, max: 100)" },
                    "type": "number",
                    "required": false
                }
            ]
        },
        {
            "name": "search_by_title",
            "description": { "zh": "通过文章标题搜索", "en": "Search articles by title." },
            "parameters": [
                {
                    "name": "title",
                    "description": { "zh": "文章标题或标题关键词", "en": "Article title or title keyword(s)" },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "rows",
                    "description": { "zh": "返回结果数量，默认 10，最大 100", "en": "Number of results to return (default: 10, max: 100)" },
                    "type": "number",
                    "required": false
                }
            ]
        },
        {
            "name": "search_by_issn",
            "description": { "zh": "通过期刊 ISSN 查询该期刊发表的文章", "en": "Search articles published in a journal by ISSN." },
            "parameters": [
                {
                    "name": "issn",
                    "description": { "zh": "期刊的 ISSN 标识符，例如 '1476-4687'", "en": "Journal ISSN identifier, e.g. '1476-4687'" },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "rows",
                    "description": { "zh": "返回结果数量，默认 10，最大 100", "en": "Number of results to return (default: 10, max: 100)" },
                    "type": "number",
                    "required": false
                }
            ]
        }
    ]
}*/
const CrossrefSearch = (function () {
    const BASE_URL = "https://api.crossref.org";
    const DEFAULT_ROWS = 10;
    const MAX_ROWS = 100;

    function buildQueryString(params: Record<string, string>): string {
        return Object.entries(params)
            .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
            .join("&");
    }

    type SearchByDoiParams = {
        doi: string;
    };

    type SearchByKeywordParams = {
        query: string;
        rows?: number;
        sort?: string;
        order?: string;
    };

    type SearchByAuthorParams = {
        author: string;
        rows?: number;
    };

    type SearchByTitleParams = {
        title: string;
        rows?: number;
    };

    type SearchByISSNParams = {
        issn: string;
        rows?: number;
    };

    /**
     * 格式化作者信息
     */
    function formatAuthors(authors: any[]): string {
        if (!authors || authors.length === 0) return "N/A";
        return authors
            .slice(0, 5) // 只显示前5个作者
            .map((author: any) => {
                const given = author.given || "";
                const family = author.family || "";
                return `${given} ${family}`.trim();
            })
            .filter(name => name.length > 0)
            .join(", ");
    }

    /**
     * 格式化日期
     */
    function formatDate(dateParts: number[][]): string {
        if (!dateParts || dateParts.length === 0 || !dateParts[0]) return "N/A";
        const parts = dateParts[0];
        if (parts.length === 1) return `${parts[0]}`;
        if (parts.length === 2) return `${parts[0]}-${String(parts[1]).padStart(2, '0')}`;
        if (parts.length === 3) return `${parts[0]}-${String(parts[1]).padStart(2, '0')}-${String(parts[2]).padStart(2, '0')}`;
        return "N/A";
    }

    /**
     * 格式化单篇文章信息
     */
    function formatArticle(item: any, index?: number): string {
        const lines: string[] = [];

        if (index !== undefined) {
            lines.push(`\n=== Article ${index + 1} ===`);
        }

        // 标题
        const title = item.title && item.title.length > 0 ? item.title[0] : "No Title";
        lines.push(`Title: ${title}`);

        // DOI
        if (item.DOI) {
            lines.push(`DOI: ${item.DOI}`);
            lines.push(`URL: https://doi.org/${item.DOI}`);
        }

        // 作者
        const authors = formatAuthors(item.author);
        lines.push(`Authors: ${authors}`);

        // 发表日期
        const publishedDate = formatDate(item.published?.['date-parts'] || item['published-print']?.['date-parts'] || item['published-online']?.['date-parts']);
        lines.push(`Published: ${publishedDate}`);

        // 期刊/会议
        if (item['container-title'] && item['container-title'].length > 0) {
            lines.push(`Journal/Conference: ${item['container-title'][0]}`);
        }

        // ISSN
        if (item.ISSN && item.ISSN.length > 0) {
            lines.push(`ISSN: ${item.ISSN.join(', ')}`);
        }

        // 出版商
        if (item.publisher) {
            lines.push(`Publisher: ${item.publisher}`);
        }

        // 类型
        if (item.type) {
            lines.push(`Type: ${item.type}`);
        }

        // 引用次数
        if (item['is-referenced-by-count'] !== undefined) {
            lines.push(`Citations: ${item['is-referenced-by-count']}`);
        }

        // 摘要（如果有）
        if (item.abstract) {
            // 移除 HTML 标签
            const abstractText = item.abstract.replace(/<[^>]*>/g, '');
            lines.push(`Abstract: ${abstractText.substring(0, 500)}${abstractText.length > 500 ? '...' : ''}`);
        }

        return lines.join('\n');
    }

    /**
     * 通过 DOI 查询文章
     */
    async function searchByDoi(params: SearchByDoiParams): Promise<any> {
        const { doi } = params;

        if (!doi || doi.trim() === "") {
            return {
                success: false,
                message: "请提供有效的 DOI"
            };
        }

        try {
            const url = `${BASE_URL}/works/${encodeURIComponent(doi)}`;
            const client = OkHttp.newClient();
            const response = await client.get(url, {
                'User-Agent': 'Operit/1.0 (mailto:support@example.com)'
            });

            if (!response.isSuccessful()) {
                return {
                    success: false,
                    message: `查询失败: HTTP ${response.statusCode} - ${response.statusMessage}`
                };
            }

            const data = response.json();

            if (data.status === "ok" && data.message) {
                const article = formatArticle(data.message);
                return {
                    success: true,
                    message: "查询成功",
                    data: article
                };
            } else {
                return {
                    success: false,
                    message: "未找到该 DOI 对应的文章"
                };
            }
        } catch (error: any) {
            return {
                success: false,
                message: `查询失败: ${error.message}`
            };
        }
    }

    /**
     * 通过关键词搜索文章
     */
    async function searchByKeyword(params: SearchByKeywordParams): Promise<any> {
        const { query, rows = DEFAULT_ROWS, sort = "relevance", order = "desc" } = params;

        if (!query || query.trim() === "") {
            return {
                success: false,
                message: "请提供有效的搜索关键词"
            };
        }

        const actualRows = Math.min(Math.max(rows, 1), MAX_ROWS);

        try {
            const queryString = buildQueryString({
                query: query,
                rows: String(actualRows),
                sort: sort,
                order: order
            });

            const url = `${BASE_URL}/works?${queryString}`;
            const client = OkHttp.newClient();
            const response = await client.get(url, {
                'User-Agent': 'Operit/1.0 (mailto:support@example.com)'
            });

            if (!response.isSuccessful()) {
                return {
                    success: false,
                    message: `搜索失败: HTTP ${response.statusCode} - ${response.statusMessage}`
                };
            }

            const data = response.json();

            if (data.status === "ok" && data.message && data.message.items) {
                const items = data.message.items;
                const totalResults = data.message['total-results'];

                const results = items.map((item: any, index: number) => formatArticle(item, index));

                const summary = `Found ${totalResults} results (showing ${items.length}):\n${results.join('\n\n')}`;

                return {
                    success: true,
                    message: "搜索成功",
                    data: summary,
                    total: totalResults,
                    count: items.length
                };
            } else {
                return {
                    success: false,
                    message: "未找到相关文章"
                };
            }
        } catch (error: any) {
            return {
                success: false,
                message: `搜索失败: ${error.message}`
            };
        }
    }

    /**
     * 通过作者搜索文章
     */
    async function searchByAuthor(params: SearchByAuthorParams): Promise<any> {
        const { author, rows = DEFAULT_ROWS } = params;

        if (!author || author.trim() === "") {
            return {
                success: false,
                message: "请提供有效的作者名字"
            };
        }

        const actualRows = Math.min(Math.max(rows, 1), MAX_ROWS);

        try {
            const queryString = buildQueryString({
                'query.author': author,
                rows: String(actualRows)
            });

            const url = `${BASE_URL}/works?${queryString}`;
            const client = OkHttp.newClient();
            const response = await client.get(url, {
                'User-Agent': 'Operit/1.0 (mailto:support@example.com)'
            });

            if (!response.isSuccessful()) {
                return {
                    success: false,
                    message: `搜索失败: HTTP ${response.statusCode} - ${response.statusMessage}`
                };
            }

            const data = response.json();

            if (data.status === "ok" && data.message && data.message.items) {
                const items = data.message.items;
                const totalResults = data.message['total-results'];

                const results = items.map((item: any, index: number) => formatArticle(item, index));

                const summary = `Found ${totalResults} results for author "${author}" (showing ${items.length}):\n${results.join('\n\n')}`;

                return {
                    success: true,
                    message: "搜索成功",
                    data: summary,
                    total: totalResults,
                    count: items.length
                };
            } else {
                return {
                    success: false,
                    message: `未找到该作者 "${author}" 的文章`
                };
            }
        } catch (error: any) {
            return {
                success: false,
                message: `搜索失败: ${error.message}`
            };
        }
    }

    /**
     * 通过标题搜索文章
     */
    async function searchByTitle(params: SearchByTitleParams): Promise<any> {
        const { title, rows = DEFAULT_ROWS } = params;

        if (!title || title.trim() === "") {
            return {
                success: false,
                message: "请提供有效的文章标题"
            };
        }

        const actualRows = Math.min(Math.max(rows, 1), MAX_ROWS);

        try {
            const queryString = buildQueryString({
                'query.title': title,
                rows: String(actualRows)
            });

            const url = `${BASE_URL}/works?${queryString}`;
            const client = OkHttp.newClient();
            const response = await client.get(url, {
                'User-Agent': 'Operit/1.0 (mailto:support@example.com)'
            });

            if (!response.isSuccessful()) {
                return {
                    success: false,
                    message: `搜索失败: HTTP ${response.statusCode} - ${response.statusMessage}`
                };
            }

            const data = response.json();

            if (data.status === "ok" && data.message && data.message.items) {
                const items = data.message.items;
                const totalResults = data.message['total-results'];

                const results = items.map((item: any, index: number) => formatArticle(item, index));

                const summary = `Found ${totalResults} results matching title "${title}" (showing ${items.length}):\n${results.join('\n\n')}`;

                return {
                    success: true,
                    message: "搜索成功",
                    data: summary,
                    total: totalResults,
                    count: items.length
                };
            } else {
                return {
                    success: false,
                    message: `未找到标题包含 "${title}" 的文章`
                };
            }
        } catch (error: any) {
            return {
                success: false,
                message: `搜索失败: ${error.message}`
            };
        }
    }

    /**
     * 通过 ISSN 查询期刊文章
     */
    async function searchByISSN(params: SearchByISSNParams): Promise<any> {
        const { issn, rows = DEFAULT_ROWS } = params;

        if (!issn || issn.trim() === "") {
            return {
                success: false,
                message: "请提供有效的 ISSN"
            };
        }

        const actualRows = Math.min(Math.max(rows, 1), MAX_ROWS);

        try {
            const url = `${BASE_URL}/journals/${encodeURIComponent(issn)}/works?rows=${actualRows}`;
            const client = OkHttp.newClient();
            const response = await client.get(url, {
                'User-Agent': 'Operit/1.0 (mailto:support@example.com)'
            });

            if (!response.isSuccessful()) {
                return {
                    success: false,
                    message: `查询失败: HTTP ${response.statusCode} - ${response.statusMessage}`
                };
            }

            const data = response.json();

            if (data.status === "ok" && data.message && data.message.items) {
                const items = data.message.items;
                const totalResults = data.message['total-results'];

                const results = items.map((item: any, index: number) => formatArticle(item, index));

                const summary = `Found ${totalResults} articles from journal ISSN ${issn} (showing ${items.length}):\n${results.join('\n\n')}`;

                return {
                    success: true,
                    message: "查询成功",
                    data: summary,
                    total: totalResults,
                    count: items.length
                };
            } else {
                return {
                    success: false,
                    message: `未找到 ISSN "${issn}" 对应期刊的文章`
                };
            }
        } catch (error: any) {
            return {
                success: false,
                message: `查询失败: ${error.message}`
            };
        }
    }

    /**
     * 包装函数，统一处理错误
     */
    async function wrapToolExecution(func: (params: any) => Promise<any>, params: any) {
        try {
            const result = await func(params);
            complete(result);
        } catch (error: any) {
            console.error(`工具执行失败`, error);
            complete({
                success: false,
                message: `工具执行时发生意外错误: ${error.message}`,
            });
        }
    }

    /**
     * 测试函数
     */
    async function main() {
        console.log("=== Crossref API 测试 ===\n");

        // 测试 1: 通过 DOI 查询
        console.log("1. 测试通过 DOI 查询...");
        const doiResult = await searchByDoi({ doi: "10.1038/nature12373" });
        console.log(JSON.stringify(doiResult, null, 2));
        console.log("\n");

        // 测试 2: 通过关键词搜索
        console.log("2. 测试通过关键词搜索...");
        const keywordResult = await searchByKeyword({ query: "machine learning", rows: 3 });
        console.log(JSON.stringify(keywordResult, null, 2));
        console.log("\n");

        // 测试 3: 通过作者搜索
        console.log("3. 测试通过作者搜索...");
        const authorResult = await searchByAuthor({ author: "John Smith", rows: 3 });
        console.log(JSON.stringify(authorResult, null, 2));
        console.log("\n");

        // 测试 4: 通过标题搜索
        console.log("4. 测试通过标题搜索...");
        const titleResult = await searchByTitle({ title: "neural networks", rows: 3 });
        console.log(JSON.stringify(titleResult, null, 2));
        console.log("\n");

        console.log("=== 测试完成 ===");
    }

    return {
        search_by_doi: (params: any) => wrapToolExecution(searchByDoi, params),
        search_by_keyword: (params: any) => wrapToolExecution(searchByKeyword, params),
        search_by_author: (params: any) => wrapToolExecution(searchByAuthor, params),
        search_by_title: (params: any) => wrapToolExecution(searchByTitle, params),
        search_by_issn: (params: any) => wrapToolExecution(searchByISSN, params),
        main,
    };
})();

// 导出工具函数
exports.search_by_doi = CrossrefSearch.search_by_doi;
exports.search_by_keyword = CrossrefSearch.search_by_keyword;
exports.search_by_author = CrossrefSearch.search_by_author;
exports.search_by_title = CrossrefSearch.search_by_title;
exports.search_by_issn = CrossrefSearch.search_by_issn;
exports.main = CrossrefSearch.main;
