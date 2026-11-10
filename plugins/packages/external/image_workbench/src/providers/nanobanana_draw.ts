import { apiKey, endpoint, poll, referenceUrls, request, requiredString, saveRemote } from './shared';
import type { ProviderResult, ResolvedImageRequest, SavedFile } from './shared';

interface Task { code: number; msg?: string; data?: { id?: string; status?: string; failure_reason?: string; results?: { url?: string }[] } }

/** Submits the canonical Grsai asynchronous task schema and saves its result images. */
export async function generateImage(input: ResolvedImageRequest): Promise<ProviderResult> {
  if (input.resolution && (input.model !== 'nano-banana-pro' || !['1K', '2K', '4K'].includes(input.resolution))) throw new Error('1K、2K、4K 分辨率仅适用于 nano-banana-pro。');
  const urls = await referenceUrls(input.reference_images);
  const key = apiKey('NANOBANANA_API_KEY');
  const url = endpoint('NANOBANANA_API_BASE_URL', 'https://grsai.dakka.com.cn', '/v1', 'draw/nano-banana');
  const created = await request<Task>(url, key, { model: input.model, prompt: input.prompt, webHook: '-1', shutProgress: false, ...(input.aspect_ratio ? { aspectRatio: input.aspect_ratio } : {}), ...(input.resolution ? { imageSize: input.resolution } : {}), ...(urls.length ? { urls } : {}) });
  if (created.code !== 0) throw new Error(`Nano Banana：${created.msg}`);
  const id = requiredString(created.data?.id, '任务 ID');
  const resultUrl = url.slice(0, -'nano-banana'.length) + 'result';
  const completed = await poll(() => request<Task>(resultUrl, key, { id }), value => {
    if (value.code === -22) return 'pending';
    if (value.code !== 0) throw new Error(`Nano Banana：${value.msg}`);
    switch (value.data?.status) {
      case 'succeeded': return 'done';
      case 'running': case 'queued': return 'pending';
      default: throw new Error(`Nano Banana 任务失败：${JSON.stringify(value.data)}`);
    }
  }, 5000);
  const results = completed.data?.results;
  if (!Array.isArray(results) || results.length === 0) throw new Error('Nano Banana 未返回生成图片。');
  const files: SavedFile[] = [];
  for (const image of results) files.push(await saveRemote('nanobanana', requiredString(image.url, '图片 URL'), 'png', input.file_name));
  return { prompt: input.prompt, model: input.model, files };
}
