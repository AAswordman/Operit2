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
import { BinaryMessenger, MethodResult } from '@ohos/flutter_ohos';
import EventChannel from '@ohos/flutter_ohos/src/main/ets/plugin/common/EventChannel';
import { RecordConfig } from './RecordConfig';
import { IRecorder } from './IRecorder';

import { MediaRecorder } from './MediaRecorder';
import { AudioStreamRecorder } from './AudioStreamRecorder';
import { RecorderRecordStreamHandler } from './RecorderRecordStreamHandler';

import { RecorderStateStreamHandler } from './RecorderStateStreamHandler';

export class RecorderWrapper {
  private readonly TAG: string = 'RecorderWrapper';
  private readonly EVENTS_STATE_CHANNEL = "com.llfbandit.record/events/"
  private readonly EVENTS_RECORD_CHANNEL = "com.llfbandit.record/eventsRecord/"
  private recorderId: string | null;
  private messenger: BinaryMessenger;
  private recorder: IRecorder | null = null;
  private recorderStateStreamHandler: RecorderStateStreamHandler = new RecorderStateStreamHandler();
  private recorderRecordStreamHandler: RecorderRecordStreamHandler = new RecorderRecordStreamHandler();
  private eventChannel: EventChannel | null = null;
  private eventRecordChannel: EventChannel | null = null
  private isStreamMode: boolean = false;

  /** Creates one recorder instance and its isolated event channels. */
  constructor(recorderId: string, message: BinaryMessenger) {
    this.recorderId = recorderId;
    this.messenger = message;
    this.init();
  }

  /** Registers state and PCM stream event channel handlers. */
  init(): void {
    this.eventChannel = new EventChannel(this.messenger, this.EVENTS_STATE_CHANNEL + this.recorderId)
    this.eventChannel!.setStreamHandler(this.recorderStateStreamHandler)
    this.eventRecordChannel = new EventChannel(this.messenger, this.EVENTS_RECORD_CHANNEL + this.recorderId)
    this.eventRecordChannel!.setStreamHandler(this.recorderRecordStreamHandler)
  }

  /** Starts one encoded recording that writes to a file. */
  startRecordingToFile(config: RecordConfig, result: MethodResult) {
    this.isStreamMode = false;
    this.startRecording(config, result)
  }

  /** Starts one PCM recording that publishes byte chunks. */
  startRecordingToStream(config: RecordConfig, result: MethodResult) {
    this.isStreamMode = true;
    this.startRecording(config, result)
  }

  /** Selects the recorder mode and coordinates recorder replacement. */
  private startRecording(config: RecordConfig, result: MethodResult): void {
    try {
      if (this.recorder != null && this.shouldRecreateRecorder()) {
        const oldRecorder = this.recorder;
        this.recorder = null;
        this.releaseRecorderForModeSwitch(oldRecorder).then(() => {
          this.continueStartRecording(config, result);
        }).catch((e: Error) => {
          result.error(this.TAG + " record", e.message, null);
        });
        return;
      }

      this.continueStartRecording(config, result);
    } catch (e) {
      result.error(this.TAG + " record", e.message, null);
    }
  }

  /** Starts the selected recorder after any mode transition completes. */
  private continueStartRecording(config: RecordConfig, result: MethodResult): void {
    if (this.recorder == null) {
      this.recorder = this.createRecorder();
      this.start(config, result)
    } else if (this.recorder!.isRecording) {
      this.recorder!.stop(() => {
        this.start(config, result)
      });
    } else {
      this.start(config, result)
    }
  }

  /** Releases one recorder before replacing it with another mode. */
  private async releaseRecorderForModeSwitch(recorder: IRecorder): Promise<void> {
    if (recorder instanceof AudioStreamRecorder) {
      await recorder.disposeAsync();
      return;
    }

    recorder.dispose();
  }

  /** Returns whether the active recorder has the wrong stream mode. */
  private shouldRecreateRecorder(): boolean {
    if (this.recorder == null) {
      return false;
    }

    const isStreamRecorder = this.recorder instanceof AudioStreamRecorder;
    return this.isStreamMode !== isStreamRecorder;
  }

  /** Invokes the file or stream recorder start contract. */
  private start(config: RecordConfig, result: MethodResult): void {
    if (this.isStreamMode) {
      const streamRecorder = this.recorder as AudioStreamRecorder;
      streamRecorder.startAsync(config).then(() => {
        result.success(null);
      }).catch((e: Error) => {
        result.error(this.TAG + " record", e.message, null);
      });
      return;
    }

    this.recorder!.start(config);
    result.success(null);
  }

  /** Pauses the active recorder. */
  pause(result: MethodResult) {
    try {
      this.recorder!.pause()
      result.success(null)
    } catch (e) {
      result.error("record", e.message, null)
    }
  }

  /** Reports whether the active recorder is paused. */
  isPaused(result: MethodResult) {
    if (this.recorder == null) {
      result.success(false);
    } else {
      result.success(this.recorder.isPaused);
    }
  }

  /** Reports whether the active recorder is recording. */
  isRecording(result: MethodResult) {
    if(this.recorder == null){
      result.success(false);
    }else{
      result.success(this.recorder.isRecording);
    }
  }

  /** Reports current and maximum recorder amplitude. */
  getAmplitude(result: MethodResult) {
    if (this.recorder != null) {
      let amps = this.recorder!.getAmplitude();
      let amp: Map<string, number> = new Map<string, number>();
      amp.set("current", amps[0]);
      amp.set("max", amps[1]);
      result.success(amp)
    } else {
      result.success(null)
    }
  }

  /** Resumes the active recorder. */
  resume(result: MethodResult) {
    try {
      this.recorder!.resume()
      result.success(null)
    } catch (e) {
      result.error(this.TAG + " record", e, "")
    }
  }

  /** Stops recording and returns a path only for file mode. */
  stop(result: MethodResult) {
    try {
      if (this.recorder == null) {
        result.success(null)
      } else {
        this.recorder!.stop((path) => {
          if (this.isStreamMode || path == null || path.trim().length === 0) {
            result.success(null);
          } else {
            result.success(path);
          }
        });
      }
    } catch (e) {
      result.error("record", e, "")
    }
  }

  /** Cancels the active recorder. */
  cancel(result: MethodResult) {
    try {
      this.recorder!.cancel()
      result.success(null)
    } catch (e) {
      result.error(this.TAG + " record", e, "")
    }
  }

  /** Creates the recorder implementation selected by the current mode. */
  private createRecorder(): IRecorder {
    if (this.isStreamMode) {
      return new AudioStreamRecorder(
        this.recorderStateStreamHandler,
        this.recorderRecordStreamHandler,
      );
    }

    return new MediaRecorder(this.recorderStateStreamHandler, this.recorderRecordStreamHandler);
  }

  /** Releases the recorder and detaches both event channels. */
  public dispose(): void {
    try {
      if (this.recorder != null) {
        this.recorder.dispose();
      }
    } catch (error) {
      console.error(this.TAG + ' record' + ' dispose ' + error);
    } finally {
      this.recorder = null;
    }
    this.eventChannel!.setStreamHandler(null)
    this.eventChannel = null
    this.eventRecordChannel!.setStreamHandler(null)
    this.eventRecordChannel = null
  }
}
