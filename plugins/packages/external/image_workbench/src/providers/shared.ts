import type { ImageRequest, ProviderId } from '../model';

export type ResolvedImageRequest = ImageRequest & { model: string };
export interface SavedFile { file_path: string }
export interface ProviderResult { prompt: string; model: string; files: SavedFile[] }

/** Reads the selected service's credential without exposing it in results or diagnostics. */
export function apiKey(name: string): string {
  const value = getEnv(name)?.trim();
  if (!value) throw new Error(`请配置 ${name}。`);
  return value;
}

/** Resolves a documented API base or exact endpoint into one canonical endpoint. */
export function endpoint(key: string, defaultBase: string, prefix: string, resource: string): string {
  const configured = getEnv(key)?.trim();
  const base = (configured ? configured : defaultBase).replace(/\/+$/, '');
  if (!/^https?:\/\/[^\s?#]+$/.test(base)) throw new Error(`接口地址无效：${key}`);
  const suffix = `${prefix}/${resource}`;
  if (base.endsWith(suffix)) return base;
  return base.endsWith(prefix) ? `${base}/${resource}` : `${base}${suffix}`;
}

/** Sends one JSON request through the platform-neutral Host HTTP client. */
export async function request<T>(url: string, key: string, body?: unknown, headers: Record<string, string> = {}): Promise<T> {
  const client = OkHttp.newBuilder().connectTimeout(600000).readTimeout(600000).writeTimeout(600000).build();
  const builder = client.newRequest().url(url).method(body === undefined ? 'GET' : 'POST').headers({ accept: 'application/json', 'content-type': 'application/json', Authorization: `Bearer ${key}`, ...headers });
  if (body !== undefined) builder.body(JSON.stringify(body), 'json');
  const response = await builder.build().execute();
  if (!response.isSuccessful()) throw new Error(`生成服务请求失败（HTTP ${response.statusCode}）：${response.content}`);
  return JSON.parse(response.content) as T;
}

/** Requires an explicit nonempty string in a service response. */
export function requiredString(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) throw new Error(`服务未返回有效的 ${label}。`);
  return value;
}

/** Polls only declared processing states and propagates transport, parsing and service failures. */
export async function poll<T>(query: () => Promise<T>, state: (value: T) => 'pending' | 'done', interval: number): Promise<T> {
  const deadline = Date.now() + 600000;
  while (Date.now() < deadline) {
    const value = await query();
    if (state(value) === 'done') return value;
    await Tools.System.sleep(interval);
  }
  throw new Error('生成任务等待超过 10 分钟，请到服务端查看任务状态。');
}

/** Creates a provider output directory through VFS and reports actual filesystem failures. */
async function directory(provider: ProviderId): Promise<string> {
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

/** Creates a readable output name without overwriting prior generations. */
function fileName(custom: string | undefined, extension: string): string {
  const prefix = custom === undefined ? 'generation' : custom.trim();
  if (!prefix || /[\\/:*?"<>|]/.test(prefix) || prefix === '.' || prefix === '..') throw new Error('保存名称不能为空或包含目录与特殊字符。');
  return `${prefix.slice(0, 64)}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}.${extension}`;
}

/** Saves one explicitly encoded image into the current plugin's VFS output tree. */
export async function saveBase64(provider: ProviderId, content: string, extension: 'png' | 'jpeg', custom?: string): Promise<SavedFile> {
  requiredString(content, '图片内容');
  const path = `${await directory(provider)}/${fileName(custom, extension)}`;
  const written = await Tools.Files.writeBinary(path, content);
  if (!written.successful) throw new Error(`保存图片失败：${written.details}`);
  return { file_path: path };
}

/** Downloads one remote asset directly into VFS through the existing Host file tool. */
export async function saveRemote(provider: ProviderId, url: string, extension: 'png' | 'jpeg' | 'mp4', custom?: string): Promise<SavedFile> {
  requiredString(url, '文件 URL');
  if (!/^https?:\/\//.test(url)) throw new Error('生成服务返回了无效的文件 URL。');
  const path = `${await directory(provider)}/${fileName(custom, extension)}`;
  const downloaded = await Tools.Files.download(url, path);
  if (!downloaded.successful) throw new Error(`下载生成文件失败：${downloaded.details}`);
  return { file_path: path };
}

/** Maps a selected image's extension to an explicit supported upload media type. */
export function imageMime(path: string): string {
  const match = /\.([a-z0-9]+)(?:[?#].*)?$/i.exec(path);
  const types: Record<string, string> = { png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', webp: 'image/webp', gif: 'image/gif', heic: 'image/heic', heif: 'image/heif' };
  const type = match && types[match[1].toLowerCase()];
  if (!type) throw new Error('参考图片需要带有受支持的图片扩展名。');
  return type;
}

/** Uploads explicitly selected local references and leaves public image URLs unchanged. */
export async function referenceUrls(images: string[] = []): Promise<string[]> {
  const urls: string[] = [];
  for (const image of images) {
    if (/^https?:\/\//.test(image)) { urls.push(image); continue; }
    const response = await Tools.Net.uploadFile({ url: 'https://beeimg.com/api/upload/file/json/', method: 'POST', form_data: { apikey: apiKey('BEEIMG_API_KEY') }, files: [{ field_name: 'file', file_path: image, content_type: imageMime(image) }] });
    if (response.statusCode < 200 || response.statusCode >= 300) throw new Error(`参考图上传失败：HTTP ${response.statusCode}`);
    const value = JSON.parse(response.content) as { files?: { status?: string; url?: string } };
    if (value.files?.status !== 'Success') throw new Error('Beeimg 未确认参考图上传成功。');
    urls.push(requiredString(value.files.url, '参考图 URL'));
  }
  return urls;
}
