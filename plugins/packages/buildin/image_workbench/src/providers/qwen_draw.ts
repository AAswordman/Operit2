import { apiKey, endpoint, poll, request, requiredString, saveRemote } from './shared';
import type { ProviderResult, ResolvedImageRequest, SavedFile } from './shared';

interface Task { output?: { task_id?: string; task_status?: string; results?: { url?: string }[] }; message?: string }

/** Creates one DashScope job, polls its declared status and saves every returned image. */
export async function generateImage(input: ResolvedImageRequest): Promise<ProviderResult> {
  const url = endpoint('DASHSCOPE_API_BASE_URL', 'https://dashscope.aliyuncs.com', '/api/v1', 'services/aigc/text2image/image-synthesis');
  const key = apiKey('DASHSCOPE_API_KEY');
  const parameters = { n: input.count === undefined ? 1 : input.count, ...(input.size ? { size: input.size.replace('x', '*') } : {}), ...(input.negative_prompt ? { negative_prompt: input.negative_prompt } : {}), ...(input.optimize_prompt === undefined ? {} : { prompt_extend: input.optimize_prompt }), ...(input.watermark === undefined ? {} : { watermark: input.watermark }) };
  const created = await request<Task>(url, key, { model: input.model, input: { prompt: input.prompt }, parameters }, { 'X-DashScope-Async': 'enable' });
  const id = requiredString(created.output?.task_id, 'task_id');
  const taskUrl = url.slice(0, -'services/aigc/text2image/image-synthesis'.length) + `tasks/${encodeURIComponent(id)}`;
  const completed = await poll(() => request<Task>(taskUrl, key), value => {
    switch (value.output?.task_status) {
      case 'SUCCEEDED': return 'done';
      case 'PENDING': case 'RUNNING': return 'pending';
      default: throw new Error(`通义千问生成任务失败：${JSON.stringify(value)}`);
    }
  }, 2000);
  const results = completed.output?.results;
  if (!Array.isArray(results) || results.length === 0) throw new Error('通义千问未返回生成图片。');
  const files: SavedFile[] = [];
  for (const image of results) files.push(await saveRemote('qwen', requiredString(image.url, '图片 URL'), 'png', input.file_name));
  return { prompt: input.prompt, model: input.model, files };
}
