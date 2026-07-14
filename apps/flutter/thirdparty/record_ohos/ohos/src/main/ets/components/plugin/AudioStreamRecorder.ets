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
import { BusinessError } from '@kit.BasicServicesKit';
import { RecordConfig } from './RecordConfig';
import { AudioEncoder } from './AudioEncoder';
import { RecordState } from './RecordState';
import { IRecorder } from './IRecorder';
import { RecorderStateStreamHandler } from './RecorderStateStreamHandler';
import { RecorderRecordStreamHandler } from './RecorderRecordStreamHandler';
import { buildStreamCapturerOptions } from './AudioUtils';

export class AudioStreamRecorder implements IRecorder {
  private readonly TAG = 'AudioStreamRecorder';
  private audioCapturer: audio.AudioCapturer | null = null;
  private stateHandler: RecorderStateStreamHandler;
  private recordHandler: RecorderRecordStreamHandler;
  private currentAmplitude: number = -160.0;
  private maxAmplitude: number = -160.0;
  isRecording: boolean = false;
  isPaused: boolean = false;
  private readonly readDataHandler = (buffer: ArrayBuffer): void => {
    this.onReadData(buffer);
  };
  private releasePromise: Promise<void> | null = null;

  /** Creates a PCM stream recorder bound to state and chunk event handlers. */
  constructor(
    stateHandler: RecorderStateStreamHandler,
    recordHandler: RecorderRecordStreamHandler,
  ) {
    this.stateHandler = stateHandler;
    this.recordHandler = recordHandler;
  }

  /** Rejects the synchronous entry point used only by file recorders. */
  start(_config: RecordConfig): void {
    console.error(this.TAG + ' Use startAsync() for stream recording.');
  }

  /** Creates and starts an AudioCapturer for one PCM stream configuration. */
  async startAsync(config: RecordConfig): Promise<void> {
    this.validateStreamEncoder(config);
    await this.ensureReleased();

    const options = buildStreamCapturerOptions(config);
    this.audioCapturer = await audio.createAudioCapturer(options);

    this.audioCapturer.on('readData', this.readDataHandler);

    await this.startCapturer();
    this.maxAmplitude = -160.0;
    this.updateState(RecordState.RECORD);
  }

  /** Stops capture and reports an empty path for stream mode. */
  stop(callback?: (path: string) => void): void {
    this.ensureReleased().then(() => {
      this.updateState(RecordState.STOP);
      if (callback) {
        callback('');
      }
    }).catch((error: BusinessError) => {
      console.error(this.TAG + ' stop failed: ' + error.message);
      this.updateState(RecordState.STOP);
      if (callback) {
        callback('');
      }
    });
  }

  /** Cancels active stream capture and publishes the stopped state. */
  cancel(): void {
    this.ensureReleased().then(() => {
      this.updateState(RecordState.STOP);
    }).catch((error: BusinessError) => {
      console.error(this.TAG + ' cancel failed: ' + error.message);
      this.updateState(RecordState.STOP);
    });
  }

  /** Pauses chunk delivery while leaving the AudioCapturer active. */
  pause(): void {
    if (!this.isRecording || this.isPaused) {
      return;
    }

    // AudioCapturer has no pause API on OHOS — drop chunks in onReadData instead.
    this.updateState(RecordState.PAUSE);
  }

  /** Resumes chunk delivery from the active AudioCapturer. */
  resume(): void {
    if (!this.isRecording || !this.isPaused) {
      return;
    }

    this.updateState(RecordState.RECORD);
  }

  /** Returns current and maximum PCM amplitude in decibels. */
  getAmplitude(): Array<number> {
    return [this.currentAmplitude, this.maxAmplitude];
  }

  /** Starts asynchronous release without waiting for completion. */
  dispose(): void {
    this.ensureReleased().catch((error: BusinessError) => {
      console.error(this.TAG + ' dispose failed: ' + error.message);
    });
  }

  /** Releases the AudioCapturer and waits for completion. */
  async disposeAsync(): Promise<void> {
    await this.ensureReleased();
  }

  /** Returns the single in-flight capturer release operation. */
  private ensureReleased(): Promise<void> {
    if (this.releasePromise != null) {
      return this.releasePromise;
    }

    this.releasePromise = this.releaseCapturer().finally(() => {
      this.releasePromise = null;
    });
    return this.releasePromise;
  }

  /** Requires PCM16 because record streams expose raw sample bytes. */
  private validateStreamEncoder(config: RecordConfig): void {
    if (config.encoder !== AudioEncoder.pcm16bits) {
      throw new Error(`${config.encoder} not supported in streaming mode.`);
    }
  }

  /** Starts the native AudioCapturer callback API as a promise. */
  private async startCapturer(): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      this.audioCapturer!.start((err: BusinessError) => {
        if (err) {
          reject(new Error('Failed to start AudioCapturer: ' + err.message));
          return;
        }
        resolve();
      });
    });
  }

  /** Publishes one non-empty PCM chunk while recording is active. */
  private onReadData(buffer: ArrayBuffer): void {
    if (!this.isRecording || this.isPaused || buffer.byteLength === 0) {
      return;
    }

    const chunk = new Uint8Array(buffer).slice();
    this.updateAmplitude(chunk);
    this.recordHandler.sendRecordChunkEvent(chunk);
  }

  /** Updates current and peak amplitude from little-endian PCM16 bytes. */
  private updateAmplitude(bytes: Uint8Array): void {
    if (bytes.byteLength < 2) {
      this.currentAmplitude = -160.0;
      return;
    }

    const pcm = new Int16Array(bytes.buffer, bytes.byteOffset, bytes.byteLength / 2);
    let maxSample = 0;
    for (let i = 0; i < pcm.length; i++) {
      maxSample = Math.max(maxSample, Math.abs(pcm[i]));
    }

    if (maxSample <= 0) {
      this.currentAmplitude = -160.0;
      return;
    }

    this.currentAmplitude = 20 * (Math.log10(maxSample / 32767.0));
    this.maxAmplitude = Math.max(this.maxAmplitude, this.currentAmplitude);
  }

  /** Updates recorder flags and publishes one record state event. */
  private updateState(state: RecordState): void {
    switch (state) {
      case RecordState.PAUSE:
        this.isRecording = true;
        this.isPaused = true;
        this.stateHandler.sendStateEvent(RecordState.PAUSE);
        break;
      case RecordState.RECORD:
        this.isRecording = true;
        this.isPaused = false;
        this.stateHandler.sendStateEvent(RecordState.RECORD);
        break;
      case RecordState.STOP:
        this.isRecording = false;
        this.isPaused = false;
        this.stateHandler.sendStateEvent(RecordState.STOP);
        break;
      default:
        break;
    }
  }

  /** Stops, detaches, and releases the current AudioCapturer. */
  private async releaseCapturer(): Promise<void> {
    const capturer = this.audioCapturer;
    if (capturer == null) {
      return;
    }

    this.audioCapturer = null;
    this.isRecording = false;
    this.isPaused = false;

    capturer.off('readData', this.readDataHandler);

    await new Promise<void>((resolve) => {
      capturer.stop((err: BusinessError) => {
        if (err) {
          console.error(this.TAG + ' capturer stop failed: ' + err.message);
        }
        resolve();
      });
    });

    await capturer.release();
  }
}
