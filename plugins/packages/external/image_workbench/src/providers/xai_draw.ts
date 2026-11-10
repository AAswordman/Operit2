import { apiKey, imageMime, poll, request, requiredString, saveRemote } from './shared';
import type { ProviderResult, ResolvedImageRequest } from './shared';
import type { VideoRequest } from '../model';

/** Generates one xAI image with the shared input and output contract. */
export async function generateImage(input: ResolvedImageRequest): Promise<ProviderResult> {
  const response = await request<{ data?: { url?: string }[] }>('https://api.x.ai/v1/images/generations', apiKey('XAI_API_KEY'), { model: input.model, prompt: input.prompt, ...(input.size ? { size: input.size } : {}) });
  const file = await saveRemote('xai', requiredString(response.data?.[0]?.url, '图片 URL'), 'jpeg', input.file_name);
  return { prompt: input.prompt, model: input.model, files: [file] };
}

/** Encodes a selected local image for xAI without uploading it to a separate image host. */
async function videoImage(uri: string): Promise<string> {
  if (/^https?:\/\//.test(uri)) return uri;
  const content = await Tools.Files.readBinary(uri);
  return `data:${imageMime(uri)};base64,${requiredString(content.contentBase64, '参考图片内容')}`;
}

/** Preserves text-to-video, image-to-video and video editing through one xAI job transport. */
export async function generateVideo(input: VideoRequest & { model: string }): Promise<ProviderResult> {
  if (input.reference_video && [input.aspect_ratio, input.resolution, input.duration].some(value => value !== undefined)) throw new Error('视频编辑模式不支持设置比例、分辨率或时长。');
  if (input.aspect_ratio && !['1:1', '16:9', '9:16', '4:3', '3:4', '3:2', '2:3'].includes(input.aspect_ratio)) throw new Error('xAI 不支持此视频比例。');
  if (input.resolution && !['480p', '720p'].includes(input.resolution)) throw new Error('视频分辨率应为 480p 或 720p。');
  if (input.duration !== undefined && (!Number.isInteger(input.duration) || input.duration < 1 || input.duration > 15)) throw new Error('视频时长应为 1–15 秒的整数。');
  const key = apiKey('XAI_API_KEY');
  const body = { model: input.model, prompt: input.prompt, ...(input.reference_image ? { image: { url: await videoImage(input.reference_image) } } : {}), ...(input.reference_video ? { video_url: input.reference_video } : { aspect_ratio: input.aspect_ratio === undefined ? '16:9' : input.aspect_ratio, resolution: input.resolution === undefined ? '480p' : input.resolution, duration: input.duration === undefined ? 5 : input.duration }) };
  const created = await request<{ request_id?: string }>('https://api.x.ai/v1/videos/generations', key, body);
  const id = requiredString(created.request_id, 'request_id');
  const completed = await poll(() => request<{ status: string; video?: { url?: string } }>(`https://api.x.ai/v1/videos/${encodeURIComponent(id)}`, key), value => {
    switch (value.status) {
      case 'done': return 'done';
      case 'pending': case 'processing': return 'pending';
      default: throw new Error(`xAI 视频任务失败：${JSON.stringify(value)}`);
    }
  }, 5000);
  const file = await saveRemote('xai', requiredString(completed.video?.url, '视频 URL'), 'mp4', input.file_name);
  return { prompt: input.prompt, model: input.model, files: [file] };
}
