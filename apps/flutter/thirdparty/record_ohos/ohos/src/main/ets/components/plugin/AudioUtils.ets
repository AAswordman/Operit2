/*
 * Copyright (C) 2025 Huawei Device Co., Ltd.
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
import { audio } from '@kit.AudioKit';
import { RecordConfig } from './RecordConfig';

/** Maps one requested numeric sample rate to the OHOS audio enum. */
export function mapSampleRate(rate: number): audio.AudioSamplingRate {
  switch (rate) {
    case 8000:
      return audio.AudioSamplingRate.SAMPLE_RATE_8000;
    case 11025:
      return audio.AudioSamplingRate.SAMPLE_RATE_11025;
    case 12000:
      return audio.AudioSamplingRate.SAMPLE_RATE_12000;
    case 16000:
      return audio.AudioSamplingRate.SAMPLE_RATE_16000;
    case 22050:
      return audio.AudioSamplingRate.SAMPLE_RATE_22050;
    case 24000:
      return audio.AudioSamplingRate.SAMPLE_RATE_24000;
    case 32000:
      return audio.AudioSamplingRate.SAMPLE_RATE_32000;
    case 44100:
      return audio.AudioSamplingRate.SAMPLE_RATE_44100;
    case 48000:
      return audio.AudioSamplingRate.SAMPLE_RATE_48000;
    case 64000:
      return audio.AudioSamplingRate.SAMPLE_RATE_64000;
    case 96000:
      return audio.AudioSamplingRate.SAMPLE_RATE_96000;
    default:
      throw new Error(`Unsupported sample rate: ${rate}`);
  }
}

/** Maps one requested channel count to the OHOS audio enum. */
export function mapChannels(channels: number): audio.AudioChannel {
  switch (channels) {
    case 1:
      return audio.AudioChannel.CHANNEL_1;
    case 2:
      return audio.AudioChannel.CHANNEL_2;
    case 3:
      return audio.AudioChannel.CHANNEL_3;
    case 4:
      return audio.AudioChannel.CHANNEL_4;
    case 5:
      return audio.AudioChannel.CHANNEL_5;
    case 6:
      return audio.AudioChannel.CHANNEL_6;
    case 7:
      return audio.AudioChannel.CHANNEL_7;
    case 8:
      return audio.AudioChannel.CHANNEL_8;
    default:
      throw new Error(`Unsupported channel count: ${channels}`);
  }
}

/** Builds raw PCM16 microphone capture options from record configuration. */
export function buildStreamCapturerOptions(config: RecordConfig): audio.AudioCapturerOptions {
  const sampleRate = config.sampleRate > 0 ? config.sampleRate : 44100;
  const numChannels = config.numChannels > 0 ? config.numChannels : 1;

  return {
    streamInfo: {
      samplingRate: mapSampleRate(sampleRate),
      channels: mapChannels(numChannels),
      sampleFormat: audio.AudioSampleFormat.SAMPLE_FORMAT_S16LE,
      encodingType: audio.AudioEncodingType.ENCODING_TYPE_RAW,
    },
    capturerInfo: {
      source: audio.SourceType.SOURCE_TYPE_MIC,
      capturerFlags: 0,
    },
  };
}
