import { WeChatApi, encodeAsciiBase64 } from "./api.js";
import {
  CDN_BASE,
  MAX_MEDIA_BYTES,
  integer,
  nonblank,
  object,
  text,
  wechatUrl,
  type JsonObject,
  type MediaKind,
  type MediaParams,
} from "./model.js";

/** Returns the number of decoded bytes represented by one base64 string. */
function byteLength(base64: string): number {
  const padding = base64.endsWith("==") ? 2 : base64.endsWith("=") ? 1 : 0;
  return Math.max(0, Math.floor((base64.length * 3) / 4) - padding);
}

/** Selects one explicit CDN media field from an incoming item. */
export function mediaReference(item: JsonObject): {
  kind: MediaKind;
  media: JsonObject;
  key?: string;
  name?: string;
  voiceFormat?: number;
} | null {
  if (item.type === 2) {
    const image = object(item.image_item, "image_item");
    return {
      kind: "image",
      media: object(image.media, "image_item.media"),
      key:
        image.aeskey === undefined
          ? undefined
          : text(image.aeskey, "image_item.aeskey"),
    };
  }
  if (item.type === 3) {
    const voice = object(item.voice_item, "voice_item");
    if (voice.media === undefined && typeof voice.text === "string")
      return null;
    return {
      kind: "voice",
      media: object(voice.media, "voice_item.media"),
      voiceFormat:
        voice.encode_type === undefined
          ? undefined
          : integer(voice.encode_type, "voice encode_type", 1, 8),
    };
  }
  if (item.type === 4) {
    const file = object(item.file_item, "file_item");
    return {
      kind: "file",
      media: object(file.media, "file_item.media"),
      name:
        file.file_name === undefined
          ? undefined
          : text(file.file_name, "file_item.file_name"),
    };
  }
  if (item.type === 5) {
    const video = object(item.video_item, "video_item");
    return { kind: "video", media: object(video.media, "video_item.media") };
  }
  return null;
}

/** Reads and bounds one local media source through the host file interface. */
async function readSource(
  api: WeChatApi,
  source: string,
): Promise<{ base64: string; size: number }> {
  nonblank(source, "source");
  let base64: string;
  if (/^https:\/\//i.test(source)) {
    const result = await api.http.request({
      url: source,
      method: "GET",
      responseType: "arraybuffer",
      connect_timeout: 15,
      read_timeout: 60,
      follow_redirects: false,
    });
    if (result.statusCode < 200 || result.statusCode >= 300)
      throw new Error(`媒体来源 HTTP ${result.statusCode}`);
    base64 = nonblank(result.contentBase64, "source binary data");
  } else {
    if (/^[a-z][a-z0-9+.-]*:\/\//i.test(source))
      throw new Error("媒体地址只接受 HTTPS 或宿主文件路径。");
    const info = await Tools.Files.info(source);
    integer(info.size, "source file size", 1, MAX_MEDIA_BYTES);
    const result = await Tools.Files.readBinary(source);
    base64 = nonblank(result.contentBase64, "source binary data");
  }
  const size = byteLength(base64);
  if (size === 0 || size > MAX_MEDIA_BYTES)
    throw new Error("微信媒体文件大小必须在 1 到 25 MiB 之间。");
  return { base64, size };
}

/** Uploads one encrypted media file through the official getuploadurl/CDN flow. */
export async function uploadMedia(
  api: WeChatApi,
  userId: string,
  kind: MediaKind,
  source: string,
  options: Partial<MediaParams> = {},
): Promise<JsonObject> {
  if (
    kind !== "image" &&
    kind !== "video" &&
    kind !== "file" &&
    kind !== "voice"
  )
    throw new Error("kind must be image, video, file or voice");
  if (kind === "voice") {
    integer(options.voice_format, "voice_format", 1, 8);
    integer(options.sample_rate, "sample_rate", 8000, 192000);
    integer(options.duration_ms, "duration_ms", 1, 600000);
  }
  const input = await readSource(api, source);
  const key = CryptoJS.binary.randomHex(16);
  const fileKey = `operit-${CryptoJS.binary.randomHex(16)}`;
  const mediaType = { image: 1, video: 2, file: 3, voice: 4 }[kind];
  const encrypted = CryptoJS.binary.encryptAesEcb(input.base64, key);
  const upload = await api.post("getuploadurl", {
    filekey: fileKey,
    media_type: mediaType,
    to_user_id: nonblank(userId, "user_id"),
    rawsize: input.size,
    rawfilemd5: CryptoJS.binary.md5(input.base64),
    filesize: byteLength(encrypted),
    no_need_thumb: true,
    aeskey: key,
  });
  const uploadParam =
    upload.upload_param === undefined
      ? ""
      : text(upload.upload_param, "upload_param");
  const uploadUrl =
    upload.upload_full_url === undefined
      ? `${CDN_BASE}/upload?encrypted_query_param=${encodeURIComponent(nonblank(uploadParam, "upload_param"))}&filekey=${encodeURIComponent(fileKey)}`
      : wechatUrl(upload.upload_full_url, "upload_full_url");
  const response = await api.http.upload(uploadUrl, encrypted);
  if (response.statusCode !== 200)
    throw new Error(`微信 CDN 上传 HTTP ${response.statusCode}`);
  const encryptedParam = Object.entries(response.headers).find(
    ([name]) => name.toLowerCase() === "x-encrypted-param",
  )?.[1];
  const downloadParam = nonblank(encryptedParam, "x-encrypted-param");
  const media = {
    encrypt_query_param: downloadParam,
    aes_key: encodeAsciiBase64(key),
    encrypt_type: 1,
  };
  if (kind === "image")
    return { type: 2, image_item: { media, mid_size: byteLength(encrypted) } };
  if (kind === "video")
    return {
      type: 5,
      video_item: { media, video_size: byteLength(encrypted) },
    };
  if (kind === "voice")
    return {
      type: 3,
      voice_item: {
        media,
        encode_type: options.voice_format,
        sample_rate: options.sample_rate,
        playtime: options.duration_ms,
      },
    };
  const name =
    options.file_name === undefined
      ? source.split(/[\\/]/).pop()!
      : nonblank(options.file_name, "file_name");
  return {
    type: 4,
    file_item: { media, file_name: name, len: String(input.size) },
  };
}

/** Downloads and decrypts one inbound CDN media item to the plugin incoming directory. */
export async function downloadMedia(
  api: WeChatApi,
  item: JsonObject,
  messageId: string,
): Promise<{
  filePath: string;
  fileName: string;
  mimeType: string;
  fileSize: number;
  content: string;
}> {
  const selected = mediaReference(item);
  if (selected === null) throw new Error("消息项目不包含可下载的媒体。");
  const fullUrl =
    selected.media.full_url === undefined
      ? ""
      : text(selected.media.full_url, "full_url");
  const encryptedParam =
    selected.media.encrypt_query_param === undefined
      ? ""
      : text(selected.media.encrypt_query_param, "encrypt_query_param");
  const url =
    fullUrl !== ""
      ? wechatUrl(fullUrl, "media full_url")
      : `${CDN_BASE}/download?encrypted_query_param=${encodeURIComponent(nonblank(encryptedParam, "encrypt_query_param"))}`;
  const response = await api.http.request({
    url,
    method: "GET",
    responseType: "arraybuffer",
    connect_timeout: 15,
    read_timeout: 60,
    follow_redirects: false,
  });
  if (response.statusCode < 200 || response.statusCode >= 300)
    throw new Error(`微信 CDN 下载 HTTP ${response.statusCode}`);
  const encrypted = nonblank(response.contentBase64, "download binary data");
  if (byteLength(encrypted) > MAX_MEDIA_BYTES + 16)
    throw new Error("微信媒体下载超过 25 MiB 限制。");
  const key =
    selected.key ??
    decodeUtf8Base64(nonblank(selected.media.aes_key, "media.aes_key"));
  const plain = CryptoJS.binary.decryptAesEcb(encrypted, key);
  const directory = `${getPluginConfigDir()}/incoming`;
  const made = await Tools.Files.mkdir(directory, true);
  if (!made.successful) throw new Error(made.details);
  const filePath = `${directory}/${encodeURIComponent(messageId)}-${CryptoJS.binary.randomHex(8)}`;
  const written = await Tools.Files.writeBinary(filePath, plain);
  if (!written.successful) throw new Error(written.details);
  const info = await Tools.Files.info(filePath);
  const mimeType = mediaMime(
    selected.kind,
    plain,
    selected.voiceFormat,
    selected.name,
  );
  const extensions: Record<string, string> = {
    "image/jpeg": "jpg",
    "image/png": "png",
    "image/gif": "gif",
    "image/webp": "webp",
    "video/mp4": "mp4",
    "audio/mpeg": "mp3",
    "audio/amr": "amr",
    "audio/ogg": "ogg",
  };
  const extension = extensions[mimeType];
  const fileName =
    selected.name ??
    `${selected.kind}-${messageId}${extension === undefined ? "" : `.${extension}`}`;
  return {
    filePath,
    fileName,
    mimeType,
    fileSize: integer(info.size, "saved media size", 0, MAX_MEDIA_BYTES),
    content: filePath,
  };
}

/** Decodes a base64 key representation back into the required hexadecimal form. */
function decodeUtf8Base64(value: string): string {
  const bytes = decodeBase64(value);
  if (bytes.length === 32) {
    const hexadecimal = String.fromCharCode(...bytes);
    if (!/^[0-9a-f]{32}$/i.test(hexadecimal))
      throw new Error("Invalid hexadecimal media key");
    return hexadecimal;
  }
  if (bytes.length !== 16)
    throw new Error(
      "Media key must contain 16 raw bytes or 32 hexadecimal characters",
    );
  return bytes.map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** Decodes base64 protocol keys into their original byte values. */
function decodeBase64(value: string): number[] {
  if (
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(
      value,
    )
  )
    throw new Error("Invalid base64 media key");
  const alphabet =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  const clean = value.replace(/=+$/, "");
  const output: number[] = [];
  for (let index = 0; index < clean.length; index += 4) {
    const a = alphabet.indexOf(clean[index]),
      b = alphabet.indexOf(clean[index + 1] ?? "A");
    const c = alphabet.indexOf(clean[index + 2] ?? "A"),
      d = alphabet.indexOf(clean[index + 3] ?? "A");
    if (a < 0 || b < 0 || c < 0 || d < 0)
      throw new Error("Invalid base64 media key");
    const bits = (a << 18) | (b << 12) | (c << 6) | d;
    output.push((bits >>> 16) & 255);
    if (index + 2 < clean.length) output.push((bits >>> 8) & 255);
    if (index + 3 < clean.length) output.push(bits & 255);
  }
  return output;
}

/** Identifies supported image signatures and explicit voice formats for native Chat attachments. */
function mediaMime(
  kind: MediaKind,
  data: string,
  voiceFormat?: number,
  fileName?: string,
): string {
  if (kind === "image") {
    const prefix = decodeBase64(data.slice(0, 16));
    if (prefix[0] === 0xff && prefix[1] === 0xd8) return "image/jpeg";
    if (
      prefix[0] === 0x89 &&
      prefix[1] === 0x50 &&
      prefix[2] === 0x4e &&
      prefix[3] === 0x47
    )
      return "image/png";
    if (String.fromCharCode(...prefix.slice(0, 3)) === "GIF")
      return "image/gif";
    if (
      String.fromCharCode(...prefix.slice(0, 4)) === "RIFF" &&
      String.fromCharCode(...prefix.slice(8, 12)) === "WEBP"
    )
      return "image/webp";
    throw new Error("微信图片不是受支持的 JPEG、PNG、GIF 或 WebP 文件。");
  }
  if (kind === "video") return "video/mp4";
  if (kind === "voice") {
    const formats: Record<number, string> = {
      1: "audio/L16",
      2: "audio/adpcm",
      3: "application/octet-stream",
      4: "audio/speex",
      5: "audio/amr",
      6: "audio/silk",
      7: "audio/mpeg",
      8: "audio/ogg",
    };
    return voiceFormat === undefined
      ? "application/octet-stream"
      : formats[voiceFormat];
  }
  const extension = fileName?.split(".").pop()?.toLowerCase();
  const types: Record<string, string> = {
    pdf: "application/pdf",
    txt: "text/plain",
    csv: "text/csv",
    json: "application/json",
    docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  };
  return extension === undefined || types[extension] === undefined
    ? "application/octet-stream"
    : types[extension];
}
