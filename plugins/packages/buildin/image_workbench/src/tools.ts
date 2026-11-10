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

import { generateImage, generateVideo, listMethods } from './service';
import type { ImageRequest, VideoRequest } from './model';

/** Lists real provider capabilities and credential readiness without submitting generation requests. */
export function list_methods() {
  return { success: true, message: '可用生图方式', data: listMethods() };
}

/** Generates images through the shared service used by the workbench. */
export async function generate_image(params: ImageRequest) {
  return { success: true, message: '图片已生成并保存。', data: await generateImage(params) };
}

/** Retains xAI video generation through the same provider and asset vocabulary. */
export async function generate_video(params: VideoRequest) {
  return { success: true, message: '视频已生成并保存。', data: await generateVideo(params) };
}
