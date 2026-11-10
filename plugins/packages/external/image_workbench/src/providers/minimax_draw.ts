import { apiKey, endpoint, referenceUrls, request, saveBase64 } from './shared';
import type { ProviderResult, ResolvedImageRequest, SavedFile } from './shared';

/** Converts shared size, count, reference and optimization options into the MiniMax schema. */
export async function generateImage(input: ResolvedImageRequest): Promise<ProviderResult> {
  if (input.size && input.aspect_ratio) throw new Error('MiniMax 的像素尺寸与画面比例只能填写一种。');
  const dimensions: { width?: number; height?: number } = {};
  if (input.size) {
    const [width, height] = input.size.split('x').map(Number);
    if (input.model !== 'image-01' || [width, height].some(value => value < 512 || value > 2048 || value % 8 !== 0)) throw new Error('MiniMax 像素尺寸仅支持 image-01，宽高需为 512–2048 之间的 8 的倍数。');
    Object.assign(dimensions, { width, height });
  }
  if (input.aspect_ratio && !['1:1', '16:9', '4:3', '3:2', '2:3', '3:4', '9:16', ...(input.model === 'image-01' ? ['21:9'] : [])].includes(input.aspect_ratio)) throw new Error('MiniMax 不支持此画面比例。');
  const urls = await referenceUrls(input.reference_images);
  const response = await request<{ base_resp?: { status_code: number; status_msg: string }; data?: { image_base64?: string[] } }>(endpoint('MINIMAX_API_BASE_URL', 'https://api.minimaxi.com', '/v1', 'image_generation'), apiKey('MINIMAX_API_KEY'), { model: input.model, prompt: input.prompt, n: input.count === undefined ? 1 : input.count, response_format: 'base64', ...dimensions,
    ...(input.aspect_ratio ? { aspect_ratio: input.aspect_ratio } : {}), ...(input.seed === undefined ? {} : { seed: input.seed }), ...(input.optimize_prompt === undefined ? {} : { prompt_optimizer: input.optimize_prompt }), ...(input.watermark === undefined ? {} : { aigc_watermark: input.watermark }), ...(urls.length ? { subject_reference: urls.map(url => ({ type: 'character', image_file: url })) } : {}) });
  if (response.base_resp && response.base_resp.status_code !== 0) throw new Error(`MiniMax：${response.base_resp.status_msg}`);
  const images = response.data?.image_base64;
  if (!Array.isArray(images) || images.length === 0) throw new Error('MiniMax 未返回 image_base64。');
  const files: SavedFile[] = [];
  for (const image of images) files.push(await saveBase64('minimax', image, 'jpeg', input.file_name));
  return { prompt: input.prompt, model: input.model, files };
}
