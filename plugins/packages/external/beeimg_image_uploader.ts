/* METADATA
{
    "api_version": "2.0.0",
    "name": "beeimg_image_uploader_v2",

    "display_name": {
        "zh": "Beeimg 图片上传器 V2",
        "en": "Beeimg Image Uploader V2"
    },
    "description": {
        "zh": "BeeIMG（https://beeimg.com/）工具将本地图片上传到图床并返回图片url，配合图片生成工具实现图生图（图生图务必开启）。",
        "en": "Upload a local image to BeeIMG (https://beeimg.com/) and return a public image URL. Useful for image-to-image workflows (make sure img2img is enabled)."
    },
    "env": [
        "BEEIMG_API_KEY"
    ],
    "category": "Media",
    "tools": [
        {
            "name": "upload_image",
            "description": {
                "zh": "使用 multipart 上传将本地图片上传到 BeeIMG 图床并返回图片 url。",
                "en": "Upload a local image to BeeIMG via multipart upload and return the image URL."
            },
            "parameters": [
                { "name": "file_path", "description": { "zh": "要上传的图片文件绝对路径 (建议使用 /sdcard/ 开头的完整路径)。", "en": "Absolute path of the image file to upload (recommended: full path starting with /sdcard/)." }, "type": "string", "required": true },
                { "name": "album_id", "description": { "zh": "相册ID (可选)。", "en": "Album ID (optional)." }, "type": "string", "required": false },
                { "name": "privacy", "description": { "zh": "隐私设置，'public' 或 'private' (可选)。", "en": "Privacy setting: 'public' or 'private' (optional)." }, "type": "string", "required": false }
            ]
        }
    ]
}*/

const beeimgUploader = (function () {
    // API配置
    const API_ENDPOINT = "https://beeimg.com/api/upload/file/json/";

    function isRecord(value: unknown): value is Record<string, unknown> {
        return typeof value === "object" && value !== null;
    }

    function getErrorMessage(error: unknown): string {
        if (error instanceof Error) return error.message;
        return String(error);
    }

    function getErrorStack(error: unknown): string | undefined {
        if (error instanceof Error) return error.stack;
        return undefined;
    }

    function getApiKey(): string {
        return getEnv("BEEIMG_API_KEY") || "";
    }

    function guessMimeTypeFromPath(filePath: string): string {
        const lower = filePath.toLowerCase();
        if (lower.endsWith(".png")) return "image/png";
        if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
        if (lower.endsWith(".webp")) return "image/webp";
        if (lower.endsWith(".gif")) return "image/gif";
        return "application/octet-stream";
    }

    function safeJsonParseLoose(text: string): unknown {
        const trimmed = (text || "").trim();
        if (!trimmed) return null;
        try {
            return JSON.parse(trimmed) as unknown;
        } catch (e) {
            const start = trimmed.indexOf("{");
            const end = trimmed.lastIndexOf("}");
            if (start !== -1 && end !== -1 && end > start) {
                return JSON.parse(trimmed.substring(start, end + 1)) as unknown;
            }
            throw e;
        }
    }

    interface UploadImageParams {
        file_path: string;
        album_id?: string;
        privacy?: string;
    }

    interface UploadImageResult {
        url: string;
        thumbnail_url?: string;
        page_url?: string;
        details: string;
    }

    async function upload_image(params: UploadImageParams): Promise<UploadImageResult> {
        const file_path = params?.file_path;
        const album_id = params?.album_id;
        const privacy = params?.privacy;

        if (!file_path || String(file_path).trim().length === 0) {
            throw new Error("参数 file_path 不能为空。");
        }

        // 1. 检查文件是否存在
        const fileExists = await Tools.Files.exists(file_path);
        if (!fileExists.exists) {
            throw new Error(`文件未找到: ${file_path} (请尝试使用 /sdcard/ 开头的绝对路径)`);
        }

        // 2. 准备参数
        const apiKey = getApiKey();
        if (!apiKey) {
            throw new Error("BEEIMG_API_KEY 未配置，请在环境变量中设置 BeeIMG 的 API Key。");
        }

        const form_data: Record<string, string> = {
            apikey: apiKey
        };
        if (album_id) form_data.albumid = String(album_id);
        if (privacy) form_data.privacy = String(privacy);

        // 3. 发起 multipart 上传
        console.log("正在执行上传...");

        const resp = await Tools.Net.uploadFile({
            url: API_ENDPOINT,
            method: "POST",
            headers: {
                // 尽量模拟浏览器 UA，降低被风控概率
                "User-Agent": "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Mobile Safari/537.36"
            },
            form_data,
            files: [
                {
                    field_name: "file",
                    file_path: file_path,
                    content_type: guessMimeTypeFromPath(file_path)
                }
            ]
        });

        if (resp.statusCode < 200 || resp.statusCode >= 300) {
            throw new Error(`上传请求失败 (HTTP ${resp.statusCode})。可能是网络问题或 IP 被屏蔽。\n响应: ${resp.content}`);
        }

        const responseText = (resp.content || "").trim();
        if (!responseText) {
            throw new Error("上传失败：服务器返回了空内容。请检查网络连接或尝试关闭 VPN。");
        }

        let jsonResponse: unknown;
        try {
            jsonResponse = safeJsonParseLoose(responseText);
        } catch (e: unknown) {
            throw new Error(`API 响应解析失败。服务器返回内容不是 JSON:\n${responseText.substring(0, 200)}...`);
        }

        const files = isRecord(jsonResponse) && isRecord(jsonResponse["files"]) ? (jsonResponse["files"] as Record<string, unknown>) : null;
        const ok = !!files && (files["status"] === "Success" || files["code"] === "200" || files["code"] === 200);
        const url = files ? files["url"] : undefined;

        if (ok && (typeof url === "string" || typeof url === "number") && String(url).trim().length > 0) {
            return {
                url: String(url),
                thumbnail_url: files && files["thumbnail_url"] ? String(files["thumbnail_url"]) : undefined,
                page_url: files && files["view_url"] ? String(files["view_url"]) : undefined,
                details: `上传成功! URL: ${String(url)}`
            };
        } else {
            const errMsg = files
                ? (files["status"] || files["message"] || JSON.stringify(files))
                : "未知错误";
            throw new Error(`BeeIMG API 报错: ${errMsg}`);
        }
    }

    async function wrap<TParams, TResult>(func: (params: TParams) => Promise<TResult>, params: TParams) {
        try {
            const result = await func(params);
            complete({ success: true, message: "图片上传成功", data: result });
        }
        catch (error: unknown) {
            console.error(`Error: ${getErrorMessage(error)}`);
            complete({ success: false, message: getErrorMessage(error), error_stack: getErrorStack(error) });
        }
    }

    return {
        upload_image: (p: UploadImageParams) => wrap(upload_image, p)
    };
})();

exports.upload_image = beeimgUploader.upload_image;
