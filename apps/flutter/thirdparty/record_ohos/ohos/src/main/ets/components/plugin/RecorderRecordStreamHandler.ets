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
import { Any } from '@ohos/flutter_ohos';
import { EventSink, StreamHandler } from '@ohos/flutter_ohos/src/main/ets/plugin/common/EventChannel';

export class RecorderRecordStreamHandler implements StreamHandler {
  private eventSink: EventSink | null = null;

  /** Stores the Flutter event sink for PCM chunks. */
  onListen(args: Any, events: EventSink): void {
    this.eventSink = events;
  }

  /** Clears the PCM event sink when Flutter cancels its subscription. */
  onCancel() {
    this.eventSink = null;
  }

  /** Sends one owned PCM byte chunk to Flutter. */
  sendRecordChunkEvent(buffer: Uint8Array) {
    this.eventSink?.success(buffer)
  }
}
