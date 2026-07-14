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
import { MethodCall, MethodResult, BinaryMessenger, MethodCallHandler } from '@ohos/flutter_ohos';
import { RecorderWrapper } from './RecorderWrapper';
import { RecordConfig } from './RecordConfig';
import { AudioEncoder } from './AudioEncoder';
import { Utils } from './Utils';
import PermissionManager from '../permission/PermissionManager';

export class MethodCallHandlerImpl implements MethodCallHandler {
  private readonly TAG = 'MethodCallHandlerImpl';
  private messenger: BinaryMessenger;
  private recorders: Map<string, RecorderWrapper> = new Map<string, RecorderWrapper>();
  private permissionManager: PermissionManager;

  /** Creates the method handler with one messenger and permission manager. */
  constructor(messenger: BinaryMessenger, permissionManager: PermissionManager) {
    this.messenger = messenger;
    this.permissionManager = permissionManager;
  }

  /** Dispatches one record platform-interface method call. */
  async onMethodCall(call: MethodCall, result: MethodResult): Promise<void> {
    const recorderId: string = call.argument('recorderId');
    if (recorderId === null || recorderId.trim() === '') {
      result.error(this.TAG + ' ARGUMENT_ERROR', 'Call missing mandatory parameter recorderId.', null);
      return;
    }
    if (call.method == "create") {
      try {
        this.createRecorder(recorderId);
        result.success(null);
      } catch (e) {
        const error = e as Error;
        result.error(this.TAG + " record", "Cannot create recording configuration.", error.message);
      }
      return;
    }
    let recorder = this.recorders.get(recorderId);
    if (recorder == null) {
      result.error(this.TAG + " record",
        "Recorder has not yet been created or has already been disposed.", null
      )
      return
    }
    switch (call.method) {
      case "start":
        this.permissionManager.checkPermission().then(async (granted: boolean) => {
          if (!granted) {
            result.error(this.TAG + " record", "Microphone permission denied.", null);
            return;
          }
          let config = this.getRecordConfig(call);
          recorder?.startRecordingToFile(config, result);
        }).catch((error: Error) => {
          result.error(this.TAG + " record", "Permission check failed: " + error.message, null);
        })
        break;
      case "startStream":
        try {
          let config = this.getRecordConfig(call);
          if (config.encoder !== AudioEncoder.pcm16bits) {
            result.error(this.TAG + " record",
              `${config.encoder} not supported in streaming mode.`, null);
            break;
          }
          this.permissionManager.checkPermission().then(async (granted: boolean) => {
            if (!granted) {
              result.error(this.TAG + " record", "Microphone permission denied.", null);
              return;
            }
            recorder?.startRecordingToStream(config, result);
          }).catch((error: Error) => {
            result.error(this.TAG + " record", "Permission check failed: " + error.message, null);
          })
        } catch (e) {
          const error = e as Error;
          result.error(this.TAG + " record", "Cannot create recording configuration.", error.message)
        }
        break;
      case "stop":
        recorder.stop(result);
        break;
      case "pause":
        recorder.pause(result);
        break;
      case "resume":
        recorder.resume(result);
        break;
      case "isPaused":
        recorder.isPaused(result);
        break;
      case "isRecording":
        recorder.isRecording(result);
        break;
      case "cancel":
        recorder.cancel(result);
        break;
      case "hasPermission":
        this.permissionManager.hasPermission(result, call.argument('request') as boolean)
        break;
      case "getAmplitude":
        recorder.getAmplitude(result);
        break;
      case "listInputDevices":
        result.success(this.getDeviceMapList());
        break;
      case "dispose":
        this.disposeRecorder(recorder, recorderId);
        result.success(null)
        break;
      case "isEncoderSupported":
        result.success(true);
        break;
      default:
        result.notImplemented()
        break;
    }
  };

  /** Parses one platform-interface recording configuration. */
  getRecordConfig(call: MethodCall): RecordConfig {
      let config = call.argument("androidConfig") as Map<string, Object>;
      const recordConfig = new RecordConfig();
      recordConfig.path = call.argument("path");
      recordConfig.encoder = Utils.firstNonNull(call.argument("encoder"), "aacLc");
      recordConfig.bitRate = Utils.firstNonNull(call.argument("bitRate"), 128000);
      recordConfig.sampleRate = Utils.firstNonNull(call.argument("sampleRate"), 44100);
      recordConfig.numChannels = Utils.firstNonNull(call.argument("numChannels"), 2);
      recordConfig.autoGain = Utils.firstNonNull(call.argument("autoGain"), false);
      recordConfig.echoCancel = Utils.firstNonNull(call.argument("echoCancel"), false);
      recordConfig.noiseSuppress = Utils.firstNonNull(call.argument("noiseSuppress"), false);
      let isMuteAudio: boolean = false;
      if (config) {
        isMuteAudio = config.get("muteAudio") as boolean ?? false;
      }
      recordConfig.muteAudio = isMuteAudio;
      const streamBufferSize = call.argument("streamBufferSize") as number;
      if (streamBufferSize !== undefined && streamBufferSize !== null) {
        recordConfig.streamBufferSize = streamBufferSize;
      }
      return recordConfig;
  }

  /** Creates and stores one recorder by its stable identifier. */
  createRecorder(recorderId: string): void {
    let recorder: RecorderWrapper = new RecorderWrapper(recorderId, this.messenger);
    this.recorders.set(recorderId, recorder);
  }

  /** Returns the microphone input device exposed by this plugin. */
  getDeviceMapList(): Array<Map<string, string>> {
    const deviceInfo = new Map<string, string>();
    deviceInfo.set("id", "4");
    deviceInfo.set("label", "Microphone (Built-in Microphone)");
    const deviceList: Array<Map<string, string>> = [deviceInfo];
    return deviceList;
  }

  /** Releases every recorder owned by this method handler. */
  async dispose(): Promise<void> {
    console.info(`${this.TAG}: Disposing all recorders...`);
    const disposePromises: Promise<void>[] = [];

    this.recorders.forEach((value: RecorderWrapper, key: string) => {
      disposePromises.push(this.disposeRecorder(value, key));
    });

    // Wait for every recorder release operation to complete.
    await Promise.all(disposePromises);
    this.recorders.clear();
    console.info(`${this.TAG}: All recorders disposed.`);
  }

  /** Releases and removes one recorder entry. */
  async disposeRecorder(recorder: RecorderWrapper, recorderId: string): Promise<void> {
    try {
      await recorder.dispose();
    } catch (error) {
      const err = error as Error;
      console.error(`${this.TAG}: Error disposing recorder ${recorderId}: ${err.message}`);
    } finally {
      this.recorders.delete(recorderId);
    }
  }
}
