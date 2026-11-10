export type ProviderId = 'openai' | 'qwen' | 'zhipu' | 'xai' | 'minimax' | 'nanobanana';
export type ImageOption = 'size' | 'aspect_ratio' | 'resolution' | 'count' | 'negative_prompt' | 'optimize_prompt' | 'watermark' | 'seed' | 'reference_images';

export interface Provider {
  id: ProviderId;
  title: string;
  description: string;
  model: string;
  apiKey: string;
  endpointKey?: string;
  endpoint?: string;
  options: ImageOption[];
}

export interface ImageRequest {
  provider: ProviderId;
  prompt: string;
  model?: string;
  size?: string;
  aspect_ratio?: string;
  resolution?: string;
  count?: number;
  negative_prompt?: string;
  optimize_prompt?: boolean;
  watermark?: boolean;
  seed?: number;
  reference_images?: string[];
  file_name?: string;
}

export interface VideoRequest {
  provider: 'xai';
  prompt: string;
  model?: string;
  aspect_ratio?: string;
  resolution?: string;
  duration?: number;
  reference_image?: string;
  reference_video?: string;
  file_name?: string;
}

export interface GeneratedAsset {
  uri: string;
  markdown: string;
}

export interface GenerationResult {
  provider: ProviderId;
  kind: 'image' | 'video';
  prompt: string;
  model: string;
  assets: GeneratedAsset[];
  markdown: string;
  created_at: number;
}

export interface ToolResponse<T> {
  success: boolean;
  message: string;
  data?: T;
}
