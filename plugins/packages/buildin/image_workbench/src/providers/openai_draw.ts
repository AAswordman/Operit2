import { apiKey, endpoint, request, saveBase64, saveRemote } from './shared';
import type { ProviderResult, ResolvedImageRequest } from './shared';

/** Generates through the OpenAI image schema and saves the response's explicit encoding. */
export async function generateImage(input: ResolvedImageRequest): Promise<ProviderResult> {
  const response = await request<{ data?: { b64_json?: string; url?: string }[] }>(endpoint('OPENAI_API_BASE_URL', 'https://api.openai.com', '/v1', 'images/generations'), apiKey('OPENAI_API_KEY'), { model: input.model, prompt: input.prompt, response_format: 'b64_json', ...(input.size ? { size: input.size } : {}) });
  const item = response.data?.[0];
  if (!item || (!!item.b64_json === !!item.url)) throw new Error('OpenAI 响应需要包含一种明确的图片编码：b64_json 或 url。');
  const file = item.b64_json ? await saveBase64('openai', item.b64_json, 'png', input.file_name) : await saveRemote('openai', item.url!, 'png', input.file_name);
  return { prompt: input.prompt, model: input.model, files: [file] };
}
