import { apiKey, request, requiredString, saveRemote } from './shared';
import type { ProviderResult, ResolvedImageRequest } from './shared';

/** Generates a GLM image through the existing Zhipu endpoint. */
export async function generateImage(input: ResolvedImageRequest): Promise<ProviderResult> {
  const response = await request<{ data?: { url?: string }[] }>('https://open.bigmodel.cn/api/paas/v4/images/generations', apiKey('ZHIPU_API_KEY'), { model: input.model, prompt: input.prompt, size: input.size === undefined ? '1024x1024' : input.size });
  const file = await saveRemote('zhipu', requiredString(response.data?.[0]?.url, '图片 URL'), 'png', input.file_name);
  return { prompt: input.prompt, model: input.model, files: [file] };
}
