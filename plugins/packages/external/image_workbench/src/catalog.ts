import type { Provider, ProviderId, ImageRequest, VideoRequest } from './model';

export const PACKAGE_NAME = 'image_workbench';
export const PROVIDERS: Provider[] = [
  { id: 'openai', title: 'OpenAI', description: 'OpenAI 与兼容图像接口', model: 'gpt-image-1', apiKey: 'OPENAI_API_KEY', endpointKey: 'OPENAI_API_BASE_URL', endpoint: 'https://api.openai.com', options: ['size'] },
  { id: 'qwen', title: '通义千问', description: '百炼 / 通义万相，支持负面提示词', model: 'qwen-image-plus', apiKey: 'DASHSCOPE_API_KEY', endpointKey: 'DASHSCOPE_API_BASE_URL', endpoint: 'https://dashscope.aliyuncs.com', options: ['size', 'count', 'negative_prompt', 'optimize_prompt', 'watermark'] },
  { id: 'zhipu', title: '智谱', description: 'GLM Image 图像生成', model: 'glm-image', apiKey: 'ZHIPU_API_KEY', options: ['size'] },
  { id: 'xai', title: 'xAI / Grok', description: '图像生成，也可切换到视频', model: 'grok-2-image-1212', apiKey: 'XAI_API_KEY', options: ['size'] },
  { id: 'minimax', title: 'MiniMax', description: '文生图、参考图与批量生成', model: 'image-01', apiKey: 'MINIMAX_API_KEY', endpointKey: 'MINIMAX_API_BASE_URL', endpoint: 'https://api.minimaxi.com', options: ['size', 'aspect_ratio', 'count', 'optimize_prompt', 'watermark', 'seed', 'reference_images'] },
  { id: 'nanobanana', title: 'Nano Banana', description: 'Grsai 接口，支持参考图与高分辨率', model: 'nano-banana-pro', apiKey: 'NANOBANANA_API_KEY', endpointKey: 'NANOBANANA_API_BASE_URL', endpoint: 'https://grsai.dakka.com.cn', options: ['aspect_ratio', 'resolution', 'reference_images'] },
];

/** Resolves a provider by its exact public identifier. */
export function providerById(id: string): Provider {
  const provider = PROVIDERS.find(item => item.id === id);
  if (!provider) throw new Error(`未知生图方式：${id}`);
  return provider;
}

/** Rejects malformed shared strings and invalid output names before any paid request. */
function validateCommon(input: { prompt: string; model?: string; file_name?: string }): void {
  if (typeof input.prompt !== 'string' || !input.prompt.trim()) throw new Error('请填写提示词。');
  if (input.model !== undefined && (typeof input.model !== 'string' || !input.model.trim())) throw new Error('模型名称不能为空。');
  if (input.file_name !== undefined && (typeof input.file_name !== 'string' || !input.file_name.trim() || /[\\/:*?"<>|]/.test(input.file_name) || ['.', '..'].includes(input.file_name.trim()))) throw new Error('保存名称不能为空或包含目录与特殊字符。');
}

/** Validates shared input before any provider can issue a request or upload a file. */
export function validateImageRequest(input: ImageRequest): Provider {
  const provider = providerById(input.provider);
  validateCommon(input);
  const allowed = new Set<string>(['provider', 'prompt', 'model', 'file_name', ...provider.options]);
  for (const [key, value] of Object.entries(input)) {
    if (value !== undefined && !allowed.has(key)) throw new Error(`${provider.title} 不支持参数 ${key}。`);
  }
  for (const key of ['size', 'aspect_ratio', 'resolution', 'negative_prompt'] as const) {
    if (input[key] !== undefined && (typeof input[key] !== 'string' || !input[key]!.trim())) throw new Error(`${key} 需要填写非空文字。`);
  }
  if (input.count !== undefined && (!Number.isInteger(input.count) || input.count < 1 || input.count > (provider.id === 'minimax' ? 9 : 4))) throw new Error('图片数量超出所选方式允许的范围。');
  if (input.seed !== undefined && !Number.isInteger(input.seed)) throw new Error('随机种子必须为整数。');
  for (const key of ['optimize_prompt', 'watermark'] as const) {
    if (input[key] !== undefined && typeof input[key] !== 'boolean') throw new Error(`${key} 必须是布尔值。`);
  }
  if (input.size !== undefined && !/^\d+x\d+$/.test(input.size)) throw new Error('尺寸格式应为 1024x1024。');
  if (input.reference_images !== undefined && (!Array.isArray(input.reference_images) || input.reference_images.some(uri => typeof uri !== 'string' || !uri.trim()))) throw new Error('参考图必须是非空 URL 或 VFS 路径的数组。');
  return provider;
}

/** Applies the same validation boundary to workbench and assistant video requests. */
export function validateVideoRequest(input: VideoRequest): Provider {
  if (input.provider !== 'xai') throw new Error('当前只有 xAI 支持视频生成。');
  validateCommon(input);
  const allowed = new Set(['provider', 'prompt', 'model', 'file_name', 'aspect_ratio', 'resolution', 'duration', 'reference_image', 'reference_video']);
  for (const [key, value] of Object.entries(input)) if (value !== undefined && !allowed.has(key)) throw new Error(`视频生成不支持参数 ${key}。`);
  for (const key of ['aspect_ratio', 'resolution', 'reference_image', 'reference_video'] as const) if (input[key] !== undefined && (typeof input[key] !== 'string' || !input[key]!.trim())) throw new Error(`${key} 需要填写非空文字。`);
  if (input.reference_image && input.reference_video) throw new Error('参考图片和参考视频只能选择一种。');
  if (input.reference_video && !/^https?:\/\//.test(input.reference_video)) throw new Error('参考视频需要填写公网 URL。');
  return providerById(input.provider);
}

/** Converts visible form values to the same typed request accepted by AI tool calls. */
export function imageRequestFromFields(provider: ProviderId, prompt: string, fields: Record<string, string>): ImageRequest {
  const request: Record<string, unknown> = { provider, prompt };
  for (const [key, value] of Object.entries(fields)) {
    if (!value.trim()) continue;
    if (key === 'count' || key === 'seed') request[key] = Number(value);
    else if (key === 'optimize_prompt' || key === 'watermark') {
      if (value !== 'true' && value !== 'false') throw new Error(`${key} 请选择开启或关闭。`);
      request[key] = value === 'true';
    } else if (key === 'reference_images') request[key] = value.split('\n').map(uri => uri.trim()).filter(Boolean);
    else request[key] = value.trim();
  }
  const typed = request as unknown as ImageRequest;
  validateImageRequest(typed);
  return typed;
}
