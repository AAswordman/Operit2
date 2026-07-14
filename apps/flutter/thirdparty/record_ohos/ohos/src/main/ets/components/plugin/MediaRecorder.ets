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
import { BusinessError } from '@kit.BasicServicesKit';
import fs from '@ohos.file.fs';
import { RecordConfig } from './RecordConfig';
import { AudioEncoder } from './AudioEncoder';

import { RecordState } from './RecordState';
import { IRecorder } from './IRecorder';
import { RecorderStateStreamHandler } from './RecorderStateStreamHandler';
import { RecorderRecordStreamHandler } from './RecorderRecordStreamHandler';
import { media } from '@kit.MediaKit';

export class MediaRecorder implements IRecorder {
  private readonly TAG = 'MediaRecorder';
  private file: fs.File | null = null;
  private mAvRecorder: media.AVRecorder | null = null;
  private fileMediaPath: string = '';
  private fileConfigPath: string = '';
  private mMaxAmplitudeCur: number = 0.0001;
  private mMaxAmplitude: number = -160.0001
  isRecording: boolean = false;
  isPaused: boolean = false;
  private recorderStateStreamHandler: RecorderStateStreamHandler | null = null;

  /** Creates an encoded recorder bound to one state event handler. */
  constructor(recorderStateStreamHandler: RecorderStateStreamHandler,
    recorderRecordStreamHandler: RecorderRecordStreamHandler) {
    this.recorderStateStreamHandler = recorderStateStreamHandler;
  }

  /** Returns current and maximum amplitude for encoded recording. */
  getAmplitude(): Array<number> {
    if (this.mAvRecorder != null || this.mAvRecorder != undefined) {
      this.mAvRecorder.getAudioCapturerMaxAmplitude().then((maxAmplitude) => {
        this.mMaxAmplitudeCur = maxAmplitude;
      });
      let fixedCur = this.mMaxAmplitudeCur + 0.0001;
      let fixedMax = this.mMaxAmplitude + 0.0001;
      return [fixedCur, fixedMax];
    } else {
      return [0.0001, 0.0001];
    }
  }

  /** Creates the native AVRecorder used by one file recording. */
  private createRecorder(config: RecordConfig): void {
    media.createAVRecorder((error: BusinessError, recorder: media.AVRecorder) => {
      if (recorder != null) {
        this.mAvRecorder = recorder;
        this.realStartRecording(config);
      } else {
        console.error(this.TAG + `Failed to create AVRecorder, error message:${error.message}`);
      }
    });
  }

  /** Starts one encoded recording with the supplied configuration. */
  start(config: RecordConfig): void {
    this.stopRecording();
    if (this.mAvRecorder == null || this.mAvRecorder == undefined) {
      this.createRecorder(config);
    } else {
      this.realStartRecording(config);
    }

  }

  /** Configures and prepares the native AVRecorder output file. */
  private realStartRecording(config: RecordConfig): void {
    let avRecorderProfile: media.AVRecorderProfile = {
      audioBitrate: config.bitRate,
      audioChannels: config.numChannels,
      audioCodec: this.getEncoder(config.encoder),
      audioSampleRate: config.sampleRate,
      fileFormat: this.getFormat(config.encoder)
    };
    this.fileConfigPath = config.path;
    this.fileMediaPath = 'fd://' + this.getFileFD(config.path);
    let avRecorderConfig: media.AVRecorderConfig = {
      audioSourceType: media.AudioSourceType.AUDIO_SOURCE_TYPE_MIC,
      profile: avRecorderProfile,
      url: this.fileMediaPath
    }
    console.info(this.TAG + ' zcyRecordTag _ets realStartRecording  filePath: ' + config.path);
    if (this.mAvRecorder != null && this.mAvRecorder != undefined) {
      this.mAvRecorder.prepare(avRecorderConfig, (err: BusinessError) => {
        if (err) {
          console.error(this.TAG + 'Failed to prepare and error is ' + err.message);
        } else {
          this.startRecordAfterPrepare();
        }
      })
    }
  }

  /** Starts AVRecorder capture after native preparation completes. */
  startRecordAfterPrepare(): void {
    this.mAvRecorder!.start((err: BusinessError) => {
      if (err) {
        console.error(this.TAG + 'Failed to start AVRecorder and error is ' + err.message);
      } else {
        this.updateState(RecordState.RECORD);
      }
    });
  }

  /** Stops encoded recording and reports its output path. */
  async stop(callback?: (path: string) => void): Promise<void> {
    if (this.mAvRecorder != null) {
      if (this.mAvRecorder.state === 'started' || this.mAvRecorder.state === 'paused') {
        await this.mAvRecorder.stop();
      }
      await this.mAvRecorder.reset();
      await this.mAvRecorder.release();
      this.mAvRecorder = null;
      if (this.file != null) {
        fs.closeSync(this.file);
        this.file = null;
      }
      this.updateState(RecordState.STOP);
    }
    if (callback) {
      callback(this.fileConfigPath);
    }
  }

  /** Cancels encoded recording and removes its output. */
  cancel() {
    this.stopRecording();
  }

  /** Pauses active encoded recording. */
  async pause(): Promise<void> {
    if (this.mAvRecorder != null && this.mAvRecorder.state === 'started') {
      await this.mAvRecorder.pause();
      this.updateState(RecordState.PAUSE);
    }
  }

  /** Resumes paused encoded recording. */
  async resume(): Promise<void> {
    if (this.mAvRecorder != null && this.mAvRecorder.state === 'paused') {
      await this.mAvRecorder.resume();
      this.updateState(RecordState.RECORD);
    }
  }

  /** Releases native AVRecorder resources. */
  dispose() {
    this.stopRecording();
  }

  /** Stops and releases the native AVRecorder instance. */
  async stopRecording(): Promise<void> {
    if (this.mAvRecorder != null) {
      try {
        if (this.mAvRecorder.state === 'started' || this.mAvRecorder.state === 'paused' || this.isRecording ||
        this.isPaused) {
          this.mAvRecorder.stop()
        }
        await this.mAvRecorder.reset();
        await this.mAvRecorder.release();
        this.mAvRecorder = null;
        if (this.file != null) {
          fs.closeSync(this.file);
          this.file = null;
        }
      } catch (e) {
        console.error(`${this.TAG}: Error during recorder stop: ${JSON.stringify(e)}`);
      }
    }
    this.updateState(RecordState.STOP)
    this.mMaxAmplitude = -160.0
  }

  /** Updates flags and publishes one encoded recorder state. */
  updateState(state: RecordState): void {
    switch (state) {
      case RecordState.PAUSE:
        this.isRecording = true
        this.isPaused = true
        this.recorderStateStreamHandler!.sendStateEvent(RecordState.PAUSE)
        break;
      case RecordState.RECORD:
        this.isRecording = true
        this.isPaused = false
        this.recorderStateStreamHandler!.sendStateEvent(RecordState.RECORD)
        break;
      case RecordState.STOP:
        this.isRecording = false
        this.isPaused = false
        this.recorderStateStreamHandler!.sendStateEvent(RecordState.STOP)
        break;
      default:
        break;
    }
  }

  /** Maps one record encoder id to an OHOS codec MIME type. */
  getEncoder(encoder: string): media.CodecMimeType {
    switch (encoder) {
      case AudioEncoder.aacLc:
        return media.CodecMimeType.AUDIO_AAC;
      case AudioEncoder.aacEld:
        return media.CodecMimeType.AUDIO_AAC;
      case AudioEncoder.aacHe:
        return media.CodecMimeType.AUDIO_AAC;
      default:
        throw new Error(`Unsupported OHOS file encoder: ${encoder}`);
    }
  }

  /** Maps one record encoder id to an OHOS container format. */
  getFormat(encoder: string): media.ContainerFormatType {
    switch (encoder) {
      case AudioEncoder.aacLc:
      case AudioEncoder.aacEld:
      case AudioEncoder.aacHe:
        return media.ContainerFormatType.CFT_MPEG_4A;
      default:
        throw new Error(`Unsupported OHOS file encoder: ${encoder}`);
    }
  }

  /** Opens one output file and returns its native descriptor. */
  getFileFD(filePath: string): number {
    this.file = fs.openSync(filePath, fs.OpenMode.READ_WRITE | fs.OpenMode.CREATE);
    const fd: number = this.file.fd;
    return fd;
  }
}
