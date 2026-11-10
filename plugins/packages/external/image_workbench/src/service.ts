import { PROVIDERS, validateImageRequest, validateVideoRequest } from './catalog';
import type { GeneratedAsset, GenerationResult, ImageRequest, Provider, VideoRequest } from './model';
import { generateImage as openai } from './providers/openai_draw';
import { generateImage as qwen } from './providers/qwen_draw';
import { generateImage as zhipu } from './providers/zhipu_draw';
import { generateImage as xai, generateVideo as xaiVideo } from './providers/xai_draw';
import { generateImage as minimax } from './providers/minimax_draw';
import { generateImage as nanobanana } from './providers/nanobanana_draw';
import type { ProviderResult, ResolvedImageRequest } from './providers/shared';

const ADAPTERS: Record<ImageRequest['provider'], (input: ResolvedImageRequest) => Promise<ProviderResult>> = { openai, qwen, zhipu, xai, minimax, nanobanana };

/** Reports available generation methods and credential readiness without disclosing secrets. */
export function listMethods() {
  return PROVIDERS.map(provider => ({ ...provider, configured: !!getEnv(provider.apiKey)?.trim(), video: provider.id === 'xai' }));
}

/** Requires only the selected provider's credential before starting network or file operations. */
function requireCredentials(provider: Provider, references: string[] = []): void {
  if (!getEnv(provider.apiKey)?.trim()) throw new Error(`请在生图工作台中配置 ${provider.title} 的 API Key。`);
  if (references.some(uri => !/^https?:\/\//.test(uri)) && !getEnv('BEEIMG_API_KEY')?.trim()) throw new Error('使用本地参考图需要先配置 Beeimg API Key，参考图将上传到 Beeimg。');
}

/** Builds one stable asset contract for workbench previews and assistant responses. */
function result(provider: Provider, kind: 'image' | 'video', raw: ProviderResult): GenerationResult {
  const paths = raw.files.map(file => file.file_path);
  if (paths.length === 0 || paths.some(path => typeof path !== 'string' || !path)) throw new Error('生成服务未返回有效的文件路径。');
  const assets: GeneratedAsset[] = paths.map(uri => ({ uri, markdown: kind === 'image' ? `![生成的图片](${uri})` : `<video controls src="${uri}"></video>` }));
  return { provider: provider.id, kind, prompt: raw.prompt, model: raw.model, assets, markdown: assets.map(asset => asset.markdown).join('\n\n'), created_at: Date.now() };
}

/** Translates the shared image request into the selected provider's transport exactly once. */
export async function generateImage(input: ImageRequest): Promise<GenerationResult> {
  const provider = validateImageRequest(input);
  requireCredentials(provider, input.reference_images);
  const request = { ...input, prompt: input.prompt.trim(), model: input.model === undefined ? provider.model : input.model.trim() };
  return result(provider, 'image', await ADAPTERS[provider.id](request));
}

/** Preserves xAI video generation behind a shared provider, prompt, reference and asset contract. */
export async function generateVideo(input: VideoRequest): Promise<GenerationResult> {
  const provider = validateVideoRequest(input);
  requireCredentials(provider);
  return result(provider, 'video', await xaiVideo({ ...input, prompt: input.prompt.trim(), model: input.model === undefined ? 'grok-imagine-video' : input.model.trim() }));
}
