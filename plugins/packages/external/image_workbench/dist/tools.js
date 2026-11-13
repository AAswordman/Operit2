/* METADATA
{
  "name": "image_workbench",
  "display_name": {
    "zh": "生图工作台",
    "en": "Image Workbench"
  },
  "description": {
    "zh": "统一生图工具。通过provider选择服务；可先调用list_methods查询支持的参数。生成结果统一返回assets文件列表和markdown引用。",
    "en": "Unified image generation. Select a provider and inspect list_methods for supported options. All methods return assets and Markdown."
  },
  "category": "Draw",
  "env": [
    {
      "name": "OPENAI_API_KEY",
      "description": {
        "zh": "OPENAI_API_KEY",
        "en": "OPENAI_API_KEY"
      },
      "required": false
    },
    {
      "name": "OPENAI_API_BASE_URL",
      "description": {
        "zh": "OPENAI_API_BASE_URL",
        "en": "OPENAI_API_BASE_URL"
      },
      "required": false
    },
    {
      "name": "DASHSCOPE_API_KEY",
      "description": {
        "zh": "DASHSCOPE_API_KEY",
        "en": "DASHSCOPE_API_KEY"
      },
      "required": false
    },
    {
      "name": "DASHSCOPE_API_BASE_URL",
      "description": {
        "zh": "DASHSCOPE_API_BASE_URL",
        "en": "DASHSCOPE_API_BASE_URL"
      },
      "required": false
    },
    {
      "name": "ZHIPU_API_KEY",
      "description": {
        "zh": "ZHIPU_API_KEY",
        "en": "ZHIPU_API_KEY"
      },
      "required": false
    },
    {
      "name": "XAI_API_KEY",
      "description": {
        "zh": "XAI_API_KEY",
        "en": "XAI_API_KEY"
      },
      "required": false
    },
    {
      "name": "MINIMAX_API_KEY",
      "description": {
        "zh": "MINIMAX_API_KEY",
        "en": "MINIMAX_API_KEY"
      },
      "required": false
    },
    {
      "name": "MINIMAX_API_BASE_URL",
      "description": {
        "zh": "MINIMAX_API_BASE_URL",
        "en": "MINIMAX_API_BASE_URL"
      },
      "required": false
    },
    {
      "name": "NANOBANANA_API_KEY",
      "description": {
        "zh": "NANOBANANA_API_KEY",
        "en": "NANOBANANA_API_KEY"
      },
      "required": false
    },
    {
      "name": "NANOBANANA_API_BASE_URL",
      "description": {
        "zh": "NANOBANANA_API_BASE_URL",
        "en": "NANOBANANA_API_BASE_URL"
      },
      "required": false
    },
    {
      "name": "BEEIMG_API_KEY",
      "description": {
        "zh": "BEEIMG_API_KEY",
        "en": "BEEIMG_API_KEY"
      },
      "required": false
    }
  ],
  "tools": [
    {
      "name": "list_methods",
      "description": {
        "zh": "列出生图方式、支持参数、默认模型和密钥配置状态，不返回密钥。",
        "en": "List supported providers, options, default models and credential readiness without secrets."
      },
      "parameters": []
    },
    {
      "name": "generate_image",
      "description": {
        "zh": "统一生成图片。通过provider选择方式，使用统一参数。返回已保存的VFS文件列表assets和markdown图片引用。",
        "en": "Generate images with shared parameters and return saved VFS assets and Markdown references."
      },
      "parameters": [
        {
          "name": "provider",
          "description": {
            "zh": "生图方式：openai、qwen、zhipu、xai、minimax、nanobanana",
            "en": "生图方式：openai、qwen、zhipu、xai、minimax、nanobanana"
          },
          "type": "string",
          "required": true
        },
        {
          "name": "prompt",
          "description": {
            "zh": "提示词 / Prompt",
            "en": "提示词 / Prompt"
          },
          "type": "string",
          "required": true
        },
        {
          "name": "model",
          "description": {
            "zh": "模型名（可选，默认模型见 list_methods）",
            "en": "模型名（可选，默认模型见 list_methods）"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "file_name",
          "description": {
            "zh": "输出文件名，不包含目录或扩展名",
            "en": "输出文件名，不包含目录或扩展名"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "size",
          "description": {
            "zh": "像素尺寸，格式为宽x高，如1024x1024；OpenAI、Qwen、智谱、xAI、MiniMax支持",
            "en": "像素尺寸，格式为宽x高，如1024x1024；OpenAI、Qwen、智谱、xAI、MiniMax支持"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "aspect_ratio",
          "description": {
            "zh": "画面比例，例如16:9；MiniMax、Nano Banana支持",
            "en": "画面比例，例如16:9；MiniMax、Nano Banana支持"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "resolution",
          "description": {
            "zh": "Nano Banana分辨率：1K、2K、4K",
            "en": "Nano Banana分辨率：1K、2K、4K"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "count",
          "description": {
            "zh": "图片数量；Qwen为1-4，MiniMax为1-9",
            "en": "图片数量；Qwen为1-4，MiniMax为1-9"
          },
          "type": "number",
          "required": false
        },
        {
          "name": "negative_prompt",
          "description": {
            "zh": "负面提示词；Qwen支持",
            "en": "负面提示词；Qwen支持"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "optimize_prompt",
          "description": {
            "zh": "提示词优化；Qwen、MiniMax支持",
            "en": "提示词优化；Qwen、MiniMax支持"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "watermark",
          "description": {
            "zh": "是否添加水印；Qwen、MiniMax支持",
            "en": "是否添加水印；Qwen、MiniMax支持"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "seed",
          "description": {
            "zh": "随机种子；MiniMax支持",
            "en": "随机种子；MiniMax支持"
          },
          "type": "number",
          "required": false
        },
        {
          "name": "reference_images",
          "description": {
            "zh": "参考图片URL或VFS路径数组；MiniMax、Nano Banana支持。本地文件会上传到Beeimg，需要BEEIMG_API_KEY",
            "en": "参考图片URL或VFS路径数组；MiniMax、Nano Banana支持。本地文件会上传到Beeimg，需要BEEIMG_API_KEY"
          },
          "type": "array",
          "required": false
        }
      ]
    },
    {
      "name": "generate_video",
      "description": {
        "zh": "通过xAI生成或编辑视频，结果使用与图片相同的assets文件列表结构。",
        "en": "Generate or edit a video through xAI with the shared asset result structure."
      },
      "parameters": [
        {
          "name": "provider",
          "description": {
            "zh": "当前固定为xai",
            "en": "当前固定为xai"
          },
          "type": "string",
          "required": true
        },
        {
          "name": "prompt",
          "description": {
            "zh": "提示词 / Prompt",
            "en": "提示词 / Prompt"
          },
          "type": "string",
          "required": true
        },
        {
          "name": "model",
          "description": {
            "zh": "模型名（可选，默认模型见 list_methods）",
            "en": "模型名（可选，默认模型见 list_methods）"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "file_name",
          "description": {
            "zh": "输出文件名，不包含目录或扩展名",
            "en": "输出文件名，不包含目录或扩展名"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "aspect_ratio",
          "description": {
            "zh": "画面比例，如16:9",
            "en": "画面比例，如16:9"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "resolution",
          "description": {
            "zh": "480p或720p",
            "en": "480p或720p"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "duration",
          "description": {
            "zh": "视频时长，1-15秒",
            "en": "视频时长，1-15秒"
          },
          "type": "number",
          "required": false
        },
        {
          "name": "reference_image",
          "description": {
            "zh": "图生视频输入图片URL或VFS路径",
            "en": "图生视频输入图片URL或VFS路径"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "reference_video",
          "description": {
            "zh": "视频编辑输入视频URL，与reference_image互斥",
            "en": "视频编辑输入视频URL，与reference_image互斥"
          },
          "type": "string",
          "required": false
        }
      ]
    }
  ]
}
*/
"use strict";
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/tools.ts
var tools_exports = {};
__export(tools_exports, {
  generate_image: () => generate_image,
  generate_video: () => generate_video,
  list_methods: () => list_methods
});
module.exports = __toCommonJS(tools_exports);

// src/catalog.ts
var PROVIDERS = [
  { id: "openai", title: "OpenAI", description: "OpenAI 与兼容图像接口", model: "gpt-image-1", apiKey: "OPENAI_API_KEY", endpointKey: "OPENAI_API_BASE_URL", endpoint: "https://api.openai.com", options: ["size"] },
  { id: "qwen", title: "通义千问", description: "百炼 / 通义万相，支持负面提示词", model: "qwen-image-plus", apiKey: "DASHSCOPE_API_KEY", endpointKey: "DASHSCOPE_API_BASE_URL", endpoint: "https://dashscope.aliyuncs.com", options: ["size", "count", "negative_prompt", "optimize_prompt", "watermark"] },
  { id: "zhipu", title: "智谱", description: "GLM Image 图像生成", model: "glm-image", apiKey: "ZHIPU_API_KEY", options: ["size"] },
  { id: "xai", title: "xAI / Grok", description: "图像生成，也可切换到视频", model: "grok-2-image-1212", apiKey: "XAI_API_KEY", options: ["size"] },
  { id: "minimax", title: "MiniMax", description: "文生图、参考图与批量生成", model: "image-01", apiKey: "MINIMAX_API_KEY", endpointKey: "MINIMAX_API_BASE_URL", endpoint: "https://api.minimaxi.com", options: ["size", "aspect_ratio", "count", "optimize_prompt", "watermark", "seed", "reference_images"] },
  { id: "nanobanana", title: "Nano Banana", description: "Grsai 接口，支持参考图与高分辨率", model: "nano-banana-pro", apiKey: "NANOBANANA_API_KEY", endpointKey: "NANOBANANA_API_BASE_URL", endpoint: "https://grsai.dakka.com.cn", options: ["aspect_ratio", "resolution", "reference_images"] }
];
function providerById(id) {
  const provider = PROVIDERS.find((item) => item.id === id);
  if (!provider) throw new Error(`未知生图方式：${id}`);
  return provider;
}
function validateCommon(input) {
  if (typeof input.prompt !== "string" || !input.prompt.trim()) throw new Error("请填写提示词。");
  if (input.model !== void 0 && (typeof input.model !== "string" || !input.model.trim())) throw new Error("模型名称不能为空。");
  if (input.file_name !== void 0 && (typeof input.file_name !== "string" || !input.file_name.trim() || /[\\/:*?"<>|]/.test(input.file_name) || [".", ".."].includes(input.file_name.trim()))) throw new Error("保存名称不能为空或包含目录与特殊字符。");
}
function validateImageRequest(input) {
  const provider = providerById(input.provider);
  validateCommon(input);
  const allowed = /* @__PURE__ */ new Set(["provider", "prompt", "model", "file_name", ...provider.options]);
  for (const [key, value] of Object.entries(input)) {
    if (value !== void 0 && !allowed.has(key)) throw new Error(`${provider.title} 不支持参数 ${key}。`);
  }
  for (const key of ["size", "aspect_ratio", "resolution", "negative_prompt"]) {
    if (input[key] !== void 0 && (typeof input[key] !== "string" || !input[key].trim())) throw new Error(`${key} 需要填写非空文字。`);
  }
  if (input.count !== void 0 && (!Number.isInteger(input.count) || input.count < 1 || input.count > (provider.id === "minimax" ? 9 : 4))) throw new Error("图片数量超出所选方式允许的范围。");
  if (input.seed !== void 0 && !Number.isInteger(input.seed)) throw new Error("随机种子必须为整数。");
  for (const key of ["optimize_prompt", "watermark"]) {
    if (input[key] !== void 0 && typeof input[key] !== "boolean") throw new Error(`${key} 必须是布尔值。`);
  }
  if (input.size !== void 0 && !/^\d+x\d+$/.test(input.size)) throw new Error("尺寸格式应为 1024x1024。");
  if (input.reference_images !== void 0 && (!Array.isArray(input.reference_images) || input.reference_images.some((uri) => typeof uri !== "string" || !uri.trim()))) throw new Error("参考图必须是非空 URL 或 VFS 路径的数组。");
  return provider;
}
function validateVideoRequest(input) {
  if (input.provider !== "xai") throw new Error("当前只有 xAI 支持视频生成。");
  validateCommon(input);
  const allowed = /* @__PURE__ */ new Set(["provider", "prompt", "model", "file_name", "aspect_ratio", "resolution", "duration", "reference_image", "reference_video"]);
  for (const [key, value] of Object.entries(input)) if (value !== void 0 && !allowed.has(key)) throw new Error(`视频生成不支持参数 ${key}。`);
  for (const key of ["aspect_ratio", "resolution", "reference_image", "reference_video"]) if (input[key] !== void 0 && (typeof input[key] !== "string" || !input[key].trim())) throw new Error(`${key} 需要填写非空文字。`);
  if (input.reference_image && input.reference_video) throw new Error("参考图片和参考视频只能选择一种。");
  if (input.reference_video && !/^https?:\/\//.test(input.reference_video)) throw new Error("参考视频需要填写公网 URL。");
  return providerById(input.provider);
}

// src/providers/shared.ts
function apiKey(name) {
  const value = getEnv(name)?.trim();
  if (!value) throw new Error(`请配置 ${name}。`);
  return value;
}
function endpoint(key, defaultBase, prefix, resource) {
  const configured = getEnv(key)?.trim();
  const base = (configured ? configured : defaultBase).replace(/\/+$/, "");
  if (!/^https?:\/\/[^\s?#]+$/.test(base)) throw new Error(`接口地址无效：${key}`);
  const suffix = `${prefix}/${resource}`;
  if (base.endsWith(suffix)) return base;
  return base.endsWith(prefix) ? `${base}/${resource}` : `${base}${suffix}`;
}
async function request(url, key, body, headers = {}) {
  const client = OkHttp.newBuilder().connectTimeout(6e5).readTimeout(6e5).writeTimeout(6e5).build();
  const builder = client.newRequest().url(url).method(body === void 0 ? "GET" : "POST").headers({ accept: "application/json", "content-type": "application/json", Authorization: `Bearer ${key}`, ...headers });
  if (body !== void 0) builder.body(JSON.stringify(body), "json");
  const response = await builder.build().execute();
  if (!response.isSuccessful()) throw new Error(`生成服务请求失败（HTTP ${response.statusCode}）：${response.content}`);
  return JSON.parse(response.content);
}
function requiredString(value, label) {
  if (typeof value !== "string" || !value.trim()) throw new Error(`服务未返回有效的 ${label}。`);
  return value;
}
async function poll(query, state, interval) {
  const deadline = Date.now() + 6e5;
  while (Date.now() < deadline) {
    const value = await query();
    if (state(value) === "done") return value;
    await Tools.System.sleep(interval);
  }
  throw new Error("生成任务等待超过 10 分钟，请到服务端查看任务状态。");
}
async function directory(provider) {
  const root = getPluginConfigDir();
  for (const path of [root, `${root}/generations`, `${root}/generations/${provider}`]) {
    const exists = await Tools.Files.exists(path);
    if (!exists.exists) {
      const created = await Tools.Files.mkdir(path);
      if (!created.successful) throw new Error(`无法创建生成目录：${created.details}`);
    }
  }
  return `${root}/generations/${provider}`;
}
function fileName(custom, extension) {
  const prefix = custom === void 0 ? "generation" : custom.trim();
  if (!prefix || /[\\/:*?"<>|]/.test(prefix) || prefix === "." || prefix === "..") throw new Error("保存名称不能为空或包含目录与特殊字符。");
  return `${prefix.slice(0, 64)}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}.${extension}`;
}
async function saveBase64(provider, content, extension, custom) {
  requiredString(content, "图片内容");
  const path = `${await directory(provider)}/${fileName(custom, extension)}`;
  const written = await Tools.Files.writeBinary(path, content);
  if (!written.successful) throw new Error(`保存图片失败：${written.details}`);
  return { file_path: path };
}
async function saveRemote(provider, url, extension, custom) {
  requiredString(url, "文件 URL");
  if (!/^https?:\/\//.test(url)) throw new Error("生成服务返回了无效的文件 URL。");
  const path = `${await directory(provider)}/${fileName(custom, extension)}`;
  const downloaded = await Tools.Files.download(url, path);
  if (!downloaded.successful) throw new Error(`下载生成文件失败：${downloaded.details}`);
  return { file_path: path };
}
function imageMime(path) {
  const match = /\.([a-z0-9]+)(?:[?#].*)?$/i.exec(path);
  const types = { png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", webp: "image/webp", gif: "image/gif", heic: "image/heic", heif: "image/heif" };
  const type = match && types[match[1].toLowerCase()];
  if (!type) throw new Error("参考图片需要带有受支持的图片扩展名。");
  return type;
}
async function referenceUrls(images = []) {
  const urls = [];
  for (const image of images) {
    if (/^https?:\/\//.test(image)) {
      urls.push(image);
      continue;
    }
    const response = await Tools.Net.uploadFile({ url: "https://beeimg.com/api/upload/file/json/", method: "POST", form_data: { apikey: apiKey("BEEIMG_API_KEY") }, files: [{ field_name: "file", file_path: image, content_type: imageMime(image) }] });
    if (response.statusCode < 200 || response.statusCode >= 300) throw new Error(`参考图上传失败：HTTP ${response.statusCode}`);
    const value = JSON.parse(response.content);
    if (value.files?.status !== "Success") throw new Error("Beeimg 未确认参考图上传成功。");
    urls.push(requiredString(value.files.url, "参考图 URL"));
  }
  return urls;
}

// src/providers/openai_draw.ts
async function generateImage(input) {
  const response = await request(endpoint("OPENAI_API_BASE_URL", "https://api.openai.com", "/v1", "images/generations"), apiKey("OPENAI_API_KEY"), { model: input.model, prompt: input.prompt, response_format: "b64_json", ...input.size ? { size: input.size } : {} });
  const item = response.data?.[0];
  if (!item || !!item.b64_json === !!item.url) throw new Error("OpenAI 响应需要包含一种明确的图片编码：b64_json 或 url。");
  const file = item.b64_json ? await saveBase64("openai", item.b64_json, "png", input.file_name) : await saveRemote("openai", item.url, "png", input.file_name);
  return { prompt: input.prompt, model: input.model, files: [file] };
}

// src/providers/qwen_draw.ts
async function generateImage2(input) {
  const url = endpoint("DASHSCOPE_API_BASE_URL", "https://dashscope.aliyuncs.com", "/api/v1", "services/aigc/text2image/image-synthesis");
  const key = apiKey("DASHSCOPE_API_KEY");
  const parameters = { n: input.count === void 0 ? 1 : input.count, ...input.size ? { size: input.size.replace("x", "*") } : {}, ...input.negative_prompt ? { negative_prompt: input.negative_prompt } : {}, ...input.optimize_prompt === void 0 ? {} : { prompt_extend: input.optimize_prompt }, ...input.watermark === void 0 ? {} : { watermark: input.watermark } };
  const created = await request(url, key, { model: input.model, input: { prompt: input.prompt }, parameters }, { "X-DashScope-Async": "enable" });
  const id = requiredString(created.output?.task_id, "task_id");
  const taskUrl = url.slice(0, -"services/aigc/text2image/image-synthesis".length) + `tasks/${encodeURIComponent(id)}`;
  const completed = await poll(() => request(taskUrl, key), (value) => {
    switch (value.output?.task_status) {
      case "SUCCEEDED":
        return "done";
      case "PENDING":
      case "RUNNING":
        return "pending";
      default:
        throw new Error(`通义千问生成任务失败：${JSON.stringify(value)}`);
    }
  }, 2e3);
  const results = completed.output?.results;
  if (!Array.isArray(results) || results.length === 0) throw new Error("通义千问未返回生成图片。");
  const files = [];
  for (const image of results) files.push(await saveRemote("qwen", requiredString(image.url, "图片 URL"), "png", input.file_name));
  return { prompt: input.prompt, model: input.model, files };
}

// src/providers/zhipu_draw.ts
async function generateImage3(input) {
  const response = await request("https://open.bigmodel.cn/api/paas/v4/images/generations", apiKey("ZHIPU_API_KEY"), { model: input.model, prompt: input.prompt, size: input.size === void 0 ? "1024x1024" : input.size });
  const file = await saveRemote("zhipu", requiredString(response.data?.[0]?.url, "图片 URL"), "png", input.file_name);
  return { prompt: input.prompt, model: input.model, files: [file] };
}

// src/providers/xai_draw.ts
async function generateImage4(input) {
  const response = await request("https://api.x.ai/v1/images/generations", apiKey("XAI_API_KEY"), { model: input.model, prompt: input.prompt, ...input.size ? { size: input.size } : {} });
  const file = await saveRemote("xai", requiredString(response.data?.[0]?.url, "图片 URL"), "jpeg", input.file_name);
  return { prompt: input.prompt, model: input.model, files: [file] };
}
async function videoImage(uri) {
  if (/^https?:\/\//.test(uri)) return uri;
  const content = await Tools.Files.readBinary(uri);
  return `data:${imageMime(uri)};base64,${requiredString(content.contentBase64, "参考图片内容")}`;
}
async function generateVideo(input) {
  if (input.reference_video && [input.aspect_ratio, input.resolution, input.duration].some((value) => value !== void 0)) throw new Error("视频编辑模式不支持设置比例、分辨率或时长。");
  if (input.aspect_ratio && !["1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3"].includes(input.aspect_ratio)) throw new Error("xAI 不支持此视频比例。");
  if (input.resolution && !["480p", "720p"].includes(input.resolution)) throw new Error("视频分辨率应为 480p 或 720p。");
  if (input.duration !== void 0 && (!Number.isInteger(input.duration) || input.duration < 1 || input.duration > 15)) throw new Error("视频时长应为 1–15 秒的整数。");
  const key = apiKey("XAI_API_KEY");
  const body = { model: input.model, prompt: input.prompt, ...input.reference_image ? { image: { url: await videoImage(input.reference_image) } } : {}, ...input.reference_video ? { video_url: input.reference_video } : { aspect_ratio: input.aspect_ratio === void 0 ? "16:9" : input.aspect_ratio, resolution: input.resolution === void 0 ? "480p" : input.resolution, duration: input.duration === void 0 ? 5 : input.duration } };
  const created = await request("https://api.x.ai/v1/videos/generations", key, body);
  const id = requiredString(created.request_id, "request_id");
  const completed = await poll(() => request(`https://api.x.ai/v1/videos/${encodeURIComponent(id)}`, key), (value) => {
    switch (value.status) {
      case "done":
        return "done";
      case "pending":
      case "processing":
        return "pending";
      default:
        throw new Error(`xAI 视频任务失败：${JSON.stringify(value)}`);
    }
  }, 5e3);
  const file = await saveRemote("xai", requiredString(completed.video?.url, "视频 URL"), "mp4", input.file_name);
  return { prompt: input.prompt, model: input.model, files: [file] };
}

// src/providers/minimax_draw.ts
async function generateImage5(input) {
  if (input.size && input.aspect_ratio) throw new Error("MiniMax 的像素尺寸与画面比例只能填写一种。");
  const dimensions = {};
  if (input.size) {
    const [width, height] = input.size.split("x").map(Number);
    if (input.model !== "image-01" || [width, height].some((value) => value < 512 || value > 2048 || value % 8 !== 0)) throw new Error("MiniMax 像素尺寸仅支持 image-01，宽高需为 512–2048 之间的 8 的倍数。");
    Object.assign(dimensions, { width, height });
  }
  if (input.aspect_ratio && !["1:1", "16:9", "4:3", "3:2", "2:3", "3:4", "9:16", ...input.model === "image-01" ? ["21:9"] : []].includes(input.aspect_ratio)) throw new Error("MiniMax 不支持此画面比例。");
  const urls = await referenceUrls(input.reference_images);
  const response = await request(endpoint("MINIMAX_API_BASE_URL", "https://api.minimaxi.com", "/v1", "image_generation"), apiKey("MINIMAX_API_KEY"), {
    model: input.model,
    prompt: input.prompt,
    n: input.count === void 0 ? 1 : input.count,
    response_format: "base64",
    ...dimensions,
    ...input.aspect_ratio ? { aspect_ratio: input.aspect_ratio } : {},
    ...input.seed === void 0 ? {} : { seed: input.seed },
    ...input.optimize_prompt === void 0 ? {} : { prompt_optimizer: input.optimize_prompt },
    ...input.watermark === void 0 ? {} : { aigc_watermark: input.watermark },
    ...urls.length ? { subject_reference: urls.map((url) => ({ type: "character", image_file: url })) } : {}
  });
  if (response.base_resp && response.base_resp.status_code !== 0) throw new Error(`MiniMax：${response.base_resp.status_msg}`);
  const images = response.data?.image_base64;
  if (!Array.isArray(images) || images.length === 0) throw new Error("MiniMax 未返回 image_base64。");
  const files = [];
  for (const image of images) files.push(await saveBase64("minimax", image, "jpeg", input.file_name));
  return { prompt: input.prompt, model: input.model, files };
}

// src/providers/nanobanana_draw.ts
async function generateImage6(input) {
  if (input.resolution && (input.model !== "nano-banana-pro" || !["1K", "2K", "4K"].includes(input.resolution))) throw new Error("1K、2K、4K 分辨率仅适用于 nano-banana-pro。");
  const urls = await referenceUrls(input.reference_images);
  const key = apiKey("NANOBANANA_API_KEY");
  const url = endpoint("NANOBANANA_API_BASE_URL", "https://grsai.dakka.com.cn", "/v1", "draw/nano-banana");
  const created = await request(url, key, { model: input.model, prompt: input.prompt, webHook: "-1", shutProgress: false, ...input.aspect_ratio ? { aspectRatio: input.aspect_ratio } : {}, ...input.resolution ? { imageSize: input.resolution } : {}, ...urls.length ? { urls } : {} });
  if (created.code !== 0) throw new Error(`Nano Banana：${created.msg}`);
  const id = requiredString(created.data?.id, "任务 ID");
  const resultUrl = url.slice(0, -"nano-banana".length) + "result";
  const completed = await poll(() => request(resultUrl, key, { id }), (value) => {
    if (value.code === -22) return "pending";
    if (value.code !== 0) throw new Error(`Nano Banana：${value.msg}`);
    switch (value.data?.status) {
      case "succeeded":
        return "done";
      case "running":
      case "queued":
        return "pending";
      default:
        throw new Error(`Nano Banana 任务失败：${JSON.stringify(value.data)}`);
    }
  }, 5e3);
  const results = completed.data?.results;
  if (!Array.isArray(results) || results.length === 0) throw new Error("Nano Banana 未返回生成图片。");
  const files = [];
  for (const image of results) files.push(await saveRemote("nanobanana", requiredString(image.url, "图片 URL"), "png", input.file_name));
  return { prompt: input.prompt, model: input.model, files };
}

// src/service.ts
var ADAPTERS = { openai: generateImage, qwen: generateImage2, zhipu: generateImage3, xai: generateImage4, minimax: generateImage5, nanobanana: generateImage6 };
function listMethods() {
  return PROVIDERS.map((provider) => ({ ...provider, configured: !!getEnv(provider.apiKey)?.trim(), video: provider.id === "xai" }));
}
function requireCredentials(provider, references = []) {
  if (!getEnv(provider.apiKey)?.trim()) throw new Error(`请在生图工作台中配置 ${provider.title} 的 API Key。`);
  if (references.some((uri) => !/^https?:\/\//.test(uri)) && !getEnv("BEEIMG_API_KEY")?.trim()) throw new Error("使用本地参考图需要先配置 Beeimg API Key，参考图将上传到 Beeimg。");
}
function result(provider, kind, raw) {
  const paths = raw.files.map((file) => file.file_path);
  if (paths.length === 0 || paths.some((path) => typeof path !== "string" || !path)) throw new Error("生成服务未返回有效的文件路径。");
  const assets = paths.map((uri) => ({ uri, markdown: kind === "image" ? `![生成的图片](${uri})` : `<video controls src="${uri}"></video>` }));
  return { provider: provider.id, kind, prompt: raw.prompt, model: raw.model, assets, markdown: assets.map((asset) => asset.markdown).join("\n\n"), created_at: Date.now() };
}
async function generateImage7(input) {
  const provider = validateImageRequest(input);
  requireCredentials(provider, input.reference_images);
  const request2 = { ...input, prompt: input.prompt.trim(), model: input.model === void 0 ? provider.model : input.model.trim() };
  return result(provider, "image", await ADAPTERS[provider.id](request2));
}
async function generateVideo2(input) {
  const provider = validateVideoRequest(input);
  requireCredentials(provider);
  return result(provider, "video", await generateVideo({ ...input, prompt: input.prompt.trim(), model: input.model === void 0 ? "grok-imagine-video" : input.model.trim() }));
}

// src/tools.ts
function list_methods() {
  return { success: true, message: "可用生图方式", data: listMethods() };
}
async function generate_image(params) {
  return { success: true, message: "图片已生成并保存。", data: await generateImage7(params) };
}
async function generate_video(params) {
  return { success: true, message: "视频已生成并保存。", data: await generateVideo2(params) };
}
