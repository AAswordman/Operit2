/* METADATA
{
  "name": "agnes_video",
  "display_name": {
    "zh": "Agnes 视频生成",
    "en": "Agnes Video"
  },
  "description": {
    "zh": "使用 Agnes Video 2.5 API 生成视频，支持文生视频、首尾帧控制和多模态参考。非阻塞：提交任务立即返回 video_id，之后用 query_video 查询进度，完成时自动下载到插件私有目录。需在环境变量中二选一填写 AGNES_BASE_URL：apihub.agnes-ai.com（国际站）或 api.agnes-ai.cn（国内站），Key 与站点不能混用。",
    "en": "Generate videos via Agnes Video 2.5 API, supporting text-to-video, first/last frame control and multimodal reference. Non-blocking: submit returns video_id immediately; poll with query_video to check progress and auto-download into the plugin private directory. Pick one AGNES_BASE_URL in the environment settings: apihub.agnes-ai.com (international) or api.agnes-ai.cn (China); keys and sites cannot be mixed."
  },
  "category": "Media",
  "env": [
    {
      "name": "AGNES_API_KEY",
      "description": {
        "zh": "Agnes AI API Key（必填）",
        "en": "Agnes AI API key (required)"
      },
      "required": true
    },
    {
      "name": "AGNES_BASE_URL",
      "description": {
        "zh": "必填，二选一：apihub.agnes-ai.com 或 api.agnes-ai.cn",
        "en": "Required, one of: apihub.agnes-ai.com or api.agnes-ai.cn"
      },
      "required": true
    },
    {
      "name": "AGNES_VIDEO_MODEL",
      "description": {
        "zh": "可选，agnes-video-2.5 或 -2.5-flash，默认前者",
        "en": "Optional: agnes-video-2.5 or -2.5-flash, default former"
      },
      "required": false
    }
  ],
  "tools": [
    {
      "name": "submit_video",
      "description": {
        "zh": "提交视频生成任务，立即返回 video_id，不等待生成完成。之后用 query_video 查询结果。",
        "en": "Submit a video generation task and return video_id immediately without waiting. Use query_video later to fetch the result."
      },
      "parameters": [
        { "name": "prompt", "description": { "zh": "视频内容描述（必填）。reference 模式可用 <Picture N>、<Audio N>、<Video N> 指代输入素材，编号从 1 开始", "en": "Video content description (required). In reference mode use <Picture N>, <Audio N>, <Video N> to refer to inputs, numbered from 1" }, "type": "string", "required": true },
        { "name": "mode", "description": { "zh": "生成模式：text（纯文本）、keyframe（首尾帧控制）、reference（多模态参考）。默认 text", "en": "Generation mode: text, keyframe (first/last frame control) or reference (multimodal reference). Defaults to text" }, "type": "string", "required": false },
        { "name": "seconds", "description": { "zh": "视频时长（字符串 '4'-'12'，默认 '5'）", "en": "Video duration (string '4'-'12', default '5')" }, "type": "string", "required": false },
        { "name": "size", "description": { "zh": "输出分辨率档位：720P、1080P、1K、2K", "en": "Output resolution tier: 720P, 1080P, 1K or 2K" }, "type": "string", "required": false },
        { "name": "aspect_ratio", "description": { "zh": "画幅比例，默认 16:9", "en": "Aspect ratio, defaults to 16:9" }, "type": "string", "required": false },
        { "name": "seed", "description": { "zh": "随机种子，用于提高结果可复现性（可选）", "en": "Random seed for reproducibility (optional)" }, "type": "integer", "required": false },
        { "name": "first_frame", "description": { "zh": "首帧图片 URL（keyframe 模式，与 last_frame 至少提供一个）", "en": "First frame image URL (keyframe mode; at least one of first_frame or last_frame is required)" }, "type": "string", "required": false },
        { "name": "last_frame", "description": { "zh": "尾帧图片 URL（keyframe 模式，与 first_frame 至少提供一个）", "en": "Last frame image URL (keyframe mode; at least one of first_frame or last_frame is required)" }, "type": "string", "required": false },
        { "name": "images", "description": { "zh": "参考图片 URL 数组（reference 模式）。2.5 最多 8 张，2.5 Flash 最多 5 张", "en": "Reference image URL array (reference mode). Up to 8 for 2.5, up to 5 for 2.5 Flash" }, "type": "array", "required": false },
        { "name": "audios", "description": { "zh": "参考音频 URL 数组（reference 模式，最多 3 段）", "en": "Reference audio URL array (reference mode, up to 3)" }, "type": "array", "required": false },
        { "name": "videos", "description": { "zh": "参考视频数组（reference 模式，最多 1 个，2.5 Flash 不支持）。元素为 { url, start_seconds?, require_audio? } 对象，或直接传 URL 字符串", "en": "Reference video array (reference mode, up to 1, not supported by 2.5 Flash). Elements are { url, start_seconds?, require_audio? } objects, or plain URL strings" }, "type": "array", "required": false },
        { "name": "model", "description": { "zh": "覆盖模型：agnes-video-2.5 或 agnes-video-2.5-flash（可选，不填则取环境变量 AGNES_VIDEO_MODEL，再退回 agnes-video-2.5）", "en": "Override model: agnes-video-2.5 or agnes-video-2.5-flash (optional; falls back to env AGNES_VIDEO_MODEL, then agnes-video-2.5)" }, "type": "string", "required": false }
      ]
    },
    {
      "name": "query_video",
      "description": {
        "zh": "根据 video_id 查询任务状态。完成时自动下载视频到插件私有目录并返回本地路径。",
        "en": "Query task status by video_id. When completed, auto-download the video into the plugin private directory and return the local path."
      },
      "parameters": [
        { "name": "video_id", "description": { "zh": "提交任务时返回的 video_id（必填）", "en": "The video_id returned by submit_video (required)" }, "type": "string", "required": true },
        { "name": "model", "description": { "zh": "查询用模型名（可选，须与提交任务时一致；不填则取环境变量 AGNES_VIDEO_MODEL，再退回 agnes-video-2.5）", "en": "Model name used for the query (optional; must match the submit-time model. Falls back to env AGNES_VIDEO_MODEL, then agnes-video-2.5)" }, "type": "string", "required": false },
        { "name": "file_name", "description": { "zh": "自定义保存文件名（不含路径和扩展名，可选）", "en": "Custom output file name (without path and extension, optional)" }, "type": "string", "required": false }
      ]
    },
    {
      "name": "prompt_guide",
      "description": {
        "zh": "返回视频提示词的推荐写法，按主体与场景、动作与变化、镜头语言、视觉风格、声音与节奏、一致性要求六个层次组织。调用 submit_video 之前可先执行本工具，以获得更稳定的结果。",
        "en": "Return the recommended prompt writing guide for video generation, organised across six layers: subject and scene, action and change, camera language, visual style, sound and rhythm, and consistency requirements. Run this before submit_video for more stable results."
      },
      "parameters": []
    }
  ]
}*/

const agnesVideo = (function () {
    const HTTP_TIMEOUT_MS = 600000;
    const client = OkHttp.newBuilder()
        .connectTimeout(HTTP_TIMEOUT_MS)
        .readTimeout(HTTP_TIMEOUT_MS)
        .writeTimeout(HTTP_TIMEOUT_MS)
        .build();

    const INTERNATIONAL_BASE_URL = "https://apihub.agnes-ai.com/v1";
    const CHINA_BASE_URL = "https://api.agnes-ai.cn/v1";
    const SUPPORTED_BASE_URLS = [INTERNATIONAL_BASE_URL, CHINA_BASE_URL];
    const DEFAULT_MODEL = "agnes-video-2.5";
    const FLASH_MODEL = "agnes-video-2.5-flash";
    const SUPPORTED_MODELS = [DEFAULT_MODEL, FLASH_MODEL];
    const SUPPORTED_MODES = ["text", "keyframe", "reference"];
    const SUPPORTED_SIZES = ["720P", "1080P", "1K", "2K"];

    const PLUGIN_DIR = getPluginConfigDir("agnes_video");

    function getApiKey(): string {
        const apiKey = getEnv("AGNES_API_KEY");
        if (!apiKey) {
            throw new Error("AGNES_API_KEY 未配置，请在环境变量中设置 Agnes AI 的 API Key。");
        }
        return apiKey;
    }

    function resolveBaseUrl(): string {
        const raw = (getEnv("AGNES_BASE_URL") || "").trim();
        if (!raw) {
            throw new Error(
                `AGNES_BASE_URL 未配置，请选择 Agnes API 站点：${INTERNATIONAL_BASE_URL}（国际站）或 ${CHINA_BASE_URL}（国内站）。`
            );
        }

        const normalized = normalizeBaseUrl(raw);
        if (SUPPORTED_BASE_URLS.indexOf(normalized) < 0) {
            throw new Error(
                `AGNES_BASE_URL 只能是 ${INTERNATIONAL_BASE_URL}（国际站）或 ${CHINA_BASE_URL}（国内站），收到: ${raw}`
            );
        }
        return normalized;
    }

    function normalizeBaseUrl(raw: string): string {
        let value = raw.trim();
        if (!value) return value;
        if (!/^https?:\/\//i.test(value)) {
            value = `https://${value}`;
        }
        if (/^https?:\/\/apihub\.agnes-ai\.com\/?$/i.test(value)) return INTERNATIONAL_BASE_URL;
        if (/^https?:\/\/api\.agnes-ai\.cn\/?$/i.test(value)) return CHINA_BASE_URL;
        return value.replace(/\/+$/, "");
    }

    function resolveModel(explicit?: string | null): string {
        const fromParam = (explicit || "").trim();
        if (fromParam) return fromParam;
        const fromEnv = (getEnv("AGNES_VIDEO_MODEL") || "").trim();
        return fromEnv || DEFAULT_MODEL;
    }

    function assertSupportedModel(model: string): void {
        if (SUPPORTED_MODELS.indexOf(model) < 0) {
            throw new Error(
                `不支持的模型 \"${model}\"，可选值为: ${SUPPORTED_MODELS.join(", ")}。`
            );
        }
    }

    function createVideoEndpoint(baseUrl: string): string {
        return `${baseUrl}/videos`;
    }

    function buildQueryEndpoint(baseUrl: string, videoId: string, model: string): string {
        const origin = baseUrl.endsWith("/v1") ? baseUrl.slice(0, -3) : baseUrl;
        return (
            `${origin}/agnesapi?video_id=${encodeURIComponent(videoId)}` +
            `&model_name=${encodeURIComponent(model)}`
        );
    }

    function sanitizeFileName(name: string): string {
        const safe = name.replace(/[\\/:*?\"<>|]/g, "_").trim();
        if (!safe) return `agnes_video_${Date.now()}`;
        return safe.substring(0, 80);
    }

    async function ensureDirectory(): Promise<void> {
        try {
            const result = await Tools.Files.mkdir(PLUGIN_DIR, true);
            if (!result.successful) {
                console.warn(`创建目录失败(可能已存在): ${PLUGIN_DIR} -> ${result.details}`);
            }
        } catch (e: any) {
            console.warn(`创建目录异常: ${PLUGIN_DIR} -> ${e?.message || e}`);
        }
    }

    function normalizeSeconds(raw?: string | number | null): string | undefined {
        if (raw === undefined || raw === null) return undefined;
        const text = String(raw).trim();
        if (!text) return undefined;
        const value = Number(text);
        if (!Number.isFinite(value) || value < 4 || value > 12) {
            throw new Error(`seconds 必须是 4 到 12 之间的数值（字符串形式），收到: ${text}`);
        }
        return String(value);
    }

    function normalizeSize(raw?: string | null): string | undefined {
        const text = (raw || "").trim();
        if (!text) return undefined;
        const upper = text.toUpperCase();
        if (SUPPORTED_SIZES.indexOf(upper) < 0) {
            throw new Error(
                `size 不支持 \"${text}\"，可选值为: ${SUPPORTED_SIZES.join(", ")}。`
            );
        }
        return upper;
    }

    function normalizeMode(raw?: string | null): string {
        const text = (raw || "").trim().toLowerCase();
        if (!text) return "text";
        if (SUPPORTED_MODES.indexOf(text) < 0) {
            throw new Error(
                `mode 不支持 \"${raw}\"，可选值为: ${SUPPORTED_MODES.join(", ")}。`
            );
        }
        return text;
    }

    function normalizeUrlList(raw: any, field: string): string[] {
        if (raw === undefined || raw === null) return [];
        if (!Array.isArray(raw)) {
            throw new Error(`${field} 必须是 URL 字符串数组。`);
        }
        const urls: string[] = [];
        for (const item of raw) {
            if (typeof item !== "string" || !item.trim()) {
                throw new Error(`${field} 中每个元素必须是非空 URL 字符串。`);
            }
            urls.push(item.trim());
        }
        return urls;
    }

    function normalizeVideoRefs(raw: any): any[] {
        if (raw === undefined || raw === null) return [];
        if (!Array.isArray(raw)) {
            throw new Error("videos 必须是数组。");
        }
        const refs: any[] = [];
        for (const item of raw) {
            if (typeof item === "string") {
                if (!item.trim()) {
                    throw new Error("videos 中的 URL 字符串不能为空。");
                }
                refs.push({ url: item.trim() });
                continue;
            }
            if (!item || typeof item !== "object" || typeof item.url !== "string" || !item.url.trim()) {
                throw new Error("videos 中每个元素必须是 { url, start_seconds?, require_audio? } 对象或 URL 字符串。");
            }
            const ref: any = { url: item.url.trim() };
            if (item.start_seconds !== undefined && item.start_seconds !== null) {
                const startSeconds = Number(item.start_seconds);
                if (!Number.isFinite(startSeconds) || startSeconds < 0) {
                    throw new Error("videos[].start_seconds 必须是非负数值。");
                }
                ref.start_seconds = startSeconds;
            }
            if (item.require_audio !== undefined && item.require_audio !== null) {
                ref.require_audio = Boolean(item.require_audio);
            }
            refs.push(ref);
        }
        return refs;
    }

    function buildCreateRequestBody(params: any, model: string): any {
        const mode = normalizeMode(params.mode);

        const firstFrame = (params.first_frame || "").trim();
        const lastFrame = (params.last_frame || "").trim();
        const images = normalizeUrlList(params.images, "images");
        const audios = normalizeUrlList(params.audios, "audios");
        const videos = normalizeVideoRefs(params.videos);

        const body: any = {
            model: model,
            prompt: String(params.prompt).trim(),
            mode: mode,
        };

        const seconds = normalizeSeconds(params.seconds);
        if (seconds !== undefined) body.seconds = seconds;

        const size = normalizeSize(params.size);
        if (size !== undefined) body.size = size;

        const aspectRatio = (params.aspect_ratio || "").trim();
        if (aspectRatio) body.aspect_ratio = aspectRatio;

        if (params.seed !== undefined && params.seed !== null && String(params.seed).trim() !== "") {
            const seed = Number(params.seed);
            if (!Number.isInteger(seed)) {
                throw new Error(`seed 必须是整数，收到: ${params.seed}`);
            }
            body.seed = seed;
        }

        if (mode === "text") {
            if (firstFrame || lastFrame || images.length > 0 || audios.length > 0 || videos.length > 0) {
                throw new Error("mode=text 不接受 first_frame、last_frame、images、audios 或 videos。请改用 keyframe 或 reference 模式。");
            }
        } else if (mode === "keyframe") {
            if (!firstFrame && !lastFrame) {
                throw new Error("mode=keyframe 需要 first_frame 或 last_frame 至少提供一个。");
            }
            if (images.length > 0 || audios.length > 0 || videos.length > 0) {
                throw new Error("mode=keyframe 不接受 images、audios 或 videos。需要素材参考时请改用 reference 模式。");
            }
            if (firstFrame) body.first_frame = firstFrame;
            if (lastFrame) body.last_frame = lastFrame;
        } else {
            if (firstFrame || lastFrame) {
                throw new Error("mode=reference 不接受 first_frame 或 last_frame。需要首尾帧控制时请改用 keyframe 模式。");
            }
            if (images.length === 0 && audios.length === 0 && videos.length === 0) {
                throw new Error("mode=reference 需要 images、audios 或 videos 至少一类非空。");
            }

            if (model === FLASH_MODEL) {
                if (videos.length > 0) {
                    throw new Error("agnes-video-2.5-flash 不支持参考视频（videos），请改用 agnes-video-2.5。");
                }
                if (images.length > 5) {
                    throw new Error(`agnes-video-2.5-flash 的参考图片最多 5 张，当前传入 ${images.length} 张。`);
                }
                if (audios.length > 3) {
                    throw new Error(`agnes-video-2.5-flash 的参考音频最多 3 段，当前传入 ${audios.length} 段。`);
                }
            } else {
                if (images.length > 8) {
                    throw new Error(`参考图片最多 8 张，当前传入 ${images.length} 张。`);
                }
                if (audios.length > 3) {
                    throw new Error(`参考音频最多 3 段，当前传入 ${audios.length} 段。`);
                }
                if (videos.length > 1) {
                    throw new Error(`参考视频最多 1 个，当前传入 ${videos.length} 个。`);
                }
            }

            const mediaCount = images.length + audios.length + videos.length;
            if (mediaCount > 12) {
                throw new Error(`单次请求中的参考媒体文件总数不得超过 12 个，当前传入 ${mediaCount} 个。`);
            }

            if (images.length > 0) body.images = images;
            if (audios.length > 0) body.audios = audios;
            if (videos.length > 0) body.videos = videos;
        }

        return body;
    }

    async function createVideoTask(params: any): Promise<any> {
        const apiKey = getApiKey();
        const model = resolveModel(params.model);
        assertSupportedModel(model);

        const baseUrl = resolveBaseUrl();
        const endpoint = createVideoEndpoint(baseUrl);
        const body = buildCreateRequestBody(params, model);

        const headers = {
            "accept": "application/json",
            "content-type": "application/json",
            "Authorization": `Bearer ${apiKey}`,
        };

        const request = client
            .newRequest()
            .url(endpoint)
            .method("POST")
            .headers(headers)
            .body(JSON.stringify(body), "json");

        const response = await request.build().execute();
        if (!response.isSuccessful()) {
            throw new Error(`创建视频任务失败: ${response.statusCode} - ${response.content}`);
        }

        let parsed: any;
        try {
            parsed = JSON.parse(response.content);
        } catch (e: any) {
            throw new Error(`解析创建任务响应失败: ${e?.message || e}`);
        }

        const videoId = parsed.video_id;
        if (!videoId) {
            throw new Error(
                `创建任务响应中未找到 video_id，响应: ${JSON.stringify(parsed)}`
            );
        }

        console.log(
            `视频任务创建成功 -> video_id: ${videoId}, task_id: ${parsed.task_id || parsed.id || ""}, status: ${parsed.status}`
        );

        return {
            video_id: String(videoId),
            task_id: String(parsed.task_id || parsed.id || ""),
            status: parsed.status || "queued",
            model: parsed.model || model,
            size: parsed.size || "",
            seconds: parsed.seconds || "",
        };
    }

    async function queryVideoResult(videoId: string, model: string): Promise<any> {
        const apiKey = getApiKey();
        const baseUrl = resolveBaseUrl();
        const queryUrl = buildQueryEndpoint(baseUrl, videoId, model);

        const request = client
            .newRequest()
            .url(queryUrl)
            .method("GET")
            .headers({
                "accept": "application/json",
                "Authorization": `Bearer ${apiKey}`,
            });

        const response = await request.build().execute();
        if (!response.isSuccessful()) {
            throw new Error(`查询视频结果失败: ${response.statusCode} - ${response.content}`);
        }

        let parsed: any;
        try {
            parsed = JSON.parse(response.content);
        } catch (e: any) {
            throw new Error(`解析查询结果响应失败: ${e?.message || e}`);
        }

        return {
            status: parsed.status || "unknown",
            progress: typeof parsed.progress === "number" ? parsed.progress : 0,
            url: parsed.url,
            video_id: parsed.video_id || videoId,
            task_id: parsed.task_id || parsed.id || "",
            size: parsed.size || "",
            seconds: parsed.seconds || "",
            quality: parsed.quality || "",
            error: parsed.error,
        };
    }

    async function downloadVideo(videoUrl: string, filePath: string): Promise<string> {
        console.log(`下载视频: ${videoUrl} -> ${filePath}`);
        const downloadResult = await Tools.Files.download(videoUrl, filePath);
        if (!downloadResult.successful) {
            throw new Error(`下载视频失败: ${downloadResult.details}`);
        }
        console.log(`视频已保存到: ${filePath}`);
        return filePath;
    }

    async function submitVideo(params: any): Promise<any> {
        if (!params || typeof params.prompt !== "string" || !params.prompt.trim()) {
            throw new Error("参数 prompt 不能为空。");
        }

        await ensureDirectory();
        const creationResult = await createVideoTask(params);

        const hintLines: string[] = [];
        hintLines.push("视频任务已提交。");
        hintLines.push(`video_id: ${creationResult.video_id}`);
        if (creationResult.task_id) hintLines.push(`task_id: ${creationResult.task_id}`);
        hintLines.push(`状态: ${creationResult.status}`);
        hintLines.push(`模型: ${creationResult.model}`);
        hintLines.push(`提示词: ${String(params.prompt).trim()}`);
        hintLines.push(`分辨率: ${creationResult.size || "(由服务端决定)"}`);
        hintLines.push(`时长: ${creationResult.seconds || "(未知)"} 秒`);
        hintLines.push("");
        hintLines.push(
            `后续请使用 query_video 工具，传入 video_id=\"${creationResult.video_id}\"${
                creationResult.model !== DEFAULT_MODEL ? ` 和 model=\"${creationResult.model}\"` : ""
            } 来查询进度。`
        );
        hintLines.push(`生成完成后会自动下载到插件私有目录 ${PLUGIN_DIR}/。`);

        return {
            video_id: creationResult.video_id,
            task_id: creationResult.task_id,
            status: creationResult.status,
            model: creationResult.model,
            size: creationResult.size,
            seconds: creationResult.seconds,
            hint: hintLines.join("\n"),
        };
    }

    async function queryVideo(params: any): Promise<any> {
        if (!params || typeof params.video_id !== "string" || !params.video_id.trim()) {
            throw new Error("参数 video_id 不能为空。");
        }

        const videoId = params.video_id.trim();
        const model = resolveModel(params.model);
        assertSupportedModel(model);

        const result = await queryVideoResult(videoId, model);

        if (result.status === "completed") {
            if (!result.url) {
                throw new Error(
                    `任务已完成但响应中缺少视频 url，无法下载。video_id: ${videoId}`
                );
            }
            await ensureDirectory();
            const fileName = params.file_name
                ? sanitizeFileName(params.file_name)
                : `agnes_video_${videoId}`;
            const filePath = `${PLUGIN_DIR}/${fileName}.mp4`;
            await downloadVideo(result.url, filePath);

            const hintLines: string[] = [];
            hintLines.push("视频已生成完毕并下载到本地。");
            hintLines.push(`本地路径: ${filePath}`);
            hintLines.push(`video_id: ${videoId}`);
            hintLines.push(`分辨率: ${result.size || "未知"}`);
            hintLines.push(`时长: ${result.seconds || "未知"} 秒`);

            return {
                status: "completed",
                file_path: filePath,
                file_uri: `file://${filePath}`,
                video_url: result.url,
                video_id: videoId,
                task_id: result.task_id,
                size: result.size,
                seconds: result.seconds,
                hint: hintLines.join("\n"),
            };
        }

        if (result.status === "failed") {
            const errInfo = result.error ? JSON.stringify(result.error) : "未知错误";
            throw new Error(`视频生成失败: ${errInfo}`);
        }

        const hintLines: string[] = [];
        hintLines.push("任务尚未完成。");
        hintLines.push(`video_id: ${videoId}`);
        hintLines.push(`状态: ${result.status}`);
        hintLines.push(`进度: ${result.progress}%`);

        return {
            status: result.status,
            progress: result.progress,
            video_id: videoId,
            task_id: result.task_id,
            hint: hintLines.join("\n"),
        };
    }

    async function submit_video_wrapper(params: any) {
        const result = await submitVideo(params);
        const hintLines: string[] = [];
        hintLines.push(`视频任务已成功提交，video_id: ${result.video_id}。`);
        if (result.model) {
            hintLines.push(
                result.model !== DEFAULT_MODEL
                    ? `查询时请传入 model="${result.model}"。`
                    : "查询时无需额外传 model。"
            );
        }
        hintLines.push(result.hint);
        return {
            success: true,
            message: hintLines.join("\n"),
            data: result,
        };
    }

    async function query_video_wrapper(params: any) {
        const result = await queryVideo(params);
        return {
            success: true,
            message: result.hint,
            data: result,
        };
    }

    const PROMPT_GUIDE_STEPS = [
        "主体与场景：明确人物、物体、环境和时间。",
        "动作与变化：描述主体如何移动，以及场景如何变化。",
        "镜头语言：指定推、拉、摇、移、跟拍、固定镜头或景别。",
        "视觉风格：补充光线、色彩、材质、写实程度和氛围。",
        "声音与节奏：需要时描述环境声、动作声或引用音频素材。",
        "一致性要求：说明需要保持不变的主体外观、产品细节或构图。",
    ];

    function buildPromptGuide(): any {
        const lines: string[] = [];
        lines.push("为了获得更稳定的结果，建议按以下顺序描述提示词：");
        PROMPT_GUIDE_STEPS.forEach((step, index) => {
            lines.push(`${index + 1}. ${step}`);
        });

        return {
            guide: lines.join("\n"),
            steps: PROMPT_GUIDE_STEPS.map((step, index) => ({
                order: index + 1,
                text: step,
            })),
        };
    }

    async function prompt_guide_wrapper(_params: any) {
        const result = buildPromptGuide();
        return {
            success: true,
            message: result.guide,
            data: result,
        };
    }

    return {
        submit_video: submit_video_wrapper,
        query_video: query_video_wrapper,
        prompt_guide: prompt_guide_wrapper,
    };
})();

exports.submit_video = agnesVideo.submit_video;
exports.query_video = agnesVideo.query_video;
exports.prompt_guide = agnesVideo.prompt_guide;
